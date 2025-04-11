use std::{
    fs::{File, OpenOptions},
    io::{BufRead, BufReader, LineWriter, Write},
};

use anyhow::anyhow;
use clap::{arg, ArgMatches, Command};
use hyprland::{
    ctl::switch_xkb_layout::SwitchXKBLayoutCmdTypes,
    data::{Devices, Keyboard as HyprlandKeyboard},
    shared::HyprData,
};

mod xkb;

use serde::Deserialize;
use xkb::{get_xkb_layouts, XkbLayout};
use xshell::{cmd, Shell};

use crate::{
    system_atlas::SYSTEM_ATLAS,
    util::{determine_wm, dmenu::Dmenu, WM},
};

// TODO: do these really need to be Strings?
#[derive(Debug, Clone)]
struct AlternativeLayout {
    name: String,
    dotfiles_path: String,
}

#[derive(Debug, Clone)]
enum KeyboardLayout {
    Xkb(XkbLayout),
    Alternative(AlternativeLayout),
}

#[derive(Deserialize, Debug, Clone)]
struct HyprctlKbLayoutOption {
    option: String,
    #[serde(rename(deserialize = "str"))]
    value: String,
    set: bool,
}

fn get_layout_names_hyprland(sh: &Shell) -> anyhow::Result<Vec<String>> {
    let layout_option_str = cmd!(sh, "hyprctl getoption input:kb_layout -j").read()?;
    let layout_option = serde_json::from_str::<HyprctlKbLayoutOption>(&layout_option_str)?;

    if !layout_option.set {
        anyhow::bail!(
            "'{}' is unset in the Hyprland configuration!",
            layout_option.option
        );
    }

    let layouts = layout_option
        .value
        .split(',')
        .map(|w| w.to_owned())
        .collect();

    Ok(layouts)
}

/// Get the current layout identifier number by reading the backing file of the eww keyboard-layout
/// widget, which we maintain ourselves. This is the only way of determining the current layout id,
/// other than maintaining a translation map between full layout+options+variant names and ids
fn get_current_layout_id_hyprland() -> anyhow::Result<u8> {
    let file = File::open(SYSTEM_ATLAS.eww_keyboard_layout)?;
    let bufreader = BufReader::new(file);
    let last_line = bufreader
        .lines()
        .last()
        .ok_or(anyhow!("The file should not be empty"))??;

    // Line format: json "[<id>,<name>]"
    // Could parse the json, but it might be simpler to parse the line by hand
    let (id_part, _) = last_line
        .split_once(',')
        .ok_or(anyhow!("Last line did not have the expected format"))?;
    // Get rid of the opening bracket
    let id_part = &id_part[1..];
    let id = id_part.parse::<u8>()?;

    Ok(id)
}

fn set_layout_by_id_hyprland(keyboards: &[HyprlandKeyboard], id: u8) -> anyhow::Result<()> {
    for kb in keyboards {
        hyprland::ctl::switch_xkb_layout::call(&kb.name, SwitchXKBLayoutCmdTypes::Id(id))?;
    }
    Ok(())
}

pub fn run(sh: &Shell, args: &ArgMatches) -> anyhow::Result<Option<String>> {
    let wm = determine_wm();

    match wm {
        WM::Hyprland => run_hyprland(sh, args),
        WM::GenericX11 => run_x11(sh, args),
    }
}

fn lookup_keyboard_layout_id_by_name_hyprland(
    layout_names: &[impl AsRef<str>],
    name: &str,
) -> anyhow::Result<u8> {
    Ok(layout_names
        .iter()
        .enumerate()
        .find(|(_, layout_name)| layout_name.as_ref() == name)
        .ok_or(anyhow!(
            "The given layout name does not correspond to an existing layout"
        ))?
        .0
        .try_into()?)
}

fn lookup_keyboard_layout_name_by_id_hyprland(
    layout_names: &[impl AsRef<str>],
    id: u8,
) -> anyhow::Result<String> {
    Ok(layout_names
        .get(id as usize)
        .ok_or(anyhow!("Given layout id should be valid at this point"))?
        .as_ref()
        .to_owned())
}

/// We're appending to the end of the backing file - eww uses `tail -F` to read the last line,
/// which might act wonkily if you truncate the file...
/// We're building the json by hand because it's very simple to do.
/// * `id`: id of the keyboard layout
/// * `name`: short name of the keyboard layout
fn write_layout_to_backing_file_hyprland(id: u8, name: &str) -> anyhow::Result<()> {
    let file = OpenOptions::new()
        .create(false)
        .append(true)
        .open(SYSTEM_ATLAS.eww_keyboard_layout)?;
    let mut writer = LineWriter::new(&file);
    writeln!(writer, "[{},\"{}\"]", id, name)?;

    Ok(())
}

fn run_hyprland(sh: &Shell, args: &ArgMatches) -> anyhow::Result<Option<String>> {
    let xkb_layouts = get_xkb_layouts(sh)?;
    let kyria_layout = AlternativeLayout {
        name: "kyria".to_owned(),
        dotfiles_path: SYSTEM_ATLAS.canary_dotfiles.to_owned(),
    };

    let mut all_layouts: Vec<KeyboardLayout> =
        xkb_layouts.into_iter().map(KeyboardLayout::Xkb).collect();
    all_layouts.push(KeyboardLayout::Alternative(kyria_layout));

    let keyboards = Devices::get()?.keyboards;
    let layout_names = get_layout_names_hyprland(sh)?;

    let changed_layout_id = match args.subcommand() {
        Some(("next", _)) => {
            // TODO: not sure what to do here yet...
            // it should not cycle between qwerty layouts and canary, because that switch is a bit
            // more elaborate than changing the xkb layout...
            // switching between qwerty only is ok, maybe I can keep that
            unimplemented!()
        }
        Some(("prev", _)) => {
            unimplemented!()
        }
        Some(("choose", _)) => {
            let chosen_layout_name = Dmenu::new(sh).choose_one(
                "Choose keyboard layout",
                &layout_names,
                String::as_ref,
                true,
            )?;
            let id = lookup_keyboard_layout_id_by_name_hyprland(&layout_names, chosen_layout_name)?;
            set_layout_by_id_hyprland(&keyboards, id)?;
            Some(id)
        }
        Some(("get", _)) => {
            let id = get_current_layout_id_hyprland()?;
            let name = lookup_keyboard_layout_name_by_id_hyprland(&layout_names, id)?;
            println!("{name}");
            None
        }
        Some(("set", set_args)) => {
            let name = set_args
                .get_one::<String>("NAME")
                .expect("NAME should be a required argument");
            let id = lookup_keyboard_layout_id_by_name_hyprland(&layout_names, name)?;
            set_layout_by_id_hyprland(&keyboards, id)?;
            Some(id)
        }
        _ => None,
    };

    if let Some(id) = changed_layout_id {
        let name = lookup_keyboard_layout_name_by_id_hyprland(&layout_names, id)?;
        write_layout_to_backing_file_hyprland(id, &name)?;
    }

    Ok(None)
}

fn run_x11(_sh: &Shell, _args: &ArgMatches) -> anyhow::Result<Option<String>> {
    unimplemented!()
}

pub fn command_extension(cmd: Command) -> Command {
    let inner_subcommands = [
        Command::new("next").about("Select the next keyboard layout"),
        Command::new("prev").about("Select the previous keyboard layout"),
        Command::new("choose").about("Choose a keyboard layout from the list of layouts"),
        Command::new("get").about("Get the current keyboard layout name"),
        Command::new("set")
            .about("Select the layout specified by name")
            .arg(arg!([NAME] "Name of the keyboard layout").required(true)),
    ];
    cmd.subcommand_required(true)
        .subcommands(inner_subcommands.iter())
}

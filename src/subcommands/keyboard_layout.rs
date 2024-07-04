use std::{
    fs::{File, OpenOptions},
    io::{BufRead, BufReader, LineWriter, Write},
};

use anyhow::anyhow;
use clap::{arg, ArgMatches, Command};
use hyprland::{
    ctl::switch_xkb_layout::SwitchXKBLayoutCmdTypes,
    data::{Devices, Keyboard},
    shared::HyprData,
};

use xshell::Shell;

use crate::{
    system_atlas::SYSTEM_ATLAS,
    util::{determine_wm, dmenu::get_platform_dmenu, WM},
};

/// The hyprland library does not allow us to query the list of registered keyboard layouts, and
/// we cannot do it through the hyprctl cli either - we have to parse the config file.
/// For now, we care only about the layout, and not the variant or the options, because we only
/// have one of each layout.
/// This is a _very_ naive implementation! I could make this more elaborate, but it would make
/// more sense to contribute to the hyprland library instead.
fn get_layout_names_hyprland() -> anyhow::Result<Vec<String>> {
    let hyprland_config = File::open(SYSTEM_ATLAS.hyprland)?;
    let reader = BufReader::new(hyprland_config);

    for line in reader.lines() {
        let Ok(line) = line else {
            continue;
        };

        let line_trimmed = line.trim();
        if !line_trimmed.starts_with("kb_layout") {
            continue;
        }

        let Some((_, layouts)) = line_trimmed.split_once('=') else {
            // This is fine; maybe we found the word we're looking for in a comment
            // (not very probable in this case though...
            continue;
        };

        return Ok(layouts
            .split(',')
            .map(|l| l.trim().to_owned())
            .collect::<Vec<_>>());
    }

    Err(anyhow::anyhow!(
        "Did not find definitions for keyboard layouts in the Hyprland configuration file!"
    ))
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

fn set_layout_by_id_hyprland(keyboards: &[Keyboard], id: u8) -> anyhow::Result<()> {
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
    let keyboards = Devices::get()?.keyboards;
    let layout_names = get_layout_names_hyprland()?;
    let layout_names_str = layout_names.iter().map(|e| e.as_ref()).collect::<Vec<_>>();

    let changed_layout_id = match args.subcommand() {
        Some(("next", _)) => {
            let id = get_current_layout_id_hyprland()?;
            let next = if id + 1 >= layout_names.len() as u8 {
                0
            } else {
                id + 1
            };
            set_layout_by_id_hyprland(&keyboards, next)?;
            Some(next)
        }
        Some(("prev", _)) => {
            let id = get_current_layout_id_hyprland()?;
            // This can overflow, but we're not gonna have more than 256 layouts...
            let prev = id.checked_sub(1).unwrap_or(layout_names.len() as u8 - 1);
            set_layout_by_id_hyprland(&keyboards, prev)?;
            Some(prev)
        }
        Some(("choose", _)) => {
            let chosen_layout_name = get_platform_dmenu().choose_one(
                sh,
                "Choose keyboard layout",
                &layout_names_str,
                true,
            )?;
            let id =
                lookup_keyboard_layout_id_by_name_hyprland(&layout_names, &chosen_layout_name)?;
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

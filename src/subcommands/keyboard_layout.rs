use std::{
    fs::File,
    io::{BufRead, BufReader},
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
    util::{determine_wm, dmenu, WM},
};

/// The hyprland library does not allow us to query the list of registered keyboard layouts, and
/// we cannot do it through the hyprctl cli either - we have to parse the config file.
/// For now, we care only about the layout, and not the variant or the options, because we only
/// have one of each layout.
/// This is a _very_ naive implementation! I could make this more elaborate, but it would make
/// more sense to contribute to the hyprland library instead.
fn get_layout_names_hyprland() -> anyhow::Result<Vec<String>> {
    println!("{}", SYSTEM_ATLAS.hyprland);
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
            // (not very probable in this case though...)
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

fn get_current_layout_name_hyprland() -> anyhow::Result<String> {
    Ok(Devices::get()?
        .keyboards
        .first()
        .ok_or(anyhow!("Could not find any connected keyboards"))?
        .active_keymap
        .clone())
}

fn select_next_layout_hyprland(keyboards: &[Keyboard]) -> anyhow::Result<()> {
    for kb in keyboards {
        hyprland::ctl::switch_xkb_layout::call(&kb.name, SwitchXKBLayoutCmdTypes::Next)?;
    }
    Ok(())
}

fn select_prev_layout_hyprland(keyboards: &[Keyboard]) -> anyhow::Result<()> {
    for kb in keyboards {
        hyprland::ctl::switch_xkb_layout::call(&kb.name, SwitchXKBLayoutCmdTypes::Previous)?;
    }
    Ok(())
}

fn set_layout_by_name_hyprland(
    layouts: &[impl AsRef<str>],
    keyboards: &[Keyboard],
    name: &str,
) -> anyhow::Result<()> {
    let layout_id: u8 = layouts
        .iter()
        .enumerate()
        .find(|(_, &ref layout_name)| layout_name.as_ref() == name)
        .ok_or(anyhow!(
            "The given layout name does not correspond to an existing layout"
        ))?
        .0
        .try_into()?;

    for kb in keyboards {
        hyprland::ctl::switch_xkb_layout::call(&kb.name, SwitchXKBLayoutCmdTypes::Id(layout_id))?;
    }
    Ok(())
}

pub fn run(sh: &Shell, args: &ArgMatches) -> anyhow::Result<()> {
    let wm = determine_wm();

    match wm {
        WM::Hyprland => run_hyprland(sh, args),
        WM::GenericX11 => run_x11(sh, args),
    }
}

fn run_hyprland(sh: &Shell, args: &ArgMatches) -> anyhow::Result<()> {
    let keyboards = Devices::get()?.keyboards;

    let changed_layout = match args.subcommand() {
        Some(("next", _)) => {
            select_next_layout_hyprland(&keyboards)?;
            get_current_layout_name_hyprland().ok()
        }
        Some(("prev", _)) => {
            select_prev_layout_hyprland(&keyboards)?;
            get_current_layout_name_hyprland().ok()
        }
        Some(("choose", _)) => todo!(),
        Some(("get", _)) => {
            get_current_layout_name_hyprland()?;
            None
        }
        Some(("set", set_args)) => {
            let name = set_args
                .get_one::<String>("NAME")
                .expect("NAME should be a required argument");
            let layouts = get_layout_names_hyprland()?;
            set_layout_by_name_hyprland(&layouts, &keyboards, name)?;
            get_current_layout_name_hyprland().ok()
        }
        _ => None,
    };

    if let Some(changed_layout) = changed_layout {
        // TODO: notify the system bar (eww in our case)
        println!("Layout is now {}", changed_layout);
    }

    Ok(())
}

fn run_x11(sh: &Shell, args: &ArgMatches) -> anyhow::Result<()> {
    unimplemented!()
}

pub fn command_extension(cmd: Command) -> Command {
    let inner_subcommands = vec![
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

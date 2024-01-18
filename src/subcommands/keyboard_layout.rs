use std::{
    fs::File,
    io::{BufRead, BufReader},
};

use clap::{ArgMatches, Command};

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

fn get_layout_names(sh: &Shell, wm: WM) -> anyhow::Result<Vec<String>> {
    match wm {
        WM::Hyprland => get_layout_names_hyprland(),
        _ => unimplemented!(),
    }
}

fn set_layout(sh: &Shell, layout_index: usize, wm: WM) -> anyhow::Result<()> {
    unimplemented!()
}

pub fn command_extension(cmd: Command) -> Command {
    cmd
}

fn print_layout_names(layout_names: &[String]) {
    println!("Found keyboard layouts:");
    for layout_name in layout_names {
        println!("{}", layout_name);
    }
}

pub fn run(sh: &Shell, _: &ArgMatches) -> anyhow::Result<()> {
    let wm = determine_wm();
    let layout_names = get_layout_names(sh, wm)?;

    print_layout_names(&layout_names);

    // unwrap: don't want to continue if string is empty
    // let result_index_str = dmenu(sh, "Choose keyboard layout", &layout_names, true).unwrap();
    //
    // // unwrap: split always return at least 1 element
    // let result_index_str = result_index_str.split(':').next().unwrap();
    //
    // let result_index = usize::from_str(result_index_str)?;
    //
    // set_layout(sh, result_index, wm)?;

    // TODO: notify the system bar (eww in our case)

    Ok(())
}

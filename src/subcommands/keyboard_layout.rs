use clap::{ArgMatches, Command};
use std::str::FromStr;

use xshell::{cmd, Shell};

use crate::{
    system_atlas::SystemAtlas,
    util::{determine_wm, dmenu, WM},
};

fn get_layout_names_hyprland(sh: &Shell) -> anyhow::Result<Vec<String>> {
    // The hyprland library does not allow us to query the list of registered keyboard layouts, and
    // we cannot do it through the hyprctl cli either - we have to parse the config file
    let hyprland_config_file = format!("{}/hypr", env!("XDG_CONFIG_HOME"))

}

fn get_layout_names(sh: &Shell, wm: WM) -> anyhow::Result<Vec<String>> {
    unimplemented!()
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

pub fn run(sh: &Shell, _: &ArgMatches, _: &SystemAtlas) -> anyhow::Result<()> {
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

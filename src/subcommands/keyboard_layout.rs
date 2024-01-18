use clap::{ArgMatches, Command};
use std::str::FromStr;

use xshell::{cmd, Shell};

use crate::{
    system_atlas::SystemAtlas,
    util::{determine_wm, dmenu, WM},
};

fn get_layout_names(sh: &Shell, wm: WM) -> anyhow::Result<Vec<String>> {
    unimplemented!()
}

fn set_layout(sh: &Shell, layout_index: usize, wm: WM) -> anyhow::Result<()> {
    unimplemented!()
}

pub fn command_extension(cmd: Command) -> Command {
    cmd
}

pub fn run(sh: &Shell, _: &ArgMatches, _: &SystemAtlas) -> anyhow::Result<()> {
    let wm = determine_wm();
    let layout_names = get_layout_names(sh, wm)?;

    // unwrap: don't want to continue if string is empty
    let result_index_str = dmenu(sh, "Choose keyboard layout", &layout_names, true).unwrap();

    // unwrap: split always return at least 1 element
    let result_index_str = result_index_str.split(':').next().unwrap();

    let result_index = usize::from_str(result_index_str)?;

    set_layout(sh, result_index, wm)?;

    // TODO: notify the system bar (eww in our case)

    Ok(())
}

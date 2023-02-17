use clap::ArgMatches;
use xshell::{Shell, cmd};

use crate::util::dmenu;

pub fn run(sh: &Shell, _: &ArgMatches) -> anyhow::Result<()> {

    let list_output = cmd!(sh, "lutris -l").ignore_stderr().read()?;
    let mut choices = list_output
        .split('\n')
        .map(|s| s.split('|').take(2).collect::<Vec<_>>().join("|"))
        .collect::<Vec<_>>();
    choices.sort();

    // unwrap: we don't want to continue if result is empty
    let result = dmenu(sh, "Choose game", &choices, true).unwrap();

    // unwrap: result always contains a pipe, and the first element is always a number
    let num = result.split('|').next().unwrap().trim();

    let _ = cmd!(sh, "lutris lutris:rungameid/{num}").run();
    Ok(())
}

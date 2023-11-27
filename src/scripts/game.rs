use clap::{arg, ArgMatches, Command};
use xshell::{cmd, Shell};

use crate::{system_atlas::SystemAtlas, util::dmenu};

pub fn command(cmd: Command<'static>) -> Command<'static> {
    cmd.arg(arg!([GAME]))
}

pub fn run(sh: &Shell, args: &ArgMatches, _: &SystemAtlas) -> anyhow::Result<()> {
    let list_output = cmd!(sh, "lutris -l").ignore_stderr().read()?;
    let mut choices = list_output
        .split('\n')
        .map(|s| s.split('|').take(2).collect::<Vec<_>>().join("|"))
        .collect::<Vec<_>>();
    choices.sort();

    let mut filtered_choices = choices.clone();
    let search = "";
    if let Some(search) = args.get_one::<String>("GAME") {
        filtered_choices.retain(|name| name.contains(search));
    }

    let result: String;

    // If there is only one result, there's no point in showing dmenu;
    // game should be launched directly
    if filtered_choices.len() == 1 {
        result = filtered_choices.remove(0);
    } else if filtered_choices.is_empty() {
        result = dmenu(
            sh,
            &format!("Choose game (no matches found for '{search}')"),
            &choices,
            true,
        )
        .unwrap();
    } else {
        // unwrap: we don't want to continue if result is empty
        result = dmenu(sh, "Choose game", &filtered_choices, true).unwrap();
    }

    // unwrap: result always contains a pipe, and the first element is always a number
    let num = result.split('|').next().unwrap().trim();

    let _ = cmd!(sh, "lutris lutris:rungameid/{num}").run();
    Ok(())
}

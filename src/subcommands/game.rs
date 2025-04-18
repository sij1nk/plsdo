use clap::{arg, ArgMatches, Command};
use xshell::{cmd, Shell};

use crate::util::dmenu::Dmenu;

pub fn command_extension(cmd: Command) -> Command {
    cmd.arg(arg!([GAME]))
}

pub fn run(sh: &Shell, args: &ArgMatches) -> anyhow::Result<Option<String>> {
    let dmenu = Dmenu::new(sh);
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

    // If there is only one result, there's no point in showing dmenu;
    // game should be launched directly
    let result = if filtered_choices.len() == 1 {
        filtered_choices[0].as_str()
    } else if filtered_choices.is_empty() {
        dmenu
            .choose_one(
                &format!("Choose game (no matches found for '{search}')"),
                &choices,
                String::as_ref,
            )
            .unwrap()
    } else {
        // unwrap: we don't want to continue if result is empty
        dmenu
            .choose_one("Choose game", &filtered_choices, String::as_ref)
            .unwrap()
    };

    // unwrap: result always contains a pipe, and the first element is always a number
    let num = result.split('|').next().unwrap().trim();

    let _ = cmd!(sh, "lutris lutris:rungameid/{num}").run();
    Ok(None)
}

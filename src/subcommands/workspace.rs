use clap::{arg, ArgMatches, Command};
use xshell::Shell;

pub fn command_extension(cmd: Command) -> Command {
    let inner_subcommands = vec![
        Command::new("focus")
            .about("Move focus to the specified workspace")
            .subcommand_required(true)
            .subcommands(
                vec![
                    Command::new("next")
                        .about("Move focus to the next workspace on the monitor")
                        .arg(arg!([MONITOR] "Identifier of the monitor").required(true)),
                    Command::new("prev")
                        .about("Move focus to the previous workspace on the monitor")
                        .arg(arg!([MONITOR] "Identifier of the monitor").required(true)),
                    Command::new("id")
                        .about("Move focus to the workspace with the given identifier")
                        .arg(arg!([WORKSPACE] "Identifier of the workspace").required(true)),
                ]
                .iter(),
            ),
        Command::new("move")
            .about("Move focus and the current window to the specified workspace")
            .arg(arg!([WORKSPACE] "Identifier of the workspace").required(true)),
        Command::new("open_pinned")
            .about("Open and navigate to a pinned window")
            .arg(
                arg!([PROGRAM] "The name of the program whose pinned window to navigate to")
                    .required(true),
            ),
    ];
    cmd.subcommand_required(true)
        .subcommands(inner_subcommands.iter())
}

pub fn run(_: &Shell, _: &ArgMatches) -> anyhow::Result<()> {
    Ok(())
}

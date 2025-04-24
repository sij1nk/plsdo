use clap::{ArgMatches, Command};
use xshell::Shell;

pub fn command_extension(cmd: Command) -> Command {
    cmd
}

pub fn run(sh: &Shell, _: &ArgMatches) -> anyhow::Result<Option<String>> {
    todo!()
}

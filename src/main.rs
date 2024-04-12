use plsdo::{get_command, run_subcommand};
use xshell::Shell;

fn main() -> anyhow::Result<()> {
    let shell = Shell::new()?;

    let command = get_command();
    let matches = command.get_matches();
    run_subcommand(&shell, &matches)?;

    Ok(())
}

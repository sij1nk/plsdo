use plsdo::{get_command, run_subcommand};
use xshell::Shell;

const LESLIE: &str = "https://www.youtube.com/watch?v=aw1O6jpASTI";

#[test]
fn dingy() -> anyhow::Result<()> {
    let shell = Shell::new()?;

    let command = get_command();
    let matches = command.get_matches_from(vec!["plsdo", "ytdl", "download", "url", LESLIE]);
    run_subcommand(&shell, &matches)?;

    Ok(())
}

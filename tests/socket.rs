use std::{thread, time::Duration};

use ntest::timeout;
use plsdo::{get_command, run_subcommand};
use xshell::Shell;

fn run(shell: &Shell, params: &[&str]) -> anyhow::Result<Option<String>> {
    let command = get_command();
    let matches = command.get_matches_from(params);
    run_subcommand(shell, &matches)
}

#[test]
#[timeout(500)]
fn works_with_emulated_data() -> anyhow::Result<()> {
    let shell = Shell::new()?;

    let mut filename = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    filename.push_str("/tests/inputs/");
    filename.push_str("emulated_file1");

    let _handle = thread::spawn(|| {
        let shell = Shell::new().unwrap();
        let _ = run(&shell, &["plsdo", "ytdl", "run_aggregator"]);
    });

    thread::sleep(Duration::from_millis(200));

    run(&shell, &["plsdo", "ytdl", "emulate", &filename])?;
    let result = run(&shell, &["plsdo", "ytdl", "get_download_progress"])?;

    // TODO: check res

    Ok(())
}

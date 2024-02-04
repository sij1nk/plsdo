use clap::{ArgMatches, Command};
use std::{
    io::{BufRead, BufReader},
    process::{Command as StdCommand, Stdio},
};
use xshell::{cmd, Shell};

pub fn command_extension(cmd: Command) -> Command {
    cmd
}

pub fn run(sh: &Shell, _: &ArgMatches) -> anyhow::Result<()> {
    let mut child = StdCommand::new("yt-dlp")
        .args([
            "-r",
            "4096",
            "-f",
            "598",
            "-q",
            "--progress",
            "--newline",
            "https://www.youtube.com/watch?v=6IF5V6tv9LM",
        ])
        .stdout(Stdio::piped())
        .spawn()
        .expect("it to work");
    let stdout = child.stdout.take().expect("Child should have stdout");
    let bufreader = BufReader::new(stdout);
    for line in bufreader.lines() {
        match line {
            Ok(line) => println!("{}", line),
            Err(err) => println!("Error: {:?}", err),
        }
    }

    let ecode = child.wait().expect("wait on child failed");
    println!("Ecode: {}", ecode);
    Ok(())
}

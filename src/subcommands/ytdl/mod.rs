use clap::{arg, ArgMatches, Command};
use std::{
    io::{BufRead, BufReader},
    process::{Command as StdCommand, Stdio},
};
use xshell::{cmd, Shell};

mod progress_server;

struct DownloadProgress {
    percent: u8,
    total_size: u32,
    download_speed: u32,
    eta: u32, // TODO: format?
}

struct DownloadMetadata {
    title: String,
    url: String,
    filename: String,
}

pub fn command_extension(cmd: Command) -> Command {
    let inner_subcommands = [
        Command::new("download").about("Download a video or audio file")
        .arg_required_else_help(true)
            .subcommand_required(true).subcommands(
                [
                    Command::new("url").about("Download a video or audio file from a given URL").arg_required_else_help(true).arg(arg!([URL] "The URL to download from")),
                    Command::new("clipboard").about("Download a video or audio file, trying to interpret the clipboard contents as an URL")
                ]
            ),
        Command::new("run_progress_server").about("Run the progress server, which aggregates the progress of ongoing downloads"),
        Command::new("get_download_progress").about("Get the progress of ongoing downloads")
    ];
    cmd.subcommand_required(true)
        .arg_required_else_help(true)
        .subcommands(inner_subcommands.iter())
}

fn download(download_args: &ArgMatches) -> anyhow::Result<()> {
    let url = match download_args.subcommand() {
        Some(("url", url_args)) => {
            let url_arg = url_args
                .get_one::<String>("URL")
                .expect("URL should be a required argument");
            Some(url_arg.as_str())
        }
        Some(("clipboard", clipboard_args)) => {
            // TODO: read from top of clipboard
            Some("https://www.youtube.com/watch?v=6IF5V6tv9LM")
        }
        _ => None,
    };

    let Some(url) = url else {
        return Err(anyhow::anyhow!(
            "Did not receive a valid URL to download a video from"
        ));
    };

    let mut child = StdCommand::new("yt-dlp")
        .args([
            "-r",
            "4096",
            "-f",
            "160",
            // "-q",
            "--progress",
            "--newline",
            url,
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

pub fn run(sh: &Shell, args: &ArgMatches) -> anyhow::Result<()> {
    match args.subcommand() {
        Some(("download", download_args)) => download(download_args)?,
        Some(("run_progress_server", _)) => progress_server::run_progress_server()?,
        Some(("get_download_progress", _)) => {}
        _ => {}
    }

    Ok(())
}

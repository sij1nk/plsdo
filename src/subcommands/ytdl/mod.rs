use clap::{arg, ArgMatches, Command};
use serde::{Deserialize, Serialize};
use std::{
    io::{BufRead, BufReader},
    os::unix::net::UnixStream,
    process::{Command as StdCommand, Stdio},
    str::FromStr,
};
use strum::IntoEnumIterator;
use strum_macros::{Display, EnumIter, EnumString};
use xshell::Shell;

use crate::{
    system_atlas::SYSTEM_ATLAS,
    util::{dmenu, get_clipboard_contents},
};

use self::ytdl_line::YtdlLine;

mod aggregator;
mod test_macros;
mod ytdl_line;

#[derive(Debug, Display, Clone, Copy, EnumString, EnumIter)]
// #[strum(serialize_all = "PascalCase")]
enum DownloadFormat {
    UpTo1440p,
    UpTo1080p,
    UpTo720p,
    UpTo480p,
    WorstVideo,
    AudioOnly,
}

fn get_download_format_specifier(format: &DownloadFormat) -> &'static [&'static str] {
    match format {
        DownloadFormat::UpTo1440p => {
            &["-f", "bestvideo[height<=1440]+bestaudio/best[height<=1440]"]
        }
        DownloadFormat::UpTo1080p => {
            &["-f", "bestvideo[height<=1080]+bestaudio/best[height<=1080]"]
        }
        DownloadFormat::UpTo720p => &["-f", "bestvideo[height<=720]+bestaudio/best[height<=720]"],
        DownloadFormat::UpTo480p => &["-f", "bestvideo[height<=480]+bestaudio/best[height<=480]"],
        DownloadFormat::WorstVideo => &["-S", "+size,+br,+res,+fps"],
        DownloadFormat::AudioOnly => &["-x", "--audio-format", "mp3"],
    }
}

pub fn command_extension(cmd: Command) -> Command {
    let inner_subcommands = [
        Command::new("download").about("Download a video or audio file")
            .arg_required_else_help(true)
            .subcommand_required(true).subcommands(
                [
                    Command::new("url").about("Download a video or audio file from a given URL")
                        .arg_required_else_help(true)
                        .arg(arg!([URL] "The URL to download from")),
                    Command::new("clipboard").about("Download a video or audio file, trying to interpret the clipboard contents as an URL")
                ]
            ),
        Command::new("run_aggregator").about("Run the aggregator server, which aggregates the progress of ongoing downloads"),
        Command::new("get_download_progress").about("Get the progress of ongoing downloads from the aggregator")
    ];
    cmd.subcommand_required(true)
        .arg_required_else_help(true)
        .subcommands(inner_subcommands.iter())
}

fn get_download_url(download_args: &ArgMatches) -> anyhow::Result<String> {
    match download_args.subcommand() {
        Some(("url", url_args)) => {
            let url_arg = url_args
                .get_one::<String>("URL")
                .expect("URL should be a required argument");
            Ok(url_arg.clone())
        }
        Some(("clipboard", _)) => {
            let string_from_clipboard = get_clipboard_contents()?;
            Ok(string_from_clipboard)
        }
        _ => panic!("Missing required subcommand for 'download'"),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum Message {
    QueryMessage,
    DownloadProcessMessage(DownloadProcessMessage),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DownloadProcessMessage {
    pid: u32,
    payload: MessagePayload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum MessagePayload {
    YtdlLine(YtdlLine),
    ProcessExited(i32),
}

fn send_message(stream: &UnixStream, message: &Message) -> anyhow::Result<()> {
    serde_json::to_writer(stream, message)?;
    Ok(())
}

fn process_lines(
    pid: u32,
    stream: &UnixStream,
    lines: impl Iterator<Item = std::io::Result<String>>,
) {
    for line in lines {
        match line {
            Ok(line) => {
                if let Ok(line) = ytdl_line::parse(&line) {
                    let message = Message::DownloadProcessMessage(DownloadProcessMessage {
                        pid,
                        payload: MessagePayload::YtdlLine(line),
                    });
                    if let Err(err) = send_message(stream, &message) {
                        eprintln!("Could not send message!");
                        eprintln!("{:?}", err);
                    }
                }
            }
            Err(err) => eprintln!("Error: {:?}", err),
        }
    }
}

fn download(sh: &Shell, download_args: &ArgMatches) -> anyhow::Result<()> {
    let url = get_download_url(download_args)?;

    let formats: Vec<_> = DownloadFormat::iter().map(|opt| opt.to_string()).collect();
    let choice = dmenu(sh, "Choose download format", &formats, true)?;
    let format = DownloadFormat::from_str(&choice)?;
    let format_specifier = get_download_format_specifier(&format);

    let mut child = StdCommand::new("yt-dlp")
        .args([format_specifier, &["--progress", "--newline", &url]].concat())
        .stdout(Stdio::piped())
        .spawn()
        .expect("it to work");
    let stdout = child.stdout.take().expect("Child should have stdout");
    let bufreader = BufReader::new(stdout);
    let pid = child.id();
    let stream = UnixStream::connect(SYSTEM_ATLAS.ytdl_aggregator_socket)?;
    process_lines(pid, &stream, bufreader.lines());

    let ecode = child.wait().expect("wait on child failed");
    let ecode = ecode.code().unwrap_or(1);
    let message = Message::DownloadProcessMessage(DownloadProcessMessage {
        pid,
        payload: MessagePayload::ProcessExited(ecode),
    });
    let _ = send_message(&stream, &message);
    Ok(())
}

pub fn run(sh: &Shell, args: &ArgMatches) -> anyhow::Result<()> {
    match args.subcommand() {
        Some(("download", download_args)) => download(sh, download_args)?,
        Some(("run_aggregator", _)) => aggregator::run()?,
        Some(("get_download_progress", _)) => {}
        _ => {}
    }

    Ok(())
}

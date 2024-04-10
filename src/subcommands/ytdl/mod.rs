use clap::{arg, value_parser, ArgMatches, Command, ValueEnum};
use serde::{Deserialize, Serialize};
use std::{
    io::{BufRead, BufReader, Read, Write},
    os::unix::net::UnixStream,
    process::{Command as StdCommand, Stdio},
};
use strum::IntoEnumIterator;
use strum_macros::{Display, EnumIter};
use xshell::Shell;

use crate::{
    system_atlas::SYSTEM_ATLAS,
    util::{dmenu, get_clipboard_contents},
};

use self::ytdl_line::YtdlLine;

mod aggregator;
mod test_macros;
mod ytdl_line;

// TODO: allow better values for --format argument (e.g. "1440p", "worst-video") but keep dmenu
// working
#[derive(Debug, Display, Clone, Copy, EnumIter, ValueEnum)]
#[clap(rename_all = "verbatim")]
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
            ).arg(arg!(-f --format <FORMAT>).value_parser(value_parser!(DownloadFormat))),
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

fn send_message(mut stream: &UnixStream, message: &Message) -> anyhow::Result<()> {
    let mut string = serde_json::to_string(message)?;
    string.push('\0');
    stream.write_all(string.as_bytes())?;
    // serde_json::to_writer(stream, message)?;
    Ok(())
}

fn send_query_message(mut stream: &UnixStream, message: &Message) -> anyhow::Result<String> {
    send_message(stream, message)?;
    let mut response = String::new();
    stream.read_to_string(&mut response)?;
    Ok(response)
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
    let format = download_args
        .get_one::<DownloadFormat>("format")
        .copied()
        .ok_or(anyhow::anyhow!("Download format was not specified"))
        .or_else(|_| {
            let formats: Vec<_> = DownloadFormat::iter().map(|opt| opt.to_string()).collect();
            dmenu(sh, "Choose download format", &formats, true).and_then(|value| {
                DownloadFormat::from_str(&value, true).map_err(|e| anyhow::anyhow!(e))
            })
        })?;

    println!("{:?}", format);
    let format_specifier = get_download_format_specifier(&format);

    let mut child = StdCommand::new("yt-dlp")
        .args(
            [
                format_specifier,
                &["--progress", "--newline", "-r", "16384", &url],
            ]
            .concat(),
        )
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
        Some(("get_download_progress", _)) => {
            let message = Message::QueryMessage;
            let stream = UnixStream::connect(SYSTEM_ATLAS.ytdl_aggregator_socket)?;
            let response = send_query_message(&stream, &message)?;
            println!("{response}");
        }
        _ => {}
    }

    Ok(())
}

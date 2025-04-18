use anyhow::Context;
use clap::{arg, value_parser, ArgMatches, Command, ValueEnum};
use serde::{Deserialize, Serialize};
use std::os::unix::net::UnixDatagram;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;
use xshell::Shell;

use crate::{
    system_atlas::SYSTEM_ATLAS,
    util::{determine_wm, dmenu::Dmenu, Clipboard, RealClipboard},
};

use self::{
    downloader::{DownloadWaitHandle, Downloader, YtdlDownloader},
    ytdl_line::YtdlLine,
};

mod aggregator;
mod downloader;
mod test_macros;
mod ytdl_line;

pub type ProcessId = u32;

// TODO: allow better values for --format argument (e.g. "1440p", "worst-video") but keep dmenu
// working
#[derive(Debug, strum_macros::Display, Clone, Copy, EnumIter, ValueEnum)]
#[clap(rename_all = "verbatim")]
enum DownloadFormat {
    #[clap(name = "1440p")]
    #[strum(serialize = "1440p")]
    UpTo1440p,
    #[clap(name = "1080p")]
    #[strum(serialize = "1080p")]
    UpTo1080p,
    #[clap(name = "720p")]
    #[strum(serialize = "720p")]
    UpTo720p,
    #[clap(name = "480p")]
    #[strum(serialize = "480p")]
    UpTo480p,
    #[clap(name = "worst")]
    #[strum(serialize = "worst")]
    WorstVideo,
    #[clap(name = "audio-only")]
    #[strum(serialize = "audio-only")]
    AudioOnly,
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
                    Command::new("clipboard").about("Download a video or audio file, trying to interpret the clipboard contents as an URL"),
                ]
            ).arg(arg!(-f --format <FORMAT>).value_parser(value_parser!(DownloadFormat))),
        Command::new("run_aggregator").about("Run the aggregator server, which aggregates the progress of ongoing downloads"),
        Command::new("get_download_progress").about("Get the progress of ongoing downloads from the aggregator")
    ];
    cmd.subcommand_required(true)
        .arg_required_else_help(true)
        .subcommands(inner_subcommands.iter())
}

fn get_download_url(
    download_args: &ArgMatches,
    clipboard: impl Clipboard,
) -> anyhow::Result<String> {
    match download_args.subcommand() {
        Some(("url", url_args)) => {
            let url_arg = url_args
                .get_one::<String>("URL")
                .expect("URL should be a required argument");
            Ok(url_arg.clone())
        }
        Some(("clipboard", _)) => {
            let string_from_clipboard = clipboard.get_one()?;
            Ok(string_from_clipboard)
        }
        _ => panic!("Missing required subcommand for 'download'"),
    }
}

fn get_download_format(download_args: &ArgMatches, sh: &Shell) -> anyhow::Result<DownloadFormat> {
    download_args
        .get_one::<DownloadFormat>("format")
        .copied()
        // this error is never returned, but we have to give it something for the Option -> Result conversion
        .ok_or(anyhow::anyhow!("Download format was not specified"))
        .or_else(|_| {
            let formats: Vec<_> = DownloadFormat::iter().map(|opt| opt.to_string()).collect();
            let formats_str = formats.iter().map(|e| e.as_ref()).collect::<Vec<_>>();
            Dmenu::new(sh)
                .numbered()
                .auto_select()
                .choose_one_str("Choose download format", &formats_str)
                .and_then(|value| {
                    DownloadFormat::from_str(&value, true).map_err(|e| anyhow::anyhow!(e))
                })
        })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum Message {
    QueryMessage,
    DownloadProcessMessage(DownloadProcessMessage),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DownloadProcessMessage {
    pid: ProcessId,
    payload: MessagePayload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum MessagePayload {
    YtdlLine(YtdlLine),
    ProcessExited(i32),
}

fn send_message(socket: &UnixDatagram, message: &Message) -> anyhow::Result<()> {
    let string = serde_json::to_string(message)?;
    socket.send(string.as_bytes())?;
    Ok(())
}

fn send_query_message(socket: &UnixDatagram, message: &Message) -> anyhow::Result<String> {
    send_message(socket, message)?;
    // TODO: find optimal buffer size
    let mut buf = vec![0; 1024];
    let _ = socket.recv(buf.as_mut_slice());
    println!("Query: received answer");
    let response = String::from_utf8(buf)?;
    Ok(response)
}

fn process_lines(
    pid: ProcessId,
    stream: &UnixDatagram,
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

fn connect_to_aggregator(path: Option<&str>) -> anyhow::Result<UnixDatagram> {
    let socket = match path {
        Some(str) => UnixDatagram::bind(str),
        None => UnixDatagram::unbound(),
    }?;
    socket
        .connect(SYSTEM_ATLAS.ytdl_aggregator_socket)
        .context("Cannot connect to the aggregator process; is it running?")?;
    Ok(socket)
}

fn download(
    sh: &Shell,
    download_args: &ArgMatches,
    clipboard: impl Clipboard,
    downloader: impl Downloader,
) -> anyhow::Result<()> {
    let url = get_download_url(download_args, clipboard)?;
    let format = get_download_format(download_args, sh)?;

    let (pid, stdout_lines, wait_handle) = downloader.download(url, &format)?;

    let stream = connect_to_aggregator(None)?;
    process_lines(pid, &stream, stdout_lines);

    let ecode = wait_handle.wait().expect("wait on child failed"); // TODO: return error instead of panic
    let ecode = ecode.code().unwrap_or(1);

    // TODO: extract
    let message = Message::DownloadProcessMessage(DownloadProcessMessage {
        pid,
        payload: MessagePayload::ProcessExited(ecode),
    });
    let _ = send_message(&stream, &message);
    Ok(())
}

pub fn run(sh: &Shell, args: &ArgMatches) -> anyhow::Result<Option<String>> {
    match args.subcommand() {
        Some(("download", download_args)) => {
            let wm = determine_wm();
            let clipboard = RealClipboard::new(wm);
            let downloader = YtdlDownloader::new();
            download(sh, download_args, clipboard, downloader)?
        }
        Some(("run_aggregator", _)) => aggregator::run()?,
        Some(("get_download_progress", _)) => {
            let message = Message::QueryMessage;
            let socket_path = generate_socket_path();
            let socket = connect_to_aggregator(Some(&socket_path))?;
            let response = send_query_message(&socket, &message)?;
            println!("Response:");
            println!("{response}");
            return Ok(Some(response));
        }
        _ => {}
    }

    Ok(None)
}

fn generate_socket_path() -> String {
    let secs_since_epoch = std::time::SystemTime::UNIX_EPOCH
        .elapsed()
        .expect("UNIX_EPOCH is later than current system time")
        .as_secs();
    format!("/tmp/plsdo-ytdl-download-process-{secs_since_epoch}")
}

use anyhow::Context;
use clap::{arg, value_parser, ArgMatches, Command, ValueEnum};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs::File,
    io::{BufRead, BufReader},
    os::unix::net::UnixDatagram,
};
use strum::IntoEnumIterator;
use strum_macros::EnumIter;
use xshell::Shell;

use crate::{
    system_atlas::SYSTEM_ATLAS,
    util::{
        determine_wm,
        dmenu::{get_platform_dmenu, Dmenu},
        Clipboard, RealClipboard,
    },
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
        Command::new("emulate").about("Emulate a ytdl download by reading ytdl stdout from a file (for testing purposes)")
            .arg_required_else_help(true)
            .arg(arg!([FILE] "The file to read from")),
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

// TODO: get dmenu as impl trait parameter
fn get_download_format(
    download_args: &ArgMatches,
    sh: &Shell,
    dmenu: Box<dyn Dmenu>,
) -> anyhow::Result<DownloadFormat> {
    download_args
        .get_one::<DownloadFormat>("format")
        .copied()
        // this error is never returned, but we have to give it something for the Option -> Result conversion
        .ok_or(anyhow::anyhow!("Download format was not specified"))
        .or_else(|_| {
            let formats: Vec<_> = DownloadFormat::iter().map(|opt| opt.to_string()).collect();
            let formats_str = formats.iter().map(|e| e.as_ref()).collect::<Vec<_>>();
            dmenu
                .choose_one(sh, "Choose download format", &formats_str, true)
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
    // FIXME: hangs
    socket.recv(buf.as_mut_slice())?;
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

type EmulatedFileContents = BTreeMap<ProcessId, Vec<String>>;

fn read_emulated_file(filename: &str) -> anyhow::Result<EmulatedFileContents> {
    let mut contents: EmulatedFileContents = BTreeMap::new();
    let mut current_pid: Option<ProcessId> = None;

    let file = File::open(filename)?;
    let bufreader = BufReader::new(file);
    for line in bufreader.lines().map_while(Result::ok) {
        if let Ok(pid) = line.parse::<ProcessId>() {
            current_pid = Some(pid);
            continue;
        }

        let Some(pid) = current_pid else {
            return Err(anyhow::anyhow!(
                "First line of emulated file should be a process id"
            ));
        };

        contents.entry(pid).or_default().push(line);
    }

    Ok(contents)
}

fn connect_to_aggregator() -> anyhow::Result<UnixDatagram> {
    let socket = UnixDatagram::unbound()?;
    // NOTE: causes Resource temporarily unavailable (ecode 11) when reading query message response
    // should look into how unix sockets work in depth...
    // socket.set_read_timeout(Some(Duration::from_secs(1)))?;
    // socket.set_write_timeout(Some(Duration::from_secs(1)))?;
    socket
        .connect(SYSTEM_ATLAS.ytdl_aggregator_socket)
        .context("Cannot connect to the aggregator process; is it running?")?;
    Ok(socket)
}

fn emulate_download(emulate_args: &ArgMatches) -> anyhow::Result<()> {
    let filename = emulate_args
        .get_one::<String>("FILE")
        .expect("FILE should be a required argument");
    let stream = connect_to_aggregator()?;
    let contents = read_emulated_file(filename)?;
    for (pid, lines) in contents {
        process_lines(pid, &stream, lines.into_iter().map(Ok));
    }

    Ok(())
}

fn download(
    sh: &Shell,
    download_args: &ArgMatches,
    clipboard: impl Clipboard,
    downloader: impl Downloader,
) -> anyhow::Result<()> {
    let dmenu = get_platform_dmenu();
    let url = get_download_url(download_args, clipboard)?;
    let format = get_download_format(download_args, sh, dmenu)?;

    let (pid, stdout_lines, wait_handle) = downloader.download(url, &format)?;

    let stream = connect_to_aggregator()?;
    // TODO: consider mapping to parsed lines, and send separately
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
        Some(("emulate", emulate_args)) => emulate_download(emulate_args)?,
        Some(("run_aggregator", _)) => aggregator::run()?,
        Some(("get_download_progress", _)) => {
            let message = Message::QueryMessage;
            let socket = connect_to_aggregator()?;
            let response = send_query_message(&socket, &message)?;
            println!("{response}");
            return Ok(Some(response));
        }
        _ => {}
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use crate::subcommands::ytdl::read_emulated_file;

    #[test]
    fn read_emulated_file_works() {
        let mut filename = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        filename.push_str("/tests/inputs/");
        filename.push_str("emulated_file1");
        let contents = read_emulated_file(&filename).unwrap();

        assert_eq!(contents.len(), 3);
        assert_eq!(contents.get(&1000).unwrap().len(), 12);
        assert_eq!(contents.get(&1001).unwrap().len(), 3);
        assert_eq!(contents.get(&1002).unwrap().len(), 13);
    }
}

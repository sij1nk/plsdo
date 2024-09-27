use std::{
    collections::BTreeMap,
    fs,
    os::unix::net::{SocketAddr, UnixDatagram},
    path::Path,
    sync::{Arc, Mutex},
};

use serde::{Deserialize, Serialize};

use crate::system_atlas::SYSTEM_ATLAS;

use super::{ytdl_line::Progress, DownloadProcessMessage, Message, ProcessId};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UrlOnly {
    url: String,
}

impl UrlOnly {
    fn create_metadata(self, path_string: String) -> anyhow::Result<DownloadMetadata> {
        let path = Path::new(&path_string);
        let title = String::from(path.file_stem().and_then(|t| t.to_str()).ok_or(
            anyhow::anyhow!("Downloaded file has no file extension, or is not valid UTF-8"),
        )?);
        let metadata = DownloadMetadata {
            url: self.url,
            path: path_string,
            title,
        };
        Ok(metadata)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DownloadMetadata {
    url: String,
    path: String,
    title: String,
}

impl DownloadMetadata {
    fn create_full_download_info(self, progress: Progress) -> FullDownloadInfo {
        FullDownloadInfo {
            metadata: self,
            progress,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FullDownloadInfo {
    metadata: DownloadMetadata,
    progress: Progress,
}

impl FullDownloadInfo {
    fn update_progress(&mut self, progress: Progress) {
        self.progress = progress;
    }
    fn is_completed(&self) -> bool {
        match self.progress {
            Progress::Downloading(ref p) => p.percent == 100,
            Progress::Extracting => false,
        }
    }
    fn set_as_completed(&mut self) {
        if let Progress::Downloading(ref mut p) = self.progress {
            p.percent = 100;
            p.download_speed = 0;
            p.eta = 0;
        }
    }
    fn set_as_extracting(&mut self) {
        self.progress = Progress::Extracting;
    }
}

// TODO: prepare progress server to be a systemd service? logging and stuff?
// TODO: pretty printing progress to console?

type State = Arc<Mutex<BTreeMap<ProcessId, DownloadInfo>>>;

#[derive(Debug, Serialize, Deserialize)]
enum DownloadInfo {
    UrlOnly(UrlOnly),
    MetadataOnly(DownloadMetadata),
    Full(FullDownloadInfo),
}

fn handle_query_message(
    state: &State,
    socket: &UnixDatagram,
    dp_addr: &SocketAddr,
) -> anyhow::Result<()> {
    let dp_socket_path = dp_addr.as_pathname().ok_or(anyhow::anyhow!(
        "Download process socket is unbound; can't reply to query message"
    ))?;
    let state = state.lock().expect("lock to work");
    let state_string = serde_json::to_string_pretty(&*state)?;
    println!("State string: {state_string}");
    socket.send_to(state_string.as_bytes(), dp_socket_path)?;
    Ok(())
}

fn handle_message(
    state: &State,
    message: Message,
    socket: &UnixDatagram,
    dp_addr: &SocketAddr,
) -> anyhow::Result<()> {
    println!("Message: {:?}", message);
    match message {
        Message::QueryMessage => handle_query_message(state, socket, dp_addr),
        Message::DownloadProcessMessage(message) => handle_download_process_message(state, message),
    }
}

fn handle_download_process_message(
    state: &State,
    message: DownloadProcessMessage,
) -> anyhow::Result<()> {
    let mut state = state.lock().expect("lock to work");

    let mut handle_exit = |pid: ProcessId, ecode: i32| {
        let _dlinfo = state.remove(&pid);
        if ecode != 0 {
            return Err(anyhow::anyhow!(
                "Download Process exited with exit code {}",
                ecode
            ));
        }
        Ok(())
    };

    match message.payload {
        super::MessagePayload::YtdlLine(line) => match line {
            super::ytdl_line::YtdlLine::VideoUrl(url) => {
                // TODO: maybe get angry here if pid is already tracked?
                state.insert(message.pid, DownloadInfo::UrlOnly(UrlOnly { url }));
            }
            super::ytdl_line::YtdlLine::VideoDownloadPath(path_string) => {
                let dlinfo = state.remove(&message.pid).ok_or(anyhow::anyhow!(
                    "Received VideoDownloadPath for a download which was not tracked"
                ))?;
                let DownloadInfo::UrlOnly(url) = dlinfo else {
                    return Err(anyhow::anyhow!(
                        "Received VideoDownloadPath while DownloadInfo wasn't UrlOnly"
                    ));
                };
                let metadata = url.create_metadata(path_string)?;
                state.insert(message.pid, DownloadInfo::MetadataOnly(metadata));
            }
            super::ytdl_line::YtdlLine::VideoDownloadProgress(progress) => {
                let dlinfo = state.remove(&message.pid).ok_or(anyhow::anyhow!(
                    "Received VideoDownloadProgress for a download which was not tracked"
                ))?;
                let full_dlinfo = match dlinfo {
                    DownloadInfo::UrlOnly(_) => {
                        return Err(anyhow::anyhow!(
                            "Received VideoDownloadProgress before VideoDownloadPath"
                        ))
                    }
                    DownloadInfo::MetadataOnly(metadata) => {
                        metadata.create_full_download_info(progress)
                    }
                    DownloadInfo::Full(mut full_dlinfo) => {
                        full_dlinfo.update_progress(progress);
                        full_dlinfo
                    }
                };
                state.insert(message.pid, DownloadInfo::Full(full_dlinfo));
            }
            super::ytdl_line::YtdlLine::VideoDownloadDone => {
                // dlinfo should not be removed yet, in case audio is extracted later
                let dlinfo = state.remove(&message.pid).ok_or(anyhow::anyhow!(
                    "Received VideoDownloadDone for a download which was not tracked"
                ))?;
                let DownloadInfo::Full(mut full_dlinfo) = dlinfo else {
                    return Err(anyhow::anyhow!(
                        "Received VideoDownloadDone before VideoDownloadProgress"
                    ));
                };
                full_dlinfo.set_as_completed();
                state.insert(message.pid, DownloadInfo::Full(full_dlinfo));
            }
            super::ytdl_line::YtdlLine::VideoDownloadError(_) => {
                let _dlinfo = state.remove(&message.pid);
            }
            super::ytdl_line::YtdlLine::VideoExtractAudio(_) => {
                let dlinfo = state.remove(&message.pid).ok_or(anyhow::anyhow!(
                    "Received VideoExtractAudio for a download which was not tracked"
                ))?;
                let DownloadInfo::Full(mut full_dlinfo) = dlinfo else {
                    return Err(anyhow::anyhow!(
                        "Received VideoExtractAudio before VideoDownloadProgress"
                    ));
                };
                if !full_dlinfo.is_completed() {
                    return Err(anyhow::anyhow!(
                        "Received VideoExtractAudio for a download which is not completed"
                    ));
                }
                full_dlinfo.set_as_extracting();
                state.insert(message.pid, DownloadInfo::Full(full_dlinfo));
            }
            super::ytdl_line::YtdlLine::Exit(ecode) => handle_exit(message.pid, ecode)?,
            // TODO: playlists later
            super::ytdl_line::YtdlLine::PlaylistUrl(_) => todo!(),
            super::ytdl_line::YtdlLine::PlaylistName(_) => todo!(),
            super::ytdl_line::YtdlLine::PlaylistVideoCount(_) => todo!(),
            super::ytdl_line::YtdlLine::PlaylistVideoIndex(_) => todo!(),
            super::ytdl_line::YtdlLine::PlaylistDownloadDone => todo!(),
        },
        super::MessagePayload::ProcessExited(ecode) => handle_exit(message.pid, ecode)?,
    }

    Ok(())
}

fn start_socket() -> anyhow::Result<UnixDatagram> {
    let _ = fs::remove_file(SYSTEM_ATLAS.ytdl_aggregator_socket);
    let listener = UnixDatagram::bind(SYSTEM_ATLAS.ytdl_aggregator_socket)?;
    Ok(listener)
}

/// Launch a daemon process, which maintains a map of ongoing ytdl downloads
pub fn run() -> anyhow::Result<()> {
    let state: State = Arc::new(Mutex::new(BTreeMap::new()));
    let socket = start_socket()?;

    // TODO: what would be the optimal size?
    let mut buf = vec![0; 1024];

    loop {
        match socket.recv_from(buf.as_mut_slice()) {
            Ok((n, dp_addr)) => {
                println!("Aggregator: Received {n} bytes");

                match serde_json::from_slice::<Message>(&buf[0..n]) {
                    Ok(message) => {
                        let _ = handle_message(&state, message, &socket, &dp_addr);
                    }
                    Err(e) => eprintln!("{e:?}"),
                };
            }
            Err(e) => return Err(e.into()),
        }
    }
}

#[cfg(test)]
#[allow(clippy::declare_interior_mutable_const)]
#[allow(clippy::borrow_interior_mutable_const)]
mod tests {
    use std::cell::LazyCell;

    use crate::subcommands::ytdl::ytdl_line::{DownloadProgress, YtdlLine};
    use crate::subcommands::ytdl::MessagePayload;

    use super::*;

    const OTHER_ADDR: LazyCell<SocketAddr> =
        LazyCell::new(|| SocketAddr::from_pathname("/some/path").unwrap());

    fn create_message(pid: u32, mut message: DownloadProcessMessage) -> Message {
        message.pid = pid;
        Message::DownloadProcessMessage(message)
    }

    fn create_messages(pid: u32, messages: &mut [DownloadProcessMessage]) -> Vec<Message> {
        for message in messages.iter_mut() {
            message.pid = pid;
        }
        messages
            .iter()
            .cloned()
            .map(Message::DownloadProcessMessage)
            .collect()
    }

    fn msg(line: YtdlLine) -> DownloadProcessMessage {
        DownloadProcessMessage {
            pid: 0,
            payload: MessagePayload::YtdlLine(line),
        }
    }

    fn url(url: &str) -> DownloadProcessMessage {
        msg(YtdlLine::VideoUrl(url.into()))
    }

    fn path(path: &str) -> DownloadProcessMessage {
        msg(YtdlLine::VideoDownloadPath(path.into()))
    }

    fn progress(
        percent: u32,
        total_size: u32,
        download_speed: u32,
        eta: u32,
    ) -> DownloadProcessMessage {
        msg(YtdlLine::VideoDownloadProgress(Progress::Downloading(
            DownloadProgress {
                percent,
                total_size,
                download_speed,
                eta,
            },
        )))
    }

    fn done() -> DownloadProcessMessage {
        msg(YtdlLine::VideoDownloadDone)
    }

    fn exited() -> DownloadProcessMessage {
        DownloadProcessMessage {
            pid: 0,
            payload: MessagePayload::ProcessExited(0),
        }
    }

    #[test]
    fn processing_single_download() {
        let socket = UnixDatagram::unbound().unwrap();
        let state: State = Arc::new(Mutex::new(BTreeMap::new()));
        let messages = create_messages(
            42,
            &mut [
                url("url"),
                path("path"),
                progress(1, 100, 20, 100),
                progress(20, 100, 40, 70),
                progress(60, 100, 40, 30),
                progress(100, 100, 40, 30),
                done(),
            ],
        );

        let last_message = create_messages(42, &mut [exited()]).pop().unwrap();

        for message in messages {
            let _ = handle_message(&state, message, &socket, &OTHER_ADDR);
        }

        assert_eq!(state.lock().unwrap().len(), 1);

        let _ = handle_message(&state, last_message, &socket, &OTHER_ADDR);

        assert_eq!(state.lock().unwrap().len(), 0);
    }

    #[test]
    fn processing_two_interleaved_downloads() {
        let socket = UnixDatagram::unbound().unwrap();
        let state: State = Arc::new(Mutex::new(BTreeMap::new()));
        let messages1 = vec![
            create_message(42, url("url")),
            create_message(43, url("url2")),
            create_message(42, path("path")),
            create_message(42, progress(1, 100, 20, 100)),
            create_message(43, path("path2")),
        ];

        let messages2 = vec![
            create_message(42, progress(20, 100, 40, 70)),
            create_message(42, progress(60, 100, 40, 30)),
            create_message(43, progress(0, 200, 10, 100)),
            create_message(43, progress(10, 200, 10, 90)),
            create_message(43, progress(20, 200, 10, 80)),
            create_message(42, progress(100, 100, 40, 30)),
            create_message(42, done()),
            create_message(43, progress(30, 200, 10, 70)),
            create_message(43, progress(40, 200, 10, 60)),
            create_message(43, progress(50, 200, 10, 50)),
            create_message(43, progress(60, 200, 10, 40)),
            create_message(42, exited()),
        ];

        let messages3 = vec![
            create_message(43, progress(70, 200, 10, 30)),
            create_message(43, progress(80, 200, 10, 20)),
            create_message(43, progress(90, 200, 10, 10)),
            create_message(43, progress(100, 200, 10, 0)),
            create_message(43, done()),
            create_message(43, exited()),
        ];

        for message in messages1 {
            let _ = handle_message(&state, message, &socket, &OTHER_ADDR);
        }

        assert_eq!(state.lock().unwrap().len(), 2);

        for message in messages2 {
            let _ = handle_message(&state, message, &socket, &OTHER_ADDR);
        }

        assert_eq!(state.lock().unwrap().len(), 1);

        for message in messages3 {
            let _ = handle_message(&state, message, &socket, &OTHER_ADDR);
        }

        assert_eq!(state.lock().unwrap().len(), 0);
    }

    #[test]
    #[should_panic]
    fn processing_out_of_order_path_message_fails() {
        let socket = UnixDatagram::unbound().unwrap();
        let state: State = Arc::new(Mutex::new(BTreeMap::new()));
        let messages = create_messages(42, &mut [path("path")]);

        handle_message(&state, messages[0].clone(), &socket, &OTHER_ADDR).unwrap();
    }
}

use std::{
    collections::BTreeMap,
    os::unix::net::UnixListener,
    path::Path,
    sync::{Arc, Mutex},
};

use crate::{subcommands::ytdl::Message, system_atlas::SYSTEM_ATLAS};

use super::ytdl_line::Progress;

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
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

type ProcessId = u32;
type State = Arc<Mutex<BTreeMap<ProcessId, DownloadInfo>>>;

enum DownloadInfo {
    UrlOnly(UrlOnly),
    MetadataOnly(DownloadMetadata),
    Full(FullDownloadInfo),
}

fn process_message(state: &State, message: Message) -> anyhow::Result<()> {
    let mut state = state.lock().expect("lock to work");

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
            // TODO: playlists later
            super::ytdl_line::YtdlLine::PlaylistUrl(_) => todo!(),
            super::ytdl_line::YtdlLine::PlaylistName(_) => todo!(),
            super::ytdl_line::YtdlLine::PlaylistVideoCount(_) => todo!(),
            super::ytdl_line::YtdlLine::PlaylistVideoIndex(_) => todo!(),
            super::ytdl_line::YtdlLine::PlaylistDownloadDone => todo!(),
        },
        super::MessagePayload::ProcessExited(ecode) => {
            let _dlinfo = state.remove(&message.pid);
            if ecode != 0 {
                return Err(anyhow::anyhow!(
                    "DownloadScript process exited with exit code {}",
                    ecode
                ));
            }
        }
    }

    Ok(())
}

/// Launch a daemon process, which maintains a map of ongoing ytdl downloads
pub fn run() -> anyhow::Result<()> {
    let state: State = Arc::new(Mutex::new(BTreeMap::new()));
    let listener = UnixListener::bind(SYSTEM_ATLAS.ytdl_aggregator_socket)?;

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let message: Message = serde_json::from_reader(stream)?;
                let _ = process_message(&state, message);
            }
            Err(err) => break,
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn process_message_for_single_video_works() {
        let state: State = Arc::new(Mutex::new(BTreeMap::new()));
        let messages: Vec<Message> = vec![];

        for message in messages {
            let _ = process_message(&state, message);
        }
    }
}

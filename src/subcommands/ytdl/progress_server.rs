use std::{
    collections::BTreeMap,
    os::unix::net::UnixListener,
    sync::{Arc, Mutex},
};

use super::{DownloadMetadata, DownloadProgress};

// TODO: serialization over the unix socket (serde? some binary format or json?)
// TODO: prepare progress server to be a systemd service? logging and stuff?
// TODO: pretty printing progress to console?

const SOCKET_PATH: &str = "/tmp/plsdo-ytdl-progress-server.sock";

type ProcessId = u32;

enum Message {
    BeginDownload(ProcessId, DownloadMetadata),
    ReportDownloadProgress(ProcessId, DownloadProgress),
    GetDownloadProgress,
}

/// Launch a daemon process, which maintains a map of ongoing ytdl downloads
pub fn run_progress_server() -> std::io::Result<()> {
    let download_progresses: Arc<Mutex<BTreeMap<ProcessId, (DownloadMetadata, DownloadProgress)>>>;
    let listener = UnixListener::bind(SOCKET_PATH)?;

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => println!("{:?}", stream),
            Err(err) => break,
        }
    }

    Ok(())
}

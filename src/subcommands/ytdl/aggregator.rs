use std::{
    collections::BTreeMap,
    io::Read,
    os::unix::net::UnixListener,
    sync::{Arc, Mutex},
};

use crate::{subcommands::ytdl::Message, system_atlas::SYSTEM_ATLAS};

use super::{ytdl_line::DownloadProgress, DownloadMetadata};

// TODO: serialization over the unix socket (serde? some binary format or json?)
// TODO: prepare progress server to be a systemd service? logging and stuff?
// TODO: pretty printing progress to console?

type ProcessId = u32;

/// Launch a daemon process, which maintains a map of ongoing ytdl downloads
pub fn run() -> std::io::Result<()> {
    let download_progress_map: Arc<
        Mutex<BTreeMap<ProcessId, (DownloadMetadata, DownloadProgress)>>,
    >;
    let listener = UnixListener::bind(SYSTEM_ATLAS.ytdl_aggregator_socket)?;

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let message: Message = serde_json::from_reader(stream)?;
                println!("{:?}", message);
            }
            Err(err) => break,
        }
    }

    Ok(())
}

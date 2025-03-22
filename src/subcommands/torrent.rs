use clap::{ArgMatches, Command};
use transmission_rpc::{
    types::{TorrentGetField, TorrentStatus},
    TransClient,
};
use url::Url;
use xshell::Shell;

pub fn command_extension(cmd: Command) -> Command {
    cmd
}

pub fn run(_: &Shell, _args: &ArgMatches) -> anyhow::Result<Option<String>> {
    let async_runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    let url = Url::parse("http://localhost:9091/transmission/rpc")?;
    let mut client = TransClient::new(url);
    let torrents_future = client.torrent_get(
        Some(vec![TorrentGetField::Status, TorrentGetField::Eta]),
        None,
    );
    let torrents = async_runtime
        .block_on(torrents_future)
        .map_err(anyhow::Error::from_boxed)?
        .arguments
        .torrents;

    let total_eta: Option<i64> = torrents
        .iter()
        .filter(|&t| t.status == Some(TorrentStatus::Downloading))
        .map(|t| t.eta)
        .sum();

    if let Some(total_eta) = total_eta {
        if total_eta > 0 {
            let eta_minutes = ((total_eta as f64) / 60f64).ceil();
            println!("{}m", eta_minutes);
            return Ok(None);
        }
    }

    let some_uploading = torrents
        .iter()
        .filter(|&t| t.status == Some(TorrentStatus::Seeding))
        .count();

    if some_uploading > 0 {
        println!("up");
    }

    Ok(None)
}

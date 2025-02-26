use clap::ArgMatches;

use anyhow::Context;

use crate::util::listener::{get_pidfile_lock, write_pid};

const PIDFILE: &str = "/tmp/plsdo-audio-device-listener.pid";

pub fn run(_args: &ArgMatches) -> anyhow::Result<()> {
    let mut lock = get_pidfile_lock(PIDFILE)?;
    let mut guard = lock
        .try_write()
        .context("The listener is already running")?;
    write_pid(&mut guard)?;

    println!("Hello from audio listener!");

    Ok(())
}

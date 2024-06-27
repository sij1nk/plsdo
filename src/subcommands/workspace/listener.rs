use std::fs::OpenOptions;
use std::io::prelude::Write;

use anyhow::Context;
use clap::ArgMatches;
use fd_lock::{RwLock, RwLockWriteGuard};

const PIDFILE: &str = "/tmp/plsdo-hypr-workspace-listener.pid";

fn get_pidfile_lock() -> anyhow::Result<RwLock<std::fs::File>> {
    let pidfile = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(false)
        .open(PIDFILE)?;
    Ok(RwLock::new(pidfile))
}

fn write_pid(guard: &mut RwLockWriteGuard<'_, std::fs::File>) -> anyhow::Result<()> {
    let pid_string = format!("{}", std::process::id());
    Ok(write!(guard, "{pid_string}")?)
}

pub fn run(_args: &ArgMatches) -> anyhow::Result<()> {
    let mut lock = get_pidfile_lock()?;
    let mut guard = lock
        .try_write()
        .context("The listener is already running")?;
    write_pid(&mut guard)?;

    let mut i = 0;
    loop {
        println!("Hello from workspace listener! {i}");
        std::thread::sleep(std::time::Duration::from_secs(1));
        i += 1;
    }
}

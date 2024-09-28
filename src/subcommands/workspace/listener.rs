use std::fs::OpenOptions;
use std::io::prelude::Write;

use anyhow::Context;
use clap::ArgMatches;
use fd_lock::{RwLock, RwLockWriteGuard};
use hyprland::shared::WorkspaceType;

use super::{update_system_bar_layout, write_workspace_state_to_backing_file};

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

fn handle_workspace_changed_event(_data: WorkspaceType) {
    if let Err(e) = write_workspace_state_to_backing_file() {
        eprintln!("Failed to write to backing file: {}", e);
    }
}

fn handle_monitor_added_or_removed_event(_monitor_name: String) {
    if let Err(e) = update_system_bar_layout() {
        eprintln!("Failed to handle monitor added event: {}", e);
    };
    if let Err(e) = write_workspace_state_to_backing_file() {
        eprintln!("Failed to write to backing file: {}", e);
    }
}

pub fn run(_args: &ArgMatches) -> anyhow::Result<()> {
    let mut lock = get_pidfile_lock()?;
    let mut guard = lock
        .try_write()
        .context("The listener is already running")?;
    write_pid(&mut guard)?;

    let mut listener = hyprland::event_listener::EventListener::new();

    listener.add_workspace_change_handler(handle_workspace_changed_event);
    listener.add_monitor_added_handler(handle_monitor_added_or_removed_event);
    listener.add_monitor_removed_handler(handle_monitor_added_or_removed_event);

    listener.start_listener()?;

    Ok(())
}

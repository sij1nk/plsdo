use std::io::prelude::Write;
use std::{fs::OpenOptions, io::LineWriter};

use anyhow::Context;
use clap::ArgMatches;
use fd_lock::{RwLock, RwLockWriteGuard};
use hyprland::shared::WorkspaceType;

use crate::system_atlas::SYSTEM_ATLAS;

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

fn write_submap_to_backing_file(submap_name: String) -> anyhow::Result<()> {
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(SYSTEM_ATLAS.hypr_submap)?;

    let mut writer = LineWriter::new(&file);
    writer.write_fmt(format_args!("{}\n", submap_name))?;

    Ok(())
}

fn handle_submap_change_event(submap_name: String) {
    if let Err(e) = write_submap_to_backing_file(submap_name) {
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
    listener.add_sub_map_change_handler(handle_submap_change_event);

    listener.start_listener()?;

    Ok(())
}

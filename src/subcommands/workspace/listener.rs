use std::{
    fs::OpenOptions,
    io::{LineWriter, Write},
};

use anyhow::Context;
use clap::ArgMatches;
use hyprland::event_listener::{MonitorAddedEventData, WorkspaceEventData};

use crate::{
    system_atlas::SYSTEM_ATLAS,
    util::listener::{get_pidfile_lock, write_pid},
};

use super::{update_system_bar_layout, write_workspace_state_to_backing_file};

const PIDFILE: &str = "/tmp/plsdo-hypr-workspace-listener.pid";

fn handle_workspace_changed_event(_data: WorkspaceEventData) {
    if let Err(e) = write_workspace_state_to_backing_file() {
        eprintln!("Failed to write to backing file: {}", e);
    }
}

fn handle_monitor_added_event(_data: MonitorAddedEventData) {
    if let Err(e) = update_system_bar_layout() {
        eprintln!("Failed to handle monitor added event: {}", e);
    };
    if let Err(e) = write_workspace_state_to_backing_file() {
        eprintln!("Failed to write to backing file: {}", e);
    }
}

fn handle_monitor_removed_event(_monitor_name: String) {
    if let Err(e) = update_system_bar_layout() {
        eprintln!("Failed to handle monitor added event: {}", e);
    };
    if let Err(e) = write_workspace_state_to_backing_file() {
        eprintln!("Failed to write to backing file: {}", e);
    }
}

pub fn write_submap_to_backing_file(submap_name: String) -> anyhow::Result<()> {
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
    let mut lock = get_pidfile_lock(PIDFILE)?;
    let mut guard = lock
        .try_write()
        .context("The listener is already running")?;
    write_pid(&mut guard)?;

    let mut listener = hyprland::event_listener::EventListener::new();

    listener.add_workspace_changed_handler(handle_workspace_changed_event);
    listener.add_monitor_added_handler(handle_monitor_added_event);
    listener.add_monitor_removed_handler(handle_monitor_removed_event);
    listener.add_sub_map_changed_handler(handle_submap_change_event);

    listener.start_listener()?;

    Ok(())
}

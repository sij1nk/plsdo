use std::io::prelude::Write;
use std::io::LineWriter;
use std::{collections::BTreeSet, fs::OpenOptions};

use anyhow::Context;
use clap::ArgMatches;
use fd_lock::{RwLock, RwLockWriteGuard};
use hyprland::data::Clients;
use hyprland::shared::{HyprData, WorkspaceType};

use crate::system_atlas::SYSTEM_ATLAS;

use super::{get_active_workspace_ids, WorkspaceId};

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

#[derive(Debug)]
struct OccupiedWorkspaceIds {
    inner: BTreeSet<WorkspaceId>,
}

impl FromIterator<WorkspaceId> for OccupiedWorkspaceIds {
    fn from_iter<T: IntoIterator<Item = WorkspaceId>>(iter: T) -> Self {
        Self {
            inner: BTreeSet::from_iter(iter),
        }
    }
}

impl std::fmt::Display for OccupiedWorkspaceIds {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let len = self.inner.len();
        if len == 0 {
            return Ok(());
        }

        let mut iter = self.inner.iter();
        for _ in 0..len - 1 {
            write!(
                f,
                "{},",
                iter.next()
                    .expect("The item should exist, because we're within bounds")
            )?;
        }
        write!(f, "{}", iter.next().expect("The last item should exist"))?;

        Ok(())
    }
}

/// Append the state of the workspaces to a file, from which the eww workspaces widget can read it
/// from. The output should be the following JSON array:
/// `[<active_secondary_workspace>,<active_primary_workspace>,[<occupied_workspace>...]]`
/// Odd numbered workspaces belong to the primary monitor; even numbered workspaces belong to the
/// secondary one.
fn write_workspace_state_to_backing_file(_data: WorkspaceType) -> anyhow::Result<()> {
    let (primary_active_id, secondary_active_id) = get_active_workspace_ids()?;

    let occupied_workspace_ids = Clients::get()?
        .into_iter()
        .map(|cl| cl.workspace.id)
        .filter(|&id| id > 0)
        .collect::<OccupiedWorkspaceIds>();

    let file = OpenOptions::new()
        .create(false)
        .append(true)
        .open(SYSTEM_ATLAS.eww_workspaces)?;
    let mut writer = LineWriter::new(&file);
    writeln!(
        writer,
        "[{},{},[{}]]",
        secondary_active_id, primary_active_id, occupied_workspace_ids
    )?;

    Ok(())
}

pub fn run(_args: &ArgMatches) -> anyhow::Result<()> {
    let mut lock = get_pidfile_lock()?;
    let mut guard = lock
        .try_write()
        .context("The listener is already running")?;
    write_pid(&mut guard)?;

    let mut listener = hyprland::event_listener::EventListener::new();
    listener.add_workspace_change_handler(|data| {
        if let Err(e) = write_workspace_state_to_backing_file(data) {
            eprintln!("Failed to write to backing file: {}", e);
        }
    });
    listener.start_listener()?;

    Ok(())
}

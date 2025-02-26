//! A listener is a daemonized operation, that runs in the background and reacts to events.
//!
//! Having more than one instance of the same listener running at the same time will result
//! in multiple reactions to the same event. This is unnecessary, and may even be harmful in
//! some cases, so we should ensure that the a listener can't be launched if an instance
//! of it is already running.

use std::fs::OpenOptions;
use std::io::Write;

use fd_lock::{RwLock, RwLockWriteGuard};

/// Create an `RwLock` on the file. The listener must obtain an `RwLockWriteGuard`,
/// and hold it for the entire duration of its runtime.
///
/// * `pidfile_path`: path to the pidfile
pub fn get_pidfile_lock(pidfile_path: &str) -> anyhow::Result<RwLock<std::fs::File>> {
    let pidfile = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(false)
        .open(pidfile_path)?;
    Ok(RwLock::new(pidfile))
}

/// Write the current process' pid into the pidfile.
///
/// * `guard`: the `RwLockWriteGuard` for the pidfile
pub fn write_pid(guard: &mut RwLockWriteGuard<'_, std::fs::File>) -> anyhow::Result<()> {
    let pid_string = format!("{}", std::process::id());
    Ok(write!(guard, "{pid_string}")?)
}

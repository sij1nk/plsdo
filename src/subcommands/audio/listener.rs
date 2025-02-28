use std::ffi::OsStr;

use clap::ArgMatches;
use mio::{Events, Interest, Poll, Token};

use anyhow::Context;
use udev::Device;
use xshell::Shell;

use crate::{
    constants::EARBUDS_NAME,
    util::listener::{get_pidfile_lock, write_pid},
};

use super::state::{
    find_matching_output, get_all_audio_outputs, get_current_audio_state, set_audio_output,
    write_to_backing_file,
};

const PIDFILE: &str = "/tmp/plsdo-audio-device-listener.pid";

fn get_property_value(device: &Device, property: impl AsRef<OsStr>) -> Option<&str> {
    device.property_value(property).and_then(|s| s.to_str())
}

fn handle_udev_event(event: udev::Event) -> anyhow::Result<()> {
    let device = event.device();

    let action = get_property_value(&device, "ACTION");
    let Some("remove") = action else {
        anyhow::bail!("Not a remove event")
    };

    let name = get_property_value(&device, "NAME");
    let is_earbuds = name.is_some_and(|n| n.trim_start_matches('"').starts_with(EARBUDS_NAME));

    if is_earbuds {
        let sh = Shell::new()?;
        let outputs = get_all_audio_outputs(&sh)?;
        let matching_output = find_matching_output(&outputs, "Headphones")?;
        set_audio_output(&sh, &matching_output.name)?;

        let audio_state = get_current_audio_state(&sh)?;
        write_to_backing_file(audio_state)?;
    }

    Ok(())
}

pub fn run(_args: &ArgMatches) -> anyhow::Result<()> {
    let mut lock = get_pidfile_lock(PIDFILE)?;
    let mut guard = lock
        .try_write()
        .context("The listener is already running")?;
    write_pid(&mut guard)?;

    let mut socket = udev::MonitorBuilder::new()?.listen()?;

    let mut poll = Poll::new()?;
    let mut events = Events::with_capacity(16);
    let source_token = Token(0);

    poll.registry()
        .register(&mut socket, source_token, Interest::READABLE)?;

    loop {
        poll.poll(&mut events, None)?;

        for event in &events {
            if event.token() == source_token && event.is_readable() {
                socket.iter().for_each(|ev| {
                    if let Err(err) = handle_udev_event(ev) {
                        eprintln!("{}", err);
                    }
                });
            }
        }
    }
}

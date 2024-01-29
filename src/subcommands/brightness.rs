use std::time::Duration;

use clap::{arg, value_parser, ArgMatches, Command, ValueEnum};
use dbus::blocking::Connection;
use xshell::Shell;

use crate::system_atlas::SystemAtlas;

#[derive(ValueEnum, Clone, Debug)]
enum Direction {
    Up,
    Down,
}

pub fn command_extension(cmd: Command) -> Command {
    cmd.arg(
        arg!(-d --direction <DIRECTION>)
            .value_parser(value_parser!(Direction))
            .required(true),
    )
    .arg(
        arg!([DELTA])
            .value_parser(value_parser!(i32).range(1..))
            .required(true),
    )
}

pub fn run(sh: &Shell, args: &ArgMatches, atlas: &SystemAtlas) -> anyhow::Result<()> {
    let direction = args
        .get_one::<Direction>("direction")
        .expect("Direction argument is required");
    let delta = *args
        .get_one::<i32>("DELTA")
        .expect("DELTA argument is required");

    let signed_delta: f64 = match direction {
        Direction::Up => delta as f64 / 100.0,
        Direction::Down => -delta as f64 / 100.0,
    };

    let connection = Connection::new_session()?;
    let proxy = connection.with_proxy("rs.wl-gammarelay", "/", Duration::from_secs(1));
    proxy.method_call("rs.wl.gammarelay", "UpdateBrightness", (signed_delta,))?;

    Ok(())
}

use std::fs::OpenOptions;
use std::io::{LineWriter, Write};
use std::time::Duration;

use clap::{arg, value_parser, ArgMatches, Command, ValueEnum};
use dbus::{arg, blocking::Connection};
use xshell::Shell;

use crate::system_atlas::SYSTEM_ATLAS;

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

fn determine_delta(args: &ArgMatches) -> anyhow::Result<f64> {
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

    Ok(signed_delta)
}

fn write_brightness_to_backing_file(brightness: f64) -> anyhow::Result<()> {
    let file = OpenOptions::new()
        .create(false)
        .append(true)
        .open(SYSTEM_ATLAS.eww_brightness)?;
    let mut writer = LineWriter::new(&file);
    writeln!(writer, "{}", brightness.round())?;

    Ok(())
}

pub fn run(_: &Shell, args: &ArgMatches) -> anyhow::Result<()> {
    let signed_delta = determine_delta(args)?;

    let connection = Connection::new_session()?;
    let proxy = connection.with_proxy("rs.wl-gammarelay", "/", Duration::from_secs(1));

    proxy.method_call("rs.wl.gammarelay", "UpdateBrightness", (signed_delta,))?;
    use dbus::blocking::stdintf::org_freedesktop_dbus::Properties;

    let brightness_refarg: Box<dyn arg::RefArg> = proxy.get("rs.wl.gammarelay", "Brightness")?;
    let brightness = 100.0
        * brightness_refarg.as_f64().ok_or(anyhow::anyhow!(
            "rs.wl.gammarelay.Brightness is not an f64 value"
        ))?;

    write_brightness_to_backing_file(brightness)?;

    Ok(())
}

use clap::{arg, value_parser, ArgMatches, Command, ValueEnum};
use dbus::{arg, blocking::Connection};
use std::fs::OpenOptions;
use std::io::{LineWriter, Write};
use std::time::Duration;
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
            .value_parser(value_parser!(i16).range(1..))
            .required(true),
    )
}

fn determine_delta(args: &ArgMatches) -> anyhow::Result<i16> {
    let direction = args
        .get_one::<Direction>("direction")
        .expect("Direction argument is required");
    let delta = *args
        .get_one::<i16>("DELTA")
        .expect("DELTA argument is required");

    let signed_delta: i16 = match direction {
        Direction::Up => delta,
        Direction::Down => -delta,
    };

    Ok(signed_delta)
}

fn write_colortemp_to_backing_file(colortemp: u16) -> anyhow::Result<()> {
    let file = OpenOptions::new()
        .create(false)
        .append(true)
        .open(SYSTEM_ATLAS.eww_colortemp)?;
    let mut writer = LineWriter::new(&file);
    writeln!(writer, "{}", colortemp)?;

    Ok(())
}

pub fn run(_: &Shell, args: &ArgMatches) -> anyhow::Result<()> {
    let delta = determine_delta(args)?;

    let connection = Connection::new_session()?;
    let proxy = connection.with_proxy("rs.wl-gammarelay", "/", Duration::from_secs(1));

    proxy.method_call("rs.wl.gammarelay", "UpdateTemperature", (delta,))?;
    use dbus::blocking::stdintf::org_freedesktop_dbus::Properties;

    let colortemp_refarg: Box<dyn arg::RefArg> = proxy.get("rs.wl.gammarelay", "Temperature")?;
    let colortemp = colortemp_refarg.as_u64().ok_or(anyhow::anyhow!(
        "rs.wl.gammarelay.Temperature is not an unsigned value"
    ))?;

    write_colortemp_to_backing_file(colortemp as u16)?;

    Ok(())
}

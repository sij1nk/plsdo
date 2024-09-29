use std::{fs::OpenOptions, io::LineWriter, io::Write};

use clap::{arg, value_parser, ArgMatches, Command, ValueEnum};
use xshell::{cmd, Shell};

use crate::system_atlas::SYSTEM_ATLAS;

#[derive(Debug, Clone)]
struct Volume {
    value: u32,
    is_muted: bool,
}

#[derive(ValueEnum, Clone, Debug)]
enum Direction {
    Up,
    Down,
}

const SINK: &str = "@DEFAULT_SINK@";

pub fn command_extension(cmd: Command) -> Command {
    let inner_subcommands = [
        Command::new("set")
            .about("Set the audio volume")
            .arg(
                arg!(-d --direction <DIRECTION>)
                    .value_parser(value_parser!(Direction))
                    .required(true),
            )
            .arg(
                arg!([DELTA])
                    .value_parser(value_parser!(i16).range(1..))
                    .required(true),
            ),
        Command::new("toggle-mute").about("Toggle audio mute"),
    ];

    cmd.subcommand_required(true)
        .arg_required_else_help(true)
        .subcommands(inner_subcommands.iter())
}

fn write_volume_to_backing_file(volume: Volume) -> anyhow::Result<()> {
    let file = OpenOptions::new()
        .create(false)
        .append(true)
        .open(SYSTEM_ATLAS.eww_volume)?;
    let mut writer = LineWriter::new(&file);

    let is_muted_str = if volume.is_muted { "muted" } else { "unmuted" };

    writeln!(writer, "[{},\"{}\"]", volume.value, is_muted_str)?;

    Ok(())
}

fn toggle_mute(sh: &Shell) -> anyhow::Result<()> {
    Ok(cmd!(sh, "pactl set-sink-mute {SINK} toggle").run()?)
}

fn set_volume(sh: &Shell, delta: i16) -> anyhow::Result<()> {
    let delta_str = format!("{:+}", delta);
    cmd!(sh, "pactl set-sink-volume {SINK} {delta_str}%").run()?;
    Ok(())
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

fn get_current_volume(sh: &Shell) -> anyhow::Result<Volume> {
    let is_muted = cmd!(sh, "pactl get-sink-mute {SINK}")
        .read()?
        .split_once(' ')
        .ok_or(anyhow::anyhow!(
            "Got unexpected output from `pactl get-sink-mute"
        ))?
        .1
        == "yes";

    let volume_str = cmd!(sh, "pactl get-sink-volume {SINK}").read()?;
    let volume_percent = volume_str
        .split('/')
        .nth(1)
        .ok_or(anyhow::anyhow!(
            "Got unexpected output from `pactl get-sink-volume"
        ))?
        .trim();
    let volume = volume_percent[..volume_percent.len() - 1].parse::<u32>()?;

    Ok(Volume {
        value: volume,
        is_muted,
    })
}

pub fn run(sh: &Shell, args: &ArgMatches) -> anyhow::Result<Option<String>> {
    let volume = match args.subcommand() {
        Some(("set", set_args)) => {
            let delta = determine_delta(set_args)?;
            set_volume(sh, delta)?;
            Some(get_current_volume(sh)?)
        }
        Some(("toggle-mute", _)) => {
            toggle_mute(sh)?;
            Some(get_current_volume(sh)?)
        }
        _ => None,
    };

    if let Some(volume) = volume {
        write_volume_to_backing_file(volume)?;
    }

    Ok(None)
}

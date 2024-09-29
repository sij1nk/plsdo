use std::{fs::OpenOptions, io::LineWriter, io::Write};

use clap::{arg, value_parser, ArgMatches, Command, ValueEnum};
use serde::Serialize;
use xshell::{cmd, Shell};

use crate::system_atlas::SYSTEM_ATLAS;

#[derive(Serialize, Debug, Clone)]
struct AudioState {
    value: u32,
    is_muted: bool,
    output: AudioOutput,
}

#[derive(ValueEnum, Clone, Debug)]
enum Direction {
    Up,
    Down,
}

#[derive(ValueEnum, Serialize, Clone, Debug)]
enum AudioOutput {
    Headphones,
    Speakers,
}

const SINK: &str = "@DEFAULT_SINK@";

pub fn command_extension(cmd: Command) -> Command {
    let volume_subcommands = [
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

    let inner_subcommands = [
        Command::new("init").about("Initialize audio and audio-related information"),
        Command::new("output")
            .about("Change the audio output")
            .arg_required_else_help(true)
            .arg(arg!([OUTPUT] "The output device")),
        Command::new("volume")
            .about("Change the audio volume")
            .arg_required_else_help(true)
            .subcommand_required(true)
            .subcommands(volume_subcommands),
    ];

    cmd.subcommand_required(true)
        .arg_required_else_help(true)
        .subcommands(inner_subcommands.iter())
}

fn write_to_backing_file(audio_state: AudioState) -> anyhow::Result<()> {
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(SYSTEM_ATLAS.eww_audio)?;
    let mut writer = LineWriter::new(&file);

    serde_json::to_writer(&mut writer, &audio_state)?;
    writer.write_all(b"\n")?;

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

fn get_current_audio_state(sh: &Shell) -> anyhow::Result<AudioState> {
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

    Ok(AudioState {
        value: volume,
        is_muted,
        output: AudioOutput::Headphones,
    })
}

fn initialize(sh: &Shell) -> anyhow::Result<()> {
    let audio_state = get_current_audio_state(sh)?;
    write_to_backing_file(audio_state)?;

    Ok(())
}

pub fn run(sh: &Shell, args: &ArgMatches) -> anyhow::Result<Option<String>> {
    match args.subcommand() {
        Some(("init", _)) => initialize(sh),
        Some(("output", _output_args)) => Ok(()),
        Some(("volume", volume_args)) => {
            let audio_state = match volume_args.subcommand() {
                Some(("set", set_args)) => {
                    let delta = determine_delta(set_args)?;
                    set_volume(sh, delta)?;
                    Some(get_current_audio_state(sh)?)
                }
                Some(("toggle-mute", _)) => {
                    toggle_mute(sh)?;
                    Some(get_current_audio_state(sh)?)
                }
                _ => None,
            };

            if let Some(audio_state) = audio_state {
                write_to_backing_file(audio_state)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }?;

    Ok(None)
}

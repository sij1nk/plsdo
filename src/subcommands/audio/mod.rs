use std::{fs::OpenOptions, io::LineWriter, io::Write};

use clap::{arg, value_parser, ArgAction, ArgMatches, Command, ValueEnum};
use serde::{Deserialize, Serialize};
use xshell::{cmd, Shell};

use crate::{system_atlas::SYSTEM_ATLAS, util::dmenu::get_platform_dmenu};

#[derive(Serialize, Debug, Clone)]
struct AudioState {
    volume: u32,
    is_muted: bool,
    output: AudioOutputFriendlyName,
}

#[derive(ValueEnum, Serialize, Clone, Debug, PartialEq, Eq)]
enum AudioOutputFriendlyName {
    Headphones,
    Speakers,
    Unrecognized,
}

#[derive(Deserialize, Clone, Debug)]
struct PactlAudioSink {
    name: String,
    description: String,
}

#[derive(Serialize, Clone, Debug)]
struct AudioOutput {
    name: String,
    description: String,
    friendly_name: AudioOutputFriendlyName,
}

impl AudioOutput {
    fn is_matching(&self, needle: &str) -> bool {
        let is_type_matching = AudioOutputFriendlyName::from_str(needle, true)
            .map(|t| t == self.friendly_name)
            .unwrap_or(false);
        if is_type_matching {
            true
        } else {
            self.name.contains(needle) || self.description.contains(needle)
        }
    }
}

impl From<PactlAudioSink> for AudioOutput {
    fn from(sink: PactlAudioSink) -> Self {
        // TODO: this is not very elaborate... maybe there is a way to assign labels to sinks in pipewire?
        // I should look into that
        let output_type = if sink
            .description
            .starts_with("Family 17h/19h HD Audio Controller")
        {
            AudioOutputFriendlyName::Speakers
        } else if sink
            .description
            .starts_with("Navi 21/23 HDMI/DP Audio Controller")
        {
            AudioOutputFriendlyName::Headphones
        } else {
            AudioOutputFriendlyName::Unrecognized
        };

        Self {
            name: sink.name,
            description: sink.description,
            friendly_name: output_type,
        }
    }
}

const SINK: &str = "@DEFAULT_SINK@";

pub fn command_extension(cmd: Command) -> Command {
    let volume_subcommands = [
        Command::new("set")
            .about("Set the audio volume")
            .arg(
                arg!(-r --relative <IS_RELATIVE> "Treat value as relative")
                    .value_parser(value_parser!(bool))
                    .action(ArgAction::SetTrue)
                    .required(false),
            )
            .arg(
                arg!([VALUE])
                    .value_parser(value_parser!(i16))
                    .allow_negative_numbers(true)
                    .required(true),
            ),
        Command::new("toggle-mute").about("Toggle audio mute"),
    ];

    let output_subcommands = [
        Command::new("get").about("Get information about the current audio output"),
        Command::new("get-all").about("Get information about all audio outputs"),
        Command::new("set")
            .about("Set the current audio output")
            .arg(
            arg!([NEEDLE] "A string that matches the friendly name, name, description of the audio output")
                .required(true),
        ),
        Command::new("choose").about("Choose an audio output from the list of outputs"),
    ];

    let inner_subcommands = [
        Command::new("init").about("Initialize audio and audio-related information"),
        Command::new("output")
            .about("Change the audio output")
            .arg_required_else_help(true)
            .subcommand_required(true)
            .subcommands(output_subcommands),
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

fn set_volume(sh: &Shell, value: i16, is_relative: bool) -> anyhow::Result<()> {
    let value_str = if is_relative {
        format!("{:+}", value)
    } else {
        format!("{}", value)
    };
    cmd!(sh, "pactl set-sink-volume {SINK} {value_str}%").run()?;
    Ok(())
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

    let output = get_current_audio_output(sh)?;

    Ok(AudioState {
        volume,
        is_muted,
        output: output.friendly_name,
    })
}

fn initialize(sh: &Shell) -> anyhow::Result<()> {
    set_volume(sh, 25, false)?;
    let audio_state = get_current_audio_state(sh)?;
    write_to_backing_file(audio_state)
}

fn get_all_audio_outputs(sh: &Shell) -> anyhow::Result<Vec<AudioOutput>> {
    let sinks_json = cmd!(sh, "pactl -f json list sinks").read()?;
    let sinks: Vec<PactlAudioSink> = serde_json::from_str(&sinks_json)?;
    let outputs = sinks.into_iter().map(AudioOutput::from).collect::<Vec<_>>();

    Ok(outputs)
}

fn get_current_audio_output(sh: &Shell) -> anyhow::Result<AudioOutput> {
    let outputs = get_all_audio_outputs(sh)?;
    let current_output_name = cmd!(sh, "pactl get-default-sink").read()?;

    outputs
        .into_iter()
        .find(|o| o.name == current_output_name)
        .ok_or(anyhow::anyhow!(
            "Could not find current audio output by name"
        ))
}

fn set_audio_output(sh: &Shell, name: &str) -> anyhow::Result<()> {
    cmd!(sh, "pactl set-default-sink {name}").run()?;
    Ok(())
}

fn find_matching_output<'a>(
    outputs: &'a [AudioOutput],
    needle: &str,
) -> anyhow::Result<&'a AudioOutput> {
    let matching_outputs = outputs
        .iter()
        .filter(|o| o.is_matching(needle))
        .collect::<Vec<_>>();

    let len = matching_outputs.len();

    if len == 0 {
        anyhow::bail!("No output was found matching the needle '{}'", needle);
    }

    if len > 1 {
        anyhow::bail!(
            "Multiple outputs were found matching the needle '{}'",
            needle
        );
    }

    Ok(matching_outputs[0])
}

fn handle_output_subcommand(sh: &Shell, args: &ArgMatches) -> anyhow::Result<()> {
    match args.subcommand() {
        Some(("get", _)) => {
            let current_output = get_current_audio_output(sh)?;
            let current_output_json = serde_json::to_string_pretty(&current_output)?;
            println!("{}", current_output_json);
            Ok(())
        }
        Some(("get-all", _)) => {
            let outputs = get_all_audio_outputs(sh)?;
            let outputs_json = serde_json::to_string_pretty(&outputs)?;
            println!("{}", outputs_json);
            Ok(())
        }
        Some(("set", set_args)) => {
            let outputs = get_all_audio_outputs(sh)?;
            let needle = set_args
                .get_one::<String>("NEEDLE")
                .expect("NEEDLE should be a required argument");

            let matching_output = find_matching_output(&outputs, needle)?;

            set_audio_output(sh, &matching_output.name)
        }
        Some(("choose", _)) => {
            let outputs = get_all_audio_outputs(sh)?;
            let mut choices = outputs
                .iter()
                .map(|o| format!("{:?} | {}", o.friendly_name, o.description))
                .collect::<Vec<_>>();
            choices.sort();
            let choices_str = choices.iter().map(|e| e.as_ref()).collect::<Vec<&str>>();
            let result =
                get_platform_dmenu().choose_one(sh, "Choose audio output", &choices_str, true)?;

            let result_friendly_name = result
                .split_once("|")
                .expect("the seperator to be found")
                .0
                .trim();
            let matching_output = find_matching_output(&outputs, result_friendly_name)?;

            set_audio_output(sh, &matching_output.name)
        }
        _ => Ok(()),
    }
}

fn handle_volume_subcommand(sh: &Shell, args: &ArgMatches) -> anyhow::Result<()> {
    match args.subcommand() {
        Some(("set", set_args)) => {
            let is_relative = set_args
                .get_one::<bool>("relative")
                .copied()
                .unwrap_or(false);
            let value = *set_args
                .get_one::<i16>("VALUE")
                .expect("VALUE argument is required");

            if !is_relative && value < 0 {
                anyhow::bail!("Cannot use negative value in non-relative mode!");
            }

            set_volume(sh, value, is_relative)
        }
        Some(("toggle-mute", _)) => toggle_mute(sh),
        _ => Ok(()),
    }
}

pub fn run(sh: &Shell, args: &ArgMatches) -> anyhow::Result<Option<String>> {
    match args.subcommand() {
        Some(("init", _)) => initialize(sh),
        Some(("output", output_args)) => handle_output_subcommand(sh, output_args),
        Some(("volume", volume_args)) => handle_volume_subcommand(sh, volume_args),
        _ => Ok(()),
    }?;

    let audio_state = get_current_audio_state(sh)?;
    write_to_backing_file(audio_state)?;

    Ok(None)
}

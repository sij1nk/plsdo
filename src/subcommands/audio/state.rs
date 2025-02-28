use std::{
    fs::OpenOptions,
    io::{LineWriter, Write},
};

use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use xshell::{cmd, Shell};

use crate::{
    constants::{EARBUDS_NAME, HEADPHONES_CONTROLLER_NAME, SPEAKERS_CONTROLLER_NAME},
    system_atlas::SYSTEM_ATLAS,
};

const SINK: &str = "@DEFAULT_SINK@";

#[derive(Serialize, Debug, Clone)]
pub struct AudioState {
    volume: u32,
    is_muted: bool,
    output: AudioOutputFriendlyName,
}

#[derive(ValueEnum, Serialize, Clone, Debug, PartialEq, Eq)]
pub enum AudioOutputFriendlyName {
    Headphones,
    Speakers,
    Earbuds,
    Unrecognized,
}

#[derive(Deserialize, Clone, Debug)]
struct PactlAudioSink {
    name: String,
    description: String,
}

#[derive(Serialize, Clone, Debug)]
pub struct AudioOutput {
    pub name: String,
    pub description: String,
    pub friendly_name: AudioOutputFriendlyName,
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
        let output_type = if sink.description.starts_with(SPEAKERS_CONTROLLER_NAME) {
            AudioOutputFriendlyName::Speakers
        } else if sink.description.starts_with(HEADPHONES_CONTROLLER_NAME) {
            AudioOutputFriendlyName::Headphones
        } else if sink.description.starts_with(EARBUDS_NAME) {
            AudioOutputFriendlyName::Earbuds
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

pub fn write_to_backing_file(audio_state: AudioState) -> anyhow::Result<()> {
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(SYSTEM_ATLAS.eww_audio)?;
    let mut writer = LineWriter::new(&file);

    serde_json::to_writer(&mut writer, &audio_state)?;
    writer.write_all(b"\n")?;

    Ok(())
}

pub fn toggle_mute(sh: &Shell) -> anyhow::Result<()> {
    Ok(cmd!(sh, "pactl set-sink-mute {SINK} toggle").run()?)
}

pub fn set_volume(sh: &Shell, value: i16, is_relative: bool) -> anyhow::Result<()> {
    let value_str = if is_relative {
        format!("{:+}", value)
    } else {
        format!("{}", value)
    };
    cmd!(sh, "pactl set-sink-volume {SINK} {value_str}%").run()?;
    Ok(())
}

pub fn get_current_audio_state(sh: &Shell) -> anyhow::Result<AudioState> {
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

pub fn get_all_audio_outputs(sh: &Shell) -> anyhow::Result<Vec<AudioOutput>> {
    let sinks_json = cmd!(sh, "pactl -f json list sinks").read()?;
    let sinks: Vec<PactlAudioSink> = serde_json::from_str(&sinks_json)?;
    let outputs = sinks.into_iter().map(AudioOutput::from).collect::<Vec<_>>();

    Ok(outputs)
}

pub fn get_current_audio_output(sh: &Shell) -> anyhow::Result<AudioOutput> {
    let outputs = get_all_audio_outputs(sh)?;
    let current_output_name = cmd!(sh, "pactl get-default-sink").read()?;

    outputs
        .into_iter()
        .find(|o| o.name == current_output_name)
        .ok_or(anyhow::anyhow!(
            "Could not find current audio output by name"
        ))
}

pub fn set_audio_output(sh: &Shell, name: &str) -> anyhow::Result<()> {
    cmd!(sh, "pactl set-default-sink {name}").run()?;
    Ok(())
}

pub fn find_matching_output<'a>(
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

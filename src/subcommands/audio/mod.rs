use clap::{arg, value_parser, ArgAction, ArgMatches, Command};
use state::{
    find_matching_output, get_all_audio_outputs, get_current_audio_output, get_current_audio_state,
    set_audio_output, set_volume, toggle_mute, write_to_backing_file,
};
use xshell::Shell;

use crate::util::dmenu::get_platform_dmenu;

mod listener;
mod state;

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
        Command::new("run_listener")
            .about("Launch the listener process, that reacts to audio device added/removed events"),
    ];

    cmd.subcommand_required(true)
        .arg_required_else_help(true)
        .subcommands(inner_subcommands.iter())
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

fn initialize(sh: &Shell) -> anyhow::Result<()> {
    set_volume(sh, 25, false)?;
    let audio_state = get_current_audio_state(sh)?;
    write_to_backing_file(audio_state)
}

pub fn run(sh: &Shell, args: &ArgMatches) -> anyhow::Result<Option<String>> {
    match args.subcommand() {
        Some(("init", _)) => initialize(sh),
        Some(("output", output_args)) => handle_output_subcommand(sh, output_args),
        Some(("volume", volume_args)) => handle_volume_subcommand(sh, volume_args),
        Some(("run_listener", run_listener_args)) => listener::run(run_listener_args),
        _ => Ok(()),
    }?;

    let audio_state = get_current_audio_state(sh)?;
    write_to_backing_file(audio_state)?;

    Ok(None)
}

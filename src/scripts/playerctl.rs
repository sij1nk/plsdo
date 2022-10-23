use std::{fs::{OpenOptions, File}, io::{Write, Read}, path::PathBuf};

use clap::{ArgMatches, Command, Parser, Subcommand, FromArgMatches};
use xshell::{Shell, cmd};

const SELECTED_PLAYER_FILENAME: &str = "selected-player.confset";

fn get_player_specific_subcommand(player: &str, cmd: &PlayerCommand) -> anyhow::Result<&'static str> {
    let variants = match cmd {             // playerctl     mpd
        PlayerCommand::Play =>              ("play",       "play"),
        PlayerCommand::Pause =>             ("pause",      "pause"),
        PlayerCommand::Toggle =>            ("play-pause", "toggle"),
        PlayerCommand::Stop =>              ("stop",       "stop"),
        PlayerCommand::Skip { delta: _ } => ("position",   "seek"),
        PlayerCommand::Next =>              ("next",       "next"),
        PlayerCommand::Prev =>              ("previous",   "prev"),
        _ => return Err(anyhow::anyhow!("{:?} is not supposed to show up here!", cmd))
    };

    Ok(if player == "mpd" {variants.1} else {variants.0})
}

#[derive(Parser, Clone, Debug, Eq, PartialEq, Hash)]
enum PlayerCommand {
    SelectPlayer { player: Option<String> },
    Play,
    Pause,
    Toggle,
    Stop,
    Skip { delta: i32 },
    Next,
    Prev
}

fn get_selected_player_file_path() -> PathBuf {
    let mut path = dirs::home_dir().unwrap();
    path.push(".cache");
    path.push(SELECTED_PLAYER_FILENAME);
    path
}

fn write_selected_player_to_file(selected: &str) -> anyhow::Result<()> {
    // unwrap: we don't want to continue if home doesn't exist
    let path = get_selected_player_file_path();

    let mut selected_player_file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&path)?;

    if let Err(err) = selected_player_file.write_all(selected.as_bytes()) {
        std::fs::remove_file(path)?;
        return Err(anyhow::anyhow!("Could not select player! Reason: {}", err.to_string()));
    }

    Ok(())
}

fn get_selected_player_from_file() -> anyhow::Result<String> {

    let path = get_selected_player_file_path();

    let mut file = File::open(path)?;
    let mut contents = String::new(); 
    let _ = file.read_to_string(&mut contents)?;

    Ok(contents)
}

fn invoke_player_command(sh: &Shell, player: &str, command: PlayerCommand) -> anyhow::Result<()> {
    let executable = if player == "mpd" { "mpc" } else { "playerctl" };
    let executable_subcommand = get_player_specific_subcommand(player, &command)?;

    let mut cmd = cmd!(sh, "{executable}")
        .arg(executable_subcommand);

    if executable == "playerctl" {
        cmd = cmd.arg("-p").arg(player);
    }

    if let PlayerCommand::Skip { delta } = command {
        cmd = cmd.arg(delta.to_string());
    }


    println!("{cmd}");
    cmd.run()?;

    Ok(())
}

pub fn command(cmd: Command<'static>) -> Command<'static> {
    PlayerCommand::augment_subcommands(cmd)
}

pub fn run(sh: &Shell, args: &ArgMatches) -> anyhow::Result<()> {
    let subcmd = PlayerCommand::from_arg_matches(args)
        .map_err(|err| err.exit())
        .unwrap();

    if let PlayerCommand::SelectPlayer { player } = subcmd {
        let players = cmd!(sh, "playerctl -l").read()?;
        let mut players: Vec<_> = players.split('\n').map(|player| {
            if let Some((before, _after)) = player.split_once('.') {
                before
            } else {
                player
            }
        }).collect();

        // TODO(rg): detect if mpd is up, and only include it if it is up. For now we pretend
        // it's always up
        players.push("mpd");

        let selected_player = if let Some(player) = player {
            player
        } else {

            cmd!(sh, "wofi -d --prompt 'Choose media player to control'")
                .stdin(players.join("\n"))
                .read()?
        };

        if !players.iter().any(|&player| player == selected_player) {
            return Err(anyhow::anyhow!("selected media player does not exist"));
        }

        write_selected_player_to_file(&selected_player)?;
    } else {
        let player = get_selected_player_from_file()?;
        invoke_player_command(sh, &player, subcmd)?;
    }
    Ok(())
}



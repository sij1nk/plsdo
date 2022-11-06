use std::{fs::{OpenOptions, File}, io::{Write, Read}, path::PathBuf};

use clap::{ArgMatches, Command, Parser, Subcommand, FromArgMatches};
use xshell::{Shell, cmd};

use crate::util::dmenu;

// TODO:
// - show-status:
//    - basic text output
//    - add styling
//    - click to toggle; rclick to cycle players, if possible
// - update selplayer when opening spotify
// - spotify probs cant poke us when its status changes, 
//   so the statusblock needs to poll every few seconds
// - port mpcsignal from old dotfiles to make mpd poke us on status change
// - force statusblock to update when command is given
// - done?

const SELECTED_PLAYER_FILENAME: &str = "selected-player.confset";

fn get_playerctl_subcommand(cmd: &PlayerCommand) -> anyhow::Result<&'static str> {
    let subcommand = match cmd {         
        PlayerCommand::Play =>              "play",     
        PlayerCommand::Pause =>             "pause",   
        PlayerCommand::Toggle =>            "play-pause",
        PlayerCommand::Stop =>              "stop",     
        PlayerCommand::Skip { delta: _ } => "position",
        PlayerCommand::Next =>              "next",    
        PlayerCommand::Prev =>              "previous",
        _ => return Err(anyhow::anyhow!("{:?} is not supposed to show up here!", cmd))
    };

    Ok(subcommand)
}

#[derive(Parser, Clone, Debug, Eq, PartialEq, Hash)]
enum PlayerCommand {
    ShowStatus,
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
    let playerctl_subcommand = get_playerctl_subcommand(&command)?;

    let mut cmd = cmd!(sh, "playerctl")
        .arg(playerctl_subcommand)
        .arg("-p").arg(player);

    if let PlayerCommand::Skip { delta } = command {
        cmd = cmd.arg(delta.to_string());
    }

    cmd.run()?;

    Ok(())
}

fn show_status(player: &str) {
    if player == "spotify" {
        println!("spotify playing some shit");
    } else {
        println!("mpd or something else playing some shit");
    }
}

pub fn command(cmd: Command<'static>) -> Command<'static> {
    PlayerCommand::augment_subcommands(cmd)
}

pub fn run(sh: &Shell, args: &ArgMatches) -> anyhow::Result<()> {
    // TODO: unwrap
    let subcmd = PlayerCommand::from_arg_matches(args)
        .map_err(|err| err.exit())
        .unwrap();

    if let PlayerCommand::SelectPlayer { player } = subcmd {
        let players = cmd!(sh, "playerctl -l").read()?;
        let players: Vec<_> = players.split('\n').map(|player| {
            if let Some((before, _after)) = player.split_once('.') {
                before
            } else {
                player
            }
        }).collect();

        let selected_player = if let Some(player) = player {
            player
        } else {
            // unwrap: we don't want to continue if the resulting string is empty
            dmenu(sh, "Choose media player to control", &players, true).unwrap()
        };

        write_selected_player_to_file(&selected_player)?;
    } else if PlayerCommand::ShowStatus == subcmd {
        let player = get_selected_player_from_file()?;
        show_status(&player);
    } else {
        let player = get_selected_player_from_file()?;
        invoke_player_command(sh, &player, subcmd)?;
    }
    Ok(())
}



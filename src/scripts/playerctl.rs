use std::{fs::OpenOptions, io::Write};

use clap::{ArgMatches, Command, Parser, Subcommand, FromArgMatches};
use xshell::{Shell, cmd};

const SELECTED_PLAYER_FILENAME: &str = "selected-player.confset";

#[derive(Parser, Debug)]
enum Subcommands {
    SelectPlayer,
    Play,
    Pause,
    Stop,
    Skip {
        delta: i32
    },
    Next,
    Prev
}

pub fn command(cmd: Command<'static>) -> Command<'static> {
    Subcommands::augment_subcommands(cmd)
}

pub fn run(sh: &Shell, args: &ArgMatches) -> anyhow::Result<()> {
    let subcmd = Subcommands::from_arg_matches(args)
        .map_err(|err| err.exit())
        .unwrap();

    // unwrap: we don't want to continue if home doesn't exist
    let mut path = dirs::home_dir().unwrap();
    path.push(".cache");
    path.push(SELECTED_PLAYER_FILENAME);

    match subcmd {
        Subcommands::SelectPlayer => {
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

            let chosen_player = cmd!(sh, "wofi -d --prompt 'Choose media player to control'")
                .stdin(players.join("\n"))
                .read()?;

            if !players.iter().any(|&player| player == chosen_player) {
                return Err(anyhow::anyhow!("Chosen media player does not exist"));
            }

            let mut selected_player_file = OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(&path)?;

            // TODO: error handling; if something went wrong, delete the file
            if let Err(err) = selected_player_file.write_all(chosen_player.as_bytes()) {
                std::fs::remove_file(path)?;
                return Err(anyhow::anyhow!("Could not select player! Reason: {}", err.to_string()));
            }
        }

        // TODO
        Subcommands::Play => println!("play"),
        Subcommands::Pause => println!("pause"),
        Subcommands::Stop => println!("stop"),
        Subcommands::Next => println!("next"),
        Subcommands::Prev => println!("prev"),
        Subcommands::Skip { delta } => println!("skip {delta}")
    }
    Ok(())
}



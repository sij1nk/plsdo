use clap::{arg, value_parser, ArgMatches, Command, ValueEnum};
use std::io::Write;
use xshell::Shell;
use crate::util;

#[derive(ValueEnum, Clone, Debug)]
enum Direction {
    Up,
    Down,
}

pub fn command(cmd: Command<'static>) -> Command<'static> {
    cmd.arg(
        arg!(-d --direction <DIRECTION>)
            .value_parser(value_parser!(Direction))
            .required(false),
    )
    .arg(
        arg!([DELTA])
            .value_parser(value_parser!(i32).range(1..))
            .required(true),
    )
}

pub fn run(_sh: &Shell, args: &ArgMatches) -> anyhow::Result<()> {
    let dir = args.get_one::<Direction>("direction");

    // unwrap: argument is required
    let delta = args.get_one::<i32>("DELTA").unwrap();

    util::modify_file(".dotfiles/config/alacritty/alacritty.yml", "# Point size\n", 
        |lines, writer| {
            let previous_value_line = lines.next().ok_or_else(|| anyhow::anyhow!("Line containing previous value not found"))?;

            let new_value = if let Some(dir) = dir {
                let previous_value = previous_value_line
                    .trim()
                    .split_once(' ')
                    .ok_or_else(|| anyhow::anyhow!("Could not split line containing previous value"))?
                    .1
                    .parse::<i32>()?;

                match dir {
                    Direction::Up => previous_value + delta,
                    Direction::Down => previous_value - delta
                }
            } else {
                *delta
            };

            writer.write_all(format!("  size: {}\n", new_value).as_bytes())?;

            Ok(())
        })?;

    Ok(())
}

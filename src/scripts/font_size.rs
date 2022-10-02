use clap::{arg, value_parser, ArgMatches, Command, ValueEnum};
use std::fs::File;
use std::io::{BufRead, BufReader};
use xshell::Shell;

const FILENAME: &str = ".dotfiles/config/alacritty/alacritty.yml";

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

pub fn run(sh: &Shell, args: &ArgMatches) -> anyhow::Result<()> {
    // let dir = args.get_one::<Direction>("DIRECTION");
    let delta = args.get_one::<i32>("DELTA");

    let mut path = dirs::home_dir().unwrap();
    path.push(FILENAME);
    println!("{:?}", path);

    let file = File::open(path)?;
    let mut reader = BufReader::new(file);

    let mut line = String::new();
    while let Ok(_) = reader.read_line(&mut line) {
        println!("{line}");
    }

    Ok(())
}

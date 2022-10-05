use clap::{arg, value_parser, ArgMatches, Command, ValueEnum};
use std::fs::{OpenOptions};
use std::io::{BufReader, Read, BufWriter, Seek, SeekFrom, Write};
use xshell::Shell;

const FILENAME: &str = ".dotfiles/config/alacritty/alacritty.yml";
const SPLITTER: &str = "  # Point size\n";
const SIZE_PREFIX: &str = "  size:";

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

    // unwrap: we don't want to continue if home doesn't exist
    let mut path = dirs::home_dir().unwrap();
    path.push(FILENAME);
    println!("{:?}", dir);
    println!("{:?}", path);

    let file = 
        OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)?;
        // File::open(path)?;
    let mut reader = BufReader::new(&file);
    let mut writer = BufWriter::new(&file);
    let mut contents = String::new();
    reader.read_to_string(&mut contents)?;

    let mut split = contents.split(SPLITTER);
    let beginning_len = split.next().ok_or_else(|| anyhow::anyhow!("File beginning not found"))?.len();
    let (previous_value_line, rest) = split.next().ok_or_else(|| anyhow::anyhow!("File ending not found"))?
        .split_once('\n').ok_or_else(|| anyhow::anyhow!("Could not find newline to split on"))?;

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

    // unwrap: usize to u64 will work on a 64 bit target
    writer.seek(SeekFrom::Start(beginning_len.try_into().unwrap()))?;
    writer.write_all(SPLITTER.as_bytes())?;
    writer.write_all(format!("{} {}\n", SIZE_PREFIX, new_value).as_bytes())?;
    writer.write_all(rest.as_bytes())?;
    writer.flush()?;

    Ok(())
}

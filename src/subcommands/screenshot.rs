use std::str::FromStr;

use anyhow::Context;
use clap::{arg, builder::OsStr, value_parser, ArgMatches, Command, ValueEnum};
use xshell::Shell;

#[derive(ValueEnum, Clone, Debug)]
enum Output {
    File,
    Clipboard,
}

#[derive(ValueEnum, Clone, Debug)]
enum Target {
    Full,
    Window,
    Monitor,
    Area,
}

impl From<Target> for OsStr {
    fn from(value: Target) -> Self {
        match value {
            Target::Full => "full",
            Target::Window => "window",
            Target::Monitor => "monitor",
            Target::Area => "area",
        }
        .into()
    }
}

#[derive(Debug, Clone)]
struct Area {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

impl FromStr for Area {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parse_part = |s: Option<&str>, part_name: &str| -> anyhow::Result<u32> {
            s.ok_or_else(|| anyhow::anyhow!("`{}` is missing from area definition", part_name))?
                .parse::<u32>()
                .with_context(|| format!("`{}` is not an unsigned number", part_name))
        };

        let mut parts = s.split(',');
        let x = parse_part(parts.next(), "x")?;
        let y = parse_part(parts.next(), "y")?;
        let width = parse_part(parts.next(), "width")?;
        let height = parse_part(parts.next(), "height")?;

        if parts.next().is_some() {
            anyhow::bail!("Area definition contains more parts than expected")
        }

        Ok(Area {
            x,
            y,
            width,
            height,
        })
    }
}

pub fn command_extension(cmd: Command) -> Command {
    cmd.arg(
        arg!(-o --output <OUTPUT> "Where the screenshot should be saved to")
            .value_parser(value_parser!(Output))
    )
    .arg(
        arg!(-t --target <TARGET> "The part of the display to screenshot")
            .value_parser(value_parser!(Target)),
    )
    .arg(arg!(-a --area <AREA> "The area to screenshot, in `x,y,w,h` format. Required if the `area` target is used")
        .required_if_eq("target", Target::Area)
        .value_parser(Area::from_str))
    .arg(arg!(-m --monitor <MONITOR> "The name of the monitor to screenshot. Required if the `monitor` target is used")
        .required_if_eq("target", Target::Monitor)
        .value_parser(clap::builder::NonEmptyStringValueParser::new()))
}

pub fn run(sh: &Shell, _: &ArgMatches) -> anyhow::Result<Option<String>> {
    Ok(None)
}

use std::str::FromStr;

use anyhow::Context;
use clap::{arg, value_parser, ArgMatches, Command, ValueEnum};
use hyprland::{
    data::{Clients, Monitors},
    shared::HyprData,
};
use strum::IntoEnumIterator;
use strum_macros::{Display, EnumIter, EnumString};
use xshell::{cmd, Shell};

use crate::util::dmenu::Dmenu;

#[derive(ValueEnum, Clone, Debug)]
enum Output {
    File,
    Clipboard,
}

#[derive(Clone, Debug, Display, EnumString, EnumIter)]
#[strum(serialize_all = "lowercase")]
enum TargetName {
    Full,
    Window,
    Monitor,
    Area,
}

#[derive(Debug, Clone)]
enum Target {
    Full,
    Window(Area),
    Monitor(String),
    Area(Area),
}

impl Target {
    fn to_grim_arguments(&self) -> String {
        todo!()
    }
}

#[derive(Debug, Clone)]
enum Area {
    Rectangle {
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    },
    String(String),
}

impl Area {
    fn from_slurp(s: String) -> Self {
        Self::String(s)
    }
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

        Ok(Area::Rectangle {
            x,
            y,
            width,
            height,
        })
    }
}

fn get_window_target() -> anyhow::Result<Target> {
    let clients = Clients::get()?;
    let window = clients
        .iter()
        .find(|c| c.focus_history_id == 0)
        .ok_or_else(|| anyhow::anyhow!("Could not find the last focused window"))?;

    if window.at.0.is_negative()
        || window.at.1.is_negative()
        || window.size.0.is_negative()
        || window.size.1.is_negative()
    {
        anyhow::bail!("Window dimensions are invalid: one of x,y,width,height are negative");
    }

    Ok(Target::Window(Area::Rectangle {
        x: window.at.0 as u32,
        y: window.at.1 as u32,
        width: window.size.0 as u32,
        height: window.size.1 as u32,
    }))
}

fn dmenu_monitor(sh: &Shell) -> anyhow::Result<String> {
    let monitors = Monitors::get()?;
    let monitors = monitors.iter().map(|m| m.name.as_ref()).collect::<Vec<_>>();

    Dmenu::new(sh)
        .numbered()
        .auto_select()
        .choose_one_str("Select monitor", &monitors)
}

fn get_monitor_target(sh: &Shell, monitor: Option<&String>) -> anyhow::Result<Target> {
    if let Some(monitor) = monitor {
        Ok(Target::Monitor(monitor.clone()))
    } else {
        let monitor = dmenu_monitor(sh)?;
        Ok(Target::Monitor(monitor))
    }
}

fn get_area_target(sh: &Shell, area: Option<&Area>) -> anyhow::Result<Target> {
    if let Some(area) = area {
        Ok(Target::Area(area.clone()))
    } else {
        let area = cmd!(sh, "slurp").read()?;
        Ok(Target::Area(Area::from_slurp(area)))
    }
}

fn get_target(sh: &Shell, args: &ArgMatches) -> anyhow::Result<Target> {
    match args.subcommand() {
        Some(("full", _)) => Ok(Target::Full),
        Some(("window", _)) => get_window_target(),
        Some(("monitor", monitor_args)) => {
            let monitor = monitor_args.get_one::<String>("MONITOR");
            get_monitor_target(sh, monitor)
        }
        Some(("area", area_args)) => {
            let area = area_args.get_one::<Area>("AREA");
            get_area_target(sh, area)
        }
        _ => {
            let target_name = dmenu_target_name(sh)?;
            match target_name {
                TargetName::Full => Ok(Target::Full),
                TargetName::Window => get_window_target(),
                TargetName::Monitor => get_monitor_target(sh, None),
                TargetName::Area => get_area_target(sh, None),
            }
        }
    }
}

fn dmenu_target_name(sh: &Shell) -> anyhow::Result<TargetName> {
    let target_names = TargetName::iter()
        .map(|t| t.to_string())
        .collect::<Vec<_>>();
    let result = Dmenu::new(sh).numbered().auto_select().choose_one(
        "Select screenshot target",
        &target_names,
        String::as_ref,
    )?;
    TargetName::from_str(result).map_err(|e| e.into())
}

pub fn command_extension(cmd: Command) -> Command {
    cmd.subcommands([
        Command::new("full").about("Screenshot the entire display"),
        Command::new("window").about("Screenshot the focused window"),
        Command::new("monitor")
            .about("Screenshot the selected monitor, or the entire display, if there's only one monitor")
            .arg(arg!([MONITOR] "The name of the monitor to screenshot. Required if the `monitor` target is used")
                .value_parser(clap::builder::NonEmptyStringValueParser::new())),
        Command::new("area")
            .about("Screenshot the selected area")
            .arg(arg!([AREA] "The area to screenshot, in `x,y,w,h` format. Required if the `area` target is used")
                .value_parser(Area::from_str)),
    ]).arg(
        arg!(-o --output <OUTPUT> "Where the screenshot should be saved to")
            .value_parser(value_parser!(Output)))
}

pub fn run(sh: &Shell, args: &ArgMatches) -> anyhow::Result<Option<String>> {
    let target = get_target(sh, args)?;
    println!("{:?}", target);
    todo!()
}

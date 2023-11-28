use std::{
    fs::OpenOptions,
    io::{BufWriter, Write},
};

use clap::{ArgMatches, Command};
use xshell::Shell;

use crate::system_atlas::SystemAtlas;

enum Severity {
    None,
    Low,
    Medium,
    High,
}

trait Widget<T> {
    fn initialize();
    fn get_severity(t: &T) -> Option<Severity>;
    fn read_value(&self) -> T;
}

struct Brightness {
    min_value: f32,
    max_value: f32,
    status_filename: String,
    pid_filename: String,
}

impl Brightness {
    fn new() -> Self {
        Self {
            min_value: 0.0,
            max_value: 1.0,
            status_filename: String::from("/tmp/eww-brightness"),
            pid_filename: String::from("/tmp/eww-brightness.pid"),
        }
    }

    fn initialize() {}
}

impl Widget<f32> for Brightness {
    fn get_severity(value: &f32) -> Option<Severity> {
        if *value < 0.25 {
            Some(Severity::None)
        } else if *value < 0.50 {
            Some(Severity::Low)
        } else if *value < 0.75 {
            Some(Severity::Medium)
        } else {
            Some(Severity::High)
        }
    }

    fn read_value(&self) -> f32 {
        todo!()
    }

    fn initialize() {
        todo!()
    }
}

struct ColorTemperature {
    value: u32,
}

struct Volume {
    value: u32,
}

fn init_file(path: &str, value: &[u8]) -> anyhow::Result<()> {
    let file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(path)?;
    let mut writer = BufWriter::new(&file);
    writer.write_all(value)?;
    Ok(())
}

fn init_brightness(path: &str, value: u32) -> anyhow::Result<()> {
    init_file(path, format!("{}\n", value).as_bytes())?;

    Ok(())
}

pub fn command(cmd: Command<'static>) -> Command<'static> {
    cmd
}

pub fn run(sh: &Shell, args: &ArgMatches, atlas: &SystemAtlas) -> anyhow::Result<()> {
    init_brightness(atlas.eww_brightness, 100)?;

    Ok(())
}

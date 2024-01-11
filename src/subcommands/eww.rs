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

struct Brightness;

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

fn init_gamma(path: &str, value: u32) -> anyhow::Result<()> {
    Ok(())
}

fn init_volume(path: &str, value: u32) -> anyhow::Result<()> {
    Ok(())
}

fn init_show_all(path: &str, value: bool) -> anyhow::Result<()> {
    Ok(())
}

pub fn command_extension(cmd: Command) -> Command {
    cmd
}

pub fn run(sh: &Shell, args: &ArgMatches, atlas: &SystemAtlas) -> anyhow::Result<()> {
    init_brightness(atlas.eww_brightness, 100)?;
    init_gamma(atlas.eww_gamma, 4200)?;
    init_volume(atlas.eww_volume, 20)?;
    init_show_all(atlas.eww_show_all, false)?;

    Ok(())
}

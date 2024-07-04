#![allow(dead_code)]

use std::env;
use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path;

pub mod dmenu;
use wl_clipboard_rs::paste::{get_contents, ClipboardType, Error, MimeType, Seat};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum WM {
    Hyprland,
    GenericX11,
}

pub trait Clipboard {
    fn get_one(&self) -> anyhow::Result<String>;
    fn get_many(&self, n: u32) -> anyhow::Result<&[String]>;
}

pub struct RealClipboard {
    wm: WM,
}

impl RealClipboard {
    pub fn new(wm: WM) -> Self {
        Self { wm }
    }

    fn get_one_wayland(&self) -> anyhow::Result<String> {
        let result = get_contents(ClipboardType::Regular, Seat::Unspecified, MimeType::Text);
        match result {
            Ok((mut pipe, _)) => {
                let mut contents = vec![];
                pipe.read_to_end(&mut contents)?;
                Ok(String::from_utf8_lossy(&contents).to_string())
            }
            Err(Error::NoSeats) | Err(Error::ClipboardEmpty) | Err(Error::NoMimeType) => {
                Ok(String::new())
            }
            Err(err) => Err(err.into()),
        }
    }

    fn get_one_x11(&self) -> anyhow::Result<String> {
        unimplemented!()
    }

    fn get_many_wayland(&self, _n: u32) -> anyhow::Result<&[String]> {
        unimplemented!()
    }

    fn get_many_x11(&self, _n: u32) -> anyhow::Result<&[String]> {
        unimplemented!()
    }
}

impl Clipboard for RealClipboard {
    fn get_one(&self) -> anyhow::Result<String> {
        match self.wm {
            WM::Hyprland => self.get_one_wayland(),
            WM::GenericX11 => self.get_one_x11(),
        }
    }

    fn get_many(&self, n: u32) -> anyhow::Result<&[String]> {
        match self.wm {
            WM::Hyprland => self.get_many_wayland(n),
            WM::GenericX11 => self.get_many_x11(n),
        }
    }
}

pub fn determine_wm() -> WM {
    if let Some(value) = env::vars()
        .find(|(k, _)| k == "XDG_CURRENT_DESKTOP")
        .map(|(_, v)| v)
    {
        match value.as_str() {
            "Hyprland" => WM::Hyprland,
            _ => WM::GenericX11,
        }
    } else {
        WM::GenericX11
    }
}

pub fn modify_file<F>(path: &str, splitter: &str, modifier: F) -> anyhow::Result<()>
where
    F: FnOnce(&mut LinesWithEndings, &mut BufWriter<&File>) -> anyhow::Result<()>,
{
    // unwrap: we don't want to continue if home doesn't exist
    let path = Path::new(path);
    let mut temp_path = path.to_path_buf();

    let file_name = path
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("Given path is not pointing to a file"))?
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("File name is not valid UTF-8"))?;

    temp_path.push(".cache");
    temp_path.push(file_name.to_string() + ".plsdo");

    let file = File::open(path)?;
    let mut reader = BufReader::new(&file);

    let temp_file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&temp_path)?;
    let mut writer = BufWriter::new(&temp_file);

    let mut contents = String::new();
    reader.read_to_string(&mut contents)?;

    let (before, after) = contents
        .split_once(splitter)
        .ok_or_else(|| anyhow::anyhow!("Could not find splitter string"))?;

    let mut after_lines = after.into();

    writer.write_all(before.as_bytes())?;
    writer.write_all(splitter.as_bytes())?;

    modifier(&mut after_lines, &mut writer)?;

    let rest = after_lines.collect::<String>();

    writer.write_all(rest.as_bytes())?;
    writer.flush()?;

    std::fs::rename(temp_path, path)?;

    Ok(())
}

pub struct LinesWithEndings<'a> {
    input: &'a str,
}

impl<'a> From<&'a str> for LinesWithEndings<'a> {
    fn from(input: &'a str) -> Self {
        Self { input }
    }
}

impl<'a> Iterator for LinesWithEndings<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<&'a str> {
        if self.input.is_empty() {
            return None;
        }
        let split = self
            .input
            .find('\n')
            .map(|i| i + 1)
            .unwrap_or(self.input.len());
        let (line, rest) = self.input.split_at(split);
        self.input = rest;
        Some(line)
    }
}

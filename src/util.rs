#![allow(dead_code)]

use std::env;
use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path;

use anyhow::Context;
use xshell::{cmd, Shell};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum WM {
    Hyprland,
    Sway,
    X11,
}

pub fn determine_wm() -> WM {
    if let Some(value) = env::vars()
        .find(|(k, _)| k == "XDG_CURRENT_DESKTOP")
        .map(|(_, v)| v)
    {
        match value.as_str() {
            "sway" => WM::Sway,
            "Hyprland" => WM::Hyprland,
            _ => WM::X11,
        }
    } else {
        WM::X11
    }
}

fn dmenu_inner_x11(sh: &Shell, prompt: &str, choices_joined: &str) -> anyhow::Result<String> {
    Ok(
        cmd!(sh, "dmenu -p {prompt} -i -l 10 -fn 'monospace:size=24'")
            .stdin(choices_joined)
            .read()?,
    )
}

pub fn dmenu<T>(
    sh: &Shell,
    prompt: &str,
    choices: &[T],
    forbid_invalid: bool,
) -> anyhow::Result<String>
where
    T: AsRef<str>,
{
    let choices_joined = choices
        .iter()
        .map(|c| c.as_ref())
        .collect::<Vec<_>>()
        .join("\n");

    let chosen: anyhow::Result<String> =
        if let Some((_, session_type)) = std::env::vars().find(|(k, _)| k == "XDG_SESSION_TYPE") {
            if session_type == "wayland" {
                cmd!(sh, "wofi -d --prompt {prompt}")
                    .stdin(&choices_joined)
                    .read()
                    .map_err(anyhow::Error::new)
            } else {
                dmenu_inner_x11(sh, prompt, &choices_joined)
            }
        } else {
            dmenu_inner_x11(sh, prompt, &choices_joined)
        };

    let chosen = chosen.context("Aborted")?;

    if forbid_invalid && !choices_joined.contains(&chosen) {
        return Err(anyhow::anyhow!("Invalid input given"));
    }

    Ok(chosen)
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

pub fn trim_sides(s: &str) -> &str {
    let mut chars = s.chars();
    chars.next();
    chars.next_back();
    chars.as_str()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trim_sides_works() {
        let s = "\"Hello world\"";
        assert_eq!(trim_sides(s), "Hello world");
    }

    #[test]
    fn trim_sides_works_on_empty_string() {
        let s = "";
        assert_eq!(trim_sides(s), "");
    }
}

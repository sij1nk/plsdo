#![allow(dead_code)]

use std::{fs::{File, OpenOptions}, io::{BufReader, BufWriter, Read, Write }, str::Split};

use xshell::{cmd, Shell};

// TODO: maybe the 2nd argument could be more generic
pub fn dmenu(sh: &Shell, prompt: &str, choices: String) -> anyhow::Result<String> {
    if let Some((_, session_type)) = std::env::vars().find(|(k, _)| k == "XDG_SESSION_TYPE") {
        if session_type == "wayland" {
            return Ok(cmd!(sh, "wofi -d --prompt {prompt}")
                .stdin(choices)
                .read()?);
        }
    }

    Ok(cmd!(sh, "dmenu -p {prompt} -i -fn 'monospace:size=24'")
        .stdin(choices)
        .read()?)
}


pub fn modify_file<F>(path_from_home: &str, splitter: &str, modifier: F) -> anyhow::Result<()>
    where F: FnOnce(&mut Split<char>, &mut BufWriter<&File>) -> anyhow::Result<()>
{
    // unwrap: we don't want to continue if home doesn't exist
    let mut path = dirs::home_dir().unwrap();
    let mut temp_path = path.clone();

    path.push(path_from_home);

    let file_name = path.file_name()
        .ok_or_else(|| anyhow::anyhow!("Given path is not pointing to a file"))?
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("File name is not valid UTF-8"))?;

    temp_path.push(".cache");
    temp_path.push(file_name.to_string() + ".confset");

    let file = File::open(&path)?;
    let mut reader = BufReader::new(&file);

    let temp_file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&temp_path)?;
    let mut writer = BufWriter::new(&temp_file);

    let mut contents = String::new();
    reader.read_to_string(&mut contents)?;

    let (before, after) = contents.split_once(splitter).ok_or_else(|| anyhow::anyhow!("Could not find splitter string"))?;

    let mut after_lines = after.split('\n');

    writer.write_all(before.as_bytes())?;
    writer.write_all(splitter.as_bytes())?;

    modifier(&mut after_lines, &mut writer)?;

    let rest = after_lines.intersperse("\n").collect::<String>();

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

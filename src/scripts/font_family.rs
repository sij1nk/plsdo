use std::{io::{BufReader, Read, BufWriter, SeekFrom, Write, Seek}, fs::{File, OpenOptions}};

use clap::ArgMatches;
use xshell::{Shell, cmd};

const FILENAME: &str = ".dotfiles/config/fontconfig/fonts.conf";
const TEMP_FILENAME: &str = ".cache/confset_font_family";
const TARGET: &str = "<family>monospace</family>\n";

pub fn run(sh: &Shell, _: &ArgMatches) -> anyhow::Result<()> {

    let mut path = dirs::home_dir().unwrap();
    path.push(FILENAME);

    let mut temp_path = dirs::home_dir().unwrap();
    temp_path.push(TEMP_FILENAME);

    let file = File::open(&path)?;
    let temp_file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&temp_path)?;

    let mut reader = BufReader::new(&file);
    let mut writer = BufWriter::new(&temp_file);

    let mut contents = String::new();
    reader.read_to_string(&mut contents)?;

    let (before, after) = contents.split_once(TARGET).ok_or_else(|| anyhow::anyhow!("Could not find target string"))?;

    let mut font_families = Vec::new();

    let mut after_lines = after.split('\n');

    while let Some(line) = after_lines.next() {
        if line.trim().starts_with("</prefer>") {
            break;
        }

        if let Some(family) = line.trim().strip_prefix("<family>") {
            if let Some(family) = family.strip_suffix("</family>") {
                font_families.push(family);
            }
        }
    }

    let rest = after_lines.intersperse("\n").collect::<String>();
    println!("before: {before}");
    println!("rest: {rest}");

    let chosen = cmd!(sh, "wofi -d --prompt 'Choose font family'")
        .stdin(font_families.join("\n"))
        .read()?;

    if !font_families.iter().any(|&family| family == chosen) {
        return Err(anyhow::anyhow!("Chosen value is not a valid font family name"));
    }

    writer.write_all(before.as_bytes())?;
    writer.write_all(TARGET.as_bytes())?;
    writer.write_all(b"    <prefer>\n")?;

    writer.write_all(format!("      <family>{}</family>\n", chosen).as_bytes())?;

    for family in font_families.iter().filter(|&&f| f != chosen) {
        writer.write_all(format!("      <family>{}</family>\n", family).as_bytes())?;
    }

    writer.write_all(b"    </prefer>\n")?;
    writer.write_all(rest.as_bytes())?;
    writer.flush()?;

    std::fs::rename(temp_path, path)?;

    Ok(())
}

use std::{io::{BufReader, Read, BufWriter, Write}, fs::{File, OpenOptions}, str::Split};

use clap::ArgMatches;
use xshell::{Shell, cmd};


fn modify_file<F>(path_from_home: &str, splitter: &str, modifier: F) -> anyhow::Result<()>
    where F: FnOnce(&mut Split<char>, &mut BufWriter<&File>) -> anyhow::Result<String>
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

    let rest = modifier(&mut after_lines, &mut writer)?;

    writer.write_all(rest.as_bytes())?;
    writer.flush()?;

    std::fs::rename(temp_path, path)?;

    Ok(())
}


pub fn run(sh: &Shell, _: &ArgMatches) -> anyhow::Result<()> {

    modify_file(".dotfiles/config/fontconfig/fonts.conf", "<family>monospace</family>\n", 
        |lines, writer| {
        let mut font_families = Vec::new();

        for line in lines.by_ref() {
            if line.trim().starts_with("</prefer>") {
                break;
            }

            if let Some(family) = line.trim().strip_prefix("<family>") {
                if let Some(family) = family.strip_suffix("</family>") {
                    // Ignore icons fonts. Selecting an icon font as the main font in fontconfig works
                    // fine, but setting it as default in alacritty would break it
                    if !family.to_lowercase().contains("icon") {
                        font_families.push(family);
                    }
                }
            }
        }

        let rest = lines.intersperse("\n").collect::<String>();

        let chosen = cmd!(sh, "wofi -d --prompt 'Choose font family'")
            .stdin(font_families.join("\n"))
            .read()?;

        if !font_families.iter().any(|&family| family == chosen) {
            return Err(anyhow::anyhow!("Chosen value is not a valid font family name"));
        }

        modify_file(".dotfiles/config/alacritty/alacritty.yml", "font:\n",
            |lines, writer| {

                for line in lines.by_ref() {
                    if line.trim().starts_with("family:") {
                        let _ = writer.write(b"    family: ")?;
                        _ = writer.write(chosen.as_bytes())?;
                        _ = writer.write(b"\n");
                        break;
                    }

                    writer.write_all(line.as_bytes())?;
                    let _ = writer.write(b"\n")?;
                }

                let rest = lines.intersperse("\n").collect::<String>();

                Ok(rest)
            })?;

        writer.write_all(b"    <prefer>\n")?;
        writer.write_all(format!("      <family>{}</family>\n", chosen).as_bytes())?;

        for family in font_families.iter().filter(|&&f| f != chosen) {
            writer.write_all(format!("      <family>{}</family>\n", family).as_bytes())?;
        }
        writer.write_all(b"    </prefer>\n")?;

        Ok(rest)
    })?;

    Ok(())
}

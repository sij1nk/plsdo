use std::io::Write;
use crate::util;

use clap::ArgMatches;
use xshell::{Shell, cmd};

pub fn run(sh: &Shell, _: &ArgMatches) -> anyhow::Result<()> {

    util::modify_file(".dotfiles/config/fontconfig/fonts.conf", "<family>monospace</family>\n", 
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

        let chosen = cmd!(sh, "wofi -d --prompt 'Choose font family'")
            .stdin(font_families.join("\n"))
            .read()?;

        if !font_families.iter().any(|&family| family == chosen) {
            return Err(anyhow::anyhow!("Chosen value is not a valid font family name"));
        }

        util::modify_file(".dotfiles/config/alacritty/alacritty.yml", "font:\n",
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

                Ok(())
            })?;

        writer.write_all(b"    <prefer>\n")?;
        writer.write_all(format!("      <family>{}</family>\n", chosen).as_bytes())?;

        for family in font_families.iter().filter(|&&f| f != chosen) {
            writer.write_all(format!("      <family>{}</family>\n", family).as_bytes())?;
        }
        writer.write_all(b"    </prefer>\n")?;

        Ok(())
    })?;

    Ok(())
}

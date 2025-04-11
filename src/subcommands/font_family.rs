use crate::{
    system_atlas::SYSTEM_ATLAS,
    util::{self, dmenu::Dmenu},
};
use std::io::Write;

use clap::{ArgMatches, Command};
use xshell::Shell;

pub fn command_extension(cmd: Command) -> Command {
    cmd
}

pub fn run(sh: &Shell, _: &ArgMatches) -> anyhow::Result<Option<String>> {
    util::modify_file(
        SYSTEM_ATLAS.fontconfig,
        "<family>monospace</family>\n",
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

            // unwrap: we don't want to continue if the string is empty
            let chosen = Dmenu::new(sh)
                .choose_one_str("Choose font family", &font_families, true)
                .unwrap();

            util::modify_file(SYSTEM_ATLAS.alacritty, "font:\n", |lines, writer| {
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
        },
    )?;

    Ok(None)
}

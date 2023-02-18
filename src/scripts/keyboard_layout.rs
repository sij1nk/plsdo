use anyhow::anyhow;
use clap::ArgMatches;
use std::{str::FromStr, fs::File, io::Read};

use serde::Deserialize;
use serde_json::Value;
use xshell::{cmd, Shell};

use crate::util::{dmenu, determine_wm, WM};

#[derive(Debug, Deserialize)]
struct RiverLayoutDefinition {
    name: String,
    label: String,
    variant: String,
    options: String,
    layout: String
}

#[derive(Debug, Deserialize)]
struct RiverConfig {
    layouts: Vec<RiverLayoutDefinition>
}

fn read_river_layouts() -> anyhow::Result<Vec<RiverLayoutDefinition>> {
    let mut path = dirs::home_dir().unwrap();
    path.push(".dotfiles/config/river/layouts.toml");

    let mut file = File::open(&path)?;
    let mut contents = String::new();
    let _ = file.read_to_string(&mut contents)?;

    let config: RiverConfig = toml::from_str(&contents)?;

    Ok(config.layouts)
}

fn get_layout_names(sh: &Shell, wm: WM) -> anyhow::Result<Vec<String>> {
    match wm {
        WM::Sway => {
            let result = cmd!(sh, "swaymsg -t get_inputs").read()?;
            let json: Value = serde_json::from_str(&result)?;
            let layout_names = &json[0]["xkb_layout_names"];

            if *layout_names == Value::Null {
                return Err(anyhow!("Could not find list of accepted keyboard layouts"));
            }

            if let Value::Array(layout_names) = layout_names {
                let layout_names = layout_names
                    .iter()
                    .enumerate()
                    .filter_map(|(n, e)| {
                        if let Value::String(s) = e {
                            Some(format!("{}: {}", n, s))
                        } else {
                            None
                        }
                    })
                .collect::<Vec<_>>();

                Ok(layout_names)
            } else {
                Err(anyhow!(
                        "Expected a list of accepted keyboard layouts, got something else"
                ))
            }
        },
        WM::River => {
            let layout_names = read_river_layouts()?
                .iter()
                .enumerate()
                .map(|(i, def)| {
                    let mut s = String::new();
                    s.push_str(&i.to_string());
                    s.push(':');
                    s.push(' ');
                    s.push_str(&def.name);
                    s
                }).collect::<Vec<_>>();
            Ok(layout_names)
        },
        _ => todo!()
    }
}


fn set_layout(sh: &Shell, layout_index: usize, wm: WM) -> anyhow::Result<()> {
    match wm {
        WM::Sway => {
            let s = layout_index.to_string();
            cmd!(sh, "swaymsg input type:keyboard xkb_switch_layout {s}")
                .quiet()
                .run()?;

            Ok(())
        },
        WM::River => {
            let layouts = read_river_layouts()?;
            let chosen = &layouts[layout_index];

            let variant = &chosen.variant;
            let options = &chosen.options;
            let layout = &chosen.layout;
            cmd!(sh, "riverctl keyboard-layout -variant {variant} -options {options} {layout}")
                .quiet()
                .run()?;

            Ok(())
        },
        _ => todo!()
    }
}

pub fn run(sh: &Shell, _: &ArgMatches) -> anyhow::Result<()> {
    let wm = determine_wm();
    let layout_names = get_layout_names(sh, wm)?;


    // unwrap: don't want to continue if string is empty
    let result_index_str = dmenu(sh, "Choose keyboard layout", &layout_names, true).unwrap();

    // unwrap: split always return at least 1 element
    let result_index_str = result_index_str.split(':').next().unwrap();

    let result_index = usize::from_str(result_index_str)?;

    set_layout(sh, result_index, wm)?;

    let waybar_pid = cmd!(sh, "pidof waybar").quiet().read()?;

    cmd!(sh, "kill -RTMIN+1 {waybar_pid}").quiet().run()?;

    Ok(())
}

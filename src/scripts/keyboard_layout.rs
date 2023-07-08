use anyhow::anyhow;
use clap::ArgMatches;
use std::str::FromStr;

use serde_json::Value;
use xshell::{cmd, Shell};

use crate::util::{determine_wm, dmenu, WM};

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
        }
        _ => todo!(),
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
        }
        _ => todo!(),
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

use anyhow::anyhow;
use clap::ArgMatches;
use std::str::FromStr;

use serde_json::Value;
use xshell::{cmd, Shell};

pub fn run(sh: &Shell, _: &ArgMatches) -> anyhow::Result<()> {
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
            .collect::<Vec<_>>()
            .join("\n");

        let result_index_str = cmd!(sh, "wofi -d --prompt 'Choose keyboard layout'")
            .stdin(layout_names)
            .read()?;

        // unwrap: split always return at least 1 element
        let result_index_str = result_index_str.split(':').next().unwrap();

        // We don't actually care about the parsed value, we only care that it's parseable
        let _ = i32::from_str(result_index_str)?;

        cmd!(
            sh,
            "swaymsg input type:keyboard xkb_switch_layout {result_index_str}"
        )
        .quiet()
        .run()?;

        let waybar_pid = cmd!(sh, "pidof waybar").quiet().read()?;

        cmd!(sh, "kill -RTMIN+1 {waybar_pid}").quiet().run()?;
    } else {
        return Err(anyhow!(
            "Expected a list of accepted keyboard layouts, got something else"
        ));
    }

    Ok(())
}

use std::{
    fs::{read_to_string, File},
    io::{BufRead, BufReader, Read},
};

use anyhow::Context;
use serde::{de::Visitor, Deserialize, Deserializer};
use xshell::{cmd, Shell};

#[derive(Debug, Clone)]
pub struct XkbLayoutData {
    hyprland_id: u8,
    // There are some other xkb fields here, but I'm not using them currently
    layout: String,
    variant: String,
    options: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct XkbLayout {
    name: String,
    data: XkbLayoutData,
}

#[derive(Deserialize, Debug, Clone)]
struct HyprctlOption {
    option: String,
    #[serde(rename(deserialize = "str"))]
    #[serde(deserialize_with = "deserialize_values")]
    values: Vec<String>,
}

fn deserialize_values<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    struct StringVisitor;

    impl Visitor<'_> for StringVisitor {
        type Value = Vec<String>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("values")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(v.split(',').map(|w| w.to_owned()).collect())
        }
    }

    deserializer.deserialize_string(StringVisitor)
}

fn get_hypr_option(sh: &Shell, option: &str) -> anyhow::Result<HyprctlOption> {
    let json_string = cmd!(sh, "hyprctl getoption {option} -j").read()?;
    serde_json::from_str::<HyprctlOption>(&json_string)
        .with_context(|| format!("Failed to read Hyprland option '{}'", option))
}

fn read_xkb_variants_lines() -> anyhow::Result<Vec<String>> {
    let path = "/usr/share/X11/xkb/rules/base.lst";
    let content = read_to_string(path).context("Failed to read xkb rules file")?;
    let mut lines = content.lines();

    // skip ahead
    while lines.next().is_some_and(|l| l.trim() != "! variant") {}

    let variants = lines
        .take_while(|l| l.trim() != "! option")
        .map(|l| l.to_owned())
        .collect::<Vec<_>>();

    Ok(variants)
}

pub fn get_xkb_layouts(sh: &Shell) -> anyhow::Result<Vec<XkbLayout>> {
    let xkb_variants_lines = read_xkb_variants_lines()?;
    let layout_data = get_hyprland_xkb_config(sh)?;

    // NOTE: we could iterate only once, and check for all layout data on each line,
    // but that would be more complicated

    layout_data
        .into_iter()
        .map(|ld| {
            let layout_name = xkb_variants_lines
                .iter()
                .find_map(|line| {
                    let line = line.trim();
                    let line = line.strip_prefix(&ld.variant)?;
                    let line = line.trim();
                    let line = line.strip_prefix(&ld.layout)?;
                    let line = line.strip_prefix(':')?;
                    let name = line.trim();
                    Some(name)
                })
                .ok_or_else(|| anyhow::anyhow!("Could not find matching layout name"))?;

            Ok(XkbLayout {
                name: layout_name.to_owned(),
                data: ld,
            })
        })
        .collect()
}

pub fn get_hyprland_xkb_config(sh: &Shell) -> anyhow::Result<Vec<XkbLayoutData>> {
    let layouts = get_hypr_option(sh, "input:kb_layout")?;
    let variants = get_hypr_option(sh, "input:kb_variant")?;
    let options = get_hypr_option(sh, "input:kb_options")?;

    if layouts.values.len() != variants.values.len() {
        anyhow::bail!(
            "Invalid Hyprland configuration: number of values in {} and {} don't match",
            layouts.option,
            variants.option
        );
    }

    Ok(layouts
        .values
        .iter()
        .enumerate()
        .map(|(i, layout)| XkbLayoutData {
            hyprland_id: i as u8,
            layout: layout.clone(),
            variant: variants.values[i].clone(),
            options: options.values.clone(),
        })
        .collect())
}

use std::{
    fmt::Display,
    fs::{File, OpenOptions},
    io::{BufRead, BufReader, LineWriter, Write},
    path::PathBuf,
};

use anyhow::{anyhow, Context};
use clap::{arg, value_parser, ArgMatches, Command};
use hyprland::{ctl::switch_xkb_layout::SwitchXKBLayoutCmdTypes, data::Devices, shared::HyprData};

mod xkb;

use serde::{Deserialize, Serialize};
use xkb::{get_xkb_layouts, XkbLayout};
use xshell::{cmd, Shell};

use crate::{
    system_atlas::SYSTEM_ATLAS,
    util::{determine_wm, dmenu::Dmenu, WM},
};

// TODO: do these really need to be Strings?
#[derive(Debug, Clone)]
struct AlternativeLayout {
    id: String,
    name: String,
    dotfiles_path: String,
}

#[derive(Debug, Clone)]
enum KeyboardLayout {
    Xkb(XkbLayout),
    Alternative(AlternativeLayout),
}

impl KeyboardLayout {
    fn id(&self) -> &str {
        match self {
            Self::Xkb(layout) => layout.data.layout.as_str(),
            Self::Alternative(layout) => layout.id.as_str(),
        }
    }
    fn name(&self) -> &str {
        match self {
            Self::Xkb(layout) => layout.name.as_str(),
            Self::Alternative(layout) => layout.name.as_str(),
        }
    }
    fn dotfiles_path(&self) -> &str {
        match self {
            Self::Xkb(_) => SYSTEM_ATLAS.main_dotfiles,
            Self::Alternative(layout) => layout.dotfiles_path.as_str(),
        }
    }
    fn persisted_data(&self) -> PersistedData {
        PersistedData {
            layout_id: self.id().to_owned(),
        }
    }
}

impl PartialEq for KeyboardLayout {
    fn eq(&self, other: &Self) -> bool {
        self.id() == other.id()
    }
}

impl Display for KeyboardLayout {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Xkb(layout) => write!(f, "{} ({})", layout.data.layout, layout.name),
            Self::Alternative(layout) => write!(f, "{}", layout.name),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct PersistedData {
    layout_id: String,
}

impl PersistedData {
    fn read() -> anyhow::Result<Option<Self>> {
        let file =
            File::open(SYSTEM_ATLAS.keyboard_layout).context("The backing file does not exist")?;
        let bufreader = BufReader::new(file);
        let last_line = bufreader.lines().last();

        match last_line {
            Some(line) => {
                let line = line.context("Failed to read lines from the backing file")?;
                Ok(Some(serde_json::from_str::<Self>(&line)?))
            }
            None => Ok(None),
        }
    }

    fn write(&self) -> anyhow::Result<()> {
        let file = OpenOptions::new()
            .create(false)
            .append(true)
            .open(SYSTEM_ATLAS.keyboard_layout)?;
        let mut writer = LineWriter::new(&file);
        serde_json::to_writer(&mut writer, self)?;
        writer.write_all(b"\n")?;
        Ok(())
    }
}

fn get_current_layout(all_layouts: &[KeyboardLayout]) -> anyhow::Result<&KeyboardLayout> {
    let persisted_data = PersistedData::read()?;

    match persisted_data {
        Some(data) => all_layouts
            .iter()
            .find(|l| l.id() == data.layout_id)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Persisted keyboard layout data does not match any of the known layouts"
                )
            }),
        None => {
            // some heuristics to try figure out the keyboard layout in use
            std::fs::read_link("/home/rg/.config/nvim/init.lua")
                .context("Example file does not exist, or is not a symlink")
                .and_then(|path| {
                    path.iter()
                        .filter_map(|s| s.to_str())
                        .find(|s| s.contains(".dotfiles"))
                        .map(|s| s.to_owned())
                        .ok_or_else(|| {
                            anyhow!("Example file is not symlinked to a dotfiles folder")
                        })
                })
                .map(|dotfiles_path_segment| {
                    let found_layout = all_layouts.iter().find(|l| match l {
                        KeyboardLayout::Xkb(_) => false,
                        KeyboardLayout::Alternative(_) => PathBuf::from(l.dotfiles_path())
                            .iter()
                            .filter_map(|s| s.to_str())
                            .any(|segment| segment == dotfiles_path_segment),
                    });

                    match found_layout {
                        Some(layout) => layout,
                        None => &all_layouts[0],
                    }
                })
        }
    }
}

fn set_hyprland_layout_by_id(id: u8) -> anyhow::Result<()> {
    let keyboards = Devices::get()?.keyboards;
    for kb in keyboards {
        hyprland::ctl::switch_xkb_layout::call(&kb.name, SwitchXKBLayoutCmdTypes::Id(id))?;
    }
    Ok(())
}

fn collect_all_layouts(sh: &Shell) -> anyhow::Result<Vec<KeyboardLayout>> {
    let xkb_layouts = get_xkb_layouts(sh)?;
    let kyria_layout = AlternativeLayout {
        id: "ky".to_owned(),
        name: "kyria".to_owned(),
        dotfiles_path: SYSTEM_ATLAS.canary_dotfiles.to_owned(),
    };

    let mut all_layouts: Vec<KeyboardLayout> =
        xkb_layouts.into_iter().map(KeyboardLayout::Xkb).collect();
    all_layouts.push(KeyboardLayout::Alternative(kyria_layout));

    Ok(all_layouts)
}

fn switch_dotfiles(sh: &Shell, from_path: &str, to_path: &str) -> anyhow::Result<()> {
    let dotter_local_path = "/home/rg/.dotfiles/.dotter/local.toml";

    sh.change_dir(from_path);
    cmd!(sh, "dotter undeploy -y -l {dotter_local_path}").run()?;

    sh.change_dir(to_path);
    cmd!(sh, "dotter deploy -y -l {dotter_local_path}").run()?;

    Ok(())
}

fn set_layout(
    sh: &Shell,
    current_layout: &KeyboardLayout,
    new_layout: &KeyboardLayout,
) -> anyhow::Result<()> {
    match new_layout {
        KeyboardLayout::Xkb(xkb_layout) => {
            if let KeyboardLayout::Alternative(_) = current_layout {
                switch_dotfiles(
                    sh,
                    current_layout.dotfiles_path(),
                    new_layout.dotfiles_path(),
                )?;
            }
            set_hyprland_layout_by_id(xkb_layout.data.hyprland_id)?;
            new_layout.persisted_data().write()?;
        }
        KeyboardLayout::Alternative(_) => {
            if new_layout == current_layout {
                return Ok(());
            }

            switch_dotfiles(
                sh,
                current_layout.dotfiles_path(),
                new_layout.dotfiles_path(),
            )?;
            set_hyprland_layout_by_id(0)?;
            new_layout.persisted_data().write()?;
        }
    }

    Ok(())
}

fn run_hyprland(sh: &Shell, args: &ArgMatches) -> anyhow::Result<Option<String>> {
    let layouts = collect_all_layouts(sh)?;
    let current_layout = get_current_layout(&layouts)?;

    match args.subcommand() {
        Some(("init", _)) => initialize(current_layout)?,
        Some(("next", _)) => {
            // TODO: not sure what to do here yet...
            // it should not cycle between qwerty layouts and canary, because that switch is a bit
            // more elaborate than changing the xkb layout...
            // switching between qwerty only is ok, maybe I can keep that
            unimplemented!()
        }
        Some(("prev", _)) => {
            unimplemented!()
        }
        Some(("choose", _)) => {
            let chosen_layout = Dmenu::new(sh).choose_one(
                "Choose keyboard layout",
                &layouts,
                |layout| layout.name(),
                true,
            )?;
            set_layout(sh, current_layout, chosen_layout)?
        }
        Some(("get", _)) => println!("{}", current_layout),
        Some(("set", set_args)) => {
            let id = set_args
                .get_one::<usize>("ID")
                .expect("ID should be a required argument");
            let layout = layouts.get(*id).ok_or_else(|| {
                anyhow::anyhow!(
                    "Given layout index is out of bounds (max: {})",
                    layouts.len()
                )
            })?;
            set_layout(sh, current_layout, layout)?;
        }
        _ => {}
    };

    Ok(None)
}

fn initialize(current_layout: &KeyboardLayout) -> anyhow::Result<()> {
    if let KeyboardLayout::Xkb(layout) = current_layout {
        set_hyprland_layout_by_id(layout.data.hyprland_id)
    } else {
        Ok(())
    }
}

fn run_x11(_sh: &Shell, _args: &ArgMatches) -> anyhow::Result<Option<String>> {
    unimplemented!()
}

pub fn run(sh: &Shell, args: &ArgMatches) -> anyhow::Result<Option<String>> {
    let wm = determine_wm();

    match wm {
        WM::Hyprland => run_hyprland(sh, args),
        WM::GenericX11 => run_x11(sh, args),
    }
}

pub fn command_extension(cmd: Command) -> Command {
    let inner_subcommands = [
        Command::new("init").about("Initialize keyboard layout"),
        Command::new("next").about("Select the next keyboard layout"),
        Command::new("prev").about("Select the previous keyboard layout"),
        Command::new("choose").about("Choose a keyboard layout from the list of layouts"),
        Command::new("get").about("Get the current keyboard layout name"),
        Command::new("set")
            .about("Select the layout specified by name")
            .arg(
                arg!([ID] "Identifier (index) of the keyboard layout")
                    .value_parser(value_parser!(usize))
                    .required(true),
            ),
    ];
    cmd.subcommand_required(true)
        .subcommands(inner_subcommands.iter())
}

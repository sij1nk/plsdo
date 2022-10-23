#![feature(iter_intersperse)]

use clap::{command, ArgMatches, Command};
use xshell::Shell;

type Definition = (
    &'static str,
    &'static str,
    Option<fn(Command<'static>) -> Command<'static>>,
);
type Script = fn(&Shell, &ArgMatches) -> anyhow::Result<()>;

mod scripts;
mod util;

// TODO(rg):
// Possibly get this from build.rs? Would mean that we wouldn't have to touch the main.rs
// file when adding a new menu / script
const SCRIPTS: &[(Definition, Script)] = &[
    (
        ("power", "Shut down, reboot or suspend the machine", None),
        scripts::power::run,
    ),
    (
        ("keyboard_layout", "Change the keyboard layout", None),
        scripts::keyboard_layout::run,
    ),
    (
        (
            "font_size",
            "Change the font size",
            Some(scripts::font_size::command),
        ),
        scripts::font_size::run,
    ),
    (
        ("font_family", "Change the font family", None),
        scripts::font_family::run
    ),
    (
        ("playerctl", "Control media players", Some(scripts::playerctl::command)),
        scripts::playerctl::run,
    )
];

fn main() -> anyhow::Result<()> {
    // TODO(rg): shell is not always needed (e.g. font_size)
    let shell = Shell::new()?;

    let matches = command!()
        .subcommand_required(true)
        .subcommands(SCRIPTS.iter().map(|&((name, about, args), _)| {
            let base_command = Command::new(name).about(about);
            if let Some(args) = args {
                args(base_command)
            } else {
                base_command
            }
        }))
        .get_matches();

    let (subcmd_name, subcmd_args) = matches.subcommand().unwrap();
    let script = SCRIPTS
        .iter()
        .find(|&&((name, _, _), _)| name == subcmd_name)
        .unwrap()
        .1;

    script(&shell, subcmd_args)?;

    Ok(())
}

use clap::{command, ArgMatches, Command};
use define_scripts_macro::define_scripts;
use system_atlas::SystemAtlas;
use xshell::Shell;

/// TODO: unused
type Definition = (
    &'static str,                             // name
    &'static str,                             // description
    fn(Command<'static>) -> Command<'static>, // command extension
);
type Script = fn(&Shell, &ArgMatches, &SystemAtlas) -> anyhow::Result<()>;

mod scripts;
mod system_atlas;
mod util;

// Each plsdo script can be invoked as a subcommand on the plsdo command.
// Scripts are expected to live under the `scripts` folder, and must provide implementations for
// the `run` and `command` functions.
//
// TODO: consider calling scripts subcommands instead, because that's what they are
// TODO: consider turning scripts into struct which implement a trait that defines the signatures
// for `run` and `command`. Right now, it's unclear what signatures they're supposed to have,
// unless we read the macro definition
define_scripts!([
    (power, "Shut down, reboot or suspend the machine"),
    (keyboard_layout, "Change the keyboard layout"),
    (font_size, "Change the font size"),
    (font_family, "Change the font family"),
    (playerctl, "Control media players"),
    (game, "Launch a game through Lutris"),
    (eww, "Manage the eww daemon and eww widgets")
]);

fn main() -> anyhow::Result<()> {
    // TODO(rg): shell is not always needed (e.g. font_size)
    let shell = Shell::new()?;
    let atlas = SystemAtlas::new();

    let matches = command!()
        .subcommand_required(true)
        .subcommands(SCRIPTS.iter().map(|&((name, about, args), _)| {
            let base_command = Command::new(name).about(about);
            args(base_command)
        }))
        .get_matches();

    let (subcmd_name, subcmd_args) = matches.subcommand().unwrap();
    // TODO: unwrap
    let script = SCRIPTS
        .iter()
        .find(|&&((name, _, _), _)| name == subcmd_name)
        .unwrap()
        .1;

    script(&shell, subcmd_args, &atlas)?;

    Ok(())
}

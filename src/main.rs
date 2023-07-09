use clap::{command, ArgMatches, Command};
use define_scripts_macro::define_scripts;
use system_atlas::SystemAtlas;
use xshell::Shell;

type Definition = (
    &'static str,                             // name
    &'static str,                             // description
    fn(Command<'static>) -> Command<'static>, // command extension
);
type Script = fn(&Shell, &ArgMatches, &SystemAtlas) -> anyhow::Result<()>;

mod scripts;
mod system_atlas;
mod util;

define_scripts!([
    (power, "Shut down, reboot or suspend the machine"),
    (keyboard_layout, "Change the keyboard layout"),
    (font_size, "Change the font size"),
    (font_family, "Change the font family"),
    (playerctl, "Control media players"),
    (game, "Launch a game through Lutris")
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

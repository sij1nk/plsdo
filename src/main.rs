use clap::{command, ArgMatches, Command};
use define_subcommands_macro::define_subcommands;
use xshell::Shell;

mod subcommands;
mod system_atlas;
mod util;

type Definition = (
    &'static str,           // name
    &'static str,           // description
    fn(Command) -> Command, // command extension
);
type Script = fn(&Shell, &ArgMatches) -> anyhow::Result<()>;

// Each plsdo subcommand can be invoked as a subcommand on the plsdo command. Subcommands are
// expected to live under the `subcommands` folder, and must provide implementations for the `run`
// and `command_extension` functions.
// TODO: more flexibility may be needed later (e.g. subcommands broken into multiple source files)
define_subcommands!([
    (power, "Shut down, reboot or suspend the machine"),
    (keyboard_layout, "Change the keyboard layout"),
    (font_size, "Change the font size"),
    (font_family, "Change the font family"),
    (playerctl, "Control media players"),
    (game, "Launch a game through Lutris"),
    (workspace, "Manage desktop workspaces"),
    (brightness, "Adjust the screen brightness")
]);

fn main() -> anyhow::Result<()> {
    let shell = Shell::new()?;

    let matches = command!()
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommands(
            SUBCOMMANDS
                .iter()
                .map(|&((name, about, command_extension), _)| {
                    let base_command = Command::new(name).about(about);
                    command_extension(base_command)
                }),
        )
        .get_matches();

    let (subcmd_name, subcmd_args) = matches.subcommand().expect(
        "A subcommand is always received;
otherwise clap exits before getting this far",
    );
    let subcommand = SUBCOMMANDS
        .iter()
        .find(|&&((name, _, _), _)| name == subcmd_name)
        .expect("A valid subcommand name is supplied; otherwise clap exits")
        .1;

    subcommand(&shell, subcmd_args)?;

    Ok(())
}

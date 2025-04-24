use clap::{command, ArgMatches, Command};
use define_subcommands_macro::define_subcommands;
use xshell::Shell;

mod constants;
mod subcommands;
mod system_atlas;
mod util;

pub type Definition = (
    &'static str,           // name
    &'static str,           // description
    fn(Command) -> Command, // command extension
);
pub type Script = fn(&Shell, &ArgMatches) -> anyhow::Result<Option<String>>;

// Each plsdo subcommand can be invoked as a subcommand on the plsdo command. Subcommands are
// expected to live under the `subcommands` folder, and must provide implementations for the `run`
// and `command_extension` functions.
define_subcommands!([
    (power, "Shut down, reboot or suspend the machine"),
    (keyboard, "Change the keyboard layout"),
    (font_size, "Change the font size"),
    (font_family, "Change the font family"),
    (playerctl, "Control media players"),
    (game, "Launch a game through Lutris"),
    (workspace, "Manage desktop workspaces"),
    (brightness, "Adjust the screen brightness"),
    (colortemp, "Adjust the screen color temperature"),
    (audio, "Adjust the audio volume or output"),
    (ytdl, "Download videos using yt-dlp"),
    (torrent, "Manage torrents"),
    (screenshot, "Take screenshots")
]);

pub fn run_subcommand(shell: &Shell, matches: &ArgMatches) -> anyhow::Result<Option<String>> {
    let (subcmd_name, subcmd_args) = matches.subcommand().expect(
        "A subcommand is always received;
otherwise clap exits before getting this far",
    );

    let subcommand = SUBCOMMANDS
        .iter()
        .find(|&&((name, _, _), _)| name == subcmd_name)
        .map(|found| found.1)
        .expect("A valid subcommand name is supplied; otherwise clap exists");

    subcommand(shell, subcmd_args)
}

pub fn get_command() -> Command {
    command!()
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
}

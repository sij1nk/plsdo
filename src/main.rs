use xshell::Shell;
use clap::{command, Command};

type Menu = fn(&Shell) -> anyhow::Result<()>;

mod menus;
mod util;

// TODO(rg): 
// Possibly get this from build.rs? Would mean that we wouldn't have to touch the main.rs
// file when adding a new menu / script
const MENUS: &[(&str, &str, Menu)] = &[
    ("power", "Shut down, reboot or suspend the machine", menus::power::run),
    ("keyboard_layout", "Change the keyboard layout", menus::keyboard_layout::run)
];

fn main() -> anyhow::Result<()> {
    let shell = Shell::new()?;
    
    let matches = command!()
        .subcommand_required(true)
        .subcommands(MENUS.iter().map(|menu| Command::new(menu.0).about(menu.1)))
        .get_matches();

    let menu_name = matches.subcommand().unwrap().0;
    let menu = MENUS.iter().find(|&&elem| elem.0 == menu_name).unwrap().2;

    menu(&shell)?;

    Ok(())
}

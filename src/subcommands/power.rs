use clap::{ArgMatches, Command};
use std::str::FromStr;
use std::string::ToString;
use strum::IntoEnumIterator;
use strum_macros::{Display, EnumIter, EnumString};
use xshell::{cmd, Shell};

use crate::util::dmenu::get_platform_dmenu;

#[derive(Debug, Display, EnumString, EnumIter)]
#[strum(serialize_all = "lowercase")]
enum PowerMenuOption {
    Shutdown,
    Suspend,
    Reboot,
}

impl PowerMenuOption {
    fn execute(&self, sh: &Shell) -> anyhow::Result<()> {
        let cmd = match *self {
            PowerMenuOption::Shutdown => cmd!(sh, "shutdown now"),
            PowerMenuOption::Suspend => cmd!(sh, "systemctl suspend"),
            PowerMenuOption::Reboot => cmd!(sh, "reboot"),
        };

        cmd.run()?;

        Ok(())
    }
}

pub fn command_extension(cmd: Command) -> Command {
    cmd
}

pub fn run(sh: &Shell, _: &ArgMatches) -> anyhow::Result<Option<String>> {
    let opts: Vec<_> = PowerMenuOption::iter().map(|opt| opt.to_string()).collect();
    let opts_str = opts.iter().map(|i| i.as_ref()).collect::<Vec<_>>();
    let dmenu = get_platform_dmenu();

    let result = dmenu.choose_one(sh, "Choose operation", &opts_str, true)?;

    PowerMenuOption::from_str(&result)?.execute(sh)?;

    Ok(None)
}

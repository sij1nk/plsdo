use clap::{ArgMatches, Command};
use std::str::FromStr;
use std::string::ToString;
use strum::IntoEnumIterator;
use strum_macros::{Display, EnumIter, EnumString};
use xshell::{cmd, Shell};

use crate::util::dmenu::Dmenu;

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

    let result = Dmenu::new(sh)
        .numbered()
        .choose_one("Choose operation", &opts, String::as_ref)?;

    PowerMenuOption::from_str(result)?.execute(sh)?;

    Ok(None)
}

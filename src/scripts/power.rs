use clap::ArgMatches;
use std::str::FromStr;
use std::string::ToString;
use strum::IntoEnumIterator;
use strum_macros::{Display, EnumIter, EnumString};
use xshell::{cmd, Shell};

use crate::util::dmenu;

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
            PowerMenuOption::Suspend => cmd!(sh, "reboot"),
            PowerMenuOption::Reboot => cmd!(sh, "systemctl suspend"),
        };

        cmd.run()?;

        Ok(())
    }
}

pub fn run(sh: &Shell, _: &ArgMatches) -> anyhow::Result<()> {
    let opts: Vec<_> = PowerMenuOption::iter().map(|opt| opt.to_string()).collect();

    // unwrap: don't want to continue if string is empty 
    let result = dmenu(sh, "Choose operation", &opts, true).unwrap();

    PowerMenuOption::from_str(&result)?.execute(sh)?;

    Ok(())
}

use clap::ArgMatches;
use std::str::FromStr;
use std::string::ToString;
use strum::IntoEnumIterator;
use strum_macros::{Display, EnumIter, EnumString};
use xshell::{cmd, Shell};

use crate::{system_atlas::SystemAtlas, util::dmenu};

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

pub fn run(sh: &Shell, _: &ArgMatches, _: &SystemAtlas) -> anyhow::Result<()> {
    let opts: Vec<_> = PowerMenuOption::iter().map(|opt| opt.to_string()).collect();

    let result = dmenu(sh, "Choose operation", &opts, true)?;

    PowerMenuOption::from_str(&result)?.execute(sh)?;

    Ok(())
}

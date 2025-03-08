use anyhow::Context;
use xshell::{cmd, Shell};

fn is_wayland_session() -> bool {
    std::env::vars()
        .find(|(k, _)| k == "XDG_SESSION_TYPE")
        .map(|(_, v)| v == "wayland")
        .unwrap_or(false)
}

pub fn get_platform_dmenu() -> Box<dyn Dmenu> {
    if is_wayland_session() {
        Box::new(WaylandDmenu {})
    } else {
        Box::new(X11Dmenu {})
    }
}

pub trait Dmenu {
    fn choose_one(
        &self,
        sh: &Shell,
        prompt: &str,
        choices: &[&str],
        forbid_invalid: bool,
    ) -> anyhow::Result<String>;
}

pub struct WaylandDmenu;

impl Dmenu for WaylandDmenu {
    fn choose_one(
        &self,
        sh: &Shell,
        prompt: &str,
        choices: &[&str],
        forbid_invalid: bool,
    ) -> anyhow::Result<String> {
        let choices_joined = choices.join("\n");
        let lines = choices.len();
        let lines_str = format!("{lines}");

        let chosen = cmd!(sh, "bemenu -l {lines_str} --prompt {prompt}")
            .stdin(&choices_joined)
            .read()
            .map_err(anyhow::Error::new);

        let chosen = chosen.context("Aborted")?;

        if forbid_invalid && !choices_joined.contains(&chosen) {
            anyhow::bail!("Invalid input given");
        }

        Ok(chosen)
    }
}

struct X11Dmenu;

impl Dmenu for X11Dmenu {
    fn choose_one(
        &self,
        sh: &Shell,
        prompt: &str,
        choices: &[&str],
        forbid_invalid: bool,
    ) -> anyhow::Result<String> {
        let choices_joined = choices.join("\n");
        let lines = choices.len().min(10);
        let lines_str = format!("{lines}");

        let chosen: anyhow::Result<String> = cmd!(
            sh,
            "dmenu -p {prompt} -i -l {lines_str} -fn 'monospace:size=24'"
        )
        .stdin(&choices_joined)
        .read()
        .map_err(anyhow::Error::new);

        let chosen = chosen.context("Aborted")?;

        if forbid_invalid && !choices_joined.contains(&chosen) {
            anyhow::bail!("Invalid input given")
        }

        Ok(chosen)
    }
}

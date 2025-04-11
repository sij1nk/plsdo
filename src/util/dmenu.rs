use anyhow::Context;
use xshell::{cmd, Shell};

pub struct Dmenu<'a> {
    sh: &'a Shell,
    dmenu_fn: fn(&Shell, &str, String, usize) -> anyhow::Result<String>,
}

impl<'a> Dmenu<'a> {
    pub fn new(sh: &'a Shell) -> Self {
        // TODO: nicer thing do to would be to lazy_static / once_cell this at the start of the
        // program?
        let dmenu_fn = if is_wayland_session() {
            wayland_dmenu
        } else {
            x11_dmenu
        };
        Self { sh, dmenu_fn }
    }

    pub fn choose_one<'c, T>(
        &self,
        prompt: &str,
        choices: &'c [T],
        stringifier: impl Fn(&'c T) -> &'c str,
        forbid_invalid: bool,
    ) -> anyhow::Result<&'c T> {
        let choice_strs = choices.iter().map(stringifier).collect::<Vec<_>>();

        let chosen = self.choose_one_str(prompt, &choice_strs, forbid_invalid)?;
        let i = choice_strs
            .iter()
            .position(|&choice| choice == chosen)
            .ok_or_else(|| anyhow::anyhow!("Chosen string is not recognized"))?;

        Ok(&choices[i])
    }

    pub fn choose_one_str(
        &self,
        prompt: &str,
        choices: &[&str],
        forbid_invalid: bool,
    ) -> anyhow::Result<String> {
        let choices_string = choices.join("\n");
        let chosen =
            (self.dmenu_fn)(self.sh, prompt, choices_string, choices.len()).context("Aborted")?;

        if forbid_invalid && !choices.contains(&chosen.as_str()) {
            anyhow::bail!("Invalid input given");
        }

        Ok(chosen)
    }
}

fn is_wayland_session() -> bool {
    std::env::vars()
        .find(|(k, _)| k == "XDG_SESSION_TYPE")
        .map(|(_, v)| v == "wayland")
        .unwrap_or(false)
}

fn x11_dmenu(
    sh: &Shell,
    prompt: &str,
    choices_string: String,
    choices_len: usize,
) -> anyhow::Result<String> {
    let lines = choices_len.min(10);
    let lines_str = format!("{lines}");

    cmd!(
        sh,
        "dmenu -p {prompt} -i -l {lines_str} -fn 'monospace:size=24'"
    )
    .stdin(&choices_string)
    .read()
    .map_err(anyhow::Error::new)
}

fn wayland_dmenu(
    sh: &Shell,
    prompt: &str,
    choices_string: String,
    choices_len: usize,
) -> anyhow::Result<String> {
    let lines_str = format!("{choices_len}");

    cmd!(sh, "bemenu -l {lines_str} --prompt {prompt}")
        .stdin(&choices_string)
        .read()
        .map_err(anyhow::Error::new)
}

use anyhow::Context;
use xshell::{cmd, Shell};

pub struct Dmenu<'a> {
    sh: &'a Shell,
    dmenu_fn: fn(&Shell, &str, String, usize, bool, bool) -> anyhow::Result<String>,
    allow_invalid: bool,
    numbered: bool,
    auto_select: bool,
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
        Self {
            sh,
            dmenu_fn,
            allow_invalid: false,
            numbered: false,
            auto_select: false,
        }
    }

    pub fn allow_invalid(mut self) -> Self {
        self.allow_invalid = true;
        self
    }

    pub fn numbered(mut self) -> Self {
        self.numbered = true;
        self
    }

    pub fn auto_select(mut self) -> Self {
        self.auto_select = true;
        self
    }

    pub fn choose_one<'c, T>(
        &self,
        prompt: &str,
        choices: &'c [T],
        stringifier: impl Fn(&'c T) -> &'c str,
    ) -> anyhow::Result<&'c T> {
        let choice_strs = choices.iter().map(stringifier).collect::<Vec<_>>();

        let chosen = self.choose_one_str(prompt, &choice_strs)?;
        let i = choice_strs
            .iter()
            .position(|&choice| choice == chosen)
            .ok_or_else(|| anyhow::anyhow!("Chosen string is not recognized"))?;

        Ok(&choices[i])
    }

    pub fn choose_one_str(&self, prompt: &str, choices: &[&str]) -> anyhow::Result<String> {
        let choices_string = if self.numbered {
            let numbered_choices = choices
                .iter()
                .enumerate()
                .map(|(i, s)| format!("_{}: {}", i + 1, s))
                .collect::<Vec<_>>();
            numbered_choices.join("\n")
        } else {
            choices.join("\n")
        };
        let mut chosen = (self.dmenu_fn)(
            self.sh,
            prompt,
            choices_string,
            choices.len(),
            self.numbered,
            self.auto_select,
        )
        .context("Aborted")?;

        if self.numbered {
            let (_number_prefix, chosen_str) =
                chosen.split_once(':').expect("chosen to be numbered");
            chosen = chosen_str.trim_start().to_owned()
        }

        if !self.allow_invalid && !choices.contains(&chosen.as_str()) {
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
    _numbered: bool,    // dmenu does not support filtering
    _auto_select: bool, // dmenu does not support auto select
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
    numbered: bool,
    auto_select: bool,
) -> anyhow::Result<String> {
    let lines_str = format!("{choices_len}");
    let auto_select_prompt = if auto_select { " [AS]" } else { "" };
    let auto_select = if auto_select { "--auto-select" } else { "" };
    let filter = if numbered { "-F_" } else { "" };

    cmd!(
        sh,
        "bemenu -l {lines_str} --prompt {prompt}{auto_select_prompt} {auto_select} {filter}"
    )
    .stdin(&choices_string)
    .read()
    .map_err(anyhow::Error::new)
}

pub struct SystemAtlas<'a> {
    pub alacritty: &'a str,
    pub fontconfig: &'a str,
}

impl<'a> SystemAtlas<'a> {
    pub fn new() -> Self {
        Self {
            alacritty: "~/.config/alacritty/alacritty.yaml",
            fontconfig: "~/.config/fontconfig/fonts.conf",
        }
    }
}

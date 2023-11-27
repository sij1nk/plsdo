pub struct SystemAtlas<'a> {
    pub alacritty: &'a str,
    pub fontconfig: &'a str,
    pub eww_brightness: &'a str,
    pub eww_gamma: &'a str,
    pub eww_volume: &'a str,
    pub eww_show_all: &'a str,
    pub eww_workspaces: &'a str,
}

impl<'a> SystemAtlas<'a> {
    pub fn new() -> Self {
        Self {
            alacritty: "~/.config/alacritty/alacritty.yaml",
            fontconfig: "~/.config/fontconfig/fonts.conf",
            eww_brightness: "/tmp/eww-brightness",
            eww_gamma: "/tmp/eww-gamma",
            eww_volume: "/tmp/eww-volume",
            eww_show_all: "/tmp/eww-show-all",
            eww_workspaces: "/tmp/eww-workspaces",
        }
    }
}

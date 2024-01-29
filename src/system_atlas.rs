/// Collection of system paths which we are interested about.
/// NOTE: File::open does not expand '~', so it's safer to specify the full path!
pub struct SystemAtlas<'a> {
    pub alacritty: &'a str,
    pub fontconfig: &'a str,
    pub eww_brightness: &'a str,
    pub eww_colortemp: &'a str,
    pub eww_volume: &'a str,
    pub eww_workspaces: &'a str,
    pub eww_keyboard_layout: &'a str,
    pub hyprland: &'a str,
}

pub const SYSTEM_ATLAS: SystemAtlas = SystemAtlas {
    alacritty: "/home/rg/.config/alacritty/alacritty.yaml",
    fontconfig: "/home/rg/.config/fontconfig/fonts.conf",
    eww_brightness: "/tmp/eww-brightness",
    eww_colortemp: "/tmp/eww-colortemp",
    eww_volume: "/tmp/eww-volume",
    eww_workspaces: "/tmp/eww-workspaces",
    eww_keyboard_layout: "/tmp/eww-keyboard-layout",
    hyprland: "/home/rg/.config/hypr/hyprland.conf",
};

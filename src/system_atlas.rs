/// Collection of system paths which we are interested about.
/// NOTE: File::open does not expand '~', so it's safer to specify the full path!
pub struct SystemAtlas<'a> {
    pub alacritty: &'a str,
    pub fontconfig: &'a str,
    pub eww_brightness: &'a str,
    pub eww_colortemp: &'a str,
    pub eww_audio: &'a str,
    pub eww_workspaces: &'a str,
    pub keyboard_layout: &'a str,
    pub ytdl_aggregator_socket: &'a str,
    pub hypr_submap: &'a str,
    pub main_dotfiles: &'a str,
    pub canary_dotfiles: &'a str,
}

pub const SYSTEM_ATLAS: SystemAtlas = SystemAtlas {
    alacritty: "/home/rg/.config/alacritty/alacritty.yaml",
    fontconfig: "/home/rg/.config/fontconfig/fonts.conf",
    eww_brightness: "/tmp/eww-brightness",
    eww_colortemp: "/tmp/eww-colortemp",
    eww_audio: "/tmp/eww-audio",
    eww_workspaces: "/tmp/eww-workspaces",
    keyboard_layout: "/home/rg/.cache/keyboard-layout",
    ytdl_aggregator_socket: "/tmp/plsdo-ytdl-aggregator.sock",
    hypr_submap: "/tmp/hypr-submap",
    main_dotfiles: "/home/rg/.dotfiles",
    canary_dotfiles: "/home/rg/.dotfiles__canary",
};

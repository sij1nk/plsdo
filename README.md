# plsdo

All the shell scripts I use to complement my "bare-bones" Linux setup,
rewritten in Rust.

(forever WIP, I'm afraid 😔)

```plaintext
Usage: plsdo <COMMAND>

Commands:
  power            Shut down, reboot or suspend the machine
  keyboard_layout  Change the keyboard layout
  font_size        Change the font size
  font_family      Change the font family
  playerctl        Control media players
  game             Launch a game through Lutris
  workspace        Manage desktop workspaces
  brightness       Adjust the screen brightness
  colortemp        Adjust the screen color temperature
  audio            Adjust the audio volume or output
  ytdl             Download videos using yt-dlp
  help             Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

## But... why?

1. I like customizability. I want to configure my setup to be **exactly** the
   way I want it to look and function like. I do not want to clutter my setup
   with functionality that I never use. I feel like achieving this goal by
   starting out with a minimal setup, and extending it with the stuff you want, is
   a lot less difficult and messy, than stripping down a fully fledged OS like
   Windows, or a DE like Gnome.
2. During the past few years, I've gradually transitioned into a mostly
   keyboard-driven workflow. I don't want to take my hands off the keyboard,
   grab the mouse, and click around in some awkwardly constructed GUI to do
   something, if I can configure a keyboard shortcut for it.
3. I like the concept of shell scripting, but I hate the weirdness of the
   syntax and all the various esoteric details you have to keep in mind. Any
   script over 100 lines becomes unmanageable in my experience.
4. I like writing Rust

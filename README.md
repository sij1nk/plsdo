# plsdo

All the shell scripts I use to complement my "bare-bones" Linux setup, rewritten in Rust. WIP

```
USAGE:
    plsdo <SUBCOMMAND>

OPTIONS:
    -h, --help       Print help information
    -V, --version    Print version information

SUBCOMMANDS:
    font_family        Change the font family
    font_size          Change the font size
    game               Launch a game through Lutris
    help               Print this message or the help of the given subcommand(s)
    keyboard_layout    Change the keyboard layout
    playerctl          Control media players
    power              Shut down, reboot or suspend the machine

    (... and hopefully many more to come!)
```

## But... why?

1. I like customizability. I want to configure my setup to be **exactly** the way I want it to
   look and function like. I do not want to clutter my setup with functionality that I never use.
   I feel like the approach of starting out with a minimal setup and extending it with the stuff
   you want is a lot less difficult and messy than stripping down a fully fledged OS like Windows,
   or a DE like Gnome.
2. During the past few years, I've gradually transitioned into a mostly keyboard-driven workflow.
   Most of what I do (aside from gaming, and drooling over myself while binging Youtube) involves
   text manipulation. Why should I take my hand off the keyboard, grab the mouse, and navigate
   around in awkwardly constructed GUIs to achieve something, when I could configure a keyboard
   shortcut to `plsdo` to perform the exact same action in a fraction of the time?
3. I like the concept of shell scripting, but I hate the weirdness of the syntax and all the
   various esoteric details you have to keep in mind. Any script over 100 lines becomes
   unmanageable in my experience. 
4. Using a more general-purpose language would make it easier to abstract over functionality,
   simplifying the process of adding new scripts (at least, this is my hypothesis). I chose Rust
   because it's performant (not that it matters too much in this case), and because I like writing
   Rust and I wish to get better at it.

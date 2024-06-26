use clap::{arg, value_parser, ArgMatches, Command};
use gio::{prelude::AppInfoExt, AppInfo, AppLaunchContext};
use hyprland::{
    data::{Clients, Monitors},
    dispatch::{Dispatch, DispatchType},
    shared::{HyprData, WorkspaceId},
};
use xshell::{cmd, Shell};

mod listener;

/// A program whose window is pinned to a specific workspace. The window should always be opened on
/// this workspace, but can be freely moved to other workspaces afterwards. Opening the pinned
/// program again would not launch a new instance of the program - instead, it would navigate to
/// the workspace where the first instance's window is.
/// The program *must* have a desktop entry associated with it
/// * `name`: value of the `Name` field in the desktop entry
/// * `wm_class`: the WM class of the window
/// * `workspace`: the id of workspace on which the pinned program's window is opened by default
struct PinnedProgram<'a> {
    name: &'a str,
    wm_class: &'a str,
}

pub fn command_extension(cmd: Command) -> Command {
    let inner_subcommands = [
        Command::new("focus")
            .about("Move focus to the specified workspace")
            .arg_required_else_help(true)
            .subcommand_required(true)
            .subcommands(
                [
                    Command::new("next")
                        .about("Move focus to the next workspace on the specified monitor")
                        .arg_required_else_help(true)
                        .arg(
                            arg!([MONITOR] "Identifier of the monitor")
                                .value_parser(value_parser!(u8)),
                        ),
                    Command::new("next-current")
                        .about("Move focus to the next workspace on the current monitor"),
                    Command::new("prev")
                        .about("Move focus to the previous workspace on the specified monitor")
                        .arg_required_else_help(true)
                        .arg(
                            arg!([MONITOR] "Identifier of the monitor")
                                .value_parser(value_parser!(u8)),
                        ),
                    Command::new("prev-current")
                        .about("Move focus to the next workspace on the current monitor"),
                    Command::new("id")
                        .about("Move focus to the workspace with the given identifier")
                        .arg_required_else_help(true)
                        .arg(
                            arg!([WORKSPACE] "Identifier of the workspace")
                                .value_parser(value_parser!(WorkspaceId)),
                        ),
                ]
                .iter(),
            ),
        Command::new("move")
            .about("Move focus and the current window to the specified workspace")
            .arg_required_else_help(true)
            .subcommand_required(true)
            .subcommands(
                [
                    Command::new("next")
                        .about("Move focus and the current window to the next workspace on the specified monitor")
                        .arg_required_else_help(true)
                        .arg(
                            arg!([MONITOR] "Identifier of the monitor")
                                .value_parser(value_parser!(u8)),
                        ),
                    Command::new("next-current")
                        .about("Move focus and the current window to the next workspace on the current monitor"),
                    Command::new("prev")
                        .about("Move focus and the current window to the previous workspace on the specified monitor")
                        .arg_required_else_help(true)
                        .arg(
                            arg!([MONITOR] "Identifier of the monitor")
                                .value_parser(value_parser!(u8)),
                        ),
                    Command::new("prev-current")
                        .about("Move focus and the current window to the next workspace on the current monitor"),
                    Command::new("id")
                        .about("Move focus and the current window to the workspace with the given identifier")
                        .arg_required_else_help(true)
                        .arg(
                            arg!([WORKSPACE] "Identifier of the workspace")
                                .value_parser(value_parser!(WorkspaceId)),
                        ),
                ]
                .iter(),
            ),
        Command::new("open_pinned")
            .about("Open and navigate to a pinned window")
            .arg_required_else_help(true)
            .arg(arg!([PROGRAM] "The name of the program whose pinned window to navigate to")),
        Command::new("run_listener")
            .about("Launch the hypr event listener process")
    ];
    cmd.subcommand_required(true)
        .arg_required_else_help(true)
        .subcommands(inner_subcommands.iter())
}

fn focus_window_by_wm_class(wm_class: &str) -> anyhow::Result<()> {
    let dispatch = DispatchType::FocusWindow(
        hyprland::dispatch::WindowIdentifier::ClassRegularExpression(wm_class),
    );
    Dispatch::call(dispatch)?;
    Ok(())
}

/// Get the next/prev workspace's id on the given monitor.
/// This impl assumes a couple things:
/// - number of total workspaces is 10
/// - primary monitor has all the odd numbered workspaces
/// - secondary monitor has all the even numbered workspaces
/// * `monitor_id`: the id of the monitor on which we change the workspace
/// * `delta`: how much to increase the current workspace id by
fn get_relative_workspace_id_on_monitor(
    monitor_id: u8,
    delta: WorkspaceId,
) -> anyhow::Result<WorkspaceId> {
    let workspace_count = 10;
    let (primary_active_id, secondary_active_id) = get_active_workspace_ids()?;
    let workspace_id = if monitor_id != 0 {
        primary_active_id
    } else {
        secondary_active_id
    };

    let mut next_workspace_id = workspace_id + delta;
    if next_workspace_id <= 0 {
        next_workspace_id += workspace_count
    } else if next_workspace_id > workspace_count {
        next_workspace_id -= workspace_count;
    }
    Ok(next_workspace_id)
}

/// Get the currently focused / active (~ where the cursor is) monitor.
/// Hyprland-rs does not provide the equivalent of `hyprctl activeworkspace`, so we're invoking it
/// by hand.
/// * `sh`: a shell
fn get_active_monitor_id(sh: &Shell) -> anyhow::Result<u8> {
    let err = "Could not find monitor id in `hyprctl activeworkspace output";
    let output = cmd!(sh, "hyprctl activeworkspace").read()?;
    let monitor = output
        .lines()
        .find(|line| line.trim_start().starts_with("monitorID"))
        .ok_or(anyhow::anyhow!(err))?
        .split_once(' ')
        .ok_or(anyhow::anyhow!(err))?
        .1
        .parse::<u8>()?;

    Ok(monitor)
}

fn get_focus_workspace_dispatcher<'a>(
    workspace_id: WorkspaceId,
    move_window: bool,
) -> DispatchType<'a> {
    if move_window {
        DispatchType::MoveToWorkspace(
            hyprland::dispatch::WorkspaceIdentifierWithSpecial::Id(workspace_id),
            None,
        )
    } else {
        DispatchType::Workspace(hyprland::dispatch::WorkspaceIdentifierWithSpecial::Id(
            workspace_id,
        ))
    }
}

fn focus_workspace(sh: &Shell, args: &ArgMatches, move_window: bool) -> anyhow::Result<()> {
    match args.subcommand() {
        Some(("next", next_args)) => {
            let monitor_id = next_args
                .get_one::<u8>("MONITOR")
                .expect("MONITOR should be a required argument");
            let workspace_id = get_relative_workspace_id_on_monitor(*monitor_id, 2)?;
            let dispatch = get_focus_workspace_dispatcher(workspace_id, move_window);
            Dispatch::call(dispatch)?;
        }
        Some(("next-current", _)) => {
            let monitor_id = get_active_monitor_id(sh)?;
            let workspace_id = get_relative_workspace_id_on_monitor(monitor_id, 2)?;
            let dispatch = get_focus_workspace_dispatcher(workspace_id, move_window);
            Dispatch::call(dispatch)?;
        }
        Some(("prev", prev_args)) => {
            let monitor_id = prev_args
                .get_one::<u8>("MONITOR")
                .expect("MONITOR should be a required argument");
            let workspace_id = get_relative_workspace_id_on_monitor(*monitor_id, -2)?;
            let dispatch = get_focus_workspace_dispatcher(workspace_id, move_window);
            Dispatch::call(dispatch)?;
        }
        Some(("prev-current", _)) => {
            let monitor_id = get_active_monitor_id(sh)?;
            let workspace_id = get_relative_workspace_id_on_monitor(monitor_id, -2)?;
            let dispatch = get_focus_workspace_dispatcher(workspace_id, move_window);
            Dispatch::call(dispatch)?;
        }
        Some(("id", id_args)) => {
            let id = id_args
                .get_one::<WorkspaceId>("WORKSPACE")
                .expect("WORKSPACE should be a required argument");
            let dispatch = get_focus_workspace_dispatcher(*id, move_window);
            Dispatch::call(dispatch)?;
        }
        _ => return Ok(()),
    };

    Ok(())
}

fn open_pinned(args: &ArgMatches) -> anyhow::Result<()> {
    let program_name = args
        .get_one::<String>("PROGRAM")
        .expect("PROGRAM should be a required argument");

    let pinned_programs = [
        PinnedProgram {
            name: "newsboat",
            wm_class: "newsboat",
        },
        PinnedProgram {
            name: "ncmpcpp",
            wm_class: "ncmpcpp",
        },
        PinnedProgram {
            name: "btop",
            wm_class: "btop",
        },
        PinnedProgram {
            name: "pulsemixer",
            wm_class: "pulsemixer",
        },
        PinnedProgram {
            name: "notes",
            wm_class: "notes",
        },
        PinnedProgram {
            name: "Firefox Web Browser",
            wm_class: "firefox",
        },
    ];

    let Some(pinned_program) = pinned_programs
        .iter()
        .find(|program| program.name == program_name)
    else {
        return Err(anyhow::anyhow!(
            "The program '{}' is not a pinned program",
            program_name
        ));
    };

    if let Some(already_running_program) = Clients::get()?
        .into_iter()
        .find(|cl| cl.class == pinned_program.wm_class)
    {
        focus_window_by_wm_class(&already_running_program.class)?;
    } else {
        let appinfos = AppInfo::all();
        let Some(appinfo) = appinfos
            .iter()
            .find(|appinfo| appinfo.name() == pinned_program.name)
        else {
            return Err(anyhow::anyhow!(
                "Could not find desktop entry for the pinned program '{}'",
                pinned_program.name
            ));
        };

        // focus_workspace_by_id(pinned_program.workspace_id)?;
        appinfo.launch(&[], AppLaunchContext::NONE)?;
        focus_window_by_wm_class(pinned_program.wm_class)?;
    };

    Ok(())
}

/// Get the active workspace ids for the primary and secondary monitor.
/// This assumes that exactly 2 monitors are connected.
pub fn get_active_workspace_ids() -> anyhow::Result<(WorkspaceId, WorkspaceId)> {
    let mut active_workspace_ids = Monitors::get()?
        .into_iter()
        .map(|mon| mon.active_workspace.id)
        .collect::<Vec<_>>();
    active_workspace_ids.sort_by(|&id1, _| {
        if id1 % 2 == 1 {
            std::cmp::Ordering::Less
        } else {
            std::cmp::Ordering::Greater
        }
    });

    let primary = active_workspace_ids
        .first()
        .ok_or(anyhow::anyhow!("Found no active workspaces, expected two"))?;
    let secondary = active_workspace_ids.get(1).ok_or(anyhow::anyhow!(
        "Only found one active workspace, expected two"
    ))?;

    Ok((*primary, *secondary))
}

pub fn run(sh: &Shell, args: &ArgMatches) -> anyhow::Result<()> {
    match args.subcommand() {
        Some(("focus", focus_args)) => focus_workspace(sh, focus_args, false),
        Some(("move", move_args)) => focus_workspace(sh, move_args, true),
        Some(("open_pinned", open_pinned_args)) => open_pinned(open_pinned_args),
        Some(("run_listener", run_listener_args)) => listener::run(run_listener_args),
        _ => Ok(()),
    }?;

    Ok(())
}

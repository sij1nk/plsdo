use std::{
    collections::{BTreeSet, HashMap},
    fs::OpenOptions,
    io::{LineWriter, Write},
};

use anyhow::Context;
use clap::{arg, value_parser, ArgMatches, Command, ValueEnum};
use gio::{prelude::AppInfoExt, AppInfo, AppLaunchContext};
use hyprland::{
    data::{Clients, Monitors},
    dispatch::{Dispatch, DispatchType},
    shared::{HyprData, WorkspaceId},
};
use serde::Serialize;
use xshell::{cmd, Shell};

use crate::system_atlas::SYSTEM_ATLAS;

mod listener;

#[derive(Serialize, Debug)]
struct WorkspaceState {
    active_primary_workspace_id: WorkspaceId,
    active_secondary_workspace_id: Option<WorkspaceId>,
    occupied_workspace_ids: Vec<WorkspaceId>,
}

/// Append the state of the workspaces to a file, from which the eww workspaces widget can read it
/// from.
pub fn write_workspace_state_to_backing_file() -> anyhow::Result<()> {
    let active_workspaces = get_active_workspaces()?;

    let active_primary_workspace_id =
        *active_workspaces
            .get(Monitor::Primary.get_name())
            .ok_or(anyhow::anyhow!(
                "There is no active workspace on the primary monitor"
            ))?;
    let active_secondary_workspace_id = active_workspaces
        .get(Monitor::Secondary.get_name())
        .copied();

    let occupied_workspace_ids = Clients::get()?
        .into_iter()
        .map(|cl| cl.workspace.id)
        .filter(|&id| id > 0)
        .collect::<BTreeSet<_>>();

    let workspace_state = WorkspaceState {
        active_primary_workspace_id,
        active_secondary_workspace_id,
        occupied_workspace_ids: occupied_workspace_ids.into_iter().collect::<Vec<_>>(),
    };

    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(SYSTEM_ATLAS.eww_workspaces)?;
    let mut writer = LineWriter::new(&file);

    serde_json::to_writer(&mut writer, &workspace_state)?;
    writer.write_all(b"\n")?;

    Ok(())
}

pub fn update_system_bar_layout() -> anyhow::Result<()> {
    let monitor_configuration =
        MonitorConfiguration::get().context("Failed to handle monitor added event")?;

    let sh = Shell::new()?;
    cmd!(sh, "eww close-all").run()?;

    match monitor_configuration {
        MonitorConfiguration::Single => cmd!(sh, "eww open primary-single").run()?,
        MonitorConfiguration::Dual => cmd!(sh, "eww open-many primary secondary").run()?,
        _ => {}
    };

    Ok(())
}

#[derive(Debug, Clone)]
enum MonitorConfiguration {
    Single,
    Dual,
    Unrecognized,
}

impl MonitorConfiguration {
    fn get() -> anyhow::Result<Self> {
        let active_workspaces = get_active_workspaces()?;

        let monitors = active_workspaces
            .keys()
            .map(|mon_name| Monitor::try_from(mon_name.as_ref()))
            .collect::<Result<Vec<Monitor>, _>>();

        match monitors {
            Ok(monitors) => {
                let is_single = monitors.len() == 1 && monitors.contains(&Monitor::Primary);
                let is_dual = monitors.len() == 2;

                if is_single {
                    return Ok(Self::Single);
                }
                if is_dual {
                    return Ok(Self::Dual);
                }
            }
            Err(e) => {
                println!("There are unrecognized monitors connected; Error: {}", e);
                return Ok(Self::Unrecognized);
            }
        }

        Ok(Self::Unrecognized)
    }
}

#[derive(ValueEnum, Debug, Clone, Hash, Eq, PartialEq)]
enum Monitor {
    Primary,
    Secondary,
}

type HyprMonitorName<'a> = &'a str;

impl Monitor {
    fn get_name(&self) -> &str {
        match self {
            Self::Primary => "HDMI-A-1",
            Self::Secondary => "DP-1",
        }
    }

    fn get_first_and_last_workspace_id(
        &self,
        is_single: bool,
    ) -> Option<(WorkspaceId, WorkspaceId)> {
        match self {
            Self::Primary => {
                let last = if is_single { 10 } else { 5 };
                Some((1, last))
            }
            Self::Secondary => {
                if is_single {
                    None
                } else {
                    Some((6, 10))
                }
            }
        }
    }
}

impl TryFrom<HyprMonitorName<'_>> for Monitor {
    type Error = anyhow::Error;

    fn try_from(value: HyprMonitorName) -> Result<Self, Self::Error> {
        match value {
            "HDMI-A-1" => Ok(Self::Primary),
            "DP-1" => Ok(Self::Secondary),
            _ => Err(anyhow::anyhow!("Found unexpected monitor name {}", value)),
        }
    }
}

enum WorkspaceDirection {
    Next,
    Previous,
}

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
        Command::new("init").about("Initialize workspaces and workspace-related information"),
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
                                .value_parser(value_parser!(Monitor))
                                .required(true),
                        ),
                    Command::new("next-current")
                        .about("Move focus to the next workspace on the current monitor"),
                    Command::new("prev")
                        .about("Move focus to the previous workspace on the specified monitor")
                        .arg_required_else_help(true)
                        .arg(
                            arg!([MONITOR] "Identifier of the monitor")
                                .value_parser(value_parser!(Monitor))
                                .required(true),
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
                                .value_parser(value_parser!(Monitor))
                                .required(true),
                        ),
                    Command::new("next-current")
                        .about("Move focus and the current window to the next workspace on the current monitor"),
                    Command::new("prev")
                        .about("Move focus and the current window to the previous workspace on the specified monitor")
                        .arg_required_else_help(true)
                        .arg(
                            arg!([MONITOR] "Identifier of the monitor")
                                .value_parser(value_parser!(Monitor))
                                .required(true),
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
    Dispatch::call(dispatch)
        .with_context(|| format!("Could not focus window by wm_class: {}", wm_class))?;
    Ok(())
}

/// Get the next/prev workspace's id on the given monitor.
/// * `monitor`: the monitor on which we change the workspace
/// * `direction`: the direction in which we want to go
fn get_relative_workspace_id_on_monitor(
    monitor: &Monitor,
    direction: WorkspaceDirection,
) -> anyhow::Result<WorkspaceId> {
    let active_workspaces = get_active_workspaces()?;

    let active_workspace_id = active_workspaces
        .get(monitor.get_name())
        .ok_or(anyhow::anyhow!(
            "Monitor {:?} is not connected according to hyprland",
            monitor
        ))?;

    let monitors = active_workspaces
        .keys()
        .map(|mon_name| Monitor::try_from(mon_name.as_ref()))
        .collect::<Result<Vec<Monitor>, _>>()
        .context("Not sure how to manage workspaces on an unrecognized monitor")?;

    let is_single = monitors.len() == 1;
    let is_dual = monitors.len() == 2;

    if !is_dual && !is_single {
        anyhow::bail!("Unrecognized monitor configuration: neither single or dual");
    }

    let Some((first_workspace_id, last_workspace_id)) =
        monitor.get_first_and_last_workspace_id(is_single)
    else {
        anyhow::bail!("Unrecognized monitor configuration");
    };

    let delta = match direction {
        WorkspaceDirection::Next => 1,
        WorkspaceDirection::Previous => -1,
    };

    let mut next_workspace_id = active_workspace_id + delta;

    if next_workspace_id > last_workspace_id {
        next_workspace_id = first_workspace_id;
    } else if next_workspace_id < first_workspace_id {
        next_workspace_id = last_workspace_id;
    }

    Ok(next_workspace_id)
}

/// Get the currently focused / active (~ where the cursor is) monitor.
/// Hyprland-rs does not provide the equivalent of `hyprctl activeworkspace`, so we're invoking it
/// by hand.
/// * `sh`: a shell
fn get_active_monitor(sh: &Shell) -> anyhow::Result<Monitor> {
    let err = "Could not find monitor name in `hyprctl activeworkspace output";
    let output = cmd!(sh, "hyprctl activeworkspace").read()?;
    let monitor_name = output
        .lines()
        .next()
        .ok_or(anyhow::anyhow!(err))?
        .split_once("on monitor ")
        .ok_or(anyhow::anyhow!(err))?
        .1;
    Monitor::try_from(monitor_name)
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
            let monitor = next_args
                .get_one::<Monitor>("MONITOR")
                .expect("MONITOR should be a required argument");
            let workspace_id =
                get_relative_workspace_id_on_monitor(monitor, WorkspaceDirection::Next)?;
            let dispatch = get_focus_workspace_dispatcher(workspace_id, move_window);
            Dispatch::call(dispatch)?;
        }
        Some(("next-current", _)) => {
            let monitor = get_active_monitor(sh)?;
            let workspace_id =
                get_relative_workspace_id_on_monitor(&monitor, WorkspaceDirection::Next)?;
            let dispatch = get_focus_workspace_dispatcher(workspace_id, move_window);
            Dispatch::call(dispatch)?;
        }
        Some(("prev", prev_args)) => {
            let monitor = prev_args
                .get_one::<Monitor>("MONITOR")
                .expect("MONITOR should be a required argument");
            let workspace_id =
                get_relative_workspace_id_on_monitor(monitor, WorkspaceDirection::Previous)?;
            let dispatch = get_focus_workspace_dispatcher(workspace_id, move_window);
            Dispatch::call(dispatch)?;
        }
        Some(("prev-current", _)) => {
            let monitor = get_active_monitor(sh)?;
            let workspace_id =
                get_relative_workspace_id_on_monitor(&monitor, WorkspaceDirection::Previous)?;
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
            name: "Firefox",
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

    if let Some(already_running_program) = Clients::get()
        .context("Failed to get hyprland clients")?
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
        appinfo
            .launch(&[], AppLaunchContext::NONE)
            .context("Failed to launch pinned program")?;
        focus_window_by_wm_class(pinned_program.wm_class)?;
    };

    Ok(())
}

/// Get the active workspace ids for the primary and secondary monitor.
/// This assumes that exactly 2 monitors are connected.
pub fn get_active_workspaces() -> anyhow::Result<HashMap<String, WorkspaceId>> {
    Ok(Monitors::get()?
        .into_iter()
        .map(|hypr_mon| (hypr_mon.name, hypr_mon.active_workspace.id))
        .collect::<HashMap<_, _>>())
}

fn initialize() -> anyhow::Result<()> {
    write_workspace_state_to_backing_file()?;
    update_system_bar_layout()?;
    Ok(())
}

pub fn run(sh: &Shell, args: &ArgMatches) -> anyhow::Result<Option<String>> {
    match args.subcommand() {
        Some(("init", _)) => initialize(),
        Some(("focus", focus_args)) => focus_workspace(sh, focus_args, false),
        Some(("move", move_args)) => focus_workspace(sh, move_args, true),
        Some(("open_pinned", open_pinned_args)) => open_pinned(open_pinned_args),
        Some(("run_listener", run_listener_args)) => listener::run(run_listener_args),
        _ => Ok(()),
    }?;

    Ok(None)
}

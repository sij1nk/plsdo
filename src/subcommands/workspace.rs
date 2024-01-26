use std::{collections::BTreeSet, fmt::Display, fs::OpenOptions, io::LineWriter, io::Write};

use clap::{arg, value_parser, ArgMatches, Command};
use hyprland::{
    data::{Clients, Monitors},
    dispatch::{Dispatch, DispatchType},
    shared::{HyprData, WorkspaceId},
};
use xshell::Shell;

use crate::system_atlas::SYSTEM_ATLAS;

pub fn command_extension(cmd: Command) -> Command {
    let inner_subcommands = vec![
        Command::new("focus")
            .about("Move focus to the specified workspace")
            .arg_required_else_help(true)
            .subcommand_required(true)
            .subcommands(
                vec![
                    Command::new("next")
                        .about("Move focus to the next workspace on the monitor")
                        .arg_required_else_help(true)
                        .arg(arg!([MONITOR] "Identifier of the monitor")),
                    Command::new("prev")
                        .about("Move focus to the previous workspace on the monitor")
                        .arg_required_else_help(true)
                        .arg(arg!([MONITOR] "Identifier of the monitor")),
                    Command::new("id")
                        .about("Move focus to the workspace with the given identifier")
                        .arg_required_else_help(true)
                        .arg(arg!([WORKSPACE] "Identifier of the workspace")),
                ]
                .iter(),
            ),
        Command::new("move")
            .about("Move focus and the current window to the specified workspace")
            .arg_required_else_help(true)
            .arg(
                arg!([WORKSPACE] "Identifier of the workspace")
                    .value_parser(value_parser!(WorkspaceId)),
            ),
        Command::new("open_pinned")
            .about("Open and navigate to a pinned window")
            .arg_required_else_help(true)
            .arg(arg!([PROGRAM] "The name of the program whose pinned window to navigate to")),
    ];
    cmd.subcommand_required(true)
        .arg_required_else_help(true)
        .subcommands(inner_subcommands.iter())
}

struct OccupiedWorkspaceIds {
    inner: BTreeSet<WorkspaceId>,
}

impl FromIterator<WorkspaceId> for OccupiedWorkspaceIds {
    fn from_iter<T: IntoIterator<Item = WorkspaceId>>(iter: T) -> Self {
        Self {
            inner: BTreeSet::from_iter(iter),
        }
    }
}

impl Display for OccupiedWorkspaceIds {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let len = self.inner.len();
        let mut iter = self.inner.iter();
        for _ in 0..len - 1 {
            write!(
                f,
                "{},",
                iter.next()
                    .expect("The item should exist, because we're within bounds")
            )?;
        }
        write!(f, "{}", iter.next().expect("The last item should exist"))?;

        Ok(())
    }
}

/// Append the state of the workspaces to a file, from which the eww workspaces widget can read it
/// from. The output should be the following JSON array:
/// `[<active_primary_workspace>,<active_secondary_workspace>,[<occupied_workspace>...]]`
/// Odd numbered workspaces belong to the primary monitor; even numbered workspaces belong to the
/// secondary one.
fn write_workspace_state_to_backing_file() -> anyhow::Result<()> {
    let mut active_workspace_ids = Monitors::get()?
        .map(|mon| mon.active_workspace.id)
        .collect::<Vec<_>>();
    active_workspace_ids.sort_by(|&id1, _| {
        if id1 % 2 == 1 {
            std::cmp::Ordering::Less
        } else {
            std::cmp::Ordering::Greater
        }
    });

    let occupied_workspace_ids = Clients::get()?
        .map(|cl| cl.workspace.id)
        .filter(|&id| id > 0)
        .collect::<OccupiedWorkspaceIds>();

    let file = OpenOptions::new()
        .create(false)
        .append(true)
        .open(SYSTEM_ATLAS.eww_workspaces)?;
    let mut writer = LineWriter::new(&file);
    write!(
        writer,
        "[{},{},[{}]]\n",
        active_workspace_ids[0], active_workspace_ids[1], occupied_workspace_ids
    )?;

    Ok(())
}

fn focus_workspace(args: &ArgMatches) -> anyhow::Result<()> {
    match args.subcommand() {
        Some(("next", _)) => {}
        Some(("prev", _)) => {}
        Some(("id", _)) => {}
        _ => {}
    }
    Ok(())
}

fn move_to_workspace(args: &ArgMatches) -> anyhow::Result<()> {
    let id = args
        .get_one::<WorkspaceId>("WORKSPACE")
        .expect("WORKSPACE should be a required argument");

    let dispatch = DispatchType::MoveFocusedWindowToWorkspace(
        hyprland::dispatch::WorkspaceIdentifier::Id(*id),
    );

    Ok(Dispatch::call(dispatch)?)
}

fn open_pinned(args: &ArgMatches) -> anyhow::Result<()> {
    Ok(())
}

pub fn run(_: &Shell, args: &ArgMatches) -> anyhow::Result<()> {
    match args.subcommand() {
        Some(("focus", focus_args)) => focus_workspace(focus_args),
        Some(("move", move_args)) => move_to_workspace(move_args),
        Some(("open_pinned", open_pinned_args)) => open_pinned(open_pinned_args),
        _ => Ok(()),
    }?;

    Ok(())
}

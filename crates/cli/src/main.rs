use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use commands::{run_move, run_status, run_validate};
use control_tower_application::Direction;
mod commands;
mod deps;
mod guides;
mod web;

#[derive(Debug, Parser)]
#[command(name = "control-tower", about = "Move through local executable stages")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Print the embedded Control Tower agent guidance.
    Guide {
        #[arg(value_enum)]
        action: Option<guides::GuideAction>,
    },
    /// Validate the workspace in the current directory without running roles.
    Validate,
    /// Open the local browser workbench for the current project.
    Ui,
    /// Apply stages through the requested stage number.
    Up {
        #[arg(long)]
        workspace: PathBuf,
        #[arg(long)]
        stage: u32,
    },
    /// Revert stages until the requested completed stage (0 means the baseline).
    Down {
        #[arg(long)]
        workspace: PathBuf,
        #[arg(long)]
        stage: u32,
    },
    /// Show the saved stage position and UUID for a workspace.
    Status {
        #[arg(long)]
        workspace: PathBuf,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match &cli.command {
        Command::Guide { action } => {
            guides::print(*action);
            return ExitCode::SUCCESS;
        }
        Command::Ui => match web::run_from_current_directory() {
            Ok(()) => return ExitCode::SUCCESS,
            Err(message) => {
                eprintln!("error: {message}");
                return ExitCode::FAILURE;
            }
        },
        _ => {}
    }
    let workspace_root = match &cli.command {
        Command::Validate => match std::env::current_dir() {
            Ok(path) => path,
            Err(error) => {
                eprintln!("error: cannot read current directory: {error}");
                return ExitCode::FAILURE;
            }
        },
        Command::Up { workspace, .. }
        | Command::Down { workspace, .. }
        | Command::Status { workspace } => match resolve_workspace(workspace) {
            Ok(path) => path,
            Err(message) => {
                eprintln!("error: {message}");
                return ExitCode::FAILURE;
            }
        },
        Command::Guide { .. } | Command::Ui => unreachable!("handled above"),
    };
    let workbench = match deps::workbench(&workspace_root) {
        Ok(service) => service,
        Err(error) => {
            eprintln!("error: {error}. Run explicit local database setup; see README.md");
            return ExitCode::FAILURE;
        }
    };

    match cli.command {
        Command::Validate => run_validate(&workbench, &workspace_root),
        Command::Up { stage, .. } => run_move(&workbench, &workspace_root, Direction::Up, stage),
        Command::Down { stage, .. } => {
            run_move(&workbench, &workspace_root, Direction::Down, stage)
        }
        Command::Status { .. } => run_status(&workbench, &workspace_root),
        Command::Guide { .. } | Command::Ui => unreachable!("handled above"),
    }
}

fn resolve_workspace(workspace: &Path) -> Result<PathBuf, String> {
    let path = workspace
        .canonicalize()
        .map_err(|error| format!("cannot open workspace {}: {error}", workspace.display()))?;
    if !path.is_dir() {
        return Err(format!("workspace {} is not a directory", path.display()));
    }
    Ok(path)
}

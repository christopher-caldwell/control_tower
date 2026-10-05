use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use commands::{run_move, run_status, run_validate};
use control_tower_application::Direction;
use control_tower_database::operations;
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
    /// Validate the workflow in the current directory without running roles.
    Validate,
    /// Open the local browser workbench for the current workspace.
    Ui,
    /// Apply stages through the requested stage number.
    Up {
        #[arg(long)]
        workflow: PathBuf,
        #[arg(long)]
        stage: u32,
    },
    /// Revert stages until the requested completed stage (0 means the baseline).
    Down {
        #[arg(long)]
        workflow: PathBuf,
        #[arg(long)]
        stage: u32,
    },
    /// Show the saved stage position and UUID for a workflow.
    Status {
        #[arg(long)]
        workflow: PathBuf,
    },
    /// Prepare or inspect a workflow's local database explicitly.
    Db {
        #[command(subcommand)]
        operation: DatabaseOperation,
    },
}

#[derive(Debug, Subcommand)]
enum DatabaseOperation {
    /// Create the local state directory and SQLite file if necessary.
    #[command(name = "bootstrap-local")]
    Bootstrap { path: PathBuf },
    /// Apply the supported versioned schema to an existing SQLite file.
    #[command(name = "migrate-local")]
    Migrate { path: PathBuf },
    /// Verify the supported schema version and migration history read-only.
    #[command(name = "verify-local")]
    Verify { path: PathBuf },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    if let Command::Db { operation } = &cli.command {
        return run_database_operation(operation);
    }
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
    let workflow_root = match &cli.command {
        Command::Validate => match std::env::current_dir() {
            Ok(path) => path,
            Err(error) => {
                eprintln!("error: cannot read current directory: {error}");
                return ExitCode::FAILURE;
            }
        },
        Command::Up { workflow, .. }
        | Command::Down { workflow, .. }
        | Command::Status { workflow } => match resolve_workflow(workflow) {
            Ok(path) => path,
            Err(message) => {
                eprintln!("error: {message}");
                return ExitCode::FAILURE;
            }
        },
        Command::Guide { .. } | Command::Ui | Command::Db { .. } => unreachable!("handled above"),
    };
    let workbench = match deps::workbench(&workflow_root) {
        Ok(service) => service,
        Err(error) => {
            eprintln!("error: {error}. Run explicit local database setup; see README.md");
            return ExitCode::FAILURE;
        }
    };

    match cli.command {
        Command::Validate => run_validate(&workbench, &workflow_root),
        Command::Up { stage, .. } => run_move(&workbench, &workflow_root, Direction::Up, stage),
        Command::Down { stage, .. } => run_move(&workbench, &workflow_root, Direction::Down, stage),
        Command::Status { .. } => run_status(&workbench, &workflow_root),
        Command::Guide { .. } | Command::Ui | Command::Db { .. } => unreachable!("handled above"),
    }
}

fn run_database_operation(operation: &DatabaseOperation) -> ExitCode {
    let path = match operation {
        DatabaseOperation::Bootstrap { path }
        | DatabaseOperation::Migrate { path }
        | DatabaseOperation::Verify { path } => path,
    };
    let workflow = match resolve_workflow(path) {
        Ok(path) => path,
        Err(message) => {
            eprintln!("error: {message}");
            return ExitCode::FAILURE;
        }
    };
    let database = workflow.join(".control_tower/state.sqlite3");
    let result = match operation {
        DatabaseOperation::Bootstrap { .. } => operations::bootstrap(&database),
        DatabaseOperation::Migrate { .. } => operations::migrate(&database),
        DatabaseOperation::Verify { .. } => operations::verify(&database),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("database operation failed: {error}");
            ExitCode::FAILURE
        }
    }
}

fn resolve_workflow(workflow: &Path) -> Result<PathBuf, String> {
    let path = workflow
        .canonicalize()
        .map_err(|error| format!("cannot open workflow {}: {error}", workflow.display()))?;
    if !path.is_dir() {
        return Err(format!("workflow {} is not a directory", path.display()));
    }
    Ok(path)
}

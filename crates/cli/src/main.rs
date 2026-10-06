use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use commands::{run_move, run_status, run_validate};
use control_tower_application::{Direction, WorkbenchStatus};
use control_tower_database::operations;
mod commands;
mod deps;
mod guides;
mod web;
mod workspace;

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
    /// Validate a selected workflow in the current Workspace without running Stage Actions.
    Validate {
        #[arg(long)]
        workflow: PathBuf,
    },
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
    Bootstrap {
        #[arg(long)]
        workflow: PathBuf,
    },
    /// Apply the supported versioned schema to an existing SQLite file.
    #[command(name = "migrate-local")]
    Migrate {
        #[arg(long)]
        workflow: PathBuf,
    },
    /// Verify the supported schema version and migration history read-only.
    #[command(name = "verify-local")]
    Verify {
        #[arg(long)]
        workflow: PathBuf,
    },
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
    let workflow_arg = match &cli.command {
        Command::Validate { workflow }
        | Command::Up { workflow, .. }
        | Command::Down { workflow, .. }
        | Command::Status { workflow } => workflow,
        Command::Guide { .. } | Command::Ui | Command::Db { .. } => unreachable!("handled above"),
    };
    let invocation_root = match std::env::current_dir() {
        Ok(path) => path,
        Err(error) => {
            return report_command_failure(&cli.command, &error.to_string(), None);
        }
    };
    let workspace_result = if matches!(&cli.command, Command::Status { .. }) {
        workspace::load_without_dotenv(&invocation_root)
    } else {
        workspace::load(&invocation_root)
    };
    let workspace = match workspace_result {
        Ok(workspace) => workspace,
        Err(message) => {
            let checkpoint = best_effort_movement_checkpoint(&invocation_root, workflow_arg);
            return report_command_failure(&cli.command, &message, checkpoint.as_ref());
        }
    };
    let workflow_root = match resolve_workflow(&workspace.root, workflow_arg) {
        Ok(path) => path,
        Err(message) => {
            return report_command_failure(&cli.command, &message, None);
        }
    };
    let workbench = match deps::workbench(&workflow_root, workspace.env_overrides) {
        Ok(service) => service,
        Err(error) => {
            let message = format!("{error}. Run explicit local database setup; see README.md");
            return report_command_failure(&cli.command, &message, None);
        }
    };

    match cli.command {
        Command::Validate { .. } => run_validate(&workbench, &workflow_root),
        Command::Up { stage, .. } => run_move(&workbench, &workflow_root, Direction::Up, stage),
        Command::Down { stage, .. } => run_move(&workbench, &workflow_root, Direction::Down, stage),
        Command::Status { .. } => run_status(&workbench, &workflow_root),
        Command::Guide { .. } | Command::Ui | Command::Db { .. } => unreachable!("handled above"),
    }
}

fn best_effort_movement_checkpoint(root: &Path, workflow_arg: &Path) -> Option<WorkbenchStatus> {
    let workspace = workspace::load_without_dotenv(root).ok()?;
    let workflow = resolve_workflow(&workspace.root, workflow_arg).ok()?;
    let workbench = deps::workbench(&workflow, workspace.env_overrides).ok()?;
    workbench.status(&workflow).ok()
}

fn report_command_failure(
    command: &Command,
    message: &str,
    checkpoint: Option<&WorkbenchStatus>,
) -> ExitCode {
    match command {
        Command::Up { stage, .. } => {
            commands::report_movement_preflight_failure(Direction::Up, *stage, message, checkpoint)
        }
        Command::Down { stage, .. } => commands::report_movement_preflight_failure(
            Direction::Down,
            *stage,
            message,
            checkpoint,
        ),
        _ => {
            eprintln!("error: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run_database_operation(operation: &DatabaseOperation) -> ExitCode {
    let path = match operation {
        DatabaseOperation::Bootstrap { workflow }
        | DatabaseOperation::Migrate { workflow }
        | DatabaseOperation::Verify { workflow } => workflow,
    };
    let workspace = match std::env::current_dir()
        .map_err(|e| e.to_string())
        .and_then(|path| workspace::load_without_dotenv(&path))
    {
        Ok(workspace) => workspace,
        Err(message) => {
            eprintln!("error: {message}");
            return ExitCode::FAILURE;
        }
    };
    let workflow = match resolve_workflow(&workspace.root, path) {
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
        Ok(()) => {
            let operation = match operation {
                DatabaseOperation::Bootstrap { .. } => "bootstrapped",
                DatabaseOperation::Migrate { .. } => "migration succeeded",
                DatabaseOperation::Verify { .. } => "verified",
            };
            println!("Local database {operation}: {}", database.display());
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("database operation failed: {error}");
            ExitCode::FAILURE
        }
    }
}

fn resolve_workflow(workspace: &Path, workflow: &Path) -> Result<PathBuf, String> {
    let selected = if workflow.is_absolute() {
        workflow.to_path_buf()
    } else {
        workspace.join(workflow)
    };
    let path = selected
        .canonicalize()
        .map_err(|error| format!("cannot open workflow {}: {error}", workflow.display()))?;
    if !path.is_dir() {
        return Err(format!("workflow {} is not a directory", path.display()));
    }
    Ok(path)
}

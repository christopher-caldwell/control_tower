use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use control_tower_adapters::{
    FilesystemStageDiscovery, SqliteWorkbenchStateStore, SystemExecutableRunner,
};
use control_tower_application::{
    Direction, ExecutionEvent, MoveStatus, Workbench, WorkbenchError, WorkbenchState,
};

#[derive(Debug, Parser)]
#[command(name = "control-tower", about = "Move through local executable stages")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
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
    let workspace = match &cli.command {
        Command::Up { workspace, .. }
        | Command::Down { workspace, .. }
        | Command::Status { workspace } => workspace,
    };
    let workspace_root = match resolve_workspace(workspace) {
        Ok(path) => path,
        Err(message) => {
            eprintln!("error: {message}");
            return ExitCode::FAILURE;
        }
    };
    let state_store = match SqliteWorkbenchStateStore::open(&workspace_root) {
        Ok(store) => store,
        Err(message) => {
            eprintln!("error: {message}");
            return ExitCode::FAILURE;
        }
    };
    let workbench = Workbench::new(
        Box::new(FilesystemStageDiscovery),
        Box::new(state_store),
        Box::new(SystemExecutableRunner),
    );

    match cli.command {
        Command::Up { stage, .. } => run_move(&workbench, &workspace_root, Direction::Up, stage),
        Command::Down { stage, .. } => {
            run_move(&workbench, &workspace_root, Direction::Down, stage)
        }
        Command::Status { .. } => run_status(&workbench, &workspace_root),
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

fn run_move(
    workbench: &Workbench,
    workspace: &Path,
    direction: Direction,
    target: u32,
) -> ExitCode {
    let outcome = match workbench.move_to(workspace, direction, target) {
        Ok(outcome) => outcome,
        Err(error) => {
            eprintln!("error: {}", format_workbench_error(error));
            return ExitCode::FAILURE;
        }
    };
    for execution in &outcome.executions {
        render_execution(execution);
    }
    match outcome.status {
        MoveStatus::Complete(state) => {
            println!("{direction} complete. {}", position_message(&state));
            ExitCode::SUCCESS
        }
        MoveStatus::Stopped { state, failure } => {
            eprintln!("error: {failure}. {}", position_message(&state));
            ExitCode::FAILURE
        }
    }
}

fn run_status(workbench: &Workbench, workspace: &Path) -> ExitCode {
    let status = match workbench.status(workspace) {
        Ok(status) => status,
        Err(error) => {
            eprintln!("error: {}", format_workbench_error(error));
            return ExitCode::FAILURE;
        }
    };
    if status.state.completed_stage_count == 0 {
        println!("Completed stage: baseline (0)");
    } else {
        let stage = &status.stages[status.state.completed_stage_count - 1];
        println!("Completed stage: {} ({})", stage.number, stage.name);
    }
    match &status.state.uuid {
        Some(uuid) => println!("UUID: {uuid}"),
        None => println!("UUID: not created"),
    }
    if let Some(pending) = status.state.pending {
        let stage = &status.stages[pending.stage_index];
        println!(
            "Pending verification: {} {} ({})",
            pending.direction, stage.number, stage.name
        );
    }
    println!("Discovered stages: {}", status.stages.len());
    ExitCode::SUCCESS
}

fn render_execution(execution: &ExecutionEvent) {
    match &execution.result {
        Ok(output) => {
            let stdout = io::stdout();
            let _ = stdout.lock().write_all(&output.stdout);
            let stderr = io::stderr();
            let _ = stderr.lock().write_all(&output.stderr);
            if output.success {
                println!(
                    "[stage {} {}] succeeded",
                    execution.stage_number, execution.role
                );
            } else {
                eprintln!(
                    "[stage {} {}] exited with status {}",
                    execution.stage_number,
                    execution.role,
                    output
                        .exit_code
                        .map_or_else(|| "signal".to_owned(), |code| code.to_string())
                );
            }
        }
        Err(message) => eprintln!(
            "[stage {} {}] could not start: {message}",
            execution.stage_number, execution.role
        ),
    }
}

fn position_message(state: &WorkbenchState) -> String {
    if let Some(pending) = state.pending {
        format!(
            "{} stage(s) complete; stage index {} has pending {} verification",
            state.completed_stage_count,
            pending.stage_index + 1,
            pending.direction
        )
    } else {
        format!("{} stage(s) complete", state.completed_stage_count)
    }
}

fn format_workbench_error(error: WorkbenchError) -> String {
    error.to_string()
}

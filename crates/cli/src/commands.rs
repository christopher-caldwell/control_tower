use control_tower_application::{
    Direction, ExecutionEvent, MoveStatus, MoveToInput, Workbench, WorkbenchState,
};
use std::io::{self, Write};
use std::path::Path;
use std::process::ExitCode;

pub(super) fn run_move(
    workbench: &Workbench,
    workspace: &Path,
    direction: Direction,
    target: u32,
) -> ExitCode {
    let outcome = match workbench.move_to(MoveToInput {
        workspace_root: workspace,
        direction,
        target_stage: target,
    }) {
        Ok(outcome) => outcome,
        Err(error) => {
            eprintln!("error: {}", error);
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

pub(super) fn run_status(workbench: &Workbench, workspace: &Path) -> ExitCode {
    let status = match workbench.status(workspace) {
        Ok(status) => status,
        Err(error) => {
            eprintln!("error: {}", error);
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

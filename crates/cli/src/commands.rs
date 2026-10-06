use control_tower_application::{
    Direction, ExecutionEvent, ExecutionProgress, MoveStatus, MoveToInput, MovementChoice, Stage,
    TransitionFailure, Workbench, WorkbenchState, WorkbenchStatus,
};
use std::io::{self, Write};
use std::path::Path;
use std::process::ExitCode;

pub(super) fn run_move(
    workbench: &Workbench,
    workflow: &Path,
    direction: Direction,
    target: u32,
) -> ExitCode {
    println!("Workflow: {}", workflow.display());
    println!("Requested: {direction} to stage {target}");
    let outcome = match workbench.move_to_observed(
        MoveToInput {
            workflow_root: workflow,
            direction,
            target_stage: target,
            expected_checkpoint: None,
        },
        &mut |progress| match progress {
            ExecutionProgress::Starting { stage, role } => {
                println!(
                    "[stage {} {} ({})] starting",
                    stage.number, role, stage.name
                );
                let _ = io::stdout().flush();
            }
            ExecutionProgress::Finished { stage, execution } => render_execution(stage, execution),
        },
    ) {
        Ok(outcome) => outcome,
        Err(error) => {
            let checkpoint = workbench.status(workflow).ok();
            return report_movement_preflight_failure(
                direction,
                target,
                &error.to_string(),
                checkpoint.as_ref(),
            );
        }
    };
    let choices = outcome.verification_choices();
    let exit_code = match &outcome.status {
        MoveStatus::Complete(state) => {
            println!("{direction} complete.");
            if outcome.executions.is_empty() {
                println!("No roles ran.");
            }
            render_position(state, &outcome.stages);
            ExitCode::SUCCESS
        }
        MoveStatus::Stopped { state, failure } => {
            eprintln!("error: {failure}");
            if let TransitionFailure::StateCouldNotBeSaved { attempted, .. } = failure {
                println!("Last confirmed checkpoint:");
                render_position(state, &outcome.stages);
                println!("Unconfirmed checkpoint update:");
                render_position(attempted, &outcome.stages);
            } else {
                render_position(state, &outcome.stages);
            }
            if let Some(choices) = choices {
                let executable = std::env::current_exe().unwrap_or_else(|_| {
                    std::env::args_os()
                        .next()
                        .expect("invoked executable")
                        .into()
                });
                println!("Retry this check only:");
                render_choice(&executable, workflow, choices.retry);
                if let Some(reverse) = choices.reverse {
                    println!("Reverse the active stage:");
                    render_choice(&executable, workflow, reverse);
                }
            } else if matches!(
                failure,
                TransitionFailure::ExecutableFailed { .. }
                    | TransitionFailure::ExecutableCouldNotStart { .. }
                    | TransitionFailure::StateCouldNotBeSaved { .. }
            ) {
                println!("Inspect author-owned effects before choosing further movement.");
            }
            if matches!(
                failure,
                TransitionFailure::ExecutableFailed {
                    role: control_tower_application::ExecutableRole::VerifyUp
                        | control_tower_application::ExecutableRole::VerifyDown,
                    exit_code: Some(code),
                    ..
                }
                if *code != 0
            ) {
                ExitCode::from(3)
            } else {
                ExitCode::FAILURE
            }
        }
    };
    render_movement_summary(direction, target, &outcome.status, &outcome.stages);
    exit_code
}

pub(super) fn report_movement_preflight_failure(
    direction: Direction,
    target: u32,
    error: &str,
    checkpoint: Option<&WorkbenchStatus>,
) -> ExitCode {
    eprintln!("error: {error}");
    match checkpoint {
        Some(checkpoint) => {
            render_movement_position(
                "stopped before movement",
                direction,
                target,
                &checkpoint.state,
                &checkpoint.stages,
            );
            if let Some(pending) = checkpoint.state.pending {
                let stage = &checkpoint.stages[pending.stage_index];
                println!(
                    "  Pending verification: {} {} ({})",
                    pending.direction, stage.number, stage.name
                );
            }
            println!("  No roles ran.");
            println!("  Failure: {error}");
        }
        None => {
            println!(
                "Result: stopped before movement; requested {direction} to {target}; position unavailable"
            );
            println!("  No roles ran.");
            println!("  Failure: {error}");
        }
    }
    ExitCode::FAILURE
}

fn render_movement_summary(
    direction: Direction,
    target: u32,
    status: &MoveStatus,
    stages: &[Stage],
) {
    let (label, state, failure) = match status {
        MoveStatus::Complete(state) => ("complete", state, None),
        MoveStatus::Stopped { state, failure } => ("stopped", state, Some(failure)),
    };
    render_movement_position(label, direction, target, state, stages);
    if let Some(pending) = state.pending {
        let stage = &stages[pending.stage_index];
        println!(
            "  Pending verification: {} {} ({})",
            pending.direction, stage.number, stage.name
        );
    }
    if let Some(failure) = failure {
        match failure {
            TransitionFailure::ExecutableFailed {
                stage_number,
                role,
                exit_code,
            } => {
                let child_status =
                    exit_code.map_or_else(|| "signal/unknown".to_owned(), |code| code.to_string());
                println!(
                    "  Failed role: stage {stage_number} {role}; child exit status {child_status}"
                );
            }
            TransitionFailure::ExecutableCouldNotStart {
                stage_number, role, ..
            } => {
                println!("  Failed role: stage {stage_number} {role}; executable did not start");
            }
            _ => println!("  Failure: {failure}"),
        }
    }
}

fn render_movement_position(
    result: &str,
    direction: Direction,
    target: u32,
    state: &WorkbenchState,
    stages: &[Stage],
) {
    let position = match state.completed_stage_count {
        0 => "baseline (0)".to_owned(),
        count => {
            let stage = &stages[count - 1];
            format!("{} ({})", stage.number, stage.name)
        }
    };
    println!("Result: {result}; requested {direction} to {target}; position {position}");
}

pub(super) fn run_status(workbench: &Workbench, workflow: &Path) -> ExitCode {
    let status = match workbench.status(workflow) {
        Ok(status) => status,
        Err(error) => {
            eprintln!("error: {error}");
            return ExitCode::FAILURE;
        }
    };
    render_position(&status.state, &status.stages);
    ExitCode::SUCCESS
}

pub(super) fn run_validate(workbench: &Workbench, workflow: &Path) -> ExitCode {
    let status = match workbench.status(workflow) {
        Ok(status) => status,
        Err(error) => {
            eprintln!("error: {error}");
            return ExitCode::FAILURE;
        }
    };
    println!("Workflow loaded successfully: {}", workflow.display());
    println!("Discovered stages: {}", status.stages.len());
    ExitCode::SUCCESS
}

fn render_position(state: &WorkbenchState, stages: &[Stage]) {
    if state.completed_stage_count == 0 {
        println!("Completed stage: baseline (0)");
    } else {
        let stage = &stages[state.completed_stage_count - 1];
        println!("Completed stage: {} ({})", stage.number, stage.name);
    }
    match &state.uuid {
        Some(uuid) => println!("UUID: {uuid}"),
        None => println!("UUID: not created"),
    }
    if let Some(pending) = state.pending {
        let stage = &stages[pending.stage_index];
        println!(
            "Pending verification: {} {} ({})",
            pending.direction, stage.number, stage.name
        );
    }
    println!("Discovered stages: {}", stages.len());
}

fn render_choice(executable: &Path, workflow: &Path, choice: MovementChoice) {
    println!(
        "  {} {} --workflow {} --stage {}",
        shell_quote(&executable.to_string_lossy()),
        choice.direction,
        shell_quote(&workflow.to_string_lossy()),
        choice.target_stage
    );
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

// Separators belong to presentation; ProcessOutput keeps the exact raw bytes.
fn render_stream(writer: &mut impl Write, identity: &str, stream: &str, bytes: &[u8]) {
    if !bytes.is_empty() {
        let _ = writeln!(writer, "{identity} {stream}:");
        let _ = writer.write_all(bytes);
        if !bytes.ends_with(b"\n") {
            let _ = writeln!(writer);
        }
    }
}

fn render_execution(stage: &Stage, execution: &ExecutionEvent) {
    let identity = format!(
        "[stage {} {} ({})]",
        stage.number, execution.role, stage.name
    );
    let mut stdout = io::stdout().lock();
    let mut stderr = io::stderr().lock();
    match &execution.result {
        Ok(output) => {
            render_stream(&mut stdout, &identity, "stdout", &output.stdout);
            render_stream(&mut stderr, &identity, "stderr", &output.stderr);
            if output.success {
                let _ = writeln!(stdout, "{identity} succeeded");
            } else {
                let code = output
                    .exit_code
                    .map_or_else(|| "signal".to_owned(), |code| code.to_string());
                let _ = writeln!(stderr, "{identity} exited with status {code}");
            }
        }
        Err(error) => {
            let _ = writeln!(stderr, "{identity} could not start: {error}");
        }
    }
    let _ = stdout.flush();
    let _ = stderr.flush();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_paths_round_trip_and_stream_framing_preserves_bytes() {
        let quoted = shell_quote("/tmp/fixture with 'apostrophes'");
        let output = std::process::Command::new("sh")
            .args(["-c", &format!("printf '%s' {quoted}")])
            .output()
            .unwrap();
        assert!(output.status.success());
        assert_eq!(output.stdout, b"/tmp/fixture with 'apostrophes'");
        let mut framed = vec![];
        render_stream(&mut framed, "[stage 200 up]", "stdout", b"raw\xff\0");
        assert_eq!(framed, b"[stage 200 up] stdout:\nraw\xff\0\n");
    }
}

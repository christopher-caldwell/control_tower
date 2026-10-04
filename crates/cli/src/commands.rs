use control_tower_application::{
    Direction, ExecutionEvent, ExecutionProgress, MoveStatus, MoveToInput, MovementChoice, Stage,
    TransitionFailure, Workbench, WorkbenchState,
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
    println!("Workspace: {}", workspace.display());
    println!("Requested: {direction} to stage {target}");
    let outcome = match workbench.move_to_observed(
        MoveToInput {
            workspace_root: workspace,
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
            eprintln!("error: {error}");
            return ExitCode::FAILURE;
        }
    };
    let choices = outcome.verification_choices();
    match &outcome.status {
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
                render_choice(&executable, workspace, choices.retry);
                if let Some(reverse) = choices.reverse {
                    println!("Reverse the active stage:");
                    render_choice(&executable, workspace, reverse);
                }
            } else if matches!(
                failure,
                TransitionFailure::ExecutableFailed { .. }
                    | TransitionFailure::ExecutableCouldNotStart { .. }
                    | TransitionFailure::StateCouldNotBeSaved { .. }
            ) {
                println!("Inspect author-owned effects before choosing further movement.");
            }
            ExitCode::FAILURE
        }
    }
}

pub(super) fn run_status(workbench: &Workbench, workspace: &Path) -> ExitCode {
    let status = match workbench.status(workspace) {
        Ok(status) => status,
        Err(error) => {
            eprintln!("error: {error}");
            return ExitCode::FAILURE;
        }
    };
    render_position(&status.state, &status.stages);
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

fn render_choice(executable: &Path, workspace: &Path, choice: MovementChoice) {
    println!(
        "  {} {} --workspace {} --stage {}",
        shell_quote(&executable.to_string_lossy()),
        choice.direction,
        shell_quote(&workspace.to_string_lossy()),
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

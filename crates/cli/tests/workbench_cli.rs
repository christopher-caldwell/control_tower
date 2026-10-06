#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static NEXT_WORKFLOW: AtomicU64 = AtomicU64::new(0);

struct Workflow(PathBuf);

impl Workflow {
    fn from_fixture() -> Self {
        let fixture = Self::from_fixture_unprepared();
        for operation in ["bootstrap-local", "migrate-local"] {
            let output = database_operation(&fixture, operation);
            assert!(output.status.success(), "{}", output_text(&output));
        }
        fixture
    }

    fn from_fixture_unprepared() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after the Unix epoch")
            .as_nanos();
        let workspace = std::env::temp_dir().join(format!(
            "control-tower-{}-{nonce}-{}",
            std::process::id(),
            NEXT_WORKFLOW.fetch_add(1, Ordering::Relaxed)
        ));
        Self::from_fixture_at(workspace.join("workflows/uuid-file"))
    }

    fn from_fixture_at(workflow: PathBuf) -> Self {
        let workspace = workflow.parent().unwrap().parent().unwrap();
        fs::create_dir_all(workspace.join("workflows")).expect("create Workspace inventory");
        fs::write(
            workspace.join("control-tower.toml"),
            "[workspace]\nlabel = \"Fixture\"\n",
        )
        .unwrap();
        copy_directory(
            &Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../examples/simple/workflows/uuid-file"),
            &workflow,
        );
        Self(workflow)
    }

    fn path(&self) -> &Path {
        &self.0
    }

    fn workspace(&self) -> &Path {
        self.0.parent().unwrap().parent().unwrap()
    }
}

impl Drop for Workflow {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn copy_directory(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).expect("create fixture directory");
    for entry in fs::read_dir(source).expect("read fixture directory") {
        let entry = entry.expect("read fixture entry");
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        if source_path.is_dir() {
            copy_directory(&source_path, &destination_path);
        } else {
            fs::copy(&source_path, &destination_path).expect("copy fixture file");
            fs::set_permissions(&destination_path, fs::Permissions::from_mode(0o755))
                .expect("make fixture file executable");
        }
    }
}

fn move_to(workflow: &Workflow, direction: &str, target: u32) -> Output {
    Command::new(env!("CARGO_BIN_EXE_control-tower"))
        .arg(direction)
        .arg("--workflow")
        .arg(workflow.path())
        .arg("--stage")
        .arg(target.to_string())
        .current_dir(workflow.workspace())
        .output()
        .expect("run Control Tower CLI")
}

fn database_operation(workflow: &Workflow, operation: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_control-tower"))
        .arg("db")
        .arg(operation)
        .arg("--workflow")
        .arg(workflow.path())
        .current_dir(workflow.workspace())
        .output()
        .expect("run Control Tower database operation")
}

fn status(workflow: &Workflow) -> Output {
    Command::new(env!("CARGO_BIN_EXE_control-tower"))
        .arg("status")
        .arg("--workflow")
        .arg(workflow.path())
        .current_dir(workflow.workspace())
        .output()
        .expect("run Control Tower status")
}

fn output_text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn uuid_file(workflow: &Workflow) -> Option<PathBuf> {
    let data = workflow.path().join("data");
    let mut entries = match fs::read_dir(data) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return None,
        Err(error) => panic!("read UUID data directory: {error}"),
    };
    let file = entries.next()?.expect("read UUID file entry").path();
    assert!(
        entries.next().is_none(),
        "fixture should contain one UUID file"
    );
    Some(file)
}

fn write_executable(path: &Path, contents: &str) {
    fs::write(path, contents).expect("write fixture executable");
    fs::set_permissions(path, fs::Permissions::from_mode(0o755))
        .expect("make fixture executable runnable");
}

#[test]
fn walks_fixture_forward_and_backward_across_cli_processes() {
    let workflow = Workflow::from_fixture();
    observe_roles(&workflow);

    let first_up = move_to(&workflow, "up", 1);
    assert!(first_up.status.success(), "{}", output_text(&first_up));
    assert!(
        output_text(&first_up)
            .contains("Result: complete; requested up to 1; position 1 (create-file)")
    );
    let first_uuid_file = uuid_file(&workflow).expect("stage 1 creates the UUID file");
    assert_eq!(fs::read(&first_uuid_file).unwrap(), b"");
    let uuid = first_uuid_file
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string();

    let no_op = move_to(&workflow, "up", 1);
    assert!(no_op.status.success(), "{}", output_text(&no_op));
    assert!(
        output_text(&no_op)
            .contains("Result: complete; requested up to 1; position 1 (create-file)")
    );
    assert!(output_text(&no_op).contains("No roles ran."));

    let up_to_three = move_to(&workflow, "up", 3);
    assert!(
        up_to_three.status.success(),
        "{}",
        output_text(&up_to_three)
    );
    let third_uuid_file = uuid_file(&workflow).expect("UUID file remains present");
    assert_eq!(third_uuid_file.file_name().unwrap().to_string_lossy(), uuid);
    assert_eq!(fs::read(&third_uuid_file).unwrap(), b"hello to you");

    let down_to_two = move_to(&workflow, "down", 2);
    assert!(
        down_to_two.status.success(),
        "{}",
        output_text(&down_to_two)
    );
    assert_eq!(fs::read(&third_uuid_file).unwrap(), b"hello");

    let down_to_one = move_to(&workflow, "down", 1);
    assert!(
        down_to_one.status.success(),
        "{}",
        output_text(&down_to_one)
    );
    assert_eq!(fs::read(&third_uuid_file).unwrap(), b"");

    let down_to_zero = move_to(&workflow, "down", 0);
    assert!(
        down_to_zero.status.success(),
        "{}",
        output_text(&down_to_zero)
    );
    assert!(uuid_file(&workflow).is_none());
    let final_status = status(&workflow);
    assert!(
        final_status.status.success(),
        "{}",
        output_text(&final_status)
    );
    assert!(String::from_utf8_lossy(&final_status.stdout).contains("UUID: not created"));
    assert_eq!(
        checkpoint(&workflow),
        control_tower_application::WorkbenchState::default()
    );
    assert_calls_and_uuid(
        &workflow,
        &[
            "1 up",
            "1 verify-up",
            "2 up",
            "2 verify-up",
            "3 up",
            "3 verify-up",
            "3 down",
            "3 verify-down",
            "2 down",
            "2 verify-down",
            "1 down",
            "1 verify-down",
        ],
        &uuid,
    );
}

#[test]
fn retries_failed_up_verification_without_replaying_mutation() {
    let workflow = Workflow::from_fixture();
    let stage_two = workflow.path().join("stages/002-write-hello");
    write_executable(
        &stage_two.join("up"),
        "#!/bin/sh\nset -eu\nprintf 'up\\n' >> \"$CONTROL_TOWER_WORKFLOW/stage-2-up.log\"\nprintf 'hello' > \"$CONTROL_TOWER_WORKFLOW/data/$CONTROL_TOWER_UUID\"\n",
    );
    write_executable(
        &stage_two.join("verify-up"),
        "#!/bin/sh\nset -eu\nprintf 'verify\\n' >> \"$CONTROL_TOWER_WORKFLOW/stage-2-verify-up.log\"\nmarker=\"$CONTROL_TOWER_WORKFLOW/verify-up-attempted\"\nif [ ! -e \"$marker\" ]; then touch \"$marker\"; exit 23; fi\ntest \"$(cat \"$CONTROL_TOWER_WORKFLOW/data/$CONTROL_TOWER_UUID\")\" = hello\n",
    );

    let first = move_to(&workflow, "up", 2);
    assert!(!first.status.success(), "verification should fail once");
    assert!(output_text(&first).contains("verify-up exited with status 23"));
    let pending_status = status(&workflow);
    assert!(String::from_utf8_lossy(&pending_status.stdout).contains("Pending verification: up 2"));
    assert_eq!(
        fs::read_to_string(workflow.path().join("stage-2-up.log")).unwrap(),
        "up\n"
    );

    let retry = move_to(&workflow, "up", 2);
    assert!(retry.status.success(), "{}", output_text(&retry));
    assert!(String::from_utf8_lossy(&retry.stdout).contains("stage 2 verify-up"));
    assert!(!String::from_utf8_lossy(&retry.stdout).contains("stage 2 up"));
    assert_eq!(
        fs::read_to_string(workflow.path().join("stage-2-up.log")).unwrap(),
        "up\n"
    );
    assert_eq!(
        fs::read_to_string(workflow.path().join("stage-2-verify-up.log")).unwrap(),
        "verify\nverify\n"
    );
}

#[test]
fn retries_failed_down_verification_without_replaying_mutation() {
    let workflow = Workflow::from_fixture();
    let stage_three = workflow.path().join("stages/003-add-to-you");
    write_executable(
        &stage_three.join("down"),
        "#!/bin/sh\nset -eu\nprintf 'down\\n' >> \"$CONTROL_TOWER_WORKFLOW/stage-3-down.log\"\ntest \"$(cat \"$CONTROL_TOWER_WORKFLOW/data/$CONTROL_TOWER_UUID\")\" = 'hello to you'\nprintf hello > \"$CONTROL_TOWER_WORKFLOW/data/$CONTROL_TOWER_UUID\"\n",
    );
    write_executable(
        &stage_three.join("verify-down"),
        "#!/bin/sh\nset -eu\nprintf 'verify\\n' >> \"$CONTROL_TOWER_WORKFLOW/stage-3-verify-down.log\"\nmarker=\"$CONTROL_TOWER_WORKFLOW/verify-down-attempted\"\nif [ ! -e \"$marker\" ]; then touch \"$marker\"; exit 24; fi\ntest \"$(cat \"$CONTROL_TOWER_WORKFLOW/data/$CONTROL_TOWER_UUID\")\" = hello\n",
    );

    let up = move_to(&workflow, "up", 3);
    assert!(up.status.success(), "{}", output_text(&up));
    let first_down = move_to(&workflow, "down", 2);
    assert!(
        !first_down.status.success(),
        "verification should fail once"
    );
    assert!(output_text(&first_down).contains("verify-down exited with status 24"));
    let pending_status = status(&workflow);
    assert!(
        String::from_utf8_lossy(&pending_status.stdout).contains("Pending verification: down 3")
    );
    assert_eq!(
        fs::read_to_string(workflow.path().join("stage-3-down.log")).unwrap(),
        "down\n"
    );

    let retry = move_to(&workflow, "down", 2);
    assert!(retry.status.success(), "{}", output_text(&retry));
    assert!(String::from_utf8_lossy(&retry.stdout).contains("stage 3 verify-down"));
    assert!(!String::from_utf8_lossy(&retry.stdout).contains("stage 3 down"));
    assert_eq!(
        fs::read_to_string(workflow.path().join("stage-3-down.log")).unwrap(),
        "down\n"
    );
    assert_eq!(
        fs::read_to_string(workflow.path().join("stage-3-verify-down.log")).unwrap(),
        "verify\nverify\n"
    );
    assert_eq!(fs::read(uuid_file(&workflow).unwrap()).unwrap(), b"hello");
}

#[test]
fn omitted_directional_verifiers_do_not_block_movement() {
    let workflow = Workflow::from_fixture();
    let stage_two = workflow.path().join("stages/002-write-hello");
    fs::remove_file(stage_two.join("verify-up")).unwrap();
    fs::remove_file(stage_two.join("verify-down")).unwrap();

    let up = move_to(&workflow, "up", 2);
    assert!(up.status.success(), "{}", output_text(&up));
    assert_eq!(fs::read(uuid_file(&workflow).unwrap()).unwrap(), b"hello");

    let down = move_to(&workflow, "down", 1);
    assert!(down.status.success(), "{}", output_text(&down));
    assert_eq!(fs::read(uuid_file(&workflow).unwrap()).unwrap(), b"");
}

#[test]
fn failed_mutation_stops_before_its_verifier_and_later_stages() {
    let workflow = Workflow::from_fixture();
    let stage_two = workflow.path().join("stages/002-write-hello");
    let stage_three = workflow.path().join("stages/003-add-to-you");
    write_executable(
        &stage_two.join("up"),
        "#!/bin/sh\nprintf 'mutation\\n' >> \"$CONTROL_TOWER_WORKFLOW/mutation.log\"\nexit 17\n",
    );
    write_executable(
        &stage_two.join("verify-up"),
        "#!/bin/sh\nprintf 'verify\\n' >> \"$CONTROL_TOWER_WORKFLOW/verifier.log\"\n",
    );
    write_executable(
        &stage_three.join("up"),
        "#!/bin/sh\nprintf 'stage 3\\n' >> \"$CONTROL_TOWER_WORKFLOW/later-stage.log\"\n",
    );

    let failed_move = move_to(&workflow, "up", 3);
    assert!(!failed_move.status.success(), "mutation should fail");
    assert!(output_text(&failed_move).contains("stage 2 up exited with status 17"));
    assert_eq!(
        fs::read_to_string(workflow.path().join("mutation.log")).unwrap(),
        "mutation\n"
    );
    assert!(!workflow.path().join("verifier.log").exists());
    assert!(!workflow.path().join("later-stage.log").exists());

    let final_status = status(&workflow);
    let status_text = String::from_utf8_lossy(&final_status.stdout);
    assert!(status_text.contains("Completed stage: 1"));
    assert!(!status_text.contains("Pending verification:"));
}

#[test]
fn ordinary_cli_does_not_bootstrap_or_migrate() {
    let workflow = Workflow::from_fixture();
    fs::remove_dir_all(workflow.path().join(".control_tower")).unwrap();
    let output = status(&workflow);
    assert!(!output.status.success());
    assert!(output_text(&output).contains("explicit local database setup"));
    assert!(!workflow.path().join(".control_tower").exists());
    let down = move_to(&workflow, "down", 0);
    assert!(!down.status.success());
    assert!(!workflow.path().join(".control_tower").exists());
    let validate = Command::new(env!("CARGO_BIN_EXE_control-tower"))
        .arg("validate")
        .arg("--workflow")
        .arg(workflow.path())
        .current_dir(workflow.workspace())
        .output()
        .expect("run Control Tower validation");
    assert!(!validate.status.success());
    assert!(!workflow.path().join(".control_tower").exists());
    let bootstrap = database_operation(&workflow, "bootstrap-local");
    assert!(bootstrap.status.success(), "{}", output_text(&bootstrap));
    assert!(output_text(&bootstrap).contains("Local database bootstrapped:"));
    let database = workflow.path().join(".control_tower/state.sqlite3");
    let output = move_to(&workflow, "up", 1);
    assert!(!output.status.success());
    assert!(output_text(&output).contains("schema is not initialized"));
    assert!(uuid_file(&workflow).is_none());
    let verify = database_operation(&workflow, "verify-local");
    assert!(!verify.status.success());
    assert!(output_text(&verify).contains("schema is not initialized"));
    assert!(database.exists());
}

#[test]
fn explicit_database_operations_are_available_through_the_cli() {
    let workflow = Workflow::from_fixture_unprepared();
    let database = workflow.path().join(".control_tower/state.sqlite3");

    let migrate_before_bootstrap = database_operation(&workflow, "migrate-local");
    assert!(!migrate_before_bootstrap.status.success());
    assert!(!database.exists());
    let verify_before_bootstrap = database_operation(&workflow, "verify-local");
    assert!(!verify_before_bootstrap.status.success());
    assert!(!database.exists());

    let bootstrap = database_operation(&workflow, "bootstrap-local");
    assert!(bootstrap.status.success(), "{}", output_text(&bootstrap));
    assert!(database.is_file());
    let table_count: i64 = rusqlite::Connection::open(&database)
        .unwrap()
        .query_row(
            "SELECT count(*) FROM sqlite_schema WHERE type = 'table'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(table_count, 0, "bootstrap must not create schema tables");

    let migrate = database_operation(&workflow, "migrate-local");
    assert!(migrate.status.success(), "{}", output_text(&migrate));
    assert!(output_text(&migrate).contains("Local database migration succeeded:"));
    let verify = database_operation(&workflow, "verify-local");
    assert!(verify.status.success(), "{}", output_text(&verify));
    assert!(output_text(&verify).contains("Local database verified:"));
    let rerun_migrate = database_operation(&workflow, "migrate-local");
    assert!(
        rerun_migrate.status.success(),
        "{}",
        output_text(&rerun_migrate)
    );
    let rerun_text = output_text(&rerun_migrate);
    assert!(rerun_text.contains("Local database migration succeeded:"));
    assert!(!rerun_text.contains("migrations applied"));
    let rerun_verify = database_operation(&workflow, "verify-local");
    assert!(
        rerun_verify.status.success(),
        "{}",
        output_text(&rerun_verify)
    );
    assert!(output_text(&rerun_verify).contains("Local database verified:"));
}

#[test]
fn malformed_workspace_dotenv_does_not_block_status_or_database_operations() {
    let workflow = Workflow::from_fixture();
    fs::write(workflow.workspace().join(".env"), "INVALID LINE\n").unwrap();

    let status_output = status(&workflow);
    assert!(
        status_output.status.success(),
        "{}",
        output_text(&status_output)
    );
    assert!(
        String::from_utf8_lossy(&status_output.stdout).contains("Completed stage: baseline (0)")
    );

    for operation in ["bootstrap-local", "migrate-local", "verify-local"] {
        let output = database_operation(&workflow, operation);
        assert!(
            output.status.success(),
            "{operation}: {}",
            output_text(&output)
        );
    }
}

#[test]
fn old_positional_database_path_is_a_usage_error() {
    let workflow = Workflow::from_fixture_unprepared();
    let output = Command::new(env!("CARGO_BIN_EXE_control-tower"))
        .args(["db", "bootstrap-local"])
        .arg(workflow.path())
        .current_dir(workflow.workspace())
        .output()
        .expect("run Control Tower CLI");
    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn verifier_start_failure_is_operational_exit_one() {
    let workflow = Workflow::from_fixture();
    let verifier = workflow.path().join("stages/001-create-file/verify-up");
    fs::set_permissions(&verifier, fs::Permissions::from_mode(0o644)).unwrap();

    let output = move_to(&workflow, "up", 1);
    assert_eq!(output.status.code(), Some(1));
    assert!(output_text(&output).contains("could not start stage 1 verify-up"));
    assert!(
        String::from_utf8_lossy(&output.stdout)
            .contains("Failed role: stage 1 verify-up; executable did not start")
    );
}

#[test]
fn verifier_termination_by_signal_is_operational_exit_one() {
    for (direction, target, verifier) in [("up", 1, "verify-up"), ("down", 0, "verify-down")] {
        let workflow = Workflow::from_fixture();
        if direction == "down" {
            let up = move_to(&workflow, "up", 1);
            assert!(up.status.success(), "{}", output_text(&up));
        }
        write_executable(
            &workflow
                .path()
                .join("stages/001-create-file")
                .join(verifier),
            "#!/bin/sh\nkill -TERM $$\n",
        );

        let output = move_to(&workflow, direction, target);
        assert_eq!(output.status.code(), Some(1), "{}", output_text(&output));
        let text = output_text(&output);
        assert!(text.contains("signal or unknown status"));
        assert!(text.contains(&format!(
            "Failed role: stage 1 {verifier}; child exit status signal/unknown"
        )));
        assert!(text.contains(&format!("Pending verification: {direction} 1")));
    }
}

#[test]
fn workspace_dotenv_defaults_reach_roles_with_shell_and_control_tower_precedence() {
    let workflow = Workflow::from_fixture();
    fs::write(
        workflow.workspace().join(".env"),
        "CT_ENV_FROM_FILE=dotenv\nCT_ENV_OVERRIDE=dotenv\nCONTROL_TOWER_STAGE=wrong\n",
    )
    .unwrap();
    write_executable(
        &workflow.path().join("stages/001-create-file/up"),
        "#!/bin/sh\nset -eu\ndirectory=\"$CONTROL_TOWER_WORKFLOW/data\"\nmkdir -p \"$directory\"\n: > \"$directory/$CONTROL_TOWER_UUID\"\nprintf '%s|%s|%s' \"$CT_ENV_FROM_FILE\" \"$CT_ENV_OVERRIDE\" \"$CONTROL_TOWER_STAGE\" > \"$CONTROL_TOWER_WORKFLOW/env-record\"\n",
    );
    let output = Command::new(env!("CARGO_BIN_EXE_control-tower"))
        .args(["up", "--workflow"])
        .arg(workflow.path())
        .args(["--stage", "1"])
        .env("CT_ENV_OVERRIDE", "shell")
        .current_dir(workflow.workspace())
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", output_text(&output));
    assert_eq!(
        fs::read(workflow.path().join("env-record")).unwrap(),
        b"dotenv|shell|1"
    );
}

#[test]
fn invalid_workspace_dotenv_blocks_validation_and_movement_before_roles_run() {
    let workflow = Workflow::from_fixture();
    let marker = workflow.path().join("dotenv-role-ran");
    let up = workflow.path().join("stages/001-create-file/up");
    let original_up = fs::read_to_string(&up).unwrap();
    write_executable(
        &up,
        &format!("{original_up}\ntouch \"$CONTROL_TOWER_WORKFLOW/dotenv-role-ran\"\n"),
    );
    let initial = move_to(&workflow, "up", 1);
    assert!(initial.status.success(), "{}", output_text(&initial));
    assert!(marker.exists());
    fs::write(workflow.workspace().join(".env"), "INVALID LINE\n").unwrap();

    for args in [vec!["validate", "--workflow"], vec!["down", "--workflow"]] {
        let is_down = args[0] == "down";
        let mut command = Command::new(env!("CARGO_BIN_EXE_control-tower"));
        command.args(args).arg(workflow.path());
        if is_down {
            command.args(["--stage", "0"]);
        }
        let output = command.current_dir(workflow.workspace()).output().unwrap();
        assert!(!output.status.success());
        assert!(output_text(&output).contains("Workspace .env"));
        if is_down {
            assert_eq!(output.status.code(), Some(1));
            assert!(String::from_utf8_lossy(&output.stdout).contains(
                "Result: stopped before movement; requested down to 0; position 1 (create-file)"
            ));
            assert!(String::from_utf8_lossy(&output.stdout).contains("No roles ran."));
        }
        assert!(marker.exists());
    }
    let saved = checkpoint(&workflow);
    assert_eq!(saved.completed_stage_count, 1);
    assert!(saved.pending.is_none());
}

#[test]
fn parsed_movement_failures_always_report_target_and_known_or_unavailable_position() {
    let invalid_target_workflow = Workflow::from_fixture();
    let invalid_target = move_to(&invalid_target_workflow, "up", 999);
    assert_eq!(invalid_target.status.code(), Some(1));
    let invalid_target_output = output_text(&invalid_target);
    assert!(
        invalid_target_output.contains(
            "Result: stopped before movement; requested up to 999; position baseline (0)"
        )
    );
    assert!(invalid_target_output.contains("No roles ran."));
    assert!(!invalid_target_workflow.path().join("calls.log").exists());
    let database = rusqlite::Connection::open(
        invalid_target_workflow
            .path()
            .join(".control_tower/state.sqlite3"),
    )
    .unwrap();
    let checkpoint_rows: i64 = database
        .query_row("SELECT count(*) FROM workbench_state", [], |row| row.get(0))
        .unwrap();
    assert_eq!(checkpoint_rows, 0);

    let invalid_workspace = Workflow::from_fixture();
    fs::remove_file(invalid_workspace.workspace().join("control-tower.toml")).unwrap();
    let startup_failure = move_to(&invalid_workspace, "down", 0);
    assert_eq!(startup_failure.status.code(), Some(1));
    assert!(
        output_text(&startup_failure)
            .contains("Result: stopped before movement; requested down to 0; position unavailable")
    );
    assert!(output_text(&startup_failure).contains("No roles ran."));

    let discovery_workflow = Workflow::from_fixture();
    fs::remove_dir_all(discovery_workflow.path().join("stages")).unwrap();
    let discovery_failure = move_to(&discovery_workflow, "down", 0);
    assert_eq!(discovery_failure.status.code(), Some(1));
    let discovery_output = output_text(&discovery_failure);
    assert!(
        discovery_output
            .contains("Result: stopped before movement; requested down to 0; position unavailable")
    );
    assert!(discovery_output.contains("No roles ran."));

    let state_workflow = Workflow::from_fixture();
    fs::remove_file(state_workflow.path().join(".control_tower/state.sqlite3")).unwrap();
    let state_failure = move_to(&state_workflow, "up", 1);
    assert_eq!(state_failure.status.code(), Some(1));
    let state_output = output_text(&state_failure);
    assert!(
        state_output
            .contains("Result: stopped before movement; requested up to 1; position unavailable")
    );
    assert!(state_output.contains("No roles ran."));
    assert!(
        !state_workflow
            .path()
            .join(".control_tower/state.sqlite3")
            .exists()
    );
}

#[test]
fn database_operations_reject_paths_that_are_not_existing_directories() {
    let workflow = Workflow::from_fixture_unprepared();
    let missing = workflow.path().join("missing");
    let output = Command::new(env!("CARGO_BIN_EXE_control-tower"))
        .args(["db", "bootstrap-local", "--workflow"])
        .arg(missing)
        .current_dir(workflow.workspace())
        .output()
        .expect("run Control Tower database operation");
    assert!(!output.status.success());
    assert!(output_text(&output).contains("cannot open workflow"));

    let file = workflow.path().join("not-a-directory");
    fs::write(&file, "file").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_control-tower"))
        .args(["db", "bootstrap-local", "--workflow"])
        .arg(file)
        .current_dir(workflow.workspace())
        .output()
        .expect("run Control Tower database operation");
    assert!(!output.status.success());
    assert!(output_text(&output).contains("is not a directory"));
}

// Keep the author's scripts as the external semantic authority, adding only
// call/UUID observation and a first-attempt verifier failure for these tests.
fn observe_roles(workflow: &Workflow) {
    for directory in fs::read_dir(workflow.path().join("stages")).unwrap() {
        let directory = directory.unwrap().path();
        for role in ["up", "down", "verify-up", "verify-down"] {
            let path = directory.join(role);
            let script = fs::read_to_string(&path).unwrap();
            let (shebang, body) = script.split_once('\n').unwrap();
            write_executable(
                &path,
                &format!(
                    "{shebang}\nprintf '%s %s %s\\n' \"$CONTROL_TOWER_STAGE\" \"$CONTROL_TOWER_ROLE\" \"$CONTROL_TOWER_UUID\" >> \"$CONTROL_TOWER_WORKFLOW/calls.log\"\n{body}"
                ),
            );
        }
    }
}

fn fail_verifier_once(workflow: &Workflow, stage: &str, role: &str) {
    let path = workflow.path().join("stages").join(stage).join(role);
    let script = fs::read_to_string(&path).unwrap();
    // Insert after the observation so even the intentionally failed check is logged.
    let (prefix, body) = script.split_once("set -eu\n").unwrap();
    write_executable(
        &path,
        &format!(
            "{prefix}set -eu\nmarker=\"$CONTROL_TOWER_WORKFLOW/{stage}-{role}-attempted\"\nif [ ! -e \"$marker\" ]; then touch \"$marker\"; exit 23; fi\n{body}"
        ),
    );
}

fn checkpoint(workflow: &Workflow) -> control_tower_application::WorkbenchState {
    use control_tower_application::WorkbenchQueries;
    control_tower_database::workbench::SqliteWorkbenchQueries::open(
        &workflow.path().join(".control_tower/state.sqlite3"),
    )
    .unwrap()
    .read_checkpoint()
    .unwrap()
    .unwrap()
}

fn assert_checkpoint(workflow: &Workflow, completed: usize, pending: Option<(&str, usize)>) {
    let state = checkpoint(workflow);
    assert_eq!(state.completed_stage_count, completed);
    assert_eq!(
        state
            .pending
            .map(|p| (p.direction.as_str(), p.stage_index + 1)),
        pending
    );
    let output = status(workflow);
    assert!(output.status.success(), "{}", output_text(&output));
    let text = output_text(&output);
    assert!(text.contains(&format!("Completed stage: {completed}")));
    if let Some((direction, stage)) = pending {
        assert!(text.contains(&format!("Pending verification: {direction} {stage}")));
    } else {
        assert!(!text.contains("Pending verification:"));
    }
}

fn assert_calls_and_uuid(workflow: &Workflow, expected: &[&str], uuid: &str) {
    let text = fs::read_to_string(workflow.path().join("calls.log")).unwrap();
    let calls: Vec<_> = text
        .lines()
        .map(|line| {
            let (call, observed_uuid) = line.rsplit_once(' ').unwrap();
            assert_eq!(observed_uuid, uuid);
            call
        })
        .collect();
    assert_eq!(calls, expected);
}

#[test]
fn failed_verify_up_backs_out_same_stage_or_farther_across_cli_processes() {
    for target in [2, 1] {
        let workflow = Workflow::from_fixture();
        observe_roles(&workflow);
        fail_verifier_once(&workflow, "003-add-to-you", "verify-up");
        let first = move_to(&workflow, "up", 3);
        assert!(!first.status.success(), "{}", output_text(&first));
        assert_eq!(first.status.code(), Some(3));
        assert!(output_text(&first).contains("verify-up exited with status 23"));
        let summary = String::from_utf8_lossy(&first.stdout);
        assert!(summary.contains("Result: stopped; requested up to 3; position 2 (write-hello)"));
        assert!(summary.contains("Pending verification: up 3 (add-to-you)"));
        assert!(summary.contains("Failed role: stage 3 verify-up; child exit status 23"));
        assert_checkpoint(&workflow, 2, Some(("up", 3)));
        let uuid = checkpoint(&workflow).uuid.unwrap();
        assert_eq!(
            fs::read(uuid_file(&workflow).unwrap()).unwrap(),
            b"hello to you"
        );

        let reverse = move_to(&workflow, "down", target);
        assert!(reverse.status.success(), "{}", output_text(&reverse));
        assert_checkpoint(&workflow, target as usize, None);
        assert_eq!(checkpoint(&workflow).uuid.as_deref(), Some(uuid.as_str()));
        assert_eq!(
            fs::read(uuid_file(&workflow).unwrap()).unwrap(),
            if target == 2 { &b"hello"[..] } else { &b""[..] }
        );
        let mut expected = vec![
            "1 up",
            "1 verify-up",
            "2 up",
            "2 verify-up",
            "3 up",
            "3 verify-up",
            "3 down",
            "3 verify-down",
        ];
        if target == 1 {
            expected.extend(["2 down", "2 verify-down"]);
        }
        assert_calls_and_uuid(&workflow, &expected, &uuid);
    }
}

#[test]
fn rollback_verifier_failure_persists_and_retries_only_check_before_walking_farther() {
    let workflow = Workflow::from_fixture();
    observe_roles(&workflow);
    fail_verifier_once(&workflow, "003-add-to-you", "verify-up");
    fail_verifier_once(&workflow, "003-add-to-you", "verify-down");
    assert!(!move_to(&workflow, "up", 3).status.success());
    assert_checkpoint(&workflow, 2, Some(("up", 3)));
    let uuid = checkpoint(&workflow).uuid.unwrap();
    let reverse = move_to(&workflow, "down", 1);
    assert!(!reverse.status.success(), "{}", output_text(&reverse));
    assert!(output_text(&reverse).contains("verify-down exited with status 23"));
    assert_checkpoint(&workflow, 2, Some(("down", 3)));
    assert_eq!(checkpoint(&workflow).uuid.as_deref(), Some(uuid.as_str()));
    assert_eq!(fs::read(uuid_file(&workflow).unwrap()).unwrap(), b"hello");

    let retry = move_to(&workflow, "down", 1);
    assert!(retry.status.success(), "{}", output_text(&retry));
    assert_checkpoint(&workflow, 1, None);
    assert_eq!(fs::read(uuid_file(&workflow).unwrap()).unwrap(), b"");
    assert_calls_and_uuid(
        &workflow,
        &[
            "1 up",
            "1 verify-up",
            "2 up",
            "2 verify-up",
            "3 up",
            "3 verify-up",
            "3 down",
            "3 verify-down",
            "3 verify-down",
            "2 down",
            "2 verify-down",
        ],
        &uuid,
    );
}

#[test]
fn pending_down_reverses_up_and_reverse_verification_is_resumable_across_processes() {
    for fail_reverse_verifier in [false, true] {
        let workflow = Workflow::from_fixture();
        observe_roles(&workflow);
        let up = move_to(&workflow, "up", 3);
        assert!(up.status.success(), "{}", output_text(&up));
        let uuid = checkpoint(&workflow).uuid.unwrap();
        fail_verifier_once(&workflow, "003-add-to-you", "verify-down");
        let down = move_to(&workflow, "down", 2);
        assert!(!down.status.success(), "{}", output_text(&down));
        assert_checkpoint(&workflow, 3, Some(("down", 3)));
        if fail_reverse_verifier {
            fail_verifier_once(&workflow, "003-add-to-you", "verify-up");
        }
        let reverse = move_to(&workflow, "up", 3);
        assert_eq!(
            reverse.status.success(),
            !fail_reverse_verifier,
            "{}",
            output_text(&reverse)
        );
        let mut expected = vec![
            "1 up",
            "1 verify-up",
            "2 up",
            "2 verify-up",
            "3 up",
            "3 verify-up",
            "3 down",
            "3 verify-down",
            "3 up",
            "3 verify-up",
        ];
        if fail_reverse_verifier {
            assert_checkpoint(&workflow, 3, Some(("up", 3)));
            let retry = move_to(&workflow, "up", 3);
            assert!(retry.status.success(), "{}", output_text(&retry));
            expected.push("3 verify-up");
        }
        assert_checkpoint(&workflow, 3, None);
        assert_eq!(checkpoint(&workflow).uuid.as_deref(), Some(uuid.as_str()));
        assert_eq!(
            fs::read(uuid_file(&workflow).unwrap()).unwrap(),
            b"hello to you"
        );
        assert_calls_and_uuid(&workflow, &expected, &uuid);
    }
}

#[test]
fn role_feedback_arrives_while_roles_wait_and_output_is_not_replayed() {
    use std::io::{BufRead, BufReader, Read};
    use std::process::{Child, Stdio};
    use std::sync::mpsc;
    use std::time::{Duration, Instant};
    struct Running {
        child: Child,
        workflow: PathBuf,
    }
    impl Drop for Running {
        fn drop(&mut self) {
            for n in [1, 2] {
                let _ = fs::write(self.workflow.join(format!("release-{n}")), b"");
            }
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
    let workflow = Workflow::from_fixture();
    for (n, name) in [(1, "001-create-file"), (2, "002-write-hello")] {
        fs::remove_file(workflow.path().join("stages").join(name).join("verify-up")).unwrap();
        let mutation = if n == 1 {
            "mkdir -p \"$CONTROL_TOWER_WORKFLOW/data\"\n: > \"$CONTROL_TOWER_WORKFLOW/data/$CONTROL_TOWER_UUID\""
        } else {
            "printf hello > \"$CONTROL_TOWER_WORKFLOW/data/$CONTROL_TOWER_UUID\""
        };
        write_executable(
            &workflow.path().join("stages").join(name).join("up"),
            &format!(
                "#!/bin/sh\nset -eu\ntouch \"$CONTROL_TOWER_WORKFLOW/waiting-{n}\"\nattempt=0\nwhile [ ! -e \"$CONTROL_TOWER_WORKFLOW/release-{n}\" ]; do attempt=$((attempt+1)); test \"$attempt\" -lt 400; sleep 0.05; done\n{mutation}\nprintf 'role-{n}-bytes'\nprintf 'role-{n}-stderr' >&2\n"
            ),
        );
    }
    let child = Command::new(env!("CARGO_BIN_EXE_control-tower"))
        .args(["up", "--workflow"])
        .arg(workflow.path())
        .args(["--stage", "2"])
        .current_dir(workflow.workspace())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut running = Running {
        child,
        workflow: workflow.path().to_owned(),
    };
    let stdout = running.child.stdout.take().unwrap();
    let mut stderr = running.child.stderr.take().unwrap();
    let (tx, rx) = mpsc::channel();
    let reader = std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines() {
            if tx.send(line.unwrap()).is_err() {
                break;
            }
        }
    });
    let mut text = String::new();
    let mut until = |needle: &str| {
        loop {
            let line = rx
                .recv_timeout(Duration::from_secs(10))
                .expect("feedback before role release");
            text.push_str(&line);
            text.push('\n');
            if line.contains(needle) {
                break;
            }
        }
    };
    until("[stage 1 up (create-file)] starting");
    let deadline = Instant::now() + Duration::from_secs(10);
    while !workflow.path().join("waiting-1").exists() {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(running.child.try_wait().unwrap().is_none());
    fs::write(workflow.path().join("release-1"), b"").unwrap();
    until("[stage 1 up (create-file)] succeeded");
    until("[stage 2 up (write-hello)] starting");
    assert!(running.child.try_wait().unwrap().is_none());
    fs::write(workflow.path().join("release-2"), b"").unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(exit) = running.child.try_wait().unwrap() {
            assert!(exit.success());
            break;
        }
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(10));
    }
    reader.join().unwrap();
    for line in rx {
        text.push_str(&line);
        text.push('\n');
    }
    let mut errors = vec![];
    stderr.read_to_end(&mut errors).unwrap();
    assert_eq!(text.matches("role-1-bytes").count(), 1);
    assert_eq!(text.matches("role-2-bytes").count(), 1);
    assert!(text.contains("role-1-bytes\n[stage 1 up (create-file)] succeeded"));
    assert!(
        text.find("role-1-bytes").unwrap()
            < text.find("[stage 2 up (write-hello)] starting").unwrap()
    );
    assert_eq!(errors,b"[stage 1 up (create-file)] stderr:\nrole-1-stderr\n[stage 2 up (write-hello)] stderr:\nrole-2-stderr\n");
}

fn suggested_command(text: &str, heading: &str) -> String {
    text.split_once(heading)
        .unwrap()
        .1
        .lines()
        .nth(1)
        .unwrap()
        .trim()
        .to_owned()
}

#[test]
fn sparse_retry_and_reversal_commands_are_usable_and_resolve_only_the_active_stage() {
    let mut workflow = Workflow::from_fixture();
    let quoted = workflow.path().with_file_name(format!(
        "{} user's fixture",
        workflow.path().file_name().unwrap().to_string_lossy()
    ));
    fs::rename(workflow.path(), &quoted).unwrap();
    workflow.0 = quoted;
    for (old, new) in [
        ("001-create-file", "010-create-file"),
        ("002-write-hello", "200-write-hello"),
        ("003-add-to-you", "900-add-to-you"),
    ] {
        fs::rename(
            workflow.path().join("stages").join(old),
            workflow.path().join("stages").join(new),
        )
        .unwrap();
    }
    observe_roles(&workflow);
    fail_verifier_once(&workflow, "200-write-hello", "verify-up");
    let failed = move_to(&workflow, "up", 900);
    assert!(!failed.status.success());
    let text = output_text(&failed);
    assert!(text.contains("Completed stage: 10 (create-file)"));
    assert!(text.contains("Pending verification: up 200 (write-hello)"));
    assert!(!text.contains("stage index"));
    let retry = suggested_command(&text, "Retry this check only:");
    assert!(retry.ends_with("--stage 200"));
    assert!(retry.contains("'\\''"));
    assert!(retry.starts_with(&format!("'{}'", env!("CARGO_BIN_EXE_control-tower"))));
    let retry_output = Command::new("sh")
        .args(["-c", &retry])
        .current_dir(workflow.workspace())
        .output()
        .unwrap();
    assert!(
        retry_output.status.success(),
        "{}",
        output_text(&retry_output)
    );
    assert_eq!(checkpoint(&workflow).completed_stage_count, 2);
    let retry_text = output_text(&retry_output);
    assert!(retry_text.contains("stage 200 verify-up"));
    assert!(!retry_text.contains("stage 200 up"));
    assert!(!retry_text.contains("stage 900"));
    assert!(output_text(&move_to(&workflow, "up", 200)).contains("No roles ran"));
    assert!(move_to(&workflow, "up", 900).status.success());
    fail_verifier_once(&workflow, "900-add-to-you", "verify-down");
    let failed = move_to(&workflow, "down", 0);
    let text = output_text(&failed);
    let reverse = suggested_command(&text, "Reverse the active stage:");
    assert!(reverse.ends_with("--stage 900"));
    let reversed = Command::new("sh")
        .args(["-c", &reverse])
        .current_dir(workflow.workspace())
        .output()
        .unwrap();
    assert!(reversed.status.success(), "{}", output_text(&reversed));
    assert!(!output_text(&reversed).contains("No roles ran"));
    assert_eq!(
        fs::read(uuid_file(&workflow).unwrap()).unwrap(),
        b"hello to you"
    );
    let failed = move_to(&workflow, "down", 0); // Once-failing check now passes, so the whole walk settles.
    assert!(failed.status.success());
    assert_eq!(
        checkpoint(&workflow),
        control_tower_application::WorkbenchState::default()
    );
}

#[test]
fn failure_choices_do_not_advertise_missing_reverse_or_misclassify_failed_reverse_mutation() {
    let workflow = Workflow::from_fixture();
    fail_verifier_once(&workflow, "003-add-to-you", "verify-up");
    let stage = workflow.path().join("stages/003-add-to-you");
    fs::remove_file(stage.join("down")).unwrap();
    fs::remove_file(stage.join("verify-down")).unwrap();
    let failed = move_to(&workflow, "up", 3);
    let text = output_text(&failed);
    assert!(text.contains("Retry this check only:"));
    assert!(!text.contains("Reverse the active stage:"));
    write_executable(&stage.join("down"), "#!/bin/sh\nexit 17\n");
    let failed = move_to(&workflow, "down", 2);
    let text = output_text(&failed);
    assert!(text.contains("Pending verification: up 3"));
    assert!(text.contains("Inspect author-owned effects"));
    assert!(!text.contains("Retry this check only:"));
    assert!(!text.contains("Reverse the active stage:"));
}

#[test]
fn guide_index_and_exact_embedded_actions_work_without_a_workflow() {
    let workflow = Workflow::from_fixture();
    let cwd = workflow.path().join("not-a-workflow");
    fs::create_dir(&cwd).unwrap();
    let index = Command::new(env!("CARGO_BIN_EXE_control-tower"))
        .arg("guide")
        .current_dir(&cwd)
        .output()
        .unwrap();
    assert!(index.status.success(), "{}", output_text(&index));
    let text = String::from_utf8(index.stdout).unwrap();
    let actions: Vec<_> = text
        .lines()
        .filter_map(|line| {
            line.strip_prefix("- `")
                .and_then(|line| line.split_once('`'))
                .map(|(action, _)| action)
        })
        .collect();
    assert_eq!(
        actions,
        [
            "create_workflow",
            "edit_workflow",
            "workflow_contract",
            "operate_workflow",
            "recover_workflow",
        ]
    );
    assert!(!cwd.join(".control_tower").exists());

    for (action, heading) in [
        ("create_workflow", "# Create a workflow"),
        ("edit_workflow", "# Edit an existing workflow"),
        ("workflow_contract", "# Workflow and executable contract"),
        ("operate_workflow", "# Operate an existing workflow"),
        (
            "recover_workflow",
            "# Recover a failed, pending, or uncertain workflow",
        ),
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_control-tower"))
            .args(["guide", action])
            .current_dir(&cwd)
            .output()
            .unwrap();
        assert!(output.status.success(), "{}", output_text(&output));
        assert!(
            String::from_utf8(output.stdout)
                .unwrap()
                .starts_with(heading)
        );
    }

    let create = Command::new(env!("CARGO_BIN_EXE_control-tower"))
        .args(["guide", "create_workflow"])
        .current_dir(&cwd)
        .output()
        .unwrap();
    let create = String::from_utf8(create.stdout).unwrap();
    assert!(create.contains("control-tower validate"));
    assert!(!create.contains("status --workflow"));
    let edit = Command::new(env!("CARGO_BIN_EXE_control-tower"))
        .args(["guide", "edit_workflow"])
        .current_dir(&cwd)
        .output()
        .unwrap();
    let edit = String::from_utf8(edit.stdout).unwrap();
    assert!(edit.contains("control-tower validate"));
    assert!(edit.contains("status --workflow workflows/NAME"));
    assert!(!edit.contains("1. Choose a new directory"));

    let extra_action = Command::new(env!("CARGO_BIN_EXE_control-tower"))
        .args(["guide", "validate"])
        .current_dir(cwd)
        .output()
        .unwrap();
    assert!(!extra_action.status.success());
}

#[test]
fn validate_uses_selected_workflow_without_running_roles_or_changing_storage() {
    let workflow = Workflow::from_fixture();
    let role_marker = workflow.path().join("validate-ran-a-role");
    for entry in fs::read_dir(workflow.path().join("stages")).unwrap() {
        let directory = entry.unwrap().path();
        for role in ["up", "down", "verify-up", "verify-down"] {
            let path = directory.join(role);
            if path.is_file() {
                write_executable(
                    &path,
                    "#!/bin/sh\nset -eu\ntouch \"$CONTROL_TOWER_WORKFLOW/validate-ran-a-role\"\n",
                );
            }
        }
    }
    let database = workflow.path().join(".control_tower/state.sqlite3");
    let before = fs::read(&database).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_control-tower"))
        .arg("validate")
        .arg("--workflow")
        .arg(workflow.path())
        .current_dir(workflow.workspace())
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", output_text(&output));
    assert!(String::from_utf8_lossy(&output.stdout).contains("Workflow loaded successfully:"));
    assert!(String::from_utf8_lossy(&output.stdout).contains("Discovered stages: 3"));
    assert!(!role_marker.exists(), "validation must not execute a role");
    assert_eq!(fs::read(&database).unwrap(), before);

    let path_option = Command::new(env!("CARGO_BIN_EXE_control-tower"))
        .args(["validate", "--workflow"])
        .arg(workflow.path())
        .current_dir(workflow.workspace())
        .output()
        .unwrap();
    assert!(
        path_option.status.success(),
        "{}",
        output_text(&path_option)
    );

    let invalid = workflow.path().join("not-a-workflow");
    fs::create_dir(&invalid).unwrap();
    let failed = Command::new(env!("CARGO_BIN_EXE_control-tower"))
        .arg("validate")
        .arg("--workflow")
        .arg(&invalid)
        .current_dir(&invalid)
        .output()
        .unwrap();
    assert!(!failed.status.success());
    assert!(output_text(&failed).contains("error:"));
    assert!(!invalid.join(".control_tower").exists());
}

#[test]
fn conditional_sqlite_save_failures_report_confirmed_checkpoint_and_retain_role_output() {
    for (name, condition, expected_calls) in [
        (
            "pending",
            "NEW.pending_stage_index IS NOT NULL",
            vec!["1 up"],
        ),
        (
            "final",
            "NEW.completed_stage_count = 1",
            vec!["1 up", "1 verify-up"],
        ),
    ] {
        let workflow = Workflow::from_fixture();
        observe_roles(&workflow);
        let database = workflow.path().join(".control_tower/state.sqlite3");
        let connection = rusqlite::Connection::open(database).unwrap();
        connection.execute_batch(&format!("CREATE TRIGGER reject_checkpoint BEFORE INSERT ON workbench_state WHEN {condition} BEGIN SELECT RAISE(FAIL, 'injected {name} failure'); END;")).unwrap();
        let failed = move_to(&workflow, "up", 3);
        assert!(!failed.status.success());
        let text = String::from_utf8_lossy(&failed.stdout);
        assert!(text.contains("created "));
        assert!(text.contains("[stage 1 up (create-file)] succeeded"));
        assert!(!text.contains("[stage 2"));
        assert!(!text.contains("Retry this check only:"));
        let (confirmed, attempted) = text
            .split_once("Last confirmed checkpoint:\n")
            .unwrap()
            .1
            .split_once("Unconfirmed checkpoint update:\n")
            .unwrap();
        let status = status(&workflow);
        assert!(status.status.success());
        assert_eq!(confirmed, String::from_utf8_lossy(&status.stdout));
        assert!(confirmed.contains("Completed stage: baseline (0)"));
        assert_eq!(
            confirmed.contains("Pending verification: up 1"),
            name == "final"
        );
        if name == "pending" {
            assert!(attempted.contains("Pending verification: up 1"));
            assert!(!text.contains("UUID file exists"));
        } else {
            assert!(attempted.contains("Completed stage: 1 (create-file)"));
            assert!(text.contains("UUID file exists"));
        }
        let uuid = checkpoint(&workflow).uuid.unwrap();
        assert_calls_and_uuid(&workflow, &expected_calls, &uuid);
        connection
            .execute_batch("DROP TRIGGER reject_checkpoint")
            .unwrap();
        let retry = move_to(&workflow, "up", 1);
        assert!(retry.status.success());
        let text = output_text(&retry);
        assert_eq!(
            text.contains("[stage 1 up (create-file)] starting"),
            name == "pending"
        );
        assert_eq!(checkpoint(&workflow).completed_stage_count, 1);
    }
}

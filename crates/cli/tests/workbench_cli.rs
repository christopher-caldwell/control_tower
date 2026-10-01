#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static NEXT_WORKSPACE: AtomicU64 = AtomicU64::new(0);

struct Workspace(PathBuf);

impl Workspace {
    fn from_fixture() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after the Unix epoch")
            .as_nanos();
        let workspace = std::env::temp_dir().join(format!(
            "control-tower-{}-{nonce}-{}",
            std::process::id(),
            NEXT_WORKSPACE.fetch_add(1, Ordering::Relaxed)
        ));
        copy_directory(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/uuid-file"),
            &workspace,
        );
        let database = workspace.join(".control_tower/state.sqlite3");
        control_tower_database::operations::bootstrap(&database)
            .expect("bootstrap fixture database");
        control_tower_database::operations::migrate(&database).expect("migrate fixture database");
        Self(workspace)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for Workspace {
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

fn move_to(workspace: &Workspace, direction: &str, target: u32) -> Output {
    Command::new(env!("CARGO_BIN_EXE_control-tower"))
        .arg(direction)
        .arg("--workspace")
        .arg(workspace.path())
        .arg("--stage")
        .arg(target.to_string())
        .output()
        .expect("run Control Tower CLI")
}

fn status(workspace: &Workspace) -> Output {
    Command::new(env!("CARGO_BIN_EXE_control-tower"))
        .arg("status")
        .arg("--workspace")
        .arg(workspace.path())
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

fn uuid_file(workspace: &Workspace) -> Option<PathBuf> {
    let data = workspace.path().join("data");
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
    let workspace = Workspace::from_fixture();
    observe_roles(&workspace);

    let first_up = move_to(&workspace, "up", 1);
    assert!(first_up.status.success(), "{}", output_text(&first_up));
    let first_uuid_file = uuid_file(&workspace).expect("stage 1 creates the UUID file");
    assert_eq!(fs::read(&first_uuid_file).unwrap(), b"");
    let uuid = first_uuid_file
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string();

    let up_to_three = move_to(&workspace, "up", 3);
    assert!(
        up_to_three.status.success(),
        "{}",
        output_text(&up_to_three)
    );
    let third_uuid_file = uuid_file(&workspace).expect("UUID file remains present");
    assert_eq!(third_uuid_file.file_name().unwrap().to_string_lossy(), uuid);
    assert_eq!(fs::read(&third_uuid_file).unwrap(), b"hello to you");

    let down_to_two = move_to(&workspace, "down", 2);
    assert!(
        down_to_two.status.success(),
        "{}",
        output_text(&down_to_two)
    );
    assert_eq!(fs::read(&third_uuid_file).unwrap(), b"hello");

    let down_to_one = move_to(&workspace, "down", 1);
    assert!(
        down_to_one.status.success(),
        "{}",
        output_text(&down_to_one)
    );
    assert_eq!(fs::read(&third_uuid_file).unwrap(), b"");

    let down_to_zero = move_to(&workspace, "down", 0);
    assert!(
        down_to_zero.status.success(),
        "{}",
        output_text(&down_to_zero)
    );
    assert!(uuid_file(&workspace).is_none());
    let final_status = status(&workspace);
    assert!(
        final_status.status.success(),
        "{}",
        output_text(&final_status)
    );
    assert!(String::from_utf8_lossy(&final_status.stdout).contains("UUID: not created"));
    assert_eq!(
        checkpoint(&workspace),
        control_tower_application::WorkbenchState::default()
    );
    assert_calls_and_uuid(
        &workspace,
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
    let workspace = Workspace::from_fixture();
    let stage_two = workspace.path().join("stages/002-write-hello");
    write_executable(
        &stage_two.join("up"),
        "#!/bin/sh\nset -eu\nprintf 'up\\n' >> \"$CONTROL_TOWER_WORKSPACE/stage-2-up.log\"\nprintf 'hello' > \"$CONTROL_TOWER_WORKSPACE/data/$CONTROL_TOWER_UUID\"\n",
    );
    write_executable(
        &stage_two.join("verify-up"),
        "#!/bin/sh\nset -eu\nprintf 'verify\\n' >> \"$CONTROL_TOWER_WORKSPACE/stage-2-verify-up.log\"\nmarker=\"$CONTROL_TOWER_WORKSPACE/verify-up-attempted\"\nif [ ! -e \"$marker\" ]; then touch \"$marker\"; exit 23; fi\ntest \"$(cat \"$CONTROL_TOWER_WORKSPACE/data/$CONTROL_TOWER_UUID\")\" = hello\n",
    );

    let first = move_to(&workspace, "up", 2);
    assert!(!first.status.success(), "verification should fail once");
    assert!(output_text(&first).contains("verify-up exited with status 23"));
    let pending_status = status(&workspace);
    assert!(String::from_utf8_lossy(&pending_status.stdout).contains("Pending verification: up 2"));
    assert_eq!(
        fs::read_to_string(workspace.path().join("stage-2-up.log")).unwrap(),
        "up\n"
    );

    let retry = move_to(&workspace, "up", 2);
    assert!(retry.status.success(), "{}", output_text(&retry));
    assert!(String::from_utf8_lossy(&retry.stdout).contains("stage 2 verify-up"));
    assert!(!String::from_utf8_lossy(&retry.stdout).contains("stage 2 up"));
    assert_eq!(
        fs::read_to_string(workspace.path().join("stage-2-up.log")).unwrap(),
        "up\n"
    );
    assert_eq!(
        fs::read_to_string(workspace.path().join("stage-2-verify-up.log")).unwrap(),
        "verify\nverify\n"
    );
}

#[test]
fn retries_failed_down_verification_without_replaying_mutation() {
    let workspace = Workspace::from_fixture();
    let stage_three = workspace.path().join("stages/003-add-to-you");
    write_executable(
        &stage_three.join("down"),
        "#!/bin/sh\nset -eu\nprintf 'down\\n' >> \"$CONTROL_TOWER_WORKSPACE/stage-3-down.log\"\ntest \"$(cat \"$CONTROL_TOWER_WORKSPACE/data/$CONTROL_TOWER_UUID\")\" = 'hello to you'\nprintf hello > \"$CONTROL_TOWER_WORKSPACE/data/$CONTROL_TOWER_UUID\"\n",
    );
    write_executable(
        &stage_three.join("verify-down"),
        "#!/bin/sh\nset -eu\nprintf 'verify\\n' >> \"$CONTROL_TOWER_WORKSPACE/stage-3-verify-down.log\"\nmarker=\"$CONTROL_TOWER_WORKSPACE/verify-down-attempted\"\nif [ ! -e \"$marker\" ]; then touch \"$marker\"; exit 24; fi\ntest \"$(cat \"$CONTROL_TOWER_WORKSPACE/data/$CONTROL_TOWER_UUID\")\" = hello\n",
    );

    let up = move_to(&workspace, "up", 3);
    assert!(up.status.success(), "{}", output_text(&up));
    let first_down = move_to(&workspace, "down", 2);
    assert!(
        !first_down.status.success(),
        "verification should fail once"
    );
    assert!(output_text(&first_down).contains("verify-down exited with status 24"));
    let pending_status = status(&workspace);
    assert!(
        String::from_utf8_lossy(&pending_status.stdout).contains("Pending verification: down 3")
    );
    assert_eq!(
        fs::read_to_string(workspace.path().join("stage-3-down.log")).unwrap(),
        "down\n"
    );

    let retry = move_to(&workspace, "down", 2);
    assert!(retry.status.success(), "{}", output_text(&retry));
    assert!(String::from_utf8_lossy(&retry.stdout).contains("stage 3 verify-down"));
    assert!(!String::from_utf8_lossy(&retry.stdout).contains("stage 3 down"));
    assert_eq!(
        fs::read_to_string(workspace.path().join("stage-3-down.log")).unwrap(),
        "down\n"
    );
    assert_eq!(
        fs::read_to_string(workspace.path().join("stage-3-verify-down.log")).unwrap(),
        "verify\nverify\n"
    );
    assert_eq!(fs::read(uuid_file(&workspace).unwrap()).unwrap(), b"hello");
}

#[test]
fn omitted_directional_verifiers_do_not_block_movement() {
    let workspace = Workspace::from_fixture();
    let stage_two = workspace.path().join("stages/002-write-hello");
    fs::remove_file(stage_two.join("verify-up")).unwrap();
    fs::remove_file(stage_two.join("verify-down")).unwrap();

    let up = move_to(&workspace, "up", 2);
    assert!(up.status.success(), "{}", output_text(&up));
    assert_eq!(fs::read(uuid_file(&workspace).unwrap()).unwrap(), b"hello");

    let down = move_to(&workspace, "down", 1);
    assert!(down.status.success(), "{}", output_text(&down));
    assert_eq!(fs::read(uuid_file(&workspace).unwrap()).unwrap(), b"");
}

#[test]
fn failed_mutation_stops_before_its_verifier_and_later_stages() {
    let workspace = Workspace::from_fixture();
    let stage_two = workspace.path().join("stages/002-write-hello");
    let stage_three = workspace.path().join("stages/003-add-to-you");
    write_executable(
        &stage_two.join("up"),
        "#!/bin/sh\nprintf 'mutation\\n' >> \"$CONTROL_TOWER_WORKSPACE/mutation.log\"\nexit 17\n",
    );
    write_executable(
        &stage_two.join("verify-up"),
        "#!/bin/sh\nprintf 'verify\\n' >> \"$CONTROL_TOWER_WORKSPACE/verifier.log\"\n",
    );
    write_executable(
        &stage_three.join("up"),
        "#!/bin/sh\nprintf 'stage 3\\n' >> \"$CONTROL_TOWER_WORKSPACE/later-stage.log\"\n",
    );

    let failed_move = move_to(&workspace, "up", 3);
    assert!(!failed_move.status.success(), "mutation should fail");
    assert!(output_text(&failed_move).contains("stage 2 up exited with status 17"));
    assert_eq!(
        fs::read_to_string(workspace.path().join("mutation.log")).unwrap(),
        "mutation\n"
    );
    assert!(!workspace.path().join("verifier.log").exists());
    assert!(!workspace.path().join("later-stage.log").exists());

    let final_status = status(&workspace);
    let status_text = String::from_utf8_lossy(&final_status.stdout);
    assert!(status_text.contains("Completed stage: 1"));
    assert!(!status_text.contains("Pending verification:"));
}

#[test]
fn ordinary_cli_does_not_bootstrap_or_migrate() {
    let workspace = Workspace::from_fixture();
    fs::remove_dir_all(workspace.path().join(".control_tower")).unwrap();
    let output = status(&workspace);
    assert!(!output.status.success());
    assert!(output_text(&output).contains("explicit local database setup"));
    assert!(!workspace.path().join(".control_tower").exists());
    let database = workspace.path().join(".control_tower/state.sqlite3");
    control_tower_database::operations::bootstrap(&database).unwrap();
    let output = move_to(&workspace, "up", 1);
    assert!(!output.status.success());
    assert!(output_text(&output).contains("schema is not initialized"));
    assert!(uuid_file(&workspace).is_none());
    assert!(control_tower_database::operations::verify(&database).is_err());
}

// Keep the author's scripts as the external semantic authority, adding only
// call/UUID observation and a first-attempt verifier failure for these tests.
fn observe_roles(workspace: &Workspace) {
    for directory in fs::read_dir(workspace.path().join("stages")).unwrap() {
        let directory = directory.unwrap().path();
        for role in ["up", "down", "verify-up", "verify-down"] {
            let path = directory.join(role);
            let script = fs::read_to_string(&path).unwrap();
            let (shebang, body) = script.split_once('\n').unwrap();
            write_executable(
                &path,
                &format!(
                    "{shebang}\nprintf '%s %s %s\\n' \"$CONTROL_TOWER_STAGE\" \"$CONTROL_TOWER_ROLE\" \"$CONTROL_TOWER_UUID\" >> \"$CONTROL_TOWER_WORKSPACE/calls.log\"\n{body}"
                ),
            );
        }
    }
}

fn fail_verifier_once(workspace: &Workspace, stage: &str, role: &str) {
    let path = workspace.path().join("stages").join(stage).join(role);
    let script = fs::read_to_string(&path).unwrap();
    // Insert after the observation so even the intentionally failed check is logged.
    let (prefix, body) = script.split_once("set -eu\n").unwrap();
    write_executable(
        &path,
        &format!(
            "{prefix}set -eu\nmarker=\"$CONTROL_TOWER_WORKSPACE/{stage}-{role}-attempted\"\nif [ ! -e \"$marker\" ]; then touch \"$marker\"; exit 23; fi\n{body}"
        ),
    );
}

fn checkpoint(workspace: &Workspace) -> control_tower_application::WorkbenchState {
    use control_tower_application::WorkbenchQueries;
    control_tower_database::workbench::SqliteWorkbenchQueries::open(
        &workspace.path().join(".control_tower/state.sqlite3"),
    )
    .unwrap()
    .read_checkpoint()
    .unwrap()
    .unwrap()
}

fn assert_checkpoint(workspace: &Workspace, completed: usize, pending: Option<(&str, usize)>) {
    let state = checkpoint(workspace);
    assert_eq!(state.completed_stage_count, completed);
    assert_eq!(
        state
            .pending
            .map(|p| (p.direction.as_str(), p.stage_index + 1)),
        pending
    );
    let output = status(workspace);
    assert!(output.status.success(), "{}", output_text(&output));
    let text = output_text(&output);
    assert!(text.contains(&format!("Completed stage: {completed}")));
    if let Some((direction, stage)) = pending {
        assert!(text.contains(&format!("Pending verification: {direction} {stage}")));
    } else {
        assert!(!text.contains("Pending verification:"));
    }
}

fn assert_calls_and_uuid(workspace: &Workspace, expected: &[&str], uuid: &str) {
    let text = fs::read_to_string(workspace.path().join("calls.log")).unwrap();
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
        let workspace = Workspace::from_fixture();
        observe_roles(&workspace);
        fail_verifier_once(&workspace, "003-add-to-you", "verify-up");
        let first = move_to(&workspace, "up", 3);
        assert!(!first.status.success(), "{}", output_text(&first));
        assert!(output_text(&first).contains("verify-up exited with status 23"));
        assert_checkpoint(&workspace, 2, Some(("up", 3)));
        let uuid = checkpoint(&workspace).uuid.unwrap();
        assert_eq!(
            fs::read(uuid_file(&workspace).unwrap()).unwrap(),
            b"hello to you"
        );

        let reverse = move_to(&workspace, "down", target);
        assert!(reverse.status.success(), "{}", output_text(&reverse));
        assert_checkpoint(&workspace, target as usize, None);
        assert_eq!(checkpoint(&workspace).uuid.as_deref(), Some(uuid.as_str()));
        assert_eq!(
            fs::read(uuid_file(&workspace).unwrap()).unwrap(),
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
        assert_calls_and_uuid(&workspace, &expected, &uuid);
    }
}

#[test]
fn rollback_verifier_failure_persists_and_retries_only_check_before_walking_farther() {
    let workspace = Workspace::from_fixture();
    observe_roles(&workspace);
    fail_verifier_once(&workspace, "003-add-to-you", "verify-up");
    fail_verifier_once(&workspace, "003-add-to-you", "verify-down");
    assert!(!move_to(&workspace, "up", 3).status.success());
    assert_checkpoint(&workspace, 2, Some(("up", 3)));
    let uuid = checkpoint(&workspace).uuid.unwrap();
    let reverse = move_to(&workspace, "down", 1);
    assert!(!reverse.status.success(), "{}", output_text(&reverse));
    assert!(output_text(&reverse).contains("verify-down exited with status 23"));
    assert_checkpoint(&workspace, 2, Some(("down", 3)));
    assert_eq!(checkpoint(&workspace).uuid.as_deref(), Some(uuid.as_str()));
    assert_eq!(fs::read(uuid_file(&workspace).unwrap()).unwrap(), b"hello");

    let retry = move_to(&workspace, "down", 1);
    assert!(retry.status.success(), "{}", output_text(&retry));
    assert_checkpoint(&workspace, 1, None);
    assert_eq!(fs::read(uuid_file(&workspace).unwrap()).unwrap(), b"");
    assert_calls_and_uuid(
        &workspace,
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
        let workspace = Workspace::from_fixture();
        observe_roles(&workspace);
        let up = move_to(&workspace, "up", 3);
        assert!(up.status.success(), "{}", output_text(&up));
        let uuid = checkpoint(&workspace).uuid.unwrap();
        fail_verifier_once(&workspace, "003-add-to-you", "verify-down");
        let down = move_to(&workspace, "down", 2);
        assert!(!down.status.success(), "{}", output_text(&down));
        assert_checkpoint(&workspace, 3, Some(("down", 3)));
        if fail_reverse_verifier {
            fail_verifier_once(&workspace, "003-add-to-you", "verify-up");
        }
        let reverse = move_to(&workspace, "up", 3);
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
            assert_checkpoint(&workspace, 3, Some(("up", 3)));
            let retry = move_to(&workspace, "up", 3);
            assert!(retry.status.success(), "{}", output_text(&retry));
            expected.push("3 verify-up");
        }
        assert_checkpoint(&workspace, 3, None);
        assert_eq!(checkpoint(&workspace).uuid.as_deref(), Some(uuid.as_str()));
        assert_eq!(
            fs::read(uuid_file(&workspace).unwrap()).unwrap(),
            b"hello to you"
        );
        assert_calls_and_uuid(&workspace, &expected, &uuid);
    }
}

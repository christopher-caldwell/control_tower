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

use control_tower_application::{
    Direction, ExecutableRole, ExecutableRunner, Invocation, StageDiscovery,
};
use control_tower_infrastructure::{
    executable_runner::SystemExecutableRunner, stage_discovery::FilesystemStageDiscovery,
};
use std::error::Error;
use std::io;

fn has_io_source(mut error: &(dyn Error + 'static)) -> bool {
    loop {
        if error.is::<io::Error>() {
            return true;
        }
        match error.source() {
            Some(source) => error = source,
            None => return false,
        }
    }
}

#[test]
fn filesystem_and_process_errors_keep_io_sources() {
    let missing =
        std::env::temp_dir().join(format!("control-tower-missing-{}", std::process::id()));
    let discovery = FilesystemStageDiscovery.discover(&missing).unwrap_err();
    assert!(has_io_source(&discovery));
    let invocation = Invocation {
        executable: missing.join("up"),
        working_directory: std::env::temp_dir(),
        workflow_root: missing,
        stage_number: 1,
        direction: Direction::Up,
        role: ExecutableRole::Up,
        uuid: "opaque".into(),
    };
    let execution = SystemExecutableRunner::with_env_overrides(Default::default())
        .run(&invocation)
        .unwrap_err();
    assert!(has_io_source(&execution));
}

use control_tower_application::{ExecutableRunError, ExecutableRunner, Invocation, ProcessOutput};
use std::process::Command;

pub struct SystemExecutableRunner;

impl ExecutableRunner for SystemExecutableRunner {
    fn run(&self, invocation: &Invocation) -> Result<ProcessOutput, ExecutableRunError> {
        let output = Command::new(&invocation.executable)
            .current_dir(&invocation.working_directory)
            .env("CONTROL_TOWER_WORKFLOW", &invocation.workflow_root)
            .env("CONTROL_TOWER_UUID", &invocation.uuid)
            .env("CONTROL_TOWER_STAGE", invocation.stage_number.to_string())
            .env("CONTROL_TOWER_DIRECTION", invocation.direction.as_str())
            .env("CONTROL_TOWER_ROLE", invocation.role.as_str())
            .output()
            .map_err(|error| {
                ExecutableRunError::new(ContextError::new(
                    invocation.executable.display().to_string(),
                    error,
                ))
            })?;
        Ok(ProcessOutput {
            success: output.status.success(),
            exit_code: output.status.code(),
            stdout: output.stdout,
            stderr: output.stderr,
        })
    }
}

#[derive(Debug, thiserror::Error)]
#[error("{context}: {source}")]
struct ContextError {
    context: String,
    #[source]
    source: Box<dyn std::error::Error + Send + Sync>,
}
impl ContextError {
    fn new(
        context: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self {
            context: context.into(),
            source: Box::new(source),
        }
    }
}

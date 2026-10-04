use std::fs;
use std::path::{Path, PathBuf};

use control_tower_application::{Stage, StageDiscovery, StageDiscoveryError};

pub struct FilesystemStageDiscovery;

impl StageDiscovery for FilesystemStageDiscovery {
    fn discover(&self, workflow_root: &Path) -> Result<Vec<Stage>, StageDiscoveryError> {
        let stages_root = workflow_root.join("stages");
        let entries = fs::read_dir(&stages_root).map_err(|error| {
            StageDiscoveryError::new(ContextError::new(
                format!("cannot read {}", stages_root.display()),
                error,
            ))
        })?;
        let mut stages = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|error| {
                StageDiscoveryError::new(ContextError::new("cannot read a stage entry", error))
            })?;
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let folder_name = entry.file_name().to_string_lossy().into_owned();
            let (number, name) = parse_stage_name(&folder_name)?;
            let up = executable_file(&path, "up")?;
            let down = executable_file(&path, "down")?;
            let verify_up = executable_file(&path, "verify-up")?;
            let verify_down = executable_file(&path, "verify-down")?;
            if up.is_none() && down.is_none() {
                return Err(StageDiscoveryError::message(format!(
                    "{} must contain an `up` or `down` executable",
                    path.display()
                )));
            }
            if up.is_none() && verify_up.is_some() {
                return Err(StageDiscoveryError::message(format!(
                    "{} contains `verify-up` but no `up` executable",
                    path.display()
                )));
            }
            if down.is_none() && verify_down.is_some() {
                return Err(StageDiscoveryError::message(format!(
                    "{} contains `verify-down` but no `down` executable",
                    path.display()
                )));
            }
            stages.push(Stage {
                number,
                name,
                directory: path,
                up,
                down,
                verify_up,
                verify_down,
            });
        }
        stages.sort_by_key(|stage| stage.number);
        for pair in stages.windows(2) {
            if pair[0].number == pair[1].number {
                return Err(StageDiscoveryError::message(format!(
                    "stage number {} is used more than once",
                    pair[0].number
                )));
            }
        }
        Ok(stages)
    }
}

fn parse_stage_name(folder_name: &str) -> Result<(u32, String), StageDiscoveryError> {
    let (number_text, name) = match folder_name.split_once('-') {
        Some((number, name)) => (number, name),
        None => (folder_name, ""),
    };
    if number_text.is_empty()
        || !number_text
            .chars()
            .all(|character| character.is_ascii_digit())
    {
        return Err(StageDiscoveryError::message(format!(
            "stage directory `{folder_name}` must begin with a number, such as `001-create-file`"
        )));
    }
    let number = number_text.parse::<u32>().map_err(|error| {
        StageDiscoveryError::new(ContextError::new(
            format!("stage number `{number_text}` is too large"),
            error,
        ))
    })?;
    if number == 0 {
        return Err(StageDiscoveryError::message(
            "stage numbers must be greater than zero",
        ));
    }
    let name = if name.is_empty() {
        folder_name.to_owned()
    } else {
        name.to_owned()
    };
    Ok((number, name))
}

fn executable_file(directory: &Path, name: &str) -> Result<Option<PathBuf>, StageDiscoveryError> {
    let path = directory.join(name);
    match fs::metadata(&path) {
        Ok(metadata) if metadata.is_file() => Ok(Some(path)),
        Ok(_) => Err(StageDiscoveryError::message(format!(
            "{} is not a regular file",
            path.display()
        ))),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(StageDiscoveryError::new(ContextError::new(
            format!("cannot inspect {}", path.display()),
            error,
        ))),
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

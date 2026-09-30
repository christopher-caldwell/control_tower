use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use control_tower_application::{
    Direction, ExecutableRunner, Invocation, ProcessOutput, Stage, StageDiscovery, WorkbenchState,
    WorkbenchStateStore,
};
use rusqlite::{Connection, OptionalExtension, params};

pub struct FilesystemStageDiscovery;

impl StageDiscovery for FilesystemStageDiscovery {
    fn discover(&self, workspace_root: &Path) -> Result<Vec<Stage>, String> {
        let stages_root = workspace_root.join("stages");
        let entries = fs::read_dir(&stages_root)
            .map_err(|error| format!("cannot read {}: {error}", stages_root.display()))?;
        let mut stages = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|error| format!("cannot read a stage entry: {error}"))?;
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
                return Err(format!(
                    "{} must contain an `up` or `down` executable",
                    path.display()
                ));
            }
            if up.is_none() && verify_up.is_some() {
                return Err(format!(
                    "{} contains `verify-up` but no `up` executable",
                    path.display()
                ));
            }
            if down.is_none() && verify_down.is_some() {
                return Err(format!(
                    "{} contains `verify-down` but no `down` executable",
                    path.display()
                ));
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
                return Err(format!(
                    "stage number {} is used more than once",
                    pair[0].number
                ));
            }
        }
        Ok(stages)
    }
}

fn parse_stage_name(folder_name: &str) -> Result<(u32, String), String> {
    let (number_text, name) = match folder_name.split_once('-') {
        Some((number, name)) => (number, name),
        None => (folder_name, ""),
    };
    if number_text.is_empty()
        || !number_text
            .chars()
            .all(|character| character.is_ascii_digit())
    {
        return Err(format!(
            "stage directory `{folder_name}` must begin with a number, such as `001-create-file`"
        ));
    }
    let number = number_text
        .parse::<u32>()
        .map_err(|_| format!("stage number `{number_text}` is too large"))?;
    if number == 0 {
        return Err("stage numbers must be greater than zero".to_owned());
    }
    let name = if name.is_empty() {
        folder_name.to_owned()
    } else {
        name.to_owned()
    };
    Ok((number, name))
}

fn executable_file(directory: &Path, name: &str) -> Result<Option<PathBuf>, String> {
    let path = directory.join(name);
    match fs::metadata(&path) {
        Ok(metadata) if metadata.is_file() => Ok(Some(path)),
        Ok(_) => Err(format!("{} is not a regular file", path.display())),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!("cannot inspect {}: {error}", path.display())),
    }
}

pub struct SqliteWorkbenchStateStore {
    connection: Connection,
}

impl SqliteWorkbenchStateStore {
    pub fn open(workspace_root: &Path) -> Result<Self, String> {
        let state_directory = workspace_root.join(".control_tower");
        fs::create_dir_all(&state_directory)
            .map_err(|error| format!("cannot create {}: {error}", state_directory.display()))?;
        let database_path = state_directory.join("state.sqlite3");
        let connection = Connection::open(&database_path)
            .map_err(|error| format!("cannot open {}: {error}", database_path.display()))?;
        connection
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS workbench_state (
                    id INTEGER PRIMARY KEY CHECK (id = 1),
                    completed_stage_count INTEGER NOT NULL,
                    uuid TEXT,
                    pending_stage_index INTEGER,
                    pending_direction TEXT
                );",
            )
            .map_err(|error| format!("cannot initialize {}: {error}", database_path.display()))?;
        Ok(Self { connection })
    }
}

impl WorkbenchStateStore for SqliteWorkbenchStateStore {
    fn load(&self) -> Result<Option<WorkbenchState>, String> {
        let row = self
            .connection
            .query_row(
                "SELECT completed_stage_count, uuid, pending_stage_index, pending_direction
                 FROM workbench_state WHERE id = 1",
                [],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, Option<String>>(1)?,
                        row.get::<_, Option<i64>>(2)?,
                        row.get::<_, Option<String>>(3)?,
                    ))
                },
            )
            .optional()
            .map_err(|error| error.to_string())?;
        let Some((completed_count, uuid, pending_index, pending_direction)) = row else {
            return Ok(None);
        };
        if completed_count < 0 {
            return Err("completed stage count cannot be negative".to_owned());
        }
        let pending = match (pending_index, pending_direction) {
            (None, None) => None,
            (Some(index), Some(direction)) if index >= 0 => {
                let direction = match direction.as_str() {
                    "up" => Direction::Up,
                    "down" => Direction::Down,
                    other => return Err(format!("unknown pending direction `{other}`")),
                };
                Some(control_tower_application::PendingTransition {
                    stage_index: index as usize,
                    direction,
                })
            }
            _ => return Err("incomplete pending transition fields".to_owned()),
        };
        Ok(Some(WorkbenchState {
            completed_stage_count: completed_count as usize,
            uuid,
            pending,
        }))
    }

    fn save(&self, state: &WorkbenchState) -> Result<(), String> {
        let completed_count = i64::try_from(state.completed_stage_count)
            .map_err(|_| "completed stage count is too large".to_owned())?;
        let (pending_index, pending_direction) = match state.pending {
            Some(pending) => (
                Some(
                    i64::try_from(pending.stage_index)
                        .map_err(|_| "pending stage is too large".to_owned())?,
                ),
                Some(pending.direction.as_str()),
            ),
            None => (None, None),
        };
        self.connection
            .execute(
                "INSERT INTO workbench_state
                    (id, completed_stage_count, uuid, pending_stage_index, pending_direction)
                 VALUES (1, ?1, ?2, ?3, ?4)
                 ON CONFLICT(id) DO UPDATE SET
                    completed_stage_count = excluded.completed_stage_count,
                    uuid = excluded.uuid,
                    pending_stage_index = excluded.pending_stage_index,
                    pending_direction = excluded.pending_direction",
                params![
                    completed_count,
                    state.uuid,
                    pending_index,
                    pending_direction
                ],
            )
            .map_err(|error| error.to_string())?;
        Ok(())
    }
}

pub struct SystemExecutableRunner;

impl ExecutableRunner for SystemExecutableRunner {
    fn run(&self, invocation: &Invocation) -> Result<ProcessOutput, String> {
        let output = Command::new(&invocation.executable)
            .current_dir(&invocation.working_directory)
            .env("CONTROL_TOWER_WORKSPACE", &invocation.workspace_root)
            .env("CONTROL_TOWER_UUID", &invocation.uuid)
            .env("CONTROL_TOWER_STAGE", invocation.stage_number.to_string())
            .env("CONTROL_TOWER_DIRECTION", invocation.direction.as_str())
            .env("CONTROL_TOWER_ROLE", invocation.role.as_str())
            .output()
            .map_err(|error| format!("{}: {error}", invocation.executable.display()))?;
        Ok(ProcessOutput {
            success: output.status.success(),
            exit_code: output.status.code(),
            stdout: output.stdout,
            stderr: output.stderr,
        })
    }
}

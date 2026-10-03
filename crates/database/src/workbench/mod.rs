use control_tower_application::{
    Direction, PersistenceError, WorkbenchQueries, WorkbenchState, WorkbenchWrites,
};
use rusqlite::{Connection, OpenFlags, OptionalExtension, params};
use std::path::Path;

pub struct SqliteWorkbenchQueries {
    connection: Connection,
}
pub struct SqliteWorkbenchWrites {
    connection: Connection,
}

impl SqliteWorkbenchQueries {
    pub fn open(database_path: &Path) -> Result<Self, PersistenceError> {
        let connection =
            Connection::open_with_flags(database_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
                .map_err(PersistenceError::new)?;
        super::operations::verify_schema(&connection)?;
        Ok(Self { connection })
    }
}
impl SqliteWorkbenchWrites {
    pub fn open(database_path: &Path) -> Result<Self, PersistenceError> {
        let connection =
            Connection::open_with_flags(database_path, OpenFlags::SQLITE_OPEN_READ_WRITE)
                .map_err(PersistenceError::new)?;
        super::operations::verify_schema(&connection)?;
        Ok(Self { connection })
    }
}
impl WorkbenchQueries for SqliteWorkbenchQueries {
    fn read_checkpoint(&self) -> Result<Option<WorkbenchState>, PersistenceError> {
        let row = self
            .connection
            .query_row(include_str!("sql/read_checkpoint.sql"), [], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, Option<i64>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                ))
            })
            .optional()
            .map_err(PersistenceError::new)?;
        let Some((completed_count, uuid, pending_index, pending_direction)) = row else {
            return Ok(None);
        };
        if completed_count < 0 {
            return Err(PersistenceError::message(
                "completed stage count cannot be negative",
            ));
        }
        let pending = match (pending_index, pending_direction) {
            (None, None) => None,
            (Some(index), Some(direction)) if index >= 0 => {
                let direction = match direction.as_str() {
                    "up" => Direction::Up,
                    "down" => Direction::Down,
                    other => {
                        return Err(PersistenceError::message(format!(
                            "unknown pending direction `{other}`"
                        )));
                    }
                };
                Some(control_tower_application::PendingTransition {
                    stage_index: index as usize,
                    direction,
                })
            }
            _ => {
                return Err(PersistenceError::message(
                    "incomplete pending transition fields",
                ));
            }
        };
        Ok(Some(WorkbenchState {
            completed_stage_count: completed_count as usize,
            uuid,
            pending,
        }))
    }
}

impl WorkbenchWrites for SqliteWorkbenchWrites {
    fn record_checkpoint(&self, state: &WorkbenchState) -> Result<(), PersistenceError> {
        let completed_count =
            i64::try_from(state.completed_stage_count).map_err(PersistenceError::new)?;
        let (pending_index, pending_direction) = match state.pending {
            Some(pending) => (
                Some(i64::try_from(pending.stage_index).map_err(PersistenceError::new)?),
                Some(pending.direction.as_str()),
            ),
            None => (None, None),
        };
        self.connection
            .execute(
                include_str!("sql/record_checkpoint.sql"),
                params![
                    completed_count,
                    state.uuid,
                    pending_index,
                    pending_direction
                ],
            )
            .map_err(PersistenceError::new)?;
        Ok(())
    }
}

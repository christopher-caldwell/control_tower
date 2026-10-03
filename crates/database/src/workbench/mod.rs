use control_tower_application::{
    Direction, PersistenceError, WorkbenchQueries, WorkbenchState, WorkbenchWrites,
};
use rusqlite::{Connection, OpenFlags, OptionalExtension, params};
use std::{
    path::Path,
    process,
    sync::{Mutex, MutexGuard},
};

pub struct SqliteWorkbenchQueries {
    connection: Mutex<Connection>,
}
pub struct SqliteWorkbenchWrites {
    connection: Mutex<Connection>,
}

impl SqliteWorkbenchQueries {
    pub fn open(database_path: &Path) -> Result<Self, PersistenceError> {
        let connection =
            Connection::open_with_flags(database_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
                .map_err(PersistenceError::new)?;
        super::operations::verify_schema(&connection)?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }
}
impl SqliteWorkbenchWrites {
    pub fn open(database_path: &Path) -> Result<Self, PersistenceError> {
        let connection =
            Connection::open_with_flags(database_path, OpenFlags::SQLITE_OPEN_READ_WRITE)
                .map_err(PersistenceError::new)?;
        super::operations::verify_schema(&connection)?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }
}
impl WorkbenchQueries for SqliteWorkbenchQueries {
    fn read_checkpoint(&self) -> Result<Option<WorkbenchState>, PersistenceError> {
        let row = {
            let connection = lock_connection(&self.connection, "query");
            connection
                .query_row(include_str!("sql/read_checkpoint.sql"), [], |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, Option<String>>(1)?,
                        row.get::<_, Option<i64>>(2)?,
                        row.get::<_, Option<String>>(3)?,
                    ))
                })
                .optional()
                .map_err(PersistenceError::new)?
        };
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
        let connection = lock_connection(&self.connection, "write");
        connection
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

fn lock_connection<'a>(
    connection: &'a Mutex<Connection>,
    lane: &str,
) -> MutexGuard<'a, Connection> {
    match connection.lock() {
        Ok(guard) => guard,
        Err(_) => {
            eprintln!(
                "Control Tower SQLite {lane} connection is poisoned; exiting because checkpoint state may be inconsistent."
            );
            process::exit(1);
        }
    }
}

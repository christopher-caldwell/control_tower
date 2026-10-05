//! Explicit local database operations. Ordinary CLI startup never calls these.
use std::path::Path;

use control_tower_application::PersistenceError;
use rusqlite::{Connection, OpenFlags};

/// Provision the local SQLite file; do not create application tables.
pub fn bootstrap(database_path: &Path) -> Result<(), PersistenceError> {
    if let Some(parent) = database_path.parent() {
        std::fs::create_dir_all(parent).map_err(PersistenceError::new)?;
    }
    Connection::open(database_path).map_err(PersistenceError::new)?;
    Ok(())
}

/// Apply versioned schema changes after explicit bootstrap, including adoption
/// of the pre-migration v0 table without changing its rows or representation.
pub fn migrate(database_path: &Path) -> Result<(), PersistenceError> {
    let mut connection =
        Connection::open_with_flags(database_path, OpenFlags::SQLITE_OPEN_READ_WRITE)
            .map_err(PersistenceError::new)?;
    let version: i64 = connection
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(PersistenceError::new)?;
    match version {
        0 => {
            let transaction = connection.transaction().map_err(PersistenceError::new)?;
            transaction
                .execute_batch(include_str!("../migrations/0001-workbench-state.sql"))
                .map_err(PersistenceError::new)?;
            transaction.commit().map_err(PersistenceError::new)?;
        }
        1 => {}
        _ => {
            return Err(PersistenceError::message(
                "unsupported SQLite schema version",
            ));
        }
    }
    verify_schema(&connection)
}

/// Inspect supported history without mutating schema or creating a file.
pub fn verify(database_path: &Path) -> Result<(), PersistenceError> {
    let connection = Connection::open_with_flags(database_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(PersistenceError::new)?;
    verify_schema(&connection)
}

pub(crate) fn verify_schema(connection: &Connection) -> Result<(), PersistenceError> {
    let version: i64 = connection
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(PersistenceError::new)?;
    if version != 1 {
        return Err(PersistenceError::message(
            "SQLite schema is not initialized; bootstrap and migrate the workflow database first",
        ));
    }
    let name: String = connection
        .query_row(
            "SELECT name FROM control_tower_migrations WHERE version = 1",
            [],
            |row| row.get(0),
        )
        .map_err(PersistenceError::new)?;
    if name != "workbench-state" {
        return Err(PersistenceError::message(
            "unsupported SQLite migration history",
        ));
    }
    Ok(())
}

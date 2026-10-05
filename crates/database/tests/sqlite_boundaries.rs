use std::error::Error;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use control_tower_application::{
    Direction, PendingTransition, WorkbenchQueries, WorkbenchState, WorkbenchWrites,
};
use control_tower_database::{
    operations,
    workbench::{SqliteWorkbenchQueries, SqliteWorkbenchWrites},
};
use rusqlite::{Connection, OpenFlags};

static NEXT: AtomicU64 = AtomicU64::new(0);
struct Database(PathBuf);
impl Database {
    fn new() -> Self {
        Self(
            std::env::temp_dir()
                .join(format!(
                    "control-tower-sqlite-test-{}-{}",
                    std::process::id(),
                    NEXT.fetch_add(1, Ordering::Relaxed)
                ))
                .join("state.sqlite3"),
        )
    }
    fn setup(&self) {
        operations::bootstrap(&self.0).unwrap();
        operations::migrate(&self.0).unwrap();
        operations::verify(&self.0).unwrap();
    }
}
impl Drop for Database {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(self.0.parent().unwrap());
    }
}

#[test]
fn explicit_setup_and_checkpoint_round_trip_use_real_sqlite() {
    let database = Database::new();
    assert!(operations::migrate(&database.0).is_err());
    assert!(!database.0.exists());
    operations::bootstrap(&database.0).unwrap();
    assert!(SqliteWorkbenchQueries::open(&database.0).is_err());
    operations::migrate(&database.0).unwrap();
    operations::migrate(&database.0).unwrap();
    operations::verify(&database.0).unwrap();
    let queries = SqliteWorkbenchQueries::open(&database.0).unwrap();
    let writes = SqliteWorkbenchWrites::open(&database.0).unwrap();
    assert_eq!(queries.read_checkpoint().unwrap(), None);
    // Either adjacent last-completed position is valid for either pending
    // direction: normal verification and verification after reversal.
    for completed_stage_count in [2, 3] {
        for direction in [Direction::Up, Direction::Down] {
            let checkpoint = WorkbenchState {
                completed_stage_count,
                uuid: Some("opaque-uuid".into()),
                pending: Some(PendingTransition {
                    stage_index: 2,
                    direction,
                }),
            };
            writes.record_checkpoint(&checkpoint).unwrap();
            assert_eq!(queries.read_checkpoint().unwrap(), Some(checkpoint));
        }
    }
    writes
        .record_checkpoint(&WorkbenchState::default())
        .unwrap();
    assert_eq!(
        queries.read_checkpoint().unwrap(),
        Some(WorkbenchState::default())
    );
    let connection =
        Connection::open_with_flags(&database.0, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    assert!(
        connection
            .execute("DELETE FROM workbench_state", [])
            .is_err()
    );
    println!("SQLite version: {}", rusqlite::version());
}

#[test]
fn driver_failures_preserve_their_original_sources() {
    let database = Database::new();
    database.setup();
    let queries = SqliteWorkbenchQueries::open(&database.0).unwrap();
    let writes = SqliteWorkbenchWrites::open(&database.0).unwrap();
    Connection::open(&database.0)
        .unwrap()
        .execute("DROP TABLE workbench_state", [])
        .unwrap();
    let read_error = queries.read_checkpoint().unwrap_err();
    let write_error = writes
        .record_checkpoint(&WorkbenchState::default())
        .unwrap_err();
    // Downcasts are test-only assertions about diagnostics, never Application behavior.
    for error in [read_error, write_error] {
        assert!(error.source().unwrap().is::<rusqlite::Error>());
    }
}

#[test]
fn legacy_table_adoption_keeps_existing_checkpoint() {
    let database = Database::new();
    operations::bootstrap(&database.0).unwrap();
    let connection = Connection::open(&database.0).unwrap();
    connection.execute_batch("CREATE TABLE workbench_state (id INTEGER PRIMARY KEY CHECK (id = 1), completed_stage_count INTEGER NOT NULL, uuid TEXT, pending_stage_index INTEGER, pending_direction TEXT); INSERT INTO workbench_state VALUES (1, 1, 'legacy-uuid', 1, 'up');").unwrap();
    operations::migrate(&database.0).unwrap();
    let checkpoint = SqliteWorkbenchQueries::open(&database.0)
        .unwrap()
        .read_checkpoint()
        .unwrap()
        .unwrap();
    assert_eq!(checkpoint.completed_stage_count, 1);
    assert_eq!(checkpoint.uuid.as_deref(), Some("legacy-uuid"));
    assert_eq!(
        checkpoint.pending,
        Some(PendingTransition {
            stage_index: 1,
            direction: Direction::Up
        })
    );
    let history: i64 = connection
        .query_row("SELECT COUNT(*) FROM control_tower_migrations", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(history, 1);
}

#[test]
fn failed_migration_rolls_back_ddl_and_version() {
    let database = Database::new();
    operations::bootstrap(&database.0).unwrap();
    let connection = Connection::open(&database.0).unwrap();
    connection
        .execute(
            "CREATE TABLE control_tower_migrations (wrong_column TEXT)",
            [],
        )
        .unwrap();
    assert!(operations::migrate(&database.0).is_err());
    let version: i64 = connection
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .unwrap();
    assert_eq!(version, 0);
    assert!(connection.prepare("SELECT * FROM workbench_state").is_err());
}

use control_tower_application::{PersistenceError, Workbench};
use control_tower_database::workbench::{SqliteWorkbenchQueries, SqliteWorkbenchWrites};
use control_tower_infrastructure::{
    executable_runner::SystemExecutableRunner,
    stage_definition_reader::FilesystemStageDefinitionReader,
    stage_discovery::FilesystemStageDiscovery,
};
use std::{collections::HashMap, path::Path};

pub(super) fn workbench(
    workflow: &Path,
    env_overrides: HashMap<String, String>,
) -> Result<Workbench, PersistenceError> {
    let database = workflow.join(".control_tower/state.sqlite3");
    let queries = SqliteWorkbenchQueries::open(&database)?;
    let writes = SqliteWorkbenchWrites::open(&database)?;
    Ok(Workbench::new(
        Box::new(FilesystemStageDiscovery),
        Box::new(queries),
        Box::new(writes),
        Box::new(SystemExecutableRunner::with_env_overrides(env_overrides)),
        Box::new(FilesystemStageDefinitionReader),
    ))
}

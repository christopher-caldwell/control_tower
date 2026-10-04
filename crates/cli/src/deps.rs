use control_tower_application::{PersistenceError, Workbench};
use control_tower_database::workbench::{SqliteWorkbenchQueries, SqliteWorkbenchWrites};
use control_tower_infrastructure::{
    executable_runner::SystemExecutableRunner,
    stage_definition_reader::FilesystemStageDefinitionReader,
    stage_discovery::FilesystemStageDiscovery,
};
use std::path::Path;

pub(super) fn workbench(workspace: &Path) -> Result<Workbench, PersistenceError> {
    let database = workspace.join(".control_tower/state.sqlite3");
    let queries = SqliteWorkbenchQueries::open(&database)?;
    let writes = SqliteWorkbenchWrites::open(&database)?;
    Ok(Workbench::new(
        Box::new(FilesystemStageDiscovery),
        Box::new(queries),
        Box::new(writes),
        Box::new(SystemExecutableRunner),
        Box::new(FilesystemStageDefinitionReader),
    ))
}

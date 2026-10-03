use control_tower_application::{StageDefinitionReadError, StageDefinitionReader};
use std::{fs, path::Path};

pub struct FilesystemStageDefinitionReader;

impl StageDefinitionReader for FilesystemStageDefinitionReader {
    fn read(&self, path: &Path) -> Result<Vec<u8>, StageDefinitionReadError> {
        fs::read(path).map_err(StageDefinitionReadError::new)
    }
}

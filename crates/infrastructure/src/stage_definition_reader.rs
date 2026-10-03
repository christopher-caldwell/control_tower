use control_tower_application::{
    StageDefinitionContents, StageDefinitionReadError, StageDefinitionReader,
};
use std::{fs, io::Read, path::Path};

pub struct FilesystemStageDefinitionReader;

impl StageDefinitionReader for FilesystemStageDefinitionReader {
    fn read(
        &self,
        path: &Path,
        maximum_bytes: usize,
    ) -> Result<StageDefinitionContents, StageDefinitionReadError> {
        let file = fs::File::open(path).map_err(StageDefinitionReadError::new)?;
        let mut bytes = Vec::new();
        file.take(maximum_bytes.saturating_add(1) as u64)
            .read_to_end(&mut bytes)
            .map_err(StageDefinitionReadError::new)?;
        let truncated = bytes.len() > maximum_bytes;
        bytes.truncate(maximum_bytes);
        Ok(StageDefinitionContents { bytes, truncated })
    }
}

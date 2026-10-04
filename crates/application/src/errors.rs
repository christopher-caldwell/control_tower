use std::error::Error;

// Each capability owns an opaque error, with diagnostic sources kept intact.
macro_rules! opaque_error {
    ($name:ident) => {
        #[derive(Debug, thiserror::Error)]
        #[error("{source}")]
        pub struct $name {
            #[source]
            source: Box<dyn Error + Send + Sync>,
        }
        impl $name {
            pub fn new(source: impl Error + Send + Sync + 'static) -> Self {
                Self {
                    source: Box::new(source),
                }
            }
            pub fn message(message: impl Into<String>) -> Self {
                Self::new(ContractMessage(message.into()))
            }
        }
    };
}

#[derive(Debug, thiserror::Error)]
#[error("{0}")]
struct ContractMessage(String);

opaque_error!(PersistenceError);
opaque_error!(StageDiscoveryError);
opaque_error!(ExecutableRunError);

#[derive(Debug, thiserror::Error)]
pub enum StatusError {
    #[error("stage discovery failed: {0}")]
    StageDiscovery(#[from] StageDiscoveryError),
    #[error("workbench state failed: {0}")]
    Persistence(#[from] PersistenceError),
    #[error("invalid stored workbench state: {0}")]
    InvalidState(String),
}

#[derive(Debug, thiserror::Error)]
pub enum MoveToError {
    #[error("stage discovery failed: {0}")]
    StageDiscovery(#[from] StageDiscoveryError),
    #[error("workbench state failed: {0}")]
    Persistence(#[from] PersistenceError),
    #[error("invalid stored workbench state: {0}")]
    InvalidState(String),
    #[error("invalid target: {0}")]
    InvalidTarget(String),
    #[error("the workflow checkpoint changed; refresh before submitting this movement")]
    StaleCheckpoint,
}

impl From<StatusError> for MoveToError {
    fn from(error: StatusError) -> Self {
        match error {
            StatusError::StageDiscovery(error) => Self::StageDiscovery(error),
            StatusError::Persistence(error) => Self::Persistence(error),
            StatusError::InvalidState(message) => Self::InvalidState(message),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum StageDefinitionsError {
    #[error("stage discovery failed: {0}")]
    StageDiscovery(#[from] StageDiscoveryError),
    #[error("stage {0} was not found in this Workflow")]
    UnknownStage(u32),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn use_case_errors_keep_the_capability_source_chain() {
        let status = StatusError::from(PersistenceError::new(std::io::Error::other("read failed")));
        let movement = MoveToError::from(status);
        let capability = movement.source().unwrap();
        assert!(capability.is::<PersistenceError>());
        assert!(capability.source().unwrap().is::<std::io::Error>());

        let status = StatusError::from(StageDiscoveryError::new(std::io::Error::other(
            "discovery failed",
        )));
        assert!(
            status
                .source()
                .unwrap()
                .source()
                .unwrap()
                .is::<std::io::Error>()
        );
    }
}

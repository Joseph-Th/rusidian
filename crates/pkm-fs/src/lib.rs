pub mod state;
pub mod repositories;
pub mod retriever;
pub mod blob_store;

pub use state::{load_vault, SharedVault, VaultState, IngestionItem};
pub use repositories::{
    FsNoteRepo, FsSourceRepo, FsEntityRepo, FsLinkRepo, FsViewRepo, FsAgentActionRepo,
};
pub use retriever::FsRetriever;
pub use blob_store::BlobStore;

use thiserror::Error;

pub type Result<T> = std::result::Result<T, FsError>;

#[derive(Debug, Error)]
pub enum FsError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),

    #[error("serde: {0}")]
    Serde(#[from] serde_json::Error),

    #[error(transparent)]
    Core(#[from] pkm_core::CoreError),
}

impl From<FsError> for pkm_core::CoreError {
    fn from(e: FsError) -> Self {
        match e {
            FsError::Core(c) => c,
            other => pkm_core::CoreError::Invariant(other.to_string()),
        }
    }
}

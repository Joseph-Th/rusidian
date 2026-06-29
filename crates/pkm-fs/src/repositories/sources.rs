use pkm_core::ports::SourceRepo;
use pkm_core::source::Source;
use pkm_core::id::SourceId;
use pkm_core::Result;
use std::path::PathBuf;
use crate::state::SharedVault;

pub struct FsSourceRepo {
    pub state: SharedVault,
    pub vault_path: PathBuf,
}

impl FsSourceRepo {
    /// Update the ingestion state of a source in-memory and mark dirty.
    /// Does not rewrite the source file on disk (state-only update).
    pub fn update_ingestion_state(&self, id: SourceId, ingestion_state: pkm_core::ingestion::IngestionState) -> Result<()> {
        let mut state = self.state.write().unwrap();
        if let Some(source) = state.sources.get_mut(&id) {
            source.ingestion_state = ingestion_state;
            source.updated_at = pkm_core::Timestamp::now_utc();
            state.mark_dirty();
            Ok(())
        } else {
            Err(pkm_core::CoreError::NotFound(format!("Source not found: {}", id)))
        }
    }
}

impl SourceRepo for FsSourceRepo {
    fn create(&self, source: &Source) -> Result<()> {
        {
            let mut state = self.state.write().unwrap();
            state.sources.insert(source.id, source.clone());
            state.mark_dirty();
        }
        let file_name = format!("{}.md", source.id);
        let file_path = self.vault_path.join("sources").join(file_name);
        std::fs::write(&file_path, &source.raw_content)
            .map_err(|e| pkm_core::CoreError::Invariant(e.to_string()))?;
        Ok(())
    }

    fn get(&self, id: SourceId) -> Result<Option<Source>> {
        let state = self.state.read().unwrap();
        Ok(state.sources.get(&id).cloned())
    }

    fn list(&self, limit: Option<usize>) -> Result<Vec<Source>> {
        let state = self.state.read().unwrap();
        let mut sources: Vec<Source> = state.sources.values().cloned().collect();
        sources.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        if let Some(l) = limit {
            sources.truncate(l);
        }
        Ok(sources)
    }
}

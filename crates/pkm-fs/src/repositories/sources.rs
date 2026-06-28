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

impl SourceRepo for FsSourceRepo {
    fn create(&self, source: &Source) -> Result<()> {
        let mut state = self.state.write().unwrap();
        state.sources.insert(source.id, source.clone());

        // Save raw content to sources/source-<id>.txt
        let file_name = format!("source-{}.txt", source.id);
        let file_path = self.vault_path.join("sources").join(file_name);
        std::fs::write(&file_path, &source.raw_content)
            .map_err(|e| pkm_core::CoreError::Invariant(e.to_string()))?;

        // Save metadata to .pkm/sources.json
        let sources_path = self.vault_path.join(".pkm").join("sources.json");
        let sources_json = serde_json::to_string_pretty(&state.sources)
            .map_err(|e| pkm_core::CoreError::Invariant(e.to_string()))?;
        std::fs::write(sources_path, sources_json)
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

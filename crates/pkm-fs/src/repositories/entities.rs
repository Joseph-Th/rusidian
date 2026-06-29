use pkm_core::ports::EntityRepo;
use pkm_core::entity::Entity;
use pkm_core::id::EntityId;
use pkm_core::Result;
use std::path::PathBuf;
use crate::state::{SharedVault, persist_metadata};

pub struct FsEntityRepo {
    pub state: SharedVault,
    pub vault_path: PathBuf,
}

impl EntityRepo for FsEntityRepo {
    fn create(&self, entity: &Entity) -> Result<()> {
        let save_data = {
            let mut state = self.state.write().unwrap();
            state.entities.insert(entity.id, entity.clone());
            state.extract_save_data()
        };
        persist_metadata(&self.vault_path, &save_data)
            .map_err(|e| pkm_core::CoreError::Invariant(e.to_string()))?;
        Ok(())
    }

    fn get(&self, id: EntityId) -> Result<Option<Entity>> {
        let state = self.state.read().unwrap();
        Ok(state.entities.get(&id).cloned())
    }

    fn set_merged_into(&self, loser_id: EntityId, survivor_id: EntityId) -> Result<()> {
        let save_data = {
            let mut state = self.state.write().unwrap();
            if let Some(loser) = state.entities.get_mut(&loser_id) {
                loser.merged_into = Some(survivor_id);
                loser.updated_at = pkm_core::Timestamp::now_utc();
                loser.version += 1;
            }
            state.extract_save_data()
        };
        persist_metadata(&self.vault_path, &save_data)
            .map_err(|e| pkm_core::CoreError::Invariant(e.to_string()))?;
        Ok(())
    }

    fn clear_merged_into(&self, entity_id: EntityId) -> Result<()> {
        let save_data = {
            let mut state = self.state.write().unwrap();
            if let Some(entity) = state.entities.get_mut(&entity_id) {
                entity.merged_into = None;
                entity.updated_at = pkm_core::Timestamp::now_utc();
                entity.version += 1;
            }
            state.extract_save_data()
        };
        persist_metadata(&self.vault_path, &save_data)
            .map_err(|e| pkm_core::CoreError::Invariant(e.to_string()))?;
        Ok(())
    }
}

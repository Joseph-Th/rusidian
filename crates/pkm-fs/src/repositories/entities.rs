use pkm_core::ports::EntityRepo;
use pkm_core::entity::Entity;
use pkm_core::id::EntityId;
use pkm_core::Result;
use std::path::PathBuf;
use crate::state::SharedVault;

pub struct FsEntityRepo {
    pub state: SharedVault,
    pub vault_path: PathBuf,
}

impl FsEntityRepo {
    fn save_entities(&self, state: &crate::state::VaultState) -> Result<()> {
        let entities_path = self.vault_path.join(".pkm").join("entities.json");
        let entities_json = serde_json::to_string_pretty(&state.entities)
            .map_err(|e| pkm_core::CoreError::Invariant(e.to_string()))?;
        std::fs::write(entities_path, entities_json)
            .map_err(|e| pkm_core::CoreError::Invariant(e.to_string()))?;
        Ok(())
    }
}

impl EntityRepo for FsEntityRepo {
    fn create(&self, entity: &Entity) -> Result<()> {
        {
            let mut state = self.state.write().unwrap();
            state.entities.insert(entity.id, entity.clone());
        }
        let state = self.state.read().unwrap();
        self.save_entities(&state)?;
        Ok(())
    }

    fn get(&self, id: EntityId) -> Result<Option<Entity>> {
        let state = self.state.read().unwrap();
        Ok(state.entities.get(&id).cloned())
    }

    fn set_merged_into(&self, loser_id: EntityId, survivor_id: EntityId) -> Result<()> {
        {
            let mut state = self.state.write().unwrap();
            if let Some(loser) = state.entities.get_mut(&loser_id) {
                loser.merged_into = Some(survivor_id);
                loser.updated_at = pkm_core::Timestamp::now_utc();
                loser.version += 1;
            }
        }
        let state = self.state.read().unwrap();
        self.save_entities(&state)?;
        Ok(())
    }

    fn clear_merged_into(&self, entity_id: EntityId) -> Result<()> {
        {
            let mut state = self.state.write().unwrap();
            if let Some(entity) = state.entities.get_mut(&entity_id) {
                entity.merged_into = None;
                entity.updated_at = pkm_core::Timestamp::now_utc();
                entity.version += 1;
            }
        }
        let state = self.state.read().unwrap();
        self.save_entities(&state)?;
        Ok(())
    }
}

use pkm_core::entity::{Entity, LinkBackup};
use pkm_core::ports::EntityRepo;
use pkm_core::id::{EntityId, ObjectRef};
use pkm_core::Result;
use std::path::PathBuf;
use crate::state::SharedVault;

pub struct FsEntityRepo {
    pub state: SharedVault,
    pub vault_path: PathBuf,
}

impl EntityRepo for FsEntityRepo {
    fn create(&self, entity: &Entity) -> Result<()> {
        let mut state = self.state.write().unwrap();
        state.entities.insert(entity.id, entity.clone());
        state.mark_dirty();
        Ok(())
    }

    fn get(&self, id: EntityId) -> Result<Option<Entity>> {
        let state = self.state.read().unwrap();
        Ok(state.entities.get(&id).cloned())
    }

    fn set_merged_into(&self, loser_id: EntityId, survivor_id: EntityId) -> Result<()> {
        let mut state = self.state.write().unwrap();
        if let Some(loser) = state.entities.get_mut(&loser_id) {
            loser.merged_into = Some(survivor_id);
            loser.updated_at = pkm_core::Timestamp::now_utc();
            loser.version += 1;
        }

        // Re-point all links that reference the loser entity to the survivor
        let loser_ref = ObjectRef::Entity(loser_id);
        let survivor_ref = ObjectRef::Entity(survivor_id);
        let links_to_update: Vec<pkm_core::link::Link> = state
            .links
            .values()
            .filter(|l| l.from == loser_ref || l.to == loser_ref)
            .cloned()
            .collect();

        // Store backups before mutating, enabling rollback
        let backups: Vec<LinkBackup> = links_to_update
            .iter()
            .map(|l| LinkBackup {
                link_id: l.id,
                original_from: l.from,
                original_to: l.to,
            })
            .collect();

        if let Some(loser) = state.entities.get_mut(&loser_id) {
            loser.merged_link_backups = backups;
        }

        for mut link in links_to_update {
            let changed = link.from == loser_ref || link.to == loser_ref;
            if link.from == loser_ref {
                link.from = survivor_ref;
            }
            if link.to == loser_ref {
                link.to = survivor_ref;
            }
            if changed {
                link.version += 1;
                link.updated_at = pkm_core::Timestamp::now_utc();
                let link_id = link.id;
                // Remove old index entries using the pre-modification link stored in state
                if let Some(old_link) = state.links.get(&link_id).cloned() {
                    state.index_link_remove(&old_link);
                }
                // Clone the updated link so we can pass it to index_link_add
                // without conflicting with the mutable insert borrow
                state.links.insert(link_id, link.clone());
                state.index_link_add(&link);
            }
        }

        state.mark_dirty();
        Ok(())
    }

    fn clear_merged_into(&self, entity_id: EntityId) -> Result<()> {
        // Take backups from the entity (dropping entity borrow) so we can
        // mutably borrow state for both entity and link updates.
        let backups = {
            let mut state = self.state.write().unwrap();
            if let Some(entity) = state.entities.get_mut(&entity_id) {
                std::mem::take(&mut entity.merged_link_backups)
            } else {
                return Ok(());
            }
        };

        // Restore links from backups in a separate scope
        {
            let mut state = self.state.write().unwrap();
            for backup in &backups {
                if let Some(old) = state.links.get(&backup.link_id).cloned() {
                    let mut updated = old.clone();
                    state.index_link_remove(&old);
                    updated.from = backup.original_from;
                    updated.to = backup.original_to;
                    updated.version += 1;
                    updated.updated_at = pkm_core::Timestamp::now_utc();
                    state.links.insert(backup.link_id, updated.clone());
                    state.index_link_add(&updated);
                }
            }
        }

        // Clear the merged_into flag
        {
            let mut state = self.state.write().unwrap();
            if let Some(entity) = state.entities.get_mut(&entity_id) {
                entity.merged_into = None;
                entity.updated_at = pkm_core::Timestamp::now_utc();
                entity.version += 1;
            }
            state.mark_dirty();
        }
        Ok(())
    }
}

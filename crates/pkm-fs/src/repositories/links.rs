use pkm_core::ports::LinkRepo;
use pkm_core::link::Link;
use pkm_core::id::{LinkId, ObjectRef};
use pkm_core::Result;
use std::path::PathBuf;
use crate::state::SharedVault;

pub struct FsLinkRepo {
    pub state: SharedVault,
    pub vault_path: PathBuf,
}

impl FsLinkRepo {
    fn save_links(&self, state: &crate::state::VaultState) -> Result<()> {
        let links_path = self.vault_path.join(".pkm").join("links.json");
        let links_json = serde_json::to_string_pretty(&state.links)
            .map_err(|e| pkm_core::CoreError::Invariant(e.to_string()))?;
        std::fs::write(links_path, links_json)
            .map_err(|e| pkm_core::CoreError::Invariant(e.to_string()))?;
        Ok(())
    }
}

impl LinkRepo for FsLinkRepo {
    fn create(&self, link: &Link) -> Result<()> {
        let mut state = self.state.write().unwrap();
        state.links.insert(link.id, link.clone());
        state.rebuild_indexes();
        self.save_links(&state)?;
        Ok(())
    }

    fn get(&self, link_id: LinkId) -> Result<Option<Link>> {
        let state = self.state.read().unwrap();
        Ok(state.links.get(&link_id).cloned())
    }

    fn get_by_to(&self, target: ObjectRef) -> Result<Vec<Link>> {
        let state = self.state.read().unwrap();
        let target_key = format!("{:?}", target);
        if let Some(link_ids) = state.links_by_target.get(&target_key) {
            let res: Vec<Link> = link_ids.iter()
                .filter_map(|id| state.links.get(id).cloned())
                .collect();
            Ok(res)
        } else {
            Ok(vec![])
        }
    }

    fn get_by_from(&self, source: ObjectRef) -> Result<Vec<Link>> {
        let state = self.state.read().unwrap();
        let source_key = format!("{:?}", source);
        if let Some(link_ids) = state.links_by_source.get(&source_key) {
            let res: Vec<Link> = link_ids.iter()
                .filter_map(|id| state.links.get(id).cloned())
                .collect();
            Ok(res)
        } else {
            Ok(vec![])
        }
    }

    fn set_to(&self, link_id: LinkId, new_to: ObjectRef) -> Result<()> {
        let mut state = self.state.write().unwrap();
        if let Some(link) = state.links.get_mut(&link_id) {
            link.to = new_to;
            link.updated_at = pkm_core::Timestamp::now_utc();
            link.version += 1;
        }
        state.rebuild_indexes();
        self.save_links(&state)?;
        Ok(())
    }

    fn set_from(&self, link_id: LinkId, new_from: ObjectRef) -> Result<()> {
        let mut state = self.state.write().unwrap();
        if let Some(link) = state.links.get_mut(&link_id) {
            link.from = new_from;
            link.updated_at = pkm_core::Timestamp::now_utc();
            link.version += 1;
        }
        state.rebuild_indexes();
        self.save_links(&state)?;
        Ok(())
    }

    fn delete(&self, link_id: LinkId) -> Result<()> {
        let mut state = self.state.write().unwrap();
        state.links.remove(&link_id);
        state.rebuild_indexes();
        self.save_links(&state)?;
        Ok(())
    }
}

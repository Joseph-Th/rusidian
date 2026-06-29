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

impl LinkRepo for FsLinkRepo {
    fn create(&self, link: &Link) -> Result<()> {
        let mut state = self.state.write().unwrap();
        state.links.insert(link.id, link.clone());
        state.index_link_add(link);
        state.mark_dirty();
        Ok(())
    }

    fn get(&self, id: LinkId) -> Result<Option<Link>> {
        let state = self.state.read().unwrap();
        Ok(state.links.get(&id).cloned())
    }

    fn get_by_to(&self, target: ObjectRef) -> Result<Vec<Link>> {
        let state = self.state.read().unwrap();
        Ok(state
            .links
            .values()
            .filter(|l| l.to == target)
            .cloned()
            .collect())
    }

    fn get_by_from(&self, source: ObjectRef) -> Result<Vec<Link>> {
        let state = self.state.read().unwrap();
        Ok(state
            .links
            .values()
            .filter(|l| l.from == source)
            .cloned()
            .collect())
    }

    fn set_to(&self, link_id: LinkId, new_to: ObjectRef) -> Result<()> {
        let mut state = self.state.write().unwrap();
        if let Some(l) = state.links.get_mut(&link_id) {
            let old = l.clone();
            l.to = new_to;
            l.version += 1;
            l.updated_at = pkm_core::Timestamp::now_utc();
            let updated = l.clone();
            state.index_link_remove(&old);
            state.index_link_add(&updated);
            state.mark_dirty();
        }
        Ok(())
    }

    fn set_from(&self, link_id: LinkId, new_from: ObjectRef) -> Result<()> {
        let mut state = self.state.write().unwrap();
        if let Some(l) = state.links.get_mut(&link_id) {
            let old = l.clone();
            l.from = new_from;
            l.version += 1;
            l.updated_at = pkm_core::Timestamp::now_utc();
            let updated = l.clone();
            state.index_link_remove(&old);
            state.index_link_add(&updated);
            state.mark_dirty();
        }
        Ok(())
    }

    fn delete(&self, link_id: LinkId) -> Result<()> {
        let mut state = self.state.write().unwrap();
        let link = state.links.remove(&link_id);
        if let Some(ref link) = link {
            state.index_link_remove(link);
        }
        state.mark_dirty();
        Ok(())
    }
}

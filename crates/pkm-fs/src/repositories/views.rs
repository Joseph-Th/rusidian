use pkm_core::ports::ViewRepo;
use pkm_core::view::View;
use pkm_core::id::ViewId;
use pkm_core::Result;
use std::path::PathBuf;
use crate::state::{SharedVault, persist_metadata};

pub struct FsViewRepo {
    pub state: SharedVault,
    pub vault_path: PathBuf,
}

impl ViewRepo for FsViewRepo {
    fn create(&self, view: &View) -> Result<()> {
        let save_data = {
            let mut state = self.state.write().unwrap();
            state.views.insert(view.id, view.clone());
            state.extract_save_data()
        };
        persist_metadata(&self.vault_path, &save_data)
            .map_err(|e| pkm_core::CoreError::Invariant(e.to_string()))?;
        Ok(())
    }

    fn get(&self, id: ViewId) -> Result<Option<View>> {
        let state = self.state.read().unwrap();
        Ok(state.views.get(&id).cloned())
    }

    fn list(&self, limit: Option<usize>) -> Result<Vec<View>> {
        let state = self.state.read().unwrap();
        let mut views: Vec<View> = state.views.values().cloned().collect();
        views.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        if let Some(l) = limit {
            views.truncate(l);
        }
        Ok(views)
    }
}

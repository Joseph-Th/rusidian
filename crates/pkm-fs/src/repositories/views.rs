use pkm_core::ports::ViewRepo;
use pkm_core::view::View;
use pkm_core::id::ViewId;
use pkm_core::Result;
use std::path::PathBuf;
use crate::state::SharedVault;

pub struct FsViewRepo {
    pub state: SharedVault,
    pub vault_path: PathBuf,
}

impl FsViewRepo {
    fn save_views(&self, state: &crate::state::VaultState) -> Result<()> {
        let views_path = self.vault_path.join(".pkm").join("views.json");
        let views_json = serde_json::to_string_pretty(&state.views)
            .map_err(|e| pkm_core::CoreError::Invariant(e.to_string()))?;
        std::fs::write(views_path, views_json)
            .map_err(|e| pkm_core::CoreError::Invariant(e.to_string()))?;
        Ok(())
    }
}

impl ViewRepo for FsViewRepo {
    fn create(&self, view: &View) -> Result<()> {
        let mut state = self.state.write().unwrap();
        state.views.insert(view.id, view.clone());
        self.save_views(&state)?;
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

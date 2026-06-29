use pkm_core::ports::AgentActionRepo;
use pkm_core::agent_action::{ActionDiff, AgentAction, AgentActionStatus};
use pkm_core::id::AgentActionId;
use pkm_core::Result;
use std::path::PathBuf;
use crate::state::SharedVault;

pub struct FsAgentActionRepo {
    pub state: SharedVault,
    pub vault_path: PathBuf,
}

impl AgentActionRepo for FsAgentActionRepo {
    fn create(&self, action: &AgentAction) -> Result<()> {
        let vault_path = self.vault_path.clone();
        let mut state = self.state.write().unwrap();
        state.actions.push(action.clone());
        let _ = state.save_metadata(&vault_path);
        Ok(())
    }

    fn get(&self, id: AgentActionId) -> Result<Option<AgentAction>> {
        let state = self.state.read().unwrap();
        let action = state.actions.iter().find(|a| a.id == id).cloned();
        Ok(action)
    }

    fn set_status(&self, id: AgentActionId, new_status: AgentActionStatus) -> Result<()> {
        let vault_path = self.vault_path.clone();
        {
            let mut state = self.state.write().unwrap();
            if let Some(action) = state.actions.iter_mut().find(|a| a.id == id) {
                action.status = new_status;
            }
            let _ = state.save_metadata(&vault_path);
        }
        Ok(())
    }

    fn set_diff(&self, id: AgentActionId, diff: ActionDiff) -> Result<()> {
        let vault_path = self.vault_path.clone();
        {
            let mut state = self.state.write().unwrap();
            if let Some(action) = state.actions.iter_mut().find(|a| a.id == id) {
                action.diff = diff.clone();
            }
            let _ = state.save_metadata(&vault_path);
        }
        Ok(())
    }
}

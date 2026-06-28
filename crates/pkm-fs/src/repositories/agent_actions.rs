use pkm_core::ports::AgentActionRepo;
use pkm_core::agent_action::{AgentAction, AgentActionStatus};
use pkm_core::id::AgentActionId;
use pkm_core::Result;
use std::path::PathBuf;
use crate::state::SharedVault;

pub struct FsAgentActionRepo {
    pub state: SharedVault,
    pub vault_path: PathBuf,
}

impl FsAgentActionRepo {
    fn save_actions(&self, state: &crate::state::VaultState) -> Result<()> {
        let actions_path = self.vault_path.join(".pkm").join("actions.json");
        let actions_json = serde_json::to_string_pretty(&state.actions)
            .map_err(|e| pkm_core::CoreError::Invariant(e.to_string()))?;
        std::fs::write(actions_path, actions_json)
            .map_err(|e| pkm_core::CoreError::Invariant(e.to_string()))?;
        Ok(())
    }
}

impl AgentActionRepo for FsAgentActionRepo {
    fn create(&self, action: &AgentAction) -> Result<()> {
        {
            let mut state = self.state.write().unwrap();
            state.actions.push(action.clone());
        }
        let state = self.state.read().unwrap();
        self.save_actions(&state)?;
        Ok(())
    }

    fn get(&self, id: AgentActionId) -> Result<Option<AgentAction>> {
        let state = self.state.read().unwrap();
        let action = state.actions.iter().find(|a| a.id == id).cloned();
        Ok(action)
    }

    fn set_status(&self, id: AgentActionId, new_status: AgentActionStatus) -> Result<()> {
        {
            let mut state = self.state.write().unwrap();
            if let Some(action) = state.actions.iter_mut().find(|a| a.id == id) {
                action.status = new_status;
            }
        }
        let state = self.state.read().unwrap();
        self.save_actions(&state)?;
        Ok(())
    }

    fn set_diff(&self, id: AgentActionId, diff: serde_json::Value) -> Result<()> {
        {
            let mut state = self.state.write().unwrap();
            if let Some(action) = state.actions.iter_mut().find(|a| a.id == id) {
                action.diff = diff;
            }
        }
        let state = self.state.read().unwrap();
        self.save_actions(&state)?;
        Ok(())
    }
}

use pkm_core::ports::AgentActionRepo;
use pkm_core::agent_action::{AgentAction, AgentActionStatus};
use pkm_core::id::AgentActionId;
use pkm_core::Result;
use std::path::PathBuf;
use std::fs::OpenOptions;
use std::io::Write;
use crate::state::SharedVault;

pub struct FsAgentActionRepo {
    pub state: SharedVault,
    pub vault_path: PathBuf,
}

impl FsAgentActionRepo {
    fn rewrite_actions_file(&self, actions: &[AgentAction]) -> Result<()> {
        let actions_path = self.vault_path.join(".pkm").join("actions.jsonl");
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&actions_path)
            .map_err(|e| pkm_core::CoreError::Invariant(e.to_string()))?;

        for action in actions {
            let line = serde_json::to_string(action)
                .map_err(|e| pkm_core::CoreError::Invariant(e.to_string()))?;
            writeln!(file, "{}", line)
                .map_err(|e| pkm_core::CoreError::Invariant(e.to_string()))?;
        }
        Ok(())
    }
}

impl AgentActionRepo for FsAgentActionRepo {
    fn create(&self, action: &AgentAction) -> Result<()> {
        let mut state = self.state.write().unwrap();
        state.actions.push(action.clone());

        // Append line to .pkm/actions.jsonl
        let actions_path = self.vault_path.join(".pkm").join("actions.jsonl");
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&actions_path)
            .map_err(|e| pkm_core::CoreError::Invariant(e.to_string()))?;

        let line = serde_json::to_string(action)
            .map_err(|e| pkm_core::CoreError::Invariant(e.to_string()))?;
        writeln!(file, "{}", line)
            .map_err(|e| pkm_core::CoreError::Invariant(e.to_string()))?;

        Ok(())
    }

    fn get(&self, id: AgentActionId) -> Result<Option<AgentAction>> {
        let state = self.state.read().unwrap();
        let action = state.actions.iter().find(|a| a.id == id).cloned();
        Ok(action)
    }

    fn set_status(&self, id: AgentActionId, new_status: AgentActionStatus) -> Result<()> {
        let mut state = self.state.write().unwrap();
        if let Some(action) = state.actions.iter_mut().find(|a| a.id == id) {
            action.status = new_status;
        }
        let actions_clone = state.actions.clone();
        drop(state);
        self.rewrite_actions_file(&actions_clone)?;
        Ok(())
    }

    fn set_diff(&self, id: AgentActionId, diff: serde_json::Value) -> Result<()> {
        let mut state = self.state.write().unwrap();
        if let Some(action) = state.actions.iter_mut().find(|a| a.id == id) {
            action.diff = diff;
        }
        let actions_clone = state.actions.clone();
        drop(state);
        self.rewrite_actions_file(&actions_clone)?;
        Ok(())
    }
}

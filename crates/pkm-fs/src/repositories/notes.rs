use pkm_core::ports::NoteRepo;
use pkm_core::note::Note;
use pkm_core::block::{Block, BlockContent};
use pkm_core::id::{NoteId, BlockId};
use pkm_core::Result;
use std::path::PathBuf;
use crate::state::{SharedVault, VaultState};

pub struct FsNoteRepo {
    pub state: SharedVault,
    pub vault_path: PathBuf,
}

impl FsNoteRepo {
    fn save_note_to_disk(&self, note: &Note, state: &VaultState) -> Result<()> {
        let mut blocks: Vec<Block> = state.blocks.values()
            .filter(|b| b.note_id == note.id)
            .cloned()
            .collect();
        blocks.sort_by_key(|b| note.blocks.iter().position(|&id| id == b.id).unwrap_or(usize::MAX));
        let markdown_text = pkm_core::markdown::note_to_markdown(note, &blocks);
        let file_path = self.vault_path.join("notes").join(note.file_name());
        std::fs::write(&file_path, markdown_text)
            .map_err(|e| pkm_core::CoreError::Invariant(e.to_string()))?;
        Ok(())
    }
}

impl NoteRepo for FsNoteRepo {
    fn create(&self, note: &Note) -> Result<()> {
        let mut state = self.state.write().unwrap();
        state.notes.insert(note.id, note.clone());
        self.save_note_to_disk(note, &state)?;
        Ok(())
    }

    fn get(&self, id: NoteId) -> Result<Option<Note>> {
        let state = self.state.read().unwrap();
        Ok(state.notes.get(&id).cloned())
    }

    fn list(&self, limit: Option<usize>) -> Result<Vec<Note>> {
        let state = self.state.read().unwrap();
        let mut notes: Vec<Note> = state.notes.values().cloned().collect();
        notes.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        if let Some(l) = limit {
            notes.truncate(l);
        }
        Ok(notes)
    }

    fn update(&self, note: &Note) -> Result<()> {
        let mut state = self.state.write().unwrap();
        let old_note = state.notes.get(&note.id).cloned();
        state.notes.insert(note.id, note.clone());
        self.save_note_to_disk(note, &state)?;
        if let Some(old) = old_note {
            if old.file_name() != note.file_name() {
                let old_path = self.vault_path.join("notes").join(old.file_name());
                let _ = std::fs::remove_file(old_path);
            }
        }
        Ok(())
    }

    fn delete(&self, id: NoteId) -> Result<()> {
        let mut state = self.state.write().unwrap();
        if let Some(note) = state.notes.remove(&id) {
            let file_path = self.vault_path.join("notes").join(note.file_name());
            let _ = std::fs::remove_file(file_path);
            
            // Delete blocks associated with note
            state.blocks.retain(|_, b| b.note_id != id);
            
            // Also clean up links originating from/to this note or its blocks
            state.links.retain(|_, link| {
                let from_match = match link.from {
                    pkm_core::id::ObjectRef::Note(nid) => nid == id,
                    pkm_core::id::ObjectRef::Block(bid) => note.blocks.contains(&bid),
                    _ => false,
                };
                let to_match = match link.to {
                    pkm_core::id::ObjectRef::Note(nid) => nid == id,
                    pkm_core::id::ObjectRef::Block(bid) => note.blocks.contains(&bid),
                    _ => false,
                };
                !from_match && !to_match
            });
            state.rebuild_indexes();
            
            // Save links metadata
            let links_path = self.vault_path.join(".pkm").join("links.json");
            if let Ok(links_json) = serde_json::to_string_pretty(&state.links) {
                let _ = std::fs::write(links_path, links_json);
            }
        }
        Ok(())
    }

    fn update_block(
        &self,
        note_id: NoteId,
        block_id: BlockId,
        new_content: BlockContent,
    ) -> Result<Block> {
        let mut state = self.state.write().unwrap();
        let block = state.blocks.get_mut(&block_id).ok_or_else(|| {
            pkm_core::CoreError::Invariant(format!("Block not found: {}", block_id))
        })?;
        block.content = new_content;
        block.version += 1;
        block.updated_at = pkm_core::Timestamp::now_utc();
        let updated_block = block.clone();
        
        if let Some(note) = state.notes.get(&note_id) {
            self.save_note_to_disk(note, &state)?;
        }
        Ok(updated_block)
    }

    fn get_blocks(&self, note_id: NoteId) -> Result<Vec<Block>> {
        let state = self.state.read().unwrap();
        let mut blocks: Vec<Block> = state.blocks.values()
            .filter(|b| b.note_id == note_id)
            .cloned()
            .collect();
        if let Some(note) = state.notes.get(&note_id) {
            blocks.sort_by_key(|b| note.blocks.iter().position(|&id| id == b.id).unwrap_or(usize::MAX));
        }
        Ok(blocks)
    }

    fn get_note_id_for_block(&self, block_id: BlockId) -> Result<Option<NoteId>> {
        let state = self.state.read().unwrap();
        Ok(state.blocks.get(&block_id).map(|b| b.note_id))
    }

    fn create_block(&self, block: &Block) -> Result<()> {
        let mut state = self.state.write().unwrap();
        state.blocks.insert(block.id, block.clone());
        let note_id = block.note_id;

        if let Some(note) = state.notes.get_mut(&note_id) {
            if !note.blocks.contains(&block.id) {
                note.blocks.push(block.id);
            }
            let note_clone = note.clone();
            self.save_note_to_disk(&note_clone, &state)?;
        }
        Ok(())
    }

    fn delete_block(&self, note_id: NoteId, block_id: BlockId) -> Result<()> {
        let mut state = self.state.write().unwrap();
        state.blocks.remove(&block_id);
        if let Some(note) = state.notes.get_mut(&note_id) {
            note.blocks.retain(|id| *id != block_id);
            let note_clone = note.clone();
            self.save_note_to_disk(&note_clone, &state)?;
        }
        Ok(())
    }
}

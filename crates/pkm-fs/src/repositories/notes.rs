use pkm_core::ports::NoteRepo;
use pkm_core::note::Note;
use pkm_core::block::{Block, BlockContent};
use pkm_core::id::{NoteId, BlockId};
use pkm_core::id::ObjectRef;
use pkm_core::Result;
use std::path::PathBuf;
use crate::state::SharedVault;

pub struct FsNoteRepo {
    pub state: SharedVault,
    pub vault_path: PathBuf,
}

impl FsNoteRepo {
    fn save_note_to_disk(&self, note: &Note, state: &crate::state::VaultState) -> Result<()> {
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
        {
            let mut state = self.state.write().unwrap();
            state.notes.insert(note.id, note.clone());
        }
        let state = self.state.read().unwrap();
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
        let (old_file_name, needs_cleanup) = {
            let mut state = self.state.write().unwrap();
            let old_note = state.notes.get(&note.id).cloned();
            state.notes.insert(note.id, note.clone());
            let old_fn = old_note.as_ref().map(|n| n.file_name());
            let cleanup = old_fn.is_some() && old_fn.as_deref() != Some(&note.file_name());
            (old_fn, cleanup)
        };
        let state = self.state.read().unwrap();
        self.save_note_to_disk(note, &state)?;
        if needs_cleanup {
            if let Some(old_name) = old_file_name {
                let old_path = self.vault_path.join("notes").join(old_name);
                let _ = std::fs::remove_file(old_path);
            }
        }
        Ok(())
    }

    fn delete(&self, id: NoteId) -> Result<()> {
        let file_path = {
            let mut state = self.state.write().unwrap();
            if let Some(note) = state.notes.remove(&id) {
                let fp = self.vault_path.join("notes").join(note.file_name());
                state.blocks.retain(|_, b| b.note_id != id);
                state.links.retain(|_, link| {
                    let from_match = match link.from {
                        ObjectRef::Note(nid) => nid == id,
                        ObjectRef::Block(bid) => note.blocks.contains(&bid),
                        _ => false,
                    };
                    let to_match = match link.to {
                        ObjectRef::Note(nid) => nid == id,
                        ObjectRef::Block(bid) => note.blocks.contains(&bid),
                        _ => false,
                    };
                    !from_match && !to_match
                });
                state.rebuild_indexes();
                fp
            } else {
                return Ok(());
            }
        };
        let _ = std::fs::remove_file(file_path);
        Ok(())
    }

    fn update_block(
        &self,
        note_id: NoteId,
        block_id: BlockId,
        new_content: BlockContent,
    ) -> Result<Block> {
        let updated_block = {
            let mut state = self.state.write().unwrap();
            let block = state.blocks.get_mut(&block_id).ok_or_else(|| {
                pkm_core::CoreError::Invariant(format!("Block not found: {}", block_id))
            })?;
            block.content = new_content.clone();
            block.version += 1;
            block.updated_at = pkm_core::Timestamp::now_utc();
            let updated = block.clone();
            updated
        };
        let state = self.state.read().unwrap();
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
        let note_id = block.note_id;
        {
            let mut state = self.state.write().unwrap();
            state.blocks.insert(block.id, block.clone());
            let note = state.notes.get_mut(&note_id);
            if let Some(n) = note {
                if !n.blocks.contains(&block.id) {
                    n.blocks.push(block.id);
                }
            } else {
                return Err(pkm_core::CoreError::Invariant(format!("Note not found: {}", note_id)));
            }
        }
        let state = self.state.read().unwrap();
        if let Some(note) = state.notes.get(&note_id) {
            self.save_note_to_disk(note, &state)?;
        }
        Ok(())
    }

    fn delete_block(&self, note_id: NoteId, block_id: BlockId) -> Result<()> {
        {
            let mut state = self.state.write().unwrap();
            state.blocks.remove(&block_id);
            let note = state.notes.get_mut(&note_id);
            if let Some(n) = note {
                n.blocks.retain(|id| *id != block_id);
            } else {
                return Err(pkm_core::CoreError::Invariant(format!("Note not found: {}", note_id)));
            }
        }
        let state = self.state.read().unwrap();
        if let Some(note) = state.notes.get(&note_id) {
            self.save_note_to_disk(note, &state)?;
        }
        Ok(())
    }

    fn upsert_from_external(&self, note: &Note, blocks: &[Block]) -> Result<()> {
        use std::collections::HashSet;

        // Phase 1: all in-memory mutation under a single write lock
        {
            let mut state = self.state.write().unwrap();

            // Collect IDs of blocks currently owned by this note
            let old_block_ids: Vec<BlockId> = state
                .blocks
                .values()
                .filter(|b| b.note_id == note.id)
                .map(|b| b.id)
                .collect();

            let new_block_ids: HashSet<BlockId> = blocks.iter().map(|b| b.id).collect();

            // Remove links that pointed to blocks that are no longer present
            state.links.retain(|_, link| {
                let from_removed = match link.from {
                    ObjectRef::Block(bid) => old_block_ids.contains(&bid) && !new_block_ids.contains(&bid),
                    _ => false,
                };
                let to_removed = match link.to {
                    ObjectRef::Block(bid) => old_block_ids.contains(&bid) && !new_block_ids.contains(&bid),
                    _ => false,
                };
                !from_removed && !to_removed
            });

            // Remove old blocks
            for bid in &old_block_ids {
                state.blocks.remove(bid);
            }

            // Insert new blocks
            for block in blocks {
                state.blocks.insert(block.id, block.clone());
            }

            // Update note
            state.notes.insert(note.id, note.clone());

            // Rebuild link indexes
            state.rebuild_indexes();
        } // write lock dropped

        // Phase 2: persist to disk
        let state = self.state.read().unwrap();
        let _ = state.save_metadata(&self.vault_path);

        let file_path = self.vault_path.join("notes").join(note.file_name());
        std::fs::create_dir_all(file_path.parent().unwrap())
            .map_err(|e| pkm_core::CoreError::Invariant(e.to_string()))?;
        let markdown_text = pkm_core::markdown::note_to_markdown(note, blocks);
        std::fs::write(&file_path, markdown_text)
            .map_err(|e| pkm_core::CoreError::Invariant(e.to_string()))?;

        Ok(())
    }
}

use pkm_core::ports::NoteRepo;
use pkm_core::note::Note;
use pkm_core::block::{Block, BlockContent};
use pkm_core::id::{NoteId, BlockId};
use pkm_core::id::ObjectRef;
use pkm_core::link::Link;
use pkm_core::Result;
use std::path::PathBuf;
use crate::state::{SharedVault, persist_metadata};

pub struct FsNoteRepo {
    pub state: SharedVault,
    pub vault_path: PathBuf,
}

impl FsNoteRepo {
    fn generate_markdown(&self, note: &Note, state: &crate::state::VaultState) -> String {
        let mut blocks: Vec<Block> = state.blocks.values()
            .filter(|b| b.note_id == note.id)
            .cloned()
            .collect();
        blocks.sort_by_key(|b| note.blocks.iter().position(|&id| id == b.id).unwrap_or(usize::MAX));
        pkm_core::markdown::note_to_markdown(note, &blocks)
    }

    pub fn get_by_filename(&self, filename: &str) -> Option<Note> {
        let state = self.state.read().unwrap();
        state.notes.values().find(|n| n.file_name() == filename).cloned()
    }

    fn write_note_file(&self, note: &Note, markdown_text: &str) -> Result<()> {
        let file_path = self.vault_path.join("notes").join(note.file_name());
        std::fs::write(&file_path, markdown_text)
            .map_err(|e| pkm_core::CoreError::Invariant(e.to_string()))?;
        Ok(())
    }
}

impl NoteRepo for FsNoteRepo {
    fn create(&self, note: &Note) -> Result<()> {
        let markdown_text = {
            let mut state = self.state.write().unwrap();
            state.notes.insert(note.id, note.clone());
            self.generate_markdown(note, &state)
        };
        self.write_note_file(note, &markdown_text)?;
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
        let (old_file_name, needs_cleanup, markdown_text) = {
            let mut state = self.state.write().unwrap();
            let old_note = state.notes.get(&note.id).cloned();
            state.notes.insert(note.id, note.clone());
            let old_fn = old_note.as_ref().map(|n| n.file_name());
            let cleanup = old_fn.is_some() && old_fn.as_deref() != Some(&note.file_name());
            let md = self.generate_markdown(note, &state);
            (old_fn, cleanup, md)
        };
        self.write_note_file(note, &markdown_text)?;
        if needs_cleanup {
            if let Some(old_name) = old_file_name {
                let old_path = self.vault_path.join("notes").join(old_name);
                let _ = std::fs::remove_file(old_path);
            }
        }
        Ok(())
    }

    fn delete(&self, id: NoteId) -> Result<()> {
        let result = {
            let mut state = self.state.write().unwrap();
            if let Some(note) = state.notes.remove(&id) {
                let fp = self.vault_path.join("notes").join(note.file_name());
                state.blocks.retain(|_, b| b.note_id != id);

                // Collect links to remove before removing them
                let links_to_remove: Vec<Link> = state.links.values()
                    .filter(|link| {
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
                        from_match || to_match
                    })
                    .cloned()
                    .collect();

                // Remove links from map
                for link in &links_to_remove {
                    state.links.remove(&link.id);
                }

                // Remove from indexes individually (O(m) instead of O(n))
                for link in &links_to_remove {
                    state.index_link_remove(link);
                }

                let save_data = state.extract_save_data();
                Some((fp, save_data))
            } else {
                return Ok(());
            }
        };

        if let Some((fp, save_data)) = result {
            let _ = std::fs::remove_file(fp);
            let _ = persist_metadata(&self.vault_path, &save_data);
        }
        Ok(())
    }

    fn update_block(
        &self,
        note_id: NoteId,
        block_id: BlockId,
        new_content: BlockContent,
    ) -> Result<Block> {
        let (updated_block, markdown_text, note) = {
            let mut state = self.state.write().unwrap();
            let block = state.blocks.get_mut(&block_id).ok_or_else(|| {
                pkm_core::CoreError::Invariant(format!("Block not found: {}", block_id))
            })?;
            block.content = new_content.clone();
            block.version += 1;
            block.updated_at = pkm_core::Timestamp::now_utc();
            let updated = block.clone();
            let note_clone = state.notes.get(&note_id).cloned();
            let md = note_clone.as_ref().map(|n| self.generate_markdown(n, &state));
            (updated, md, note_clone)
        };
        if let (Some(n), Some(md)) = (note, markdown_text) {
            self.write_note_file(&n, &md)?;
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
        let (markdown_text, note) = {
            let mut state = self.state.write().unwrap();
            state.blocks.insert(block.id, block.clone());
            let note = state.notes.get_mut(&note_id);
            if let Some(n) = note {
                if !n.blocks.contains(&block.id) {
                    n.blocks.push(block.id);
                }
                let note_clone = n.clone();
                let md = self.generate_markdown(&note_clone, &state);
                (Some(md), note_clone)
            } else {
                return Err(pkm_core::CoreError::Invariant(format!("Note not found: {}", note_id)));
            }
        };
        if let Some(md) = markdown_text {
            self.write_note_file(&note, &md)?;
        }
        Ok(())
    }

    fn delete_block(&self, note_id: NoteId, block_id: BlockId) -> Result<()> {
        let (markdown_text, note) = {
            let mut state = self.state.write().unwrap();
            state.blocks.remove(&block_id);
            let note = state.notes.get_mut(&note_id);
            if let Some(n) = note {
                n.blocks.retain(|id| *id != block_id);
                let note_clone = n.clone();
                let md = self.generate_markdown(&note_clone, &state);
                (Some(md), note_clone)
            } else {
                return Err(pkm_core::CoreError::Invariant(format!("Note not found: {}", note_id)));
            }
        };
        if let Some(md) = markdown_text {
            self.write_note_file(&note, &md)?;
        }
        Ok(())
    }

    fn upsert_from_external(&self, note: &Note, blocks: &[Block]) -> Result<()> {
        use std::collections::HashSet;

        // Phase 1: all in-memory mutation under a single write lock
        let markdown_text = {
            let mut state = self.state.write().unwrap();

            // Collect IDs of blocks currently owned by this note
            let old_block_ids: Vec<BlockId> = state
                .blocks
                .values()
                .filter(|b| b.note_id == note.id)
                .map(|b| b.id)
                .collect();

            let new_block_ids: HashSet<BlockId> = blocks.iter().map(|b| b.id).collect();

            // Collect links to remove that pointed to removed blocks
            let links_to_remove: Vec<Link> = state.links.values()
                .filter(|link| {
                    let from_removed = match link.from {
                        ObjectRef::Block(bid) => old_block_ids.contains(&bid) && !new_block_ids.contains(&bid),
                        _ => false,
                    };
                    let to_removed = match link.to {
                        ObjectRef::Block(bid) => old_block_ids.contains(&bid) && !new_block_ids.contains(&bid),
                        _ => false,
                    };
                    from_removed || to_removed
                })
                .cloned()
                .collect();

            // Remove from indexes individually
            for link in &links_to_remove {
                state.index_link_remove(link);
                state.links.remove(&link.id);
            }

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

            pkm_core::markdown::note_to_markdown(note, blocks)
        };

        // Phase 2: persist to disk (no locks held)
        let save_data = {
            let state = self.state.read().unwrap();
            state.extract_save_data()
        };
        let _ = persist_metadata(&self.vault_path, &save_data);

        let file_path = self.vault_path.join("notes").join(note.file_name());
        std::fs::create_dir_all(file_path.parent().unwrap())
            .map_err(|e| pkm_core::CoreError::Invariant(e.to_string()))?;
        std::fs::write(&file_path, markdown_text)
            .map_err(|e| pkm_core::CoreError::Invariant(e.to_string()))?;

        Ok(())
    }
}

use pkm_core::ports::NoteRepo;
use pkm_core::note::Note;
use pkm_core::block::{Block, BlockContent};
use pkm_core::id::{NoteId, BlockId};
use pkm_core::id::ObjectRef;
use pkm_core::link::Link;
use pkm_core::Result;
use std::path::{Path, PathBuf};
use crate::state::SharedVault;

pub struct FsNoteRepo {
    pub state: SharedVault,
    pub vault_path: PathBuf,
}

impl FsNoteRepo {
    fn generate_markdown(&self, note: &Note, state: &crate::state::VaultState) -> String {
        let blocks: Vec<Block> = note.blocks.iter()
            .filter_map(|id| state.blocks.get(id))
            .cloned()
            .collect();
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
        let mut state = self.state.write().unwrap();
        state.notes.insert(note.id, note.clone());
        let md = self.generate_markdown(note, &state);
        self.write_note_file(note, &md)?;
        state.mark_dirty();
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
        let needs_cleanup = old_note.as_ref().map(|n| n.file_name()) != Some(note.file_name());
        let md = self.generate_markdown(note, &state);
        self.write_note_file(note, &md)?;
        state.mark_dirty();
        if needs_cleanup {
            if let Some(old_name) = old_note.map(|n| n.file_name()) {
                let old_path = self.vault_path.join("notes").join(old_name);
                let _ = std::fs::remove_file(old_path);
            }
        }
        Ok(())
    }

    fn delete(&self, id: NoteId) -> Result<()> {
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

            state.mark_dirty();
            let _ = std::fs::remove_file(fp);
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
        block.content = new_content.clone();
        block.version += 1;
        block.updated_at = pkm_core::Timestamp::now_utc();
        let updated = block.clone();
        if let Some(note) = state.notes.get(&note_id) {
            let md = self.generate_markdown(note, &state);
            self.write_note_file(note, &md)?;
            state.mark_dirty();
        }
        Ok(updated)
    }

    fn get_blocks(&self, note_id: NoteId) -> Result<Vec<Block>> {
        let state = self.state.read().unwrap();
        let blocks: Vec<Block> = state.notes.get(&note_id)
            .map(|note| {
                note.blocks.iter()
                    .filter_map(|id| state.blocks.get(id))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();
        Ok(blocks)
    }

    fn get_note_id_for_block(&self, block_id: BlockId) -> Result<Option<NoteId>> {
        let state = self.state.read().unwrap();
        Ok(state.blocks.get(&block_id).map(|b| b.note_id))
    }

    fn create_block(&self, block: &Block) -> Result<()> {
        let note_id = block.note_id;
        let mut state = self.state.write().unwrap();
        state.blocks.insert(block.id, block.clone());
        if let Some(note) = state.notes.get_mut(&note_id) {
            if !note.blocks.contains(&block.id) {
                note.blocks.push(block.id);
            }
            let note_clone = note.clone();
            let md = self.generate_markdown(&note_clone, &state);
            self.write_note_file(&note_clone, &md)?;
            state.mark_dirty();
            Ok(())
        } else {
            Err(pkm_core::CoreError::Invariant(format!("Note not found: {}", note_id)))
        }
    }

    fn delete_block(&self, note_id: NoteId, block_id: BlockId) -> Result<()> {
        let mut state = self.state.write().unwrap();

        // Remove dangling links targeting or originating from this block
        let block_ref = ObjectRef::Block(block_id);
        let links_to_remove: Vec<Link> = state.links.values()
            .filter(|l| l.from == block_ref || l.to == block_ref)
            .cloned()
            .collect();
        for link in &links_to_remove {
            state.index_link_remove(link);
            state.links.remove(&link.id);
        }
        if !links_to_remove.is_empty() {
            state.mark_dirty();
        }

        state.blocks.remove(&block_id);
        if let Some(note) = state.notes.get_mut(&note_id) {
            note.blocks.retain(|id| *id != block_id);
            let note_clone = note.clone();
            let md = self.generate_markdown(&note_clone, &state);
            self.write_note_file(&note_clone, &md)?;
            state.mark_dirty();
            Ok(())
        } else {
            Err(pkm_core::CoreError::Invariant(format!("Note not found: {}", note_id)))
        }
    }

    fn upsert_from_external(&self, note: &Note, blocks: &[Block], external_file_path: &Path) -> Result<()> {
        use std::collections::HashSet;

        // Hold the write lock across both memory mutation AND disk I/O
        // to prevent write-after-write races between concurrent threads.
        let mut state = self.state.write().unwrap();

        let old_file_name = state.notes.get(&note.id).map(|n| n.file_name());

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

        // Generate markdown and persist to disk under the same lock
        let markdown_text = pkm_core::markdown::note_to_markdown(note, blocks);

        let file_path = self.vault_path.join("notes").join(note.file_name());
        std::fs::create_dir_all(file_path.parent().unwrap())
            .map_err(|e| pkm_core::CoreError::Invariant(e.to_string()))?;
        std::fs::write(&file_path, markdown_text)
            .map_err(|e| pkm_core::CoreError::Invariant(e.to_string()))?;

        state.mark_dirty();

        // Clean up old canonical file if the note was renamed (title changed)
        if let Some(old_name) = old_file_name {
            if old_name != note.file_name() {
                let old_path = self.vault_path.join("notes").join(old_name);
                let _ = std::fs::remove_file(old_path);
            }
        }

        // Always clean up the external file that triggered this update if it
        // differs from the canonical file path. This handles the case where a
        // user renames a file externally (e.g., my-note-123.md -> new-name.md)
        // but the internal title hasn't changed — without this, the external
        // file would duplicate on every save.
        if external_file_path != file_path && external_file_path.exists() {
            let _ = std::fs::remove_file(external_file_path);
        }

        Ok(())
    }
}

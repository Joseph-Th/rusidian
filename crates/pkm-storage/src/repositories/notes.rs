//! SQLite implementation of [`pkm_core::ports::NoteRepo`].

use rusqlite::Connection;

use pkm_core::id::NoteId;
use pkm_core::note::Note;
use pkm_core::ports::NoteRepo;
use pkm_core::Result;

/// Note persistence backed by SQLite. STUB — task B2.
pub struct SqliteNoteRepo<'c> {
    pub conn: &'c Connection,
}

impl NoteRepo for SqliteNoteRepo<'_> {
    fn create(&self, _note: &Note) -> Result<()> {
        // TODO(B2): INSERT note + its blocks (ordered). Transactional.
        unimplemented!("SqliteNoteRepo::create — STATUS.md task B2")
    }

    fn get(&self, _id: NoteId) -> Result<Option<Note>> {
        // TODO(B2): SELECT note + ordered block ids -> Note.
        unimplemented!("SqliteNoteRepo::get — STATUS.md task B2")
    }
}

//! SQLite implementation of [`pkm_core::ports::SourceRepo`].

use rusqlite::Connection;

use pkm_core::id::SourceId;
use pkm_core::ports::SourceRepo;
use pkm_core::source::Source;
use pkm_core::Result;

/// Source persistence backed by SQLite. STUB — task B2.
pub struct SqliteSourceRepo<'c> {
    pub conn: &'c Connection,
}

impl SourceRepo for SqliteSourceRepo<'_> {
    fn create(&self, _source: &Source) -> Result<()> {
        // TODO(B2): INSERT into `source`. Raw content is write-once; reject an
        //           overwrite that is not an explicit audited user edit.
        unimplemented!("SqliteSourceRepo::create — STATUS.md task B2")
    }

    fn get(&self, _id: SourceId) -> Result<Option<Source>> {
        // TODO(B2): SELECT + map row -> Source (pure mapping fn, unit-tested).
        unimplemented!("SqliteSourceRepo::get — STATUS.md task B2")
    }
}

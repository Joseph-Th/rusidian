//! Connection open + pragmas.

use std::path::Path;

use rusqlite::Connection;

use crate::{migrations, Result};

/// Open (and migrate) the database at `path`. STUB — task B1.
pub fn open(_path: &Path) -> Result<Connection> {
    // TODO(B1): Connection::open(path); set PRAGMA journal_mode=WAL,
    //           foreign_keys=ON, busy_timeout; then migrations::run(&conn).
    let _ = migrations::run;
    unimplemented!("open db — STATUS.md task B1")
}

pub mod identity;
pub mod items;
pub mod migrations;
pub mod retention;
pub mod shelf;

use std::{path::Path, time::Duration};

use rusqlite::Connection;

use crate::error::Result;

/// Opens (or creates) the database with Panopin's pragmas and brings the schema up to date.
pub fn open(path: &Path) -> Result<Connection> {
    let mut conn = Connection::open(path)?;
    // WAL keeps readers off the writer's back and stays consistent after a crash.
    let mode: String =
        conn.pragma_update_and_check(None, "journal_mode", "WAL", |row| row.get(0))?;
    if !mode.eq_ignore_ascii_case("wal") {
        log::warn!("SQLite journal mode is {mode}, expected WAL");
    }
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    conn.pragma_update(None, "foreign_keys", true)?;
    conn.busy_timeout(Duration::from_secs(5))?;
    migrations::apply(&mut conn)?;
    Ok(conn)
}

#[cfg(test)]
pub(crate) fn open_in_memory() -> Result<Connection> {
    let mut conn = Connection::open_in_memory()?;
    conn.pragma_update(None, "foreign_keys", true)?;
    migrations::apply(&mut conn)?;
    Ok(conn)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_database_uses_wal_and_latest_schema() {
        let dir = std::env::temp_dir().join(format!("panopin-db-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("data.db");
        {
            let conn = open(&path).unwrap();
            let mode: String = conn
                .pragma_query_value(None, "journal_mode", |row| row.get(0))
                .unwrap();
            assert_eq!(mode.to_lowercase(), "wal");
            let version: u32 = conn
                .pragma_query_value(None, "user_version", |row| row.get(0))
                .unwrap();
            assert_eq!(version, migrations::latest_version());
        }
        // Reopening an up-to-date database is a no-op.
        drop(open(&path).unwrap());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}

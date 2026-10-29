//! Versioned schema migrations (D-012). `PRAGMA user_version` counts the applied steps.

use rusqlite::Connection;

use crate::error::{AppError, Result};

const MIGRATIONS: &[&str] = &[
    include_str!("migrations/0001_settings.sql"),
    include_str!("migrations/0002_items.sql"),
    include_str!("migrations/0003_shelved.sql"),
    include_str!("migrations/0004_edits.sql"),
    include_str!("migrations/0005_library.sql"),
    include_str!("migrations/0006_search.sql"),
    include_str!("migrations/0007_enrichment.sql"),
];

#[cfg(test)]
pub fn latest_version() -> u32 {
    u32::try_from(MIGRATIONS.len()).unwrap_or(u32::MAX)
}

/// Applies every pending migration, each in its own transaction.
pub fn apply(conn: &mut Connection) -> Result<u32> {
    apply_list(conn, MIGRATIONS)
}

fn apply_list(conn: &mut Connection, migrations: &[&str]) -> Result<u32> {
    let supported = u32::try_from(migrations.len()).unwrap_or(u32::MAX);
    let found: u32 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
    if found > supported {
        return Err(AppError::SchemaTooNew { found, supported });
    }
    for (step, sql) in (1_i64..).zip(migrations).skip(found as usize) {
        let tx = conn.transaction()?;
        tx.execute_batch(sql)?;
        tx.pragma_update(None, "user_version", step)?;
        tx.commit()?;
    }
    Ok(supported)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn version(conn: &Connection) -> u32 {
        conn.pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap()
    }

    fn table_exists(conn: &Connection, name: &str) -> bool {
        conn.query_row(
            "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
            [name],
            |row| row.get::<_, i64>(0),
        )
        .unwrap()
            == 1
    }

    #[test]
    fn fresh_database_reaches_latest_version() {
        let mut conn = Connection::open_in_memory().unwrap();
        assert_eq!(apply(&mut conn).unwrap(), latest_version());
        assert_eq!(version(&conn), latest_version());
        assert!(table_exists(&conn, "settings"));
    }

    #[test]
    fn applying_twice_changes_nothing() {
        let mut conn = Connection::open_in_memory().unwrap();
        apply(&mut conn).unwrap();
        apply(&mut conn).unwrap();
        assert_eq!(version(&conn), latest_version());
    }

    #[test]
    fn newer_schema_is_refused() {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "user_version", 99).unwrap();
        assert!(matches!(
            apply(&mut conn),
            Err(AppError::SchemaTooNew { found: 99, .. })
        ));
    }

    #[test]
    fn failing_step_rolls_back_completely() {
        let mut conn = Connection::open_in_memory().unwrap();
        let steps = [
            "CREATE TABLE a (x INTEGER);",
            "CREATE TABLE b (y INTEGER); INSERT INTO missing VALUES (1);",
        ];
        assert!(apply_list(&mut conn, &steps).is_err());
        assert_eq!(version(&conn), 1);
        assert!(table_exists(&conn, "a"));
        assert!(!table_exists(&conn, "b"));
    }
}

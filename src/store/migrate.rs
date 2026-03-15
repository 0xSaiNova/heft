//! Schema migrations for the heft database.
//!
//! Uses SQLite's PRAGMA user_version to track which migrations have been applied.
//! Each migration runs once and bumps the version. Safe to call on every open.

use rusqlite::Connection;

const CURRENT_VERSION: i64 = 1;

/// Run any pending migrations. Called on every Store::open().
pub fn run_migrations(conn: &Connection) -> rusqlite::Result<()> {
    let version: i64 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;

    if version < 1 {
        migrate_v1(conn)?;
    }

    if version < CURRENT_VERSION {
        conn.pragma_update(None, "user_version", CURRENT_VERSION)?;
    }

    Ok(())
}

/// v1: add activity tracking columns to entries table.
/// For fresh databases, init_schema already includes these columns,
/// so we silently ignore "duplicate column" errors.
fn migrate_v1(conn: &Connection) -> rusqlite::Result<()> {
    for sql in [
        "ALTER TABLE entries ADD COLUMN active INTEGER DEFAULT NULL",
        "ALTER TABLE entries ADD COLUMN active_reason TEXT DEFAULT NULL",
    ] {
        match conn.execute(sql, []) {
            Ok(_) => {}
            Err(e) => {
                let msg = e.to_string();
                if msg.contains("duplicate column") {
                    // column already exists (fresh db with init_schema), safe to skip
                } else {
                    return Err(e);
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn setup_legacy_schema(conn: &Connection) {
        conn.execute_batch(
            "CREATE TABLE entries (
                id INTEGER PRIMARY KEY,
                snapshot_id INTEGER NOT NULL,
                category TEXT NOT NULL,
                name TEXT NOT NULL,
                location TEXT NOT NULL,
                size_bytes INTEGER NOT NULL,
                reclaimable_bytes INTEGER NOT NULL,
                last_modified INTEGER,
                cleanup_hint TEXT
            )"
        ).unwrap();
    }

    #[test]
    fn migration_adds_columns_to_legacy_db() {
        let conn = Connection::open_in_memory().unwrap();
        setup_legacy_schema(&conn);

        run_migrations(&conn).unwrap();

        // verify columns exist by inserting with them
        conn.execute(
            "INSERT INTO entries (snapshot_id, category, name, location, size_bytes, reclaimable_bytes, active, active_reason)
             VALUES (1, 'test', 'test', '/tmp', 100, 100, 1, 'git commit 2h ago')",
            [],
        ).unwrap();

        let version: i64 = conn.pragma_query_value(None, "user_version", |r| r.get(0)).unwrap();
        assert_eq!(version, CURRENT_VERSION);
    }

    #[test]
    fn migration_is_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        setup_legacy_schema(&conn);

        run_migrations(&conn).unwrap();
        // running again should not error
        run_migrations(&conn).unwrap();
    }

    #[test]
    fn migration_skips_when_columns_already_exist() {
        let conn = Connection::open_in_memory().unwrap();
        // schema already has the columns (simulates fresh db)
        conn.execute_batch(
            "CREATE TABLE entries (
                id INTEGER PRIMARY KEY,
                snapshot_id INTEGER NOT NULL,
                category TEXT NOT NULL,
                name TEXT NOT NULL,
                location TEXT NOT NULL,
                size_bytes INTEGER NOT NULL,
                reclaimable_bytes INTEGER NOT NULL,
                last_modified INTEGER,
                cleanup_hint TEXT,
                active INTEGER,
                active_reason TEXT
            )"
        ).unwrap();

        // should not error despite columns already existing
        run_migrations(&conn).unwrap();
    }
}

use anyhow::{Context, Result};
use rusqlite::{Connection, OpenFlags};
use std::path::PathBuf;

/// A row from the aggregated history query.
pub struct DirEntry {
    pub cwd: String,
    /// Number of commands run in this directory.
    pub freq: i64,
    /// Most recent visit timestamp in nanoseconds since Unix epoch.
    pub last_visit_ns: i64,
}

/// Resolve the path to the Atuin history database.
///
/// Priority:
/// 1. Explicit `--db` flag
/// 2. `ATUIN_DB_PATH` env var
/// 3. `ATUIN_DATA_DIR` / history.db
/// 4. `XDG_DATA_HOME` / atuin / history.db
/// 5. ~/.local/share/atuin/history.db
pub fn resolve_db_path(cli_override: Option<&str>) -> Result<PathBuf> {
    if let Some(p) = cli_override {
        return Ok(PathBuf::from(p));
    }

    if let Ok(p) = std::env::var("ATUIN_DB_PATH") {
        return Ok(PathBuf::from(p));
    }

    if let Ok(data_dir) = std::env::var("ATUIN_DATA_DIR") {
        return Ok(PathBuf::from(data_dir).join("history.db"));
    }

    if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
        return Ok(PathBuf::from(xdg).join("atuin").join("history.db"));
    }

    let home = dirs::home_dir().context("could not determine home directory")?;
    Ok(home
        .join(".local")
        .join("share")
        .join("atuin")
        .join("history.db"))
}

/// Open the Atuin history database in read-only mode.
pub fn open(path: &PathBuf) -> Result<Connection> {
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    conn.execute_batch("PRAGMA query_only = ON;")?;
    Ok(conn)
}

/// Create the Atuin history table schema in the given connection.
///
/// This is used by tests to set up an in-memory database. It is not used
/// in production (where we read the real Atuin database).
#[cfg(test)]
pub fn create_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS history (
            id TEXT PRIMARY KEY,
            timestamp INTEGER NOT NULL,
            duration INTEGER NOT NULL,
            exit INTEGER NOT NULL,
            command TEXT NOT NULL,
            cwd TEXT NOT NULL,
            session TEXT NOT NULL,
            hostname TEXT NOT NULL,
            deleted_at INTEGER
        );",
    )?;
    Ok(())
}

/// Query the history table, returning aggregated directory entries.
///
/// If `cwd_prefix` is `Some`, restricts results to subdirectories of that path.
pub fn query_dirs(conn: &Connection, cwd_prefix: Option<&str>) -> Result<Vec<DirEntry>> {
    let (sql, params): (String, Vec<Box<dyn rusqlite::types::ToSql>>) = match cwd_prefix {
        Some(prefix) => {
            let pattern = format!("{}/%", prefix);
            (
                "SELECT cwd, count(*) AS freq, max(timestamp) AS last_visit \
                 FROM history \
                 WHERE deleted_at IS NULL AND cwd LIKE ?1 \
                 GROUP BY cwd"
                    .to_string(),
                vec![Box::new(pattern) as Box<dyn rusqlite::types::ToSql>],
            )
        }
        None => (
            "SELECT cwd, count(*) AS freq, max(timestamp) AS last_visit \
             FROM history \
             WHERE deleted_at IS NULL \
             GROUP BY cwd"
                .to_string(),
            vec![],
        ),
    };

    let mut stmt = conn.prepare(&sql)?;
    let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
    let rows = stmt.query_map(param_refs.as_slice(), |row| {
        Ok(DirEntry {
            cwd: row.get(0)?,
            freq: row.get(1)?,
            last_visit_ns: row.get(2)?,
        })
    })?;

    let mut entries = Vec::new();
    for row in rows {
        entries.push(row?);
    }
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        create_schema(&conn).unwrap();
        conn
    }

    fn insert_history(conn: &Connection, id: &str, cwd: &str, timestamp: i64) {
        conn.execute(
            "INSERT INTO history (id, timestamp, duration, exit, command, cwd, session, hostname)
             VALUES (?1, ?2, 0, 0, 'test', ?3, 'sess', 'host')",
            rusqlite::params![id, timestamp, cwd],
        )
        .unwrap();
    }

    fn insert_deleted(conn: &Connection, id: &str, cwd: &str, timestamp: i64) {
        conn.execute(
            "INSERT INTO history (id, timestamp, duration, exit, command, cwd, session, hostname, deleted_at)
             VALUES (?1, ?2, 0, 0, 'test', ?3, 'sess', 'host', ?2)",
            rusqlite::params![id, timestamp, cwd],
        )
        .unwrap();
    }

    // --- resolve_db_path ---

    #[test]
    fn resolve_db_path_cli_override() {
        let path = resolve_db_path(Some("/custom/path.db")).unwrap();
        assert_eq!(path, PathBuf::from("/custom/path.db"));
    }

    // --- query_dirs ---

    #[test]
    fn query_dirs_empty_db() {
        let conn = setup_test_db();
        let entries = query_dirs(&conn, None).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn query_dirs_aggregates_by_cwd() {
        let conn = setup_test_db();
        insert_history(&conn, "1", "/home/user/a", 100);
        insert_history(&conn, "2", "/home/user/a", 200);
        insert_history(&conn, "3", "/home/user/a", 300);
        insert_history(&conn, "4", "/home/user/b", 400);

        let entries = query_dirs(&conn, None).unwrap();
        assert_eq!(entries.len(), 2);

        let a = entries.iter().find(|e| e.cwd == "/home/user/a").unwrap();
        assert_eq!(a.freq, 3);
        assert_eq!(a.last_visit_ns, 300);

        let b = entries.iter().find(|e| e.cwd == "/home/user/b").unwrap();
        assert_eq!(b.freq, 1);
        assert_eq!(b.last_visit_ns, 400);
    }

    #[test]
    fn query_dirs_excludes_deleted() {
        let conn = setup_test_db();
        insert_history(&conn, "1", "/home/user/keep", 100);
        insert_deleted(&conn, "2", "/home/user/gone", 200);

        let entries = query_dirs(&conn, None).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].cwd, "/home/user/keep");
    }

    #[test]
    fn query_dirs_with_cwd_prefix() {
        let conn = setup_test_db();
        insert_history(&conn, "1", "/home/user/projects/foo", 100);
        insert_history(&conn, "2", "/home/user/projects/bar", 200);
        insert_history(&conn, "3", "/home/user/documents/baz", 300);

        let entries = query_dirs(&conn, Some("/home/user/projects")).unwrap();
        assert_eq!(entries.len(), 2);
        assert!(entries.iter().all(|e| e.cwd.starts_with("/home/user/projects/")));
    }

    #[test]
    fn query_dirs_prefix_does_not_match_self() {
        let conn = setup_test_db();
        // The prefix directory itself should not match (LIKE 'prefix/%' won't match 'prefix')
        insert_history(&conn, "1", "/home/user", 100);
        insert_history(&conn, "2", "/home/user/child", 200);

        let entries = query_dirs(&conn, Some("/home/user")).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].cwd, "/home/user/child");
    }
}

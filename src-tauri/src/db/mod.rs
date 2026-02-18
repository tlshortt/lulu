use rusqlite::{Connection, Result};
use std::path::Path;
use std::sync::Mutex;

pub mod session;
pub use session::{
    Session, SessionDashboardRow, SessionHistoryEvent, SessionMessage, SessionRunMetadata,
};

pub struct Database {
    pub conn: Mutex<Connection>,
}

pub fn init_database(db_path: &Path) -> Result<Database> {
    let conn = Connection::open(db_path)?;

    conn.execute_batch(
        "PRAGMA journal_mode=WAL;
         PRAGMA synchronous=NORMAL;
         PRAGMA busy_timeout=5000;
         PRAGMA foreign_keys=ON;",
    )?;

    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS sessions (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'created',
            working_dir TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            last_activity_at TEXT,
            failure_reason TEXT,
            worktree_path TEXT,
            resume_count INTEGER NOT NULL DEFAULT 0,
            active_run_id TEXT,
            last_resume_at TEXT,
            restored INTEGER NOT NULL DEFAULT 0,
            restored_at TEXT,
            recovery_hint INTEGER NOT NULL DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS messages (
            id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL,
            role TEXT NOT NULL,
            content TEXT NOT NULL,
            timestamp TEXT NOT NULL,
            FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_messages_session_id ON messages(session_id);",
    )?;

    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS session_events (
            id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL,
            run_id TEXT NOT NULL,
            seq INTEGER NOT NULL,
            event_type TEXT NOT NULL,
            payload_json TEXT NOT NULL,
            timestamp TEXT NOT NULL,
            FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE,
            UNIQUE(session_id, run_id, seq)
        );

        CREATE INDEX IF NOT EXISTS idx_session_events_session_id_timestamp
            ON session_events(session_id, timestamp, seq, id);

        CREATE INDEX IF NOT EXISTS idx_session_events_session_id_run_id_seq
            ON session_events(session_id, run_id, seq);",
    )?;

    ensure_session_column(&conn, "last_activity_at", "TEXT")?;
    ensure_session_column(&conn, "failure_reason", "TEXT")?;
    ensure_session_column(&conn, "worktree_path", "TEXT")?;
    ensure_session_column(&conn, "resume_count", "INTEGER NOT NULL DEFAULT 0")?;
    ensure_session_column(&conn, "active_run_id", "TEXT")?;
    ensure_session_column(&conn, "last_resume_at", "TEXT")?;
    ensure_session_column(&conn, "restored", "INTEGER NOT NULL DEFAULT 0")?;
    ensure_session_column(&conn, "restored_at", "TEXT")?;
    ensure_session_column(&conn, "recovery_hint", "INTEGER NOT NULL DEFAULT 0")?;

    Ok(Database { conn: Mutex::new(conn) })
}

fn ensure_session_column(conn: &Connection, column_name: &str, column_definition: &str) -> Result<()> {
    let mut stmt = conn.prepare("PRAGMA table_info(sessions)")?;
    let mut rows = stmt.query([])?;

    while let Some(row) = rows.next()? {
        let existing_name: String = row.get(1)?;
        if existing_name == column_name {
            return Ok(());
        }
    }

    conn.execute(
        &format!(
            "ALTER TABLE sessions ADD COLUMN {} {}",
            column_name, column_definition
        ),
        [],
    )?;

    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("Database error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("Lock error")]
    Lock,
}

impl serde::Serialize for DbError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn init_database_creates_file() -> Result<()> {
        let dir = tempdir().expect("failed to create temp dir");
        let db_path = dir.path().join("lulu-test.db");

        let _database = init_database(&db_path)?;

        assert!(db_path.exists(), "database file should be created");
        Ok(())
    }
}

use crate::db::{Database, DbError};
use rusqlite::params;
use serde::{Deserialize, Serialize};

fn is_terminal_status(status: &str) -> bool {
    matches!(status, "completed" | "failed" | "killed")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub name: String,
    pub status: String,
    pub working_dir: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMessage {
    pub id: String,
    pub session_id: String,
    pub role: String,
    pub content: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionDashboardRow {
    pub id: String,
    pub name: String,
    pub status: String,
    pub created_at: String,
    pub last_activity_at: Option<String>,
    pub failure_reason: Option<String>,
    pub worktree_path: Option<String>,
}

impl Database {
    pub fn create_session(&self, session: &Session) -> Result<(), DbError> {
        let mut conn = self.conn.lock().map_err(|_| DbError::Lock)?;

        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;

        tx.execute(
            "INSERT INTO sessions (id, name, status, working_dir, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                session.id,
                session.name,
                session.status,
                session.working_dir,
                session.created_at,
                session.updated_at,
            ],
        )?;

        tx.commit()?;
        Ok(())
    }

    pub fn get_session(&self, id: &str) -> Result<Option<Session>, DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::Lock)?;

        let mut stmt = conn.prepare(
            "SELECT id, name, status, working_dir, created_at, updated_at
             FROM sessions WHERE id = ?1",
        )?;

        let mut rows = stmt.query(params![id])?;

        if let Some(row) = rows.next()? {
            Ok(Some(Session {
                id: row.get(0)?,
                name: row.get(1)?,
                status: row.get(2)?,
                working_dir: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn list_sessions(&self) -> Result<Vec<Session>, DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::Lock)?;

        let mut stmt = conn.prepare(
            "SELECT id, name, status, working_dir, created_at, updated_at
             FROM sessions ORDER BY created_at DESC",
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(Session {
                id: row.get(0)?,
                name: row.get(1)?,
                status: row.get(2)?,
                working_dir: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })?;

        let mut sessions = Vec::new();
        for session in rows {
            sessions.push(session?);
        }

        Ok(sessions)
    }

    pub fn update_session_status(&self, id: &str, status: &str) -> Result<(), DbError> {
        let mut conn = self.conn.lock().map_err(|_| DbError::Lock)?;
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;

        let now = chrono::Utc::now().to_rfc3339();
        tx.execute(
            "UPDATE sessions SET status = ?1, updated_at = ?2 WHERE id = ?3",
            params![status, now, id],
        )?;

        tx.commit()?;
        Ok(())
    }

    pub fn update_session_name(&self, id: &str, name: &str) -> Result<(), DbError> {
        let mut conn = self.conn.lock().map_err(|_| DbError::Lock)?;
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;

        let now = chrono::Utc::now().to_rfc3339();
        tx.execute(
            "UPDATE sessions SET name = ?1, updated_at = ?2 WHERE id = ?3",
            params![name, now, id],
        )?;

        tx.commit()?;
        Ok(())
    }

    pub fn transition_session_terminal(&self, id: &str, status: &str) -> Result<bool, DbError> {
        if !is_terminal_status(status) {
            return Ok(false);
        }

        let mut conn = self.conn.lock().map_err(|_| DbError::Lock)?;
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;

        let now = chrono::Utc::now().to_rfc3339();
        let updated = tx.execute(
            "UPDATE sessions
             SET status = ?1, updated_at = ?2
             WHERE id = ?3 AND status = 'running'",
            params![status, now, id],
        )?;

        tx.commit()?;
        Ok(updated > 0)
    }

    pub fn delete_session(&self, id: &str) -> Result<(), DbError> {
        let mut conn = self.conn.lock().map_err(|_| DbError::Lock)?;
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;

        tx.execute("DELETE FROM messages WHERE session_id = ?1", params![id])?;
        tx.execute("DELETE FROM sessions WHERE id = ?1", params![id])?;

        tx.commit()?;
        Ok(())
    }

    pub fn update_last_activity(&self, id: &str, timestamp: &str) -> Result<(), DbError> {
        let mut conn = self.conn.lock().map_err(|_| DbError::Lock)?;
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;

        tx.execute(
            "UPDATE sessions SET last_activity_at = ?1, updated_at = ?2 WHERE id = ?3",
            params![timestamp, chrono::Utc::now().to_rfc3339(), id],
        )?;

        tx.commit()?;
        Ok(())
    }

    pub fn update_failure_reason(&self, id: &str, reason: Option<&str>) -> Result<(), DbError> {
        let mut conn = self.conn.lock().map_err(|_| DbError::Lock)?;
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;

        tx.execute(
            "UPDATE sessions SET failure_reason = ?1, updated_at = ?2 WHERE id = ?3",
            params![reason, chrono::Utc::now().to_rfc3339(), id],
        )?;

        tx.commit()?;
        Ok(())
    }

    pub fn update_worktree_path(&self, id: &str, worktree_path: Option<&str>) -> Result<(), DbError> {
        let mut conn = self.conn.lock().map_err(|_| DbError::Lock)?;
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;

        tx.execute(
            "UPDATE sessions SET worktree_path = ?1, updated_at = ?2 WHERE id = ?3",
            params![worktree_path, chrono::Utc::now().to_rfc3339(), id],
        )?;

        tx.commit()?;
        Ok(())
    }

    pub fn list_dashboard_sessions(&self) -> Result<Vec<SessionDashboardRow>, DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::Lock)?;

        let mut stmt = conn.prepare(
            "SELECT id, name, status, created_at, last_activity_at, failure_reason, worktree_path
             FROM sessions
             ORDER BY created_at DESC",
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(SessionDashboardRow {
                id: row.get(0)?,
                name: row.get(1)?,
                status: row.get(2)?,
                created_at: row.get(3)?,
                last_activity_at: row.get(4)?,
                failure_reason: row.get(5)?,
                worktree_path: row.get(6)?,
            })
        })?;

        let mut sessions = Vec::new();
        for session in rows {
            sessions.push(session?);
        }

        Ok(sessions)
    }

    pub fn get_session_worktree_path(&self, id: &str) -> Result<Option<String>, DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::Lock)?;

        let mut stmt = conn.prepare("SELECT worktree_path FROM sessions WHERE id = ?1")?;
        let mut rows = stmt.query(params![id])?;

        if let Some(row) = rows.next()? {
            let worktree_path: Option<String> = row.get(0)?;
            Ok(worktree_path)
        } else {
            Ok(None)
        }
    }

    pub fn insert_session_message(
        &self,
        session_id: &str,
        role: &str,
        content: &str,
        timestamp: &str,
    ) -> Result<(), DbError> {
        let mut conn = self.conn.lock().map_err(|_| DbError::Lock)?;
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;

        tx.execute(
            "INSERT INTO messages (id, session_id, role, content, timestamp)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                uuid::Uuid::new_v4().to_string(),
                session_id,
                role,
                content,
                timestamp,
            ],
        )?;

        tx.commit()?;
        Ok(())
    }

    pub fn list_session_messages(&self, session_id: &str) -> Result<Vec<SessionMessage>, DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::Lock)?;

        let mut stmt = conn.prepare(
            "SELECT id, session_id, role, content, timestamp
             FROM messages
             WHERE session_id = ?1
             ORDER BY timestamp ASC",
        )?;

        let rows = stmt.query_map(params![session_id], |row| {
            Ok(SessionMessage {
                id: row.get(0)?,
                session_id: row.get(1)?,
                role: row.get(2)?,
                content: row.get(3)?,
                timestamp: row.get(4)?,
            })
        })?;

        let mut messages = Vec::new();
        for message in rows {
            messages.push(message?);
        }

        Ok(messages)
    }
}

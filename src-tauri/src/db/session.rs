use crate::db::{Database, DbError};
use rusqlite::params;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub name: String,
    pub status: String,
    pub working_dir: String,
    pub created_at: String,
    pub updated_at: String,
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

    pub fn delete_session(&self, id: &str) -> Result<(), DbError> {
        let mut conn = self.conn.lock().map_err(|_| DbError::Lock)?;
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;

        tx.execute("DELETE FROM messages WHERE session_id = ?1", params![id])?;
        tx.execute("DELETE FROM sessions WHERE id = ?1", params![id])?;

        tx.commit()?;
        Ok(())
    }
}

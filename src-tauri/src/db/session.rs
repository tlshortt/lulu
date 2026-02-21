use crate::db::{Database, DbError};
use rusqlite::params;
use serde::{Deserialize, Serialize};

/// Returns `true` if `status` represents a terminal session state from which no further
/// transitions are expected (completed, failed, killed, or interrupted).
pub fn is_terminal_status(status: &str) -> bool {
    matches!(status, "completed" | "failed" | "killed" | "interrupted")
}

fn is_inflight_status(status: &str) -> bool {
    matches!(status, "starting" | "running" | "interrupting" | "resuming")
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
    pub restored: bool,
    pub restored_at: Option<String>,
    pub recovery_hint: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionHistoryEvent {
    pub id: String,
    pub session_id: String,
    pub run_id: String,
    pub seq: i64,
    pub event_type: String,
    pub payload_json: serde_json::Value,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRunMetadata {
    pub resume_count: i64,
    pub active_run_id: Option<String>,
    pub last_resume_at: Option<String>,
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
             WHERE id = ?3 AND status IN ('starting', 'running', 'interrupting', 'resuming')",
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
            "SELECT id,
                    name,
                    status,
                    created_at,
                    last_activity_at,
                    failure_reason,
                    worktree_path,
                    restored,
                    restored_at,
                    recovery_hint
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
                restored: row.get::<_, i64>(7)? != 0,
                restored_at: row.get(8)?,
                recovery_hint: row.get::<_, i64>(9)? != 0,
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

    pub fn reconcile_stale_inflight_sessions(&self) -> Result<Vec<String>, DbError> {
        let mut conn = self.conn.lock().map_err(|_| DbError::Lock)?;
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;

        let mut stmt = tx.prepare("SELECT id FROM sessions WHERE status IN ('starting', 'running', 'interrupting', 'resuming')")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;

        let mut stale_ids = Vec::new();
        for row in rows {
            stale_ids.push(row?);
        }
        drop(stmt);

        if !stale_ids.is_empty() {
            let now = chrono::Utc::now().to_rfc3339();
            tx.execute(
                "UPDATE sessions
                 SET restored = 1,
                     restored_at = ?1,
                     recovery_hint = 1,
                     updated_at = ?1
                 WHERE status IN ('starting', 'running', 'interrupting', 'resuming')",
                params![now],
            )?;
        }

        tx.commit()?;
        Ok(stale_ids)
    }

    pub fn mark_sessions_restored(&self, session_ids: &[String], restored_at: &str) -> Result<(), DbError> {
        if session_ids.is_empty() {
            return Ok(());
        }

        let mut conn = self.conn.lock().map_err(|_| DbError::Lock)?;
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let mut stmt = tx.prepare(
            "UPDATE sessions
             SET restored = 1,
                 restored_at = ?1,
                 recovery_hint = 1,
                 updated_at = ?1
             WHERE id = ?2",
        )?;

        for session_id in session_ids {
            stmt.execute(params![restored_at, session_id])?;
        }
        drop(stmt);

        tx.commit()?;
        Ok(())
    }

    pub fn clear_restored_metadata(&self, session_id: &str) -> Result<(), DbError> {
        let mut conn = self.conn.lock().map_err(|_| DbError::Lock)?;
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;

        tx.execute(
            "UPDATE sessions
             SET restored = 0,
                 restored_at = NULL,
                 recovery_hint = 0,
                 updated_at = ?1
             WHERE id = ?2 AND (restored = 1 OR recovery_hint = 1)",
            params![chrono::Utc::now().to_rfc3339(), session_id],
        )?;

        tx.commit()?;
        Ok(())
    }

    pub fn transition_session_to_interrupting(&self, id: &str) -> Result<bool, DbError> {
        let mut conn = self.conn.lock().map_err(|_| DbError::Lock)?;
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;

        let mut stmt = tx.prepare("SELECT status FROM sessions WHERE id = ?1")?;
        let mut rows = stmt.query(params![id])?;

        let Some(row) = rows.next()? else {
            return Ok(false);
        };

        let current_status: String = row.get(0)?;
        if !is_inflight_status(&current_status) {
            return Ok(false);
        }
        drop(rows);
        drop(stmt);

        let updated = tx.execute(
            "UPDATE sessions
             SET status = 'interrupting', updated_at = ?1
             WHERE id = ?2",
            params![chrono::Utc::now().to_rfc3339(), id],
        )?;

        tx.commit()?;
        Ok(updated > 0)
    }

    pub fn begin_resume_attempt(&self, id: &str, run_id: &str, resumed_at: &str) -> Result<bool, DbError> {
        let mut conn = self.conn.lock().map_err(|_| DbError::Lock)?;
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;

        let updated = tx.execute(
            "UPDATE sessions
             SET status = 'resuming',
                 resume_count = resume_count + 1,
                 active_run_id = ?1,
                 last_resume_at = ?2,
                 failure_reason = NULL,
                 updated_at = ?2
             WHERE id = ?3 AND status IN ('completed', 'interrupted')",
            params![run_id, resumed_at, id],
        )?;

        tx.commit()?;
        Ok(updated > 0)
    }

    pub fn begin_run_attempt(&self, id: &str, run_id: &str) -> Result<bool, DbError> {
        let mut conn = self.conn.lock().map_err(|_| DbError::Lock)?;
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;

        let now = chrono::Utc::now().to_rfc3339();
        let updated = tx.execute(
            "UPDATE sessions
             SET status = 'running',
                 active_run_id = ?1,
                 failure_reason = NULL,
                 updated_at = ?2,
                 last_activity_at = ?2
             WHERE id = ?3",
            params![run_id, now, id],
        )?;

        tx.commit()?;
        Ok(updated > 0)
    }

    pub fn get_session_run_metadata(&self, id: &str) -> Result<Option<SessionRunMetadata>, DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::Lock)?;

        let mut stmt = conn.prepare(
            "SELECT resume_count, active_run_id, last_resume_at
             FROM sessions
             WHERE id = ?1",
        )?;
        let mut rows = stmt.query(params![id])?;

        if let Some(row) = rows.next()? {
            Ok(Some(SessionRunMetadata {
                resume_count: row.get(0)?,
                active_run_id: row.get(1)?,
                last_resume_at: row.get(2)?,
            }))
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

    pub fn insert_session_event(
        &self,
        session_id: &str,
        run_id: &str,
        seq: u64,
        event_type: &str,
        payload_json: &serde_json::Value,
        timestamp: &str,
    ) -> Result<(), DbError> {
        let mut conn = self.conn.lock().map_err(|_| DbError::Lock)?;
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;

        tx.execute(
            "INSERT INTO session_events (id, session_id, run_id, seq, event_type, payload_json, timestamp)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(session_id, run_id, seq) DO NOTHING",
            params![
                uuid::Uuid::new_v4().to_string(),
                session_id,
                run_id,
                seq as i64,
                event_type,
                payload_json.to_string(),
                timestamp,
            ],
        )?;

        tx.commit()?;
        Ok(())
    }

    pub fn list_session_history(&self, session_id: &str) -> Result<Vec<SessionHistoryEvent>, DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::Lock)?;

        let mut stmt = conn.prepare(
            "SELECT id, session_id, run_id, seq, event_type, payload_json, timestamp
             FROM session_events
             WHERE session_id = ?1
             ORDER BY timestamp ASC, seq ASC, id ASC",
        )?;

        let rows = stmt.query_map(params![session_id], |row| {
            let payload_raw: String = row.get(5)?;
            let payload_json = serde_json::from_str(&payload_raw).map_err(|err| {
                rusqlite::Error::FromSqlConversionFailure(
                    payload_raw.len(),
                    rusqlite::types::Type::Text,
                    Box::new(err),
                )
            })?;

            Ok(SessionHistoryEvent {
                id: row.get(0)?,
                session_id: row.get(1)?,
                run_id: row.get(2)?,
                seq: row.get(3)?,
                event_type: row.get(4)?,
                payload_json,
                timestamp: row.get(6)?,
            })
        })?;

        let mut events = Vec::new();
        for event in rows {
            events.push(event?);
        }

        Ok(events)
    }
}

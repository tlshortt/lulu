use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::db::Database;
use crate::session::projection::normalize_failure_reason;
use serde_json::json;
use tauri::{AppHandle, Emitter};
use tokio::process::Child;
use tokio::sync::{Mutex, RwLock};
use tokio_util::sync::CancellationToken;

fn is_terminal_status(status: &str) -> bool {
    matches!(status, "completed" | "failed" | "killed")
}

fn normalize_terminal_status(status: &str) -> &str {
    if status == "complete" || status == "done" {
        "completed"
    } else {
        status
    }
}

pub struct TerminalTransitionResult {
    pub final_status: String,
    pub failure_message: Option<String>,
}

pub struct SessionRuntime {
    pub id: String,
    pub name: String,
    pub child: Mutex<Child>,
    killed: AtomicBool,
    terminal_transitioned: AtomicBool,
    cancel_token: CancellationToken,
}

impl SessionRuntime {
    fn new(id: String, name: String, child: Child) -> Self {
        Self {
            id,
            name,
            child: Mutex::new(child),
            killed: AtomicBool::new(false),
            terminal_transitioned: AtomicBool::new(false),
            cancel_token: CancellationToken::new(),
        }
    }

    pub fn mark_killed(&self) {
        self.killed.store(true, Ordering::SeqCst);
        self.cancel_token.cancel();
    }

    pub fn was_killed(&self) -> bool {
        self.killed.load(Ordering::SeqCst)
    }

    pub fn begin_terminal_transition(&self) -> bool {
        !self.terminal_transitioned.swap(true, Ordering::SeqCst)
    }

    pub fn cancel_token(&self) -> CancellationToken {
        self.cancel_token.clone()
    }
}

pub struct SessionSupervisor {
    runtimes: RwLock<HashMap<String, Arc<SessionRuntime>>>,
}

impl SessionSupervisor {
    pub fn new() -> Self {
        Self {
            runtimes: RwLock::new(HashMap::new()),
        }
    }

    pub async fn register(&self, session_id: String, name: String, child: Child) -> Arc<SessionRuntime> {
        let runtime = Arc::new(SessionRuntime::new(session_id.clone(), name, child));
        let mut runtimes = self.runtimes.write().await;
        runtimes.insert(session_id, runtime.clone());
        runtime
    }

    pub async fn get(&self, session_id: &str) -> Option<Arc<SessionRuntime>> {
        let runtimes = self.runtimes.read().await;
        runtimes.get(session_id).cloned()
    }

    pub async fn remove(&self, session_id: &str) -> Option<Arc<SessionRuntime>> {
        let mut runtimes = self.runtimes.write().await;
        runtimes.remove(session_id)
    }

    pub async fn begin_terminal_transition(&self, session_id: &str) -> bool {
        if let Some(runtime) = self.get(session_id).await {
            return runtime.begin_terminal_transition();
        }
        false
    }

    pub async fn finalize_terminal_transition(
        &self,
        db: &Database,
        session_id: &str,
        status: &str,
        failure_message: Option<String>,
    ) -> Result<Option<TerminalTransitionResult>, String> {
        self.finalize_terminal_transition_internal(
            db,
            session_id,
            status,
            failure_message,
        )
        .await
    }

    pub async fn finalize_terminal_transition_and_emit(
        &self,
        app: &AppHandle,
        db: &Database,
        session_id: &str,
        status: &str,
        seq: &std::sync::atomic::AtomicU64,
        failure_message: Option<String>,
        emit_structured_status: bool,
    ) -> Result<Option<TerminalTransitionResult>, String> {
        self.finalize_terminal_transition_internal(
            db,
            session_id,
            status,
            failure_message,
        )
        .await
        .map(|transition| {
            if let Some(ref transition) = transition {
                if emit_structured_status {
                    let status_event = json!({
                        "type": "status",
                        "data": {
                            "session_id": session_id,
                            "seq": seq.fetch_add(1, Ordering::SeqCst),
                            "timestamp": chrono::Utc::now().to_rfc3339(),
                            "status": transition.final_status,
                            "message": transition.failure_message,
                        }
                    });
                    let _ = app.emit("session-event", status_event);
                }
            }

            transition
        })
    }

    async fn finalize_terminal_transition_internal(
        &self,
        db: &Database,
        session_id: &str,
        status: &str,
        failure_message: Option<String>,
    ) -> Result<Option<TerminalTransitionResult>, String> {
        if !self.begin_terminal_transition(session_id).await {
            return Ok(None);
        }

        let final_status = normalize_terminal_status(status);
        if is_terminal_status(final_status) {
            db.transition_session_terminal(session_id, final_status)
                .map_err(|err| format!("Failed terminal transition for session {}: {}", session_id, err))?;
        } else {
            db.update_session_status(session_id, final_status)
                .map_err(|err| format!("Failed status update for session {}: {}", session_id, err))?;
        }

        let activity_timestamp = chrono::Utc::now().to_rfc3339();
        db.update_last_activity(session_id, &activity_timestamp)
            .map_err(|err| format!("Failed activity update for session {}: {}", session_id, err))?;

        let normalized_failure = if final_status == "failed" || final_status == "killed" {
            normalize_failure_reason(failure_message.as_deref())
        } else {
            None
        };

        if final_status == "failed" || final_status == "killed" {
            db.update_failure_reason(session_id, normalized_failure.as_deref())
                .map_err(|err| format!("Failed failure update for session {}: {}", session_id, err))?;
        }

        Ok(Some(TerminalTransitionResult {
            final_status: final_status.to_string(),
            failure_message: normalized_failure.or(failure_message),
        }))
    }

    pub async fn was_killed(&self, session_id: &str) -> bool {
        if let Some(runtime) = self.get(session_id).await {
            return runtime.was_killed();
        }
        false
    }

    pub async fn kill_session(&self, session_id: &str) -> Result<bool, String> {
        let Some(runtime) = self.get(session_id).await else {
            return Ok(false);
        };

        runtime.mark_killed();
        let mut child = runtime.child.lock().await;
        child
            .kill()
            .await
            .map_err(|err| format!("Failed to kill session process: {}", err))?;

        Ok(true)
    }

    pub async fn kill_all(&self) {
        let runtimes: Vec<Arc<SessionRuntime>> = {
            let runtimes = self.runtimes.read().await;
            runtimes.values().cloned().collect()
        };

        for runtime in runtimes {
            runtime.mark_killed();
            let mut child = runtime.child.lock().await;
            let _ = child.kill().await;
        }

        let mut runtimes = self.runtimes.write().await;
        runtimes.clear();
    }
}

impl Default for SessionSupervisor {
    fn default() -> Self {
        Self::new()
    }
}

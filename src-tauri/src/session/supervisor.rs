use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::db::{Database, is_terminal_status};
use crate::session::projection::normalize_failure_reason;
use serde_json::json;
use tauri::{AppHandle, Emitter};
use tokio::process::Child;
use tokio::time::sleep;
use tokio::sync::{Mutex, RwLock};
use tokio_util::sync::CancellationToken;

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
    interrupt_requested: AtomicBool,
    interrupt_requests: AtomicUsize,
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
            interrupt_requested: AtomicBool::new(false),
            interrupt_requests: AtomicUsize::new(0),
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

    pub fn mark_interrupt_requested(&self) {
        self.interrupt_requested.store(true, Ordering::SeqCst);
    }

    pub fn was_interrupt_requested(&self) -> bool {
        self.interrupt_requested.load(Ordering::SeqCst)
    }

    pub fn record_interrupt_attempt(&self) {
        self.interrupt_requests.fetch_add(1, Ordering::SeqCst);
    }

    pub fn interrupt_attempts(&self) -> usize {
        self.interrupt_requests.load(Ordering::SeqCst)
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
    lifecycle_ops: std::sync::Mutex<HashSet<(String, String)>>,
}

pub struct LifecycleOperationGuard<'a> {
    supervisor: &'a SessionSupervisor,
    session_id: String,
    operation: String,
}

impl Drop for LifecycleOperationGuard<'_> {
    fn drop(&mut self) {
        if let Ok(mut ops) = self.supervisor.lifecycle_ops.lock() {
            ops.remove(&(self.session_id.clone(), self.operation.clone()));
        }
    }
}

impl SessionSupervisor {
    pub fn new() -> Self {
        Self {
            runtimes: RwLock::new(HashMap::new()),
            lifecycle_ops: std::sync::Mutex::new(HashSet::new()),
        }
    }

    pub fn acquire_lifecycle_operation(
        &self,
        session_id: &str,
        operation: &str,
    ) -> Result<LifecycleOperationGuard<'_>, String> {
        let mut ops = self
            .lifecycle_ops
            .lock()
            .map_err(|_| "Lifecycle operation lock poisoned".to_string())?;
        if let Some((_, in_progress)) = ops.iter().find(|(sid, _)| sid == session_id) {
            return Err(format!(
                "Session {} already has an in-progress {} operation",
                session_id, in_progress
            ));
        }

        let key = (session_id.to_string(), operation.to_string());
        ops.insert(key);
        Ok(LifecycleOperationGuard {
            supervisor: self,
            session_id: session_id.to_string(),
            operation: operation.to_string(),
        })
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

    pub async fn interrupt_session_with_deadline(
        &self,
        db: &Database,
        session_id: &str,
        total_deadline: Duration,
    ) -> Result<(), String> {
        let _op = self.acquire_lifecycle_operation(session_id, "interrupt")?;

        let transitioned = db
            .transition_session_to_interrupting(session_id)
            .map_err(|err| format!("Failed to mark session interrupting: {}", err))?;
        if !transitioned {
            return Err("Session is not in an interruptible state".to_string());
        }

        let started = Instant::now();
        let deadline = started + total_deadline;
        let retry_deadline = started + (total_deadline / 2);

        let _ = self.request_interrupt_once(session_id).await;
        if self
            .wait_for_runtime_exit(session_id, retry_deadline.min(deadline))
            .await?
        {
            let _ = self
                .finalize_terminal_transition(db, session_id, "interrupted", None)
                .await;
            let _ = self.remove(session_id).await;
            return Ok(());
        }

        let _ = self.request_interrupt_once(session_id).await;
        if self.wait_for_runtime_exit(session_id, deadline).await? {
            let _ = self
                .finalize_terminal_transition(db, session_id, "interrupted", None)
                .await;
            let _ = self.remove(session_id).await;
            return Ok(());
        }

        db.update_session_status(session_id, "running")
            .map_err(|err| format!("Failed to restore session status after interrupt timeout: {}", err))?;

        Err("Interrupt did not complete within 10 seconds".to_string())
    }

    async fn request_interrupt_once(&self, session_id: &str) -> Result<(), String> {
        let Some(runtime) = self.get(session_id).await else {
            return Ok(());
        };

        runtime.mark_interrupt_requested();
        runtime.record_interrupt_attempt();

        let Ok(mut child) = tokio::time::timeout(Duration::from_millis(250), runtime.child.lock()).await else {
            return Ok(());
        };

        if let Ok(Some(_)) = child.try_wait() {
            return Ok(());
        }

        // Send SIGINT so the Claude CLI can clean up gracefully (flush state, run hooks).
        // start_kill() would send SIGKILL and bypass all cleanup.
        #[cfg(unix)]
        if let Some(pid) = child.id() {
            return nix::sys::signal::kill(
                nix::unistd::Pid::from_raw(pid as i32),
                nix::sys::signal::Signal::SIGINT,
            )
            .map_err(|err| format!("Failed to interrupt session process: {}", err));
        }

        // Non-Unix fallback (no direct SIGINT equivalent for child processes on Windows).
        child
            .start_kill()
            .map_err(|err| format!("Failed to interrupt session process: {}", err))
    }

    async fn wait_for_runtime_exit(&self, session_id: &str, deadline: Instant) -> Result<bool, String> {
        loop {
            if Instant::now() >= deadline {
                return Ok(false);
            }

            let Some(runtime) = self.get(session_id).await else {
                return Ok(true);
            };

            if let Ok(mut child) = tokio::time::timeout(Duration::from_millis(250), runtime.child.lock()).await {
                match child.try_wait() {
                    Ok(Some(_)) => return Ok(true),
                    Ok(None) => {}
                    Err(err) => {
                        return Err(format!(
                            "Failed checking interrupted process status for session {}: {}",
                            session_id, err
                        ));
                    }
                }
            }

            sleep(Duration::from_millis(50)).await;
        }
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

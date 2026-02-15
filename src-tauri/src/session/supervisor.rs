use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tokio::process::Child;
use tokio::sync::{Mutex, RwLock};
use tokio_util::sync::CancellationToken;

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

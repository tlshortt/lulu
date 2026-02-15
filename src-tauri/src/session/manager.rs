use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct SessionManager {
    pub sessions: Arc<Mutex<HashMap<String, SessionHandle>>>,
}

pub struct SessionHandle {
    pub id: String,
    pub name: String,
    pub child: Arc<Mutex<tokio::process::Child>>,
    pub killed: Arc<Mutex<bool>>,
}

impl SessionManager {
    pub fn new() -> Self {
        SessionManager { sessions: Arc::new(Mutex::new(HashMap::new())) }
    }

    /// Kill all running sessions on app exit
    pub async fn kill_all(&self) {
        let mut sessions = self.sessions.lock().await;
        for handle in sessions.values_mut() {
            let mut killed = handle.killed.lock().await;
            if !*killed {
                let mut child = handle.child.lock().await;
                let _ = child.kill().await;
                *killed = true;
            }
        }
        sessions.clear();
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

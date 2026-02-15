use std::sync::Arc;

use crate::session::supervisor::SessionSupervisor;

pub struct SessionManager {
    pub supervisor: Arc<SessionSupervisor>,
}

impl SessionManager {
    pub fn new() -> Self {
        SessionManager {
            supervisor: Arc::new(SessionSupervisor::new()),
        }
    }

    /// Kill all running sessions on app exit
    pub async fn kill_all(&self) {
        self.supervisor.kill_all().await;
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

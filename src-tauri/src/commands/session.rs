use crate::db::{Database, Session};
use crate::session::manager::SessionHandle;
use crate::session::{ClaudeCli, SessionManager};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};

#[derive(Clone, serde::Serialize)]
struct SessionOutput {
    session_id: String,
    line: String,
}

#[tauri::command]
pub async fn spawn_session(
    app: AppHandle,
    db: State<'_, Database>,
    manager: State<'_, Arc<Mutex<SessionManager>>>,
    name: String,
    prompt: String,
    working_dir: String,
) -> Result<String, String> {
    let cli = ClaudeCli::find().ok_or("Claude CLI not found")?;

    let session_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    let session = Session {
        id: session_id.clone(),
        name: name.clone(),
        status: "running".to_string(),
        working_dir: working_dir.clone(),
        created_at: now.clone(),
        updated_at: now,
    };

    db.create_session(&session).map_err(|e| format!("Failed to create session: {}", e))?;

    app.emit("session-started", &session_id).map_err(|e| e.to_string())?;

    let session_id_clone = session_id.clone();
    let app_clone = app.clone();
    let output_emitter = Arc::new(move |line: String| {
        let _ = app_clone
            .emit("session-output", SessionOutput { session_id: session_id_clone.clone(), line });
    });

    let child = cli.spawn_with_output(&prompt, &working_dir, output_emitter).await?;

    let child_handle = Arc::new(Mutex::new(child));
    let killed = Arc::new(Mutex::new(false));
    let session_handle = SessionHandle {
        id: session_id.clone(),
        name: name.clone(),
        child: child_handle.clone(),
        killed: killed.clone(),
    };

    {
        let manager = manager.lock().await;
        let mut sessions = manager.sessions.lock().await;
        sessions.insert(session_id.clone(), session_handle);
    }

    let manager_clone = manager.inner().clone();
    let app_clone = app.clone();
    let session_id_clone = session_id.clone();
    let child_handle_clone = child_handle.clone();

    tokio::spawn(async move {
        loop {
            let status = {
                let mut child = child_handle_clone.lock().await;
                child.try_wait()
            };

            match status {
                Ok(Some(_)) => {
                    let _ = app_clone.emit("session-complete", &session_id_clone);
                    let manager = manager_clone.lock().await;
                    let mut sessions = manager.sessions.lock().await;
                    sessions.remove(&session_id_clone);
                    break;
                }
                Ok(None) => {
                    sleep(Duration::from_millis(200)).await;
                }
                Err(e) => {
                    let _ = app_clone.emit("session-error", (&session_id_clone, e.to_string()));
                    let manager = manager_clone.lock().await;
                    let mut sessions = manager.sessions.lock().await;
                    sessions.remove(&session_id_clone);
                    break;
                }
            }
        }
    });

    Ok(session_id)
}

#[tauri::command]
pub async fn list_sessions(db: State<'_, Database>) -> Result<Vec<Session>, String> {
    db.list_sessions().map_err(|e| format!("Failed to list sessions: {}", e))
}

#[tauri::command]
pub async fn get_session(db: State<'_, Database>, id: String) -> Result<Option<Session>, String> {
    db.get_session(&id).map_err(|e| format!("Failed to get session: {}", e))
}

#[tauri::command]
pub async fn kill_session(
    manager: State<'_, Arc<Mutex<SessionManager>>>,
    db: State<'_, Database>,
    id: String,
) -> Result<(), String> {
    db.update_session_status(&id, "killed").map_err(|e| e.to_string())?;

    let manager = manager.lock().await;
    let sessions = manager.sessions.lock().await;

    if let Some(handle) = sessions.get(&id) {
        let mut killed = handle.killed.lock().await;
        if !*killed {
            let mut child = handle.child.lock().await;
            let _ = child.kill().await;
            *killed = true;
        }
    }

    Ok(())
}

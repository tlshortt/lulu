use crate::db::{Database, Session};
use crate::session::manager::SessionHandle;
use crate::session::{ClaudeCli, SessionManager};
use crate::session::{SessionEvent, SessionEventPayload};
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};
use tokio::sync::mpsc;
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
    cli_path_override: Option<String>,
) -> Result<String, String> {
    let cli_override_path =
        cli_path_override.filter(|value| !value.trim().is_empty()).map(PathBuf::from);
    let cli = ClaudeCli::find_with_override(cli_override_path)?;

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

    let (event_tx, mut event_rx) = mpsc::channel::<SessionEvent>(256);

    let child = match cli.spawn_with_events(&prompt, &working_dir, &session_id, event_tx).await {
        Ok(child) => child,
        Err(err) => {
            let _ = app.emit("session-error", (&session_id, err.clone()));
            return Err(err);
        }
    };

    let app_event = app.clone();
    let session_id_for_events = session_id.clone();
    tokio::spawn(async move {
        while let Some(event) = event_rx.recv().await {
            let frontend_event = to_frontend_session_event(&event);
            let _ = app_event.emit("session-event", frontend_event);

            match &event.payload {
                SessionEventPayload::Message { content } => {
                    let _ = app_event.emit(
                        "session-output",
                        SessionOutput {
                            session_id: event.session_id.clone(),
                            line: content.clone(),
                        },
                    );
                }
                SessionEventPayload::Status { status } => {
                    if status == "done" || status == "complete" {
                        let _ = app_event.emit("session-complete", &session_id_for_events);
                    }
                }
                SessionEventPayload::Error { message } => {
                    let _ = app_event.emit("session-error", (&event.session_id, message));
                }
                _ => {}
            }
        }
    });

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

fn to_frontend_session_event(event: &SessionEvent) -> serde_json::Value {
    match &event.payload {
        SessionEventPayload::Message { content } => {
            json!({
                "type": "message",
                "data": {
                    "session_id": &event.session_id,
                    "seq": event.seq,
                    "timestamp": &event.timestamp,
                    "content": content,
                    "complete": true
                }
            })
        }
        SessionEventPayload::ToolCall { tool_name, args } => {
            json!({
                "type": "tool_call",
                "data": {
                    "session_id": &event.session_id,
                    "seq": event.seq,
                    "timestamp": &event.timestamp,
                    "tool_name": tool_name,
                    "args": args
                }
            })
        }
        SessionEventPayload::ToolResult { tool_name, result } => {
            json!({
                "type": "tool_result",
                "data": {
                    "session_id": &event.session_id,
                    "seq": event.seq,
                    "timestamp": &event.timestamp,
                    "tool_name": tool_name,
                    "result": result
                }
            })
        }
        SessionEventPayload::Status { status } => {
            json!({
                "type": "status",
                "data": {
                    "session_id": &event.session_id,
                    "seq": event.seq,
                    "timestamp": &event.timestamp,
                    "status": status
                }
            })
        }
        SessionEventPayload::Error { message } => {
            json!({
                "type": "error",
                "data": {
                    "session_id": &event.session_id,
                    "seq": event.seq,
                    "timestamp": &event.timestamp,
                    "error": message
                }
            })
        }
    }
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

use crate::db::{Database, Session};
use crate::session::manager::SessionHandle;
use crate::session::{ClaudeCli, SessionManager};
use crate::session::{SessionEvent, SessionEventPayload};
use serde_json::json;
use std::path::Path;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::mpsc;
use tokio::sync::Mutex;

const TERMINAL_STATUSES: [&str; 3] = ["completed", "failed", "killed"];

#[derive(Clone, serde::Serialize)]
struct SessionOutput {
    session_id: String,
    line: String,
}

fn validate_working_dir(working_dir: &str) -> Result<(), String> {
    let path = Path::new(working_dir);
    if !path.exists() {
        return Err(format!("Working directory does not exist: {}", working_dir));
    }
    if !path.is_dir() {
        return Err(format!("Working directory is not a directory: {}", working_dir));
    }
    Ok(())
}

fn is_terminal_status(status: &str) -> bool {
    TERMINAL_STATUSES.contains(&status)
}

async fn remove_session_handle(
    manager: &Arc<Mutex<SessionManager>>,
    session_id: &str,
) -> Result<(), String> {
    let manager = manager.lock().await;
    let mut sessions = manager.sessions.lock().await;
    sessions.remove(session_id);
    Ok(())
}

async fn finalize_session_once(
    app: &AppHandle,
    manager: &Arc<Mutex<SessionManager>>,
    session_id: &str,
    status: &str,
    seq: &Arc<AtomicU64>,
    terminal_emitted: &Arc<AtomicBool>,
    emit_structured_status: bool,
    failure_message: Option<String>,
) {
    if terminal_emitted.swap(true, Ordering::SeqCst) {
        return;
    }

    let final_status = if status == "complete" || status == "done" {
        "completed"
    } else {
        status
    };

    let db = app.state::<Database>();
    if is_terminal_status(final_status) {
        let _ = db.transition_session_terminal(session_id, final_status);
    } else {
        let _ = db.update_session_status(session_id, final_status);
    }

    if emit_structured_status {
        let status_event = json!({
            "type": "status",
            "data": {
                "session_id": session_id,
                "seq": seq.fetch_add(1, Ordering::SeqCst),
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "status": final_status,
                "message": failure_message
            }
        });
        let _ = app.emit("session-event", status_event);
    }

    if final_status == "completed" {
        let _ = app.emit("session-complete", session_id);
    }

    if final_status == "failed" {
        let message = failure_message.unwrap_or_else(|| "Session failed".to_string());
        let _ = app.emit("session-error", (session_id, message));
    }

    let _ = remove_session_handle(manager, session_id).await;
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
    validate_working_dir(&working_dir)?;

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

    let spawned = match cli.spawn_with_events(&prompt, &working_dir, &session_id, event_tx).await {
        Ok(spawned) => spawned,
        Err(err) => {
            let _ = db.update_session_status(&session_id, "failed");
            let _ = app.emit("session-error", (&session_id, err.clone()));
            return Err(err);
        }
    };

    let sequence = spawned.seq.clone();
    let terminal_emitted = Arc::new(AtomicBool::new(false));

    let app_event = app.clone();
    let session_id_for_events = session_id.clone();
    let manager_for_events = manager.inner().clone();
    let seq_for_events = sequence.clone();
    let terminal_for_events = terminal_emitted.clone();
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
                    if is_terminal_status(status) {
                        finalize_session_once(
                            &app_event,
                            &manager_for_events,
                            &session_id_for_events,
                            status,
                            &seq_for_events,
                            &terminal_for_events,
                            false,
                            None,
                        )
                        .await;
                    }
                }
                SessionEventPayload::Error { message } => {
                    let _ = app_event.emit("session-error", (&event.session_id, message));
                }
                _ => {}
            }
        }
    });

    let child_handle = Arc::new(Mutex::new(spawned.child));
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
    let seq_clone = sequence.clone();
    let terminal_emitted_clone = terminal_emitted.clone();
    let killed_clone = killed.clone();

    tokio::spawn(async move {
        let wait_result = {
            let mut child = child_handle_clone.lock().await;
            child.wait().await
        };

        match wait_result {
            Ok(exit_status) => {
                let was_killed = {
                    let killed = killed_clone.lock().await;
                    *killed
                };
                let terminal = if was_killed {
                    "killed"
                } else if exit_status.success() {
                    "completed"
                } else {
                    "failed"
                };

                finalize_session_once(
                    &app_clone,
                    &manager_clone,
                    &session_id_clone,
                    terminal,
                    &seq_clone,
                    &terminal_emitted_clone,
                    true,
                    None,
                )
                .await;
            }
            Err(e) => {
                let message = format!("Failed waiting for session process: {}", e);
                finalize_session_once(
                    &app_clone,
                    &manager_clone,
                    &session_id_clone,
                    "failed",
                    &seq_clone,
                    &terminal_emitted_clone,
                    true,
                    Some(message),
                )
                .await;
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
        SessionEventPayload::Thinking { content } => {
            json!({
                "type": "thinking",
                "data": {
                    "session_id": &event.session_id,
                    "seq": event.seq,
                    "timestamp": &event.timestamp,
                    "content": content,
                }
            })
        }
        SessionEventPayload::ToolCall {
            call_id,
            tool_name,
            args,
        } => {
            json!({
                "type": "tool_call",
                "data": {
                    "session_id": &event.session_id,
                    "seq": event.seq,
                    "timestamp": &event.timestamp,
                    "call_id": call_id,
                    "tool_name": tool_name,
                    "args": args
                }
            })
        }
        SessionEventPayload::ToolResult {
            call_id,
            tool_name,
            result,
        } => {
            json!({
                "type": "tool_result",
                "data": {
                    "session_id": &event.session_id,
                    "seq": event.seq,
                    "timestamp": &event.timestamp,
                    "call_id": call_id,
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

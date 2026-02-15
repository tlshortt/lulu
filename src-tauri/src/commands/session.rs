use crate::db::{Database, Session, SessionMessage};
use crate::session::manager::SessionHandle;
use crate::session::projection::normalize_failure_reason;
use crate::session::{ClaudeCli, SessionManager, WorktreeService};
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

fn resolve_working_dir(working_dir: &str) -> Result<String, String> {
    let trimmed = working_dir.trim();

    if trimmed == "~" || trimmed.starts_with("~/") {
        let home = std::env::var("HOME")
            .map_err(|_| "HOME is not set; cannot resolve '~' working directory".to_string())?;

        if trimmed == "~" {
            return Ok(home);
        }

        let suffix = trimmed.trim_start_matches("~/");
        return Ok(format!("{}/{}", home, suffix));
    }

    Ok(trimmed.to_string())
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

    let _ = db.update_last_activity(session_id, &chrono::Utc::now().to_rfc3339());
    if final_status == "failed" || final_status == "killed" {
        let _ = db.update_failure_reason(
            session_id,
            normalize_failure_reason(failure_message.as_deref()).as_deref(),
        );
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
    let working_dir = resolve_working_dir(&working_dir)?;
    validate_working_dir(&working_dir)?;
    let worktree_service = WorktreeService::from_working_dir(&working_dir)?;

    let cli_override_path =
        cli_path_override.filter(|value| !value.trim().is_empty()).map(PathBuf::from);
    let cli = ClaudeCli::find_with_override(cli_override_path)?;

    let spawn_args = vec![
        "-p".to_string(),
        "<prompt redacted>".to_string(),
        "--verbose".to_string(),
        "--output-format".to_string(),
        "stream-json".to_string(),
    ];

    let session_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let worktree_path = worktree_service.create_worktree(&session_id)?;
    let worktree_path_str = worktree_path.display().to_string();

    let session = Session {
        id: session_id.clone(),
        name: name.clone(),
        status: "starting".to_string(),
        working_dir: working_dir.clone(),
        created_at: now.clone(),
        updated_at: now,
    };

    db.create_session(&session).map_err(|e| format!("Failed to create session: {}", e))?;
    db.update_worktree_path(&session_id, Some(&worktree_path_str))
        .map_err(|e| format!("Failed to persist session worktree path: {}", e))?;

    let _ = app.emit(
        "session-debug",
        json!({
            "session_id": session_id.clone(),
            "kind": "spawn",
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "cli_path": cli.path.display().to_string(),
            "args": spawn_args,
            "working_dir": working_dir.clone(),
            "worktree_path": worktree_path_str,
        }),
    );

    app.emit("session-started", &session_id).map_err(|e| e.to_string())?;

    let (event_tx, mut event_rx) = mpsc::channel::<SessionEvent>(256);

    let worktree_dir = worktree_path.display().to_string();
    let spawned = match cli
        .spawn_with_events(&prompt, &worktree_dir, &session_id, event_tx)
        .await
    {
        Ok(spawned) => spawned,
        Err(err) => {
            let _ = db.update_session_status(&session_id, "failed");
            let _ = db
                .update_failure_reason(
                    &session_id,
                    normalize_failure_reason(Some(&err)).as_deref(),
                );
            let _ = worktree_service.remove_worktree_for_session(&session_id);
            let _ = worktree_service.prune_worktrees();
            let _ = app.emit("session-error", (&session_id, err.clone()));
            return Err(err);
        }
    };

    let _ = db.update_session_status(&session_id, "running");
    let _ = db.update_last_activity(&session_id, &chrono::Utc::now().to_rfc3339());

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
                    let _ = app_event
                        .state::<Database>()
                        .update_last_activity(&event.session_id, &event.timestamp);
                    let _ = app_event.state::<Database>().insert_session_message(
                        &event.session_id,
                        "assistant",
                        content,
                        &event.timestamp,
                    );
                    let _ = app_event.emit(
                        "session-output",
                        SessionOutput {
                            session_id: event.session_id.clone(),
                            line: content.clone(),
                        },
                    );
                }
                SessionEventPayload::Status { status } => {
                    let _ = app_event
                        .state::<Database>()
                        .update_last_activity(&event.session_id, &event.timestamp);
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
                    let _ = app_event
                        .state::<Database>()
                        .update_failure_reason(
                            &event.session_id,
                            normalize_failure_reason(Some(message)).as_deref(),
                        );
                    let _ = app_event.emit(
                        "session-debug",
                        json!({
                            "session_id": event.session_id,
                            "kind": "stderr",
                            "timestamp": chrono::Utc::now().to_rfc3339(),
                            "message": message,
                        }),
                    );
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
pub async fn rename_session(
    db: State<'_, Database>,
    id: String,
    name: String,
) -> Result<(), String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("Session name cannot be empty".to_string());
    }

    db.update_session_name(&id, trimmed)
        .map_err(|e| format!("Failed to rename session: {}", e))
}

#[tauri::command]
pub async fn list_session_messages(
    db: State<'_, Database>,
    id: String,
) -> Result<Vec<SessionMessage>, String> {
    db.list_session_messages(&id)
        .map_err(|e| format!("Failed to list session messages: {}", e))
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

#[tauri::command]
pub async fn delete_session(
    manager: State<'_, Arc<Mutex<SessionManager>>>,
    db: State<'_, Database>,
    id: String,
) -> Result<(), String> {
    let session = db
        .get_session(&id)
        .map_err(|e| format!("Failed to get session before delete: {}", e))?;
    let worktree_path = db
        .get_session_worktree_path(&id)
        .map_err(|e| format!("Failed to get session worktree path: {}", e))?;

    let session_handle = {
        let manager = manager.lock().await;
        let mut sessions = manager.sessions.lock().await;
        sessions.remove(&id)
    };

    if let Some(handle) = session_handle {
        let mut child = handle.child.lock().await;
        let _ = child.kill().await;
    }

    if let (Some(session_record), Some(path)) = (session, worktree_path) {
        if let Ok(worktree_service) = WorktreeService::from_working_dir(&session_record.working_dir) {
            let _ = worktree_service.remove_worktree_at_path(Path::new(&path), true);
            let _ = worktree_service.prune_worktrees();
        }
    }

    db.delete_session(&id)
        .map_err(|e| format!("Failed to delete session: {}", e))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::resolve_working_dir;

    #[test]
    fn resolve_working_dir_expands_home_alias() {
        let home = std::env::var("HOME").expect("HOME should be set in test environment");
        let resolved_home = resolve_working_dir("~").expect("tilde should resolve");
        let resolved_subdir = resolve_working_dir("~/workspace").expect("tilde path should resolve");

        assert_eq!(resolved_home, home);
        assert_eq!(resolved_subdir, format!("{}/workspace", home));
    }

    #[test]
    fn resolve_working_dir_trims_non_tilde_paths() {
        let resolved = resolve_working_dir("  /tmp/project  ").expect("path should resolve");
        assert_eq!(resolved, "/tmp/project");
    }
}

use crate::db::{Database, Session, SessionDashboardRow, SessionMessage};
use crate::session::projection::{normalize_failure_reason, project_dashboard_row, DashboardSessionProjection};
use crate::session::{ClaudeCli, SessionManager, SessionSupervisor, WorktreeService};
use crate::session::{SessionEvent, SessionEventPayload};
use serde_json::json;
use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;
use std::sync::atomic::AtomicU64;
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

fn resolve_execution_dir_with_worktree(
    working_dir: &str,
    session_id: &str,
) -> (Option<WorktreeService>, Option<PathBuf>, String, Option<String>) {
    match WorktreeService::from_working_dir(working_dir) {
        Ok(service) => match service.create_worktree(session_id) {
            Ok(path) => {
                let execution_dir = path.display().to_string();
                (Some(service), Some(path), execution_dir, None)
            }
            Err(err) => (
                None,
                None,
                working_dir.to_string(),
                Some(format!(
                    "Worktree creation failed, using working directory directly: {}",
                    err
                )),
            ),
        },
        Err(err) => (
            None,
            None,
            working_dir.to_string(),
            Some(format!(
                "No git repository detected, using working directory directly: {}",
                err
            )),
        ),
    }
}

fn is_terminal_status(status: &str) -> bool {
    TERMINAL_STATUSES.contains(&status)
}

fn project_dashboard_rows(rows: Vec<SessionDashboardRow>) -> Vec<DashboardSessionProjection> {
    rows.into_iter().map(project_dashboard_row).collect()
}

async fn session_supervisor(manager: &Arc<Mutex<SessionManager>>) -> Arc<SessionSupervisor> {
    let manager = manager.lock().await;
    manager.supervisor.clone()
}

async fn finalize_session_once(
    app: &AppHandle,
    manager: &Arc<Mutex<SessionManager>>,
    session_id: &str,
    status: &str,
    seq: &Arc<AtomicU64>,
    emit_structured_status: bool,
    failure_message: Option<String>,
) {
    let supervisor = session_supervisor(manager).await;
    let db = app.state::<Database>();
    let transition = match supervisor
        .finalize_terminal_transition_and_emit(
            app,
            db.inner(),
            session_id,
            status,
            seq.as_ref(),
            failure_message,
            emit_structured_status,
        )
        .await
    {
        Ok(result) => result,
        Err(_) => None,
    };

    let Some(transition) = transition else {
        return;
    };
    let final_status = transition.final_status;
    let failure_message = transition.failure_message;

    if final_status == "completed" {
        let _ = app.emit("session-complete", session_id);
    }

    if final_status == "failed" {
        let message = failure_message
            .clone()
            .unwrap_or_else(|| "Session failed".to_string());
        let _ = app.emit("session-error", (session_id, message));
    }

    let _ = supervisor.remove(session_id).await;
}

pub fn reconcile_sessions_on_startup(db: &Database) -> Result<(), String> {
    let stale_reason =
        "Session was still in progress at previous shutdown and was marked failed on restart";
    let normalized_reason = normalize_failure_reason(Some(stale_reason)).unwrap_or_else(|| {
        "Session was still in progress at previous shutdown".to_string()
    });

    db.reconcile_stale_inflight_sessions(&normalized_reason)
        .map_err(|e| format!("Failed to reconcile stale sessions: {}", e))?;

    let sessions = db
        .list_sessions()
        .map_err(|e| format!("Failed to list sessions for worktree reconciliation: {}", e))?;

    let mut expected_by_repo: HashMap<PathBuf, Vec<PathBuf>> = HashMap::new();
    for session in sessions {
        let worktree_path = db
            .get_session_worktree_path(&session.id)
            .map_err(|e| format!("Failed to fetch worktree metadata for session {}: {}", session.id, e))?;

        let Some(worktree_path) = worktree_path else {
            continue;
        };

        let service = match WorktreeService::from_working_dir(&session.working_dir) {
            Ok(service) => service,
            Err(_) => continue,
        };

        expected_by_repo
            .entry(service.repo_root().to_path_buf())
            .or_default()
            .push(PathBuf::from(worktree_path));
    }

    for (repo_root, expected_paths) in expected_by_repo {
        let service = WorktreeService::new(repo_root);
        service.reconcile_managed_worktrees(&expected_paths)?;
    }

    Ok(())
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
    let session_id = uuid::Uuid::new_v4().to_string();
    let (worktree_service, worktree_path, execution_dir, fallback_message) =
        resolve_execution_dir_with_worktree(&working_dir, &session_id);

    if let Some(message) = fallback_message {
        let _ = app.emit(
            "session-debug",
            json!({
                "session_id": session_id.clone(),
                "kind": "worktree-fallback",
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "working_dir": working_dir.clone(),
                "message": message,
            }),
        );
    }

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

    let now = chrono::Utc::now().to_rfc3339();
    let worktree_path_str = worktree_path.as_ref().map(|path| path.display().to_string());

    let session = Session {
        id: session_id.clone(),
        name: name.clone(),
        status: "starting".to_string(),
        working_dir: working_dir.clone(),
        created_at: now.clone(),
        updated_at: now,
    };

    db.create_session(&session).map_err(|e| format!("Failed to create session: {}", e))?;
    db.update_worktree_path(&session_id, worktree_path_str.as_deref())
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

    let spawned = match cli
        .spawn_with_events(&prompt, &execution_dir, &session_id, event_tx)
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
            if let Some(worktree_service) = &worktree_service {
                let _ = worktree_service.remove_worktree_for_session(&session_id);
                let _ = worktree_service.prune_worktrees();
            }
            let _ = app.emit("session-error", (&session_id, err.clone()));
            return Err(err);
        }
    };

    let _ = db.update_session_status(&session_id, "running");
    let _ = db.update_last_activity(&session_id, &chrono::Utc::now().to_rfc3339());

    let sequence = spawned.seq.clone();
    let supervisor = session_supervisor(manager.inner()).await;
    let runtime = supervisor
        .register(session_id.clone(), name.clone(), spawned.child)
        .await;

    let app_event = app.clone();
    let session_id_for_events = session_id.clone();
    let manager_for_events = manager.inner().clone();
    let seq_for_events = sequence.clone();
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

    let manager_clone = manager.inner().clone();
    let app_clone = app.clone();
    let session_id_clone = session_id.clone();
    let runtime_for_wait = runtime.clone();
    let seq_clone = sequence.clone();

    tokio::spawn(async move {
        let wait_result = {
            let mut child = runtime_for_wait.child.lock().await;
            child.wait().await
        };

        match wait_result {
            Ok(exit_status) => {
                let was_killed = runtime_for_wait.was_killed();
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
pub async fn list_dashboard_sessions(
    db: State<'_, Database>,
) -> Result<Vec<DashboardSessionProjection>, String> {
    db.list_dashboard_sessions()
        .map(project_dashboard_rows)
        .map_err(|e| format!("Failed to list dashboard sessions: {}", e))
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
    id: String,
) -> Result<(), String> {
    let supervisor = session_supervisor(manager.inner()).await;
    let _ = supervisor.kill_session(&id).await;

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

    let supervisor = session_supervisor(manager.inner()).await;
    if let Some(runtime) = supervisor.remove(&id).await {
        runtime.mark_killed();
        let mut child = runtime.child.lock().await;
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
    use super::{
        project_dashboard_rows, resolve_execution_dir_with_worktree, resolve_working_dir,
    };
    use crate::db::SessionDashboardRow;
    use crate::session::projection::DASHBOARD_STATUS_FAILED;
    use tempfile::tempdir;

    fn run_git(repo_path: &std::path::Path, args: &[&str]) {
        let output = std::process::Command::new("git")
            .args(args)
            .current_dir(repo_path)
            .output()
            .expect("git command should execute");

        assert!(
            output.status.success(),
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&output.stderr)
        );
    }

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

    #[test]
    fn list_dashboard_projection_uses_locked_projection_boundary() {
        let rows = vec![SessionDashboardRow {
            id: "session-1".to_string(),
            name: "session".to_string(),
            status: "killed".to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
            last_activity_at: None,
            failure_reason: Some("  runtime\nerror ".to_string()),
            worktree_path: None,
        }];

        let projected = project_dashboard_rows(rows);
        assert_eq!(projected[0].status, DASHBOARD_STATUS_FAILED);
        assert_eq!(projected[0].failure_reason.as_deref(), Some("runtime error"));
    }

    #[test]
    fn resolve_execution_dir_falls_back_for_non_git_folder() {
        let temp = tempdir().expect("tempdir should be created");
        let working_dir = temp.path().display().to_string();

        let (service, worktree_path, execution_dir, fallback_message) =
            resolve_execution_dir_with_worktree(&working_dir, "session-non-git");

        assert!(service.is_none());
        assert!(worktree_path.is_none());
        assert_eq!(execution_dir, working_dir);
        assert!(
            fallback_message
                .as_deref()
                .unwrap_or_default()
                .contains("No git repository detected"),
            "expected explicit non-git fallback message"
        );
    }

    #[test]
    fn resolve_execution_dir_prefers_git_worktree_when_repo_ready() {
        let temp = tempdir().expect("tempdir should be created");
        run_git(temp.path(), &["init", "--initial-branch=main"]);
        run_git(temp.path(), &["config", "user.name", "Lulu Test"]);
        run_git(temp.path(), &["config", "user.email", "lulu@example.com"]);
        std::fs::write(temp.path().join("README.md"), "# test\n").expect("seed file should write");
        run_git(temp.path(), &["add", "README.md"]);
        run_git(temp.path(), &["commit", "-m", "initial"]);

        let working_dir = temp.path().display().to_string();
        let session_id = "session-git";
        let (service, worktree_path, execution_dir, fallback_message) =
            resolve_execution_dir_with_worktree(&working_dir, session_id);

        assert!(fallback_message.is_none());
        let service = service.expect("expected worktree service for git repo");
        let worktree_path = worktree_path.expect("expected created worktree path");
        assert!(worktree_path.exists());
        assert!(worktree_path.ends_with(session_id));
        assert_eq!(execution_dir, worktree_path.display().to_string());

        service
            .remove_worktree_for_session(session_id)
            .expect("worktree cleanup should succeed");
        service.prune_worktrees().expect("worktree prune should succeed");
    }

    #[test]
    fn resolve_execution_dir_falls_back_when_worktree_creation_fails() {
        let temp = tempdir().expect("tempdir should be created");
        run_git(temp.path(), &["init", "--initial-branch=main"]);

        let working_dir = temp.path().display().to_string();
        let (service, worktree_path, execution_dir, fallback_message) =
            resolve_execution_dir_with_worktree(&working_dir, "session-no-head");

        assert!(service.is_none());
        assert!(worktree_path.is_none());
        assert_eq!(execution_dir, working_dir);
        assert!(
            fallback_message
                .as_deref()
                .unwrap_or_default()
                .contains("Worktree creation failed"),
            "expected explicit worktree creation fallback message"
        );
    }
}

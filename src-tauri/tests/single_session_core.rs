use std::path::PathBuf;

use tempfile::tempdir;
use tokio::sync::mpsc;
use tokio::time::{timeout, Duration};

use tauri_app_lib::db::{init_database, Session};
use tauri_app_lib::session::{ClaudeCli, SessionEvent, SessionEventPayload};

fn collect_terminal_status(events: &[SessionEvent]) -> Vec<String> {
    events
        .iter()
        .filter_map(|event| match &event.payload {
            SessionEventPayload::Status { status }
                if status == "completed" || status == "failed" || status == "killed" =>
            {
                Some(status.clone())
            }
            _ => None,
        })
        .collect()
}

#[tokio::test]
async fn single_session_launch_stream_and_terminal_persist_success() {
    let temp = tempdir().expect("tempdir should be created");
    let db_path = temp.path().join("lulu.db");
    let db = init_database(&db_path).expect("database should initialize");

    let session_id = "session-success".to_string();
    let now = chrono::Utc::now().to_rfc3339();
    db.create_session(&Session {
        id: session_id.clone(),
        name: "Success session".to_string(),
        status: "running".to_string(),
        working_dir: temp.path().display().to_string(),
        created_at: now.clone(),
        updated_at: now,
    })
    .expect("running session should persist");

    let cli = ClaudeCli::find_with_override(Some(PathBuf::from(env!("CARGO_BIN_EXE_lulu_test_cli"))))
        .expect("fixture cli should resolve");
    let (tx, mut rx) = mpsc::channel(128);

    let mut spawned = cli
        .spawn_with_events("success", temp.path().to_str().unwrap_or("."), &session_id, tx)
        .await
        .expect("spawn should succeed");

    timeout(Duration::from_secs(5), spawned.child.wait())
        .await
        .expect("fixture should exit")
        .expect("wait should succeed");

    let mut events = Vec::new();
    loop {
        match timeout(Duration::from_millis(300), rx.recv()).await {
            Ok(Some(event)) => events.push(event),
            _ => break,
        }
    }

    assert!(!events.is_empty(), "expected streamed events");
    assert!(
        events.iter().any(|event| matches!(
            event.payload,
            SessionEventPayload::Message { .. }
                | SessionEventPayload::Thinking { .. }
                | SessionEventPayload::ToolCall { .. }
                | SessionEventPayload::ToolResult { .. }
        )),
        "expected typed message/thinking/tool events"
    );

    let terminal_statuses = collect_terminal_status(&events);
    assert_eq!(terminal_statuses.len(), 1, "terminal status must emit once");
    assert_eq!(terminal_statuses[0], "completed");

    db.update_session_status(&session_id, &terminal_statuses[0])
        .expect("terminal status update should persist");
    let stored = db
        .get_session(&session_id)
        .expect("query should succeed")
        .expect("session should exist");
    assert_eq!(stored.status, "completed");
}

#[tokio::test]
async fn single_session_failure_stream_persists_failed_once() {
    let temp = tempdir().expect("tempdir should be created");
    let db_path = temp.path().join("lulu.db");
    let db = init_database(&db_path).expect("database should initialize");

    let session_id = "session-failure".to_string();
    let now = chrono::Utc::now().to_rfc3339();
    db.create_session(&Session {
        id: session_id.clone(),
        name: "Failure session".to_string(),
        status: "running".to_string(),
        working_dir: temp.path().display().to_string(),
        created_at: now.clone(),
        updated_at: now,
    })
    .expect("running session should persist");

    let cli = ClaudeCli::find_with_override(Some(PathBuf::from(env!("CARGO_BIN_EXE_lulu_test_cli"))))
        .expect("fixture cli should resolve");
    let (tx, mut rx) = mpsc::channel(128);

    let mut spawned = cli
        .spawn_with_events("fail", temp.path().to_str().unwrap_or("."), &session_id, tx)
        .await
        .expect("spawn should succeed");

    let exit = timeout(Duration::from_secs(5), spawned.child.wait())
        .await
        .expect("fixture should exit")
        .expect("wait should succeed");
    assert!(!exit.success(), "failure fixture should exit non-zero");

    let mut events = Vec::new();
    loop {
        match timeout(Duration::from_millis(300), rx.recv()).await {
            Ok(Some(event)) => events.push(event),
            _ => break,
        }
    }

    let terminal_statuses = collect_terminal_status(&events);
    assert_eq!(terminal_statuses.len(), 1, "terminal status must emit once");
    assert_eq!(terminal_statuses[0], "failed");

    db.update_session_status(&session_id, &terminal_statuses[0])
        .expect("terminal status update should persist");
    let stored = db
        .get_session(&session_id)
        .expect("query should succeed")
        .expect("session should exist");
    assert_eq!(stored.status, "failed");
}

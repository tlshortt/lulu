use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;

use serde_json::json;
use tempfile::tempdir;
use tokio::sync::Mutex;
use tokio::time::{sleep, timeout, Duration};

use tauri_app_lib::db::{init_database, Database, Session};
use tauri_app_lib::session::{ClaudeCli, SessionEventPayload, SessionSupervisor};

#[derive(Clone)]
struct SessionSpec {
    id: String,
    prompt: String,
    expect_terminal: &'static str,
}

fn is_terminal_status(status: &str) -> bool {
    matches!(status, "completed" | "failed" | "killed" | "interrupted")
}

async fn reduce_terminal_once(
    db: Arc<Database>,
    supervisor: Arc<SessionSupervisor>,
    session_id: &str,
    status: &str,
    failure_reason: Option<String>,
    finalize_counts: Arc<Mutex<HashMap<String, usize>>>,
) {
    let transition = supervisor
        .finalize_terminal_transition(db.as_ref(), session_id, status, failure_reason)
        .await
        .expect("supervisor transition should not fail");

    if transition.is_none() {
        return;
    }

    {
        let mut counts = finalize_counts.lock().await;
        let counter = counts.entry(session_id.to_string()).or_insert(0);
        *counter += 1;
    }

    let _ = supervisor.remove(session_id).await;
}

async fn run_one_session(
    cli: ClaudeCli,
    db: Arc<Database>,
    supervisor: Arc<SessionSupervisor>,
    spec: SessionSpec,
    work_dir: String,
    finalize_counts: Arc<Mutex<HashMap<String, usize>>>,
) -> Result<(), String> {
    let (event_tx, mut event_rx) = tokio::sync::mpsc::channel(128);
    let spawned = cli
        .spawn_with_events(&spec.prompt, &work_dir, &spec.id, event_tx)
        .await?;

    let runtime = supervisor
        .register(spec.id.clone(), spec.id.clone(), spawned.child)
        .await;

    let db_for_events = db.clone();
    let supervisor_for_events = supervisor.clone();
    let finalize_counts_for_events = finalize_counts.clone();
    let id_for_events = spec.id.clone();
    let event_task = tokio::spawn(async move {
        while let Some(event) = event_rx.recv().await {
            if let SessionEventPayload::Status { status } = event.payload {
                if is_terminal_status(&status) {
                    reduce_terminal_once(
                        db_for_events.clone(),
                        supervisor_for_events.clone(),
                        &id_for_events,
                        &status,
                        None,
                        finalize_counts_for_events.clone(),
                    )
                    .await;
                }
            }
        }
    });

    let wait_result = {
        let mut child = runtime.child.lock().await;
        child.wait().await
    };

    match wait_result {
        Ok(exit) => {
            let terminal = if runtime.was_killed() {
                "killed"
            } else if exit.success() {
                "completed"
            } else {
                "failed"
            };
            reduce_terminal_once(
                db,
                supervisor,
                &spec.id,
                terminal,
                None,
                finalize_counts,
            )
            .await;
        }
        Err(err) => {
            let failure = format!("Failed waiting for session process: {}", err);
            reduce_terminal_once(
                db,
                supervisor,
                &spec.id,
                "failed",
                Some(failure),
                finalize_counts,
            )
            .await;
        }
    }

    let _ = timeout(Duration::from_secs(2), event_task).await;
    Ok(())
}

async fn wait_for_runtime_exit(runtime: Arc<tauri_app_lib::session::SessionRuntime>) {
    timeout(Duration::from_secs(2), async {
        loop {
            {
                let mut child = runtime.child.lock().await;
                if child
                    .try_wait()
                    .expect("child status should be queryable")
                    .is_some()
                {
                    break;
                }
            }

            sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("runtime should exit in time");
}

#[tokio::test]
async fn supervisor_terminal_transition_applies_once_per_session() {
    let temp = tempdir().expect("tempdir should be created");
    let db_path = temp.path().join("lulu.db");
    let db = Arc::new(init_database(&db_path).expect("database should initialize"));
    let supervisor = Arc::new(SessionSupervisor::new());
    let finalize_counts = Arc::new(Mutex::new(HashMap::<String, usize>::new()));

    let specs = vec![
        SessionSpec {
            id: "session-a".to_string(),
            prompt: "delay-ms=450".to_string(),
            expect_terminal: "completed",
        },
        SessionSpec {
            id: "session-b".to_string(),
            prompt: "delay-ms=500".to_string(),
            expect_terminal: "completed",
        },
        SessionSpec {
            id: "session-c".to_string(),
            prompt: "fail delay-ms=50".to_string(),
            expect_terminal: "failed",
        },
        SessionSpec {
            id: "session-d".to_string(),
            prompt: "delay-ms=350".to_string(),
            expect_terminal: "completed",
        },
        SessionSpec {
            id: "session-e".to_string(),
            prompt: "delay-ms=400".to_string(),
            expect_terminal: "completed",
        },
    ];

    let cli = ClaudeCli::find_with_override(Some(PathBuf::from(env!("CARGO_BIN_EXE_lulu_test_cli"))))
        .expect("fixture cli should resolve");

    let created_base = chrono::Utc::now();
    for (idx, spec) in specs.iter().enumerate() {
        let created_at = (created_base - chrono::Duration::seconds(idx as i64)).to_rfc3339();
        db.create_session(&Session {
            id: spec.id.clone(),
            name: spec.id.clone(),
            status: "running".to_string(),
            working_dir: temp.path().display().to_string(),
            created_at: created_at.clone(),
            updated_at: created_at,
        })
        .expect("session should persist");
    }

    let expected_order: Vec<String> = specs.iter().map(|spec| spec.id.clone()).collect();

    let mut tasks = Vec::new();
    for spec in specs.clone() {
        let cli_clone = ClaudeCli { path: cli.path.clone() };
        let db_clone = db.clone();
        let supervisor_clone = supervisor.clone();
        let finalize_counts_clone = finalize_counts.clone();
        let work_dir = temp.path().display().to_string();
        tasks.push(tokio::spawn(async move {
            run_one_session(
                cli_clone,
                db_clone,
                supervisor_clone,
                spec,
                work_dir,
                finalize_counts_clone,
            )
            .await
        }));
    }

    timeout(Duration::from_secs(2), async {
        loop {
            let maybe_failed = db
                .get_session("session-c")
                .expect("failed session query should work")
                .expect("failed session should exist");
            if maybe_failed.status == "failed" {
                break;
            }
            sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("one session should fail quickly");

    let session_a = db
        .get_session("session-a")
        .expect("session-a should query")
        .expect("session-a should exist");
    let session_b = db
        .get_session("session-b")
        .expect("session-b should query")
        .expect("session-b should exist");
    assert!(
        session_a.status == "running" || session_b.status == "running",
        "at least one unaffected session should still be running after failure"
    );

    for task in tasks {
        task.await
            .expect("session task should join")
            .expect("session should run");
    }

    for spec in &specs {
        let stored = db
            .get_session(&spec.id)
            .expect("session query should succeed")
            .expect("session should exist");
        assert_eq!(stored.status, spec.expect_terminal, "session {} should finish with expected terminal status", spec.id);
    }

    let counts = finalize_counts.lock().await;
    for spec in &specs {
        let count = counts.get(&spec.id).copied().unwrap_or(0);
        assert_eq!(count, 1, "session {} should transition terminal once", spec.id);
    }
    drop(counts);

    let dashboard_rows = db
        .list_dashboard_sessions()
        .expect("dashboard query should succeed");
    let observed_order: Vec<String> = dashboard_rows.iter().map(|row| row.id.clone()).collect();
    assert_eq!(
        observed_order, expected_order,
        "dashboard ordering should remain based on created_at even after terminal updates"
    );
}

#[tokio::test]
async fn interrupt_isolated_to_target_runtime_and_persists_interrupted_status() {
    let temp = tempdir().expect("tempdir should be created");
    let db_path = temp.path().join("lulu.db");
    let db = Arc::new(init_database(&db_path).expect("database should initialize"));
    let supervisor = Arc::new(SessionSupervisor::new());
    let cli = ClaudeCli::find_with_override(Some(PathBuf::from(env!("CARGO_BIN_EXE_lulu_test_cli"))))
        .expect("fixture cli should resolve");

    let work_dir = temp.path().display().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    db.create_session(&Session {
        id: "interrupt-target".to_string(),
        name: "interrupt-target".to_string(),
        status: "running".to_string(),
        working_dir: work_dir.clone(),
        created_at: now.clone(),
        updated_at: now.clone(),
    })
    .expect("target session should persist");

    db.create_session(&Session {
        id: "interrupt-sibling".to_string(),
        name: "interrupt-sibling".to_string(),
        status: "running".to_string(),
        working_dir: work_dir.clone(),
        created_at: now.clone(),
        updated_at: now,
    })
    .expect("sibling session should persist");

    let (target_tx, _target_rx) = tokio::sync::mpsc::channel(128);
    let target_spawned = cli
        .spawn_with_events("delay-ms=5000", &work_dir, "interrupt-target", target_tx)
        .await
        .expect("target process should spawn");
    let _target_runtime = supervisor
        .register(
            "interrupt-target".to_string(),
            "interrupt-target".to_string(),
            target_spawned.child,
        )
        .await;

    let (sibling_tx, _sibling_rx) = tokio::sync::mpsc::channel(128);
    let sibling_spawned = cli
        .spawn_with_events("delay-ms=5000", &work_dir, "interrupt-sibling", sibling_tx)
        .await
        .expect("sibling process should spawn");
    let sibling_runtime = supervisor
        .register(
            "interrupt-sibling".to_string(),
            "interrupt-sibling".to_string(),
            sibling_spawned.child,
        )
        .await;

    supervisor
        .interrupt_session_with_deadline(db.as_ref(), "interrupt-target", Duration::from_secs(10))
        .await
        .expect("interrupt should succeed");

    let target = db
        .get_session("interrupt-target")
        .expect("target session query should succeed")
        .expect("target session should exist");
    assert_eq!(target.status, "interrupted");

    let sibling = db
        .get_session("interrupt-sibling")
        .expect("sibling session query should succeed")
        .expect("sibling session should exist");
    assert_eq!(sibling.status, "running");
    assert!(supervisor.get("interrupt-sibling").await.is_some());
    assert!(!sibling_runtime.was_interrupt_requested());

    let target_messages = db
        .list_session_messages("interrupt-target")
        .expect("target messages should load");
    assert!(
        target_messages.is_empty(),
        "successful interrupt should not append a timeline-only message"
    );

    supervisor
        .kill_session("interrupt-sibling")
        .await
        .expect("sibling kill should succeed");
    wait_for_runtime_exit(sibling_runtime).await;
    let _ = supervisor
        .finalize_terminal_transition(db.as_ref(), "interrupt-sibling", "killed", None)
        .await
        .expect("sibling finalization should succeed");
    let _ = supervisor.remove("interrupt-sibling").await;
}

#[tokio::test]
async fn interrupt_retries_once_and_times_out_after_total_deadline() {
    let temp = tempdir().expect("tempdir should be created");
    let db_path = temp.path().join("lulu.db");
    let db = Arc::new(init_database(&db_path).expect("database should initialize"));
    let supervisor = Arc::new(SessionSupervisor::new());
    let cli = ClaudeCli::find_with_override(Some(PathBuf::from(env!("CARGO_BIN_EXE_lulu_test_cli"))))
        .expect("fixture cli should resolve");

    let work_dir = temp.path().display().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    db.create_session(&Session {
        id: "interrupt-timeout".to_string(),
        name: "interrupt-timeout".to_string(),
        status: "running".to_string(),
        working_dir: work_dir.clone(),
        created_at: now.clone(),
        updated_at: now,
    })
    .expect("session should persist");

    let (event_tx, _event_rx) = tokio::sync::mpsc::channel(128);
    let spawned = cli
        .spawn_with_events("delay-ms=20000", &work_dir, "interrupt-timeout", event_tx)
        .await
        .expect("process should spawn");
    let runtime = supervisor
        .register(
            "interrupt-timeout".to_string(),
            "interrupt-timeout".to_string(),
            spawned.child,
        )
        .await;

    let lock_runtime = runtime.clone();
    let lock_holder = tokio::spawn(async move {
        let _hold = lock_runtime.child.lock().await;
        sleep(Duration::from_secs(11)).await;
    });

    sleep(Duration::from_millis(100)).await;

    let started = Instant::now();
    let err = supervisor
        .interrupt_session_with_deadline(db.as_ref(), "interrupt-timeout", Duration::from_secs(10))
        .await
        .expect_err("interrupt should time out when runtime lock remains blocked");
    let elapsed = started.elapsed();

    assert!(
        elapsed >= Duration::from_secs(10),
        "timeout should honor 10-second total deadline"
    );
    assert!(err.contains("10 seconds"));
    assert_eq!(runtime.interrupt_attempts(), 2, "interrupt should retry once");

    let stored = db
        .get_session("interrupt-timeout")
        .expect("session query should succeed")
        .expect("session should exist");
    assert_eq!(stored.status, "running", "status should restore after timeout");

    lock_holder.await.expect("lock holder should join");

    supervisor
        .kill_session("interrupt-timeout")
        .await
        .expect("cleanup kill should succeed");
    wait_for_runtime_exit(runtime).await;
    let _ = supervisor
        .finalize_terminal_transition(db.as_ref(), "interrupt-timeout", "killed", None)
        .await
        .expect("cleanup finalization should succeed");
    let _ = supervisor.remove("interrupt-timeout").await;
}

#[tokio::test]
async fn resume_reuses_same_row_updates_metadata_and_keeps_terminal_idempotent() {
    let temp = tempdir().expect("tempdir should be created");
    let db_path = temp.path().join("lulu.db");
    let db = Arc::new(init_database(&db_path).expect("database should initialize"));
    let supervisor = Arc::new(SessionSupervisor::new());
    let finalize_counts = Arc::new(Mutex::new(HashMap::<String, usize>::new()));
    let cli = ClaudeCli::find_with_override(Some(PathBuf::from(env!("CARGO_BIN_EXE_lulu_test_cli"))))
        .expect("fixture cli should resolve");

    let work_dir = temp.path().display().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    db.create_session(&Session {
        id: "resume-session".to_string(),
        name: "resume-session".to_string(),
        status: "completed".to_string(),
        working_dir: work_dir.clone(),
        created_at: now.clone(),
        updated_at: now,
    })
    .expect("session should persist");

    let existing_gate = supervisor
        .acquire_lifecycle_operation("resume-session", "interrupt")
        .expect("initial operation gate should lock");
    let gate_err = match supervisor.acquire_lifecycle_operation("resume-session", "resume") {
        Ok(_) => panic!("second operation should be blocked"),
        Err(err) => err,
    };
    assert!(gate_err.contains("in-progress"));
    drop(existing_gate);

    let resumed_at = chrono::Utc::now().to_rfc3339();
    let run_id = uuid::Uuid::new_v4().to_string();
    let resumed = db
        .begin_resume_attempt("resume-session", &run_id, &resumed_at)
        .expect("resume metadata update should work");
    assert!(resumed);

    let (event_tx, mut event_rx) = tokio::sync::mpsc::channel(128);
    let spawned = cli
        .spawn_resume_with_events("delay-ms=60", &work_dir, "resume-session", event_tx)
        .await
        .expect("resume process should spawn");

    let runtime = supervisor
        .register(
            "resume-session".to_string(),
            "resume-session".to_string(),
            spawned.child,
        )
        .await;

    let stream_events = Arc::new(AtomicUsize::new(0));
    let stream_events_for_task = stream_events.clone();
    let db_for_events = db.clone();
    let supervisor_for_events = supervisor.clone();
    let finalize_counts_for_events = finalize_counts.clone();
    let event_task = tokio::spawn(async move {
        while let Some(event) = event_rx.recv().await {
            stream_events_for_task.fetch_add(1, Ordering::SeqCst);
            if let SessionEventPayload::Status { status } = event.payload {
                if is_terminal_status(&status) {
                    reduce_terminal_once(
                        db_for_events.clone(),
                        supervisor_for_events.clone(),
                        "resume-session",
                        &status,
                        None,
                        finalize_counts_for_events.clone(),
                    )
                    .await;
                }
            }
        }
    });

    let wait_result = {
        let mut child = runtime.child.lock().await;
        child.wait().await
    };

    let terminal = match wait_result {
        Ok(exit) if exit.success() => "completed",
        Ok(_) => "failed",
        Err(_) => "failed",
    };

    reduce_terminal_once(
        db.clone(),
        supervisor.clone(),
        "resume-session",
        terminal,
        None,
        finalize_counts.clone(),
    )
    .await;

    let _ = timeout(Duration::from_secs(2), event_task).await;

    let resumed_session = db
        .get_session("resume-session")
        .expect("session query should succeed")
        .expect("session should exist");
    assert_eq!(resumed_session.status, "completed");

    let metadata = db
        .get_session_run_metadata("resume-session")
        .expect("metadata query should succeed")
        .expect("metadata should exist");
    assert_eq!(metadata.resume_count, 1);
    assert_eq!(metadata.active_run_id.as_deref(), Some(run_id.as_str()));
    assert!(metadata.last_resume_at.is_some());

    let all_sessions = db.list_sessions().expect("session list should succeed");
    let row_count = all_sessions
        .iter()
        .filter(|session| session.id == "resume-session")
        .count();
    assert_eq!(row_count, 1, "resume must reuse the existing session row");

    assert!(
        stream_events.load(Ordering::SeqCst) > 0,
        "resume should continue streaming events"
    );

    let counts = finalize_counts.lock().await;
    assert_eq!(
        counts.get("resume-session").copied().unwrap_or(0),
        1,
        "terminal transition should apply once during resume"
    );
}

#[test]
fn session_history_keeps_deterministic_order_across_resume_runs() {
    let temp = tempdir().expect("tempdir should be created");
    let db_path = temp.path().join("lulu.db");
    let db = init_database(&db_path).expect("database should initialize");

    let created_at = chrono::Utc::now().to_rfc3339();
    db.create_session(&Session {
        id: "history-session".to_string(),
        name: "history-session".to_string(),
        status: "completed".to_string(),
        working_dir: temp.path().display().to_string(),
        created_at: created_at.clone(),
        updated_at: created_at,
    })
    .expect("session should persist");

    let run_a = "run-a";
    let run_b = "run-b";

    db.insert_session_event(
        "history-session",
        run_a,
        1,
        "status",
        &json!({ "status": "running" }),
        "2026-02-18T00:00:01Z",
    )
    .expect("first run status should persist");
    db.insert_session_event(
        "history-session",
        run_a,
        2,
        "message",
        &json!({ "content": "first run output" }),
        "2026-02-18T00:00:02Z",
    )
    .expect("first run message should persist");
    db.insert_session_event(
        "history-session",
        run_b,
        1,
        "status",
        &json!({ "status": "running" }),
        "2026-02-18T00:00:03Z",
    )
    .expect("resume run status should persist");
    db.insert_session_event(
        "history-session",
        run_b,
        2,
        "tool_call",
        &json!({ "tool_name": "apply_patch" }),
        "2026-02-18T00:00:04Z",
    )
    .expect("resume run tool call should persist");

    db.insert_session_event(
        "history-session",
        run_b,
        2,
        "tool_call",
        &json!({ "tool_name": "apply_patch" }),
        "2026-02-18T00:00:04Z",
    )
    .expect("duplicate row should be ignored by unique key");

    let history = db
        .list_session_history("history-session")
        .expect("history query should succeed");

    assert_eq!(history.len(), 4, "history should include all unique rows across runs");
    let run_seq: Vec<(String, i64)> = history
        .iter()
        .map(|event| (event.run_id.clone(), event.seq))
        .collect();
    assert_eq!(
        run_seq,
        vec![
            (run_a.to_string(), 1),
            (run_a.to_string(), 2),
            (run_b.to_string(), 1),
            (run_b.to_string(), 2),
        ],
        "history ordering should remain deterministic across resume attempts"
    );
}

#[tokio::test]
async fn cli_spawn_failure_is_actionable_and_retry_path_recovers() {
    let temp = tempdir().expect("tempdir should be created");
    let cli = ClaudeCli::find_with_override(Some(PathBuf::from(env!("CARGO_BIN_EXE_lulu_test_cli"))))
        .expect("fixture cli should resolve");

    let missing_dir = temp.path().join("does-not-exist");
    let missing_dir_str = missing_dir.display().to_string();
    let (failing_tx, _failing_rx) = tokio::sync::mpsc::channel(16);
    let failing = match cli
        .spawn_with_events("delay-ms=10", &missing_dir_str, "spawn-fail", failing_tx)
        .await
    {
        Ok(_) => panic!("spawn should fail for missing working directory"),
        Err(err) => err,
    };

    assert!(
        failing.contains("Failed to spawn Claude CLI in"),
        "failure should include spawn context"
    );
    assert!(
        failing.contains(&missing_dir_str),
        "failure should include working directory path"
    );

    let valid_dir = temp.path().display().to_string();
    let (retry_tx, _retry_rx) = tokio::sync::mpsc::channel(16);
    let mut spawned = cli
        .spawn_with_events("delay-ms=10", &valid_dir, "spawn-retry", retry_tx)
        .await
        .expect("spawn retry should succeed with valid directory");

    let exit = spawned
        .child
        .wait()
        .await
        .expect("retry process should finish cleanly");
    assert!(exit.success(), "retry process should exit successfully");
}

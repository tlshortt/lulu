use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

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
    matches!(status, "completed" | "failed" | "killed")
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

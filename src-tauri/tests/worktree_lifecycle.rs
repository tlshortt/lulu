use tauri_app_lib::commands::session::reconcile_sessions_on_startup;
use tauri_app_lib::db::{init_database, Session, SessionDashboardRow};
use tauri_app_lib::session::projection::{
    normalize_dashboard_status, project_dashboard_row, DASHBOARD_STATUS_COMPLETED,
    DASHBOARD_STATUS_FAILED, DASHBOARD_STATUS_INTERRUPTED, DASHBOARD_STATUS_RUNNING,
};
use tauri_app_lib::session::WorktreeService;
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

fn init_repo() -> tempfile::TempDir {
    let dir = tempdir().expect("tempdir should be created");
    run_git(dir.path(), &["init", "--initial-branch=main"]);
    run_git(dir.path(), &["config", "user.name", "Lulu Test"]);
    run_git(dir.path(), &["config", "user.email", "lulu@example.com"]);

    std::fs::write(dir.path().join("README.md"), "# test\n").expect("seed file should write");
    run_git(dir.path(), &["add", "README.md"]);
    run_git(dir.path(), &["commit", "-m", "initial"]);

    dir
}

#[test]
fn projection_maps_internal_terminal_states_to_failed() {
    for status in ["failed", "error", "killed", "crashed", "aborted"] {
        assert_eq!(normalize_dashboard_status(status), DASHBOARD_STATUS_FAILED);
    }

    assert_eq!(normalize_dashboard_status("interrupted"), DASHBOARD_STATUS_INTERRUPTED);
    assert_eq!(normalize_dashboard_status("completed"), DASHBOARD_STATUS_COMPLETED);
    assert_eq!(normalize_dashboard_status("running"), DASHBOARD_STATUS_RUNNING);
}

#[test]
fn projection_normalizes_dashboard_rows_to_locked_statuses() {
    let failed = SessionDashboardRow {
        id: "failed-1".to_string(),
        name: "failed session".to_string(),
        status: "killed".to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
        last_activity_at: None,
        failure_reason: Some("  one\nline\tfailure reason  ".to_string()),
        worktree_path: None,
        restored: false,
        restored_at: None,
        recovery_hint: false,
    };
    let failed_projection = project_dashboard_row(failed);
    assert_eq!(failed_projection.status, DASHBOARD_STATUS_FAILED);
    assert_eq!(failed_projection.failure_reason.as_deref(), Some("one line failure reason"));

    let completed = SessionDashboardRow {
        id: "completed-1".to_string(),
        name: "completed session".to_string(),
        status: "done".to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
        last_activity_at: None,
        failure_reason: Some("should disappear".to_string()),
        worktree_path: None,
        restored: false,
        restored_at: None,
        recovery_hint: false,
    };
    let completed_projection = project_dashboard_row(completed);
    assert_eq!(completed_projection.status, DASHBOARD_STATUS_COMPLETED);
    assert!(completed_projection.failure_reason.is_none());
}

#[test]
fn spawn_uses_session_specific_worktree_path() {
    let repo = init_repo();
    let service = WorktreeService::new(repo.path());

    let first = service
        .create_worktree("session-a")
        .expect("first worktree should create");
    let second = service
        .create_worktree("session-b")
        .expect("second worktree should create");

    assert_ne!(first, second, "session worktree paths must be unique");
    assert!(first.ends_with("session-a"));
    assert!(second.ends_with("session-b"));
    assert!(first.exists(), "first worktree path should exist");
    assert!(second.exists(), "second worktree path should exist");

    service
        .remove_worktree_for_session("session-a")
        .expect("first worktree should remove");
    service
        .remove_worktree_for_session("session-b")
        .expect("second worktree should remove");
    service.prune_worktrees().expect("worktree prune should succeed");
}

#[test]
fn startup_reconcile_marks_stale_running_as_failed() {
    let repo = init_repo();
    let db_path = repo.path().join("lulu.db");
    let db = init_database(&db_path).expect("database should initialize");

    let created_at = chrono::Utc::now().to_rfc3339();
    let session = Session {
        id: "stale-session".to_string(),
        name: "stale".to_string(),
        status: "running".to_string(),
        working_dir: repo.path().display().to_string(),
        created_at: created_at.clone(),
        updated_at: created_at,
    };
    db.create_session(&session).expect("session should persist");

    let service = WorktreeService::new(repo.path());
    let worktree_path = service
        .create_worktree("stale-session")
        .expect("worktree should create");
    db.update_worktree_path("stale-session", Some(&worktree_path.display().to_string()))
        .expect("worktree path should persist");

    reconcile_sessions_on_startup(&db).expect("startup reconciliation should succeed");

    let stored = db
        .get_session("stale-session")
        .expect("session read should succeed")
        .expect("session should exist");
    assert_eq!(stored.status, "failed");

    let dashboard = db
        .list_dashboard_sessions()
        .expect("dashboard query should succeed");
    let stale_row = dashboard
        .iter()
        .find(|row| row.id == "stale-session")
        .expect("stale row should exist");
    assert!(
        stale_row
            .failure_reason
            .as_deref()
            .unwrap_or_default()
            .contains("marked failed on restart"),
        "stale session should have inline-safe reconcile failure reason"
    );

    service
        .remove_worktree_for_session("stale-session")
        .expect("worktree should remove");
    service.prune_worktrees().expect("prune should succeed");
}

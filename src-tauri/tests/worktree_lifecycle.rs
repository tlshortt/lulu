use tauri_app_lib::session::projection::{
    normalize_dashboard_status, DASHBOARD_STATUS_COMPLETED, DASHBOARD_STATUS_FAILED,
    DASHBOARD_STATUS_RUNNING,
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
    for status in ["failed", "error", "killed", "interrupted", "crashed", "aborted"] {
        assert_eq!(normalize_dashboard_status(status), DASHBOARD_STATUS_FAILED);
    }

    assert_eq!(normalize_dashboard_status("completed"), DASHBOARD_STATUS_COMPLETED);
    assert_eq!(normalize_dashboard_status("running"), DASHBOARD_STATUS_RUNNING);
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

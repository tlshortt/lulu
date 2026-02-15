use tauri_app_lib::session::projection::{
    normalize_dashboard_status, DASHBOARD_STATUS_COMPLETED, DASHBOARD_STATUS_FAILED,
    DASHBOARD_STATUS_RUNNING,
};

#[test]
fn projection_maps_internal_terminal_states_to_failed() {
    for status in ["failed", "error", "killed", "interrupted", "crashed", "aborted"] {
        assert_eq!(normalize_dashboard_status(status), DASHBOARD_STATUS_FAILED);
    }

    assert_eq!(normalize_dashboard_status("completed"), DASHBOARD_STATUS_COMPLETED);
    assert_eq!(normalize_dashboard_status("running"), DASHBOARD_STATUS_RUNNING);
}

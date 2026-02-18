use crate::db::SessionDashboardRow;
use serde::{Deserialize, Serialize};

pub const DASHBOARD_STATUS_STARTING: &str = "Starting";
pub const DASHBOARD_STATUS_RUNNING: &str = "Running";
pub const DASHBOARD_STATUS_COMPLETED: &str = "Completed";
pub const DASHBOARD_STATUS_INTERRUPTED: &str = "Interrupted";
pub const DASHBOARD_STATUS_FAILED: &str = "Failed";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DashboardSessionProjection {
    pub id: String,
    pub name: String,
    pub status: String,
    pub created_at: String,
    pub last_activity_at: Option<String>,
    pub failure_reason: Option<String>,
    pub restored: bool,
    pub restored_at: Option<String>,
    pub recovery_hint: bool,
}

pub fn normalize_dashboard_status(status: &str) -> &'static str {
    match status {
        "starting" | "queued" | "created" => DASHBOARD_STATUS_STARTING,
        "running" => DASHBOARD_STATUS_RUNNING,
        "interrupting" => DASHBOARD_STATUS_RUNNING,
        "interrupted" => DASHBOARD_STATUS_INTERRUPTED,
        "completed" | "complete" | "done" | "success" => DASHBOARD_STATUS_COMPLETED,
        "failed"
        | "error"
        | "killed"
        | "cancelled"
        | "canceled"
        | "crashed"
        | "panic"
        | "timed_out"
        | "timeout"
        | "aborted" => DASHBOARD_STATUS_FAILED,
        _ => DASHBOARD_STATUS_RUNNING,
    }
}

pub fn project_dashboard_status(status: &str) -> String {
    normalize_dashboard_status(status).to_string()
}

pub fn normalize_failure_reason(reason: Option<&str>) -> Option<String> {
    let value = reason?.trim();
    if value.is_empty() {
        return None;
    }

    let single_line = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if single_line.len() <= 140 {
        return Some(single_line);
    }

    let trimmed: String = single_line.chars().take(137).collect();
    Some(format!("{}...", trimmed.trim_end()))
}

pub fn project_dashboard_row(row: SessionDashboardRow) -> DashboardSessionProjection {
    let projected_status = project_dashboard_status(&row.status);
    let projected_reason = if projected_status == DASHBOARD_STATUS_FAILED {
        normalize_failure_reason(row.failure_reason.as_deref())
    } else {
        None
    };

    DashboardSessionProjection {
        id: row.id,
        name: row.name,
        status: projected_status,
        created_at: row.created_at,
        last_activity_at: row.last_activity_at,
        failure_reason: projected_reason,
        restored: row.restored,
        restored_at: row.restored_at,
        recovery_hint: row.recovery_hint,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_failure_reason_coalesces_whitespace() {
        let reason = normalize_failure_reason(Some("line one\nline two\tline three"));
        assert_eq!(reason.as_deref(), Some("line one line two line three"));
    }

    #[test]
    fn interrupted_status_projects_to_interrupted_chip() {
        assert_eq!(normalize_dashboard_status("interrupted"), DASHBOARD_STATUS_INTERRUPTED);
    }
}

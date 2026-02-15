use crate::session::events::{SessionEvent, SessionEventPayload};
use serde_json::Value;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;
use tokio::sync::mpsc::error::TrySendError;
use which::which;

pub struct ClaudeCli {
    pub path: PathBuf,
}

impl ClaudeCli {
    /// Find Claude CLI in PATH or common locations
    pub fn find() -> Option<Self> {
        Self::find_with_override(None).ok()
    }

    /// Find Claude CLI with explicit override support
    pub fn find_with_override(override_path: Option<PathBuf>) -> Result<Self, String> {
        if let Some(path) = override_path {
            if path.exists() {
                return Ok(ClaudeCli { path });
            }

            return Err(format!("Invalid CLI override path: {}", path.display()));
        }

        if let Ok(path) = which("claude") {
            return Ok(ClaudeCli { path });
        }

        let home = std::env::var("HOME").map_err(|_| "HOME is not set".to_string())?;
        let locations = [
            format!("{}/.claude/bin/claude", home),
            format!("{}/.local/bin/claude", home),
            "/usr/local/bin/claude".to_string(),
        ];

        for location in locations {
            let path = PathBuf::from(&location);
            if path.exists() {
                return Ok(ClaudeCli { path });
            }
        }

        Err("Claude CLI not found in PATH or common locations".to_string())
    }

    /// Spawn Claude CLI with prompt, streaming output to callback
    pub async fn spawn_with_output(
        &self,
        prompt: &str,
        working_dir: &str,
        on_output: Arc<dyn Fn(String) + Send + Sync>,
    ) -> Result<tokio::process::Child, String> {
        let mut child = Command::new(&self.path)
            .arg("-p")
            .arg(prompt)
            .current_dir(working_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null())
            .spawn()
            .map_err(|e| format!("Failed to spawn: {}", e))?;

        let stdout = child.stdout.take().expect("stdout not captured");
        let stderr = child.stderr.take().expect("stderr not captured");

        let stdout_reader = BufReader::new(stdout);
        let mut stdout_lines = stdout_reader.lines();
        let stdout_callback = on_output.clone();

        let stderr_reader = BufReader::new(stderr);
        let mut stderr_lines = stderr_reader.lines();
        let stderr_callback = on_output.clone();

        tokio::spawn(async move {
            loop {
                match stdout_lines.next_line().await {
                    Ok(Some(line)) => (stdout_callback)(format!("[stdout] {}", line)),
                    Ok(None) => break,
                    Err(e) => {
                        (stdout_callback)(format!("[stdout error] {}", e));
                        break;
                    }
                }
            }
        });

        tokio::spawn(async move {
            loop {
                match stderr_lines.next_line().await {
                    Ok(Some(line)) => (stderr_callback)(format!("[stderr] {}", line)),
                    Ok(None) => break,
                    Err(e) => {
                        (stderr_callback)(format!("[stderr error] {}", e));
                        break;
                    }
                }
            }
        });

        Ok(child)
    }

    pub async fn spawn_with_events(
        &self,
        prompt: &str,
        working_dir: &str,
        session_id: &str,
        tx: mpsc::Sender<SessionEvent>,
    ) -> Result<tokio::process::Child, String> {
        self.ensure_compatible().await?;

        let mut child = Command::new(&self.path)
            .arg("-p")
            .arg(prompt)
            .current_dir(working_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null())
            .spawn()
            .map_err(|e| format!("Failed to spawn: {}", e))?;

        let stdout = child.stdout.take().expect("stdout not captured");
        let stderr = child.stderr.take().expect("stderr not captured");

        let seq = Arc::new(AtomicU64::new(1));
        let overflow_reported = Arc::new(AtomicBool::new(false));
        let tx_out = tx.clone();
        let out_session = session_id.to_string();
        let out_seq = seq.clone();
        let out_overflow_reported = overflow_reported.clone();

        try_send_with_overflow(
            &tx,
            session_id,
            &seq,
            &overflow_reported,
            SessionEventPayload::Status { status: "running".to_string() },
        );

        tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let event =
                    parse_output_line(&out_session, out_seq.fetch_add(1, Ordering::SeqCst), &line);
                try_send_event_with_overflow(
                    &tx_out,
                    &out_session,
                    &out_seq,
                    &out_overflow_reported,
                    event,
                );
            }
        });

        let tx_err = tx.clone();
        let err_session = session_id.to_string();
        let err_seq = seq.clone();
        let err_overflow_reported = overflow_reported.clone();
        tokio::spawn(async move {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                try_send_with_overflow(
                    &tx_err,
                    &err_session,
                    &err_seq,
                    &err_overflow_reported,
                    SessionEventPayload::Error { message: line },
                );
            }
        });

        Ok(child)
    }

    pub async fn ensure_compatible(&self) -> Result<(), String> {
        let output = Command::new(&self.path)
            .arg("--version")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| format!("Failed to probe Claude CLI version: {}", e))?;

        if !output.status.success() {
            return Err(format!(
                "Failed to probe Claude CLI version: process exited with status {}",
                output.status
            ));
        }

        let version_raw = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Self::validate_version_output(&version_raw)
    }

    pub fn validate_version_output(version_raw: &str) -> Result<(), String> {
        let (major, minor, patch) = parse_semver(version_raw).ok_or_else(|| {
            format!(
                "Unsupported Claude CLI version format: '{}'. Expected semantic version like 1.2.3",
                version_raw
            )
        })?;

        if major == 0 && minor < 9 {
            return Err(format!(
                "Unsupported Claude CLI version {}.{}.{}. Require >=0.9.0 for session-event parsing",
                major, minor, patch
            ));
        }

        Ok(())
    }
}

fn parse_semver(raw: &str) -> Option<(u64, u64, u64)> {
    for token in raw.split_whitespace() {
        let normalized = token.trim_matches(|c: char| !(c.is_ascii_digit() || c == '.'));
        if normalized.is_empty() {
            continue;
        }

        let mut parts = normalized.split('.');

        let Some(major_raw) = parts.next() else {
            continue;
        };
        let Some(minor_raw) = parts.next() else {
            continue;
        };
        let Some(patch_raw) = parts.next() else {
            continue;
        };

        let Ok(major) = major_raw.parse::<u64>() else {
            continue;
        };
        let Ok(minor) = minor_raw.parse::<u64>() else {
            continue;
        };
        let Ok(patch) = patch_raw.parse::<u64>() else {
            continue;
        };

        return Some((major, minor, patch));
    }

    None
}

fn try_send_with_overflow(
    tx: &mpsc::Sender<SessionEvent>,
    session_id: &str,
    seq: &Arc<AtomicU64>,
    overflow_reported: &Arc<AtomicBool>,
    payload: SessionEventPayload,
) {
    let event = build_event(session_id, seq.fetch_add(1, Ordering::SeqCst), payload);
    try_send_event_with_overflow(tx, session_id, seq, overflow_reported, event);
}

fn try_send_event_with_overflow(
    tx: &mpsc::Sender<SessionEvent>,
    session_id: &str,
    seq: &Arc<AtomicU64>,
    overflow_reported: &Arc<AtomicBool>,
    event: SessionEvent,
) {
    match tx.try_send(event) {
        Ok(_) => {
            overflow_reported.store(false, Ordering::SeqCst);
        }
        Err(TrySendError::Closed(_)) => {}
        Err(TrySendError::Full(_)) => {
            if !overflow_reported.swap(true, Ordering::SeqCst) {
                let overflow = build_event(
                    session_id,
                    seq.fetch_add(1, Ordering::SeqCst),
                    SessionEventPayload::Error {
                        message: "event channel overflow: dropped session output".to_string(),
                    },
                );
                let _ = tx.try_send(overflow);
            }
        }
    }
}

fn build_event(session_id: &str, seq: u64, payload: SessionEventPayload) -> SessionEvent {
    SessionEvent {
        session_id: session_id.to_string(),
        seq,
        timestamp: chrono::Utc::now().to_rfc3339(),
        payload,
    }
}

pub fn parse_output_line(session_id: &str, seq: u64, line: &str) -> SessionEvent {
    if let Ok(value) = serde_json::from_str::<Value>(line) {
        if let Some(event) = parse_json_event(session_id, seq, value) {
            return event;
        }
    }

    build_event(session_id, seq, SessionEventPayload::Message { content: line.to_string() })
}

fn parse_json_event(session_id: &str, seq: u64, value: Value) -> Option<SessionEvent> {
    let event_type = value.get("type")?.as_str()?;
    let data = value.get("data").cloned().unwrap_or(Value::Null);

    let payload = match event_type {
        "message" => {
            SessionEventPayload::Message { content: data.get("content")?.as_str()?.to_string() }
        }
        "tool_call" => {
            let tool_name = data
                .get("tool_name")
                .and_then(Value::as_str)
                .or_else(|| data.get("name").and_then(Value::as_str))?
                .to_string();
            let args = data
                .get("args")
                .cloned()
                .or_else(|| data.get("arguments").cloned())
                .unwrap_or(Value::Null);
            SessionEventPayload::ToolCall { tool_name, args }
        }
        "tool_result" => {
            let tool_name = data.get("tool_name")?.as_str()?.to_string();
            let result = data.get("result").cloned().unwrap_or(Value::Null);
            SessionEventPayload::ToolResult { tool_name, result }
        }
        "status" => SessionEventPayload::Status {
            status: data.get("status").and_then(Value::as_str).unwrap_or("unknown").to_string(),
        },
        "error" => SessionEventPayload::Error {
            message: data.get("message").and_then(Value::as_str).unwrap_or("unknown").to_string(),
        },
        _ => return None,
    };

    Some(build_event(session_id, seq, payload))
}

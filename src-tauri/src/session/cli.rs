use crate::session::events::{SessionEvent, SessionEventPayload};
use serde_json::Value;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;
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
        let tx_out = tx.clone();
        let out_session = session_id.to_string();
        let out_seq = seq.clone();

        let start = build_event(
            session_id,
            seq.fetch_add(1, Ordering::SeqCst),
            SessionEventPayload::Status {
                status: "started".to_string(),
            },
        );
        let _ = tx.try_send(start);

        tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let event = parse_output_line(
                    &out_session,
                    out_seq.fetch_add(1, Ordering::SeqCst),
                    &line,
                );
                let _ = tx_out.try_send(event);
            }
        });

        let tx_err = tx.clone();
        let err_session = session_id.to_string();
        tokio::spawn(async move {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let event = build_event(
                    &err_session,
                    seq.fetch_add(1, Ordering::SeqCst),
                    SessionEventPayload::Error { message: line },
                );
                let _ = tx_err.try_send(event);
            }
        });

        Ok(child)
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

    build_event(
        session_id,
        seq,
        SessionEventPayload::Message {
            content: line.to_string(),
        },
    )
}

fn parse_json_event(session_id: &str, seq: u64, value: Value) -> Option<SessionEvent> {
    let event_type = value.get("type")?.as_str()?;
    let data = value.get("data").cloned().unwrap_or(Value::Null);

    let payload = match event_type {
        "message" => SessionEventPayload::Message {
            content: data.get("content")?.as_str()?.to_string(),
        },
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
            status: data
                .get("status")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_string(),
        },
        "error" => SessionEventPayload::Error {
            message: data
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_string(),
        },
        _ => return None,
    };

    Some(build_event(session_id, seq, payload))
}

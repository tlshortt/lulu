use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use which::which;

pub struct ClaudeCli {
    pub path: PathBuf,
}

impl ClaudeCli {
    /// Find Claude CLI in PATH or common locations
    pub fn find() -> Option<Self> {
        if let Ok(path) = which("claude") {
            return Some(ClaudeCli { path });
        }

        let home = std::env::var("HOME").ok()?;
        let locations = [
            format!("{}/.claude/bin/claude", home),
            format!("{}/.local/bin/claude", home),
            "/usr/local/bin/claude".to_string(),
        ];

        for location in locations {
            let path = PathBuf::from(&location);
            if path.exists() {
                return Some(ClaudeCli { path });
            }
        }

        None
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
}

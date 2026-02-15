use std::path::PathBuf;
use tokio::sync::mpsc;
use tokio::time::{timeout, Duration};

use tauri_app_lib::session::{ClaudeCli, SessionEventPayload};

#[tokio::test]
async fn cli_override_spawn_emits_parsed_events_in_order() {
    let bin_path = PathBuf::from(env!("CARGO_BIN_EXE_lulu_test_cli"));
    let cli =
        ClaudeCli::find_with_override(Some(bin_path.clone())).expect("override path should work");
    assert_eq!(cli.path, bin_path, "override path must be respected");

    let (tx, mut rx) = mpsc::channel(128);
    let session_id = "test-session";
    let mut spawned = cli
        .spawn_with_events("ignored prompt", ".", session_id, tx)
        .await
        .expect("spawn should succeed");

    timeout(Duration::from_secs(5), spawned.child.wait())
        .await
        .expect("cli should exit")
        .expect("wait should succeed");

    let mut events = Vec::new();
    for _ in 0..8 {
        let event = timeout(Duration::from_millis(500), rx.recv()).await;
        match event {
            Ok(Some(item)) => events.push(item),
            _ => break,
        }
    }

    assert!(!events.is_empty(), "expected at least one emitted event");

    let mut prev_seq = 0u64;
    for event in &events {
        assert_eq!(event.session_id, session_id);
        assert!(event.seq > prev_seq, "seq must increase monotonically");
        prev_seq = event.seq;
    }

    let payloads: Vec<&SessionEventPayload> = events.iter().map(|e| &e.payload).collect();
    assert!(
        payloads
            .iter()
            .any(|p| matches!(p, SessionEventPayload::Thinking { content } if content == "Planning the response")),
        "expected a parsed thinking event"
    );
    assert!(
        payloads
            .iter()
            .any(|p| matches!(p, SessionEventPayload::Message { content } if content == "hello from test cli")),
        "expected a parsed message event"
    );
    assert!(
        payloads.iter().any(
            |p| matches!(p, SessionEventPayload::ToolCall { call_id, tool_name, .. } if call_id.as_deref() == Some("tool-1") && tool_name == "read_file")
        ),
        "expected a parsed tool_call event"
    );
    assert!(
        payloads.iter().any(
            |p| matches!(p, SessionEventPayload::ToolResult { call_id, tool_name, .. } if call_id.as_deref() == Some("tool-1") && tool_name.is_none())
        ),
        "expected a parsed tool_result event"
    );
    assert!(
        payloads
            .iter()
            .any(|p| matches!(p, SessionEventPayload::Status { status } if status == "completed")),
        "expected a terminal completed status event"
    );

    let message_idx = payloads
        .iter()
        .position(|p| matches!(p, SessionEventPayload::Message { content } if content == "hello from test cli"))
        .expect("message event should exist");
    let tool_call_idx = payloads
        .iter()
        .position(|p| {
            matches!(p, SessionEventPayload::ToolCall { call_id, tool_name, .. } if call_id.as_deref() == Some("tool-1") && tool_name == "read_file")
        })
        .expect("tool_call event should exist");
    let tool_result_idx = payloads
        .iter()
        .position(|p| {
            matches!(p, SessionEventPayload::ToolResult { call_id, tool_name, .. } if call_id.as_deref() == Some("tool-1") && tool_name.is_none())
        })
        .expect("tool_result event should exist");

    assert!(
        message_idx < tool_call_idx && tool_call_idx < tool_result_idx,
        "expected Message -> ToolCall -> ToolResult ordering"
    );
}

#[test]
fn invalid_cli_override_path_fails_fast() {
    match ClaudeCli::find_with_override(Some(PathBuf::from("/tmp/does-not-exist-lulu-cli"))) {
        Ok(_) => panic!("invalid override path should error"),
        Err(err) => assert!(err.contains("Invalid CLI override path")),
    }
}

#[test]
fn invalid_cli_override_directory_fails_fast() {
    let directory_path = std::env::temp_dir();
    match ClaudeCli::find_with_override(Some(directory_path.clone())) {
        Ok(_) => panic!("directory override path should error"),
        Err(err) => {
            assert!(err.contains("Invalid CLI override path"));
            assert!(err.contains("must be a file path"));
        }
    }
}

#[test]
fn unsupported_cli_version_is_rejected() {
    let result = ClaudeCli::validate_version_output("claude 0.8.9");
    assert!(result.is_err());
    let message = result.expect_err("unsupported versions should fail");
    assert!(message.contains("Unsupported Claude CLI version"));
}

#[test]
fn unknown_cli_version_format_is_rejected() {
    let result = ClaudeCli::validate_version_output("claude version unknown");
    assert!(result.is_err());
    let message = result.expect_err("unknown format should fail");
    assert!(message.contains("Unsupported Claude CLI version format"));
}

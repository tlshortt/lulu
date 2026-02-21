# Rust Audit: `src-tauri`

## Clippy Warnings

| File | Line | Issue |
|------|------|-------|
| `lib.rs` | 22 | `std::io::Error::new(std::io::ErrorKind::Other, err)` → use `std::io::Error::other(err)` |
| `commands/session.rs` | 151 | `match ... { Ok(x) => x, Err(_) => None }` → `.unwrap_or_default()` |
| `session/supervisor.rs` | 181 | `finalize_terminal_transition_and_emit` has 8 args (limit is 7) — design smell |
| `src/bin/lulu_test_cli.rs` | multiple | `println!("{}", literal)` where no format string is needed |
| `tests/single_session_core.rs` | 57, 123 | `loop { match ... }` → `while let` |

Run `cargo clippy --fix` to auto-apply most of these.

---

## High Severity






---

## Medium Severity








---

## Low / Style

### 8. `SeqCst` ordering on every atomic in `SessionRuntime` (`session/supervisor.rs`)

`killed`, `interrupt_requested`, `interrupt_requests`, and `terminal_transitioned` all use `Ordering::SeqCst` — the most expensive memory ordering. For single-flag booleans with no cross-variable ordering dependencies, `Acquire`/`Release` (or `Relaxed` for the `interrupt_requests` counter) would be idiomatic and sufficient.

### 9. Status values as raw `&str`/`String` throughout

Session statuses (`"running"`, `"completed"`, `"failed"`, etc.) are repeated as raw strings across 5+ files. A `SessionStatus` enum with `Serialize`/`Deserialize` + `Display` would catch typos at compile time and centralize the domain model. Currently not a bug, but a design gap that grows riskier as the codebase evolves.

### 10. Dead variable in `spawn_session` (`commands/session.rs:417-425`)

`spawn_args` is constructed (with a redacted prompt) solely for the debug event JSON — it is not the args actually passed to the CLI. Clear from context this is intentional for security (not logging the real prompt), but it allocates a `Vec<String>` that exists only to be serialized into one JSON event. A comment explaining the intent would prevent future confusion.

### 11. Unused parameter without annotation (`session/supervisor.rs:131`)

```rust
let _ = operation; // operation: &str param is accepted but immediately discarded
```

The `operation` parameter to `acquire_lifecycle_operation` is silently dropped. Either remove it (it's only called with hardcoded string literals anyway) or add a `// reserved for future logging` comment to signal intent.

---

## Summary

| Severity | Count | Key Items |
|----------|-------|-----------|
| High | 3 | Tokio runtime in event handler, stale inflight states not reconciled on startup, silent double DB write on resume |
| Medium | 4 | Triplicated `is_terminal_status`, blocking git ops in async-adjacent code, SIGKILL vs SIGINT, worktree cleanup ordering |
| Low / Style | 4 | SeqCst overuse, stringly-typed status values, dead variable, unused param |
| Clippy | 10 | Mostly auto-fixable with `cargo clippy --fix` |

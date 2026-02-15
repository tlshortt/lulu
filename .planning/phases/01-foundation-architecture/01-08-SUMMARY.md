---
phase: 01-foundation-architecture
plan: "08"
subsystem: runtime-hardening
tags: [rust, tauri-events, ipc, compatibility, sqlite, verification]

# Dependency graph
requires:
  - phase: 01-foundation-architecture
    provides: typed session events and CLI integration baseline
provides:
  - Bounded session event transport from CLI parser to Tauri event bus
  - Deterministic CLI compatibility guardrails
  - Runtime startup verification for SQLite creation/queryability
  - Frontend isolation proof for interleaved session event streams
affects: [backend-session-runtime, frontend-session-events, verification]

# Tech tracking
tech-stack:
  added: []
  patterns: [bounded mpsc bridging, fail-fast CLI version probe, isolation-first store tests]

key-files:
  created: [src-tauri/tests/runtime_startup.rs, src/lib/__tests__/sessions.isolation.test.ts]
  modified: [src-tauri/src/commands/session.rs, src-tauri/src/session/cli.rs, src-tauri/tests/cli_ipc.rs, src/lib/stores/sessions.ts, src-tauri/src/bin/lulu_test_cli.rs, package.json]

key-decisions:
  - "Keep app.emit ownership in command layer; cli module only emits typed events into channel"
  - "Enforce CLI compatibility via --version probe and explicit semver gate before spawn"
  - "Expose targeted test helpers in sessions store to verify per-session isolation without UI coupling"

# Metrics
duration: 1 session
completed: 2026-02-15
---

# Phase 01 Plan 08: Runtime Hardening + Startup Verification Summary

Bounded runtime event transport, CLI compatibility validation, startup evidence, and session isolation verification are now implemented and executable.

## Accomplishments
- Rewired `spawn_session` to use `spawn_with_events` and a bounded `tokio::mpsc` pipeline that forwards canonical `session-event` payloads and legacy compatibility events.
- Added fail-fast CLI compatibility checks (`--version` probe + semver validation) with deterministic unsupported/unknown-format errors.
- Added runtime startup integration tests proving `lulu.db` is created and schema is queryable.
- Added frontend isolation tests proving interleaved `session_id` streams do not cross buffers/event arrays.
- Added `test:phase1-startup` script for reproducible phase verification in one command.

## Verification
- `npm run check` ✅
- `npm run test:unit` ✅
- `cargo test` ✅
- `npm run test:phase1-startup` ✅
  - `cargo test --test cli_ipc --test runtime_startup` ✅
  - `npm run test:unit -- sessions.isolation` ✅

## Files Created/Modified
- `src-tauri/src/commands/session.rs`
- `src-tauri/src/session/cli.rs`
- `src-tauri/tests/cli_ipc.rs`
- `src-tauri/tests/runtime_startup.rs`
- `src-tauri/src/bin/lulu_test_cli.rs`
- `src/lib/stores/sessions.ts`
- `src/lib/__tests__/sessions.isolation.test.ts`
- `package.json`

## Notes
- Shortcut behavior (`Cmd+N`) remains deferred per user direction and is not part of this plan closure.

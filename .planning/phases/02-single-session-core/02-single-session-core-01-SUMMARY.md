---
phase: 02-single-session-core
plan: 01
subsystem: backend
tags: [tauri, rust, stream-json, sqlite, integration-tests]

requires:
  - phase: 01-foundation-architecture
    provides: Existing session spawn pipeline, SQLite schema, and frontend session-event consumers
provides:
  - Canonical stream-json normalization to typed session events
  - Idempotent terminal lifecycle reducer with durable DB terminal transitions
  - Fixture-backed integration tests for success and failure streaming flows
affects: [session-runtime, ipc-events, backend-tests]

tech-stack:
  added: []
  patterns:
    - Canonical backend event normalization before IPC emission
    - Single guarded terminal reducer for DB and frontend lifecycle convergence

key-files:
  created:
    - src-tauri/tests/single_session_core.rs
  modified:
    - src-tauri/src/session/events.rs
    - src-tauri/src/session/cli.rs
    - src-tauri/src/commands/session.rs
    - src-tauri/src/db/session.rs
    - src-tauri/src/bin/lulu_test_cli.rs
    - src-tauri/tests/cli_ipc.rs

key-decisions:
  - "Normalize Claude stream-json assistant/user/result frames into one typed payload contract before emitting session-event"
  - "Use a single terminal reducer guard to prevent duplicate completion/failure transitions from stream and child-exit paths"

patterns-established:
  - "Session terminal transitions are canonicalized to running/completed/failed/killed"
  - "Integration fixtures must mirror stream-json shape instead of legacy {type,data}-only payloads"

duration: 6 min
completed: 2026-02-15
---

# Phase 02 Plan 01: Single Session Core Summary

**Canonical stream-json ingestion now emits typed message/thinking/tool/status/error events and commits one durable terminal session state in SQLite.**

## Performance

- **Duration:** 6 min
- **Started:** 2026-02-15T19:44:43Z
- **Completed:** 2026-02-15T19:50:59Z
- **Tasks:** 3
- **Files modified:** 7

## Accomplishments
- Added stream-json mode to Claude CLI execution and normalized assistant/user/result/system frames into typed backend payloads.
- Introduced a guarded terminal reducer in `spawn_session` that validates working directories, avoids stale running records, and updates terminal status once.
- Added deterministic fixture-backed integration tests covering success/failure event streams, event ordering, and terminal DB persistence behavior.

## Task Commits

Each task was committed atomically:

1. **Task 1: Normalize Claude stream-json output into canonical typed session-event payloads** - `0688856` (feat)
2. **Task 2: Implement idempotent terminal lifecycle reducer with durable DB updates** - `30eb5de` (fix)
3. **Task 3: Add fixture-backed integration tests for launch, streaming, and terminal persistence** - `9245ded` (test)

## Files Created/Modified
- `src-tauri/src/session/events.rs` - extended typed event contract with thinking and richer tool metadata
- `src-tauri/src/session/cli.rs` - enabled `--output-format stream-json` and normalized real stream frames
- `src-tauri/src/commands/session.rs` - added single-path terminal reducer with guarded DB + frontend lifecycle handling
- `src-tauri/src/db/session.rs` - added terminal transition helper constrained to `running -> terminal`
- `src-tauri/src/bin/lulu_test_cli.rs` - expanded fixture output to realistic stream-json success/failure transcripts
- `src-tauri/tests/cli_ipc.rs` - asserted typed ordering, thinking/tool mapping, and terminal status normalization
- `src-tauri/tests/single_session_core.rs` - verified single-session launch/stream/terminal persistence scenarios

## Decisions Made
- Canonicalized terminal vocabulary to `running/completed/failed/killed` at backend boundaries, mapping legacy aliases (`complete`, `done`, `error`, `success`) at parse time.
- Let `session-event` remain the primary structured stream while keeping legacy `session-output/session-complete/session-error` emits as compatibility shims.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- Backend single-session launch/stream/terminal contract is deterministic and covered by integration tests.
- Ready for `02-02-PLAN.md` frontend alignment and verification work.

---
*Phase: 02-single-session-core*
*Completed: 2026-02-15*

## Self-Check: PASSED

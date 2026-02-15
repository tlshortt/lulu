---
phase: 03-multi-session-orchestration
plan: 01
subsystem: api
tags: [git-worktree, sqlite, tauri, session-orchestration]
requires:
  - phase: 02-single-session-core
    provides: single-session spawn/stream lifecycle and terminal persistence
provides:
  - Per-session git worktree lifecycle service for isolated spawn/delete flows
  - Dashboard projection normalization constrained to Starting/Running/Completed/Failed
  - Startup reconciliation for stale in-flight sessions and orphaned managed worktrees
affects: [phase-03-plan-02, phase-03-plan-03, dashboard-list, supervisor-runtime]
tech-stack:
  added: []
  patterns: [backend projection boundary, worktree-per-session isolation, startup reconcile-on-boot]
key-files:
  created:
    - src-tauri/src/session/projection.rs
    - src-tauri/src/session/worktree.rs
    - src-tauri/tests/worktree_lifecycle.rs
  modified:
    - src-tauri/src/db/mod.rs
    - src-tauri/src/db/session.rs
    - src-tauri/src/commands/session.rs
    - src-tauri/src/lib.rs
    - src-tauri/src/session/mod.rs
key-decisions:
  - "Normalize all terminal non-success states (killed/interrupted/crashed variants) to dashboard Failed while keeping rich internal/runtime statuses."
  - "Create one detached git worktree per session under .lulu/worktrees/<session-id> and never reuse worktrees across sessions."
  - "Run startup reconciliation on app boot to fail stale in-flight sessions and prune orphaned managed worktrees before UI commands execute."
patterns-established:
  - "Projection Boundary: dashboard payload status is always one of Starting, Running, Completed, Failed."
  - "Worktree Lifecycle: create before spawn, persist path, remove/prune on delete and reconcile on startup."
duration: 6 min
completed: 2026-02-15
---

# Phase 3 Plan 1: Backend worktree isolation + projection + reconcile Summary

**Session launches now run in isolated git worktrees with durable dashboard projection metadata and startup reconciliation that clears stale running rows into Failed.**

## Performance

- **Duration:** 6 min
- **Started:** 2026-02-15T22:58:42Z
- **Completed:** 2026-02-15T23:05:14Z
- **Tasks:** 3
- **Files modified:** 8

## Accomplishments
- Extended SQLite session schema and persistence APIs to store `last_activity_at`, `failure_reason`, and `worktree_path` metadata for dashboard projection use.
- Added strict projection normalization in `session/projection.rs` so user-facing list statuses are constrained to `Starting|Running|Completed|Failed` with one-line failure reason sanitization.
- Implemented `WorktreeService` and session command wiring so each session gets a unique detached worktree path before spawn, with cleanup/prune on delete.
- Added startup reconciliation to mark stale `starting/running` sessions as `failed` and reconcile/prune orphaned managed worktrees.

## Task Commits

Each task was committed atomically:

1. **Task 1: Add locked dashboard projection persistence and normalization** - `69aa970` (feat)
2. **Task 2: Implement per-session WorktreeService and spawn/cleanup wiring** - `e396440` (feat)
3. **Task 3: Add startup reconciliation for stale sessions and orphaned worktrees** - `b734763` (feat)

## Files Created/Modified
- `src-tauri/src/session/projection.rs` - Locked four-state dashboard projection and failure-reason normalization.
- `src-tauri/src/session/worktree.rs` - Git worktree lifecycle wrapper (create/list/remove/prune/reconcile).
- `src-tauri/tests/worktree_lifecycle.rs` - Coverage for projection normalization, session-specific worktree paths, and startup reconciliation behavior.
- `src-tauri/src/db/mod.rs` - Session metadata columns and migration-safe column guards.
- `src-tauri/src/db/session.rs` - Dashboard row query APIs, worktree metadata accessors, and stale in-flight reconciliation update.
- `src-tauri/src/commands/session.rs` - Spawn/delete orchestration wiring to worktree/projection metadata and startup reconcile entrypoint.
- `src-tauri/src/lib.rs` - Startup hook invokes reconciliation before app state wiring completes.
- `src-tauri/src/session/mod.rs` - Session module exports projection/worktree services.

## Decisions Made
- Normalized all terminal non-success runtime outcomes to dashboard `Failed` to preserve the locked four-state UX vocabulary.
- Used detached worktrees per session id under app-managed `.lulu/worktrees/` paths to prevent branch/index collisions.
- Treated stale persisted `starting/running` rows as startup-fail reconciliation cases instead of leaving rows stuck in running state.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- Backend isolation and projection contracts are in place for supervisor parallel runtime hardening in plan 03-02.
- Dashboard list layer can consume locked status/failure projection fields without introducing extra status variants.

---
*Phase: 03-multi-session-orchestration*
*Completed: 2026-02-15*

## Self-Check: PASSED

# Codebase Concerns

**Analysis Date:** 2026-02-15

## Tech Debt

**Session status never updated on completion/error:**
- Issue: Sessions are inserted as `running` but never updated to `completed`/`error` when the child process exits.
- Files: `src-tauri/src/commands/session.rs`, `src-tauri/src/db/session.rs`
- Impact: UI shows stale statuses and the database accumulates incorrect session state.
- Fix approach: Call `db.update_session_status` in the completion/error branches inside the background task.

**Messages table is unused + output not persisted:**
- Issue: `messages` table is created but no code writes to or reads from it; output is kept only in memory.
- Files: `src-tauri/src/db/mod.rs`, `src-tauri/src/db/session.rs`, `src-tauri/src/session/cli.rs`, `src/lib/stores/sessions.ts`
- Impact: Output history is lost on restart and schema adds maintenance overhead.
- Fix approach: Persist output lines to `messages` or remove the table if persistence is out of scope.

**Unused event emission:**
- Issue: `session-started` event is emitted but not consumed by the frontend.
- Files: `src-tauri/src/commands/session.rs`, `src/lib/stores/sessions.ts`
- Impact: Redundant work and missed opportunity to update UI immediately.
- Fix approach: Add a listener to refresh state or remove the unused event.

## Known Bugs

**Stale running sessions after CLI failure:**
- Symptoms: If spawning the CLI fails after the session record is created, the session remains in the DB as `running`.
- Files: `src-tauri/src/commands/session.rs`, `src-tauri/src/db/session.rs`
- Trigger: `ClaudeCli::spawn_with_output` returns error (missing CLI, invalid `working_dir`).
- Workaround: Manually delete rows from the database (no UI for cleanup).

**Sessions remain running after completion:**
- Symptoms: Completed sessions still show `running` when listing sessions.
- Files: `src-tauri/src/commands/session.rs`, `src-tauri/src/db/session.rs`
- Trigger: Child process exits normally or with error; no status update is recorded.
- Workaround: None (requires code change to update status).

## Security Considerations

**CSP disabled in Tauri config:**
- Risk: Reduced protection against injected scripts or unsafe content rendering.
- Files: `src-tauri/tauri.conf.json`
- Current mitigation: None (CSP explicitly set to `null`).
- Recommendations: Define a restrictive CSP for production builds.

**Devtools enabled in window config:**
- Risk: Exposes debugging tools in production builds.
- Files: `src-tauri/tauri.conf.json`
- Current mitigation: None.
- Recommendations: Disable `devtools` for release builds.

**Unvalidated working directory input:**
- Risk: Arbitrary path passed to `current_dir` could expose sensitive directories or cause errors.
- Files: `src/lib/components/NewSessionModal.svelte`, `src-tauri/src/commands/session.rs`, `src-tauri/src/session/cli.rs`
- Current mitigation: None (string is passed directly).
- Recommendations: Validate directory existence/permissions before spawning the CLI.

## Performance Bottlenecks

**Unbounded output accumulation in memory:**
- Problem: Session output strings grow without limit in the frontend store.
- Files: `src/lib/stores/sessions.ts`, `src/lib/components/SessionOutput.svelte`
- Cause: `sessionOutputs` concatenates lines indefinitely.
- Improvement path: Add truncation, streaming window, or persistence-backed paging.

**Polling loop per session:**
- Problem: Each session runs a 200ms polling loop to check process status.
- Files: `src-tauri/src/commands/session.rs`
- Cause: `try_wait` loop with `sleep(Duration::from_millis(200))`.
- Improvement path: Use `child.wait().await` in a task and avoid polling.

## Fragile Areas

**Shutdown path uses a new Tokio runtime + blocking call:**
- Files: `src-tauri/src/lib.rs`
- Why fragile: `Runtime::new().unwrap()` can panic and `block_on` runs while handling a window event.
- Safe modification: Avoid `unwrap`, reuse the app runtime, and keep shutdown async-safe.
- Test coverage: No automated tests for shutdown behavior.

**Nested locks with awaits:**
- Files: `src-tauri/src/session/manager.rs`, `src-tauri/src/commands/session.rs`
- Why fragile: Locks are held across `.await` while killing processes, which can stall other operations needing the same locks.
- Safe modification: Reduce lock scope and avoid holding `sessions` lock across async calls.
- Test coverage: No concurrency-focused tests.

## Scaling Limits

**Database grows without cleanup:**
- Current capacity: Unlimited growth of `sessions` (and `messages` if implemented).
- Limit: Long-running use will slow queries and increase disk usage.
- Files: `src-tauri/src/db/session.rs`
- Scaling path: Add retention policies and expose deletion in UI.

## Dependencies at Risk

**External Claude CLI dependency is unmanaged:**
- Risk: The app fails if the `claude` binary is missing or incompatible.
- Impact: Session spawn fails and leaves stale DB rows.
- Files: `src-tauri/src/session/cli.rs`, `src-tauri/src/commands/session.rs`
- Migration plan: Validate CLI version at startup and surface a clear UI error.

## Missing Critical Features

**Not detected.**

## Test Coverage Gaps

**Backend commands and session lifecycle are untested:**
- What's not tested: `spawn_session`, status updates, and shutdown behavior.
- Files: `src-tauri/src/commands/session.rs`, `src-tauri/src/session/manager.rs`, `src-tauri/src/lib.rs`
- Risk: Regressions in process management or DB state go unnoticed.
- Priority: High

**UI coverage is minimal:**
- What's not tested: Session list behavior, new session flow, and output rendering.
- Files: `src/lib/components/SessionList.svelte`, `src/lib/components/NewSessionModal.svelte`, `src/lib/components/SessionOutput.svelte`
- Risk: UI regressions and missing error handling.
- Priority: Medium

---

*Concerns audit: 2026-02-15*

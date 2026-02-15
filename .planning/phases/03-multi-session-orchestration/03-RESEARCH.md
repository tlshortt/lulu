# Phase 3: Multi-Session Orchestration - Research

**Researched:** 2026-02-15
**Domain:** Parallel session orchestration, dashboard projection, and git worktree isolation in Tauri v2 + Svelte 5 + Rust/Tokio
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
### Status and progress signals
- Use a minimal user-facing status set: Starting, Running, Completed, Failed.
- Represent all terminal errors with a single Failed state in the dashboard list (do not split Failed vs Crashed in list UX).
- Running state uses a subtle pulsing status dot.
- Do not show percentage progress; status is the primary signal.
- Do not show running subtext (for example, event type labels) in each row.
- Show recent activity age in compact relative format (for example, 5s, 2m).
- Completed state is a green Completed badge only.
- Failed state uses a red badge only, with no extra motion treatment.
- Show one-line failure reason inline in the dashboard row; full detail remains in session view.
- Do not auto-reorder sessions on status changes.
- Do not apply stale/no-activity warnings in list UX.
- Terminal-state transition timing for activity indicator: stop running indicator immediately when terminal state is reached.

### Dashboard list behavior
- Default dashboard ordering is newest first.
- List uses a comfortable two-line row density.
- Mandatory row fields are session name, status, and recent activity age.
- Selection model: single click selects row, double click opens detailed session view.
- Keep a flat list (no status-grouped sections).
- Completed/failed sessions remain in normal newest-first flow (no separate terminal section).
- Session list uses internal scrolling when content exceeds viewport.
- Activity age placement in row: right-aligned metadata position.

### Claude's Discretion
- Exact visual styling values for badges, pulse animation, and row spacing.
- Exact iconography usage (if any) alongside status labels.
- Final copy details for inline failure reason truncation and overflow.

### Deferred Ideas (OUT OF SCOPE)
None - discussion stayed within phase scope.
</user_constraints>

## Summary

Phase 3 should be planned as an orchestration and projection phase, not just a UI pass. The backend already supports one running process plus event fanout, but Phase 3 adds parallel runtime supervision (3-5 sessions), failure-domain isolation, and git worktree lifecycle management tied to each session. The dashboard should be treated as a read model over persisted session state (`created_at`, `updated_at`, terminal reason), not a direct mirror of raw stream traffic.

The key technical split is: backend owns truth and lifecycle (`starting -> running -> completed|failed`), frontend renders a stable ordered list (`created_at DESC`) with minimal status vocabulary. Keep status semantics locked per user decisions even if internal process details are richer. For LIFE-03, each session must have isolated process/task control, isolated worktree path, and idempotent terminal transition handling so one crash cannot fan out into shared state corruption.

**Primary recommendation:** Plan Phase 3 around a `SessionSupervisor + WorktreeService + DashboardProjection` architecture: supervise each session independently in Tokio, create/remove per-session git worktrees in Rust commands, and render a stable newest-first dashboard projection with compact relative activity age.

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| Tauri | `2` | Desktop IPC and app lifecycle | Existing command/event surface is already in place; official guidance favors commands + channels/events for Rust<->frontend communication |
| Svelte | `^5.0.0` | Dashboard rendering and interactions | Existing store/component architecture; lightweight reactivity for frequent status updates |
| tokio | `1` | Concurrent session supervision and subprocess lifecycle | Official async process/task primitives support independent session runtime management |
| rusqlite | `0.31` | Session list persistence and ordering | Already configured with WAL + busy timeout; matches current repository |
| Git CLI (`git worktree`) | installed `2.50.1` | Per-session repository isolation | Official git primitive for multiple linked working trees with branch safety rules |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `@tauri-apps/api` | `^2` | `invoke`, `listen`, optional `Channel` | Commands for spawn/list/open, event/channel stream for live row/session updates |
| `chrono` | `0.4` | RFC3339 timestamps | Persist `created_at`, `updated_at`, `last_activity_at` consistently |
| `uuid` | `1` | Session/worktree IDs | Unique naming for runtime handles and worktree paths |
| Intl API (`Intl.RelativeTimeFormat`) | runtime built-in | Compact activity age formatting | Render `5s`, `2m`, `1h` style labels without adding dependencies |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Tauri global events for all stream data | Tauri `Channel` for high-volume stream + events for coarse status | Tauri docs explicitly position events as simple/small payload transport and channels for ordered streaming throughput |
| Manual branch switching in one worktree | per-session `git worktree add` | Shared worktree risks conflicts across 3-5 parallel sessions |
| Dynamic list reorder by status/activity | fixed newest-first by `created_at` | Dynamic reorder hurts glanceability and conflicts with locked UX decision |

**Installation:**
```bash
# No new packages required for this phase baseline.
# Use existing repository stack and system git.
```

## Architecture Patterns

### Recommended Project Structure
```
src-tauri/src/
├── session/
│   ├── supervisor.rs     # runtime lifecycle, task/process handles, crash isolation
│   ├── worktree.rs       # git worktree create/list/remove/prune wrappers
│   └── projection.rs     # dashboard row status/activity projection events
├── commands/
│   └── session.rs        # spawn/open/list commands; worktree-aware launch flow
└── db/
    └── session.rs        # persisted list model fields (status, failure reason, activity timestamps)

src/lib/
├── stores/sessions.ts    # list projection + selected/open session behavior
└── components/SessionList.svelte
```

### Pattern 1: Session Supervisor (independent failure domains)
**What:** Track each running session with its own process handle, cancellation signal, and terminal guard; never share child handles between sessions.
**When to use:** Always for SESS-01 + LIFE-03 (3-5 concurrent sessions).
**Example:**
```rust
// Source: https://docs.rs/tokio/latest/tokio/task/struct.JoinSet.html
let mut set = tokio::task::JoinSet::new();
for session in sessions_to_start {
    set.spawn(async move {
        // own child process + own channels + own terminal transition
        run_session(session).await
    });
}
while let Some(result) = set.join_next().await {
    // handle one session finishing without impacting others
}
```

### Pattern 2: Worktree-per-session lifecycle
**What:** Create linked worktree before spawn, run Claude inside that path, and clean up with explicit remove/prune policy.
**When to use:** Required by GIT-02 for all parallel sessions.
**Example:**
```bash
# Source: https://git-scm.com/docs/git-worktree
git worktree add -b session-<id> "<base>/.lulu/worktrees/<id>" <base-commit>
git worktree list --porcelain
git worktree remove "<base>/.lulu/worktrees/<id>"
git worktree prune
```

### Pattern 3: Stable Dashboard Projection
**What:** Persist list state fields (`name`, normalized user-facing `status`, `updated_at`/`last_activity_at`, optional `failure_reason`) and sort only by `created_at DESC`.
**When to use:** Always for dashboard list rendering.
**Example:**
```ts
// Source: https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Intl/RelativeTimeFormat
const rtf = new Intl.RelativeTimeFormat("en", { style: "short", numeric: "always" });
// Convert delta seconds to compact label like "5 sec. ago" -> render compact token (5s) in UI formatter.
```

### Anti-Patterns to Avoid
- **Shared mutable runtime handle for all sessions:** Raises blast radius; one panic/lock stall can affect unrelated sessions.
- **Status derivation from raw transport events in UI only:** Produces drift between DB, backend truth, and dashboard rows.
- **Reordering rows on every update:** Violates locked decision and hurts scan stability.
- **Using one common worktree for multiple sessions:** Creates direct file/index conflicts and breaks isolation requirement.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Parallel workspace isolation | Custom `.git` manipulation | `git worktree` subcommands | Officially handles metadata, branch safety, lock/prune rules |
| High-rate IPC stream transport | Ad-hoc JSON bus over generic events only | Tauri Channels (or keep events for low-rate status only) | Tauri docs: channels are optimized for ordered streaming/high throughput |
| Relative time localization | Custom locale tables | `Intl.RelativeTimeFormat` | Standardized locale-aware formatting, no new dependency |
| Process cancellation/reaping | Custom PID signaling logic | `tokio::process::Child::{kill,wait}` semantics | Correct cross-platform behavior and zombie avoidance guidance |

**Key insight:** Most complexity in this phase is lifecycle correctness and isolation boundaries, not rendering. Reuse battle-tested primitives (git worktree, tokio process/task, Tauri IPC) and keep project logic in explicit state transitions.

## Common Pitfalls

### Pitfall 1: Event transport saturation
**What goes wrong:** Large stream output floods global events; UI stutters or drops list updates.
**Why it happens:** Tauri event system is simple but not optimized for high-throughput payloads.
**How to avoid:** Use channels for continuous output, reserve events for coarse lifecycle/status notifications.
**Warning signs:** Delayed status badge updates while output is active.

### Pitfall 2: Cross-session lock contention in shared state
**What goes wrong:** One slow session path blocks others in shared mutexes.
**Why it happens:** Holding async locks across `.await` or centralizing too much mutable state.
**How to avoid:** Keep lock scope short; isolate per-session state and communicate by message passing when practical.
**Warning signs:** Multiple sessions appear stalled after one heavy session starts.

### Pitfall 3: WAL/checkpoint starvation under long-lived readers
**What goes wrong:** SQLite WAL grows and write latency spikes.
**Why it happens:** Continuous readers can prevent checkpoints from fully completing.
**How to avoid:** Keep read transactions short; rely on default checkpointing and avoid long-held read cursors.
**Warning signs:** Growing `-wal` size, intermittent `SQLITE_BUSY`, slower session list writes.

### Pitfall 4: Worktree lifecycle drift
**What goes wrong:** Orphan worktree metadata/paths accumulate after crashes.
**Why it happens:** Worktree path deleted manually or session crashes before cleanup.
**How to avoid:** Reconcile with `git worktree list --porcelain` at startup and run targeted `remove`/`prune`.
**Warning signs:** `git worktree list` shows prunable/missing entries.

### Pitfall 5: Terminal status mismatch with locked UX
**What goes wrong:** Dashboard exposes extra terminal states (e.g., killed/interrupted) that conflict with locked four-state UX.
**Why it happens:** Internal lifecycle values leak directly to list rendering.
**How to avoid:** Normalize internal terminal outcomes to user-facing `Failed`/`Completed` at projection boundary.
**Warning signs:** List rows show statuses not in {Starting, Running, Completed, Failed}.

## Code Examples

Verified patterns from official sources:

### Ordered stream command with Tauri Channel
```rust
// Source: https://v2.tauri.app/develop/calling-frontend/
use tauri::ipc::Channel;

#[derive(Clone, serde::Serialize)]
#[serde(tag = "event", content = "data")]
enum SessionRowEvent {
    Status { session_id: String, status: String },
    Activity { session_id: String, at: String }
}

#[tauri::command]
fn watch_sessions(on_event: Channel<SessionRowEvent>) {
    // send ordered row updates as backend state changes
    let _ = on_event.send(SessionRowEvent::Status {
        session_id: "...".into(),
        status: "running".into(),
    });
}
```

### Safe child termination path
```rust
// Source: https://docs.rs/tokio/latest/tokio/process/struct.Child.html
// kill() sends termination and awaits process reaping.
child.kill().await?;
```

### Worktree reconciliation script-safe format
```bash
# Source: https://git-scm.com/docs/git-worktree
git worktree list --porcelain -z
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Single runtime session model | Explicit multi-session supervision with isolated handles | Product roadmap Phase 3 scope | Enables 3-5 concurrent sessions without shared-failure coupling |
| Event-only frontend streaming | Channels for ordered/high-volume streams + events for coarse updates | Tauri v2 docs (2025 guidance) | Better throughput and reduced UI event pressure |
| Shared repo working directory | `git worktree` per-session isolation | Established Git workflow (current docs 2.53.0) | Prevents branch/index/file conflicts across parallel agents |

**Deprecated/outdated:**
- Treating dashboard status as a direct reflection of raw tool/process sub-events is outdated for this phase; use normalized user-facing projection.

## Open Questions

1. **Worktree retention policy after terminal state**
   - What we know: `git worktree remove` requires clean worktree unless forced; stale entries are recoverable with prune/repair.
   - What's unclear: Product policy for when to clean (immediate, manual, or deferred batch cleanup).
   - Recommendation: Decide policy in planning and encode it as explicit acceptance criteria/tests.

2. **Status vocabulary reconciliation with requirement text (`interrupted`)**
   - What we know: Locked user decision mandates four user-facing states only.
   - What's unclear: Whether interrupted sessions must map to Failed in dashboard list and where interruption detail appears.
   - Recommendation: Keep list to four states; preserve richer internal reason in session detail view.

## Sources

### Primary (HIGH confidence)
- Tauri v2 - Calling frontend (events vs channels, ordering/streaming): https://v2.tauri.app/develop/calling-frontend/ (last updated 2025-05-12)
- Tauri v2 - State management and mutex guidance: https://v2.tauri.app/develop/state-management/ (last updated 2025-05-07)
- Tokio `JoinSet` docs: https://docs.rs/tokio/latest/tokio/task/struct.JoinSet.html
- Tokio `Child` docs (`kill`, `wait`, caveats): https://docs.rs/tokio/latest/tokio/process/struct.Child.html
- Tokio `Mutex` guidance: https://docs.rs/tokio/latest/tokio/sync/struct.Mutex.html
- Git worktree manual: https://git-scm.com/docs/git-worktree (latest page indicates 2.53.0, 2026-02-02)
- Git branch manual (worktree branch constraints): https://git-scm.com/docs/git-branch
- SQLite WAL doc: https://www.sqlite.org/wal.html (last updated 2025-05-31)
- SQLite transactions doc: https://www.sqlite.org/lang_transaction.html (last updated 2025-05-12)
- Repository evidence:
  - `src-tauri/src/commands/session.rs`
  - `src-tauri/src/session/manager.rs`
  - `src-tauri/src/db/mod.rs`
  - `src/lib/stores/sessions.ts`
  - `src/lib/components/SessionList.svelte`

### Secondary (MEDIUM confidence)
- MDN Intl.RelativeTimeFormat usage/baseline: https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Intl/RelativeTimeFormat (last modified 2025-07-10)

### Tertiary (LOW confidence)
- None.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - verified from repository manifests and official Tauri/Tokio/Git/SQLite docs.
- Architecture: HIGH - aligned with current code paths and official concurrency/IPC guidance.
- Pitfalls: MEDIUM-HIGH - backed by official docs plus observed current code structure; cleanup policy details remain product-dependent.

**Research date:** 2026-02-15
**Valid until:** 2026-03-15

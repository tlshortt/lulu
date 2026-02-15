# Phase 3: Multi-Session Orchestration - Research

**Researched:** 2026-02-15
**Domain:** Multi-session runtime orchestration, dashboard projection, and git worktree isolation in Tauri + Svelte + Rust
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

Phase 3 should be planned as backend orchestration first, dashboard rendering second. Current repo state already has a single-session runtime model, session event stream parsing, and newest-first DB ordering, but it does not yet have git worktree lifecycle, normalized four-state dashboard projection, or explicit 3-5 session supervision boundaries. Those are the core planning deltas for SESS-01/02/03, GIT-02, LIFE-03.

The planning shape should be: (1) isolate each session runtime (child handle, cancellation, terminal transition guard), (2) isolate each session filesystem surface (`git worktree` per session), and (3) isolate user-facing status projection from richer internal process details. This directly matches locked decisions (stable newest-first list, no auto-reorder, no progress %, one-line failure reason).

**Primary recommendation:** Use a `SessionSupervisor + WorktreeService + DashboardProjection` split, with backend as source of truth and frontend as deterministic projection consumer.

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| Tauri | `2` | Rust<->frontend command + event/channel IPC | Already in use; official docs explicitly distinguish low-volume events vs high-throughput channels |
| Tokio | `1` | Concurrent process/task lifecycle supervision | Existing runtime dependency; official `JoinSet` and `process::Child` patterns fit per-session isolation |
| rusqlite | `0.31` | Persist session rows/messages and list ordering | Already configured with WAL + busy timeout in repo; low overhead local persistence |
| Git CLI `worktree` | `git 2.50.1` installed | Isolated checkout per parallel session | Official mechanism for multi-working-tree safety and lifecycle commands |
| Svelte + SvelteKit | `^5.0.0` / `^2.9.0` | Dashboard list + selection/open interactions | Existing store/component architecture already handles live updates |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `@tauri-apps/api` | `^2` | `invoke`, `listen`, optional `Channel` bridge | Commands for lifecycle operations; events/channels for row and detail updates |
| `uuid` | `1` | Session/worktree identifiers | Unique names for worktree path and runtime lookup keys |
| `chrono` | `0.4` | RFC3339 timestamps | Durable `created_at` / `updated_at` / `last_activity_at` timestamps |
| `Intl.RelativeTimeFormat` | built-in | Compact relative age rendering | Locale-aware basis for `5s` / `2m` style age labels without extra deps |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Event-only streaming | Tauri `Channel` for high-rate streams + events for lifecycle signals | Tauri docs: events are simple JSON transport, channels are optimized for ordered streaming |
| Shared checkout with branch switching | `git worktree add/remove/prune` per session | Shared checkout violates GIT-02 isolation under 3-5 concurrent sessions |
| UI-only status derivation | Backend-normalized projection model | UI-only derivation drifts from persistence and leaks forbidden status variants |

**Installation:**
```bash
# No new package dependencies required.
# Use existing stack plus system git.
```

## Architecture Patterns

### Recommended Project Structure
```
src-tauri/src/
├── session/
│   ├── supervisor.rs    # independent per-session runtime supervision
│   ├── worktree.rs      # git worktree add/list/remove/prune/reconcile
│   └── projection.rs    # normalize internal -> locked dashboard statuses
├── commands/
│   └── session.rs       # spawn/list/kill/delete wiring across services
└── db/
    └── session.rs       # projection fields + newest-first queries

src/lib/
├── stores/sessions.ts   # dashboard row projection + select/open behavior
└── components/Sidebar.svelte
```

### Pattern 1: Per-session supervision boundary
**What:** Each session has independent child process handle, terminal guard, and cancellation control.
**When to use:** Always for SESS-01 and LIFE-03.
**Example:**
```rust
// Source: https://docs.rs/tokio/latest/tokio/task/struct.JoinSet.html
let mut set = tokio::task::JoinSet::new();
for spec in specs {
    set.spawn(async move { run_one_session(spec).await });
}
while let Some(result) = set.join_next().await {
    // one session completion does not terminate others
}
```

### Pattern 2: Worktree-per-session lifecycle
**What:** Create worktree before spawn; run Claude in worktree path; remove/prune on cleanup/reconcile.
**When to use:** Required by GIT-02 for every launched session.
**Example:**
```bash
# Source: https://git-scm.com/docs/git-worktree
git worktree add -b session-<id> "<repo>/.lulu/worktrees/<id>" <base>
git worktree list --porcelain -z
git worktree remove "<repo>/.lulu/worktrees/<id>"
git worktree prune
```

### Pattern 3: Locked dashboard projection boundary
**What:** Normalize internal statuses to exactly `Starting|Running|Completed|Failed` at backend projection edge.
**When to use:** Always for SESS-02/SESS-03 row payloads.
**Example:**
```typescript
// Source: repository + locked context constraints
const DASHBOARD_STATUS = {
  starting: "Starting",
  running: "Running",
  completed: "Completed",
  failed: "Failed"
} as const;
```

### Anti-Patterns to Avoid
- **Shared mutable runtime map locked across awaits:** creates cross-session stalls and violates failure isolation intent.
- **Passing through internal statuses (`killed`, `interrupted`) directly to list:** violates locked four-state UX contract.
- **Sorting list on status changes or updated time:** conflicts with locked newest-first and no auto-reorder behavior.
- **Manual `.git` internals manipulation:** bypasses official worktree safety and cleanup semantics.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Parallel checkout isolation | Custom `.git` directory hacking | `git worktree` commands | Handles shared refs vs per-worktree state, lock/prune/repair semantics |
| Streaming IPC for high-volume output | Giant global event payload bus | Tauri `Channel` for streams | Tauri docs mark events as not for high-throughput/low-latency payloads |
| Relative age localization | Hard-coded locale/unit tables | `Intl.RelativeTimeFormat` | Built-in locale-aware formatter with short/narrow styles |
| Process kill + reap lifecycle | Ad-hoc pid signaling | `tokio::process::Child::kill().await` / `wait()` | Tokio documents kill/wait semantics and zombie avoidance caveat |

**Key insight:** The risk in this phase is incorrect lifecycle boundaries, not missing UI controls.

## Common Pitfalls

### Pitfall 1: Event saturation hides status transitions
**What goes wrong:** Busy output traffic delays row status updates.
**Why it happens:** Event system is simple JSON transport, not optimized for high throughput.
**How to avoid:** Keep lifecycle signals lightweight; move high-rate stream data to channels.
**Warning signs:** Running indicator lingers after process already exited.

### Pitfall 2: Lock scope creates session coupling
**What goes wrong:** One session path blocks others and breaks LIFE-03 isolation.
**Why it happens:** Locks held across `.await` on shared state.
**How to avoid:** Minimize lock scope; copy needed data, then await outside lock.
**Warning signs:** Multiple sessions stall when one session emits heavy output.

### Pitfall 3: Worktree cleanup drift
**What goes wrong:** Orphan worktree entries accumulate after crashes.
**Why it happens:** Missing startup reconcile pass and missing prune/remove flow.
**How to avoid:** Reconcile against `git worktree list --porcelain -z` at startup.
**Warning signs:** `prunable` entries and missing paths in worktree list.

### Pitfall 4: WAL growth from long-lived readers
**What goes wrong:** WAL file grows and write latency spikes.
**Why it happens:** Checkpoints cannot complete during overlapping long reads.
**How to avoid:** Short read transactions and explicit periodic checkpoint strategy only if needed.
**Warning signs:** Growing `*.db-wal`, intermittent `SQLITE_BUSY`, degraded list/query latency.

### Pitfall 5: Requirement vocabulary mismatch
**What goes wrong:** Requirement text includes statuses outside locked list vocabulary.
**Why it happens:** Mixing internal lifecycle events with user-facing dashboard states.
**How to avoid:** Encode explicit normalization policy: all terminal non-success outcomes map to `Failed` in list.
**Warning signs:** Dashboard row shows any status outside Starting/Running/Completed/Failed.

## Code Examples

Verified patterns from official sources:

### Channel-based ordered stream transport
```rust
// Source: https://v2.tauri.app/develop/calling-frontend/
use tauri::ipc::Channel;

#[derive(Clone, serde::Serialize)]
#[serde(tag = "event", content = "data")]
enum RowEvent {
    Status { session_id: String, status: String },
}

#[tauri::command]
fn watch_sessions(on_event: Channel<RowEvent>) {
    let _ = on_event.send(RowEvent::Status {
        session_id: "abc".into(),
        status: "running".into(),
    });
}
```

### Child termination with reap
```rust
// Source: https://docs.rs/tokio/latest/tokio/process/struct.Child.html
child.kill().await?; // includes wait semantics in tokio
```

### Compact relative time formatter
```typescript
// Source: https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Intl/RelativeTimeFormat
const rtf = new Intl.RelativeTimeFormat("en", { style: "short", numeric: "always" });
const label = rtf.format(-5, "second"); // e.g. "5 sec. ago"
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Single-session process handling in command layer | Explicit multi-session supervisor boundary | Needed for Phase 3 scope | Enables 3-5 independent concurrent sessions |
| Event-only frontend signaling | Events for coarse updates + Channels for stream-heavy paths | Tauri v2 docs (2025) | Prevents status lag under output load |
| Shared working directory per session input | Git linked worktree per session | Long-standing git worktree model | Prevents branch/index collisions across agents |

**Deprecated/outdated:**
- Event-only high-volume transport is outdated for heavy stream payloads in Tauri v2 docs.

## Open Questions

1. **Worktree retention policy after terminal state**
   - What we know: `git worktree remove` requires clean tree unless forced; `prune` clears stale metadata.
   - What's unclear: Product policy for immediate removal vs deferred/manual cleanup.
   - Recommendation: Decide policy during planning and encode test assertions.

2. **Mapping of internal `killed/interrupted` outcomes**
   - What we know: locked UX requires four dashboard statuses only.
   - What's unclear: exact failure reason text and taxonomy in detail view.
   - Recommendation: map all terminal non-success to dashboard `Failed`, keep rich reason in session detail.

## Sources

### Primary (HIGH confidence)
- Tauri v2 Calling Frontend: https://v2.tauri.app/develop/calling-frontend/ (updated 2025-05-12)
- Tauri v2 State Management: https://v2.tauri.app/develop/state-management/ (updated 2025-05-07)
- Tokio JoinSet docs: https://docs.rs/tokio/latest/tokio/task/struct.JoinSet.html
- Tokio Child docs: https://docs.rs/tokio/latest/tokio/process/struct.Child.html
- Tokio Mutex docs: https://docs.rs/tokio/latest/tokio/sync/struct.Mutex.html
- Git worktree manual: https://git-scm.com/docs/git-worktree (latest manual 2.53.0, 2026-02-02)
- SQLite WAL docs: https://www.sqlite.org/wal.html (updated 2025-05-31)
- Repository evidence:
  - `src-tauri/src/commands/session.rs`
  - `src-tauri/src/session/manager.rs`
  - `src-tauri/src/session/cli.rs`
  - `src-tauri/src/db/mod.rs`
  - `src-tauri/src/db/session.rs`
  - `src/lib/stores/sessions.ts`
  - `src/lib/components/SessionList.svelte`
  - `.planning/REQUIREMENTS.md`

### Secondary (MEDIUM confidence)
- MDN Intl.RelativeTimeFormat: https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Intl/RelativeTimeFormat (modified 2025-07-10)

### Tertiary (LOW confidence)
- None.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - verified from repo manifests/runtime and official docs.
- Architecture: HIGH - directly supported by current code structure plus official IPC/concurrency guidance.
- Pitfalls: MEDIUM-HIGH - supported by official docs and current code behavior; cleanup policy remains product decision.

**Research date:** 2026-02-15
**Valid until:** 2026-03-15

# Phase 5: Session Lifecycle Control - Research

**Researched:** 2026-02-16
**Domain:** Session interrupt, resume, and per-session error isolation in Tauri + Svelte + Rust + SQLite
**Confidence:** MEDIUM-HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
### Interrupt controls and timing
- Show Interrupt action in both dashboard row and session detail panel.
- Require confirmation every time before interrupting.
- Use minimal confirmation copy: "Interrupt session?"
- After confirmation, show `Interrupting...` and disable interrupt, resume, and prompt input controls for that session.
- Allow interrupt for `Running` and other active in-progress states.
- On success, keep user on current view and set status to `Interrupted`.
- If interrupt does not complete, perform a silent retry first; if still not stopped, surface error after 10 seconds total.
- For dashboard-row interrupt, keep feedback compact: status chip plus inline spinner only (no row expansion, no auto-navigation).
- Do not add a separate timeline event for successful interrupt.
- No keyboard shortcut for interrupt in this phase.

### Claude's Discretion
- Resume flow behavior details (entry point and prompt composition interaction).
- User-facing status vocabulary beyond the locked interrupt states.
- Error presentation details after interrupt retry fails, as long as error isolation remains per-session.

### Deferred Ideas (OUT OF SCOPE)
None - discussion stayed within phase scope.
</user_constraints>

## Summary

Phase 5 is a lifecycle-control phase, not a button phase. The current backend already owns runtime lifecycle via `SessionSupervisor`, but its stop primitive is `kill_session()` and terminal status handling currently treats terminal states as `completed|failed|killed`. The frontend/dashboard projection also maps `interrupted` to `Failed`. That conflicts directly with the locked requirement that successful interrupt becomes `Interrupted`.

Use Claude CLI native continuation features for resume (`--resume` / `--continue` / `--fork-session`) and keep session identity deterministic with `--session-id` for new runs. Official CLI docs and local CLI help both confirm these flags. This avoids custom replay logic and keeps continuity semantics aligned with the upstream tool.

For interrupt behavior, implement a session-scoped operation state with bounded retry and a hard 10s deadline. Keep all transition authority in backend commands/events and only reflect those in Svelte stores. This is the only reliable way to satisfy per-session isolation and avoid cross-session UI race conditions.

**Primary recommendation:** Implement an explicit lifecycle state machine (`running -> interrupting -> interrupted`, `completed|interrupted -> resuming -> running|failed`) with backend-authoritative transitions, CLI-native resume flags, and strict per-session operation gates.

## Decision Addendum (Resolved Open Questions)

### Decision 1: Resume lineage model for Phase 5
**Decision:** Keep resumed turns in a single existing session row (no child rows in Phase 5).

**Why this is the correct Phase 5 choice:**
- Directly satisfies LIFE-02 (resume completed/interrupted sessions) without introducing new navigation or lineage UX not required by locked scope.
- Aligns with current architecture: existing `sessions` table has no `parent_session_id`/`lineage_root_id`, and existing projection/store/test model assumes one logical row per session.
- Lowest migration risk: avoids schema migration, avoids sidebar behavior rewrites, avoids dashboard sorting/selection edge cases.
- Better testability in this phase: existing session-isolation and dashboard tests extend naturally by asserting status transitions and appended history in the same row.

**Data model implications (implementation-ready):**
- Reuse same Lulu `session_id` row for resumed turns.
- Add run-attempt metadata, not lineage tables:
  - `resume_count INTEGER NOT NULL DEFAULT 0`
  - `active_run_id TEXT NULL` (UUID for current run attempt)
  - `last_resume_at TEXT NULL`
- Keep message history attached to the same `session_id`; distinguish turns/runs by event metadata (`run_id`, monotonic `seq`).

**UI implications (implementation-ready):**
- Sidebar remains one row; no row branching, no parent/child affordances.
- Detail view shows continuous history, with optional lightweight divider label like `Resumed` (not a separate timeline event for successful interrupt).
- Resume action appears in same row/detail controls after terminal states (`Completed` and `Interrupted`) and follows per-session disable rules.

**Explicitly deferred to later phase:** child-row lineage UX (`forked session tree`, `branch inspector`, cross-row ancestry navigation).

### Decision 2: Event transport thresholds for moving to Tauri channels
**Decision:** Keep Tauri events in Phase 5, but gate a mandatory channel migration on concrete trigger conditions below.

**Hard triggers (migrate in next phase immediately if any occur):**
- Any emitted overflow error containing `event channel overflow: dropped session output` in production/dev dogfood sessions.
- Persistent sequence integrity failures (`seq` gaps not explained by dedupe rules) in >= 1% of active sessions over 24h.
- User-visible status lag: p95 backend-emitted timestamp to frontend-applied timestamp > 750ms for 5 consecutive minutes under normal workload.

**Soft triggers (plan migration if two are true in same release cycle):**
- p95 emit-to-apply lag > 300ms for 10 consecutive minutes.
- Sustained event ingress > 60 events/sec for >= 60s in any single session.
- Frontend event queue/apply backlog > 200 pending items more than 3 times per hour.

**Near-term instrumentation to defer safely now (must add in Phase 5):**
- Backend: per-session counters for `events_emitted_total`, `events_overflow_total`, `events_dropped_estimate`, `emit_timestamp_ms`.
- Frontend store: per-session `last_applied_seq`, `seq_gap_count`, `apply_latency_ms` histogram (`event.timestamp` -> apply time).
- Debug surface: aggregate metrics in `session-debug` stream and include one snapshot in failure reports/tests.

**Verification guidance for planner/test tasks:**
- Add test asserting no overflow error event appears in standard resume/interrupt flows.
- Add test asserting monotonic `seq` per `session_id + run_id` and no dropped visible status events.
- Add test/assertion that p95 apply latency stays below 300ms in synthetic high-output fixture.

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| Tauri | `2.x` | Rust command surface, event/channel delivery to frontend | Existing app architecture and official IPC patterns fit lifecycle control |
| Svelte + SvelteKit | `^5.0.0` / `^2.9.0` | Session control UI and state derivation | Existing stores already centralize session projections and event routing |
| Rust + Tokio | Rust `1.93.1`, tokio `1.x` | Process supervision, timeout/retry orchestration | Existing runtime already uses `tokio::process::Child` and async tasks |
| rusqlite | `0.31` | Canonical session status/history persistence | Existing transaction boundaries and WAL mode already in place |
| Claude CLI | `2.1.42` installed | Session execution and continuation semantics | Officially supports `--resume`, `--continue`, `--fork-session`, `--session-id` |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `@tauri-apps/api` | `^2` | `invoke` + event listeners in frontend | Commands for interrupt/resume + status updates |
| Tauri events/channels | v2 docs current | Backend -> frontend session updates | Use events for small status payloads; use channels if stream volume grows |
| Git worktree | `2.53.0` docs, `2.50.1` installed | Session filesystem isolation across resumed turns | Keep existing worktree lifecycle operations; do not touch `.git` internals |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| CLI-native resume flags | Reconstruct transcript and start fresh process | Transcript replay is brittle and diverges from Claude's session model |
| Backend-authoritative state transitions | UI-optimistic transition ownership | UI-only ownership races with process events and breaks isolation |
| Deterministic session IDs | Parse identity from output only | Output parsing is fragile; explicit IDs are testable and stable |

**Installation:**
```bash
# No new dependencies required for baseline Phase 5 implementation.
```

## Architecture Patterns

### Recommended Project Structure
```
src-tauri/src/
├── commands/
│   └── session.rs           # add interrupt_session and resume_session commands
├── session/
│   ├── supervisor.rs        # lifecycle transition authority + 10s interrupt deadline
│   ├── cli.rs               # spawn_new / spawn_resume argument composition
│   └── projection.rs        # include Interrupted dashboard vocabulary
└── db/
    └── session.rs           # terminal/in-progress status updates include interrupted

src/lib/
├── stores/sessions.ts       # per-session in-flight operation gates and error isolation
└── components/
    ├── Sidebar.svelte       # compact row feedback (chip + inline spinner)
    └── SessionOutput.svelte # detail controls + disabled states during interrupting
```

### Pattern 1: Backend-authoritative lifecycle state machine
**What:** Add explicit states (`interrupting`, `interrupted`, optional `resuming`) and drive all transitions from Rust command/supervisor logic.
**When to use:** Every LIFE-01/LIFE-02/LIFE-04 transition.
**Example:**
```rust
// Source: https://docs.rs/tokio/latest/tokio/process/struct.Child.html
// and repository lifecycle flow in src-tauri/src/session/supervisor.rs
let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
set_status(session_id, "interrupting")?;
attempt_interrupt(session_id).await?;
if !wait_until_stopped(session_id, deadline).await {
    attempt_interrupt(session_id).await?; // silent retry
    if !wait_until_stopped(session_id, deadline).await {
        return Err("Interrupt did not complete within 10s".to_string());
    }
}
set_status(session_id, "interrupted")?;
```

### Pattern 2: CLI-native resume command composition
**What:** Continue prior conversation with `--resume` (or `--continue` where appropriate), preserve deterministic IDs via `--session-id` for newly-created sessions.
**When to use:** All resume/continue entry points in Phase 5.
**Example:**
```bash
# Source: https://code.claude.com/docs/en/cli-reference
claude -p "Initial task" --session-id "550e8400-e29b-41d4-a716-446655440000"
claude -p "Follow-up" --resume "550e8400-e29b-41d4-a716-446655440000"
claude -p "Alternative branch" --resume "550e8400-e29b-41d4-a716-446655440000" --fork-session
```

### Pattern 3: Session-scoped UI operation gates
**What:** During interrupting/resuming, disable interrupt/resume/prompt input only for the target session; never globally lock other sessions.
**When to use:** Every user action that mutates lifecycle state.
**Example:**
```typescript
// Source: repository store pattern in src/lib/stores/sessions.ts
if (sessionOp[sessionId] === "interrupting") {
  disableInterrupt = true;
  disableResume = true;
  disablePromptInput = true;
}
```

### Pattern 4: Compact dashboard feedback + rich detail panel
**What:** Keep row feedback minimal (status chip + spinner) while preserving richer error details in session detail panel.
**When to use:** Locked dashboard behavior and discretion on error presentation.
**Example:**
```typescript
// Source: locked phase decisions + existing Sidebar/SessionOutput role split
rowStatus = "Interrupting..."; // compact
detailError = "Unable to interrupt session after retry (10s)."; // detailed, per-session
```

### Pattern 5: Single-row resume with per-run namespaces
**What:** Keep one dashboard row per logical session and namespace runtime stream ordering by `run_id`.
**When to use:** All resumed runs in Phase 5.
**Example:**
```typescript
// Source: repository dedupe behavior in src/lib/stores/sessions.ts
// and Phase 5 decision addendum
const dedupeKey = `${sessionId}:${runId}:${event.type}:${event.data.seq}`;
```

### Pattern 6: Instrument-before-migrate transport strategy
**What:** Keep events now, instrument lag/overflow/seq integrity, migrate to channels only when defined triggers fire.
**When to use:** Phase 5 rollout and soak period.
**Example:**
```rust
// Source: existing overflow path in src-tauri/src/session/cli.rs
if overflow_detected {
    metrics.events_overflow_total += 1;
    emit_debug("transport-threshold-breach", session_id)?;
}
```

### Anti-Patterns to Avoid
- **Mapping `interrupted` to `Failed` in projection:** violates locked behavior and creates UX inconsistency.
- **Treating `interrupt` as global app state:** breaks per-session isolation and can block unrelated sessions.
- **Custom transcript replay for resume:** duplicates upstream semantics and increases drift risk.
- **Adding child session rows in Phase 5:** introduces schema/UX migration risk without requirement value.
- **Assuming event throughput is unbounded:** Tauri docs state events are not for high-throughput streaming; use channels if needed.
- **Relying on `(event.type, seq)` dedupe across resumed runs without namespace strategy:** can suppress valid resumed events.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Session continuation | Custom replay protocol of prior prompts/responses | Claude CLI `--resume` / `--continue` / `--fork-session` | Official semantics are stable and supported |
| Process stop lifecycle | Ad-hoc process termination scripts | Tokio `Child` `kill` / `wait` semantics + explicit deadline flow | Official async lifecycle behavior; avoids zombie/reap mistakes |
| Session identity discovery | Parsing IDs from arbitrary log text | Explicit `--session-id` assignment + stored mapping | Deterministic, testable, less parser fragility |
| Worktree management | Manual `.git/worktrees` file manipulation | `git worktree add/list/remove/prune/repair` | Official worktree invariants and cleanup behavior |
| Streaming transport upgrades | Custom frontend bridge protocol | Tauri `Channel` when event volume becomes high | Officially optimized for ordered streaming |

**Key insight:** In this phase, correctness depends on lifecycle authority and identity continuity, not UI complexity.

## Common Pitfalls

### Pitfall 1: Projection vocabulary drift (`Interrupted` vs `Failed`)
**What goes wrong:** Dashboard shows `Failed` while detail/status stream indicates interrupt success.
**Why it happens:** Current projection maps `interrupted` to failed vocabulary.
**How to avoid:** Add explicit `Interrupted` mapping in backend and frontend projection logic.
**Warning signs:** Same session renders different status across row/detail views.

### Pitfall 2: Interrupt race creates duplicate or conflicting terminal states
**What goes wrong:** Session emits multiple terminal transitions (`killed` then `interrupted`, etc.).
**Why it happens:** No idempotent operation gate for interrupt retries.
**How to avoid:** Use one per-session interrupt operation token and single terminal commit point.
**Warning signs:** Flickering status chips or repeated terminal events in timeline.

### Pitfall 3: Resume output disappears after successful resume
**What goes wrong:** Resume starts but UI shows little or no new output.
**Why it happens:** Dedupe key `(type, seq)` collides if sequence restarts on resumed runs.
**How to avoid:** Namespace sequence by run attempt or maintain monotonic sequence per session.
**Warning signs:** Backend logs show new events but timeline stays unchanged.

### Pitfall 4: Error handling leaks across sessions
**What goes wrong:** One failed interrupt disables controls or shows errors for other sessions.
**Why it happens:** Shared/global in-flight or error store instead of session-scoped state.
**How to avoid:** Store operation and error state keyed strictly by `session_id`.
**Warning signs:** Starting interrupt on one row affects other rows.

### Pitfall 5: Event transport overload assumptions
**What goes wrong:** High-volume event streams become laggy or lossy.
**Why it happens:** Using Tauri events for data volume they are not designed to handle.
**How to avoid:** Keep status events small; switch high-volume streams to channels.
**Warning signs:** UI lag under verbose streaming or delayed status updates.

### Pitfall 6: Premature lineage branching in Phase 5
**What goes wrong:** Resume introduces child rows, breaking selection/history assumptions and increasing bug surface.
**Why it happens:** Solving future lineage UX now instead of phase-scoped lifecycle requirements.
**How to avoid:** Keep one row per logical session; store run attempts as metadata (`run_id`, `resume_count`).
**Warning signs:** New `parent_session_id` migration appears in Phase 5 task list or sidebar starts showing hierarchy.

## Code Examples

Verified patterns from official sources:

### Resume and continue sessions
```bash
# Source: https://code.claude.com/docs/en/cli-reference
claude -p "Review this module"
claude -p "Now focus on error handling" --continue
claude -p "Continue this exact thread" --resume "550e8400-e29b-41d4-a716-446655440000"
```

### Structured output includes session metadata
```bash
# Source: https://code.claude.com/docs/en/headless
claude -p "Summarize this project" --output-format json
```

### Tauri emit for session status events
```rust
// Source: https://v2.tauri.app/develop/calling-frontend/
use tauri::{AppHandle, Emitter};
app.emit("session-event", payload)?;
```

### Tokio kill semantics
```rust
// Source: https://docs.rs/tokio/latest/tokio/process/struct.Child.html
child.kill().await?; // SIGKILL on Unix + wait
```

### Standard library caution about waiting/reaping
```rust
// Source: https://doc.rust-lang.org/std/process/struct.Child.html
let status = child.wait()?; // avoid leaving child as zombie
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Treat interrupt-like states as generic failure in dashboard | Explicit user-facing `Interrupted` lifecycle state | Required by Phase 5 locked decisions | Prevents semantic drift and preserves user intent |
| One-run mental model (session ends once) | Multi-turn lifecycle with resume/continue on same logical session | Claude CLI current flags + product requirements | Requires lifecycle state machine and idempotent transitions |
| Event-only IPC for all updates | Events for small status payloads, channels for high-volume ordered streams | Tauri v2 guidance (updated 2025-05-12) | Prevents transport bottlenecks as stream volume grows |
| Process stop as one-shot kill command | Bounded interrupt operation with retry and 10s error threshold | Phase 5 locked timing behavior | Improves UX predictability and failure handling |

**Deprecated/outdated:**
- Mapping `interrupted` to dashboard `Failed` is outdated for this phase.
- Treating session stop as only `kill_session` without interrupt-specific state is outdated for this phase.

## Open Questions

None blocking for Phase 5. Open questions from prior draft are now resolved by the Decision Addendum:

1. Resume lineage: **single-row model is required for Phase 5**.
2. Transport migration: **event transport remains in Phase 5 with explicit metric triggers for channel migration**.

## Sources

### Primary (HIGH confidence)
- Repository evidence:
  - `src-tauri/src/session/supervisor.rs`
  - `src-tauri/src/session/cli.rs`
  - `src-tauri/src/session/projection.rs`
  - `src-tauri/src/commands/session.rs`
  - `src-tauri/src/db/session.rs`
  - `src-tauri/src/db/mod.rs`
  - `src/lib/stores/sessions.ts`
  - `.planning/phases/05-session-lifecycle-control/05-CONTEXT.md`
- Claude CLI docs: https://code.claude.com/docs/en/cli-reference
- Claude CLI programmatic docs: https://code.claude.com/docs/en/headless
- Claude interactive controls (`Ctrl+C`): https://code.claude.com/docs/en/interactive-mode
- Local CLI verification: `claude --help`, `claude -v` (2.1.42)
- Tauri v2 docs (Calling Frontend from Rust): https://v2.tauri.app/develop/calling-frontend/ (Last updated 2025-05-12)
- Tokio child process docs: https://docs.rs/tokio/latest/tokio/process/struct.Child.html
- Rust std child process docs: https://doc.rust-lang.org/std/process/struct.Child.html (std 1.93.1)
- Git worktree manual: https://git-scm.com/docs/git-worktree (last updated in 2.53.0, 2026-02-02)

### Secondary (MEDIUM confidence)
- None.

### Tertiary (LOW confidence)
- None.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - validated by local manifests/binaries and official docs.
- Architecture: MEDIUM-HIGH - lifecycle model, lineage approach, and transport triggers are now prescriptive and implementation-ready.
- Pitfalls: MEDIUM-HIGH - confirmed by current code paths and official platform/process constraints; threshold calibration still needs runtime telemetry.

**Research date:** 2026-02-16
**Valid until:** 2026-03-16

# Phase 6: Persistence & History - Research

**Researched:** 2026-02-17
**Domain:** Durable session persistence and history reconstruction in Tauri + Rust + SQLite + Svelte
**Confidence:** MEDIUM-HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
### Restored dashboard view
- Default restored sort is active sessions first, then recent.
- Within active sessions, tie-break by newest created first.
- Row emphasis prioritizes session name over status/recency.
- If a session was running before shutdown, show last known state on reopen.
- Restored sessions show a subtle per-row restored badge (medium emphasis).
- Restored badge clears after the first new event for that session.
- Active-session status chips include a subtle recovery hint at startup.
- Interrupted sessions rank the same as other terminal states (completed/failed).
- No default filter is applied on reopen; show the full list.
- Sort can be changed by users and is remembered, but startup still applies the phase default behavior.

### Claude's Discretion
- Where sort controls are placed in the dashboard UI.
- Whether remembered sort is stored globally or by workspace/project.
- Whether/how to explain startup default override of remembered sort.
- Tie-break behavior for equal-timestamp terminal sessions when not otherwise specified.

### Deferred Ideas (OUT OF SCOPE)
None - discussion stayed within phase scope.
</user_constraints>

## Summary

Phase 6 is primarily a data-model and projection phase. The current backend already persists sessions and assistant messages in SQLite (`sessions`, `messages`) and reconciles stale startup rows, but it does **not** persist a complete event history (prompts, tool calls/results, approvals, status/error timeline) required by OUT-02. The frontend history loader (`list_session_messages`) can only hydrate message text, so history review after completion is incomplete today.

The most important planning decision is to treat persisted history as a first-class event log, not as message-only records. The app already has typed runtime events (`SessionEventPayload::{Message,Thinking,ToolCall,ToolResult,Status,Error}`) and deterministic session identity; Phase 6 should persist these canonical events with ordering and run-attempt metadata so the UI can reconstruct an accurate timeline after restart.

A second critical mismatch: startup reconciliation currently forces stale `starting|running` rows to `failed`, but the locked Phase 6 decision is to show last-known state on reopen with recovery hints. Planning must explicitly replace or conditionally alter reconciliation behavior so restored-running sessions are shown as restored/recovered (with badge/hint) rather than rewritten to failed at boot.

**Primary recommendation:** Add a durable `session_events` timeline table (with run-attempt identity and ordered sequence), switch history hydration to that table, and update startup reconciliation + dashboard sorting/projection to implement the locked restored-state behavior.

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| SQLite (bundled) via `rusqlite` | `rusqlite 0.31` | Durable local persistence for sessions and timeline | Already used in production path with WAL and transactional writes |
| Tauri commands/events | `tauri 2.x` | Frontend/backend IPC for session list + history + runtime updates | Existing architecture already uses `invoke` + `emit/listen` |
| Tokio process/event pipeline | `tokio 1.x` | Runtime event production and ordering | Existing supervisor/CLI parser already emits ordered session events |
| Svelte stores | `svelte 5` + `@tauri-apps/api ^2` | Dashboard projection and history hydration | Current stores already centralize all session state transitions |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `chrono` | `0.4` | RFC3339 timestamps for stable ordering + display | Persisting event time + restored/recovery markers |
| `uuid` | `1.x` | Session/run IDs and event row IDs | Run-attempt namespacing and idempotent inserts |
| Browser `localStorage` | Web standard | Remember user sort preference | Persisting user-selected sort mode across restarts |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `session_events` table | Keep using `messages` only | Cannot satisfy OUT-02 (missing tool calls, approvals, status/error timeline) |
| Persisted ordered timeline | Reconstruct from current in-memory store at startup | Loses events across app restart; non-durable and non-auditable |
| Keep last-known running + recovery hint | Force stale running to failed on startup | Violates locked restored-state behavior |

**Installation:**
```bash
# No new runtime dependencies required for baseline Phase 6.
```

## Architecture Patterns

### Recommended Project Structure
```
src-tauri/src/
├── db/
│   ├── mod.rs                # schema migration for session_events + indexes
│   └── session.rs            # insert/list timeline rows and restore queries
├── commands/
│   └── session.rs            # list_session_history command + startup reconciliation update
└── session/
    ├── cli.rs                # include run_id/session metadata in persisted events
    └── supervisor.rs         # startup recovery semantics, terminal persistence authority

src/lib/
├── stores/sessions.ts        # restore-aware sort model + history hydration from event log
└── components/Sidebar.svelte # sort controls + restored badge/hint behavior
```

### Pattern 1: Event-sourced session history
**What:** Persist canonical runtime events (message/thinking/tool/status/error/approval) into a single ordered timeline table keyed by session and run attempt.
**When to use:** Every backend event that should survive app restart and be reviewable post-completion.
**Example:**
```sql
-- Source: repository schema style in src-tauri/src/db/mod.rs + SQLite docs
CREATE TABLE IF NOT EXISTS session_events (
  id TEXT PRIMARY KEY,
  session_id TEXT NOT NULL,
  run_id TEXT,
  seq INTEGER NOT NULL,
  event_type TEXT NOT NULL,
  payload_json TEXT NOT NULL,
  timestamp TEXT NOT NULL,
  FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE,
  UNIQUE(session_id, run_id, seq)
);
```

### Pattern 2: Backend-authoritative restored projection
**What:** On startup, preserve last-known state for previously in-flight sessions and project recovery metadata (`restored=true`, `recovery_hint=true`) instead of coercing to failed.
**When to use:** Initial app bootstrap after unclean shutdown or app restart with prior active sessions.
**Example:**
```rust
// Source: repository startup hook in src-tauri/src/commands/session.rs and lib.rs
// Keep status as last known and mark as restored, rather than overwriting to failed.
let restored_ids = db.mark_stale_sessions_restored()?;
emit_startup_recovery_hints(&app, &restored_ids)?;
```

### Pattern 3: Two-layer sort model (startup override + remembered preference)
**What:** Apply phase-default restored sort at startup, then allow user sort changes and persist preference independently.
**When to use:** Dashboard rows after hydration and all subsequent user-initiated sort changes.
**Example:**
```typescript
// Source: locked phase decisions + existing localStorage usage in sessions.ts
const startupSort = "active-first-then-recent";
const rememberedSort = loadRememberedSort();
const effectiveSort = isStartupRestore ? startupSort : rememberedSort;
```

### Pattern 4: Badge lifecycle tied to first post-restore event
**What:** `restored` badge is per-row state that clears only after first new event for that session.
**When to use:** Sessions present at startup from persisted store.
**Example:**
```typescript
if (session.wasRestored && incomingEvent.session_id === session.id) {
  session.wasRestored = false;
}
```

### Anti-Patterns to Avoid
- **Message-only history:** storing only assistant text in `messages` and trying to infer tools/status later.
- **Frontend-only history reconstruction:** deriving durable history solely from volatile store state.
- **Startup hard-fail rewrite:** changing stale running rows to `failed` at boot when locked behavior requires last-known state display.
- **Unstable timeline ordering:** ordering history only by timestamp without deterministic tie-break (`seq`, then id).

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Durable timeline persistence | Ad-hoc JSON blobs in localStorage | SQLite normalized event table + indexed queries | Reliable restart safety, queryability, and ordering |
| Event transport for bulk streams | Large payloads over Tauri global events | Tauri channels for high-throughput streams | Official guidance: events are for small payloads |
| Transaction safety | Multi-step writes without transaction boundary | `rusqlite` transactions (`Immediate` where needed) | Avoid partial writes and torn history rows |
| Restart storage location | Hardcoded DB path strings | `app.path().app_data_dir()` | Cross-platform app data semantics in Tauri |

**Key insight:** This phase should optimize for replay correctness (what happened, in order) rather than for minimal schema changes.

## Common Pitfalls

### Pitfall 1: OUT-02 gap from message-only storage
**What goes wrong:** Completed sessions reopen with text output but missing tool calls/results, prompts, approvals, and status transitions.
**Why it happens:** `messages` table captures only assistant content inserts.
**How to avoid:** Persist each canonical `SessionEventPayload` variant to `session_events` and hydrate from it.
**Warning signs:** History panel differs significantly between live run and post-restart view.

### Pitfall 2: Resume/run seq collisions corrupt history ordering
**What goes wrong:** Later run events overwrite/dedupe earlier run events when sequence restarts.
**Why it happens:** Sequence is per spawned process and currently restarts; dedupe may ignore run boundary.
**How to avoid:** Include `run_id` in persistence unique key and in frontend dedupe key.
**Warning signs:** Missing resumed events or duplicate suppression after resume.

### Pitfall 3: Reconciliation conflicts with locked restore behavior
**What goes wrong:** Sessions that were running before shutdown appear as failed immediately on reopen.
**Why it happens:** Startup reconciliation currently rewrites `starting|running` -> `failed`.
**How to avoid:** Switch reconciliation to restored metadata + last-known-state projection.
**Warning signs:** No visible recovery hint/badge and immediate failed chips after restart.

### Pitfall 4: Remembered sort silently ignored with no affordance
**What goes wrong:** Users think sort preference is broken.
**Why it happens:** Startup override is applied but not explained.
**How to avoid:** Show subtle one-time startup hint and preserve remembered preference for post-start interactions.
**Warning signs:** Repeated user toggling of sort immediately after reopen.

### Pitfall 5: Non-deterministic tie-break for equal timestamps
**What goes wrong:** Session row order appears to jump between reloads.
**Why it happens:** Equal timestamps without stable secondary sort.
**How to avoid:** Define explicit deterministic tie-break (recommend `created_at DESC`, then `id DESC`).
**Warning signs:** Snapshot tests flake on row ordering.

## Code Examples

Verified patterns from official sources and current repository:

### Persist each event in the runtime reducer loop
```rust
// Source: repository event loop in src-tauri/src/commands/session.rs
match &event.payload {
    SessionEventPayload::Message { content } => {
        db.insert_session_event(&event.session_id, run_id, event.seq, "message", &event, &event.timestamp)?;
        db.insert_session_message(&event.session_id, "assistant", content, &event.timestamp)?;
    }
    SessionEventPayload::ToolCall { .. }
    | SessionEventPayload::ToolResult { .. }
    | SessionEventPayload::Thinking { .. }
    | SessionEventPayload::Status { .. }
    | SessionEventPayload::Error { .. } => {
        db.insert_session_event(&event.session_id, run_id, event.seq, "event", &event, &event.timestamp)?;
    }
}
```

### Use transaction boundaries for session state + history writes
```rust
// Source: SQLite transaction semantics + existing repository pattern
let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
tx.execute("UPDATE sessions SET status = ?1, updated_at = ?2 WHERE id = ?3", params![status, now, id])?;
tx.execute("INSERT INTO session_events (...) VALUES (...)", params![...])?;
tx.commit()?;
```

### Persist remembered sort preference safely in frontend
```typescript
// Source: existing localStorage approach in src/lib/stores/sessions.ts
const SORT_KEY = "lulu:dashboard-sort";
const loadSort = () => window.localStorage.getItem(SORT_KEY) ?? "active-first-then-recent";
const saveSort = (value: string) => window.localStorage.setItem(SORT_KEY, value);
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Session-only + message-only persistence | Session table includes lifecycle metadata (`resume_count`, `active_run_id`) but history still message-only | Phase 5 groundwork | Phase 6 can extend naturally to full event history |
| Startup stale rows forced to failed | Locked Phase 6 behavior requires last-known display with recovery indicators | Phase 6 decisions (2026-02-17) | Reconciliation logic must be revised |
| Single default created-at sort | Locked Phase 6 restored sort: active-first then recent, with remembered user sort after startup | Phase 6 decisions (2026-02-17) | Dashboard projection becomes mode-aware |

**Deprecated/outdated:**
- Treating `list_session_messages` as complete history is outdated for OUT-02.
- Treating startup reconciliation as unconditional failure rewrite is outdated for locked Phase 6 restore behavior.

## Open Questions

1. **Sort preference scope (global vs workspace/project)**
   - What we know: This is explicitly Claude's discretion.
   - What's unclear: Whether users expect per-repo behavior when multiple repos are used.
   - Recommendation: Default to per-workspace/project keying (e.g., `lulu:dashboard-sort:<workspace-id>`) to avoid cross-project surprise.

2. **Tie-break for equal-timestamp terminal sessions**
   - What we know: Discretionary and currently unspecified outside active-session rule.
   - What's unclear: Which deterministic fallback should be canonical.
   - Recommendation: Use `created_at DESC`, then `id DESC` for deterministic ordering and stable tests.

3. **Approval event shape in persisted history**
   - What we know: Requirement includes approvals; current event enum does not yet expose explicit approval variant.
   - What's unclear: Whether approval arrives as a dedicated CLI stream event or inferred from existing payloads.
   - Recommendation: Add a normalized persisted event type (`approval`) in storage projection even if source mapping initially comes from parsed stream subtypes.

## Sources

### Primary (HIGH confidence)
- Repository evidence:
  - `src-tauri/src/db/mod.rs` (WAL, schema, migration helpers)
  - `src-tauri/src/db/session.rs` (session/message persistence, startup reconcile behavior)
  - `src-tauri/src/commands/session.rs` (runtime event handling, current persistence boundaries)
  - `src-tauri/src/session/events.rs` (canonical event payload types)
  - `src-tauri/src/session/cli.rs` (stream parsing, resume/new spawn modes, seq behavior)
  - `src/lib/stores/sessions.ts` (dashboard sort/hydration/localStorage patterns)
  - `.planning/phases/06-persistence-history/06-CONTEXT.md`
- SQLite WAL docs: https://www.sqlite.org/wal.html (updated 2025-05-31)
- SQLite transaction docs: https://www.sqlite.org/lang_transaction.html (updated 2025-05-12)
- Tauri docs (calling frontend): https://v2.tauri.app/develop/calling-frontend/ (updated 2025-05-12)
- Tauri docs (calling rust): https://v2.tauri.app/develop/calling-rust/ (updated 2025-11-19)
- Claude docs (programmatic/CLI output and resume flags):
  - https://code.claude.com/docs/en/headless
  - https://code.claude.com/docs/en/cli-reference

### Secondary (MEDIUM confidence)
- MDN localStorage behavior and persistence semantics: https://developer.mozilla.org/en-US/docs/Web/API/Window/localStorage (updated 2025-11-30)

### Tertiary (LOW confidence)
- None.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - fully validated from local manifests/code + official docs.
- Architecture: MEDIUM-HIGH - repository constraints are clear; approval-event normalization needs implementation detail confirmation.
- Pitfalls: MEDIUM-HIGH - directly evidenced by current persistence boundaries and startup behavior.

**Research date:** 2026-02-17
**Valid until:** 2026-03-19

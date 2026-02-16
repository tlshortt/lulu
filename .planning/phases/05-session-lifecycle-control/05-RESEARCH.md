# Phase 5: Session Lifecycle Control - Research

**Researched:** 2026-02-16
**Domain:** Interrupt and resume lifecycle for long-running Claude CLI sessions in Tauri + Svelte + Rust
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

Phase 5 should be planned as a lifecycle state-machine expansion, not as a small UI-only feature. The current backend already owns process lifecycle (`SessionSupervisor`) and terminal transition idempotency, but only supports force-kill semantics (`kill_session`) and terminal states `completed|failed|killed`. The frontend currently normalizes `interrupted` to `Failed` in dashboard projection. That directly conflicts with the locked decision that successful interrupt sets status to `Interrupted`.

For resume, the standard path is to use Claude CLI built-ins rather than custom replay: `--resume/-r` (specific session), `--continue/-c` (most recent), and optionally `--fork-session` to branch. Claude CLI also supports explicit `--session-id` assignment, which is the most reliable way to make session identity deterministic from Lulu's side. This avoids brittle parsing assumptions and enables robust resume wiring.

**Primary recommendation:** Implement an explicit lifecycle model (`running -> interrupting -> interrupted -> resuming -> running|failed`) with backend-authoritative transitions, use Claude CLI `--resume`/`--session-id` for continuation, and keep dashboard/detail UX synchronized through one session-scoped interrupt operation with 10s bounded retry behavior.

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| Tauri | `2.x` | Rust command surface + frontend eventing | Existing app architecture and command/event model already in place |
| Tokio | `1.x` | Child process lifecycle and async supervision | Existing runtime already uses `tokio::process::Child` and async tasks |
| Claude CLI | `2.1.42` (installed) | Session execution, resume/continue semantics | Officially supports `--resume`, `--continue`, `--fork-session`, `--session-id` |
| rusqlite | `0.31` | Session/message persistence | Existing WAL-backed local persistence with transactional updates |
| Svelte 5 + SvelteKit 2 | `^5.0.0` / `^2.9.0` | Lifecycle controls and detail/dashboard synchronization | Existing stores/components already centralize session state |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `@tauri-apps/api` | `^2` | `invoke` + event listeners in stores | Session control commands (`interrupt`, `resume`) and status updates |
| Rust std/tokio process APIs | std `1.93.1`, tokio `1.49.0` docs | Kill/wait semantics, process id lookup | Interrupt timeout/retry flow and cleanup correctness |
| Git worktree | `2.50.1` installed | Per-session filesystem isolation retained across resume | Reuse existing per-session worktree strategy for resumed turns |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| CLI-native resume (`--resume`) | Manual replay of previous messages into new process | Manual replay is fragile and diverges from Claude's native session model |
| Backend-owned lifecycle transitions | UI-only optimistic transitions | UI-only transitions race with process state and violate per-session error isolation |
| Deterministic `--session-id` storage | Parse and infer session identity from streamed output | Parsing-only approach is brittle if output/event shapes change |

**Installation:**
```bash
# No new JS dependencies required for baseline implementation.
# Keep existing Tauri/Svelte/rusqlite stack.
```

## Architecture Patterns

### Recommended Project Structure
```
src-tauri/src/
├── commands/
│   └── session.rs          # add interrupt_session + resume_session commands
├── session/
│   ├── supervisor.rs       # add interrupt state + bounded retry/deadline helpers
│   └── cli.rs              # add spawn_resume variant and stable arg composition
└── db/
    └── session.rs          # add lifecycle metadata columns/query updates

src/lib/
├── stores/sessions.ts      # per-session interrupting/resuming UI state + command wiring
└── components/
    ├── Sidebar.svelte      # row-level interrupt control (compact spinner/chip feedback)
    └── SessionOutput.svelte # detail-level interrupt/resume controls + disabled states
```

### Pattern 1: Explicit lifecycle state machine with transient control states
**What:** Add transient states (`interrupting`, optional `resuming`) distinct from terminal states, with backend as source of truth.
**When to use:** All interrupt/resume flows for LIFE-01/LIFE-02/LIFE-04.
**Example:**
```rust
// Source: repository pattern + tokio Child docs
match status {
    "running" => begin_interrupt(session_id).await?,
    "interrupting" => Ok(()), // idempotent
    "completed" | "interrupted" => begin_resume(session_id, prompt).await?,
    _ => Err("Session is not interruptible/resumable".to_string()),
}
```

### Pattern 2: Bounded interrupt with silent retry and deadline
**What:** Execute interrupt attempt, verify terminal transition, silently retry once, then surface error at 10s.
**When to use:** Exactly matches locked interrupt timing behavior.
**Example:**
```rust
// Source: https://docs.rs/tokio/latest/tokio/process/struct.Child.html
let interrupt_deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
attempt_interrupt(session_id).await?;
if !wait_until_stopped(session_id, interrupt_deadline).await {
    attempt_interrupt(session_id).await?; // silent retry
    if !wait_until_stopped(session_id, interrupt_deadline).await {
        return Err("Session did not stop within 10s".to_string());
    }
}
```

### Pattern 3: CLI-native resume with deterministic identity
**What:** Persist a Claude session identity and resume via `--resume` (and optionally `--fork-session`) rather than synthetic replay.
**When to use:** Every continuation from completed/interrupted sessions.
**Example:**
```bash
# Source: https://code.claude.com/docs/en/cli-reference.md
claude -p "Initial prompt" --session-id "550e8400-e29b-41d4-a716-446655440000"
claude -p "Follow-up prompt" --resume "550e8400-e29b-41d4-a716-446655440000"
```

### Anti-Patterns to Avoid
- **Overloading `killed` as user-visible interrupt success:** conflicts with locked `Interrupted` success state and causes UX ambiguity.
- **Reusing per-session sequence numbers from 1 on resume without offset strategy:** collides with current frontend dedupe by `(type, seq)` and can drop new events.
- **UI-only disable/enable toggles without backend operation token:** creates double-submit and stale-control races under retry/timeout paths.
- **Hand-built resume transcript replay:** brittle against Claude output/schema changes and unnecessary given CLI resume flags.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Session continuation | Custom message replay protocol | Claude CLI `--resume` / `--continue` / `--fork-session` | Official behavior, less drift, fewer edge cases |
| Process termination semantics | Ad-hoc PID shelling and platform branches everywhere | `tokio::process::Child` kill/wait lifecycle as primary primitive | Documented semantics include wait/reap behavior |
| Session identity inference | Regex scraping of arbitrary output lines | Explicit `--session-id` persistence | Deterministic, stable, planner/test-friendly |
| Worktree session isolation | Direct `.git` internals manipulation | `git worktree add/list/remove/prune` | Official safety rules and cleanup semantics already integrated |

**Key insight:** The hard part in this phase is lifecycle correctness across retries/timeouts and resume identity, not rendering another button.

## Common Pitfalls

### Pitfall 1: Interrupt success shown before backend terminal ownership
**What goes wrong:** UI shows `Interrupted` but backend later emits conflicting terminal status (`failed`/`killed`).
**Why it happens:** Optimistic UI transition without idempotent backend terminal transition gate.
**How to avoid:** Keep transition authority in backend and emit a single terminal status outcome.
**Warning signs:** Flaky status chip transitions, status flicker, mismatched detail/header state.

### Pitfall 2: Resume event dedupe drops valid output
**What goes wrong:** Resumed run appears silent or partially missing.
**Why it happens:** Frontend dedupe currently keys status/events by `(type, seq)`; resumed stream may restart sequence numbers.
**How to avoid:** Introduce per-run sequence namespace or monotonic session-level sequence offset.
**Warning signs:** Resume command succeeds but no new timeline entries appear.

### Pitfall 3: Force-kill semantics undermine resumability
**What goes wrong:** Session marked interrupted but cannot resume reliably.
**Why it happens:** Current `kill()` is forceful (`SIGKILL` semantics) and may bypass graceful CLI-level interruption behavior.
**How to avoid:** Define explicit interrupt strategy and verify it preserves resumable session continuity.
**Warning signs:** Repeated resume failures after successful interrupt UX.

### Pitfall 4: Projection vocabulary drift between Phase 3 and Phase 5
**What goes wrong:** Dashboard and detail disagree (`Failed` vs `Interrupted`) for same lifecycle event.
**Why it happens:** Existing Phase 3 projection normalizes `interrupted` to `Failed`.
**How to avoid:** Update projection contract intentionally for Phase 5 requirements and back it with tests.
**Warning signs:** Row chip says Failed while detail header says interrupted.

## Code Examples

Verified patterns from official sources:

### Resume a specific Claude session
```bash
# Source: https://code.claude.com/docs/en/cli-reference.md
claude -r "auth-refactor" "Finish this PR"
claude -c -p "Check for type errors"
```

### Deterministic session identity
```bash
# Source: https://code.claude.com/docs/en/cli-reference.md
claude -p "Initial task" --session-id "550e8400-e29b-41d4-a716-446655440000"
```

### Process kill semantics include wait in tokio
```rust
// Source: https://docs.rs/tokio/latest/tokio/process/struct.Child.html
child.kill().await?; // tokio kill is SIGKILL + wait
```

### Tauri event/listener boundary guidance
```rust
// Source: https://v2.tauri.app/develop/calling-frontend/
use tauri::{AppHandle, Emitter};
app.emit("session-status", payload)?;
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Single-run session rows with terminal end states only | Multi-turn lifecycle per session (interrupt + resume) | Phase 5 scope | Requires explicit non-terminal lifecycle transitions |
| `kill_session` as only stop mechanism | Interrupt operation with retry/deadline and user-facing `Interrupted` | Phase 5 locked decisions | Requires new state + UX gating semantics |
| Implicit/unknown Claude session identity | Explicit resume contract using `--resume` and `--session-id` | Claude CLI current docs | Makes continuation deterministic and testable |

**Deprecated/outdated:**
- Treating `interrupted` as dashboard `Failed` is outdated for this phase's locked behavior.

## Open Questions

1. **Interrupt transport choice for Claude subprocesses**
   - What we know: current implementation uses force kill; tokio documents kill behavior clearly.
   - What's unclear: best graceful signal strategy (if any) for preserving resumability before fallback kill.
   - Recommendation: plan a compatibility spike and lock one cross-platform interrupt strategy before full UI wiring.

2. **Session model for resume lineage**
   - What we know: CLI supports resume and fork; product requires preserved history and session isolation.
   - What's unclear: whether resumed turns stay in one Lulu session row or branch into child rows with lineage.
   - Recommendation: choose one model in first plan task and encode DB schema + UX tests around it.

## Sources

### Primary (HIGH confidence)
- Repository evidence:
  - `src-tauri/src/commands/session.rs`
  - `src-tauri/src/session/supervisor.rs`
  - `src-tauri/src/session/cli.rs`
  - `src-tauri/src/session/projection.rs`
  - `src-tauri/src/db/session.rs`
  - `src/lib/stores/sessions.ts`
  - `src/lib/components/Sidebar.svelte`
  - `src/lib/components/SessionOutput.svelte`
  - `.planning/phases/05-session-lifecycle-control/05-CONTEXT.md`
- Claude CLI local help (`claude --help`) and version (`claude -v`): confirms available resume/session flags in installed binary.
- Claude CLI Reference: https://code.claude.com/docs/en/cli-reference.md
- Claude CLI Headless/Agent SDK CLI usage: https://code.claude.com/docs/en/headless.md
- Tauri v2 Calling Frontend from Rust: https://v2.tauri.app/develop/calling-frontend/ (Last updated: 2025-05-12)
- Tokio `Child` docs: https://docs.rs/tokio/latest/tokio/process/struct.Child.html
- Rust std `Child` docs: https://doc.rust-lang.org/std/process/struct.Child.html
- Git worktree manual: https://git-scm.com/docs/git-worktree (latest 2.53.0, 2026-02-02)

### Secondary (MEDIUM confidence)
- SQLite WAL behavior/perf/concurrency guidance: https://www.sqlite.org/wal.html (updated 2025-05-31)
- Claude common workflows (session resume/worktree operational patterns): https://code.claude.com/docs/en/common-workflows.md
- Claude interactive mode (`Ctrl+C` meaning in interactive UX): https://code.claude.com/docs/en/interactive-mode.md

### Tertiary (LOW confidence)
- None.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - confirmed by local manifests, installed binaries, and official docs.
- Architecture: MEDIUM-HIGH - strongly grounded in repo/runtime patterns; interrupt transport nuance remains undecided.
- Pitfalls: MEDIUM - major failure modes are clear, but final resume lineage decision is still open.

**Research date:** 2026-02-16
**Valid until:** 2026-03-16

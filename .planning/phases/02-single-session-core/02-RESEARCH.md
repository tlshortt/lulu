# Phase 2: Single Session Core - Research

**Researched:** 2026-02-15
**Domain:** Single-session runtime hardening and real-time Claude stream rendering in Tauri v2
**Confidence:** HIGH

## User Constraints

No phase `02-CONTEXT.md` exists yet, so there are no locked/deferred decisions to inherit.

Constraints provided in scope:
- Stack is fixed: Tauri v2 + Svelte 5 + Rust
- Keep existing primary event pipeline (`session-event`) with typed structured payloads
- Reuse existing structured output rendering from Phase 1
- Keep Claude integration behind current `cc-sdk` abstraction and pinned dependency strategy

## Summary

Phase 2 should be planned as a hardening and contract-alignment phase, not a greenfield build. The repository already has most primitives in place: session spawn API, SQLite persistence, event fanout, and UI rendering for message/tool/status/error blocks. The core planning value is to close correctness gaps between current behavior and Phase 2 success criteria.

Most risk is concentrated in event contract fidelity and lifecycle correctness. Current code parses a test-friendly JSON shape (`{type,data}`), but Claude CLI streaming (`--output-format stream-json`) emits raw stream events that need normalization before they can populate the existing typed `session-event` pipeline. Final status persistence is also incomplete: session completion/error can emit frontend events without reliably updating DB status, which can leave stale `running` rows and inconsistent UI headers.

**Primary recommendation:** Plan Phase 2 around a single canonical event normalization layer (CLI raw stream -> internal `SessionEventPayload`) plus deterministic lifecycle transitions (`running -> completed|failed`) persisted in DB and emitted once.

## Standard Stack

### Core
| Library | Version in repo | Purpose | Why standard here |
|---|---|---|---|
| Tauri | `tauri = "2"` | Desktop runtime + IPC | Existing app shell and command/event model already built |
| Svelte | `^5.0.0` | Frontend rendering | Existing runes-based UI and stores implemented |
| SvelteKit | `^2.9.0` + adapter-static | SPA frontend for Tauri | Required by Tauri SvelteKit guidance |
| tokio | `1` | Async subprocess and task orchestration | Already used by CLI spawning/session management |
| rusqlite | `0.31` | Session persistence | Existing DB layer + WAL pragmas already in place |

### Supporting
| Library | Version in repo | Purpose | When to use |
|---|---|---|---|
| `@tauri-apps/api` | `^2` | `invoke` + event listeners | Frontend command calls and stream subscriptions |
| `serde` / `serde_json` | `1` | Typed payload serialization | `session-event` payload contract |
| `thiserror` | `1.0` | backend error modeling | Converting internal errors to command-safe messages |
| `uuid`, `chrono` | current | IDs and timestamps | Session identity + event ordering metadata |

## Implementation Approach

1. **Canonicalize CLI output mode**
   - Spawn Claude in print/stream mode with explicit flags (`-p`, `--output-format stream-json`, and optionally `--verbose`/`--include-partial-messages` based on UX target).
   - Treat raw stdout lines as transport format only; do not couple UI to raw stream schema.

2. **Normalize to existing typed event contract**
   - Convert raw stream events to internal `SessionEventPayload` variants (`message`, `thinking`, `tool_call`, `tool_result`, `status`, `error`) in one parser module.
   - Keep `session-event` as the sole structured pipeline; avoid dual-path divergence.

3. **Make lifecycle transitions deterministic**
   - Persist `running` at spawn success.
   - Persist terminal state exactly once (`completed` or `failed`) based on process exit and parse/runtime failures.
   - Emit terminal frontend status once, from the same decision point.

4. **Harden single-session runtime behavior**
   - Validate working directory before spawn (friendly error path, no zombie DB state).
   - Keep stdout/stderr readers draining until process end to avoid pipe backpressure hazards.
   - Ensure child cleanup path remains reliable on app close.

## Architecture Pattern Alignment (Current Repo)

- **Pattern to keep:** `invoke("spawn_session")` starts backend process, frontend subscribes to stream events (`src/lib/stores/sessions.ts`, `src-tauri/src/commands/session.rs`).
- **Pattern to keep:** one primary structured event channel (`session-event`) rendered by typed event UI (`src/lib/components/SessionOutput.svelte`).
- **Pattern to adjust:** current fallback event channels (`session-output`, `session-complete`, `session-error`) can duplicate semantics and produce split-brain state; Phase 2 planning should define `session-event` as canonical and keep compatibility shims minimal.
- **Pattern to adjust:** DB session status updates are not fully synchronized with emitted terminal events; plan should enforce DB/UI/event consistency from one lifecycle reducer in Rust.

## File-Level Suggestions

- `src-tauri/src/session/cli.rs`
  - Add explicit Claude CLI output flags for stream-json mode.
  - Replace/extend `parse_json_event` to handle real stream event shapes from Claude Agent SDK/CLI streaming format.
  - Map stream deltas to buffered message/thinking/tool events suitable for current UI.

- `src-tauri/src/session/events.rs`
  - Keep existing typed payload enum; consider tightening `status` into explicit terminal/non-terminal values (`running`, `completed`, `failed`) to reduce string drift.

- `src-tauri/src/commands/session.rs`
  - Centralize terminal status handling: update DB and emit terminal event from one path only.
  - Validate working directory pre-spawn and update DB status on spawn failure.
  - Reduce duplicate emissions between process poll loop and event-consumer loop.

- `src-tauri/src/db/session.rs`
  - Ensure update helpers cover `completed` and `failed` transitions consistently.

- `src/lib/stores/sessions.ts`
  - Prefer routing all structured output through `session-event`.
  - Keep message buffering only where needed for chunk assembly; avoid re-assembling already complete backend messages.
  - Ensure final status in store aligns with backend terminal status source.

- `src/lib/types/session.ts`
  - Confirm event discriminated union exactly matches backend payload normalization target (including `thinking`).

- `src/lib/components/SessionOutput.svelte`
  - Minimal change expected; verify thinking/tool rendering works with normalized events and terminal status labels.

## Don't Hand-Roll

| Problem | Do not build | Use instead | Why |
|---|---|---|---|
| High-throughput raw streaming transport | Custom websocket bridge | Tauri channels/events with existing IPC patterns | Lower complexity, already integrated into app |
| Ad-hoc per-component event interpretation | UI-specific parsers | One backend normalization layer | Prevents schema drift and duplicate bugs |
| Custom process lifecycle scheduler | Homegrown polling framework | `tokio::process::Child` + explicit wait/kill patterns | Well-defined kill/wait semantics and cleanup behavior |

## Risks and Pitfalls

1. **Schema mismatch with real Claude stream events (HIGH)**
   - Current parser assumes `{type,data}` test shape; production stream-json emits event-style frames.
   - Mitigation: add fixture tests from real/representative stream-json transcripts.

2. **Inconsistent terminal status (HIGH)**
   - DB can remain `running` while UI receives completion event.
   - Mitigation: single terminal reducer updates DB + emits event atomically.

3. **Duplicate terminal emissions (MEDIUM)**
   - `session-complete` can fire from both status payload path and child exit poll path.
   - Mitigation: idempotent terminal transition guard.

4. **Event listener lifetime leaks in SPA context (MEDIUM)**
   - Tauri docs note listener cleanup responsibility for long-lived SPA contexts.
   - Mitigation: keep singleton initialization guard (already present) and explicitly document lifecycle.

5. **Process cleanup edge cases (MEDIUM)**
   - Tokio child behavior requires explicit kill/wait discipline for strict cleanup guarantees.
   - Mitigation: ensure kill path is awaited and child handles are removed deterministically.

## Verification Strategy

### Backend
- Extend integration tests around `spawn_with_events` to assert:
  - ordered `seq` values,
  - mapping from stream-json raw frames to typed payloads,
  - exactly one terminal status event,
  - DB terminal status persisted (`completed`/`failed`).
- Add failure-path test: invalid working directory -> command returns error, DB status not left in `running`.

### Frontend
- Keep store isolation tests and add checks for:
  - thinking visibility toggle with real normalized event shapes,
  - no duplicate terminal status rendering,
  - final status badge reflects persisted backend state after completion/error.

### End-to-end smoke
- Manual run against actual Claude CLI with stream-json flags:
  - launch named session with custom prompt + working dir,
  - observe live text/thinking/tool call/tool result stream,
  - verify app remains responsive to completion,
  - verify final status is accurate in sidebar and output panel.

## Explicit Recommendations

1. Adopt one canonical data flow for Phase 2: **Claude raw stream -> Rust normalization -> `session-event` -> Svelte store -> UI**.
2. Prioritize parser correctness with real stream-json fixtures before UI polish work.
3. Make terminal status state machine explicit and idempotent in backend command/session manager paths.
4. Treat `session-output` as compatibility-only; avoid adding new behavior to that side channel.
5. Add acceptance checks directly tied to success criteria (launch params, live stream types, no crash, terminal status correctness).

## Open Questions

1. Should Phase 2 expose full token-level partial text, or only message-level chunks for UX consistency?
   - Recommendation: stay message-level by default, but preserve enough raw metadata to support finer streaming later.

2. Should terminal status vocabulary be `complete` or `completed` everywhere?
   - Recommendation: pick one canonical value in backend and map legacy aliases at boundaries only.

## Sources

### Primary (HIGH confidence)
- Tauri docs: Calling Rust from frontend (commands/events): https://v2.tauri.app/develop/calling-rust/
- Tauri docs: Calling frontend from Rust (events/channels): https://v2.tauri.app/develop/calling-frontend/
- Tauri docs: SvelteKit integration (SPA/SSR guidance): https://v2.tauri.app/start/frontend/sveltekit/
- Tokio docs: `tokio::process::Child` semantics: https://docs.rs/tokio/latest/tokio/process/struct.Child.html
- Tokio docs: `tokio::process::Command` semantics: https://docs.rs/tokio/latest/tokio/process/struct.Command.html
- Claude Code CLI reference (flags/output formats): https://code.claude.com/docs/en/cli-reference.md
- Claude Code headless/programmatic mode (`-p`, `stream-json`): https://code.claude.com/docs/en/headless.md
- Agent SDK streaming event model: https://platform.claude.com/docs/en/agent-sdk/streaming-output
- Repo implementation references:
  - `src-tauri/src/session/cli.rs`
  - `src-tauri/src/commands/session.rs`
  - `src-tauri/src/session/events.rs`
  - `src/lib/stores/sessions.ts`
  - `src/lib/components/SessionOutput.svelte`

### Secondary (MEDIUM confidence)
- rusqlite docs (busy timeout + transaction behavior):
  - https://docs.rs/rusqlite/latest/rusqlite/struct.Connection.html#method.busy_timeout
  - https://docs.rs/rusqlite/latest/rusqlite/enum.TransactionBehavior.html

## Metadata

**Confidence breakdown:**
- Implementation approach: HIGH (directly aligned with current repo + official Tauri/Tokio/Claude docs)
- Architecture alignment: HIGH (verified from current code paths)
- Risks/pitfalls: HIGH (confirmed by code inspection and runtime semantics docs)
- Verification strategy: MEDIUM-HIGH (repo test scaffolding exists; some assertions require new fixtures)

**Research date:** 2026-02-15
**Valid until:** 2026-03-15

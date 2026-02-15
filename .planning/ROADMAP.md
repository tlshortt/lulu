# Roadmap: Lulu

## Overview

Lulu delivers a native desktop dashboard for orchestrating multiple Claude Code sessions in parallel. This roadmap moves from foundational architecture (Tauri + Svelte + cc-sdk integration) through single-session implementation, to multi-session orchestration with approvals, lifecycle control, and persistence. The journey culminates in a production-ready "mission control" for parallel AI-assisted development.

## Phases

**Phase Numbering:**
- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

Decimal phases appear between their surrounding integers in numeric order.

- [x] **Phase 1: Foundation & Architecture** - Tauri + Svelte + cc-sdk project scaffold with database layer
- [x] **Phase 2: Single Session Core** - One working Claude Code session with streaming output
- [ ] **Phase 3: Multi-Session Orchestration** - Parallel sessions with dashboard and git worktree integration
- [ ] **Phase 4: Security & Approval System** - Tool approval prompts with auto-approve rules engine
- [ ] **Phase 5: Session Lifecycle Control** - Interrupt and resume capabilities with error isolation
- [ ] **Phase 6: Persistence & History** - Session persistence across restarts and history review

## Phase Details

### Phase 1: Foundation & Architecture
**Goal**: Establish Tauri + Svelte 5 + cc-sdk infrastructure with SQLite persistence and core IPC patterns
**Depends on**: Nothing (first phase)
**Requirements**: None directly (infrastructure foundation)
**Success Criteria** (what must be TRUE):
  1. Tauri desktop app launches with Svelte 5 frontend
  2. SQLite database connection established with proper write serialization
  3. Rust backend can spawn and monitor a single cc-sdk process
  4. Basic IPC channel streams events from Rust to Svelte
  5. Project addresses 6 critical pitfalls (SQLite concurrency, bounded channels, IPC blocking, process zombies, cc-sdk API stability, client isolation)
**Plans**: 8 plans

Plans:
- [x] 01-01-PLAN.md — Tauri + Svelte scaffold with dark UI
- [x] 01-02-PLAN.md — SQLite database layer with WAL mode
- [x] 01-03-PLAN.md — cc-sdk CLI spawner with IPC
- [x] 01-04-PLAN.md — Fix Svelte app shell render (runes config + init)
- [x] 01-05-PLAN.md — Structured session output UI + buffering
- [x] 01-06-PLAN.md — Lint/format enforcement + unit test scaffolds
- [x] 01-07-PLAN.md — CLI spawn + IPC integration tests
- [x] 01-08-PLAN.md — Runtime hardening + startup verification gap closure

### Phase 2: Single Session Core
**Goal**: User can launch one named Claude Code session and view its live streaming output
**Depends on**: Phase 1
**Requirements**: SESS-01 (partial - single session), OUT-01, GIT-01
**Success Criteria** (what must be TRUE):
  1. User can launch a named Claude Code session with custom prompt and working directory
  2. User sees live streaming output (text, thinking, tool use, tool results) in real-time
  3. Session runs to completion without crashing the app
  4. Session displays final status (completed or failed) after finishing
**Plans**: 2 plans

Plans:
- [x] 02-01-PLAN.md — Backend stream-json normalization + idempotent terminal lifecycle persistence
- [x] 02-02-PLAN.md — Frontend launch flow + canonical live stream rendering with single terminal state

### Phase 3: Multi-Session Orchestration
**Goal**: User can run 3-5 parallel Claude Code sessions with unified dashboard view
**Depends on**: Phase 2
**Requirements**: SESS-01, SESS-02, SESS-03, GIT-02, LIFE-03
**Success Criteria** (what must be TRUE):
  1. User can launch multiple named sessions (3-5) that run in parallel without blocking each other
  2. User sees dashboard list showing all sessions with name, status, and recent activity age at a glance
  3. User can click into any session to view its live output stream
  4. Each session runs in isolated git worktree to prevent conflicts
  5. One crashed session does not affect other running sessions
**Plans**: 3 plans

Plans:
- [ ] 03-01-PLAN.md — Backend worktree isolation + locked status projection + startup reconciliation
- [ ] 03-02-PLAN.md — SessionSupervisor parallel runtime isolation + mixed-outcome integration coverage
- [ ] 03-03-PLAN.md — Locked dashboard list behavior, interactions, and regression protection

### Phase 4: Security & Approval System
**Goal**: User controls tool execution through approval prompts with pattern-based auto-approve rules
**Depends on**: Phase 3
**Requirements**: APPR-01, APPR-02, APPR-03, APPR-04
**Success Criteria** (what must be TRUE):
  1. User sees approval prompt when session attempts tool call not covered by auto-approve rules
  2. User can approve or reject pending tool calls from the UI
  3. User can define auto-approve rules using pattern matching (exact, prefix, wildcard)
  4. Tools matching auto-approve rules execute immediately without manual prompt
  5. Session status shows "waiting for approval" when blocked on user decision
**Plans**: TBD

Plans:
- [ ] 04-01-PLAN.md — TBD
- [ ] 04-02-PLAN.md — TBD
- [ ] 04-03-PLAN.md — TBD

### Phase 5: Session Lifecycle Control
**Goal**: User can interrupt running sessions and resume completed sessions with new prompts
**Depends on**: Phase 4
**Requirements**: LIFE-01, LIFE-02, LIFE-04
**Success Criteria** (what must be TRUE):
  1. User can interrupt a running session mid-execution
  2. Interrupted session shows "interrupted" status and preserves its history
  3. User can continue/resume a completed or interrupted session with a new prompt
  4. App handles session errors gracefully with clear error messages
  5. Sessions recover from errors without crashing the application
**Plans**: TBD

Plans:
- [ ] 05-01-PLAN.md — TBD
- [ ] 05-02-PLAN.md — TBD

### Phase 6: Persistence & History
**Goal**: Sessions persist across app restarts and users can review complete session history
**Depends on**: Phase 5
**Requirements**: SESS-04, OUT-02
**Success Criteria** (what must be TRUE):
  1. User closes app and reopens to find all sessions preserved with current status
  2. Session history includes all prompts, outputs, tool calls, and approvals
  3. User can review session logs after completion to see what the agent did
  4. Session state (running, waiting, completed, failed, interrupted) persists correctly across restarts
**Plans**: TBD

Plans:
- [ ] 06-01-PLAN.md — TBD
- [ ] 06-02-PLAN.md — TBD

## Progress

**Execution Order:**
Phases execute in numeric order: 1 → 2 → 3 → 4 → 5 → 6

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Foundation & Architecture | 8/8 | Complete | 2026-02-15 |
| 2. Single Session Core | 2/2 | Complete | 2026-02-15 |
| 3. Multi-Session Orchestration | 0/3 | Not started | - |
| 4. Security & Approval System | 0/3 | Not started | - |
| 5. Session Lifecycle Control | 0/2 | Not started | - |
| 6. Persistence & History | 0/2 | Not started | - |

---
*Last updated: 2026-02-15 after Phase 3 replanning from refreshed research/context*

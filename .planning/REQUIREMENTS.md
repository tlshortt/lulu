# Requirements: Lulu

**Defined:** 2026-02-14
**Core Value:** Run and monitor multiple Claude Code instances simultaneously from a single dashboard

## v1 Requirements

Requirements for initial release. Each maps to roadmap phases.

### Session Management

- [ ] **SESS-01**: User can launch multiple named Claude Code sessions (3-5) that run in parallel
- [ ] **SESS-02**: User can view a dashboard list showing all sessions with name, status, and progress
- [ ] **SESS-03**: User can see session status at a glance (running, waiting for approval, completed, failed, interrupted)
- [ ] **SESS-04**: Sessions persist across app restarts via SQLite storage

### Session Output

- [ ] **OUT-01**: User can click into any session to view its live streamed output (text, thinking, tool use, tool results)
- [ ] **OUT-02**: User can review session history and logs after completion

### Working Directory & Git

- [ ] **GIT-01**: Each session targets a configurable working directory
- [ ] **GIT-02**: Sessions use git worktrees to isolate parallel agents from conflicting

### Security & Approvals

- [ ] **APPR-01**: User sees approval prompts for tool operations not covered by auto-approve rules
- [ ] **APPR-02**: User can approve or reject pending tool calls from the UI
- [ ] **APPR-03**: User can define auto-approve rules using pattern matching (exact, prefix, wildcard)
- [ ] **APPR-04**: Tools matching auto-approve rules execute without manual prompt

### Session Lifecycle

- [ ] **LIFE-01**: User can interrupt a running session mid-execution
- [ ] **LIFE-02**: User can continue/resume a completed or interrupted session with a new prompt
- [ ] **LIFE-03**: One crashed session does not affect other running sessions
- [ ] **LIFE-04**: App handles session errors gracefully without crashing

## v2 Requirements

Deferred to future release. Tracked but not in current roadmap.

### UX Enhancements

- **UX-01**: User can start sessions from templates/presets for common patterns
- **UX-02**: User can see token usage and cost per session
- **UX-03**: User can review visual side-by-side diffs before committing changes
- **UX-04**: User can see agent confidence scoring for task completion likelihood

## Out of Scope

Explicitly excluded. Documented to prevent scope creep.

| Feature | Reason |
|---------|--------|
| Real-time collaboration / multi-user | Single-developer tool |
| Pipeline/chaining sessions | Parallel-only for v1 |
| Native terminal emulator | Sessions run via cc-sdk, not a terminal |
| CLI companion / HTTP API | Desktop app only for v1 |
| Fork sessions | Continue/resume is sufficient for v1 |
| Draft sessions (compose before running) | Direct launch is fine for v1 |
| Memory/learning system | Requires vector DB infrastructure, defer to v2+ |
| Cross-session coordination | Very complex, unclear user demand |
| Session teleport/handoff | Requires cloud backend |
| Slack/chat integration | Expands beyond desktop app scope |

## Traceability

Which phases cover which requirements. Updated during roadmap creation.

| Requirement | Phase | Status |
|-------------|-------|--------|
| — | — | — |

**Coverage:**
- v1 requirements: 16 total
- Mapped to phases: 0
- Unmapped: 16

---
*Requirements defined: 2026-02-14*
*Last updated: 2026-02-14 after initial definition*

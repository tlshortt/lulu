# Phase 6: Persistence & History - Context

**Gathered:** 2026-02-17
**Status:** Ready for planning

<domain>
## Phase Boundary

Persist session state across app restarts and enable history review so users can reopen the app and find preserved sessions with correct statuses, plus complete logs of prompts, outputs, tool calls, and approvals.

This phase clarifies persistence and history review behavior. It does not add new capabilities outside this boundary.

</domain>

<decisions>
## Implementation Decisions

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

</decisions>

<specifics>
## Specific Ideas

No external product references were requested.

</specifics>

<deferred>
## Deferred Ideas

None - discussion stayed within phase scope.

</deferred>

---

*Phase: 06-persistence-history*
*Context gathered: 2026-02-17*

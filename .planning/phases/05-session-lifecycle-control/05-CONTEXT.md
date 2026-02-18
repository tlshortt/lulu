# Phase 5: Session Lifecycle Control - Context

**Gathered:** 2026-02-16
**Status:** Ready for planning

<domain>
## Phase Boundary

Allow users to interrupt running/in-progress sessions and resume completed or interrupted sessions with a new prompt, while preserving session history and isolating errors per session.

</domain>

<decisions>
## Implementation Decisions

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

</decisions>

<specifics>
## Specific Ideas

- Keep interruption feedback lightweight in dashboard rows; avoid layout expansion or context switching.

</specifics>

<deferred>
## Deferred Ideas

None - discussion stayed within phase scope.

</deferred>

---

*Phase: 05-session-lifecycle-control*
*Context gathered: 2026-02-16*

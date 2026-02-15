# Phase 3: Multi-Session Orchestration - Context

**Gathered:** 2026-02-15
**Status:** Ready for planning

<domain>
## Phase Boundary

Deliver multi-session orchestration where users can run 3-5 Claude Code sessions in parallel, monitor all sessions in one dashboard, open any session's live output, and rely on isolated git worktrees so one crashed session does not impact others.

</domain>

<decisions>
## Implementation Decisions

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

</decisions>

<specifics>
## Specific Ideas

- Keep the dashboard readable at a glance with stable ordering and minimal motion.
- Favor compact, relative activity timestamps over verbose time text.

</specifics>

<deferred>
## Deferred Ideas

None - discussion stayed within phase scope.

</deferred>

---

*Phase: 03-multi-session-orchestration*
*Context gathered: 2026-02-15*

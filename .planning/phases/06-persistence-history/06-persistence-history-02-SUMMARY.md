---
phase: 06-persistence-history
plan: 02
subsystem: ui
tags: [svelte, tauri, dashboard, restore-ux, history-replay]
requires:
  - phase: 06-persistence-history
    provides: backend restore metadata projection and list_session_history timeline API
provides:
  - Restore-aware dashboard ordering with startup active-first override
  - Sidebar restored badge and recovery hint affordances with sort controls
  - Frontend regressions covering restore lifecycle and timeline replay behavior
affects: [phase-transition, restore-dashboard, session-output]
tech-stack:
  added: []
  patterns:
    - Merge list_sessions lifecycle data with list_dashboard_sessions projection metadata
    - Apply startup sort lock separately from persisted user sort preference
    - Hydrate SessionOutput timeline from list_session_history payload variants
key-files:
  created:
    - .planning/phases/06-persistence-history/06-persistence-history-02-SUMMARY.md
  modified:
    - src/lib/types/session.ts
    - src/lib/stores/sessions.ts
    - src/lib/components/Sidebar.svelte
    - src/lib/__tests__/sessions.dashboard.test.ts
    - src/lib/components/Sidebar.test.ts
    - src/lib/components/SessionOutput.test.ts
key-decisions:
  - "Use list_dashboard_sessions for restore metadata while retaining list_sessions for lifecycle-operational status fields."
  - "Lock startup ordering to active-first-then-recent and persist user-selected sort mode for post-startup interaction."
  - "Map list_session_history payload variants into typed frontend SessionEvent entries for full timeline replay."
patterns-established:
  - "Startup Sort Lock: dashboard always opens with active-first ordering before user reselects sort mode."
  - "Restore Affordance Lifecycle: restored badge/hint clear on the first new routed event for a session."
duration: 7 min
completed: 2026-02-18
---

# Phase 6 Plan 2: Persistence & History Summary

**Frontend restore UX now applies locked startup ordering, surfaces restored/recovery row affordances, and replays full persisted session timeline events after restart.**

## Performance

- **Duration:** 7 min
- **Started:** 2026-02-18T03:26:52Z
- **Completed:** 2026-02-18T03:33:59Z
- **Tasks:** 3
- **Files modified:** 6

## Accomplishments
- Added restore-aware dashboard sort model with startup active-first behavior, remembered sort persistence, and full `list_session_history` timeline hydration.
- Updated sidebar rows to show restored badge/recovery hint affordances and wired sort controls to store-managed sort state.
- Added deterministic frontend regressions for startup ordering, restored badge clearing, sort behavior, and non-message timeline replay visibility.

## Task Commits

Each task was committed atomically:

1. **Task 1: Add restore-aware dashboard sort and full history hydration in session store** - `1bb82b9` (feat)
2. **Task 2: Implement restored-row badge, recovery hint, and row emphasis updates in Sidebar** - `3b264ab` (feat)
3. **Task 3: Add frontend regressions for restore UX, startup ordering, and badge lifecycle** - `f7784ab` (test)

**Plan metadata:** Pending (created after STATE update commit)

## Files Created/Modified
- `src/lib/types/session.ts` - Added dashboard sort mode and restore-aware row metadata typing.
- `src/lib/stores/sessions.ts` - Added startup sort lock, remembered sort preference handling, restore metadata lifecycle clearing, and `list_session_history` event hydration.
- `src/lib/components/Sidebar.svelte` - Added sort controls, restored badge, recovery hint, and row emphasis updates.
- `src/lib/__tests__/sessions.dashboard.test.ts` - Added regressions for startup ordering, sort persistence behavior, and restored badge clear lifecycle.
- `src/lib/components/Sidebar.test.ts` - Added UI regressions for restored affordances and sort control wiring.
- `src/lib/components/SessionOutput.test.ts` - Added replay assertion for non-message timeline events.

## Decisions Made
- Kept `list_sessions` as lifecycle source-of-truth and merged `list_dashboard_sessions` metadata so lifecycle actions retain raw operational statuses.
- Stored sort preference under a global dashboard key while keeping startup ordering locked to Phase 6 defaults until user interaction.
- Normalized persisted history payload variants (`message`, `thinking`, `tool_call`, `tool_result`, `status`, `error`) directly into typed frontend timeline events.

## Deviations from Plan

None - plan executed exactly as written.

## Authentication Gates
None.

## Issues Encountered
None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- Phase 6 restore UX contract is fully implemented and test-covered across store projection, sidebar affordances, and session output replay.
- Phase 6 now has both summaries complete and is ready for phase transition workflow.

## Self-Check: PASSED
- FOUND: `.planning/phases/06-persistence-history/06-persistence-history-02-SUMMARY.md`
- FOUND commit: `1bb82b9`
- FOUND commit: `3b264ab`
- FOUND commit: `f7784ab`

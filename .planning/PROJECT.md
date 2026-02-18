# Lulu

## What This Is

A native desktop application that orchestrates multiple Claude Code sessions in parallel. Instead of toggling between terminal windows, Lulu provides a dashboard view where you launch named sessions ("fix auth bug", "refactor API"), monitor their status at a glance, and click into any session to see its live streaming output. Built with Tauri v2 (Rust) + Svelte 5, wrapping the `cc-sdk` Rust crate.

## Core Value

Run and monitor multiple Claude Code instances simultaneously from a single dashboard — the "mission control" for parallel AI-assisted development.

## Requirements

### Validated

- ✓ Launch multiple named Claude Code sessions that run in parallel — v1.0
- ✓ Dashboard list view showing all sessions with status and activity context — v1.0
- ✓ Session live output stream (text, thinking, tool use, tool result) — v1.0
- ✓ Configurable working directory per session with git worktree isolation — v1.0
- ✓ Interrupt and resume lifecycle controls with error isolation — v1.0
- ✓ Durable persistence and full history replay across restarts — v1.0

### Active

- [ ] Define v1.1 milestone scope and refreshed requirement set
- [ ] Improve launch-time diagnostics and recovery UX for invalid CLI/runtime setup
- [ ] Add quality-of-life workflow features (templates/presets, richer session insights)
- [ ] Re-evaluate approval-system direction (reintroduce, redesign, or keep out of scope)

### Out of Scope

- Real-time collaboration / multi-user — this is a single-developer tool
- Pipeline/chaining sessions — parallel-only for v1
- Native terminal emulator — sessions run via cc-sdk, not a terminal
- CLI companion / HTTP API — desktop app only for v1
- Fork sessions — continue/resume is sufficient for v1
- Draft sessions (compose before running) — direct launch is fine for v1

## Current State

- **Shipped version:** v1.0 MVP (2026-02-18)
- **Delivery scope:** 5 implemented phases, 21 plans, 56 tasks
- **Core capability shipped:** Parallel Claude Code mission-control desktop app with isolation, lifecycle controls, and persistence
- **Archive references:** `.planning/milestones/v1.0-ROADMAP.md`, `.planning/milestones/v1.0-REQUIREMENTS.md`, `.planning/MILESTONES.md`

## Next Milestone Goals

1. Define v1.1 scope from user feedback and day-2 operational needs.
2. Decide whether approval workflows return as a redesign or remain excluded.
3. Prioritize usability gains that improve multi-session throughput and reliability.

## Context

- Inspired by HumanLayer's CodeLayer (Go-based), but built in Rust for performance and because the user prefers Rust
- The `cc-sdk` Rust crate (v0.5) provides the Claude Code integration: streaming messages, session management, resume capability
- Typical workflow: kick off 3-5 sessions on different tasks in the same repo (different branches/worktrees), monitor status, approve dangerous tool calls, step in when needed
- Sessions are task-oriented: named by what they're doing ("fix auth bug", "add dark mode", "refactor API")
- Codebase footprint is ~10,211 tracked lines across TypeScript, Svelte, Rust, Python, and Swift sources.

## Constraints

- **Tech stack**: Tauri v2 (Rust backend) + Svelte 5 (frontend) + cc-sdk for Claude Code integration
- **Package manager**: bun for frontend dependencies
- **Platform**: macOS primary (Darwin), cross-platform later
- **Storage**: SQLite via rusqlite for session persistence
- **IPC**: Tauri's Channel<T> for streaming events from Rust to Svelte

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Svelte 5 over React | Lighter framework, runes reactivity fits Tauri's event model well | ✓ Good (v1.0 shipped on this stack) |
| Rust + cc-sdk over Go | User preference for Rust, cc-sdk provides native Claude Code integration | ✓ Good (native runtime + integration goals met) |
| Auto-approve as default posture | User wants to mostly let Claude run, only intervene for dangerous tools | ⚠ Revisit (approval system removed from v1.0 scope) |
| SQLite for persistence | Simple, embedded, no external dependencies, good enough for local app | ✓ Good (durable session/events history shipped) |
| Dashboard-first UI | Status overview is the primary view, detail on click-through | ✓ Good (core mission-control UX validated) |

---
*Last updated: 2026-02-18 after v1.0 milestone completion*

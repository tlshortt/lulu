# Lulu

## What This Is

A native desktop application that orchestrates multiple Claude Code sessions in parallel. Instead of toggling between terminal windows, Lulu provides a dashboard view where you launch named sessions ("fix auth bug", "refactor API"), monitor their status at a glance, and click into any session to see its live streaming output. Built with Tauri v2 (Rust) + Svelte 5, wrapping the `cc-sdk` Rust crate.

## Core Value

Run and monitor multiple Claude Code instances simultaneously from a single dashboard — the "mission control" for parallel AI-assisted development.

## Requirements

### Validated

(None yet — ship to validate)

### Active

- [ ] Launch multiple named Claude Code sessions that run in parallel
- [ ] Dashboard list view showing all sessions with name, status (running/completed/failed/interrupted), and progress
- [ ] Click into any session to view its live streamed output (text, thinking, tool use, tool results)
- [ ] Each session targets a configurable working directory (same repo, different branches/worktrees)
- [ ] Auto-approve rules for tool calls (pattern-based: exact match, prefix, wildcard)
- [ ] Manual approval prompt only for tools not covered by auto-approve rules
- [ ] Interrupt a running session mid-execution
- [ ] Continue/resume a completed or interrupted session with a new prompt
- [ ] Session status visible at a glance: running, waiting for approval, completed, failed, interrupted
- [ ] Sessions persist across app restarts (SQLite storage)

### Out of Scope

- Real-time collaboration / multi-user — this is a single-developer tool
- Pipeline/chaining sessions — parallel-only for v1
- Native terminal emulator — sessions run via cc-sdk, not a terminal
- CLI companion / HTTP API — desktop app only for v1
- Fork sessions — continue/resume is sufficient for v1
- Draft sessions (compose before running) — direct launch is fine for v1

## Context

- Inspired by HumanLayer's CodeLayer (Go-based), but built in Rust for performance and because the user prefers Rust
- The `cc-sdk` Rust crate (v0.5) provides the Claude Code integration: streaming messages, session management, resume capability
- Typical workflow: kick off 3-5 sessions on different tasks in the same repo (different branches/worktrees), monitor status, approve dangerous tool calls, step in when needed
- Existing preliminary planning docs (slice-1 through slice-5) provide detailed implementation guidance for the Tauri + cc-sdk integration patterns
- Sessions are task-oriented: named by what they're doing ("fix auth bug", "add dark mode", "refactor API")

## Constraints

- **Tech stack**: Tauri v2 (Rust backend) + Svelte 5 (frontend) + cc-sdk for Claude Code integration
- **Package manager**: bun for frontend dependencies
- **Platform**: macOS primary (Darwin), cross-platform later
- **Storage**: SQLite via rusqlite for session persistence
- **IPC**: Tauri's Channel<T> for streaming events from Rust to Svelte

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Svelte 5 over React | Lighter framework, runes reactivity fits Tauri's event model well | — Pending |
| Rust + cc-sdk over Go | User preference for Rust, cc-sdk provides native Claude Code integration | — Pending |
| Auto-approve as default posture | User wants to mostly let Claude run, only intervene for dangerous tools | — Pending |
| SQLite for persistence | Simple, embedded, no external dependencies, good enough for local app | — Pending |
| Dashboard-first UI | Status overview is the primary view, detail on click-through | — Pending |

---
*Last updated: 2026-02-14 after initialization*

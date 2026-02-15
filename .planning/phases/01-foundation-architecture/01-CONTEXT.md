# Phase 1: Foundation & Architecture - Context

**Gathered:** 2026-02-14
**Status:** Ready for planning

<domain>
## Phase Boundary

Establish Tauri + Svelte 5 + Rust infrastructure with cc-sdk integration and SQLite persistence. Deliver a launchable desktop app with core IPC patterns, a single Claude Code subprocess spawn capability, and the project scaffolding that all subsequent phases build on. No user-facing session features yet — this is the skeleton.

</domain>

<decisions>
## Implementation Decisions

### cc-sdk integration method
- Spawn Claude CLI as a child process (subprocess model, not SDK library or direct API)
- Auto-detect Claude CLI location (PATH, ~/.claude/bin, common locations) with user override fallback
- Parse tool calls into structured typed events (tool name, args, result) — not raw text
- Use Claude Code's `--resume` flag for session continuation in later phases
- Minimal flags for Phase 1: just prompt and working directory. No env vars or extra CLI config yet
- Single hardcoded working directory for Phase 1. Per-session directories come in Phase 2/3
- Always kill all child processes when Lulu exits (intentional or crash). Clean slate on restart
- Never auto-kill unresponsive sessions. User-initiated kill only. Show "no activity" indicator

### cc-sdk version handling (Claude's Discretion)
- Claude picks the right CLI output mode (JSON streaming if available, fallback to text parsing)
- Claude decides version compatibility strategy (strict check vs best-effort)
- Claude decides prompt passing method (CLI arg vs stdin vs file)

### App shell & window chrome
- Dark mode only. Developer-focused dark theme
- Native title bar (standard macOS/Windows/Linux)
- Sidebar + main area layout from day one (sidebar for session list, main for content)
- Visual style: Warp terminal-like — developer-focused, terminal aesthetic, code-oriented feel

### Event streaming model
- Message-level chunks, not token-by-token streaming. Buffer complete messages before showing
- Tool calls shown as distinct collapsible blocks with tool name, args, and result
- Thinking/reasoning hidden by default, toggleable to expand
- Simple session status only: running / done / error. No inferred status like "writing code"

### Dev workflow & validation
- Both unit tests (cargo test, vitest) and integration tests (real CLI spawn + IPC verification)
- shadcn/ui + Tailwind CSS for styling and components
- No CI (GitHub Actions) yet — add later when there's enough code
- Full linting and formatting from day one: eslint, prettier, rustfmt, clippy. Enforced

### Claude's Discretion
- CLI output parsing strategy (JSON stream vs text)
- cc-sdk version compatibility approach
- Prompt passing mechanism
- Loading skeleton and placeholder designs
- Exact spacing, typography, and color palette within Warp-like dark aesthetic
- Database schema design and migration approach
- IPC channel implementation details
- Error state handling patterns

</decisions>

<specifics>
## Specific Ideas

- "Warp terminal-like" aesthetic — developer-focused, dark, terminal feel. Not a generic web app in a window
- shadcn/ui components for consistent, polished UI primitives
- Tool call blocks should feel like Claude Code's own rendering — collapsible, structured, not raw text dumps

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 01-foundation-architecture*
*Context gathered: 2026-02-14*

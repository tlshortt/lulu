# Project Research Summary

**Project:** Lulu - Claude Code Orchestrator
**Domain:** Native Desktop AI Agent Orchestrator
**Researched:** 2026-02-14
**Confidence:** HIGH

## Executive Summary

Lulu is a native desktop application for orchestrating multiple parallel Claude Code sessions, enabling developers to run 3-5 concurrent AI agents on separate branches using git worktrees. Based on comprehensive research across stack, features, architecture, and pitfalls, the recommended approach is a **Tauri v2 + Rust backend with Svelte 5 frontend**, leveraging the cc-sdk (Claude Code SDK) for session management and SQLite for persistence.

The product fills a clear niche between single-session tools (Cursor, Windsurf) and complex cloud platforms (Devin). Users expect parallel sessions, real-time status monitoring, git worktree integration, and approval workflows - these are table stakes based on 2026 industry standards. The key differentiators are auto-approve rules, a unified dashboard view, and native performance without Electron's overhead. The architecture follows proven patterns: multi-session orchestration via Tokio tasks, event streaming through Tauri Channels, and rules-based approval interception.

Critical risks center on concurrency management: SQLite write serialization bottlenecks (must use BEGIN IMMEDIATE transactions), unbounded channel memory exhaustion (requires bounded channels with backpressure), and process lifecycle mismanagement (zombie accumulation, SIGSTOP corruption). These pitfalls must be addressed in Phase 1 architecture - retrofitting is expensive and error-prone. Success depends on building the orchestration layer correctly from the start, with session isolation, graceful error handling, and defensive concurrency primitives.

## Key Findings

### Recommended Stack

The research strongly recommends **Tauri v2 (2.10.x) + Svelte 5 + Rust 1.77.2+** as the core stack. Tauri v2 provides native system integration with small binary size (~600KB), security-first IPC isolation, and stable APIs released October 2024. Svelte 5's revolutionary runes reactivity system delivers 15-30% smaller bundles than Svelte 4 and is perfect for real-time streaming UIs. The Rust backend leverages tokio for async runtime, cc-sdk (v0.1.10+) for Claude Code integration, and tauri-plugin-sql for SQLite persistence.

**Core technologies:**
- **Tauri v2 (2.10.x):** Desktop framework with Rust backend - industry standard for secure, performant desktop apps with IPC isolation and native system integration
- **Svelte 5 (5.x):** Frontend framework with runes reactivity - 15-30% smaller bundles, native TypeScript, perfect for real-time streaming output
- **cc-sdk (0.1.10+):** Claude Code CLI integration - provides async clients, streaming, interrupt control, and token optimization built on tokio
- **SQLite 3.x via tauri-plugin-sql:** Embedded database for session persistence - no separate server, perfect for desktop apps with sqlx type-checking
- **Rust 1.77.2+:** Backend language required by Tauri - memory-safe, fast, excellent async support via tokio ecosystem
- **Vite 7.x + TypeScript 5.x:** Build tooling - 5x faster full builds, official Tauri recommendation, native TS support

**Critical version compatibility:**
- Tauri 2.10.x requires Rust 1.77.2+ and Node 20.19+ (for Vite 7 compatibility)
- Svelte 5 requires SvelteKit versions released after October 2024
- cc-sdk 0.1.10 built on tokio 1.x, requires async/await throughout
- Bun 1.x supported in Tauri 1.5+, but verify bun.lock compatibility

**Alternatives considered:**
- Electron (if need Chrome DevTools and maximum ecosystem compatibility, trade-off: 100MB+ bundles)
- React 19 (if team has deep React expertise, trade-off: larger bundles, no native reactivity)
- pnpm/npm over Bun (if Bun compatibility issues arise in CI/CD)

### Expected Features

Research identified clear table stakes versus differentiators based on analysis of CodeLayer, Devin, Windsurf, Cursor, and industry patterns in 2026.

**Must have (table stakes):**
- **Multiple parallel sessions (3-5)** - core value prop, every orchestrator supports this
- **Session status monitoring** - essential "at a glance" awareness (working, idle, blocked, errored)
- **Git worktree integration** - industry standard in 2026 to prevent agents from conflicting
- **Session naming/labeling** - basic UX requirement to distinguish "auth refactor" from "add tests"
- **Streaming output** - users need real-time visibility into agent progress
- **Approval prompts for dangerous operations** - critical security feature before file deletion, git push, destructive commands
- **Session pause/resume** - users need ability to interrupt agents and halt bad directions
- **Session history/logs** - review what agent did, debug issues, learn from work
- **Basic error handling** - graceful failure when agent hits errors, not crash entire app
- **Cross-session isolation** - one crashed session doesn't kill others

**Should have (competitive advantage):**
- **Auto-approve rules** - skip approval for safe, repetitive actions (NOT table stakes yet, big UX win)
- **Dashboard list view** - see all sessions, status, branches at a glance (Lulu's core differentiator)
- **Visual diff review** - side-by-side diff UI before committing (huge UX improvement over terminal)
- **Session templates/presets** - quick-start common patterns: "Add tests", "Fix linter", "Implement ticket"
- **Session cost tracking** - token usage and dollar cost per session (important for teams managing budgets)
- **Confidence scoring** - agent indicates likelihood of task completion (helps users prioritize review time)

**Defer (v2+):**
- **Memory/learning system** - agents remember coding style, patterns, APIs across sessions (requires vector DB infrastructure)
- **Cross-session coordination** - lead agent spawns sub-agents, merges results (very complex, unclear if users want this)
- **Session teleport/handoff** - start on mobile, continue in desktop app (requires cloud backend)
- **Slack/chat integration** - route tasks from team chat to PR (expands beyond desktop app use case)

**Critical feature dependencies:**
- Git worktrees must exist before parallel sessions (can't safely run parallel agents without isolation)
- Approval system must work before auto-approve rules (need baseline approval flow first)
- Session lifecycle (pause/resume/history) must work before advanced features like visual diff
- Core stability required before attempting cross-session coordination (most complex feature)

### Architecture Approach

The architecture follows a **multi-session orchestration pattern** with event-driven streaming. The Rust backend manages concurrent cc-sdk sessions as independent Tokio tasks, each with its own ClaudeSDKClient instance (never shared or pooled). Events stream through Tauri's Channel API for ordered, type-safe IPC to the Svelte 5 frontend. Global state (SessionManager, Database, RulesEngine) lives in Arc<Mutex<AppState>>, with careful async Mutex usage across await points.

**Major components:**
1. **SessionManager (Rust)** - spawns, tracks, and controls multiple cc-sdk sessions; manages concurrent Tokio tasks with HashMap<SessionId, SessionHandle> wrapped in Arc<Mutex>
2. **EventBus (Rust)** - routes streaming events from cc-sdk sessions to frontend via Tauri Channels; MPSC channel multiplexer with one Channel<SessionEvent> per session
3. **RulesEngine (Rust)** - evaluates approval conditions against tool invocations; auto-approves when rules match using pattern matching on tool names/args
4. **Database Layer (Rust)** - persists session state, approval rules, event history using tokio-rusqlite with Connection handle shared via State
5. **Session Dashboard (Svelte)** - displays real-time status of all sessions; subscribes to state derived from Tauri event streams using runes-based reactive stores
6. **Approval UI (Svelte)** - CRUD interface for managing auto-approval rules; form-based UI calling Tauri commands to persist rules

**Key architectural patterns:**
- **Pattern 1: Multi-Session with Tauri Channels** - each cc-sdk session runs as independent Tokio task, events stream through Tauri's Channel API for ordered, high-throughput IPC
- **Pattern 2: Runes-Based Global State** - Svelte 5 runes enable reactive state management with closure-wrapped $state() in .svelte.ts files for cross-component reactivity
- **Pattern 3: Thread-Safe State with Arc<Mutex<T>>** - mutable state shared across commands and background tasks wrapped in tokio::sync::Mutex for async-safe interior mutability
- **Pattern 4: Mediator Event Bus** - central event bus coordinates between session runners and frontend, decouples session logic from IPC concerns
- **Pattern 5: Conditional Rules Engine** - pattern-matching based rules engine evaluates tool invocations, declarative rules specify conditions (tool name, file patterns, working directory)

**Critical data flows:**
- Session events: cc-sdk stream → SessionRunner → EventBus → Tauri Channel → Svelte store → UI components
- Commands: UI action → invoke() → Command handler → AppState mutation → Response → Update Svelte store
- Approval decisions: Tool invocation (Rust) → Approval request event (Frontend) → User decision (Frontend) → Approval response command (Rust)

### Critical Pitfalls

Research identified 10 critical pitfalls specific to this stack, with clear prevention strategies and phase assignments.

1. **SQLite Write Serialization Bottleneck** - multiple concurrent sessions attempting to persist state cause SQLITE_BUSY errors. Prevention: use BEGIN IMMEDIATE transactions (not DEFERRED), implement write queue with single writer thread, set busy_timeout to 5000ms minimum. Phase 1 critical.

2. **Unbounded Channel Memory Exhaustion** - fast-producing sessions (verbose AI output) overwhelm slow consumers (UI rendering), causing unbounded memory growth until OOM. Prevention: NEVER use unbounded channels in production, use bounded channels with capacity limits (1000 messages/session), implement backpressure. Phase 1 critical.

3. **IPC Serialization Blocking Async Runtime** - large JSON payloads serialize synchronously on Tokio worker threads, blocking the async runtime. Prevention: offload large serialization to Tokio's blocking threadpool (spawn_blocking), chunk large payloads (<1MB per message), use incremental streaming. Phase 1 critical.

4. **Approval Workflow Deadlock** - approval request sent to frontend while async task holds lock, awaiting approval creates deadlock if user never approves or response requires same lock. Prevention: NEVER hold locks across approval await points, use message-passing for approval, implement 5-minute timeout with default deny. Phase 2 critical.

5. **cc-sdk Process Zombie Accumulation** - cc-sdk subprocess crashes but parent never reaps child, zombies accumulate consuming process table entries until OS refuses new processes. Prevention: register SIGCHLD handlers to reap immediately, use tokio::process::Command with automatic reaping, RAII guards ensure cleanup. Phase 1 critical.

6. **SIGSTOP/SIGCONT Process Interrupt Brittleness** - pausing cc-sdk session with SIGSTOP leaves process mid-operation, resuming causes inconsistent state crashes or corruption. Prevention: AVOID SIGSTOP/SIGCONT, use graceful cooperative suspension with "pause" message protocol and handshake acknowledgment. Phase 2 critical.

7. **Frontend State Synchronization Race Condition** - multiple sessions emit updates concurrently, Svelte frontend receives out of order, displays stale data or inconsistencies. Prevention: include monotonic version/sequence numbers with every state update, frontend discards updates with version < current_version. Phase 3 important.

8. **cc-sdk v0.5 API Surface Instability** - cc-sdk is pre-1.0 (currently v0.5), minor version updates introduce breaking changes that break orchestration logic. Prevention: pin exact cc-sdk version (no ^ or ~ ranges), abstract cc-sdk behind internal facade/adapter layer. Phase 1 critical.

9. **Stateful ClaudeSDKClient Cross-Contamination** - ClaudeSDKClient instances for different sessions accidentally share state, causing responses from Session A to appear in Session B. Prevention: one ClaudeSDKClient per session - never share or pool, wrap clients in session-scoped structs. Phase 1 critical.

10. **Tauri Window/WebView Process Lifecycle Mismatch** - user closes window expecting app to quit but background sessions continue, or sessions killed losing in-progress work. Prevention: hook window close events to trigger graceful session shutdown, implement "minimize to tray", show confirmation if active sessions exist. Phase 3 important.

## Implications for Roadmap

Based on research, the natural phase structure emerges from dependency analysis, architectural patterns, and pitfall prevention strategies. The roadmap should prioritize foundational architecture (Phase 1) to avoid expensive retrofitting, then build session lifecycle features (Phase 2), followed by UX polish (Phase 3).

### Phase 1: Foundation & Single Session
**Rationale:** Must establish core architecture patterns before multi-session complexity. Database layer, process lifecycle, and IPC primitives must be correct from the start - retrofitting after the fact is prohibitively expensive. This phase addresses 6 of 10 critical pitfalls.

**Delivers:** Working single cc-sdk session with Tauri IPC, SQLite persistence, and proper error handling. Proves core integration between Tauri, cc-sdk, and database layer.

**Addresses (from FEATURES.md):**
- Streaming output (table stakes) - establish Tauri Channel pattern
- Session naming/labeling (table stakes) - basic metadata storage
- Basic error handling (table stakes) - graceful degradation patterns
- Session history/logs (table stakes) - append-only log storage

**Uses (from STACK.md):**
- Tauri v2 + Svelte 5 + Rust project scaffolding
- cc-sdk integration for single session
- tauri-plugin-sql with BEGIN IMMEDIATE transactions
- Tokio async runtime with proper task spawning

**Implements (from ARCHITECTURE.md):**
- Database layer with proper schema and write serialization
- AppState with Arc<Mutex> for thread-safe shared state
- SessionRunner with single cc-sdk session in Tokio task
- Tauri Channel integration for streaming events
- Svelte stores with runes-based reactive state

**Avoids (from PITFALLS.md):**
- SQLite write serialization bottleneck (Pitfall 1) - BEGIN IMMEDIATE from start
- Unbounded channel memory exhaustion (Pitfall 2) - bounded channels with backpressure
- IPC serialization blocking (Pitfall 3) - offload to blocking pool, chunk payloads
- Process zombie accumulation (Pitfall 5) - SIGCHLD handlers and tokio::process::Command
- cc-sdk API instability (Pitfall 8) - pin exact version, abstraction layer
- ClaudeSDKClient cross-contamination (Pitfall 9) - per-session client instances

**Complexity:** HIGH (6-8 weeks) - foundational architecture with many critical decisions

### Phase 2: Multi-Session Orchestration
**Rationale:** With core architecture proven, extend to multiple parallel sessions. This is the product's core value proposition. Git worktree integration is essential before going parallel to prevent catastrophic merge conflicts.

**Delivers:** 3-5 parallel cc-sdk sessions running on separate git worktrees, with unified dashboard view and status monitoring.

**Addresses (from FEATURES.md):**
- Multiple parallel sessions (table stakes, CRITICAL)
- Git worktree integration (table stakes, CRITICAL)
- Session status monitoring (table stakes)
- Cross-session isolation (table stakes)
- Dashboard list view (differentiator, core Lulu value prop)

**Uses (from STACK.md):**
- Git worktree API for branch isolation
- tokio::spawn for concurrent session tasks
- MPSC channels for event multiplexing

**Implements (from ARCHITECTURE.md):**
- SessionManager with HashMap<SessionId, SessionHandle>
- EventBus for routing events to correct frontend channels
- Multi-session UI dashboard with real-time status

**Avoids (from PITFALLS.md):**
- Frontend state synchronization race condition (Pitfall 7) - version/sequence numbers

**Complexity:** MEDIUM (4-5 weeks) - extends proven patterns to multiple instances

### Phase 3: Approval System
**Rationale:** Approval workflows are table stakes for security but depend on working session infrastructure to intercept tool invocations. Auto-approve rules provide competitive differentiation but must wait until baseline approval system is proven stable.

**Delivers:** User approval prompts for dangerous operations, with persistent approval rules engine for auto-approving safe patterns.

**Addresses (from FEATURES.md):**
- Approval prompts for dangerous ops (table stakes, CRITICAL)
- Auto-approve rules (differentiator, big UX win)

**Uses (from STACK.md):**
- Pattern matching in Rust for rule evaluation
- SQLite for rule persistence and evaluation history
- Zod for frontend rule schema validation

**Implements (from ARCHITECTURE.md):**
- RulesEngine with conditional pattern matching
- SessionRunner + RulesEngine integration to intercept tool invocations
- Approval UI with CRUD interface for rule management

**Avoids (from PITFALLS.md):**
- Approval workflow deadlock (Pitfall 4) - never hold locks across approval await points, 5-minute timeout with default deny

**Complexity:** HIGH (4-6 weeks) - complex state machine, security implications

### Phase 4: Lifecycle & Control
**Rationale:** With multi-session and approval working, add user control over session lifecycle. Pause/resume is table stakes based on industry research, but requires careful implementation to avoid process corruption.

**Delivers:** Session pause/resume with state persistence, graceful error recovery, window lifecycle management.

**Addresses (from FEATURES.md):**
- Session pause/resume (table stakes)

**Uses (from STACK.md):**
- Cooperative suspension protocol (not SIGSTOP/SIGCONT)
- State serialization for pause/resume persistence

**Implements (from ARCHITECTURE.md):**
- Session state serialization and restoration
- Process lifecycle hooks for graceful shutdown

**Avoids (from PITFALLS.md):**
- SIGSTOP/SIGCONT corruption (Pitfall 6) - cooperative pause protocol with handshakes
- Window lifecycle mismatch (Pitfall 10) - hook close events, minimize to tray, confirmation dialogs

**Complexity:** MEDIUM (3-4 weeks) - tricky state management but clear patterns

### Phase 5: UX Polish & Differentiators
**Rationale:** Core functionality is working and stable. Now add features that differentiate from competitors and improve daily usability.

**Delivers:** Visual diff review, session templates/presets, session cost tracking.

**Addresses (from FEATURES.md):**
- Visual diff review (differentiator) - major UX improvement over terminal diffs
- Session templates/presets (differentiator) - accessibility for common patterns
- Session cost tracking (differentiator) - important for teams managing budgets

**Uses (from STACK.md):**
- Diff rendering libraries
- Syntax highlighting
- API metering for cost calculation

**Implements (from ARCHITECTURE.md):**
- Diff rendering component in Svelte
- Template storage and variable substitution
- Metering layer for token/cost tracking

**Avoids (from PITFALLS.md):**
- No specific pitfalls, but must maintain performance with rich UI components

**Complexity:** MEDIUM (4-5 weeks) - mostly UI work with some API integration

### Phase Ordering Rationale

**Dependency-driven order:**
- Phase 1 must come first - establishes architectural foundations that are expensive to change later
- Phase 2 depends on Phase 1 - multi-session extends single-session patterns
- Phase 3 depends on Phase 2 - approval system needs working sessions to intercept tool invocations
- Phase 4 depends on Phase 3 - pause/resume interacts with approval workflow
- Phase 5 can proceed once Phases 1-4 are stable - polish doesn't affect core architecture

**Risk mitigation order:**
- Address 6 critical pitfalls in Phase 1 before building on top (SQLite concurrency, channels, IPC, processes, API stability, client isolation)
- Phase 2 adds multi-session complexity only after proving single-session works
- Phase 3 tackles approval workflow deadlock risk once session infrastructure is solid
- Phase 4 addresses SIGSTOP corruption and window lifecycle after approval system exists

**Value delivery order:**
- Phase 1 delivers working single session (minimal viable demo)
- Phase 2 delivers core value prop (parallel orchestration) - first usable product
- Phase 3 delivers table stakes security (approval prompts) - first shippable product
- Phase 4 delivers table stakes control (pause/resume) - production-ready
- Phase 5 delivers competitive differentiators (visual diff, templates, cost tracking) - market-ready

### Research Flags

**Phases likely needing deeper research during planning:**
- **Phase 1:** cc-sdk integration patterns need experimentation - API surface is still stabilizing (v0.5), need to validate streaming output capture, error handling, and process lifecycle integration. Budget 1-2 days for spike.
- **Phase 2:** Git worktree cleanup patterns need validation - worktree lifecycle (creation, switching, cleanup) with concurrent sessions requires careful testing. Budget 1 day for spike.
- **Phase 3:** Rules engine pattern matching complexity - need to validate Rust pattern matching capabilities for file globs, regex, and complex conditions. Budget 1 day for design.

**Phases with standard patterns (skip research-phase):**
- **Phase 4:** Session pause/resume follows well-documented cooperative suspension patterns from game engines and audio processing literature
- **Phase 5:** Visual diff rendering has established libraries (similar to Git GUIs, code review tools) - standard implementation

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | Based on official Tauri v2, Svelte 5, cc-sdk documentation. All technologies are stable (Tauri 2.0 released Oct 2024, Svelte 5 released Oct 2024, cc-sdk v0.1.10 available). Version compatibility verified across official docs. |
| Features | HIGH | Strong evidence from direct analysis of CodeLayer, Cursor, Windsurf, Claude Code CLI, Devin docs and demos. Clear consensus on table stakes. Differentiators backed by recent releases (Devin 2.1 confidence scoring, Windsurf memory system). |
| Architecture | HIGH | Patterns validated across Tauri official docs, Svelte 5 state management guides, Rust async/concurrency best practices. Multi-session orchestration follows proven event-driven architecture patterns from AWS and Azure documentation. |
| Pitfalls | MEDIUM | SQLite concurrency, async Rust blocking, and channel backpressure pitfalls well-documented with multiple high-quality sources. cc-sdk specific pitfalls extrapolated from crate documentation and subprocess management patterns. Some scenarios require validation during implementation. |

**Overall confidence:** HIGH

Research is comprehensive across all four dimensions (stack, features, architecture, pitfalls) with official documentation, industry analysis, and architectural best practices. The stack is mature and well-documented. Feature requirements are clear from competitive analysis. Architecture patterns are proven. Pitfalls are specific and preventable.

### Gaps to Address

**During Phase 1 planning:**
- **cc-sdk streaming output parsing:** Documentation shows examples, but need to validate exact event format and error handling behavior. Mitigation: budget 1-2 day spike for integration experimentation.
- **SQLite write queue implementation:** Know we need write serialization, but exact implementation pattern (dedicated thread vs. task-local) needs design decision. Mitigation: reference Turso source code for patterns.

**During Phase 2 planning:**
- **Git worktree cleanup timing:** Unclear when to delete worktrees (immediately after session end, on app exit, manual cleanup). Need to research git worktree lifecycle best practices. Mitigation: test with multiple worktree creation/deletion cycles.

**During Phase 3 planning:**
- **Rule precedence and conflict resolution:** If multiple rules match the same tool invocation, which wins? Need clear precedence rules. Mitigation: design explicit rule ordering and priority system.

**During implementation (all phases):**
- **cc-sdk API changes:** Pre-1.0 status means API may change. Continuous monitoring of cc-sdk GitHub releases required. Mitigation: abstraction layer isolates breaking changes, pin exact version, comprehensive integration tests.

## Sources

### Primary (HIGH confidence)

**Stack & Technology:**
- Tauri v2 Official Docs (https://v2.tauri.app/) - Architecture, IPC, plugins, state management, process model
- Svelte 5 Docs (https://svelte.dev/docs/svelte/v5-migration-guide) - Runes, reactivity system, migration guide
- cc-sdk on crates.io (https://crates.io/crates/cc-sdk) - Package information, version history
- SQLite WAL Mode Documentation (https://sqlite.org/wal.html) - Write-ahead logging, concurrency model
- Tokio Documentation (https://tokio.rs/tokio/tutorial/) - Spawning, channels, async runtime patterns

**Features & Competitive Analysis:**
- Claude Code Overview (https://code.claude.com/docs/en/overview) - Official capabilities
- HumanLayer/CodeLayer Docs (https://www.humanlayer.dev/, GitHub) - MULTICLAUDC orchestration features
- Devin Release Notes (https://docs.devin.ai/release-notes/overview) - Devin 2.1 features, confidence scoring
- Windsurf Editor (https://windsurf.com/) - Cascade agent, Memory system
- Cursor IDE (https://cursor.sh/) - Composer mode, Agent mode, inline diffs

**Architecture & Patterns:**
- Tauri IPC Documentation (https://v2.tauri.app/concept/inter-process-communication/) - Channels, commands, events
- Svelte 5 State Management Guide (https://svelte.dev/docs/kit/state-management) - Official patterns
- AWS Event-Driven Architecture (https://aws.amazon.com/event-driven-architecture/) - Architectural patterns
- Azure AI Agent Design Patterns (https://learn.microsoft.com/en-us/azure/architecture/ai-ml/guide/ai-agent-design-patterns) - Orchestration patterns

### Secondary (MEDIUM confidence)

**Pitfalls & Best Practices:**
- Turso SQLite Concurrency Articles - MVCC patterns, single-writer bottleneck solutions
- Rust Async Blocking Articles (OneUpTime, medium.com) - Task poll times, blocking prevention
- Rust Channel Backpressure Guides - Bounded vs. unbounded channels, flow control patterns
- Process Signal Handling in Rust (LogRocket) - SIGCHLD, graceful shutdown, cleanup patterns
- Svelte 5 Global State Patterns (Mainmatter) - Runes-based shared state best practices

**Community Resources:**
- Tauri + Svelte 5 Templates (GitHub) - Community examples and integration patterns
- Git Worktrees for Parallel AI Coding (Medium) - Industry standard practice documentation
- Human-in-the-Loop Best Practices (permit.io) - Approval workflow patterns
- OWASP AI Agent Security Top 10 2026 - Security risks and mitigation strategies

### Tertiary (LOW confidence)

**Market & Tool Comparisons:**
- AI Coding Agents Comparison Articles - Top 7 rankings, feature matrices, market landscape
- AI Dev Tool Power Rankings (LogRocket) - Cursor vs. Windsurf vs. others
- AI Agent Orchestration Platforms (Redis) - Framework comparison and patterns

---
*Research completed: 2026-02-14*
*Ready for roadmap: yes*

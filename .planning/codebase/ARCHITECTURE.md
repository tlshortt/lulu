# Architecture

**Analysis Date:** 2026-02-15

## Pattern Overview

**Overall:** Tauri desktop shell with SvelteKit SPA frontend and Rust backend commands.

**Key Characteristics:**
- SPA UI in `src/routes` with Svelte components in `src/lib/components`.
- Rust backend exposes Tauri commands in `src-tauri/src/commands` and emits window events.
- Local persistence via SQLite in `src-tauri/src/db`.

## Layers

**Frontend UI (SvelteKit):**
- Purpose: Render UI and capture user interactions.
- Location: `src/routes`, `src/lib/components`.
- Contains: Page layouts, modal dialogs, UI widgets.
- Depends on: Svelte stores in `src/lib/stores/sessions.ts` and UI utilities in `src/lib/utils.ts`.
- Used by: SvelteKit runtime entry in `src/routes/+page.svelte`.

**State & API Bridge (Svelte stores):**
- Purpose: Encapsulate UI state and Tauri invoke/event wiring.
- Location: `src/lib/stores/sessions.ts`.
- Contains: `invoke` calls, event listeners, writable stores.
- Depends on: `@tauri-apps/api` and Tauri command names.
- Used by: Components like `src/lib/components/MainArea.svelte` and `src/lib/components/NewSessionModal.svelte`.

**Command Layer (Tauri commands):**
- Purpose: Expose backend operations to the frontend via `invoke`.
- Location: `src-tauri/src/commands/session.rs` with re-exports in `src-tauri/src/commands/mod.rs`.
- Contains: `spawn_session`, `list_sessions`, `get_session`, `kill_session`.
- Depends on: Database layer in `src-tauri/src/db` and session runtime in `src-tauri/src/session`.
- Used by: Tauri builder in `src-tauri/src/lib.rs` and frontend invocations in `src/lib/stores/sessions.ts`.

**Session Runtime (process management):**
- Purpose: Spawn and manage Claude CLI child processes.
- Location: `src-tauri/src/session/cli.rs`, `src-tauri/src/session/manager.rs`.
- Contains: `ClaudeCli` process spawning, `SessionManager` registry of running sessions.
- Depends on: Tokio process primitives, OS PATH lookup.
- Used by: `src-tauri/src/commands/session.rs`.

**Persistence (SQLite):**
- Purpose: Store session metadata.
- Location: `src-tauri/src/db/mod.rs`, `src-tauri/src/db/session.rs`.
- Contains: `Database`, `Session` model, CRUD functions.
- Depends on: `rusqlite` and file path from app data directory.
- Used by: `src-tauri/src/commands/session.rs` and initialization in `src-tauri/src/lib.rs`.

## Data Flow

**Spawn Session Flow:**

1. UI submits form in `src/lib/components/NewSessionModal.svelte`.
2. Store calls `spawnSession` in `src/lib/stores/sessions.ts`, which invokes `spawn_session`.
3. Tauri command `spawn_session` in `src-tauri/src/commands/session.rs` creates a `Session` row via `src-tauri/src/db/session.rs`.
4. `ClaudeCli::spawn_with_output` in `src-tauri/src/session/cli.rs` spawns the CLI process and streams output via Tauri events.
5. Frontend listens to `session-output` in `src/lib/stores/sessions.ts` and updates `sessionOutputs` store.
6. UI renders output in `src/lib/components/SessionOutput.svelte`.

**State Management:**
- Use Svelte writable stores in `src/lib/stores/sessions.ts` as the single source of UI state. Components read `$sessions`, `$sessionOutputs`, and `$selectedSessionId` directly.

## Key Abstractions

**SessionManager:**
- Purpose: Track and kill running CLI processes.
- Examples: `src-tauri/src/session/manager.rs`.
- Pattern: In-memory registry with `Arc<Mutex<HashMap<...>>>`.

**ClaudeCli:**
- Purpose: Locate and spawn the Claude CLI with streaming output.
- Examples: `src-tauri/src/session/cli.rs`.
- Pattern: OS path discovery + async output fan-out.

**Database:**
- Purpose: SQLite connection lifecycle and migrations.
- Examples: `src-tauri/src/db/mod.rs`, `src-tauri/src/db/session.rs`.
- Pattern: Single shared `Mutex<Connection>` with explicit transactions.

## Entry Points

**Tauri main:**
- Location: `src-tauri/src/main.rs`.
- Triggers: OS launches the desktop app.
- Responsibilities: Delegates to `tauri_app_lib::run()` in `src-tauri/src/lib.rs`.

**Tauri app builder:**
- Location: `src-tauri/src/lib.rs`.
- Triggers: Called from `main`.
- Responsibilities: Initialize DB, manage `SessionManager`, register commands, handle window close cleanup.

**SvelteKit page:**
- Location: `src/routes/+page.svelte`.
- Triggers: SPA entry route.
- Responsibilities: Bootstraps listeners via `initSessionListeners` and loads sessions.

## Error Handling

**Strategy:** Prefer `Result<_, String>` from Tauri commands and map errors to user-visible states.

**Patterns:**
- Rust errors mapped via `map_err(|e| format!(...))` in `src-tauri/src/commands/session.rs`.
- Frontend catches errors in `src/lib/components/NewSessionModal.svelte` and renders `error` state.

## Cross-Cutting Concerns

**Logging:** Event-based output from Claude CLI streamed through Tauri events in `src-tauri/src/session/cli.rs` and consumed in `src/lib/stores/sessions.ts`.
**Validation:** Form validation is client-side only in `src/lib/components/NewSessionModal.svelte`.
**Authentication:** Not implemented.

---

*Architecture analysis: 2026-02-15*

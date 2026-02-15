# Coding Conventions

**Analysis Date:** 2026-02-15

## Naming Patterns

**Files:**
- Svelte routes use `+`-prefixed filenames: `src/routes/+page.svelte`, `src/routes/+layout.svelte`, `src/routes/+layout.ts`.
- Svelte components use PascalCase: `src/lib/components/MainArea.svelte`, `src/lib/components/SessionList.svelte`, `src/lib/components/NewSessionModal.svelte`.
- UI primitives use kebab-case under per-component folders: `src/lib/components/ui/button/button.svelte`, `src/lib/components/ui/card/card-title.svelte`, `src/lib/components/ui/scroll-area/scroll-area.svelte`.
- TypeScript modules use lower-case filenames: `src/lib/stores/sessions.ts`, `src/lib/utils.ts`.
- Rust modules use snake_case filenames: `src-tauri/src/session/manager.rs`, `src-tauri/src/commands/session.rs`, `src-tauri/src/db/session.rs`.

**Functions:**
- TypeScript uses camelCase for functions and handlers: `loadSessions`, `spawnSession`, `initSessionListeners` in `src/lib/stores/sessions.ts`; `handleSubmit`, `handleClose` in `src/lib/components/NewSessionModal.svelte`.
- Rust uses snake_case for functions and commands: `spawn_session`, `list_sessions`, `kill_session` in `src-tauri/src/commands/session.rs`.

**Variables:**
- TypeScript uses camelCase for local state and stores: `selectedSessionId`, `sessionOutputs` in `src/lib/stores/sessions.ts`; `newSessionOpen`, `listenersReady` in `src/routes/+page.svelte`.
- Rust uses snake_case for locals and fields: `session_id`, `working_dir` in `src-tauri/src/commands/session.rs` and `src-tauri/src/db/session.rs`.

**Types:**
- TypeScript interfaces and types use PascalCase: `Session`, `SessionOutput` in `src/lib/stores/sessions.ts`; `WithoutChildrenOrChild` in `src/lib/utils.ts`.
- Rust structs use PascalCase: `Session` in `src-tauri/src/db/session.rs`, `SessionManager` in `src-tauri/src/session/manager.rs`.

## Code Style

**Formatting:**
- Svelte component files under `src/lib/components/` use two-space indentation: `src/lib/components/MainArea.svelte`, `src/lib/components/NewSessionModal.svelte`.
- UI component files under `src/lib/components/ui/` use tab indentation: `src/lib/components/ui/button/button.svelte`, `src/lib/components/ui/card/card.svelte`.
- Rust uses 4-space indentation with standard `rustfmt` style: `src-tauri/src/commands/session.rs`, `src-tauri/src/db/session.rs`.

**Linting:**
- No repository-level ESLint/Biome config detected (`.eslintrc*`, `eslint.config.*`, `biome.json` not present in repo root).
- Inline ESLint suppressions exist for specific rules in `src/lib/utils.ts` (e.g., `@typescript-eslint/no-explicit-any`).

## Import Organization

**Order:**
1. External packages first: `src/lib/components/MainArea.test.ts` imports `@testing-library/svelte` and `vitest` before local modules.
2. Internal `$lib` alias imports next: `src/lib/components/MainArea.test.ts` imports `$lib/stores/sessions`.
3. Relative component/module imports last: `src/lib/components/MainArea.test.ts` imports `./MainArea.svelte`.

**Path Aliases:**
- Use SvelteKit `$lib` alias for internal modules/components: `src/lib/components/Sidebar.svelte`, `src/lib/components/MainArea.svelte`, `src/lib/components/NewSessionModal.svelte`.

## Error Handling

**Patterns:**
- UI state errors are handled via local component state and `try/catch`: `src/lib/components/NewSessionModal.svelte` sets an `error` string on validation failure or caught exceptions.
- Store functions return promises and surface errors to callers: `src/lib/stores/sessions.ts` uses `invoke` and returns the result to callers.
- Rust command handlers return `Result<_, String>` and use `map_err` with formatted messages: `src-tauri/src/commands/session.rs`.

## Logging

**Framework:** Not detected (no logging usage in `src/lib/` or `src-tauri/src/`).

**Patterns:** Not detected.

## Comments

**When to Comment:**
- Use short explanatory comments for platform-specific behavior: `src/routes/+layout.ts` and `src-tauri/src/main.rs`.

**JSDoc/TSDoc:** Not detected in `src/lib/` or `src/routes/` files.

## Function Design

**Size:**
- Prefer small single-purpose handlers in Svelte components: `handleBackdropClick`, `handleSubmit` in `src/lib/components/NewSessionModal.svelte`.
- Store actions are single-responsibility async functions: `loadSessions`, `spawnSession`, `initSessionListeners` in `src/lib/stores/sessions.ts`.

**Parameters:**
- Use explicit typed parameters for public functions: `spawnSession(name: string, prompt: string, workingDir: string)` in `src/lib/stores/sessions.ts`.

**Return Values:**
- TypeScript store functions return values from side effects when needed (e.g., `spawnSession` returns `id` in `src/lib/stores/sessions.ts`).
- Rust command handlers return `Result` with typed success values: `Result<String, String>` in `src-tauri/src/commands/session.rs`.

## Module Design

**Exports:**
- Use named exports for stores and functions: `sessions`, `sessionOutputs`, `selectedSessionId` in `src/lib/stores/sessions.ts`.
- Use explicit type re-exports and aliases in UI barrels: `src/lib/components/ui/button/index.ts`, `src/lib/components/ui/card/index.ts`.

**Barrel Files:**
- UI component directories expose `index.ts` to re-export Svelte component modules and aliases: `src/lib/components/ui/button/index.ts`, `src/lib/components/ui/scroll-area/index.ts`.

---

*Convention analysis: 2026-02-15*

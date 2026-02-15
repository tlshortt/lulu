# Codebase Structure

**Analysis Date:** 2026-02-15

## Directory Layout

```
[project-root]/
├── src/                 # SvelteKit frontend (SPA)
│   ├── lib/             # Components, stores, shared utilities
│   └── routes/          # Route entry points (+page/+layout)
├── src-tauri/           # Tauri Rust backend
│   ├── src/             # Rust application code
│   └── tauri.conf.json  # Tauri app configuration
├── static/              # Static assets served by SvelteKit
├── build/               # Frontend build output (generated)
├── .svelte-kit/         # SvelteKit build artifacts (generated)
├── node_modules/        # Dependencies (generated)
├── package.json         # Frontend dependencies/scripts
├── svelte.config.js     # SvelteKit adapter configuration
└── vite.config.js       # Vite build configuration
```

## Directory Purposes

**src/**
- Purpose: Frontend SPA code.
- Contains: Svelte routes and shared UI logic.
- Key files: `src/routes/+page.svelte`, `src/routes/+layout.svelte`, `src/routes/+layout.ts`.

**src/lib/**
- Purpose: Shared frontend modules.
- Contains: Components (`.svelte`), stores (`.ts`), utilities.
- Key files: `src/lib/stores/sessions.ts`, `src/lib/utils.ts`.

**src/lib/components/**
- Purpose: Feature-level UI components.
- Contains: Layout panels and modals.
- Key files: `src/lib/components/MainArea.svelte`, `src/lib/components/Sidebar.svelte`, `src/lib/components/NewSessionModal.svelte`.

**src/lib/components/ui/**
- Purpose: Reusable UI primitives.
- Contains: Component folders with `index.ts` and `.svelte` files.
- Key files: `src/lib/components/ui/button/index.ts`, `src/lib/components/ui/button/button.svelte`.

**src/lib/stores/**
- Purpose: Svelte stores and Tauri bridge logic.
- Contains: Store modules.
- Key files: `src/lib/stores/sessions.ts`.

**src/routes/**
- Purpose: SvelteKit route entry points.
- Contains: `+layout.*` and `+page.svelte`.
- Key files: `src/routes/+page.svelte`.

**src-tauri/src/**
- Purpose: Rust backend implementation.
- Contains: Tauri commands, database, and session runtime.
- Key files: `src-tauri/src/lib.rs`, `src-tauri/src/main.rs`.

**src-tauri/src/commands/**
- Purpose: Tauri command handlers exposed to frontend.
- Contains: Command modules.
- Key files: `src-tauri/src/commands/session.rs`.

**src-tauri/src/db/**
- Purpose: SQLite initialization and CRUD.
- Contains: Database and model modules.
- Key files: `src-tauri/src/db/mod.rs`, `src-tauri/src/db/session.rs`.

**src-tauri/src/session/**
- Purpose: Process management for Claude CLI.
- Contains: `ClaudeCli` and `SessionManager`.
- Key files: `src-tauri/src/session/cli.rs`, `src-tauri/src/session/manager.rs`.

## Key File Locations

**Entry Points:**
- `src/routes/+page.svelte`: Frontend SPA entry.
- `src-tauri/src/main.rs`: Desktop app entry.
- `src-tauri/src/lib.rs`: Tauri app setup and command registration.

**Configuration:**
- `svelte.config.js`: SPA adapter/static fallback.
- `vite.config.js`: Vite build pipeline.
- `src-tauri/tauri.conf.json`: Tauri build/runtime configuration.

**Core Logic:**
- `src/lib/stores/sessions.ts`: Frontend state + Tauri invoke/listen.
- `src-tauri/src/commands/session.rs`: Session command orchestration.
- `src-tauri/src/session/cli.rs`: CLI spawning/streaming.
- `src-tauri/src/db/mod.rs`: SQLite initialization.

**Testing:**
- `src/lib/components/MainArea.test.ts`: Component test example.

## Naming Conventions

**Files:**
- Svelte components: `PascalCase.svelte` (e.g., `src/lib/components/SessionOutput.svelte`).
- SvelteKit routes: `+page.svelte`, `+layout.svelte`, `+layout.ts` in `src/routes/`.
- Rust modules: `mod.rs` with sibling module files (e.g., `src-tauri/src/commands/mod.rs`).

**Directories:**
- UI primitives use nested folders with `index.ts` exports (e.g., `src/lib/components/ui/button/`).
- Backend areas are grouped by responsibility (`commands`, `db`, `session`).

## Where to Add New Code

**New Feature:**
- Primary UI: `src/lib/components/`.
- State/bridge logic: `src/lib/stores/`.
- Backend commands: `src-tauri/src/commands/`.
- Persistence: `src-tauri/src/db/`.

**New Component/Module:**
- Reusable UI primitive: `src/lib/components/ui/<component>/` with `index.ts` and `<component>.svelte`.

**Utilities:**
- Frontend helpers: `src/lib/utils.ts` or new modules in `src/lib/`.

## Special Directories

**build/**
- Purpose: Frontend build output.
- Generated: Yes.
- Committed: Yes (present in repo).

**.svelte-kit/**
- Purpose: SvelteKit build artifacts.
- Generated: Yes.
- Committed: Yes (present in repo).

**node_modules/**
- Purpose: Package dependencies.
- Generated: Yes.
- Committed: No.

**src-tauri/target/**
- Purpose: Rust build artifacts.
- Generated: Yes.
- Committed: Yes (present in repo).

---

*Structure analysis: 2026-02-15*

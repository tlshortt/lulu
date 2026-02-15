# Technology Stack

**Analysis Date:** 2026-02-15

## Languages

**Primary:**
- TypeScript/JavaScript - SvelteKit frontend in `src/**` with config in `package.json` and `tsconfig.json`
- Rust (edition 2021) - Tauri backend in `src-tauri/src/**` with manifest in `src-tauri/Cargo.toml`

**Secondary:**
- JSON - App/build configuration in `src-tauri/tauri.conf.json`

## Runtime

**Environment:**
- Node.js - Vite/SvelteKit dev/build from `package.json` scripts
- Tauri (Rust) - Desktop runtime wired in `src-tauri/src/main.rs` and `src-tauri/src/lib.rs`

**Package Manager:**
- npm - `package.json`
- Lockfile: present (`package-lock.json`)
- Cargo - `src-tauri/Cargo.toml`
- Lockfile: present (`src-tauri/Cargo.lock`)

## Frameworks

**Core:**
- SvelteKit ^2.9.0 - SPA frontend; adapter configured in `svelte.config.js`
- Svelte ^5.0.0 - UI framework used in `src/**`
- Tauri 2 - Desktop shell configured in `src-tauri/tauri.conf.json` and Rust entry in `src-tauri/src/lib.rs`

**Testing:**
- Vitest ^4.0.18 - Unit tests configured in `vite.config.js`
- Testing Library (Svelte, Jest DOM) - `@testing-library/svelte` ^5.3.1, `@testing-library/jest-dom` ^6.9.1 in `package.json`

**Build/Dev:**
- Vite ^6.0.3 - Dev/build pipeline in `vite.config.js`
- Tailwind CSS ^4.1.18 - Styling via `tailwindcss` and `@tailwindcss/vite` in `package.json`
- Svelte adapter-static ^3.0.6 - SPA build fallback in `svelte.config.js`

## Key Dependencies

**Critical:**
- `@tauri-apps/api` ^2 - Frontend-to-backend IPC in `src/lib/stores/sessions.ts`
- `tauri` 2 - Rust app framework in `src-tauri/src/lib.rs`
- `rusqlite` 0.31 (bundled) - Local SQLite storage in `src-tauri/src/db/mod.rs`
- `tokio` 1.x - Async runtime and process management in `src-tauri/src/commands/session.rs` and `src-tauri/src/session/cli.rs`

**Infrastructure:**
- `tauri-plugin-opener` 2 - OS link/file opener via `.plugin(tauri_plugin_opener::init())` in `src-tauri/src/lib.rs`
- `uuid` 1.x - Session IDs in `src-tauri/src/commands/session.rs`
- `chrono` 0.4 - Timestamps in `src-tauri/src/commands/session.rs` and `src-tauri/src/db/session.rs`
- `serde`/`serde_json` 1.x - Serialization across Rust/IPC boundaries in `src-tauri/src/db/mod.rs` and `src-tauri/src/db/session.rs`

## Configuration

**Environment:**
- Frontend dev host uses `TAURI_DEV_HOST` in `vite.config.js`
- Vitest conditions use `VITEST` in `vite.config.js`
- Claude CLI discovery uses `HOME` in `src-tauri/src/session/cli.rs`

**Build:**
- Vite config: `vite.config.js`
- SvelteKit config: `svelte.config.js`
- TypeScript config: `tsconfig.json`
- Tauri app config: `src-tauri/tauri.conf.json`
- Rust manifest: `src-tauri/Cargo.toml`

## Platform Requirements

**Development:**
- Node.js + npm for SvelteKit/Vite (`package.json`)
- Rust toolchain + Cargo for Tauri (`src-tauri/Cargo.toml`)

**Production:**
- Desktop bundles built by Tauri per `src-tauri/tauri.conf.json` targets

---

*Stack analysis: 2026-02-15*

# External Integrations

**Analysis Date:** 2026-02-15

## APIs & External Services

**Desktop/OS Integrations:**
- Tauri IPC and shell integration - Frontend invokes backend commands and listens to events
  - SDK/Client: `@tauri-apps/api` in `src/lib/stores/sessions.ts`
  - Auth: Not applicable
- OS opener integration - Opens external URLs/files via Tauri plugin
  - SDK/Client: `tauri-plugin-opener` in `src-tauri/src/lib.rs`
  - Auth: Not applicable

**Local CLI Integration:**
- Claude CLI (external binary) - Spawns local `claude` process and streams output
  - SDK/Client: `tokio::process::Command` in `src-tauri/src/session/cli.rs`
  - Auth: Not applicable (uses local CLI setup)

## Data Storage

**Databases:**
- SQLite (local file)
  - Connection: File path created at runtime (`app_data_dir/lulu.db`) in `src-tauri/src/lib.rs`
  - Client: `rusqlite` in `src-tauri/src/db/mod.rs`

**File Storage:**
- Local filesystem only (Tauri app data directory) in `src-tauri/src/lib.rs`

**Caching:**
- None detected

## Authentication & Identity

**Auth Provider:**
- Custom/None (no auth provider detected in `src/**` or `src-tauri/**`)

## Monitoring & Observability

**Error Tracking:**
- None detected

**Logs:**
- Tauri emits session events to the frontend using `app.emit(...)` in `src-tauri/src/commands/session.rs`

## CI/CD & Deployment

**Hosting:**
- Desktop app bundled via Tauri targets in `src-tauri/tauri.conf.json`

**CI Pipeline:**
- None detected

## Environment Configuration

**Required env vars:**
- `TAURI_DEV_HOST` (Vite dev host) in `vite.config.js`
- `VITEST` (Vitest condition) in `vite.config.js`
- `HOME` (Claude CLI discovery) in `src-tauri/src/session/cli.rs`

**Secrets location:**
- Not detected

## Webhooks & Callbacks

**Incoming:**
- None detected

**Outgoing:**
- None detected

---

*Integration audit: 2026-02-15*

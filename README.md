# Lulu

Lulu is a native desktop app for running and monitoring multiple Claude Code sessions in parallel. It gives you a single dashboard to launch named sessions, track status, and inspect live output without juggling terminal windows.

## Stack

- Tauri v2 + Rust backend
- SvelteKit (Svelte 5) + TypeScript frontend
- Vite for dev/build tooling
- SQLite (`rusqlite`) for local session persistence

## Getting Started

### Prerequisites

- Node.js 20+
- Bun
- Rust toolchain (Cargo)

### Install

```bash
bun install
```

### Run in development

```bash
bun run tauri dev
```

### Build

```bash
bun run tauri build
```

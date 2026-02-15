# Phase 1: Foundation & Architecture - Research

**Researched:** 2026-02-14
**Domain:** Tauri v2 + Svelte 5 + Rust + SQLite + cc-sdk integration
**Confidence:** HIGH

## Summary

Phase 1 establishes the foundational infrastructure for a multi-session Claude Code orchestrator desktop app. Key findings:

1. **Tauri v2 + Svelte 5**: Use `create-tauri-app` with Svelte template, configure `@sveltejs/adapter-static` for SPA mode, disable SSR
2. **SQLite concurrency**: Use WAL mode + BEGIN IMMEDIATE transactions + busy timeout handler to handle concurrent writes from Tokio tasks
3. **cc-sdk CLI**: Spawn via `tokio::process::Command` with piped stdout/stderr, use async streaming via `tokio_util::codec::FramedRead`
4. **IPC**: Tauri v2 events (`app.emit()`) for Rust→Svelte streaming, `invoke()` for Svelte→Rust commands
5. **UI**: shadcn-svelte + Tailwind CSS v4 with dark mode, Warp-terminal aesthetic

**Primary recommendation:** Scaffold Tauri v2 + SvelteKit project, configure SQLite with rusqlite + WAL mode, implement basic IPC scaffolding, then add cc-sdk subprocess spawning as a simple command that returns output.

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- cc-sdk integration: Spawn Claude CLI as child process (subprocess model, not SDK library)
- Auto-detect Claude CLI location (PATH, ~/.claude/bin, common locations) with user override fallback
- Parse tool calls into structured typed events (tool name, args, result) — not raw text
- Use Claude Code's `--resume` flag for session continuation in later phases
- Minimal flags for Phase 1: just prompt and working directory
- Single hardcoded working directory for Phase 1
- Always kill all child processes when Lulu exits (intentional or crash)
- Never auto-kill unresponsive sessions. User-initiated kill only
- Dark mode only, native title bar, sidebar + main area layout
- Warp terminal-like aesthetic
- shadcn/ui + Tailwind CSS for components
- Message-level chunks, not token-by-token streaming

### Claude's Discretion
- CLI output parsing strategy (JSON stream vs text)
- cc-sdk version compatibility approach
- Prompt passing mechanism
- Loading skeleton and placeholder designs
- Exact spacing, typography, color palette
- Database schema design and migration approach
- IPC channel implementation details
- Error state handling patterns

### Deferred Ideas (OUT OF SCOPE)
None — discussion stayed within phase scope
</user_constraints>

---

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| Tauri | 2.x | Desktop app framework | Native performance, small binary size |
| Svelte | 5.x | Frontend framework | Modern reactivity, small bundle |
| SvelteKit | 2.x | Svelte meta-framework | Project scaffolding, routing |
| @sveltejs/adapter-static | latest | SPA adapter | Required for Tauri (no SSR) |
| rusqlite | 0.31.x | SQLite bindings | Primary Rust SQLite crate |
| tokio | 1.x | Async runtime | Multi-session orchestration |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| tailwindcss | 4.x | CSS framework | Styling |
| shadcn-svelte | next | UI components | Pre-built dark-themed components |
| lucide-svelte | latest | Icons | UI icons |
| clsx, tailwind-merge | latest | Class utilities | Conditional Tailwind classes |
| tokio-util | 0.7.x | Stream utilities | Framing process output |
| serde, serde_json | latest | Serialization | IPC payloads |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| rusqlite | sqlx | More compile-time checks, heavier setup |
| rusqlite | diesel | Full ORM, steeper learning curve |
| shadcn-svelte | Skeleton/SvelteBB | Less customization, different aesthetic |
| tokio | async-std | Less mature ecosystem |

**Installation:**
```bash
# Create Tauri + Svelte project
npm create tauri-app@latest lulu -- --template svelte-ts --manager npm

# In src-tauri/
cargo add rusqlite --features bundled
cargo add tokio --features full
cargo add tokio-util --features codec,io-util
cargo add serde --features derive
cargo add serde_json

# In frontend/
npm install -D tailwindcss @tailwindcss/vite
npx shadcn-svelte@next init
npm install lucide-svelte clsx tailwind-merge
```

---

## Architecture Patterns

### Recommended Project Structure
```
lulu/
├── src/                      # Svelte frontend
│   ├── lib/
│   │   ├── components/       # UI components
│   │   ├── stores/           # Svelte stores (state)
│   │   └── utils/            # Helpers (cn, etc.)
│   ├── routes/               # SvelteKit routes
│   └── app.css               # Global styles
├── src-tauri/
│   ├── src/
│   │   ├── main.rs           # Entry point
│   │   ├── lib.rs            # Tauri app setup
│   │   ├── commands/         # Tauri commands
│   │   ├── db/               # SQLite module
│   │   └── session/          # Claude CLI spawner
│   ├── Cargo.toml
│   └── tauri.conf.json
└── package.json
```

### Pattern 1: Tauri Command + Event Streaming
**What:** Invoke Rust from Svelte, receive streaming updates via events
**When to use:** Claude CLI output streaming, progress updates
**Example:**
```rust
// Rust: src-tauri/src/commands/session.rs
use tauri::{AppHandle, Emitter};
use tokio::process::Command;
use tokio_util::codec::{LinesCodec, FramedRead};
use std::process::Stdio;

#[tauri::command]
async fn spawn_session(app: AppHandle, prompt: String) -> Result<(), String> {
    let mut child = Command::new("claude")
        .arg("-p")
        .arg(&prompt)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;

    let stdout = child.stdout.take().unwrap();
    let mut framed = FramedRead::new(stdout, LinesCodec::new());

    tokio::spawn(async move {
        while let Ok(Some(line)) = framed.try_next().await {
            let _ = app.emit("session-output", line);
        }
        let _ = app.emit("session-complete", ());
    });

    Ok(())
}
```
```typescript
// Svelte: src/lib/components/Session.svelte
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';

let output = $state('');
let started = $state(false);

async function startSession(prompt: string) {
    started = true;
    await invoke('spawn_session', { prompt });
    
    await listen('session-output', (event) => {
        output += event.payload + '\n';
    });
    
    await listen('session-complete', () => {
        started = false;
    });
}
```

### Pattern 2: SQLite with WAL + Immediate Transactions
**What:** Configure SQLite for concurrent access from Tokio tasks
**When to use:** Multiple session managers writing to database
**Example:**
```rust
// src-tauri/src/db/mod.rs
use rusqlite::{Connection, Result};
use std::sync::Mutex;

pub fn init_db(path: &str) -> Result<Connection> {
    let conn = Connection::open(path)?;
    
    // WAL mode for concurrent reads
    conn.execute_batch(
        "PRAGMA journal_mode=WAL;
         PRAGMA synchronous=NORMAL;
         PRAGMA busy_timeout=5000;"
    )?;
    
    Ok(conn)
}

// Use TransactionBehavior::Immediate for writes
pub fn write_session(conn: &Mutex<Connection>, session: &Session) -> Result<()> {
    let conn = conn.lock().unwrap();
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    
    tx.execute(
        "INSERT INTO sessions (id, name, status, created_at) VALUES (?1, ?2, ?3, ?4)",
        [&session.id, &session.name, &session.status, &session.created_at],
    )?;
    
    tx.commit()?;
    Ok(())
}
```

### Pattern 3: Process Cleanup on Exit
**What:** Ensure all child processes are killed when app exits
**When to use:** Always for CLI subprocess management
**Example:**
```rust
// src-tauri/src/lib.rs
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            // Store process handles in app state for cleanup
            let session_manager = SessionManager::new();
            app.manage(Mutex::new(session_manager));
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                // Kill all child processes
                let app = window.app_handle();
                if let Some(state) = app.try_state::<Mutex<SessionManager>>() {
                    let mut manager = state.lock().unwrap();
                    manager.kill_all();
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|--------------|-----|
| IPC between Rust/Svelte | Custom WebSocket | Tauri events + invoke | Built-in, type-safe, secure |
| SQLite connection | Manual pooling | rusqlite + Mutex | Handles concurrency correctly |
| Async process spawn | std::process::Command | tokio::process::Command | Non-blocking, integrates with async |
| JSON serialization | Manual parsing | serde_json | Battle-tested, type-safe |
| Dark theme | Custom CSS | shadcn-svelte | Consistent, accessible, fast |

---

## Common Pitfalls

### Pitfall 1: SQLite Write Lock Contention
**What goes wrong:** Multiple Tokio tasks trying to write simultaneously get SQLITE_BUSY errors
**Why it happens:** Default SQLite journal mode doesn't allow concurrent writes
**How to avoid:** 
- Enable WAL mode: `PRAGMA journal_mode=WAL`
- Use BEGIN IMMEDIATE for writes: `transaction_with_behavior(TransactionBehavior::Immediate)`
- Set busy timeout: `PRAGMA busy_timeout=5000`
**Warning signs:** "database is locked" errors in logs

### Pitfall 2: Process Output Not Captured
**What goes wrong:** `stdout` is always empty when spawning processes
**Why it happens:** Forgot to set `Stdio::piped()` before spawning
**How to avoid:**
```rust
let mut child = Command::new("claude")
    .stdout(Stdio::piped())  // Must set BEFORE spawn
    .stderr(Stdio::piped())
    .spawn()
    .unwrap();
```
**Warning signs:** Empty output despite process running

### Pitfall 3: Tauri SSR Issues
**What goes wrong:** SvelteKit load functions fail in production build
**Why it happens:** Tauri doesn't support SSR - tried to use server-side features
**How to avoid:**
- Use `@sveltejs/adapter-static`
- Add `export const ssr = false` to `+layout.ts`
- Use SPA mode, not prerendered
**Warning signs:** Build errors about missing Node APIs

### Pitfall 4: Frontend Can't Find Tauri APIs
**What goes wrong:** `invoke`/`listen` imports fail at runtime
**Why it happens:** Using wrong import path or missing Tauri API package
**How to avoid:**
```bash
npm install @tauri-apps/api
```
```typescript
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
```
**Warning signs:** `window.__TAURI__` is undefined

### Pitfall 5: Child Processes Not Killed on Exit
**What goes wrong:** Claude CLI keeps running after app closes
**Why it happens:** No cleanup handler registered
**How to avoid:** Register `on_window_event` handler to kill processes on close
**Warning signs:** `claude` processes visible in system monitor after app closes

### Pitfall 6: tokio::select! Macro Misuse
**What goes wrong:** Race conditions between process wait and event listener
**Why it happens:** Not properly handling both futures in select
**How to avoid:** Use `tokio::select!` with proper branch handling:
```rust
tokio::select! {
    _ = child.wait() => { /* process ended */ }
    _ = rx => { /* external signal */ child.kill().await }
}
```

---

## Code Examples

### Database Initialization
```rust
// src-tauri/src/db/mod.rs
use rusqlite::{Connection, Result};
use std::path::Path;

pub fn init_database(db_path: &Path) -> Result<Connection> {
    let conn = Connection::open(db_path)?;
    
    // Enable WAL mode for better concurrency
    conn.execute_batch(
        "PRAGMA journal_mode=WAL;
         PRAGMA synchronous=NORMAL;
         PRAGMA busy_timeout=5000;
         PRAGMA foreign_keys=ON;"
    )?;
    
    // Create tables
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS sessions (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'created',
            working_dir TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        
        CREATE TABLE IF NOT EXISTS messages (
            id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL,
            role TEXT NOT NULL,
            content TEXT NOT NULL,
            timestamp TEXT NOT NULL,
            FOREIGN KEY (session_id) REFERENCES sessions(id)
        );"
    )?;
    
    Ok(conn)
}
```

### Claude CLI Detection
```rust
// src-tauri/src/session/cli.rs
use std::path::PathBuf;
use which::which;

pub fn find_claude_cli() -> Option<PathBuf> {
    // Try PATH first
    if let Ok(path) = which("claude") {
        return Some(path);
    }
    
    // Try common locations
    let home = std::env::var("HOME").ok()?;
    let locations = [
        format!("{}/.claude/bin/claude", home),
        format!("{}/.local/bin/claude", home),
        "/usr/local/bin/claude".to_string(),
    ];
    
    for location in locations {
        let path = PathBuf::from(&location);
        if path.exists() {
            return Some(path);
        }
    }
    
    None
}
```

### Tauri Event Emission
```rust
// Rust side
use tauri::{AppHandle, Emitter};

#[tauri::command]
async fn start_session(app: AppHandle, prompt: String) -> Result<String, String> {
    let session_id = uuid::Uuid::new_v4().to_string();
    
    app.emit("session-started", &session_id).map_err(|e| e.to_string())?;
    
    // ... spawn process ...
    
    Ok(session_id)
}

// Svelte side
import { listen } from '@tauri-apps/api/event';

onMount(() => {
    listen('session-started', (event) => {
        console.log('Session started:', event.payload);
    });
});
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Tauri v1 | Tauri v2 | 2024 | New plugin system, better security |
| Svelte 4 | Svelte 5 | 2024 | Runes reactivity, better TypeScript |
| SvelteKit SSR | adapter-static SPA | Always | Required for Tauri |
| Connection pool | Mutex + WAL | Standard | Simpler, works with Tokio |
| Raw process output | tokio-util framing | Common pattern | Structured line streaming |

**Deprecated/outdated:**
- `std::process::Command` — Use `tokio::process::Command` for async
- Svelte stores (legacy) — Use Svelte 5 runes (`$state`, `$derived`)
- Tailwind v3 — v4 is current with new `@tailwindcss/vite` plugin

---

## Open Questions

1. **CLI output parsing strategy**
   - What we know: Claude CLI can output JSON with `--json` flag in recent versions
   - What's unclear: Whether `--json` is available in all versions user might have
   - Recommendation: Implement JSON-first, fallback to text parsing

2. **Database schema for sessions**
   - What we know: Need sessions and messages tables minimum
   - What's unclear: What additional fields for Phase 1 vs future phases
   - Recommendation: Start minimal, add columns as needed (Phase 1 only needs working skeleton)

3. **Error handling between Rust/JS**
   - What we know: Tauri commands can return `Result<T, String>`
   - What's unclear: Best pattern for propagating errors to UI
   - Recommendation: Use typed error enums serialized as strings for Phase 1

---

## Sources

### Primary (HIGH confidence)
- Tauri v2 Docs: https://v2.tauri.app/ — Official setup, commands, events
- rusqlite docs: https://docs.rs/rusqlite — Concurrent access patterns
- tokio::process: https://docs.rs/tokio/latest/tokio/process/ — Async subprocess
- shadcn-svelte: https://shadcn-svelte.com/ — UI components

### Secondary (MEDIUM confidence)
- Reddit r/sveltejs: Tauri v2 + Svelte 5 setup guide
- Stack Overflow: tokio process streaming patterns
- GitHub discussions: SQLite WAL mode in Rust

### Tertiary (LOW confidence)
- Various blog posts on SQLite concurrency — marked for validation during implementation

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - All libraries well-documented, widely used
- Architecture: HIGH - Tauri v2 patterns stable, IPC patterns well-documented
- Pitfalls: HIGH - Common issues with known solutions

**Research date:** 2026-02-14
**Valid until:** 2026-03-14 (30 days for stable stack)

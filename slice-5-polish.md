# Slice 5: Polish + External Access

## Goal

Add an embedded HTTP/SSE server for external clients, an optional CLI companion, settings/configuration UI, and UX polish (syntax highlighting, collapsible sections, search).

## Prerequisites

- Slices 2-4 complete

---

## Step 1: Internal event bus

**Create `src-tauri/src/events/mod.rs`**:

```rust
pub mod sse;

use tokio::sync::broadcast;
use serde::Serialize;

#[derive(Clone, Serialize, Debug)]
#[serde(tag = "type")]
pub enum AppEvent {
    SessionStatusChanged {
        session_id: String,
        old_status: String,
        new_status: String,
    },
    NewApproval {
        approval_id: String,
        session_id: String,
        tool_name: String,
    },
    ApprovalResolved {
        approval_id: String,
        decision: String,
    },
    ConversationUpdated {
        session_id: String,
        event_count: usize,
    },
}

pub struct EventBus {
    tx: broadcast::Sender<AppEvent>,
}

impl EventBus {
    pub fn new(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity);
        Self { tx }
    }

    pub fn publish(&self, event: AppEvent) {
        // Ignore error (no subscribers)
        let _ = self.tx.send(event);
    }

    pub fn subscribe(&self) -> broadcast::Receiver<AppEvent> {
        self.tx.subscribe()
    }
}
```

Integrate: publish events from runner, approval manager, and session lifecycle. Manage as Tauri state.

---

## Step 2: Embedded SSE server

**Create `src-tauri/src/events/sse.rs`**:

```rust
use axum::{
    Router,
    routing::get,
    response::sse::{Event, Sse},
    extract::State,
};
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;
use std::sync::Arc;
use crate::events::EventBus;

pub async fn start_sse_server(
    event_bus: Arc<EventBus>,
    port: u16,
) {
    let app = Router::new()
        .route("/events", get(sse_handler))
        .route("/health", get(|| async { "ok" }))
        .with_state(event_bus);

    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", port))
        .await
        .expect("failed to bind SSE server");

    axum::serve(listener, app).await.expect("SSE server error");
}

async fn sse_handler(
    State(event_bus): State<Arc<EventBus>>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, std::convert::Infallible>>> {
    let rx = event_bus.subscribe();
    let stream = BroadcastStream::new(rx)
        .filter_map(|result| {
            result.ok().map(|event| {
                let data = serde_json::to_string(&event).unwrap_or_default();
                Ok(Event::default().data(data))
            })
        });

    Sse::new(stream)
}
```

Add REST endpoints for session and approval management:

```rust
// Additional routes:
.route("/sessions", get(list_sessions_handler).post(create_session_handler))
.route("/sessions/:id", get(get_session_handler))
.route("/approvals", get(list_approvals_handler))
.route("/approvals/:id/resolve", post(resolve_approval_handler))
```

These handlers delegate to the same store/approval logic used by Tauri commands.

### Dependencies

Add to `Cargo.toml`:

```toml
axum = "0.8"
tokio-stream = "0.1"
```

---

## Step 3: Launch SSE server from Tauri setup

**`src-tauri/src/main.rs`** — In setup:

```rust
.setup(|app| {
    // ... existing database init ...

    let event_bus = Arc::new(EventBus::new(256));
    app.manage(event_bus.clone());

    // Start SSE server in background
    let port = 7778; // configurable via settings
    tauri::async_runtime::spawn(async move {
        events::sse::start_sse_server(event_bus, port).await;
    });

    Ok(())
})
```

---

## Step 4: Optional CLI companion

**Create `src-tauri/src/bin/lulu-cli.rs`** (or a separate `lulu-cli/` workspace member):

A minimal CLI that talks to the embedded HTTP server:

```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "lulu")]
struct Cli {
    /// Server port (default: 7778)
    #[arg(short, long, default_value = "7778")]
    port: u16,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List sessions
    Sessions,
    /// Run a new session
    Run {
        /// The prompt
        prompt: String,
    },
    /// List pending approvals
    Approvals,
    /// Approve a pending tool call
    Approve {
        /// Approval ID
        id: String,
    },
    /// Deny a pending tool call
    Deny {
        /// Approval ID
        id: String,
    },
    /// Stream events (SSE)
    Watch,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let base = format!("http://127.0.0.1:{}", cli.port);

    match cli.command {
        Commands::Sessions => {
            let resp = reqwest::get(format!("{}/sessions", base)).await.unwrap();
            println!("{}", resp.text().await.unwrap());
        }
        Commands::Watch => {
            // Stream SSE events to stdout
            let mut resp = reqwest::get(format!("{}/events", base)).await.unwrap();
            while let Some(chunk) = resp.chunk().await.unwrap() {
                print!("{}", String::from_utf8_lossy(&chunk));
            }
        }
        // ... other commands similarly ...
        _ => todo!()
    }
}
```

Add `clap` and `reqwest` to dependencies.

---

## Step 5: Settings page

**Create `src/pages/Settings.tsx`**:

```tsx
import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

export function Settings() {
  // Sections:
  // 1. Default model selection (dropdown: opus, sonnet, haiku)
  // 2. Default working directory (file picker)
  // 3. Auto-approve rules (list + add/delete from Slice 3)
  // 4. API server port (number input)
  // 5. Theme (light/dark toggle)

  return (
    <div className="p-6 max-w-2xl mx-auto space-y-8">
      <h1 className="text-xl font-bold">Settings</h1>

      <section>
        <h2 className="text-sm font-semibold mb-2">Default Model</h2>
        {/* Dropdown: opus, sonnet, haiku */}
      </section>

      <section>
        <h2 className="text-sm font-semibold mb-2">Working Directory</h2>
        {/* Path input + browse button */}
      </section>

      <section>
        <h2 className="text-sm font-semibold mb-2">Auto-Approve Rules</h2>
        {/* Table from Slice 3 */}
      </section>

      <section>
        <h2 className="text-sm font-semibold mb-2">API Server</h2>
        {/* Port input, enable/disable toggle */}
      </section>
    </div>
  );
}
```

Add a settings icon/button to the header that navigates to settings.

### Settings persistence

**Database migration 004**:

```sql
CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
```

Tauri commands: `get_setting(key)`, `set_setting(key, value)`.

---

## Step 6: UX polish

### MessageStream enhancements

**`src/components/MessageStream.tsx`** — Upgrade:

1. **Syntax highlighting**: Use `shiki` or `highlight.js` for code blocks
   ```bash
   bun add shiki
   ```
   Wrap code blocks from react-markdown with a syntax highlighter.

2. **Collapsible thinking blocks**: Already `<details>` in Slice 1, but add animation and a "Thinking for Xs" timer.

3. **Copy button on code blocks**: Absolute-positioned button in top-right corner of `<pre>` blocks.

4. **Auto-scroll**: Use `useRef` + `useEffect` to scroll to bottom when new events arrive, unless user has scrolled up.

5. **Tool result formatting**: If tool result contains a file diff, render with red/green diff highlighting.

### SessionSidebar enhancements

1. **Search**: Text input at top that filters sessions by title/prompt.

2. **Delete session**: Right-click or swipe to delete (with confirmation dialog).

3. **Auto-title**: After session completes, derive a title from the first prompt line (truncated). Store via `update_session_title`.

### Global keyboard shortcuts

- `Cmd+N` — New session
- `Cmd+Enter` — Submit prompt (already done in Slice 1)
- `Cmd+K` — Focus search in sidebar
- `Escape` — Close approval dialog / deselect session

---

## Step 7: Verify

1. **SSE**: While running a session, `curl http://localhost:7778/events` → see events streaming
2. **REST**: `curl http://localhost:7778/sessions` → see session list as JSON
3. **CLI**: `lulu-cli sessions` → lists sessions; `lulu-cli watch` → streams events
4. **Settings**: Change default model → new sessions use it; change port → server restarts on new port
5. **Polish**: Code blocks have syntax highlighting; thinking blocks are collapsible; copy button works; auto-scroll works; sidebar has search

---

## Files Created/Modified

| File | Action |
|------|--------|
| `src-tauri/Cargo.toml` | Modified — add axum, tokio-stream, clap, reqwest |
| `src-tauri/src/events/mod.rs` | **New** — EventBus |
| `src-tauri/src/events/sse.rs` | **New** — Axum SSE server |
| `src-tauri/src/bin/lulu-cli.rs` | **New** — CLI companion |
| `src-tauri/src/main.rs` | Modified — setup event bus + SSE server |
| `src-tauri/src/store/migrations.rs` | Modified — migration 004 (settings) |
| `src-tauri/src/store/models.rs` | Modified — settings queries |
| `src-tauri/src/commands/mod.rs` | Modified — settings commands |
| `src/pages/Settings.tsx` | **New** |
| `src/components/MessageStream.tsx` | Modified — syntax highlighting, copy, scroll |
| `src/components/SessionSidebar.tsx` | Modified — search, delete, auto-title |
| `src/App.tsx` | Modified — routing to Settings page |
| `package.json` | Modified — add shiki |

## Dependency Summary (Slice 5)

### Rust
```toml
axum = "0.8"
tokio-stream = "0.1"
clap = { version = "4", features = ["derive"] }
reqwest = { version = "0.12", features = ["json"] }
```

### JavaScript
```bash
bun add shiki
```

# Slice 2: Session Persistence + Listing

## Goal

Sessions persist to SQLite so they survive app restarts. A sidebar lists all past sessions; clicking one loads its full conversation history.

## Prerequisites

- Slice 1 complete (streaming works end-to-end)

---

## Step 1: Add Rust dependencies

**`src-tauri/Cargo.toml`** — Add:

```toml
rusqlite = { version = "0.32", features = ["bundled"] }
```

---

## Step 2: Create the database module

**Create `src-tauri/src/store/mod.rs`**:

```rust
pub mod migrations;
pub mod models;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use rusqlite::Connection;

pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

impl Database {
    pub fn init(app_data_dir: PathBuf) -> Result<Self, String> {
        std::fs::create_dir_all(&app_data_dir).map_err(|e| e.to_string())?;
        let db_path = app_data_dir.join("lulu.db");
        let conn = Connection::open(db_path).map_err(|e| e.to_string())?;

        // Enable WAL mode for better concurrent read performance
        conn.execute_batch("PRAGMA journal_mode=WAL;").map_err(|e| e.to_string())?;

        migrations::run(&conn).map_err(|e| e.to_string())?;

        Ok(Self { conn: Arc::new(Mutex::new(conn)) })
    }

    pub fn conn(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn.lock().expect("database lock poisoned")
    }
}
```

---

## Step 3: Create migrations

**Create `src-tauri/src/store/migrations.rs`**:

```rust
use rusqlite::Connection;

const MIGRATIONS: &[&str] = &[
    // Migration 001: Sessions and conversation events
    "CREATE TABLE IF NOT EXISTS sessions (
        id TEXT PRIMARY KEY,
        title TEXT,
        prompt TEXT NOT NULL,
        status TEXT NOT NULL DEFAULT 'running',
        model TEXT,
        session_id_claude TEXT,
        cost_usd REAL,
        duration_ms REAL,
        num_turns INTEGER,
        created_at TEXT NOT NULL DEFAULT (datetime('now')),
        updated_at TEXT NOT NULL DEFAULT (datetime('now'))
    );

    CREATE TABLE IF NOT EXISTS conversation_events (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        session_id TEXT NOT NULL REFERENCES sessions(id),
        event_type TEXT NOT NULL,
        payload TEXT NOT NULL,
        created_at TEXT NOT NULL DEFAULT (datetime('now'))
    );

    CREATE INDEX IF NOT EXISTS idx_events_session
        ON conversation_events(session_id, id);

    CREATE TABLE IF NOT EXISTS schema_version (
        version INTEGER PRIMARY KEY
    );
    INSERT OR IGNORE INTO schema_version (version) VALUES (1);",
];

pub fn run(conn: &Connection) -> Result<(), rusqlite::Error> {
    let current_version: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_version",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    for (i, migration) in MIGRATIONS.iter().enumerate() {
        let version = (i + 1) as i64;
        if version > current_version {
            conn.execute_batch(migration)?;
        }
    }

    Ok(())
}
```

---

## Step 4: Create model types and query helpers

**Create `src-tauri/src/store/models.rs`**:

```rust
use serde::Serialize;

#[derive(Serialize)]
pub struct SessionSummary {
    pub id: String,
    pub title: Option<String>,
    pub prompt: String,
    pub status: String,
    pub cost_usd: Option<f64>,
    pub num_turns: Option<i64>,
    pub created_at: String,
}

#[derive(Serialize)]
pub struct SessionDetail {
    pub session: SessionSummary,
    pub events: Vec<StoredEvent>,
}

#[derive(Serialize)]
pub struct StoredEvent {
    pub id: i64,
    pub event_type: String,
    pub payload: serde_json::Value,  // Deserialized from stored JSON
    pub created_at: String,
}

// --- Query helpers (operate on &Connection) ---

use rusqlite::{params, Connection};
use crate::claude::StreamEvent;

pub fn insert_session(conn: &Connection, id: &str, prompt: &str) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT INTO sessions (id, prompt, status) VALUES (?1, ?2, 'running')",
        params![id, prompt],
    )?;
    Ok(())
}

pub fn update_session_completed(
    conn: &Connection,
    id: &str,
    status: &str,
    cost_usd: Option<f64>,
    duration_ms: Option<f64>,
    num_turns: Option<u32>,
    claude_session_id: Option<&str>,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "UPDATE sessions SET status = ?2, cost_usd = ?3, duration_ms = ?4,
         num_turns = ?5, session_id_claude = ?6, updated_at = datetime('now')
         WHERE id = ?1",
        params![id, status, cost_usd, duration_ms, num_turns, claude_session_id],
    )?;
    Ok(())
}

pub fn insert_event(
    conn: &Connection,
    session_id: &str,
    event_type: &str,
    payload: &str,  // JSON string
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT INTO conversation_events (session_id, event_type, payload) VALUES (?1, ?2, ?3)",
        params![session_id, event_type, payload],
    )?;
    Ok(())
}

pub fn list_sessions(conn: &Connection) -> Result<Vec<SessionSummary>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT id, title, prompt, status, cost_usd, num_turns, created_at
         FROM sessions ORDER BY created_at DESC"
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(SessionSummary {
            id: row.get(0)?,
            title: row.get(1)?,
            prompt: row.get(2)?,
            status: row.get(3)?,
            cost_usd: row.get(4)?,
            num_turns: row.get(5)?,
            created_at: row.get(6)?,
        })
    })?;
    rows.collect()
}

pub fn get_session_events(conn: &Connection, session_id: &str) -> Result<Vec<StoredEvent>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT id, event_type, payload, created_at
         FROM conversation_events WHERE session_id = ?1 ORDER BY id"
    )?;
    let rows = stmt.query_map(params![session_id], |row| {
        let payload_str: String = row.get(2)?;
        let payload: serde_json::Value = serde_json::from_str(&payload_str)
            .unwrap_or(serde_json::Value::Null);
        Ok(StoredEvent {
            id: row.get(0)?,
            event_type: row.get(1)?,
            payload,
            created_at: row.get(3)?,
        })
    })?;
    rows.collect()
}
```

---

## Step 5: Modify the runner to persist events

**`src-tauri/src/claude/runner.rs`** — Updated signature and body:

```rust
pub async fn run_query(
    session_id: String,
    prompt: String,
    db: Arc<Database>,
    channel: Channel<StreamEvent>,
) -> Result<(), String> {
    // Insert session row
    {
        let conn = db.conn();
        models::insert_session(&conn, &session_id, &prompt)
            .map_err(|e| e.to_string())?;
    }

    let options = cc_sdk::ClaudeCodeOptions::builder().build();
    let mut stream = cc_sdk::query(&prompt, Some(options))
        .await
        .map_err(|e| {
            // Mark session as failed
            let conn = db.conn();
            let _ = models::update_session_completed(&conn, &session_id, "failed", None, None, None, None);
            e.to_string()
        })?;

    while let Some(result) = stream.next().await {
        match result {
            Ok(message) => {
                let event = map_message_to_event(message);
                let event_type = event_type_str(&event);
                let payload_json = serde_json::to_string(&event).unwrap_or_default();

                // Persist to SQLite
                {
                    let conn = db.conn();
                    let _ = models::insert_event(&conn, &session_id, event_type, &payload_json);
                }

                // Update session on completion
                if let StreamEvent::SessionResult { ref session_id: ref sid, duration_ms, cost_usd, is_error, num_turns } = event {
                    let status = if is_error { "failed" } else { "completed" };
                    let conn = db.conn();
                    let _ = models::update_session_completed(
                        &conn, &session_id, status, cost_usd, Some(duration_ms), Some(num_turns), Some(sid),
                    );
                }

                let _ = channel.send(event);
            }
            Err(e) => {
                let conn = db.conn();
                let _ = models::update_session_completed(&conn, &session_id, "failed", None, None, None, None);
                let _ = channel.send(StreamEvent::Error { message: e.to_string() });
                break;
            }
        }
    }

    Ok(())
}

fn event_type_str(event: &StreamEvent) -> &'static str {
    match event {
        StreamEvent::Text { .. } => "text",
        StreamEvent::Thinking { .. } => "thinking",
        StreamEvent::ToolUse { .. } => "tool_use",
        StreamEvent::ToolResult { .. } => "tool_result",
        StreamEvent::SystemMessage { .. } => "system",
        StreamEvent::SessionResult { .. } => "result",
        StreamEvent::Error { .. } => "error",
    }
}
```

---

## Step 6: Add new Tauri commands

**`src-tauri/src/commands/mod.rs`** — Add:

```rust
use crate::store::{Database, models};

#[tauri::command]
pub async fn list_sessions(
    db: tauri::State<'_, Database>,
) -> Result<Vec<models::SessionSummary>, String> {
    let conn = db.conn();
    models::list_sessions(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_session(
    session_id: String,
    db: tauri::State<'_, Database>,
) -> Result<models::SessionDetail, String> {
    let conn = db.conn();
    let sessions = models::list_sessions(&conn).map_err(|e| e.to_string())?;
    let session = sessions.into_iter()
        .find(|s| s.id == session_id)
        .ok_or("Session not found")?;
    let events = models::get_session_events(&conn, &session_id)
        .map_err(|e| e.to_string())?;
    Ok(models::SessionDetail { session, events })
}
```

Update `launch_session` to accept `db: tauri::State<'_, Database>` and pass `Arc<Database>` to the runner.

---

## Step 7: Initialize Database in main.rs

**`src-tauri/src/main.rs`** — Modified:

```rust
mod claude;
mod commands;
mod store;

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir()
                .expect("failed to get app data dir");
            let db = store::Database::init(app_data_dir)
                .expect("failed to initialize database");
            app.manage(db);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::launch_session,
            commands::list_sessions,
            commands::get_session,
        ])
        .run(tauri::generate_context!())
        .expect("error running lulu");
}
```

---

## Step 8: Build the SessionSidebar component

**Create `src/components/SessionSidebar.tsx`**:

```tsx
import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

interface SessionSummary {
  id: string;
  title: string | null;
  prompt: string;
  status: string;
  cost_usd: number | null;
  num_turns: number | null;
  created_at: string;
}

interface Props {
  selectedId: string | null;
  onSelect: (id: string | null) => void;
  refreshKey: number;  // Increment to trigger refresh
}

export function SessionSidebar({ selectedId, onSelect, refreshKey }: Props) {
  const [sessions, setSessions] = useState<SessionSummary[]>([]);

  useEffect(() => {
    invoke<SessionSummary[]>("list_sessions").then(setSessions);
  }, [refreshKey]);

  return (
    <aside className="w-64 border-r overflow-y-auto">
      <div className="p-3 border-b flex justify-between items-center">
        <span className="text-sm font-semibold">Sessions</span>
        <button
          onClick={() => onSelect(null)}
          className="text-xs text-blue-600 hover:underline"
        >
          + New
        </button>
      </div>
      {sessions.map((s) => (
        <button
          key={s.id}
          onClick={() => onSelect(s.id)}
          className={`w-full text-left p-3 border-b text-sm hover:bg-gray-50
            ${selectedId === s.id ? "bg-blue-50" : ""}`}
        >
          <div className="truncate font-medium">
            {s.title ?? s.prompt.slice(0, 50)}
          </div>
          <div className="text-xs text-gray-500 mt-1 flex gap-2">
            <StatusBadge status={s.status} />
            <span>{new Date(s.created_at).toLocaleDateString()}</span>
          </div>
        </button>
      ))}
    </aside>
  );
}

function StatusBadge({ status }: { status: string }) {
  const colors: Record<string, string> = {
    running: "text-yellow-600",
    completed: "text-green-600",
    failed: "text-red-600",
  };
  return <span className={colors[status] ?? "text-gray-600"}>{status}</span>;
}
```

---

## Step 9: Update Home page with two-column layout

**`src/pages/Home.tsx`** (or update `src/App.tsx`):

```tsx
import { useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useSession } from "./hooks/useSession";
import { PromptInput } from "./components/PromptInput";
import { MessageStream } from "./components/MessageStream";
import { SessionSidebar } from "./components/SessionSidebar";

export default function App() {
  const { events, isRunning, launch } = useSession();
  const [selectedSessionId, setSelectedSessionId] = useState<string | null>(null);
  const [refreshKey, setRefreshKey] = useState(0);
  const [historicalEvents, setHistoricalEvents] = useState<any[]>([]);

  const handleSelect = useCallback(async (id: string | null) => {
    setSelectedSessionId(id);
    if (id) {
      const detail = await invoke<any>("get_session", { sessionId: id });
      setHistoricalEvents(detail.events.map((e: any) => e.payload));
    } else {
      setHistoricalEvents([]);
    }
  }, []);

  const handleLaunch = useCallback((prompt: string) => {
    setSelectedSessionId(null);
    setHistoricalEvents([]);
    launch(prompt);
    // Refresh sidebar after a short delay to show new session
    setTimeout(() => setRefreshKey((k) => k + 1), 500);
  }, [launch]);

  const displayedEvents = selectedSessionId ? historicalEvents : events;

  return (
    <div className="flex h-screen">
      <SessionSidebar
        selectedId={selectedSessionId}
        onSelect={handleSelect}
        refreshKey={refreshKey}
      />
      <div className="flex flex-col flex-1">
        <header className="border-b px-4 py-2 text-sm font-semibold">lulu</header>
        <MessageStream events={displayedEvents} />
        {!selectedSessionId && (
          <PromptInput onSubmit={handleLaunch} isRunning={isRunning} />
        )}
      </div>
    </div>
  );
}
```

---

## Step 10: Verify

1. Run a session → restart app → session visible in sidebar
2. Click past session → see full conversation history
3. New sessions appear in sidebar while running
4. Status badges show correct state (running/completed/failed)

---

## Files Created/Modified

| File | Action |
|------|--------|
| `src-tauri/Cargo.toml` | Modified — add rusqlite |
| `src-tauri/src/main.rs` | Modified — add store module, Database setup |
| `src-tauri/src/store/mod.rs` | **New** — Database struct |
| `src-tauri/src/store/migrations.rs` | **New** — Schema migrations |
| `src-tauri/src/store/models.rs` | **New** — Types + query helpers |
| `src-tauri/src/claude/runner.rs` | Modified — persist events + session status |
| `src-tauri/src/commands/mod.rs` | Modified — add list_sessions, get_session |
| `src/components/SessionSidebar.tsx` | **New** |
| `src/App.tsx` | Modified — two-column layout |

# Slice 4: Session Lifecycle

## Goal

Support the full session lifecycle: draft sessions (compose before running), continue completed sessions with a new prompt, fork sessions, and interrupt running sessions.

## Prerequisites

- Slice 2 complete (session persistence)
- Can be built in parallel with Slice 3

---

## Step 1: Database migration

**`src-tauri/src/store/migrations.rs`** — Add Migration 003:

```sql
ALTER TABLE sessions ADD COLUMN parent_session_id TEXT REFERENCES sessions(id);
ALTER TABLE sessions ADD COLUMN working_directory TEXT;
```

Update the `status` column to support these values:
- `draft` — created but not yet launched
- `starting` — transitioning to running
- `running` — Claude Code subprocess active
- `completed` — finished successfully
- `failed` — finished with error
- `interrupted` — user stopped the session

Add query helpers to `store/models.rs`:
- `insert_draft_session(conn, id, prompt, working_dir?) -> Result<()>`
- `update_session_status(conn, id, status) -> Result<()>`
- `get_session_by_id(conn, id) -> Result<SessionRow>` (full row, not just summary)

---

## Step 2: Create the SessionManager

**Create `src-tauri/src/session/mod.rs`**:

```rust
pub mod lifecycle;

use std::sync::Arc;
use dashmap::DashMap;
use tokio::sync::watch;

/// Signal sent to a running session.
#[derive(Clone, Debug)]
pub enum SessionControl {
    Continue,
    Interrupt,
}

/// Tracks a running session's control channel and task handle.
pub struct RunningSession {
    pub control_tx: watch::Sender<SessionControl>,
    pub handle: tokio::task::JoinHandle<()>,
}

pub struct SessionManager {
    running: Arc<DashMap<String, RunningSession>>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            running: Arc::new(DashMap::new()),
        }
    }

    /// Register a running session for lifecycle control.
    pub fn register(&self, session_id: String, session: RunningSession) {
        self.running.insert(session_id, session);
    }

    /// Remove a session when it completes.
    pub fn unregister(&self, session_id: &str) {
        self.running.remove(session_id);
    }

    /// Send interrupt signal to a running session.
    pub fn interrupt(&self, session_id: &str) -> Result<(), String> {
        let entry = self.running.get(session_id)
            .ok_or_else(|| format!("Session {} not running", session_id))?;
        entry.control_tx.send(SessionControl::Interrupt)
            .map_err(|e| e.to_string())
    }

    /// Check if a session is currently running.
    pub fn is_running(&self, session_id: &str) -> bool {
        self.running.contains_key(session_id)
    }
}
```

---

## Step 3: Implement lifecycle operations

**Create `src-tauri/src/session/lifecycle.rs`**:

```rust
use crate::store::{Database, models};
use crate::session::{SessionManager, RunningSession, SessionControl};
use crate::claude;
use std::sync::Arc;
use tokio::sync::watch;

/// Create a draft session (not launched yet).
pub fn create_draft(
    db: &Database,
    prompt: &str,
    working_dir: Option<&str>,
) -> Result<String, String> {
    let id = uuid::Uuid::new_v4().to_string();
    let conn = db.conn();
    models::insert_draft_session(&conn, &id, prompt, working_dir)
        .map_err(|e| e.to_string())?;
    Ok(id)
}

/// Launch a draft session (transitions draft → starting → running).
pub async fn start_session(
    session_id: &str,
    db: Arc<Database>,
    session_mgr: Arc<SessionManager>,
    // ... other deps (approval_mgr, app_handle, channel)
) -> Result<(), String> {
    // Validate session is in draft status
    let session = {
        let conn = db.conn();
        models::get_session_by_id(&conn, session_id).map_err(|e| e.to_string())?
    };
    if session.status != "draft" {
        return Err(format!("Session {} is not a draft (status: {})", session_id, session.status));
    }

    // Transition to starting
    {
        let conn = db.conn();
        models::update_session_status(&conn, session_id, "starting")
            .map_err(|e| e.to_string())?;
    }

    // Create control channel
    let (control_tx, control_rx) = watch::channel(SessionControl::Continue);

    // Spawn the runner
    let sid = session_id.to_string();
    let db_clone = db.clone();
    let handle = tauri::async_runtime::spawn(async move {
        // Pass control_rx to runner so it can check for interrupts
        // claude::runner::run_query_with_control(sid, prompt, db_clone, control_rx, ...).await
    });

    session_mgr.register(session_id.to_string(), RunningSession { control_tx, handle });
    Ok(())
}

/// Interrupt a running session.
pub async fn interrupt_session(
    session_id: &str,
    db: &Database,
    session_mgr: &SessionManager,
) -> Result<(), String> {
    session_mgr.interrupt(session_id)?;

    // Update status to interrupted
    let conn = db.conn();
    models::update_session_status(&conn, session_id, "interrupted")
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Continue a completed session with a new prompt.
/// Creates a new session linked to the parent.
pub async fn continue_session(
    parent_session_id: &str,
    new_prompt: &str,
    db: Arc<Database>,
    session_mgr: Arc<SessionManager>,
    // ... other deps
) -> Result<String, String> {
    // Validate parent is completed
    let parent = {
        let conn = db.conn();
        models::get_session_by_id(&conn, parent_session_id).map_err(|e| e.to_string())?
    };
    if parent.status != "completed" && parent.status != "interrupted" {
        return Err(format!("Cannot continue session in status: {}", parent.status));
    }

    // Create new session with parent reference
    let new_id = uuid::Uuid::new_v4().to_string();
    {
        let conn = db.conn();
        models::insert_session_with_parent(&conn, &new_id, new_prompt, parent_session_id,
            parent.working_directory.as_deref())
            .map_err(|e| e.to_string())?;
    }

    // Launch with --resume <claude_session_id> via cc-sdk options
    // The cc-sdk ClaudeCodeOptions should support session_id for resumption
    // let options = ClaudeCodeOptions::builder()
    //     .session_id(parent.session_id_claude)
    //     .build();

    // ... spawn runner with resume options ...

    Ok(new_id)
}

/// Fork a session (create a copy to experiment with).
pub fn fork_session(
    original_session_id: &str,
    db: &Database,
) -> Result<String, String> {
    let original = {
        let conn = db.conn();
        models::get_session_by_id(&conn, original_session_id).map_err(|e| e.to_string())?
    };

    let new_id = uuid::Uuid::new_v4().to_string();
    let conn = db.conn();
    models::insert_draft_session(&conn, &new_id, &original.prompt,
        original.working_directory.as_deref())
        .map_err(|e| e.to_string())?;
    // Set parent_session_id
    // models::set_parent_session(&conn, &new_id, original_session_id)?;

    Ok(new_id)
}
```

---

## Step 4: Modify the runner for interrupt support

**`src-tauri/src/claude/runner.rs`** — Add interrupt handling:

```rust
use tokio::sync::watch;
use crate::session::SessionControl;

pub async fn run_query_with_control(
    session_id: String,
    prompt: String,
    db: Arc<Database>,
    mut control_rx: watch::Receiver<SessionControl>,
    channel: Channel<StreamEvent>,
    // ... other deps
) -> Result<(), String> {
    // ... setup ...

    loop {
        tokio::select! {
            // Check for interrupt signal
            _ = control_rx.changed() => {
                if matches!(*control_rx.borrow(), SessionControl::Interrupt) {
                    // Drop the stream (kills subprocess)
                    let conn = db.conn();
                    let _ = models::update_session_status(&conn, &session_id, "interrupted");
                    let _ = channel.send(StreamEvent::Error {
                        message: "Session interrupted by user".into(),
                    });
                    break;
                }
            }
            // Process next stream event
            event = stream.next() => {
                match event {
                    Some(Ok(message)) => {
                        // ... existing event handling ...
                    }
                    Some(Err(e)) => {
                        // ... error handling ...
                        break;
                    }
                    None => break, // Stream ended
                }
            }
        }
    }

    Ok(())
}
```

---

## Step 5: Add Tauri commands

**Create `src-tauri/src/commands/session_cmds.rs`**:

```rust
use crate::session::{self, SessionManager};
use crate::store::Database;

#[tauri::command]
pub async fn create_draft_session(
    prompt: String,
    working_dir: Option<String>,
    db: tauri::State<'_, Database>,
) -> Result<String, String> {
    session::lifecycle::create_draft(&db, &prompt, working_dir.as_deref())
}

#[tauri::command]
pub async fn start_draft_session(
    session_id: String,
    db: tauri::State<'_, Database>,
    session_mgr: tauri::State<'_, SessionManager>,
    channel: tauri::ipc::Channel<crate::claude::StreamEvent>,
) -> Result<(), String> {
    // ... delegates to lifecycle::start_session
    todo!()
}

#[tauri::command]
pub async fn interrupt_session(
    session_id: String,
    db: tauri::State<'_, Database>,
    session_mgr: tauri::State<'_, SessionManager>,
) -> Result<(), String> {
    session::lifecycle::interrupt_session(&session_id, &db, &session_mgr).await
}

#[tauri::command]
pub async fn continue_session(
    session_id: String,
    prompt: String,
    db: tauri::State<'_, Database>,
    session_mgr: tauri::State<'_, SessionManager>,
    channel: tauri::ipc::Channel<crate::claude::StreamEvent>,
) -> Result<String, String> {
    // ... delegates to lifecycle::continue_session
    todo!()
}

#[tauri::command]
pub async fn fork_session(
    session_id: String,
    db: tauri::State<'_, Database>,
) -> Result<String, String> {
    session::lifecycle::fork_session(&session_id, &db)
}
```

Register all + manage `SessionManager` in `main.rs`.

---

## Step 6: Update React components

### PromptInput — contextual actions

**`src/components/PromptInput.tsx`** — Modified to show different buttons based on context:

```tsx
interface Props {
  onSubmit: (prompt: string) => void;
  onSaveDraft?: (prompt: string) => void;
  onContinue?: (prompt: string) => void;
  onFork?: () => void;
  onInterrupt?: () => void;
  isRunning: boolean;
  mode: "new" | "viewing-completed" | "viewing-running" | "viewing-draft";
}

// Render buttons based on mode:
// "new"               → [Save Draft] [Run]
// "viewing-completed" → [Continue] [Fork]     (with prompt input)
// "viewing-running"   → [Interrupt]           (no prompt input)
// "viewing-draft"     → [Launch] [Delete]     (prompt is editable)
```

### SessionSidebar — status icons

**`src/components/SessionSidebar.tsx`** — Updated:
- Draft sessions: pencil icon, gray styling
- Running: spinning indicator
- Completed: green check
- Failed: red X
- Interrupted: yellow pause

### Home page — mode awareness

**`src/App.tsx`** — Determine mode from selected session's status and render PromptInput accordingly.

---

## Step 7: Verify

1. **Draft**: Click "Save Draft" → session appears in sidebar as draft → click it → edit prompt → click "Launch" → runs
2. **Interrupt**: While a session is running → click "Interrupt" → session stops, status changes to "interrupted"
3. **Continue**: Click a completed session → type new prompt → click "Continue" → new session created with parent link, Claude resumes context
4. **Fork**: Click a completed session → click "Fork" → new draft session created with same prompt

---

## Files Created/Modified

| File | Action |
|------|--------|
| `src-tauri/src/store/migrations.rs` | Modified — migration 003 |
| `src-tauri/src/store/models.rs` | Modified — new queries |
| `src-tauri/src/session/mod.rs` | **New** — SessionManager |
| `src-tauri/src/session/lifecycle.rs` | **New** — Lifecycle operations |
| `src-tauri/src/claude/runner.rs` | Modified — interrupt support via tokio::select! |
| `src-tauri/src/commands/session_cmds.rs` | **New** — Session commands |
| `src-tauri/src/main.rs` | Modified — manage SessionManager |
| `src/components/PromptInput.tsx` | Modified — contextual action buttons |
| `src/components/SessionSidebar.tsx` | Modified — status icons |
| `src/App.tsx` | Modified — mode-aware rendering |

## Open Questions

1. **cc-sdk resume support**: Does `ClaudeCodeOptions` have a `session_id` or `resume` field? If not, we may need to pass `--resume <id>` manually via a raw args option.
2. **Subprocess kill**: When dropping the cc-sdk stream, does it SIGTERM the subprocess? Or do we need to hold a `Child` handle separately? Investigate during implementation.

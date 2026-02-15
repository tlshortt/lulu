# Architecture Research

**Domain:** Multi-Session AI Agent Orchestrator Desktop App
**Researched:** 2026-02-14
**Confidence:** HIGH

## Standard Architecture

### System Overview

```
┌────────────────────────────────────────────────────────────────────┐
│                         Svelte 5 Frontend                          │
├────────────────────────────────────────────────────────────────────┤
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐             │
│  │ Session      │  │ Approval     │  │ Settings     │             │
│  │ Dashboard    │  │ Rules UI     │  │ Panel        │             │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘             │
│         │                  │                  │                     │
│  ┌──────┴──────────────────┴──────────────────┴───────────┐        │
│  │      Svelte Stores (Runes-based State Management)      │        │
│  └──────────────────────────────────────────┬──────────────┘        │
│                                             │                       │
├─────────────────────────────────────────────┼───────────────────────┤
│                  Tauri IPC Layer            │                       │
│                                             │                       │
│  ┌────────────────────┐    ┌───────────────┴────────────────┐      │
│  │  Commands          │    │  Channel<SessionEvent>         │      │
│  │  (invoke)          │    │  (streaming events)            │      │
│  └────────┬───────────┘    └────────────────────────────────┘      │
├───────────┼────────────────────────────────────────────────────────┤
│           │                Rust Backend (Tauri v2)                 │
├───────────┼────────────────────────────────────────────────────────┤
│           ↓                                                         │
│  ┌──────────────────────────────────────────────────────────┐      │
│  │              Global State (Arc<Mutex<AppState>>)         │      │
│  └──────┬───────────────────────────────────────────┬───────┘      │
│         │                                            │              │
│  ┌──────┴────────┐  ┌───────────────┐  ┌───────────┴────────┐     │
│  │ Session       │  │ Event Bus     │  │ Rules Engine       │     │
│  │ Manager       │  │               │  │                    │     │
│  └───┬───────────┘  └───────┬───────┘  └────────────────────┘     │
│      │                      │                                      │
│  ┌───┴──────────────────────┴────────────────────────────┐        │
│  │        cc-sdk Session Runners (Tokio Tasks)           │        │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐            │        │
│  │  │ Session1 │  │ Session2 │  │ Session3 │  ...       │        │
│  │  │ Stream   │  │ Stream   │  │ Stream   │            │        │
│  │  └──────────┘  └──────────┘  └──────────┘            │        │
│  └─────────────────────────────────────────────────────────┘       │
│                                                                     │
│  ┌─────────────────────────────────────────────────────────┐       │
│  │     Database Layer (tokio-rusqlite)                     │       │
│  │  - Session persistence                                  │       │
│  │  - Approval rules                                       │       │
│  │  - Event history                                        │       │
│  └─────────────────────────────────────────────────────────┘       │
└─────────────────────────────────────────────────────────────────────┘
```

### Component Responsibilities

| Component | Responsibility | Typical Implementation |
|-----------|----------------|------------------------|
| **Session Manager** | Spawns, tracks, and controls multiple cc-sdk sessions; manages concurrent Tokio tasks | Rust struct with HashMap<SessionId, SessionHandle> wrapped in Arc<Mutex> |
| **Event Bus** | Routes streaming events from cc-sdk sessions to frontend via Tauri Channels | MPSC channel multiplexer, one Channel<SessionEvent> per session |
| **Rules Engine** | Evaluates approval conditions against tool invocations; auto-approves when rules match | Conditional evaluation system using pattern matching on tool names/args |
| **Database Layer** | Persists session state, approval rules, and event history | tokio-rusqlite with Connection handle shared via State |
| **Svelte Session Dashboard** | Displays real-time status of all sessions, streams events from backend | Svelte components subscribing to state derived from Tauri event streams |
| **Approval Rules UI** | CRUD interface for managing auto-approval rules | Form-based UI calling Tauri commands to persist rules |

## Recommended Project Structure

```
lulu/
├── src/                          # Svelte 5 frontend
│   ├── lib/
│   │   ├── stores/               # Runes-based state management
│   │   │   ├── sessions.svelte.ts    # Session state ($state + closures)
│   │   │   ├── approvals.svelte.ts   # Approval rules state
│   │   │   └── settings.svelte.ts    # App settings state
│   │   ├── components/
│   │   │   ├── SessionCard.svelte    # Individual session display
│   │   │   ├── EventStream.svelte    # Real-time event feed
│   │   │   ├── ApprovalRules.svelte  # Rules management UI
│   │   │   └── WorkingDirectoryPicker.svelte
│   │   └── tauri/                # Tauri IPC wrappers
│   │       ├── commands.ts       # invoke() wrappers
│   │       └── channels.ts       # Channel streaming setup
│   ├── routes/                   # SvelteKit routes (SPA mode)
│   │   ├── +page.svelte          # Main dashboard
│   │   └── +layout.svelte        # App shell
│   └── app.html
├── src-tauri/                    # Rust backend
│   ├── src/
│   │   ├── main.rs               # Tauri setup, state initialization
│   │   ├── state.rs              # Global AppState definition
│   │   ├── commands/             # Tauri command handlers
│   │   │   ├── mod.rs
│   │   │   ├── session.rs        # Session CRUD commands
│   │   │   └── approval.rs       # Approval rule commands
│   │   ├── session/              # Session orchestration
│   │   │   ├── mod.rs
│   │   │   ├── manager.rs        # SessionManager struct
│   │   │   ├── runner.rs         # cc-sdk session spawning
│   │   │   └── events.rs         # Event types and streaming
│   │   ├── approval/             # Approval system
│   │   │   ├── mod.rs
│   │   │   ├── engine.rs         # Rules evaluation
│   │   │   └── rules.rs          # Rule definitions
│   │   ├── db/                   # Database layer
│   │   │   ├── mod.rs
│   │   │   ├── schema.rs         # SQLite schema
│   │   │   ├── session.rs        # Session persistence
│   │   │   └── approval.rs       # Approval rules persistence
│   │   └── util/                 # Utilities
│   │       └── event_bus.rs      # Event multiplexing
│   ├── Cargo.toml
│   └── tauri.conf.json
├── package.json
└── vite.config.ts
```

### Structure Rationale

- **src/lib/stores/**: Svelte 5 runes require `.svelte.ts` files for reactive exports. Each store exposes getter/setter closures or class-based state for cross-component reactivity.
- **src-tauri/src/**: Modular organization separates concerns. `commands/` handles IPC surface area, `session/` encapsulates cc-sdk logic, `approval/` contains rules engine, `db/` handles persistence.
- **Event flow isolation**: Each session gets its own Tokio task and MPSC sender. Events flow through `event_bus.rs` which multiplexes to frontend Channels.
- **State sharing**: `AppState` in `state.rs` holds shared references (Arc<Mutex>) to SessionManager, Database connection, and Rules Engine, accessible via Tauri's State management.

## Architectural Patterns

### Pattern 1: Multi-Session with Tauri Channels

**What:** Each cc-sdk session runs as an independent Tokio task. Events stream through Tauri's Channel API, providing ordered, high-throughput IPC to the frontend.

**When to use:** When you need real-time streaming of session events (tool invocations, responses, errors) to the UI without polling.

**Trade-offs:**
- ✅ Ordered delivery, low latency, type-safe events
- ✅ Built-in backpressure handling
- ❌ Requires careful lifetime management of Channel handles
- ❌ Each session needs its own Channel (cannot multiplex)

**Example:**
```rust
// src-tauri/src/commands/session.rs
use tauri::{AppHandle, State, ipc::Channel};
use crate::session::events::SessionEvent;

#[tauri::command]
async fn start_session(
    app: AppHandle,
    working_dir: String,
    prompt: String,
    event_channel: Channel<SessionEvent>,
    state: State<'_, Arc<Mutex<AppState>>>
) -> Result<String, String> {
    let session_id = Uuid::new_v4().to_string();

    // Clone what we need for the spawned task
    let app_clone = app.clone();
    let state_clone = state.inner().clone();

    // Spawn session as Tokio task
    tokio::spawn(async move {
        let session = state_clone.lock().await.session_manager.spawn_session(
            session_id.clone(),
            working_dir,
            prompt
        ).await;

        // Stream events through channel
        while let Some(event) = session.stream.next().await {
            event_channel.send(event).unwrap();
        }
    });

    Ok(session_id)
}
```

```typescript
// src/lib/tauri/channels.ts
import { invoke, Channel } from '@tauri-apps/api/core';
import { sessionStore } from '$lib/stores/sessions.svelte';

export async function startSession(workingDir: string, prompt: string) {
  const eventChannel = new Channel<SessionEvent>();

  eventChannel.onmessage = (event) => {
    sessionStore.updateSessionEvent(event);
  };

  const sessionId = await invoke('start_session', {
    workingDir,
    prompt,
    eventChannel
  });

  return sessionId;
}
```

### Pattern 2: Runes-Based Global State (Svelte 5)

**What:** Svelte 5 runes enable reactive state management across components without traditional stores. Use closure-wrapped `$state()` in `.svelte.ts` files for shared reactive state.

**When to use:** For any state that needs to be accessed and mutated from multiple components (sessions list, approval rules, app settings).

**Trade-offs:**
- ✅ Simpler than Svelte 4 stores, more performant
- ✅ Automatic reactivity via Proxy-based tracking
- ❌ Must export closures or objects, not raw `$state()` variables
- ❌ SSR requires care (use context in SvelteKit)

**Example:**
```typescript
// src/lib/stores/sessions.svelte.ts
import { derived, type Readable } from 'svelte/store';

// Internal reactive state
let sessions = $state<Map<string, Session>>(new Map());

// Export getter/setter interface
export const sessionStore = {
  get all() {
    return Array.from(sessions.values());
  },

  get(id: string) {
    return sessions.get(id);
  },

  add(session: Session) {
    sessions.set(session.id, session);
  },

  updateSessionEvent(event: SessionEvent) {
    const session = sessions.get(event.session_id);
    if (session) {
      session.events.push(event);
      sessions.set(event.session_id, session); // Trigger reactivity
    }
  },

  remove(id: string) {
    sessions.delete(id);
  }
};
```

```svelte
<!-- src/lib/components/SessionCard.svelte -->
<script lang="ts">
import { sessionStore } from '$lib/stores/sessions.svelte';

let { sessionId } = $props<{ sessionId: string }>();

// Reactive derived value using rune
let session = $derived(sessionStore.get(sessionId));
</script>

{#if session}
  <div class="session-card">
    <h3>{session.workingDir}</h3>
    <p>Status: {session.status}</p>
    <div class="events">
      {#each session.events as event}
        <div>{event.type}: {event.message}</div>
      {/each}
    </div>
  </div>
{/if}
```

### Pattern 3: Thread-Safe State with Arc<Mutex<T>>

**What:** Tauri's State management automatically wraps managed state in Arc. For mutable state shared across commands and background tasks, wrap in Mutex for interior mutability.

**When to use:** For any mutable state accessed from multiple Tauri commands or background Tokio tasks (SessionManager, Database connection).

**Trade-offs:**
- ✅ Thread-safe, prevents data races
- ✅ Tauri handles Arc automatically via State<T>
- ❌ Lock contention can be a bottleneck (keep critical sections small)
- ❌ Async Mutex needed for async operations holding locks across await points

**Example:**
```rust
// src-tauri/src/state.rs
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::session::manager::SessionManager;
use crate::db::Database;
use crate::approval::engine::RulesEngine;

pub struct AppState {
    pub session_manager: SessionManager,
    pub db: Database,
    pub rules_engine: RulesEngine,
}

// src-tauri/src/main.rs
fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let db = Database::new("sessions.db")?;
            let state = Arc::new(Mutex::new(AppState {
                session_manager: SessionManager::new(),
                db,
                rules_engine: RulesEngine::new(),
            }));

            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            start_session,
            stop_session,
            list_sessions
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

```rust
// src-tauri/src/commands/session.rs
#[tauri::command]
async fn list_sessions(
    state: State<'_, Arc<Mutex<AppState>>>
) -> Result<Vec<SessionInfo>, String> {
    let app_state = state.lock().await;
    Ok(app_state.session_manager.list())
}
```

### Pattern 4: Mediator Event Bus for Session Orchestration

**What:** Central event bus coordinates between session runners and the frontend. Each session publishes events to the bus, which routes them to appropriate frontend Channels based on session ID.

**When to use:** When multiple sessions need to publish events without knowing about frontend consumers, and you need centralized event routing logic.

**Trade-offs:**
- ✅ Decouples session logic from IPC concerns
- ✅ Easier to add cross-session event handling (e.g., logging, persistence)
- ❌ Additional indirection layer
- ❌ Bus becomes bottleneck if event volume is very high

**Example:**
```rust
// src-tauri/src/util/event_bus.rs
use tokio::sync::mpsc;
use std::collections::HashMap;
use crate::session::events::SessionEvent;

pub struct EventBus {
    subscribers: HashMap<String, mpsc::Sender<SessionEvent>>,
}

impl EventBus {
    pub fn new() -> Self {
        Self {
            subscribers: HashMap::new(),
        }
    }

    pub fn subscribe(&mut self, session_id: String, tx: mpsc::Sender<SessionEvent>) {
        self.subscribers.insert(session_id, tx);
    }

    pub fn unsubscribe(&mut self, session_id: &str) {
        self.subscribers.remove(session_id);
    }

    pub async fn publish(&self, event: SessionEvent) {
        if let Some(tx) = self.subscribers.get(&event.session_id) {
            let _ = tx.send(event).await;
        }
    }
}
```

### Pattern 5: Conditional Rules Engine for Auto-Approval

**What:** Pattern-matching based rules engine evaluates tool invocations against configured rules. Rules specify conditions (tool name, file patterns, working directory) and auto-approve when matched.

**When to use:** To reduce user interruptions for trusted operations (e.g., auto-approve `git status` in any directory, auto-approve file reads in `src/`, but always ask for `rm -rf`).

**Trade-offs:**
- ✅ Reduces friction for repetitive approvals
- ✅ Declarative rules are easier to reason about than imperative code
- ❌ Complex conditions can be hard to express
- ❌ Rule conflicts require clear precedence logic

**Example:**
```rust
// src-tauri/src/approval/rules.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalRule {
    pub id: String,
    pub name: String,
    pub conditions: Vec<Condition>,
    pub action: Action,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Condition {
    ToolNameEquals(String),
    ToolNameMatches(String), // Regex
    FilePathMatches(String),  // Glob pattern
    WorkingDirMatches(String),
    ArgumentContains { key: String, value: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Action {
    AutoApprove,
    AutoReject,
    AlwaysAsk,
}

// src-tauri/src/approval/engine.rs
impl RulesEngine {
    pub fn evaluate(&self, invocation: &ToolInvocation) -> Action {
        for rule in &self.rules {
            if rule.conditions.iter().all(|c| c.matches(invocation)) {
                return rule.action.clone();
            }
        }
        Action::AlwaysAsk // Default
    }
}

impl Condition {
    fn matches(&self, invocation: &ToolInvocation) -> bool {
        match self {
            Condition::ToolNameEquals(name) => invocation.tool == *name,
            Condition::FilePathMatches(pattern) => {
                // Use globset crate for glob matching
                glob::Pattern::new(pattern)
                    .unwrap()
                    .matches_path(Path::new(&invocation.file_path))
            },
            // ... other conditions
        }
    }
}
```

## Data Flow

### Request Flow: Starting a New Session

```
User clicks "New Session" in UI
    ↓
SessionDashboard.svelte calls startSession()
    ↓
Tauri IPC: invoke('start_session', { workingDir, prompt, eventChannel })
    ↓
start_session command handler (Rust)
    ↓
SessionManager.spawn_session() creates new cc-sdk session
    ↓
Spawn Tokio task to run session loop
    ↓
Session stream yields events → publish to EventBus
    ↓
EventBus routes to frontend Channel
    ↓
Channel.onmessage callback in frontend
    ↓
sessionStore.updateSessionEvent(event) updates reactive state
    ↓
Svelte components re-render with new event data
```

### State Management Flow

```
AppState (Rust, Arc<Mutex>)
    ↓ (via State guard in commands)
SessionManager, Database, RulesEngine
    ↓ (reads/writes via commands)
Frontend calls Tauri commands
    ↓ (invoke responses)
Svelte stores update ($state reactivity)
    ↓ (reactive subscriptions)
Components re-render automatically
```

### Approval Flow

```
cc-sdk session requests tool invocation
    ↓
SessionRunner intercepts before execution
    ↓
Query RulesEngine.evaluate(invocation)
    ↓
If AutoApprove: execute immediately
    ↓
If AlwaysAsk: send approval request event to frontend
    ↓
Frontend displays approval modal
    ↓
User approves/rejects
    ↓
Frontend calls approve_invocation(session_id, invocation_id, decision)
    ↓
SessionRunner receives decision, proceeds or cancels
```

### Key Data Flows

1. **Session Events (Rust → Frontend):** cc-sdk stream → SessionRunner → EventBus → Tauri Channel → Svelte store → UI components
2. **Commands (Frontend → Rust):** UI action → invoke() → Command handler → AppState mutation → Response → Update Svelte store
3. **Approval Decisions (Bidirectional):** Tool invocation (Rust) → Approval request event (Frontend) → User decision (Frontend) → Approval response command (Rust)

## Integration Points

### External Services

| Service | Integration Pattern | Notes |
|---------|---------------------|-------|
| Claude Agent SDK (cc-sdk) | Direct Rust library usage in Tokio tasks | Each session is independent cc-sdk stream; manage via session::runner module |
| File System | Tauri's fs API + direct Rust std::fs | Sessions target different working directories (same repo, different branches via git worktrees) |
| SQLite | tokio-rusqlite with async connection | Single database file, connection shared via Arc in AppState; schema tracks sessions, rules, events |
| Git | Direct shell commands via Tauri's Command API or git2-rs | Sessions may run `git` commands; worktree management for parallel branch work |

### Internal Boundaries

| Boundary | Communication | Notes |
|----------|---------------|-------|
| Frontend ↔ Rust Backend | Tauri IPC (invoke + Channel) | Commands for CRUD operations, Channels for streaming events; frontend never directly accesses Rust state |
| SessionRunner ↔ RulesEngine | Direct function call | SessionRunner holds reference to RulesEngine; evaluates rules before each tool invocation |
| SessionManager ↔ Database | Direct async function call | SessionManager persists session metadata on create/update; Database writes are async (tokio-rusqlite) |
| EventBus ↔ Channels | MPSC channel per session | EventBus holds map of session_id → mpsc::Sender; Tauri Channel receives from matching mpsc::Receiver |

## Component Build Order

Based on dependency analysis, suggested implementation sequence:

### Phase 1: Foundation (MVP - Core Architecture)
1. **Database Schema & Layer** - No dependencies, needed by everything else
2. **AppState & State Management** - Basic structure for shared state
3. **Tauri IPC Commands (basic CRUD)** - Simple session metadata operations
4. **Svelte Stores (sessions, settings)** - Frontend state management foundation

**Rationale:** Establish data layer and IPC surface area before complex orchestration.

### Phase 2: Session Orchestration
5. **SessionRunner (single session)** - Integrate cc-sdk, run one session in Tokio task
6. **SessionManager (multi-session tracking)** - Add HashMap of runners, lifecycle management
7. **EventBus** - Multiplexing for multiple session streams
8. **Tauri Channel Integration** - Stream events to frontend in real-time

**Rationale:** Build session orchestration incrementally—single session first, then multi-session.

### Phase 3: Approval System
9. **ApprovalRule Types & Storage** - Define rule schema, persist in database
10. **RulesEngine Evaluation** - Conditional matching logic
11. **SessionRunner + RulesEngine Integration** - Intercept tool invocations, evaluate rules
12. **Approval UI & Commands** - Frontend for rule management and approval decisions

**Rationale:** Approval system depends on working session infrastructure to intercept tool invocations.

### Phase 4: Polish & Features
13. **Session Persistence & Resumption** - Store/restore session state across app restarts
14. **Error Handling & Recovery** - Graceful degradation, retry logic, user notifications
15. **Settings UI** - User preferences, model selection, global configuration

**Rationale:** Polish assumes working core functionality.

### Critical Dependencies
- **SessionRunner → cc-sdk**: SessionRunner cannot be built without cc-sdk integration research/experimentation
- **RulesEngine → SessionRunner**: Rules engine needs session lifecycle hooks to intercept tool invocations
- **EventBus → SessionManager**: Event routing requires knowing which sessions exist
- **Frontend Stores → Tauri Channels**: Real-time UI updates depend on streaming IPC

## Anti-Patterns

### Anti-Pattern 1: Blocking the Main Thread

**What people do:** Run cc-sdk sessions synchronously in Tauri command handlers, blocking the main thread until session completes.

**Why it's wrong:** Tauri command handlers run on the main thread. Blocking commands freeze the UI and prevent other IPC calls. Long-running sessions can take minutes.

**Do this instead:** Always spawn cc-sdk sessions as Tokio tasks using `tokio::spawn()`. Commands should return immediately with a session ID, while the task runs in the background. Use Channels to stream progress.

```rust
// ❌ WRONG
#[tauri::command]
fn start_session_blocking(prompt: String) -> String {
    let result = run_cc_sdk_session(prompt); // Blocks for minutes
    result
}

// ✅ CORRECT
#[tauri::command]
async fn start_session(
    prompt: String,
    event_channel: Channel<SessionEvent>
) -> Result<String, String> {
    let session_id = Uuid::new_v4().to_string();
    tokio::spawn(async move {
        // Run session in background
        run_cc_sdk_session(prompt, event_channel).await;
    });
    Ok(session_id) // Return immediately
}
```

### Anti-Pattern 2: Direct State Export in Svelte 5

**What people do:** Export `$state()` variables directly from `.svelte.ts` modules, expecting cross-component reactivity.

**Why it's wrong:** Svelte 5 requires state to be enclosed in a closure or object for cross-module reactivity. Direct exports lose reactivity.

**Do this instead:** Export getter/setter functions, objects with accessors, or class instances. Always wrap `$state()` variables.

```typescript
// ❌ WRONG
export let sessions = $state<Session[]>([]); // Not reactive across modules

// ✅ CORRECT (closure-based)
let sessions = $state<Session[]>([]);
export const sessionStore = {
  get all() { return sessions; },
  add(s: Session) { sessions.push(s); }
};

// ✅ CORRECT (class-based)
class SessionStore {
  sessions = $state<Session[]>([]);
  add(s: Session) { this.sessions.push(s); }
}
export const sessionStore = new SessionStore();
```

### Anti-Pattern 3: Using Standard Mutex with Async/Await

**What people do:** Use `std::sync::Mutex` in async functions that hold the lock across `.await` points.

**Why it's wrong:** `std::sync::Mutex` is not async-aware. Holding a lock across an await point can cause deadlocks or block the async runtime's worker threads.

**Do this instead:** Use `tokio::sync::Mutex` when you need to hold a lock across await points. Use `std::sync::Mutex` only for synchronous critical sections.

```rust
// ❌ WRONG
use std::sync::Mutex;

async fn update_session(state: Arc<Mutex<AppState>>) {
    let mut state = state.lock().unwrap();
    state.sessions.push(session);
    some_async_operation().await; // Deadlock risk!
}

// ✅ CORRECT
use tokio::sync::Mutex;

async fn update_session(state: Arc<Mutex<AppState>>) {
    let mut state = state.lock().await;
    state.sessions.push(session);
    some_async_operation().await; // Safe with async Mutex
}
```

### Anti-Pattern 4: One Channel Per Event Type

**What people do:** Create separate Tauri Channels for different event types (one for tool invocations, one for responses, one for errors).

**Why it's wrong:** Tauri Channels are ordered within a channel, but not across channels. Splitting events across channels loses ordering guarantees. Multiple channels also complicate frontend subscription logic.

**Do this instead:** Use a single Channel per session with an enum for event types. This preserves event ordering and simplifies frontend code.

```rust
// ❌ WRONG
#[tauri::command]
fn start_session(
    tool_channel: Channel<ToolEvent>,
    response_channel: Channel<ResponseEvent>,
    error_channel: Channel<ErrorEvent>
) { /* ... */ }

// ✅ CORRECT
#[derive(Serialize, Clone)]
#[serde(tag = "type", content = "data")]
enum SessionEvent {
    ToolInvocation { tool: String, args: Value },
    ToolResponse { result: String },
    Error { message: String },
}

#[tauri::command]
fn start_session(
    event_channel: Channel<SessionEvent>
) { /* ... */ }
```

### Anti-Pattern 5: No Backpressure Handling

**What people do:** Continuously send events through Channels without checking if the frontend is keeping up. Events accumulate in memory if frontend processes slowly.

**Why it's wrong:** Unbounded event queues can cause memory exhaustion, especially with high-frequency events (e.g., streaming large file reads).

**Do this instead:** Implement backpressure by checking Channel send results or using bounded MPSC channels. Drop or coalesce events when the buffer is full.

```rust
// ❌ WRONG
for event in session.stream {
    event_channel.send(event).unwrap(); // Panics if channel closed, no backpressure
}

// ✅ CORRECT
for event in session.stream {
    match event_channel.send(event) {
        Ok(_) => {},
        Err(e) => {
            eprintln!("Failed to send event, frontend may be overloaded: {}", e);
            break; // Stop streaming if channel is closed or full
        }
    }
}
```

## Scaling Considerations

| Scale | Architecture Adjustments |
|-------|--------------------------|
| 1-5 concurrent sessions | Baseline architecture is sufficient. Single AppState Mutex is fine. Direct MPSC channels from runners to frontend Channels. |
| 5-20 concurrent sessions | Consider lock-free data structures (dashmap) for SessionManager's session registry to reduce contention. May need event coalescing for high-frequency events. |
| 20+ concurrent sessions | Rethink single-process model. Consider splitting session orchestration into separate processes (e.g., one Tauri app coordinates multiple worker processes). Database connection pooling becomes important. |

### Scaling Priorities

1. **First bottleneck:** AppState Mutex contention when many sessions try to update state simultaneously. **Fix:** Use finer-grained locking (separate Mutex per session) or lock-free structures (dashmap crate).

2. **Second bottleneck:** Database writes blocking session runners. **Fix:** Use a write-ahead queue (channel-based batching) to decouple session logic from database I/O.

3. **Third bottleneck:** Frontend UI freezing with too many events. **Fix:** Implement event coalescing (e.g., batch updates, only send diffs) and virtual scrolling for event display.

## Sources

**Tauri v2 Architecture & IPC:**
- [Calling the Frontend from Rust | Tauri](https://v2.tauri.app/develop/calling-frontend/)
- [Inter-Process Communication | Tauri](https://v2.tauri.app/concept/inter-process-communication/)
- [State Management | Tauri](https://v2.tauri.app/develop/state-management/)
- [Long-running backend async tasks in tauri v2 - sneaky crow](https://sneakycrow.dev/blog/2024-05-12-running-async-tasks-in-tauri-v2)
- [Project Structure | Tauri](https://v2.tauri.app/start/project-structure/)

**Claude Agent SDK:**
- [Session Management - Claude API Docs](https://platform.claude.com/docs/en/agent-sdk/sessions)
- [How Claude Code works - Claude Code Docs](https://code.claude.com/docs/en/how-claude-code-works)

**Svelte 5 State Management:**
- [Runes and Global state: do's and don'ts | Mainmatter](https://mainmatter.com/blog/2025/03/11/global-state-in-svelte-5/)
- [Svelte 5 Patterns (Part 1): Simple Shared State, getContext, and Tweened Stores with $runes](https://fubits.dev/notes/svelte-5-patterns-simple-shared-state-getcontext-tweened-stores-with-runes/)

**Rust Async & Concurrency:**
- [tokio_rusqlite - Rust](https://docs.rs/tokio-rusqlite)
- [Using SQLite asynchronously - Rust Forum](https://users.rust-lang.org/t/using-sqlite-asynchronously/39658)

**Event-Driven Architecture:**
- [Event-Driven Architecture - AWS](https://aws.amazon.com/event-driven-architecture/)
- [AI Agent Orchestration Patterns - Azure Architecture Center](https://learn.microsoft.com/en-us/azure/architecture/ai-ml/guide/ai-agent-design-patterns)
- [Design Patterns: Event Bus](https://dzone.com/articles/design-patterns-event-bus)

**Rules Engine Patterns:**
- [Open Source Rust Rules Engine | GoRules](https://gorules.io/open-source/rust-rules-engine)
- [Rules Engine Pattern | DevIQ](https://deviq.com/design-patterns/rules-engine-pattern/)

---
*Architecture research for: Multi-Session AI Agent Orchestrator Desktop App (Tauri v2 + Svelte 5)*
*Researched: 2026-02-14*

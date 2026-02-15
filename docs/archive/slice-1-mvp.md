# Slice 1: MVP — Launch Session + Stream Output

## Goal

Type a prompt in the UI, press Run, and see Claude Code's streamed response rendered in real time. No persistence, no approvals, no session management — just the core loop working end-to-end.

## Context

This is the foundation slice. It scaffolds the Tauri v2 + React 19 project and establishes the streaming bridge between `cc-sdk` (Rust) and the React frontend via Tauri's `Channel<T>` IPC mechanism.

---

## Step 1: Scaffold the Tauri + React project

```bash
cd /Users/tlshortt/workspace
cargo install create-tauri-app
cargo create-tauri-app lulu --template react-ts --manager bun
cd lulu
```

This generates the full project skeleton:
- `src-tauri/` — Rust backend with `Cargo.toml`, `tauri.conf.json`, `src/main.rs`
- `src/` — React frontend with Vite config
- `package.json`, `vite.config.ts`, `tsconfig.json`, `index.html`

### Verify
```bash
cargo tauri dev
```
Should open a window with the default Tauri template.

---

## Step 2: Configure Rust dependencies

**`src-tauri/Cargo.toml`** — Add to `[dependencies]`:

```toml
cc-sdk = "0.5"
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
dirs = "5"
futures = "0.3"
```

### Verify
```bash
cd src-tauri && cargo check
```

---

## Step 3: Define StreamEvent enum

**Create `src-tauri/src/claude/mod.rs`**:

```rust
use serde::Serialize;

#[derive(Clone, Serialize)]
#[serde(tag = "type")]
pub enum StreamEvent {
    Text {
        content: String,
    },
    Thinking {
        content: String,
    },
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    ToolResult {
        tool_use_id: String,
        content: Option<String>,
        is_error: Option<bool>,
    },
    SystemMessage {
        subtype: String,
        data: serde_json::Value,
    },
    SessionResult {
        session_id: String,
        duration_ms: f64,
        duration_api_ms: f64,
        cost_usd: Option<f64>,
        is_error: bool,
        num_turns: u32,
        result: Option<String>,
    },
    Error {
        message: String,
    },
}

pub mod runner;
```

This enum is the contract between Rust and React. Every message from Claude Code gets mapped to one of these variants and serialized as JSON over the Tauri channel.

---

## Step 4: Implement the Claude Code runner

**Create `src-tauri/src/claude/runner.rs`**:

```rust
use futures::StreamExt;
use tauri::ipc::Channel;
use crate::claude::StreamEvent;

pub async fn run_query(
    prompt: String,
    channel: Channel<StreamEvent>,
) -> Result<(), String> {
    let cwd = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("/"));
    let options = cc_sdk::ClaudeCodeOptions::builder()
        .cwd(cwd)
        .build();

    let mut stream = cc_sdk::query(&prompt, Some(options))
        .await
        .map_err(|e| e.to_string())?;

    while let Some(result) = stream.next().await {
        match result {
            Ok(message) => {
                let events = map_message_to_events(message);
                for event in events {
                    let _ = channel.send(event);
                }
            }
            Err(e) => {
                let _ = channel.send(StreamEvent::Error {
                    message: e.to_string(),
                });
                break;
            }
        }
    }

    Ok(())
}

/// Maps a single cc-sdk Message to one or more StreamEvents.
///
/// cc-sdk has 4 Message variants:
///   - Message::Assistant { content: Vec<ContentBlock> }
///   - Message::User { content: String }
///   - Message::System { subtype, data }
///   - Message::Result { session_id, duration_ms, ... }
///
/// ContentBlock has 4 variants:
///   - ContentBlock::Text { text }
///   - ContentBlock::Thinking { thinking, signature }
///   - ContentBlock::ToolUse { id, name, input }
///   - ContentBlock::ToolResult { tool_use_id, content, is_error }
///
/// An Assistant message can contain multiple ContentBlocks, so we
/// flatten them into individual StreamEvents (flat list for MVP).
fn map_message_to_events(message: cc_sdk::Message) -> Vec<StreamEvent> {
    match message {
        cc_sdk::Message::Assistant(msg) => {
            msg.content.into_iter().map(|block| match block {
                cc_sdk::ContentBlock::Text(t) => StreamEvent::Text {
                    content: t.text,
                },
                cc_sdk::ContentBlock::Thinking(t) => StreamEvent::Thinking {
                    content: t.thinking,
                },
                cc_sdk::ContentBlock::ToolUse(t) => StreamEvent::ToolUse {
                    id: t.id,
                    name: t.name,
                    input: t.input,
                },
                cc_sdk::ContentBlock::ToolResult(t) => StreamEvent::ToolResult {
                    tool_use_id: t.tool_use_id,
                    content: t.content.map(|c| c.to_string()),
                    is_error: t.is_error,
                },
            }).collect()
        }
        cc_sdk::Message::User(_) => {
            // User messages are echoes of user input; skip for MVP.
            vec![]
        }
        cc_sdk::Message::System(msg) => {
            vec![StreamEvent::SystemMessage {
                subtype: msg.subtype,
                data: msg.data,
            }]
        }
        cc_sdk::Message::Result(msg) => {
            vec![StreamEvent::SessionResult {
                session_id: msg.session_id,
                duration_ms: msg.duration_ms,
                duration_api_ms: msg.duration_api_ms,
                cost_usd: msg.total_cost_usd,
                is_error: msg.is_error,
                num_turns: msg.num_turns,
                result: msg.result,
            }]
        }
    }
}
```

**Note on cc-sdk API**: The exact struct/enum accessor syntax (e.g., `msg.content`, tuple vs named fields) should be verified against `cargo doc --open` for the pinned `cc-sdk` version. The variant names and field names above are confirmed from docs.rs/cc-sdk v0.5.

**Note on session_id**: The real session_id comes from cc-sdk in two places: (1) the first `Message::System` with `subtype: "init"`, and (2) `Message::Result` at the end. The frontend should extract the session_id from the init system message — this is the ID needed for `resume` and `fork_session` in later slices.

---

## Step 5: Create the Tauri command

**Create `src-tauri/src/commands/mod.rs`**:

```rust
pub mod session;
pub use session::*;
```

**Create `src-tauri/src/commands/session.rs`**:

```rust
use tauri::ipc::Channel;
use crate::claude::StreamEvent;

#[tauri::command]
pub async fn launch_session(
    prompt: String,
    channel: Channel<StreamEvent>,
) -> Result<(), String> {
    // Session ID is NOT generated here — it comes from cc-sdk
    // via the first Message::System { subtype: "init" } event,
    // which is forwarded to the frontend as a SystemMessage StreamEvent.
    // The frontend extracts session_id from that event's `data` field.

    tauri::async_runtime::spawn(async move {
        if let Err(e) = crate::claude::runner::run_query(prompt, channel.clone()).await {
            let _ = channel.send(StreamEvent::Error { message: e });
        }
    });

    Ok(())
}
```

---

## Step 6: Wire up main.rs

**`src-tauri/src/main.rs`**:

```rust
mod claude;
mod commands;

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![commands::launch_session])
        .run(tauri::generate_context!())
        .expect("error running lulu");
}
```

### Verify
```bash
cd src-tauri && cargo check
```

---

## Step 7: Install frontend dependencies

```bash
bun add react-markdown
bun add -D tailwindcss @tailwindcss/vite
```

**Configure Tailwind** — add the Tailwind Vite plugin to `vite.config.ts`:

```typescript
import tailwindcss from "@tailwindcss/vite";

export default defineConfig({
  plugins: [react(), tailwindcss()],
  // ... existing config
});
```

**Add Tailwind import** to `src/index.css` (or `src/styles.css`, whichever the template generates):

```css
@import "tailwindcss";
```

`@tauri-apps/api` comes with the template — no other additions needed.

---

## Step 8: Build the useSession hook

**Create `src/hooks/useSession.ts`**:

```typescript
import { useState, useCallback, useRef } from "react";
import { invoke, Channel } from "@tauri-apps/api/core";

export interface StreamEvent {
  type: "Text" | "Thinking" | "ToolUse" | "ToolResult" | "SystemMessage" | "SessionResult" | "Error";
  // Fields vary by type — use discriminated union
  content?: string | null;
  id?: string;
  name?: string;
  input?: unknown;
  tool_use_id?: string;
  is_error?: boolean | null;
  session_id?: string;
  duration_ms?: number;
  duration_api_ms?: number;
  cost_usd?: number | null;
  num_turns?: number;
  result?: string | null;
  message?: string;
  subtype?: string;
  data?: unknown;
}

export function useSession() {
  const [events, setEvents] = useState<StreamEvent[]>([]);
  const [isRunning, setIsRunning] = useState(false);
  const [sessionId, setSessionId] = useState<string | null>(null);
  const sessionIdRef = useRef<string | null>(null);

  const launch = useCallback(async (prompt: string) => {
    setEvents([]);
    setIsRunning(true);
    setSessionId(null);
    sessionIdRef.current = null;

    const channel = new Channel<StreamEvent>();
    channel.onmessage = (event: StreamEvent) => {
      setEvents((prev) => [...prev, event]);

      // Extract session_id from the init system message (sent by cc-sdk)
      if (
        event.type === "SystemMessage" &&
        event.subtype === "init" &&
        event.data &&
        typeof event.data === "object" &&
        "session_id" in event.data
      ) {
        const id = (event.data as { session_id: string }).session_id;
        sessionIdRef.current = id;
        setSessionId(id);
      }

      if (event.type === "SessionResult" || event.type === "Error") {
        // SessionResult also carries session_id as a fallback
        if (!sessionIdRef.current && event.session_id) {
          setSessionId(event.session_id);
        }
        setIsRunning(false);
      }
    };

    await invoke("launch_session", { prompt, channel });
  }, []);

  return { events, isRunning, sessionId, launch };
}
```

---

## Step 9: Build the PromptInput component

**Create `src/components/PromptInput.tsx`**:

```tsx
import { useState, type FormEvent } from "react";

interface Props {
  onSubmit: (prompt: string) => void;
  isRunning: boolean;
}

export function PromptInput({ onSubmit, isRunning }: Props) {
  const [prompt, setPrompt] = useState("");

  const handleSubmit = (e: FormEvent) => {
    e.preventDefault();
    if (prompt.trim() && !isRunning) {
      onSubmit(prompt.trim());
      setPrompt("");
    }
  };

  return (
    <form onSubmit={handleSubmit} className="flex gap-2 p-4 border-t">
      <textarea
        value={prompt}
        onChange={(e) => setPrompt(e.target.value)}
        placeholder="Ask Claude Code..."
        className="flex-1 resize-none rounded border p-2 text-sm"
        rows={3}
        disabled={isRunning}
        onKeyDown={(e) => {
          if (e.key === "Enter" && e.metaKey) handleSubmit(e);
        }}
      />
      <button
        type="submit"
        disabled={isRunning || !prompt.trim()}
        className="rounded bg-blue-600 px-4 py-2 text-sm text-white disabled:opacity-50"
      >
        {isRunning ? "Running..." : "Run"}
      </button>
    </form>
  );
}
```

---

## Step 10: Build the MessageStream component

**Create `src/components/MessageStream.tsx`**:

```tsx
import ReactMarkdown from "react-markdown";
import type { StreamEvent } from "../hooks/useSession";

interface Props {
  events: StreamEvent[];
}

export function MessageStream({ events }: Props) {
  return (
    <div className="flex-1 overflow-y-auto p-4 space-y-3">
      {events.map((event, i) => (
        <EventBlock key={i} event={event} />
      ))}
    </div>
  );
}

function EventBlock({ event }: { event: StreamEvent }) {
  switch (event.type) {
    case "Text":
      return (
        <div className="prose prose-sm max-w-none">
          <ReactMarkdown>{event.content ?? ""}</ReactMarkdown>
        </div>
      );
    case "Thinking":
      return (
        <details className="text-xs text-gray-500">
          <summary>Thinking...</summary>
          <pre className="whitespace-pre-wrap">{event.content}</pre>
        </details>
      );
    case "ToolUse":
      return (
        <div className="rounded bg-gray-100 p-2 text-sm font-mono">
          <span className="font-bold text-blue-700">{event.name}</span>
          <pre className="mt-1 text-xs whitespace-pre-wrap">
            {JSON.stringify(event.input, null, 2)}
          </pre>
        </div>
      );
    case "ToolResult":
      return (
        <div className={`rounded p-2 text-sm font-mono ${event.is_error ? "bg-red-50" : "bg-green-50"}`}>
          <pre className="whitespace-pre-wrap text-xs">{event.content}</pre>
        </div>
      );
    case "SessionResult":
      return (
        <div className="rounded bg-gray-50 p-3 text-sm border">
          <span className="font-medium">Session complete</span>
          <span className="ml-3 text-gray-500">
            {event.num_turns} turns · {((event.duration_ms ?? 0) / 1000).toFixed(1)}s
            {event.cost_usd != null && ` · $${event.cost_usd.toFixed(4)}`}
          </span>
        </div>
      );
    case "Error":
      return (
        <div className="rounded bg-red-100 p-2 text-sm text-red-800">
          Error: {event.message}
        </div>
      );
    default:
      return null;
  }
}
```

---

## Step 11: Wire up the Home page

**Replace `src/App.tsx`**:

```tsx
import { useSession } from "./hooks/useSession";
import { PromptInput } from "./components/PromptInput";
import { MessageStream } from "./components/MessageStream";

export default function App() {
  const { events, isRunning, launch } = useSession();

  return (
    <div className="flex flex-col h-screen">
      <header className="border-b px-4 py-2 text-sm font-semibold">
        lulu
      </header>
      <MessageStream events={events} />
      <PromptInput onSubmit={launch} isRunning={isRunning} />
    </div>
  );
}
```

---

## Step 12: Verify end-to-end

```bash
cargo tauri dev
```

1. Window opens with "lulu" header, prompt input at bottom
2. Type "say hello" → press Run (or Cmd+Enter)
3. See streamed text appear in the message area
4. Completion summary shows at the end
5. If Claude Code is not installed, an Error event should display

---

## Files Created/Modified

| File | Action |
|------|--------|
| `src-tauri/Cargo.toml` | Modified — add dependencies (`cc-sdk`, `tokio`, `serde`, `serde_json`, `dirs`, `futures`) |
| `src-tauri/src/main.rs` | Modified — module declarations + command registration |
| `src-tauri/src/claude/mod.rs` | **New** — StreamEvent enum |
| `src-tauri/src/claude/runner.rs` | **New** — cc-sdk wrapper with message mapping |
| `src-tauri/src/commands/mod.rs` | **New** — re-exports from submodules |
| `src-tauri/src/commands/session.rs` | **New** — launch_session command |
| `vite.config.ts` | Modified — add Tailwind plugin |
| `src/index.css` | Modified — add `@import "tailwindcss"` |
| `src/hooks/useSession.ts` | **New** — session state + channel + session_id extraction |
| `src/components/PromptInput.tsx` | **New** — input component |
| `src/components/MessageStream.tsx` | **New** — event renderer |
| `src/App.tsx` | Modified — wire up components |

## Decisions Made

| Decision | Choice | Rationale |
|----------|--------|-----------|
| CLI binary discovery | Auto-discover via PATH | Simplest approach; fail with clear error if `claude` is missing |
| Working directory | Hardcode to `$HOME` | MVP default; configurable in Slice 5 settings |
| Event grouping | Flat list | Good enough for MVP; group by message in later slices |
| CSS strategy | Tailwind CSS | Matches utility classes already in component code |
| Module layout | Group by domain | `commands/session.rs` with `mod.rs` re-exports; scales to Slices 2-4 |
| Session ID source | cc-sdk (not synthetic) | Real session_id from `Message::System { subtype: "init" }` enables resume/fork in Slice 4 |

## Open Questions to Resolve During Implementation

1. **cc-sdk accessor syntax**: The `map_message_to_events` function assumes named-struct access (e.g., `msg.content`, `t.text`). Verify with `cargo doc --open` whether cc-sdk uses tuple variants or named fields.
2. **Tauri Channel + Clone**: Verify that `Channel<T>` implements `Clone` (needed to send error after spawn). If not, wrap in `Arc`.
3. **ContentValue stringification**: `ToolResult.content` is `Option<ContentValue>` in cc-sdk. Verify how `ContentValue` serializes (`.to_string()` may not be the right approach).
4. **System init message shape**: Confirm the exact `data` payload of the `Message::System { subtype: "init" }` message to ensure correct session_id extraction in the frontend.

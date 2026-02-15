# Slice 3: Approval Workflow

## Goal

When Claude Code attempts to use a tool, the user is prompted with an approval dialog. They can approve (continue), deny (stop), or set up auto-approve rules to bypass future prompts for specific tools.

## Prerequisites

- Slice 2 complete (sessions persist to SQLite)

---

## Step 1: Add database migration for approvals

**`src-tauri/src/store/migrations.rs`** — Add Migration 002:

```sql
CREATE TABLE IF NOT EXISTS approvals (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES sessions(id),
    tool_name TEXT NOT NULL,
    tool_input TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    auto_approved INTEGER NOT NULL DEFAULT 0,
    resolved_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS auto_approve_rules (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    tool_pattern TEXT NOT NULL,
    action TEXT NOT NULL DEFAULT 'approve',
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
```

Add corresponding query helpers to `store/models.rs`:
- `insert_approval(conn, id, session_id, tool_name, tool_input, auto_approved) -> Result<()>`
- `update_approval_status(conn, id, status) -> Result<()>`
- `list_pending_approvals(conn, session_id?) -> Result<Vec<Approval>>`
- `list_auto_approve_rules(conn) -> Result<Vec<AutoApproveRule>>`
- `insert_auto_approve_rule(conn, tool_pattern, action) -> Result<i64>`
- `delete_auto_approve_rule(conn, id) -> Result<()>`

---

## Step 2: Create the ApprovalManager

**Create `src-tauri/src/approval/mod.rs`**:

```rust
pub mod rules;

use std::sync::Arc;
use dashmap::DashMap;
use tokio::sync::oneshot;
use serde::{Serialize, Deserialize};

#[derive(Clone, Serialize, Deserialize)]
pub enum ApprovalDecision {
    Approved,
    Denied { reason: Option<String> },
}

pub struct PendingApproval {
    pub id: String,
    pub session_id: String,
    pub tool_name: String,
    pub tool_input: serde_json::Value,
    pub sender: oneshot::Sender<ApprovalDecision>,
}

pub struct ApprovalManager {
    pending: Arc<DashMap<String, PendingApproval>>,
}

impl ApprovalManager {
    pub fn new() -> Self {
        Self {
            pending: Arc::new(DashMap::new()),
        }
    }

    /// Create a pending approval and return a receiver to await the decision.
    pub fn create_pending(
        &self,
        id: String,
        session_id: String,
        tool_name: String,
        tool_input: serde_json::Value,
    ) -> oneshot::Receiver<ApprovalDecision> {
        let (tx, rx) = oneshot::channel();
        self.pending.insert(id.clone(), PendingApproval {
            id,
            session_id,
            tool_name,
            tool_input,
            sender: tx,
        });
        rx
    }

    /// Resolve a pending approval. Returns Err if not found.
    pub fn resolve(&self, id: &str, decision: ApprovalDecision) -> Result<(), String> {
        let (_, pending) = self.pending
            .remove(id)
            .ok_or_else(|| format!("Approval {} not found", id))?;
        pending.sender.send(decision).map_err(|_| "Receiver dropped".to_string())
    }

    /// List all pending approval IDs (for UI display).
    pub fn list_pending(&self) -> Vec<ApprovalInfo> {
        self.pending.iter().map(|entry| ApprovalInfo {
            id: entry.id.clone(),
            session_id: entry.session_id.clone(),
            tool_name: entry.tool_name.clone(),
            tool_input: entry.tool_input.clone(),
        }).collect()
    }
}

#[derive(Serialize)]
pub struct ApprovalInfo {
    pub id: String,
    pub session_id: String,
    pub tool_name: String,
    pub tool_input: serde_json::Value,
}
```

Add `dashmap = "6"` to `Cargo.toml`.

---

## Step 3: Create auto-approve rules

**Create `src-tauri/src/approval/rules.rs`**:

```rust
use crate::store::models::AutoApproveRule;

/// Check if a tool call matches any auto-approve rule.
/// Returns Some(action) if matched, None if no match.
pub fn check_auto_approve(tool_name: &str, rules: &[AutoApproveRule]) -> Option<String> {
    for rule in rules {
        if matches_pattern(&rule.tool_pattern, tool_name) {
            return Some(rule.action.clone());
        }
    }
    None
}

fn matches_pattern(pattern: &str, tool_name: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    if pattern.ends_with('*') {
        let prefix = &pattern[..pattern.len() - 1];
        return tool_name.starts_with(prefix);
    }
    pattern == tool_name
}
```

Supported patterns:
- `"Read"` — exact match
- `"Read*"` — prefix match (matches `Read`, `ReadFile`, etc.)
- `"*"` — match all tools

---

## Step 4: Integrate approvals into the runner

**`src-tauri/src/claude/runner.rs`** — This is the critical integration point.

### Investigation needed first:

Before implementing, check `cc-sdk` for:
1. Does it support `PreToolUse` hooks? → If yes, use hooks for clean interception
2. Does it expose raw events before tool execution? → If yes, intercept at stream level
3. Does `ClaudeCodeOptions` have a `permission_mode` or `permission_prompt_tool` field?

### Approach A: cc-sdk hooks (preferred)

If `cc-sdk` supports hooks:

```rust
// Configure cc-sdk to call our approval function before tool use
let options = ClaudeCodeOptions::builder()
    .pre_tool_use_hook(|tool_name, tool_input| async {
        // Check auto-approve rules
        // If no match, create pending approval and await decision
        // Return allow/deny
    })
    .build();
```

### Approach B: Stream-level interception (fallback)

If hooks are not available, we intercept at the stream level:

```rust
pub async fn run_query(
    session_id: String,
    prompt: String,
    db: Arc<Database>,
    approval_mgr: Arc<ApprovalManager>,
    app_handle: tauri::AppHandle,
    channel: Channel<StreamEvent>,
) -> Result<(), String> {
    // ... setup ...

    while let Some(result) = stream.next().await {
        match result {
            Ok(message) => {
                let event = map_message_to_event(message);

                // Intercept tool use events
                if let StreamEvent::ToolUse { ref id, ref name, ref input } = event {
                    let decision = handle_tool_approval(
                        &session_id, id, name, input,
                        &db, &approval_mgr, &app_handle,
                    ).await;

                    match decision {
                        ApprovalDecision::Denied { .. } => {
                            // Send denial event to UI, session continues
                            // (Claude Code handles the denial internally)
                        }
                        ApprovalDecision::Approved => {
                            // Tool proceeds normally
                        }
                    }
                }

                // Persist and forward as before
                persist_and_send(&session_id, &event, &db, &channel);
            }
            // ... error handling ...
        }
    }
    Ok(())
}

async fn handle_tool_approval(
    session_id: &str,
    tool_id: &str,
    tool_name: &str,
    tool_input: &serde_json::Value,
    db: &Database,
    approval_mgr: &ApprovalManager,
    app_handle: &tauri::AppHandle,
) -> ApprovalDecision {
    let approval_id = uuid::Uuid::new_v4().to_string();

    // Check auto-approve rules
    let rules = {
        let conn = db.conn();
        models::list_auto_approve_rules(&conn).unwrap_or_default()
    };

    if let Some(action) = rules::check_auto_approve(tool_name, &rules) {
        // Persist as auto-approved
        let conn = db.conn();
        let _ = models::insert_approval(&conn, &approval_id, session_id, tool_name,
            &serde_json::to_string(tool_input).unwrap(), true);
        let _ = models::update_approval_status(&conn, &approval_id, &action);

        return if action == "approve" {
            ApprovalDecision::Approved
        } else {
            ApprovalDecision::Denied { reason: Some("Auto-denied by rule".into()) }
        };
    }

    // Persist as pending
    {
        let conn = db.conn();
        let _ = models::insert_approval(&conn, &approval_id, session_id, tool_name,
            &serde_json::to_string(tool_input).unwrap(), false);
    }

    // Emit Tauri event to trigger the approval dialog
    let _ = app_handle.emit("approval-requested", ApprovalInfo {
        id: approval_id.clone(),
        session_id: session_id.to_string(),
        tool_name: tool_name.to_string(),
        tool_input: tool_input.clone(),
    });

    // Block until user decides
    let rx = approval_mgr.create_pending(
        approval_id.clone(),
        session_id.to_string(),
        tool_name.to_string(),
        tool_input.clone(),
    );

    match rx.await {
        Ok(decision) => {
            // Persist resolution
            let status = match &decision {
                ApprovalDecision::Approved => "approved",
                ApprovalDecision::Denied { .. } => "denied",
            };
            let conn = db.conn();
            let _ = models::update_approval_status(&conn, &approval_id, status);
            decision
        }
        Err(_) => {
            // Channel dropped — treat as denied
            ApprovalDecision::Denied { reason: Some("Approval cancelled".into()) }
        }
    }
}
```

**Key insight**: The runner blocks on `rx.await` until the user clicks Approve/Deny in the UI, which calls the `resolve_approval` Tauri command.

---

## Step 5: Add Tauri commands for approvals

**Create `src-tauri/src/commands/approval_cmds.rs`**:

```rust
use crate::approval::{ApprovalManager, ApprovalDecision, ApprovalInfo};
use crate::store::{Database, models};

#[tauri::command]
pub async fn resolve_approval(
    approval_id: String,
    decision: String,  // "approved" | "denied"
    reason: Option<String>,
    approval_mgr: tauri::State<'_, ApprovalManager>,
) -> Result<(), String> {
    let decision = match decision.as_str() {
        "approved" => ApprovalDecision::Approved,
        "denied" => ApprovalDecision::Denied { reason },
        _ => return Err("Invalid decision".into()),
    };
    approval_mgr.resolve(&approval_id, decision)
}

#[tauri::command]
pub async fn list_pending_approvals(
    approval_mgr: tauri::State<'_, ApprovalManager>,
) -> Result<Vec<ApprovalInfo>, String> {
    Ok(approval_mgr.list_pending())
}

#[tauri::command]
pub async fn list_auto_approve_rules(
    db: tauri::State<'_, Database>,
) -> Result<Vec<models::AutoApproveRule>, String> {
    let conn = db.conn();
    models::list_auto_approve_rules(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn add_auto_approve_rule(
    tool_pattern: String,
    action: String,
    db: tauri::State<'_, Database>,
) -> Result<i64, String> {
    let conn = db.conn();
    models::insert_auto_approve_rule(&conn, &tool_pattern, &action).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_auto_approve_rule(
    rule_id: i64,
    db: tauri::State<'_, Database>,
) -> Result<(), String> {
    let conn = db.conn();
    models::delete_auto_approve_rule(&conn, rule_id).map_err(|e| e.to_string())
}
```

Register all in `main.rs` and manage `ApprovalManager` as Tauri state.

---

## Step 6: Build the ApprovalDialog component

**Create `src/components/ApprovalDialog.tsx`**:

```tsx
import { invoke } from "@tauri-apps/api/core";

interface ApprovalRequest {
  id: string;
  session_id: string;
  tool_name: string;
  tool_input: unknown;
}

interface Props {
  approval: ApprovalRequest;
  onResolved: () => void;
}

export function ApprovalDialog({ approval, onResolved }: Props) {
  const handleDecision = async (decision: "approved" | "denied") => {
    await invoke("resolve_approval", {
      approvalId: approval.id,
      decision,
      reason: null,
    });
    onResolved();
  };

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
      <div className="bg-white rounded-lg shadow-xl max-w-lg w-full mx-4">
        <div className="p-4 border-b">
          <h2 className="text-lg font-semibold">Tool Approval Required</h2>
          <p className="text-sm text-gray-500 mt-1">
            Claude wants to use <code className="bg-gray-100 px-1 rounded">{approval.tool_name}</code>
          </p>
        </div>
        <div className="p-4 max-h-64 overflow-y-auto">
          <pre className="text-xs bg-gray-50 p-3 rounded whitespace-pre-wrap">
            {JSON.stringify(approval.tool_input, null, 2)}
          </pre>
        </div>
        <div className="p-4 border-t flex justify-end gap-2">
          <button
            onClick={() => handleDecision("denied")}
            className="px-4 py-2 text-sm rounded border hover:bg-gray-50"
          >
            Deny
          </button>
          <button
            onClick={() => handleDecision("approved")}
            className="px-4 py-2 text-sm rounded bg-blue-600 text-white hover:bg-blue-700"
          >
            Approve
          </button>
        </div>
      </div>
    </div>
  );
}
```

---

## Step 7: Build the useApprovals hook

**Create `src/hooks/useApprovals.ts`**:

```typescript
import { useState, useEffect, useCallback } from "react";
import { listen } from "@tauri-apps/api/event";

interface ApprovalRequest {
  id: string;
  session_id: string;
  tool_name: string;
  tool_input: unknown;
}

export function useApprovals() {
  const [pendingApprovals, setPendingApprovals] = useState<ApprovalRequest[]>([]);

  useEffect(() => {
    const unlisten = listen<ApprovalRequest>("approval-requested", (event) => {
      setPendingApprovals((prev) => [...prev, event.payload]);
    });

    return () => { unlisten.then((fn) => fn()); };
  }, []);

  const removeApproval = useCallback((id: string) => {
    setPendingApprovals((prev) => prev.filter((a) => a.id !== id));
  }, []);

  return {
    currentApproval: pendingApprovals[0] ?? null,
    removeApproval,
  };
}
```

---

## Step 8: Wire approvals into the App

**`src/App.tsx`** — Add:

```tsx
import { useApprovals } from "./hooks/useApprovals";
import { ApprovalDialog } from "./components/ApprovalDialog";

// Inside App component:
const { currentApproval, removeApproval } = useApprovals();

// In the render:
{currentApproval && (
  <ApprovalDialog
    approval={currentApproval}
    onResolved={() => removeApproval(currentApproval.id)}
  />
)}
```

---

## Step 9: Verify

1. Run a session with a prompt that triggers tool use (e.g., "read the file ./Cargo.toml")
2. Approval dialog appears with tool name and input
3. Click "Approve" → session continues, tool result appears
4. Click "Deny" → tool is blocked
5. Add an auto-approve rule for "Read" → next Read tool call skips the dialog
6. Approval history visible in session detail

---

## Files Created/Modified

| File | Action |
|------|--------|
| `src-tauri/Cargo.toml` | Modified — add dashmap |
| `src-tauri/src/main.rs` | Modified — manage ApprovalManager, register commands |
| `src-tauri/src/store/migrations.rs` | Modified — add migration 002 |
| `src-tauri/src/store/models.rs` | Modified — add approval types + queries |
| `src-tauri/src/approval/mod.rs` | **New** — ApprovalManager |
| `src-tauri/src/approval/rules.rs` | **New** — Auto-approve matching |
| `src-tauri/src/claude/runner.rs` | Modified — intercept tool calls |
| `src-tauri/src/commands/approval_cmds.rs` | **New** — Approval commands |
| `src/components/ApprovalDialog.tsx` | **New** |
| `src/hooks/useApprovals.ts` | **New** |
| `src/App.tsx` | Modified — add approval dialog |

## Risk: cc-sdk Integration

The biggest unknown is how `cc-sdk` handles tool permissions. Three scenarios:

1. **cc-sdk has hooks** → cleanest approach, use them
2. **cc-sdk exposes events before execution** → intercept at stream level (Approach B above)
3. **cc-sdk handles permissions internally** → may need to use `permission_mode: "plan"` or `allowed_tools: []` to force all tools through our approval, then manually invoke them

Investigate this at the start of Slice 3 before writing code.

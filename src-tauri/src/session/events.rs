use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SessionEvent {
    pub session_id: String,
    pub seq: u64,
    pub timestamp: String,
    #[serde(flatten)]
    pub payload: SessionEventPayload,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum SessionEventPayload {
    Message { content: String },
    ToolCall { tool_name: String, args: Value },
    ToolResult { tool_name: String, result: Value },
    Status { status: String },
    Error { message: String },
}

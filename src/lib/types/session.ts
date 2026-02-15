export type SessionEventType =
  | "message"
  | "tool_call"
  | "tool_result"
  | "thinking"
  | "status"
  | "error";

export interface SessionEventBase {
  session_id: string;
  seq: number;
  timestamp: string;
}

export interface MessageEventData extends SessionEventBase {
  content: string;
  complete: boolean;
}

export interface ToolCallEventData extends SessionEventBase {
  call_id?: string;
  tool_name: string;
  args: unknown;
}

export interface ToolResultEventData extends SessionEventBase {
  call_id?: string;
  tool_name?: string;
  result: unknown;
}

export interface ThinkingEventData extends SessionEventBase {
  content: string;
}

export interface StatusEventData extends SessionEventBase {
  status: "running" | "completed" | "failed" | "killed" | string;
  message?: string;
}

export interface ErrorEventData extends SessionEventBase {
  error: string;
}

export type SessionEvent =
  | { type: "message"; data: MessageEventData }
  | { type: "tool_call"; data: ToolCallEventData }
  | { type: "tool_result"; data: ToolResultEventData }
  | { type: "thinking"; data: ThinkingEventData }
  | { type: "status"; data: StatusEventData }
  | { type: "error"; data: ErrorEventData };

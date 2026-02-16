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
  status:
    | "running"
    | "completed"
    | "failed"
    | "killed"
    | "interrupting"
    | "interrupted"
    | "resuming"
    | string;
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

export interface SessionDebugEvent {
  session_id: string;
  kind: "spawn" | "stderr";
  timestamp: string;
  cli_path?: string;
  args?: string[];
  working_dir?: string;
  message?: string;
}

export type DashboardStatus =
  | "Starting"
  | "Running"
  | "Completed"
  | "Interrupted"
  | "Failed";

export type SessionOperationStatus = "interrupting" | "resuming";

export interface DashboardSessionRow {
  id: string;
  name: string;
  status: DashboardStatus;
  recentActivity: string;
  failureReason?: string;
  createdAt: string;
}

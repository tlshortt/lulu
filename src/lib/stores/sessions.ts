import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { derived, get, writable } from "svelte/store";
import type {
  DashboardSessionRow,
  DashboardSortMode,
  DashboardStatus,
  SessionDebugEvent,
  SessionEvent,
  SessionOperationStatus,
} from "$lib/types/session";

export interface Session {
  id: string;
  name: string;
  status: string;
  working_dir: string;
  created_at: string;
  updated_at: string;
  last_activity_at?: string | null;
  failure_reason?: string | null;
  restored?: boolean;
  restored_at?: string | null;
  recovery_hint?: boolean;
}

export const sessions = writable<Session[]>([]);
export const activeSessionId = writable<string | null>(null);
export const selectedSessionId = activeSessionId;
export const dashboardSelectedSessionId = writable<string | null>(null);
export const initialSessionsHydrated = writable(false);
export const initialSessionsLoadError = writable<string | null>(null);
export const sessionEvents = writable<Record<string, SessionEvent[]>>({});
export interface SessionDebugState {
  cliPath?: string;
  args?: string[];
  workingDir?: string;
  stderrTail: string[];
  updatedAt: string;
}

export const sessionDebug = writable<Record<string, SessionDebugState>>({});
export const sessionOperations = writable<
  Record<string, SessionOperationStatus>
>({});
export const sessionErrors = writable<Record<string, string>>({});

export interface SpawnRuntimeDiagnostics {
  started_at: string;
  finished_at: string;
  outcome: "success" | "spawn_failed";
  session_id: string | null;
  spawn_duration_ms: number;
  refresh_duration_ms: number | null;
  total_duration_ms: number;
  refresh_status: "succeeded" | "failed" | "skipped";
  sessions_count: number;
  has_session_in_store: boolean;
  active_session_id: string | null;
  dashboard_selected_session_id: string | null;
  error_message: string | null;
}

export const spawnRuntimeDiagnostics = writable<SpawnRuntimeDiagnostics | null>(
  null,
);

type MessageBuffers = Record<string, string>;
interface DashboardSessionProjection {
  id: string;
  name: string;
  status: string;
  created_at: string;
  last_activity_at?: string | null;
  failure_reason?: string | null;
  restored?: boolean;
  restored_at?: string | null;
  recovery_hint?: boolean;
}

interface StoredSessionHistoryEvent {
  id: string;
  session_id: string;
  run_id: string;
  seq: number;
  event_type: string;
  payload_json: {
    type?: string;
    data?: Record<string, unknown>;
  };
  timestamp: string;
}

interface LegacySessionMessage {
  id: string;
  session_id: string;
  role: string;
  content: string;
  timestamp: string;
}

const messageBuffers: MessageBuffers = {};
const loadedSessionHistory = new Set<string>();
const pendingSpawnSessionIds = new Set<string>();
let listenerInitialized = false;
let listenerInitializing = false;
let sequenceCounter = 0;
const canonicalSessionEventIds = new Set<string>();
const TERMINAL_STATUSES = new Set(["completed", "failed", "killed"]);
const FAILURE_STATUSES = new Set([
  "failed",
  "killed",
  "error",
  "cancelled",
  "canceled",
  "crashed",
]);
const dashboardNow = writable(Date.now());
const LIST_SESSIONS_TIMEOUT_MS = 1500;
const SPAWN_SESSION_TIMEOUT_MS = 15000;
const DASHBOARD_SORT_KEY = "lulu:dashboard-sort-mode";
const STARTUP_SORT_MODE: DashboardSortMode = "active-first-then-recent";

if (typeof window !== "undefined") {
  setInterval(() => {
    dashboardNow.set(Date.now());
  }, 1000);
}

const canUseStorage = () => typeof window !== "undefined";

const loadBoolean = (key: string, fallback: boolean) => {
  if (!canUseStorage()) {
    return fallback;
  }

  const value = window.localStorage.getItem(key);
  if (value === null) {
    return fallback;
  }

  return value === "true";
};

const loadString = (key: string, fallback = "") => {
  if (!canUseStorage()) {
    return fallback;
  }

  return window.localStorage.getItem(key) ?? fallback;
};

const isDashboardSortMode = (value: string): value is DashboardSortMode =>
  value === "active-first-then-recent" ||
  value === "recent" ||
  value === "oldest";

const loadDashboardSortPreference = (): DashboardSortMode => {
  const value = loadString(DASHBOARD_SORT_KEY, STARTUP_SORT_MODE);
  return isDashboardSortMode(value) ? value : STARTUP_SORT_MODE;
};

export const showThinking = writable<boolean>(
  loadBoolean("lulu:show-thinking", false),
);

export const cliPathOverride = writable<string>(
  loadString("lulu:cli-path-override", ""),
);
export const dashboardSortPreference = writable<DashboardSortMode>(
  loadDashboardSortPreference(),
);
export const dashboardSortMode = writable<DashboardSortMode>(STARTUP_SORT_MODE);

showThinking.subscribe((value) => {
  if (!canUseStorage()) {
    return;
  }

  window.localStorage.setItem("lulu:show-thinking", String(value));
});

cliPathOverride.subscribe((value) => {
  if (!canUseStorage()) {
    return;
  }

  window.localStorage.setItem("lulu:cli-path-override", value);
});

dashboardSortPreference.subscribe((value) => {
  if (!canUseStorage()) {
    return;
  }

  window.localStorage.setItem(DASHBOARD_SORT_KEY, value);
});

export const setDashboardSortMode = (mode: DashboardSortMode) => {
  dashboardSortMode.set(mode);
  dashboardSortPreference.set(mode);
};

const nextSeq = () => {
  sequenceCounter += 1;
  return sequenceCounter;
};

const createTimestamp = () => new Date().toISOString();

const normalizeStatus = (status: string) => {
  const normalized = status.toLowerCase();

  if (normalized === "complete" || normalized === "done") {
    return "completed";
  }

  if (normalized === "error") {
    return "failed";
  }

  return normalized;
};

const toDashboardStatus = (status: string): DashboardStatus => {
  const normalized = normalizeStatus(status);

  if (normalized === "interrupted") {
    return "Interrupted";
  }

  if (normalized === "running") {
    return "Running";
  }

  if (normalized === "completed") {
    return "Completed";
  }

  if (FAILURE_STATUSES.has(normalized)) {
    return "Failed";
  }

  return "Starting";
};

const toEpoch = (iso: string) => {
  const value = Date.parse(iso);
  return Number.isFinite(value) ? value : 0;
};

const compactAgeLabel = (timestamp: string, now: number) => {
  const ageMs = Math.max(0, now - toEpoch(timestamp));
  const ageSeconds = Math.floor(ageMs / 1000);

  if (ageSeconds < 60) {
    return `${ageSeconds}s`;
  }

  const ageMinutes = Math.floor(ageSeconds / 60);
  if (ageMinutes < 60) {
    return `${ageMinutes}m`;
  }

  const ageHours = Math.floor(ageMinutes / 60);
  if (ageHours < 24) {
    return `${ageHours}h`;
  }

  const ageDays = Math.floor(ageHours / 24);
  return `${ageDays}d`;
};

const toSingleLine = (value: string) => value.replace(/\s+/g, " ").trim();

const byCreatedAtDesc = (left: Session, right: Session) => {
  const dateDelta = toEpoch(right.created_at) - toEpoch(left.created_at);
  if (dateDelta !== 0) {
    return dateDelta;
  }

  return right.id.localeCompare(left.id);
};

const isActiveSortStatus = (status: string) => {
  const normalized = normalizeStatus(status);
  return (
    normalized === "starting" ||
    normalized === "running" ||
    normalized === "interrupting" ||
    normalized === "resuming"
  );
};

const sortSessions = (items: Session[], mode: DashboardSortMode) => {
  if (mode === "oldest") {
    return [...items].sort((left, right) => -byCreatedAtDesc(left, right));
  }

  if (mode === "recent") {
    return [...items].sort(byCreatedAtDesc);
  }

  return [...items].sort((left, right) => {
    const leftActive = isActiveSortStatus(left.status);
    const rightActive = isActiveSortStatus(right.status);
    if (leftActive !== rightActive) {
      return leftActive ? -1 : 1;
    }

    return byCreatedAtDesc(left, right);
  });
};

const extractFailureReason = (events: SessionEvent[]) => {
  for (let index = events.length - 1; index >= 0; index -= 1) {
    const event = events[index];

    if (event.type === "error") {
      const reason = toSingleLine(event.data.error);
      if (reason.length > 0) {
        return reason;
      }
    }

    if (
      event.type === "status" &&
      FAILURE_STATUSES.has(normalizeStatus(event.data.status))
    ) {
      const reason = toSingleLine(event.data.message ?? "");
      if (reason.length > 0) {
        return reason;
      }
    }
  }

  return undefined;
};

export const dashboardRows = derived(
  [sessions, sessionEvents, dashboardNow, dashboardSortMode],
  ([
    $sessions,
    $sessionEvents,
    $dashboardNow,
    $dashboardSortMode,
  ]): DashboardSessionRow[] =>
    sortSessions($sessions, $dashboardSortMode).map((session) => {
      const status = toDashboardStatus(session.status);
      const events = $sessionEvents[session.id] ?? [];

      return {
        id: session.id,
        name: session.name,
        status,
        recentActivity: compactAgeLabel(session.updated_at, $dashboardNow),
        failureReason:
          status === "Failed"
            ? (extractFailureReason(events) ??
              (session.failure_reason
                ? toSingleLine(session.failure_reason)
                : undefined))
            : undefined,
        createdAt: session.created_at,
        restored: session.restored ?? false,
        recoveryHint: (session.recovery_hint ?? false) && status === "Running",
      } satisfies DashboardSessionRow;
    }),
);

export function refreshDashboardNowForTests(now: number) {
  dashboardNow.set(now);
}

const isTerminalStatus = (status: string) =>
  TERMINAL_STATUSES.has(normalizeStatus(status));

const delay = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

const invokeWithTimeout = async <T>(
  command: string,
  args: Record<string, unknown> | undefined,
  timeoutMs: number,
): Promise<T> => {
  let timeoutId: ReturnType<typeof setTimeout> | undefined;

  try {
    const timeout = new Promise<never>((_, reject) => {
      timeoutId = setTimeout(() => {
        reject(new Error(`${command} timed out after ${timeoutMs}ms`));
      }, timeoutMs);
    });

    return await Promise.race([invoke<T>(command, args), timeout]);
  } finally {
    if (timeoutId) {
      clearTimeout(timeoutId);
    }
  }
};

const toErrorMessage = (value: unknown, fallback: string) => {
  if (typeof value === "string" && value.trim().length > 0) {
    return value;
  }

  if (value instanceof Error && value.message.trim().length > 0) {
    return value.message;
  }

  return fallback;
};

const normalizeSpawnSessionError = (value: unknown) => {
  const message = toErrorMessage(value, "Failed to launch a new session.");

  if (message.includes("spawn_session timed out")) {
    return "Session launch timed out after 15 seconds. Verify your working directory and Claude CLI, then try again.";
  }

  if (message.includes("Working directory does not exist")) {
    return message;
  }

  if (message.includes("Working directory is not a directory")) {
    return message;
  }

  if (message.includes("Claude CLI not found")) {
    return "Claude CLI was not found. Install the Claude CLI or set a valid CLI path override in settings, then retry.";
  }

  if (message.includes("Invalid CLI override path")) {
    return message;
  }

  if (message.includes("Unsupported Claude CLI version")) {
    return message;
  }

  return message;
};

export const beginInitialSessionsHydration = () => {
  initialSessionsHydrated.set(false);
  initialSessionsLoadError.set(null);
};

export const completeInitialSessionsHydration = (
  error: string | null = null,
) => {
  initialSessionsLoadError.set(error);
  initialSessionsHydrated.set(true);
  dashboardSortMode.set(get(dashboardSortPreference));
};

const updateSessionStatus = (
  sessionId: string,
  status: string,
  updatedAt: string,
) => {
  sessions.update((items) =>
    items.map((item) =>
      item.id === sessionId ? { ...item, status, updated_at: updatedAt } : item,
    ),
  );
};

const clearSessionRestoreIndicators = (
  sessionId: string,
  updatedAt: string,
) => {
  sessions.update((items) =>
    items.map((item) =>
      item.id === sessionId
        ? {
            ...item,
            restored: false,
            restored_at: null,
            recovery_hint: false,
            updated_at: updatedAt,
          }
        : item,
    ),
  );
};

const withSessionRecord = <T>(
  store: {
    update: (fn: (items: Record<string, T>) => Record<string, T>) => void;
  },
  sessionId: string,
  value: T | null,
) => {
  store.update((items) => {
    if (value === null) {
      if (!(sessionId in items)) {
        return items;
      }

      const next = { ...items };
      delete next[sessionId];
      return next;
    }

    return {
      ...items,
      [sessionId]: value,
    };
  });
};

const setSessionOperation = (
  sessionId: string,
  operation: SessionOperationStatus | null,
) => {
  withSessionRecord(sessionOperations, sessionId, operation);
};

const setSessionError = (sessionId: string, message: string | null) => {
  withSessionRecord(sessionErrors, sessionId, message);
};

const isSessionOperationActive = (sessionId: string) =>
  Boolean(get(sessionOperations)[sessionId]);

function addEvent(sessionId: string, event: SessionEvent) {
  sessionEvents.update((events) => {
    const current = events[sessionId] ?? [];

    if (
      current.some(
        (existing) =>
          existing.type === event.type && existing.data.seq === event.data.seq,
      )
    ) {
      return events;
    }

    if (event.type === "status") {
      const incomingStatus = normalizeStatus(event.data.status);
      const hasSameStatus = current.some(
        (existing) =>
          existing.type === "status" &&
          normalizeStatus(existing.data.status) === incomingStatus,
      );

      if (
        hasSameStatus &&
        (incomingStatus === "running" || isTerminalStatus(incomingStatus))
      ) {
        return events;
      }

      if (
        isTerminalStatus(incomingStatus) &&
        current.some(
          (existing) =>
            existing.type === "status" &&
            isTerminalStatus(existing.data.status),
        )
      ) {
        return events;
      }
    }

    const next = [...current, event].sort(
      (left, right) => left.data.seq - right.data.seq,
    );

    return {
      ...events,
      [sessionId]: next,
    };
  });
}

function flushMessageBuffer(
  sessionId: string,
  seq = nextSeq(),
  timestamp = createTimestamp(),
) {
  const buffered = messageBuffers[sessionId]?.trimEnd();
  if (!buffered) {
    return;
  }

  addEvent(sessionId, {
    type: "message",
    data: {
      session_id: sessionId,
      seq,
      timestamp,
      content: buffered,
      complete: true,
    },
  });

  messageBuffers[sessionId] = "";
}

export function appendMessage(
  sessionId: string,
  chunk: string,
  complete: boolean,
  seq = nextSeq(),
  timestamp = createTimestamp(),
) {
  if (!messageBuffers[sessionId]) {
    messageBuffers[sessionId] = "";
  }

  messageBuffers[sessionId] = `${messageBuffers[sessionId]}${chunk}`;

  if (complete) {
    flushMessageBuffer(sessionId, seq, timestamp);
  }
}

export function routeSessionEvent(event: SessionEvent) {
  const { session_id: sessionId, seq, timestamp } = event.data;
  canonicalSessionEventIds.add(sessionId);

  const activeSession = get(sessions).find((item) => item.id === sessionId);
  if (activeSession?.restored) {
    clearSessionRestoreIndicators(sessionId, timestamp);
  }

  if (event.type === "message") {
    appendMessage(
      sessionId,
      event.data.content,
      event.data.complete,
      seq,
      timestamp,
    );
    return;
  }

  if (event.type === "status") {
    const status = normalizeStatus(event.data.status);
    const normalizedEvent: SessionEvent = {
      ...event,
      data: {
        ...event.data,
        status,
      },
    };

    if (isTerminalStatus(status)) {
      flushMessageBuffer(sessionId, seq, timestamp);
    }

    addEvent(sessionId, normalizedEvent);
    updateSessionStatus(sessionId, status, timestamp);
    return;
  }

  if (event.type === "error") {
    flushMessageBuffer(sessionId, seq, timestamp);
  }

  addEvent(sessionId, event);
}

export function resetSessionEventStateForTests() {
  Object.keys(messageBuffers).forEach((sessionId) => {
    delete messageBuffers[sessionId];
  });
  canonicalSessionEventIds.clear();
  loadedSessionHistory.clear();
  sessionEvents.set({});
  sessionDebug.set({});
  sessionOperations.set({});
  sessionErrors.set({});
  sequenceCounter = 0;
  listenerInitialized = false;
  listenerInitializing = false;
  initialSessionsHydrated.set(false);
  initialSessionsLoadError.set(null);
  dashboardSortMode.set(STARTUP_SORT_MODE);
  dashboardSortPreference.set(loadDashboardSortPreference());
  dashboardNow.set(Date.now());
}

function routeSessionDebugEvent(event: SessionDebugEvent) {
  sessionDebug.update((items) => {
    const current =
      items[event.session_id] ??
      ({
        stderrTail: [],
        updatedAt: event.timestamp,
      } satisfies SessionDebugState);

    if (event.kind === "spawn") {
      return {
        ...items,
        [event.session_id]: {
          ...current,
          cliPath: event.cli_path,
          args: event.args,
          workingDir: event.working_dir,
          updatedAt: event.timestamp,
        },
      };
    }

    if (event.kind === "stderr") {
      const stderrTail = [...current.stderrTail, event.message ?? ""].slice(
        -20,
      );
      return {
        ...items,
        [event.session_id]: {
          ...current,
          stderrTail,
          updatedAt: event.timestamp,
        },
      };
    }

    return items;
  });
}

export async function loadSessions() {
  const [data, dashboard] = await Promise.all([
    invokeWithTimeout<Session[]>(
      "list_sessions",
      undefined,
      LIST_SESSIONS_TIMEOUT_MS,
    ),
    invokeWithTimeout<DashboardSessionProjection[]>(
      "list_dashboard_sessions",
      undefined,
      LIST_SESSIONS_TIMEOUT_MS,
    ),
  ]);

  const sessionList = Array.isArray(data) ? data : [];
  const dashboardRows = Array.isArray(dashboard) ? dashboard : [];

  const dashboardById = new Map(dashboardRows.map((row) => [row.id, row]));
  const current = get(sessions);
  const currentById = new Map(current.map((session) => [session.id, session]));
  const rows: Session[] = sessionList.map((session) => {
    const projection = dashboardById.get(session.id);
    return {
      ...session,
      last_activity_at:
        projection?.last_activity_at ?? session.last_activity_at ?? null,
      failure_reason:
        projection?.failure_reason ?? session.failure_reason ?? null,
      restored: projection?.restored ?? false,
      restored_at: projection?.restored_at ?? null,
      recovery_hint: projection?.recovery_hint ?? false,
    };
  });

  for (const session of sessionList) {
    pendingSpawnSessionIds.delete(session.id);
  }

  for (const sessionId of pendingSpawnSessionIds) {
    if (rows.some((session) => session.id === sessionId)) {
      continue;
    }

    const optimistic = currentById.get(sessionId);
    if (optimistic) {
      rows.unshift(optimistic);
    }
  }

  sessions.set(rows);
  activeSessionId.update((current) => current ?? rows[0]?.id ?? null);
  dashboardSelectedSessionId.update(
    (current) => current ?? rows[0]?.id ?? null,
  );

  for (const session of sessionList) {
    if (normalizeStatus(session.status) !== "running") {
      void loadSessionHistory(session.id);
    }
  }
}

const toHistoryEvents = (
  sessionId: string,
  history: StoredSessionHistoryEvent[],
): SessionEvent[] =>
  history
    .map((event): SessionEvent | null => {
      const payloadType = event.payload_json?.type;
      const payloadData = event.payload_json?.data;
      if (typeof payloadType !== "string" || !payloadData) {
        return null;
      }

      const base = {
        session_id: sessionId,
        seq: event.seq,
        timestamp: event.timestamp,
      };

      if (payloadType === "message") {
        return {
          type: "message",
          data: {
            ...base,
            content: String(payloadData.content ?? ""),
            complete: true,
          },
        };
      }

      if (payloadType === "thinking") {
        return {
          type: "thinking",
          data: {
            ...base,
            content: String(payloadData.content ?? ""),
          },
        };
      }

      if (payloadType === "tool_call") {
        return {
          type: "tool_call",
          data: {
            ...base,
            call_id:
              typeof payloadData.call_id === "string"
                ? payloadData.call_id
                : undefined,
            tool_name: String(payloadData.tool_name ?? "unknown"),
            args: payloadData.args,
          },
        };
      }

      if (payloadType === "tool_result") {
        return {
          type: "tool_result",
          data: {
            ...base,
            call_id:
              typeof payloadData.call_id === "string"
                ? payloadData.call_id
                : undefined,
            tool_name:
              typeof payloadData.tool_name === "string"
                ? payloadData.tool_name
                : undefined,
            result: payloadData.result,
          },
        };
      }

      if (payloadType === "status") {
        return {
          type: "status",
          data: {
            ...base,
            status: normalizeStatus(String(payloadData.status ?? "starting")),
          },
        };
      }

      if (payloadType === "error") {
        return {
          type: "error",
          data: {
            ...base,
            error: String(payloadData.message ?? payloadData.error ?? ""),
          },
        };
      }

      return null;
    })
    .filter((event): event is SessionEvent => event !== null);

const toLegacyHistoryEvents = (
  sessionId: string,
  messages: LegacySessionMessage[],
): SessionEvent[] =>
  messages.map((message, index) => ({
    type: "message",
    data: {
      session_id: sessionId,
      seq: index + 1,
      timestamp: message.timestamp,
      content: String(message.content ?? ""),
      complete: true,
    },
  }));

export async function bootstrapInitialSessions() {
  beginInitialSessionsHydration();

  try {
    await loadSessionsWithRetry();
    completeInitialSessionsHydration();
  } catch (error) {
    completeInitialSessionsHydration(
      toErrorMessage(error, "Failed to load sessions."),
    );
    throw error;
  }
}

export async function loadSessionHistory(sessionId: string) {
  if (loadedSessionHistory.has(sessionId)) {
    return;
  }

  const historyResult = await invoke<StoredSessionHistoryEvent[] | null>(
    "list_session_history",
    {
      id: sessionId,
    },
  ).catch(() => null);

  const history = Array.isArray(historyResult) ? historyResult : [];
  let historyEvents = toHistoryEvents(sessionId, history);

  if (historyEvents.length === 0) {
    const legacyResult = await invoke<LegacySessionMessage[] | null>(
      "list_session_messages",
      {
        id: sessionId,
      },
    ).catch(() => null);

    const legacyMessages = Array.isArray(legacyResult) ? legacyResult : [];
    historyEvents = toLegacyHistoryEvents(sessionId, legacyMessages);
  }

  loadedSessionHistory.add(sessionId);

  if (historyEvents.length === 0) {
    return;
  }

  sessionEvents.update((items) => {
    const existing = items[sessionId] ?? [];
    if (existing.length > 0) {
      return items;
    }

    return {
      ...items,
      [sessionId]: historyEvents,
    };
  });
}

export async function loadSessionsWithRetry(attempts = 5, delayMs = 150) {
  let lastError: unknown;

  for (let attempt = 1; attempt <= attempts; attempt += 1) {
    try {
      console.debug(
        `[sessions] loadSessionsWithRetry attempt ${attempt}/${attempts}`,
      );
      await loadSessions();
      console.debug("[sessions] list_sessions succeeded");
      return;
    } catch (error) {
      lastError = error;
      console.warn("[sessions] list_sessions failed", {
        attempt,
        attempts,
        error: toErrorMessage(error, "Unknown list_sessions failure"),
      });
      if (attempt < attempts) {
        await delay(delayMs);
      }
    }
  }

  throw lastError;
}

function removeSessionLocal(sessionId: string) {
  delete messageBuffers[sessionId];
  canonicalSessionEventIds.delete(sessionId);
  loadedSessionHistory.delete(sessionId);

  sessions.update((items) => items.filter((item) => item.id !== sessionId));
  sessionEvents.update((items) => {
    if (!(sessionId in items)) {
      return items;
    }

    const next = { ...items };
    delete next[sessionId];
    return next;
  });
  sessionDebug.update((items) => {
    if (!(sessionId in items)) {
      return items;
    }

    const next = { ...items };
    delete next[sessionId];
    return next;
  });

  activeSessionId.update((current) => {
    if (current !== sessionId) {
      return current;
    }

    return get(sessions)[0]?.id ?? null;
  });
  dashboardSelectedSessionId.update((current) => {
    if (current !== sessionId) {
      return current;
    }

    return get(sessions)[0]?.id ?? null;
  });
}

export async function spawnSession(
  name: string,
  prompt: string,
  workingDir: string,
) {
  await initSessionListeners();

  const startedAtMs = Date.now();
  const startedAt = new Date(startedAtMs).toISOString();

  let id: string;
  try {
    id = await invokeWithTimeout<string>(
      "spawn_session",
      {
        name,
        prompt,
        workingDir,
        cliPathOverride: loadString("lulu:cli-path-override", "") || null,
      },
      SPAWN_SESSION_TIMEOUT_MS,
    );
  } catch (error) {
    const normalized = normalizeSpawnSessionError(error);
    const finishedAtMs = Date.now();
    spawnRuntimeDiagnostics.set({
      started_at: startedAt,
      finished_at: new Date(finishedAtMs).toISOString(),
      outcome: "spawn_failed",
      session_id: null,
      spawn_duration_ms: Math.max(0, finishedAtMs - startedAtMs),
      refresh_duration_ms: null,
      total_duration_ms: Math.max(0, finishedAtMs - startedAtMs),
      refresh_status: "skipped",
      sessions_count: get(sessions).length,
      has_session_in_store: false,
      active_session_id: get(activeSessionId),
      dashboard_selected_session_id: get(dashboardSelectedSessionId),
      error_message: normalized,
    });
    throw new Error(normalized);
  }

  pendingSpawnSessionIds.add(id);
  const now = createTimestamp();
  sessions.update((items) => {
    if (items.some((item) => item.id === id)) {
      return items;
    }

    return [
      {
        id,
        name,
        status: "starting",
        working_dir: workingDir,
        created_at: now,
        updated_at: now,
      },
      ...items,
    ];
  });
  activeSessionId.set(id);
  dashboardSelectedSessionId.set(id);

  const refreshStartedAtMs = Date.now();
  let refreshStatus: "succeeded" | "failed" = "succeeded";
  let refreshErrorMessage: string | null = null;

  try {
    await loadSessions();
  } catch (error) {
    refreshStatus = "failed";
    refreshErrorMessage = toErrorMessage(
      error,
      "Unknown list_sessions failure",
    );
    console.warn("[sessions] spawn succeeded but refresh failed", {
      sessionId: id,
      error: refreshErrorMessage,
    });
  }

  const finishedAtMs = Date.now();
  const currentSessions = get(sessions);
  const currentActiveSessionId = get(activeSessionId);
  const currentDashboardSelectedSessionId = get(dashboardSelectedSessionId);
  spawnRuntimeDiagnostics.set({
    started_at: startedAt,
    finished_at: new Date(finishedAtMs).toISOString(),
    outcome: "success",
    session_id: id,
    spawn_duration_ms: Math.max(0, refreshStartedAtMs - startedAtMs),
    refresh_duration_ms: Math.max(0, finishedAtMs - refreshStartedAtMs),
    total_duration_ms: Math.max(0, finishedAtMs - startedAtMs),
    refresh_status: refreshStatus,
    sessions_count: currentSessions.length,
    has_session_in_store: currentSessions.some((session) => session.id === id),
    active_session_id: currentActiveSessionId,
    dashboard_selected_session_id: currentDashboardSelectedSessionId,
    error_message: refreshErrorMessage,
  });

  return id;
}

export async function removeSession(sessionId: string, status: string) {
  if (normalizeStatus(status) === "running") {
    await invoke("kill_session", { id: sessionId });
  }

  await invoke("delete_session", { id: sessionId });
  setSessionOperation(sessionId, null);
  setSessionError(sessionId, null);
  removeSessionLocal(sessionId);
}

export async function renameSession(sessionId: string, name: string) {
  const trimmed = name.trim();
  if (!trimmed) {
    throw new Error("Session name cannot be empty.");
  }

  await invoke("rename_session", { id: sessionId, name: trimmed });
  sessions.update((items) =>
    items.map((item) =>
      item.id === sessionId
        ? {
            ...item,
            name: trimmed,
            updated_at: new Date().toISOString(),
          }
        : item,
    ),
  );
}

export async function interruptSession(sessionId: string) {
  if (isSessionOperationActive(sessionId)) {
    throw new Error("Session operation already in progress.");
  }

  setSessionError(sessionId, null);
  setSessionOperation(sessionId, "interrupting");
  updateSessionStatus(sessionId, "interrupting", createTimestamp());

  try {
    await invoke("interrupt_session", { id: sessionId });
    setSessionError(sessionId, null);
    await loadSessions();
  } catch (error) {
    const message = toErrorMessage(error, "Failed to interrupt session.");
    setSessionError(sessionId, message);
    await loadSessions();
    throw new Error(message);
  } finally {
    setSessionOperation(sessionId, null);
  }
}

export async function resumeSession(sessionId: string, prompt: string) {
  if (isSessionOperationActive(sessionId)) {
    throw new Error("Session operation already in progress.");
  }

  const nextPrompt = prompt.trim();
  if (!nextPrompt) {
    const message = "Resume prompt cannot be empty.";
    setSessionError(sessionId, message);
    throw new Error(message);
  }

  setSessionError(sessionId, null);
  setSessionOperation(sessionId, "resuming");
  updateSessionStatus(sessionId, "resuming", createTimestamp());

  try {
    await invoke("resume_session", {
      id: sessionId,
      prompt: nextPrompt,
      cliPathOverride: loadString("lulu:cli-path-override", "") || null,
    });
    setSessionError(sessionId, null);
    await loadSessions();
  } catch (error) {
    const message = toErrorMessage(error, "Failed to resume session.");
    setSessionError(sessionId, message);
    await loadSessions();
    throw new Error(message);
  } finally {
    setSessionOperation(sessionId, null);
  }
}

export async function initSessionListeners() {
  if (listenerInitialized || listenerInitializing) {
    return;
  }

  listenerInitializing = true;

  try {
    await listen<SessionEvent>("session-event", (event) => {
      routeSessionEvent(event.payload);
    });

    await listen<SessionDebugEvent>("session-debug", (event) => {
      routeSessionDebugEvent(event.payload);
    });

    await listen<{ session_id: string; line: string }>(
      "session-output",
      (event) => {
        const { session_id: sessionId, line } = event.payload;
        if (canonicalSessionEventIds.has(sessionId)) {
          return;
        }
        appendMessage(sessionId, `${line}\n`, true);
      },
    );

    await listen<string>("session-started", (event) => {
      const sessionId = event.payload;
      if (canonicalSessionEventIds.has(sessionId)) {
        return;
      }

      const timestamp = createTimestamp();
      addEvent(sessionId, {
        type: "status",
        data: {
          session_id: sessionId,
          seq: nextSeq(),
          timestamp,
          status: "running",
        },
      });
      updateSessionStatus(sessionId, "running", timestamp);
    });

    await listen<string>("session-complete", async (event) => {
      const sessionId = event.payload;
      if (canonicalSessionEventIds.has(sessionId)) {
        await loadSessions();
        return;
      }

      flushMessageBuffer(sessionId);
      const timestamp = createTimestamp();
      addEvent(sessionId, {
        type: "status",
        data: {
          session_id: sessionId,
          seq: nextSeq(),
          timestamp,
          status: "completed",
        },
      });
      updateSessionStatus(sessionId, "completed", timestamp);
      await loadSessions();
    });

    await listen<[string, string]>("session-error", async (event) => {
      const [sessionId, error] = event.payload;
      if (canonicalSessionEventIds.has(sessionId)) {
        await loadSessions();
        return;
      }

      const timestamp = createTimestamp();
      const seq = nextSeq();
      flushMessageBuffer(sessionId, seq, timestamp);
      addEvent(sessionId, {
        type: "error",
        data: {
          session_id: sessionId,
          seq,
          timestamp,
          error,
        },
      });
      addEvent(sessionId, {
        type: "status",
        data: {
          session_id: sessionId,
          seq: nextSeq(),
          timestamp: createTimestamp(),
          status: "failed",
        },
      });
      updateSessionStatus(sessionId, "failed", timestamp);
      await loadSessions();
    });

    listenerInitialized = true;
  } finally {
    listenerInitializing = false;
  }
}

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { derived, get, writable } from "svelte/store";
import type {
  DashboardSessionRow,
  DashboardStatus,
  SessionDebugEvent,
  SessionEvent,
} from "$lib/types/session";

export interface Session {
  id: string;
  name: string;
  status: string;
  working_dir: string;
  created_at: string;
  updated_at: string;
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

type MessageBuffers = Record<string, string>;
interface StoredSessionMessage {
  id: string;
  session_id: string;
  role: string;
  content: string;
  timestamp: string;
}

const messageBuffers: MessageBuffers = {};
const loadedSessionHistory = new Set<string>();
let listenerInitialized = false;
let listenerInitializing = false;
let sequenceCounter = 0;
const canonicalSessionEventIds = new Set<string>();
const TERMINAL_STATUSES = new Set(["completed", "failed", "killed"]);
const FAILURE_STATUSES = new Set([
  "failed",
  "killed",
  "error",
  "interrupted",
  "cancelled",
  "canceled",
  "crashed",
]);
const dashboardNow = writable(Date.now());
const LIST_SESSIONS_TIMEOUT_MS = 1500;

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

export const showThinking = writable<boolean>(
  loadBoolean("lulu:show-thinking", false),
);

export const cliPathOverride = writable<string>(
  loadString("lulu:cli-path-override", ""),
);

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
  [sessions, sessionEvents, dashboardNow],
  ([$sessions, $sessionEvents, $dashboardNow]): DashboardSessionRow[] =>
    [...$sessions]
      .sort((left, right) => {
        const dateDelta = toEpoch(right.created_at) - toEpoch(left.created_at);
        if (dateDelta !== 0) {
          return dateDelta;
        }

        return right.id.localeCompare(left.id);
      })
      .map((session) => {
        const status = toDashboardStatus(session.status);
        const events = $sessionEvents[session.id] ?? [];

        return {
          id: session.id,
          name: session.name,
          status,
          recentActivity: compactAgeLabel(session.updated_at, $dashboardNow),
          failureReason:
            status === "Failed" ? extractFailureReason(events) : undefined,
          createdAt: session.created_at,
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

export const beginInitialSessionsHydration = () => {
  initialSessionsHydrated.set(false);
  initialSessionsLoadError.set(null);
};

export const completeInitialSessionsHydration = (
  error: string | null = null,
) => {
  initialSessionsLoadError.set(error);
  initialSessionsHydrated.set(true);
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
  sequenceCounter = 0;
  listenerInitialized = false;
  listenerInitializing = false;
  initialSessionsHydrated.set(false);
  initialSessionsLoadError.set(null);
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
  const data = await invokeWithTimeout<Session[]>(
    "list_sessions",
    undefined,
    LIST_SESSIONS_TIMEOUT_MS,
  );
  sessions.set(data);
  activeSessionId.update((current) => current ?? data[0]?.id ?? null);
  dashboardSelectedSessionId.update(
    (current) => current ?? data[0]?.id ?? null,
  );

  for (const session of data) {
    if (normalizeStatus(session.status) !== "running") {
      void loadSessionHistory(session.id);
    }
  }
}

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

  const messages = await invoke<StoredSessionMessage[]>(
    "list_session_messages",
    {
      id: sessionId,
    },
  );

  loadedSessionHistory.add(sessionId);

  if (messages.length === 0) {
    return;
  }

  sessionEvents.update((items) => {
    const existing = items[sessionId] ?? [];
    if (existing.length > 0) {
      return items;
    }

    const historyEvents: SessionEvent[] = messages.map((message, index) => ({
      type: "message",
      data: {
        session_id: sessionId,
        seq: index + 1,
        timestamp: message.timestamp,
        content: message.content,
        complete: true,
      },
    }));

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

  const id = await invoke<string>("spawn_session", {
    name,
    prompt,
    workingDir,
    cliPathOverride: loadString("lulu:cli-path-override", "") || null,
  });
  await loadSessions();
  activeSessionId.set(id);
  dashboardSelectedSessionId.set(id);
  return id;
}

export async function removeSession(sessionId: string, status: string) {
  if (normalizeStatus(status) === "running") {
    await invoke("kill_session", { id: sessionId });
  }

  await invoke("delete_session", { id: sessionId });
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

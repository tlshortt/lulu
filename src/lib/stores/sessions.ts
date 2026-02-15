import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { get, writable } from "svelte/store";
import type { SessionDebugEvent, SessionEvent } from "$lib/types/session";

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

const isTerminalStatus = (status: string) =>
  TERMINAL_STATUSES.has(normalizeStatus(status));

const delay = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

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
  const data = await invoke<Session[]>("list_sessions");
  sessions.set(data);
  activeSessionId.update((current) => current ?? data[0]?.id ?? null);

  for (const session of data) {
    if (normalizeStatus(session.status) !== "running") {
      void loadSessionHistory(session.id);
    }
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
      await loadSessions();
      return;
    } catch (error) {
      lastError = error;
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

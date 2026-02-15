import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { writable } from "svelte/store";
import type { SessionEvent } from "$lib/types/session";

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

type MessageBuffers = Record<string, string>;

const messageBuffers: MessageBuffers = {};
let listenerInitialized = false;
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
  sessionEvents.set({});
  sequenceCounter = 0;
  listenerInitialized = false;
}

export async function loadSessions() {
  const data = await invoke<Session[]>("list_sessions");
  sessions.set(data);
  activeSessionId.update((current) => current ?? data[0]?.id ?? null);
}

export async function spawnSession(
  name: string,
  prompt: string,
  workingDir: string,
) {
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

export async function initSessionListeners() {
  if (listenerInitialized) {
    return;
  }

  listenerInitialized = true;

  await listen<SessionEvent>("session-event", (event) => {
    routeSessionEvent(event.payload);
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
}

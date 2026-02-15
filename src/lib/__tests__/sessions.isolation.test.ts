import { beforeEach, describe, expect, it, vi } from "vitest";
import type { SessionEvent } from "$lib/types/session";

const { listenMock, invokeMock, listenerMap } = vi.hoisted(() => {
  const listeners = new Map<string, (event: { payload: unknown }) => void>();

  return {
    listenMock: vi.fn(
      async (
        eventName: string,
        callback: (event: { payload: unknown }) => void,
      ) => {
        listeners.set(eventName, callback);
        return () => {};
      },
    ),
    invokeMock: vi.fn(async (command: string) => {
      if (command === "list_sessions") {
        return [];
      }

      return null;
    }),
    listenerMap: listeners,
  };
});

vi.mock("@tauri-apps/api/event", () => ({
  listen: listenMock,
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: invokeMock,
}));

import {
  initSessionListeners,
  resetSessionEventStateForTests,
  routeSessionEvent,
  sessionEvents,
} from "$lib/stores/sessions";

const readEvents = () => {
  let snapshot: Record<string, SessionEvent[]> = {};
  const unsubscribe = sessionEvents.subscribe((value) => {
    snapshot = value;
  });
  unsubscribe();
  return snapshot;
};

describe("session event isolation", () => {
  beforeEach(() => {
    resetSessionEventStateForTests();
    listenerMap.clear();
    listenMock.mockClear();
    invokeMock.mockClear();
  });

  it("keeps interleaved message buffers isolated by session_id", () => {
    routeSessionEvent({
      type: "message",
      data: {
        session_id: "alpha",
        seq: 1,
        timestamp: "2026-01-01T00:00:00Z",
        content: "hello ",
        complete: false,
      },
    });

    routeSessionEvent({
      type: "message",
      data: {
        session_id: "beta",
        seq: 2,
        timestamp: "2026-01-01T00:00:01Z",
        content: "world",
        complete: true,
      },
    });

    routeSessionEvent({
      type: "message",
      data: {
        session_id: "alpha",
        seq: 3,
        timestamp: "2026-01-01T00:00:02Z",
        content: "there",
        complete: true,
      },
    });

    const events = readEvents();
    expect(events.alpha).toHaveLength(1);
    expect(events.beta).toHaveLength(1);

    expect(events.alpha[0]).toMatchObject({
      type: "message",
      data: {
        session_id: "alpha",
        content: "hello there",
      },
    });
    expect(events.beta[0]).toMatchObject({
      type: "message",
      data: {
        session_id: "beta",
        content: "world",
      },
    });
  });

  it("flushes only the target session buffer on terminal status", () => {
    routeSessionEvent({
      type: "message",
      data: {
        session_id: "s1",
        seq: 1,
        timestamp: "2026-01-01T00:00:00Z",
        content: "incomplete",
        complete: false,
      },
    });

    routeSessionEvent({
      type: "message",
      data: {
        session_id: "s2",
        seq: 2,
        timestamp: "2026-01-01T00:00:01Z",
        content: "second",
        complete: false,
      },
    });

    routeSessionEvent({
      type: "status",
      data: {
        session_id: "s1",
        seq: 3,
        timestamp: "2026-01-01T00:00:02Z",
        status: "completed",
      },
    });

    const events = readEvents();
    expect(events.s1?.map((event) => event.type)).toEqual([
      "message",
      "status",
    ]);
    expect(events.s2).toBeUndefined();
  });

  it("keeps one terminal status when canonical and compatibility listeners both fire", async () => {
    await initSessionListeners();

    const emitSessionEvent = listenerMap.get("session-event");
    const emitSessionComplete = listenerMap.get("session-complete");

    expect(emitSessionEvent).toBeTypeOf("function");
    expect(emitSessionComplete).toBeTypeOf("function");

    emitSessionEvent?.({
      payload: {
        type: "status",
        data: {
          session_id: "dup",
          seq: 10,
          timestamp: "2026-01-01T00:00:10Z",
          status: "completed",
        },
      },
    });
    await emitSessionComplete?.({ payload: "dup" });

    const terminalStatuses =
      readEvents().dup?.filter(
        (event) =>
          event.type === "status" &&
          ["completed", "failed", "killed"].includes(event.data.status),
      ) ?? [];

    expect(terminalStatuses).toHaveLength(1);
    expect(terminalStatuses[0]).toMatchObject({
      type: "status",
      data: { status: "completed" },
    });
  });
});

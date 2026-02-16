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
    invokeMock: vi.fn(async (command: string): Promise<unknown> => {
      if (command === "list_sessions") {
        return [];
      }

      if (command === "spawn_session") {
        return "spawned-session";
      }

      if (command === "list_session_messages") {
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
  activeSessionId,
  initSessionListeners,
  removeSession,
  resetSessionEventStateForTests,
  routeSessionEvent,
  loadSessionHistory,
  renameSession,
  sessionDebug,
  sessionEvents,
  sessions,
  spawnSession,
} from "$lib/stores/sessions";

const readEvents = () => {
  let snapshot: Record<string, SessionEvent[]> = {};
  const unsubscribe = sessionEvents.subscribe((value) => {
    snapshot = value;
  });
  unsubscribe();
  return snapshot;
};

const readDebug = () => {
  let snapshot = {} as Record<
    string,
    {
      cliPath?: string;
      args?: string[];
      workingDir?: string;
      stderrTail: string[];
      updatedAt: string;
    }
  >;
  const unsubscribe = sessionDebug.subscribe((value) => {
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

  it("allows listener initialization retry after a registration failure", async () => {
    listenMock
      .mockImplementationOnce(async () => {
        throw new Error("bridge unavailable");
      })
      .mockImplementation(async (eventName: string, callback) => {
        listenerMap.set(
          eventName,
          callback as (event: { payload: unknown }) => void,
        );
        return () => {};
      });

    await expect(initSessionListeners()).rejects.toThrow("bridge unavailable");

    await expect(initSessionListeners()).resolves.toBeUndefined();
    expect(listenerMap.get("session-event")).toBeTypeOf("function");
  });

  it("captures debug spawn args and stderr tail from backend events", async () => {
    await initSessionListeners();

    const emitDebug = listenerMap.get("session-debug");
    expect(emitDebug).toBeTypeOf("function");

    emitDebug?.({
      payload: {
        session_id: "dbg-1",
        kind: "spawn",
        timestamp: "2026-01-01T00:00:00Z",
        cli_path: "/home/user/.local/bin/claude",
        args: [
          "-p",
          "<prompt redacted>",
          "--verbose",
          "--output-format",
          "stream-json",
        ],
        working_dir: "/tmp/worktree",
      },
    });
    emitDebug?.({
      payload: {
        session_id: "dbg-1",
        kind: "stderr",
        timestamp: "2026-01-01T00:00:01Z",
        message: "Error: sample stderr",
      },
    });

    expect(readDebug()["dbg-1"]).toMatchObject({
      cliPath: "/home/user/.local/bin/claude",
      workingDir: "/tmp/worktree",
      stderrTail: ["Error: sample stderr"],
    });
  });

  it("initializes listeners before invoking spawn_session", async () => {
    await spawnSession("Name", "Prompt", "/tmp/worktree");

    expect(listenerMap.get("session-event")).toBeTypeOf("function");

    const listenOrder = listenMock.mock.invocationCallOrder[0] ?? 0;
    const invokeCallIndex = invokeMock.mock.calls.findIndex(
      ([command]) => command === "spawn_session",
    );
    expect(invokeCallIndex).toBeGreaterThanOrEqual(0);
    const invokeOrder =
      invokeMock.mock.invocationCallOrder[invokeCallIndex] ??
      Number.MAX_SAFE_INTEGER;

    expect(listenOrder).toBeLessThan(invokeOrder);
  });

  it("kills running session then deletes it and removes local state", async () => {
    sessions.set([
      {
        id: "to-remove",
        name: "To Remove",
        status: "running",
        working_dir: "/tmp/worktree",
        created_at: "2026-01-01T00:00:00Z",
        updated_at: "2026-01-01T00:00:00Z",
      },
    ]);
    activeSessionId.set("to-remove");

    routeSessionEvent({
      type: "message",
      data: {
        session_id: "to-remove",
        seq: 1,
        timestamp: "2026-01-01T00:00:00Z",
        content: "hello",
        complete: true,
      },
    });

    await removeSession("to-remove", "running");

    expect(invokeMock).toHaveBeenCalledWith("kill_session", {
      id: "to-remove",
    });
    expect(invokeMock).toHaveBeenCalledWith("delete_session", {
      id: "to-remove",
    });
    expect(readEvents()["to-remove"]).toBeUndefined();

    let sessionsSnapshot: Array<{ id: string }> = [];
    const unsubscribe = sessions.subscribe((value) => {
      sessionsSnapshot = value;
    });
    unsubscribe();
    expect(sessionsSnapshot).toHaveLength(0);
  });

  it("hydrates completed session output from persisted messages", async () => {
    invokeMock.mockImplementation(async (command: string): Promise<unknown> => {
      if (command === "list_session_messages") {
        return [
          {
            id: "msg-1",
            session_id: "persisted-1",
            role: "assistant",
            content: "fdaf",
            timestamp: "2026-01-01T00:00:01Z",
          },
        ];
      }

      return [];
    });

    await loadSessionHistory("persisted-1");

    const events = readEvents()["persisted-1"];
    expect(events).toHaveLength(1);
    expect(events?.[0]).toMatchObject({
      type: "message",
      data: {
        content: "fdaf",
      },
    });
  });

  it("renames a session and updates local store", async () => {
    sessions.set([
      {
        id: "rename-1",
        name: "Old Name",
        status: "completed",
        working_dir: "/tmp/worktree",
        created_at: "2026-01-01T00:00:00Z",
        updated_at: "2026-01-01T00:00:00Z",
      },
    ]);

    await renameSession("rename-1", "New Name");

    expect(invokeMock).toHaveBeenCalledWith("rename_session", {
      id: "rename-1",
      name: "New Name",
    });

    let snapshot: Array<{ id: string; name: string }> = [];
    const unsubscribe = sessions.subscribe((value) => {
      snapshot = value;
    });
    unsubscribe();

    expect(snapshot[0]?.name).toBe("New Name");
  });
});

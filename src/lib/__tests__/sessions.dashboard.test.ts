import { beforeEach, describe, expect, it, vi } from "vitest";
import { get } from "svelte/store";

const { listenMock, invokeMock } = vi.hoisted(() => ({
  listenMock: vi.fn(async () => () => {}),
  invokeMock: vi.fn(
    async (
      command: string,
      _args?: Record<string, unknown>,
    ): Promise<unknown> => {
      if (command === "list_sessions") {
        return [];
      }

      if (command === "list_session_messages") {
        return [];
      }

      return null;
    },
  ),
}));

const setDefaultInvokeMock = () => {
  invokeMock.mockImplementation((command: string) => {
    if (command === "list_sessions") {
      return Promise.resolve([]);
    }

    if (command === "list_session_messages") {
      return Promise.resolve([]);
    }

    return Promise.resolve(null);
  });
};

vi.mock("@tauri-apps/api/event", () => ({
  listen: listenMock,
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: invokeMock,
}));

import {
  bootstrapInitialSessions,
  dashboardRows,
  interruptSession,
  initialSessionsHydrated,
  initialSessionsLoadError,
  refreshDashboardNowForTests,
  resumeSession,
  resetSessionEventStateForTests,
  routeSessionEvent,
  sessionErrors,
  sessionOperations,
  sessions,
} from "$lib/stores/sessions";

const readDashboardRows = () => {
  let snapshot = [] as Array<{
    id: string;
    status: string;
    recentActivity: string;
    failureReason?: string;
  }>;

  const unsubscribe = dashboardRows.subscribe((value) => {
    snapshot = value;
  });
  unsubscribe();

  return snapshot;
};

describe("sessions dashboard projection", () => {
  beforeEach(() => {
    resetSessionEventStateForTests();
    vi.clearAllMocks();
    setDefaultInvokeMock();
    refreshDashboardNowForTests(Date.parse("2026-01-01T00:10:00Z"));
  });

  it("maps statuses to locked four-state vocabulary", () => {
    sessions.set([
      {
        id: "one",
        name: "One",
        status: "running",
        working_dir: "/tmp/one",
        created_at: "2026-01-01T00:00:00Z",
        updated_at: "2026-01-01T00:10:00Z",
      },
      {
        id: "two",
        name: "Two",
        status: "done",
        working_dir: "/tmp/two",
        created_at: "2026-01-01T00:01:00Z",
        updated_at: "2026-01-01T00:10:00Z",
      },
      {
        id: "three",
        name: "Three",
        status: "interrupted",
        working_dir: "/tmp/three",
        created_at: "2026-01-01T00:02:00Z",
        updated_at: "2026-01-01T00:10:00Z",
      },
      {
        id: "four",
        name: "Four",
        status: "queued",
        working_dir: "/tmp/four",
        created_at: "2026-01-01T00:03:00Z",
        updated_at: "2026-01-01T00:10:00Z",
      },
    ]);

    const rows = readDashboardRows();
    expect(rows.map((row) => row.status).sort()).toEqual([
      "Completed",
      "Interrupted",
      "Running",
      "Starting",
    ]);
  });

  it("keeps failed status mapping for killed/error-like states", () => {
    sessions.set([
      {
        id: "failed-killed",
        name: "Killed",
        status: "killed",
        working_dir: "/tmp/failed-killed",
        created_at: "2026-01-01T00:00:00Z",
        updated_at: "2026-01-01T00:10:00Z",
      },
    ]);

    expect(readDashboardRows()[0]?.status).toBe("Failed");
  });

  it("keeps rows newest-first by created_at only", () => {
    sessions.set([
      {
        id: "older",
        name: "Older",
        status: "running",
        working_dir: "/tmp/older",
        created_at: "2026-01-01T00:00:00Z",
        updated_at: "2026-01-01T00:09:58Z",
      },
      {
        id: "newer",
        name: "Newer",
        status: "running",
        working_dir: "/tmp/newer",
        created_at: "2026-01-01T00:09:00Z",
        updated_at: "2026-01-01T00:10:00Z",
      },
    ]);

    expect(readDashboardRows().map((row) => row.id)).toEqual([
      "newer",
      "older",
    ]);

    sessions.update((items) =>
      items.map((item) =>
        item.id === "older"
          ? { ...item, status: "completed", updated_at: "2026-01-01T00:10:00Z" }
          : item,
      ),
    );

    expect(readDashboardRows().map((row) => row.id)).toEqual([
      "newer",
      "older",
    ]);
  });

  it("formats compact age labels and extracts one-line failure reason", () => {
    sessions.set([
      {
        id: "failed-one",
        name: "Failed One",
        status: "failed",
        working_dir: "/tmp/failed",
        created_at: "2026-01-01T00:00:00Z",
        updated_at: "2026-01-01T00:08:30Z",
      },
    ]);

    routeSessionEvent({
      type: "error",
      data: {
        session_id: "failed-one",
        seq: 1,
        timestamp: "2026-01-01T00:08:30Z",
        error: "First line\nsecond line",
      },
    });

    const [row] = readDashboardRows();
    expect(row?.recentActivity).toBe("1m");
    expect(row?.failureReason).toBe("First line second line");
  });

  it("stops running status immediately on terminal update", () => {
    sessions.set([
      {
        id: "pulse-1",
        name: "Pulse",
        status: "running",
        working_dir: "/tmp/pulse",
        created_at: "2026-01-01T00:00:00Z",
        updated_at: "2026-01-01T00:10:00Z",
      },
    ]);

    expect(readDashboardRows()[0]?.status).toBe("Running");

    routeSessionEvent({
      type: "status",
      data: {
        session_id: "pulse-1",
        seq: 2,
        timestamp: "2026-01-01T00:10:01Z",
        status: "completed",
      },
    });

    expect(readDashboardRows()[0]?.status).toBe("Completed");
  });

  it("keeps startup hydration false until bootstrap settles", async () => {
    let resolveListSessions: ((value: unknown) => void) | undefined;

    invokeMock.mockImplementation((command: string) => {
      if (command === "list_sessions") {
        return new Promise((resolve) => {
          resolveListSessions = resolve;
        });
      }

      if (command === "list_session_messages") {
        return Promise.resolve([]);
      }

      return Promise.resolve(null);
    });

    const bootstrap = bootstrapInitialSessions();
    await Promise.resolve();

    expect(get(initialSessionsHydrated)).toBe(false);

    resolveListSessions?.([]);
    await bootstrap;

    expect(get(initialSessionsHydrated)).toBe(true);
    expect(get(initialSessionsLoadError)).toBeNull();
    expect(readDashboardRows()).toHaveLength(0);
  });

  it("marks startup hydration complete with error after retries fail", async () => {
    invokeMock.mockImplementation((command: string) => {
      if (command === "list_sessions") {
        return Promise.reject(new Error("Backend unavailable"));
      }

      if (command === "list_session_messages") {
        return Promise.resolve([]);
      }

      return Promise.resolve(null);
    });

    await expect(bootstrapInitialSessions()).rejects.toThrow(
      "Backend unavailable",
    );

    expect(get(initialSessionsHydrated)).toBe(true);
    expect(get(initialSessionsLoadError)).toBe("Backend unavailable");
  });

  it("tracks interrupt operation and clears it after success", async () => {
    sessions.set([
      {
        id: "target-1",
        name: "Target",
        status: "running",
        working_dir: "/tmp/target",
        created_at: "2026-01-01T00:00:00Z",
        updated_at: "2026-01-01T00:10:00Z",
      },
    ]);

    await interruptSession("target-1");

    expect(invokeMock).toHaveBeenCalledWith("interrupt_session", {
      id: "target-1",
    });
    expect(get(sessionOperations)).toEqual({});
    expect(get(sessionErrors)).toEqual({});
  });

  it("stores per-session interrupt errors without leaking state", async () => {
    sessions.set([
      {
        id: "target-err",
        name: "Target Error",
        status: "running",
        working_dir: "/tmp/target-err",
        created_at: "2026-01-01T00:00:00Z",
        updated_at: "2026-01-01T00:10:00Z",
      },
      {
        id: "other-session",
        name: "Other",
        status: "running",
        working_dir: "/tmp/other",
        created_at: "2026-01-01T00:01:00Z",
        updated_at: "2026-01-01T00:10:00Z",
      },
    ]);

    invokeMock.mockImplementation((command: string, args?: { id?: string }) => {
      if (command === "interrupt_session" && args?.id === "target-err") {
        return Promise.reject(new Error("Interrupt deadline exceeded"));
      }

      if (command === "list_sessions") {
        return Promise.resolve(get(sessions));
      }

      if (command === "list_session_messages") {
        return Promise.resolve([]);
      }

      return Promise.resolve(null);
    });

    await expect(interruptSession("target-err")).rejects.toThrow(
      "Interrupt deadline exceeded",
    );

    expect(get(sessionOperations)["target-err"]).toBeUndefined();
    expect(get(sessionOperations)["other-session"]).toBeUndefined();
    expect(get(sessionErrors)["target-err"]).toBe(
      "Interrupt deadline exceeded",
    );
    expect(get(sessionErrors)["other-session"]).toBeUndefined();
  });

  it("invokes resume with prompt and override payload", async () => {
    sessions.set([
      {
        id: "resume-1",
        name: "Resume",
        status: "completed",
        working_dir: "/tmp/resume",
        created_at: "2026-01-01T00:00:00Z",
        updated_at: "2026-01-01T00:10:00Z",
      },
    ]);

    await resumeSession("resume-1", "continue from tests");

    expect(invokeMock).toHaveBeenCalledWith("resume_session", {
      id: "resume-1",
      prompt: "continue from tests",
      cliPathOverride: null,
    });
    expect(get(sessionErrors)).toEqual({});
  });
});

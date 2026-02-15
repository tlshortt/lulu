import { beforeEach, describe, expect, it, vi } from "vitest";

const { listenMock, invokeMock } = vi.hoisted(() => ({
  listenMock: vi.fn(async () => () => {}),
  invokeMock: vi.fn(async (command: string): Promise<unknown> => {
    if (command === "list_sessions") {
      return [];
    }

    if (command === "list_session_messages") {
      return [];
    }

    return null;
  }),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: listenMock,
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: invokeMock,
}));

import {
  dashboardRows,
  refreshDashboardNowForTests,
  resetSessionEventStateForTests,
  routeSessionEvent,
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
        status: "killed",
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
      "Failed",
      "Running",
      "Starting",
    ]);
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
});

import { fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import { beforeEach, describe, expect, it, vi } from "vitest";
import Sidebar from "./Sidebar.svelte";
import * as sessionStores from "$lib/stores/sessions";

vi.mock("$lib/stores/sessions", async () => {
  const { writable } = await import("svelte/store");

  return {
    activeSessionId: writable<string | null>(null),
    dashboardSelectedSessionId: writable<string | null>(null),
    cliPathOverride: writable(""),
    sessions: writable([
      {
        id: "session-1",
        name: "Build dashboard",
        status: "completed",
        working_dir: "/tmp/project",
        created_at: "2026-01-01T00:00:00Z",
        updated_at: "2026-01-01T00:00:00Z",
      },
    ]),
    dashboardRows: writable([
      {
        id: "session-1",
        name: "Build dashboard",
        status: "Completed",
        recentActivity: "2m",
        createdAt: "2026-01-01T00:00:00Z",
      },
    ]),
    loadSessionHistory: vi.fn(async () => {}),
    removeSession: vi.fn(async () => {}),
  };
});

const readStore = <T>(store: {
  subscribe: (cb: (value: T) => void) => () => void;
}) => {
  let value = undefined as T;
  const unsubscribe = store.subscribe((next) => {
    value = next;
  });
  unsubscribe();
  return value;
};

describe("Sidebar dashboard interactions", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    sessionStores.sessions.set([
      {
        id: "session-1",
        name: "Build dashboard",
        status: "completed",
        working_dir: "/tmp/project",
        created_at: "2026-01-01T00:00:00Z",
        updated_at: "2026-01-01T00:00:00Z",
      },
    ]);
    (
      sessionStores.dashboardRows as unknown as {
        set: (value: unknown) => void;
      }
    ).set([
      {
        id: "session-1",
        name: "Build dashboard",
        status: "Completed",
        recentActivity: "2m",
        createdAt: "2026-01-01T00:00:00Z",
      },
    ]);
    sessionStores.activeSessionId.set(null);
    sessionStores.dashboardSelectedSessionId.set(null);
  });

  it("single click selects row without opening detail", async () => {
    render(Sidebar);

    await fireEvent.click(screen.getByText("Build dashboard"));

    await waitFor(() => {
      expect(readStore(sessionStores.dashboardSelectedSessionId)).toBe(
        "session-1",
      );
    });
    expect(readStore(sessionStores.activeSessionId)).toBeNull();
    expect(sessionStores.loadSessionHistory).toHaveBeenCalledWith("session-1");
  });

  it("double click opens selected session detail", async () => {
    render(Sidebar);

    await fireEvent.doubleClick(screen.getByText("Build dashboard"));

    await waitFor(() => {
      expect(readStore(sessionStores.activeSessionId)).toBe("session-1");
    });
    expect(readStore(sessionStores.dashboardSelectedSessionId)).toBe(
      "session-1",
    );
  });

  it("renders compact age and failure reason fields", async () => {
    (
      sessionStores.dashboardRows as unknown as {
        set: (value: unknown) => void;
      }
    ).set([
      {
        id: "failed-1",
        name: "Failed run",
        status: "Failed",
        recentActivity: "5s",
        failureReason: "Command exited with status 2",
        createdAt: "2026-01-01T00:00:00Z",
      },
    ]);
    sessionStores.sessions.set([
      {
        id: "failed-1",
        name: "Failed run",
        status: "failed",
        working_dir: "/tmp/project",
        created_at: "2026-01-01T00:00:00Z",
        updated_at: "2026-01-01T00:00:00Z",
      },
    ]);

    render(Sidebar);

    expect(screen.getByText("5s").className).toContain("shrink-0");
    expect(screen.getByText("Command exited with status 2")).toBeTruthy();
  });

  it("does not render progress percentages or running subtext", async () => {
    (
      sessionStores.dashboardRows as unknown as {
        set: (value: unknown) => void;
      }
    ).set([
      {
        id: "running-1",
        name: "Live run",
        status: "Running",
        recentActivity: "8s",
        createdAt: "2026-01-01T00:00:00Z",
      },
    ]);
    sessionStores.sessions.set([
      {
        id: "running-1",
        name: "Live run",
        status: "running",
        working_dir: "/tmp/project",
        created_at: "2026-01-01T00:00:00Z",
        updated_at: "2026-01-01T00:00:00Z",
      },
    ]);

    render(Sidebar);

    expect(screen.queryByText("42%")).toBeNull();
    expect(screen.queryByText("tool_call")).toBeNull();
  });

  it("stops running pulse when row reaches terminal state", async () => {
    (
      sessionStores.dashboardRows as unknown as {
        set: (value: unknown) => void;
      }
    ).set([
      {
        id: "pulse-1",
        name: "Pulse row",
        status: "Running",
        recentActivity: "1s",
        createdAt: "2026-01-01T00:00:00Z",
      },
    ]);
    sessionStores.sessions.set([
      {
        id: "pulse-1",
        name: "Pulse row",
        status: "running",
        working_dir: "/tmp/project",
        created_at: "2026-01-01T00:00:00Z",
        updated_at: "2026-01-01T00:00:00Z",
      },
    ]);

    const { container } = render(Sidebar);
    expect(container.querySelector(".animate-pulse")).toBeTruthy();

    (
      sessionStores.dashboardRows as unknown as {
        set: (value: unknown) => void;
      }
    ).set([
      {
        id: "pulse-1",
        name: "Pulse row",
        status: "Completed",
        recentActivity: "2s",
        createdAt: "2026-01-01T00:00:00Z",
      },
    ]);

    await waitFor(() => {
      expect(container.querySelector(".animate-pulse")).toBeNull();
    });
  });
});

import {
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/svelte";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import Sidebar from "./Sidebar.svelte";
import * as sessionStores from "$lib/stores/sessions";

vi.mock("$lib/stores/sessions", async () => {
  const { writable } = await import("svelte/store");

  return {
    activeSessionId: writable<string | null>(null),
    dashboardSelectedSessionId: writable<string | null>(null),
    initialSessionsLoadError: writable<string | null>(null),
    initialSessionsHydrated: writable(true),
    sessionOperations: writable<Record<string, "interrupting" | "resuming">>(
      {},
    ),
    sessionErrors: writable<Record<string, string>>({}),
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
    interruptSession: vi.fn(async () => {}),
    renameSession: vi.fn(async () => {}),
    removeSession: vi.fn(async () => {}),
    resumeSession: vi.fn(async () => {}),
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
  let confirmSpy: ReturnType<typeof vi.spyOn>;

  beforeEach(() => {
    vi.clearAllMocks();
    confirmSpy = vi.spyOn(window, "confirm").mockReturnValue(true);
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
    sessionStores.initialSessionsLoadError.set(null);
    sessionStores.initialSessionsHydrated.set(true);
    sessionStores.sessionOperations.set({});
    sessionStores.sessionErrors.set({});
  });

  afterEach(() => {
    confirmSpy.mockRestore();
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

  it("hides dashboard rows until initial session hydration completes", () => {
    sessionStores.initialSessionsHydrated.set(false);

    render(Sidebar);

    expect(screen.getByText("Loading sessions...")).toBeTruthy();
    expect(screen.queryByText("Build dashboard")).toBeNull();
  });

  it("shows initial load error in empty sidebar state", () => {
    sessionStores.sessions.set([]);
    (
      sessionStores.dashboardRows as unknown as {
        set: (value: unknown) => void;
      }
    ).set([]);
    sessionStores.initialSessionsHydrated.set(true);
    sessionStores.initialSessionsLoadError.set("Failed to load sessions.");

    render(Sidebar);

    expect(screen.getByText("No sessions yet")).toBeTruthy();
    expect(screen.getByText("Failed to load sessions.")).toBeTruthy();
  });

  it("renames a session from the row action", async () => {
    render(Sidebar);

    await fireEvent.click(
      screen.getByRole("button", { name: "Rename Build dashboard" }),
    );

    const input = screen.getByLabelText("Rename session");
    await fireEvent.input(input, { target: { value: "  Updated name  " } });
    await fireEvent.keyDown(input, { key: "Enter", code: "Enter" });

    await waitFor(() => {
      expect(sessionStores.renameSession).toHaveBeenCalledWith(
        "session-1",
        "Updated name",
      );
    });
  });

  it("confirms row interrupt with required copy and invokes store action", async () => {
    sessionStores.sessions.set([
      {
        id: "run-1",
        name: "Running session",
        status: "running",
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
        id: "run-1",
        name: "Running session",
        status: "Running",
        recentActivity: "1s",
        createdAt: "2026-01-01T00:00:00Z",
      },
    ]);

    render(Sidebar);

    await fireEvent.click(screen.getByRole("button", { name: "Interrupt" }));

    expect(confirmSpy).toHaveBeenCalledWith("Interrupt session?");
    expect(sessionStores.interruptSession).toHaveBeenCalledWith("run-1");
  });

  it("shows compact interrupting feedback as chip and spinner", () => {
    sessionStores.sessions.set([
      {
        id: "run-1",
        name: "Running session",
        status: "running",
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
        id: "run-1",
        name: "Running session",
        status: "Running",
        recentActivity: "1s",
        createdAt: "2026-01-01T00:00:00Z",
      },
    ]);
    sessionStores.sessionOperations.set({ "run-1": "interrupting" });

    const { container } = render(Sidebar);

    expect(screen.getAllByText("Interrupting...").length).toBeGreaterThan(0);
    expect(container.querySelector(".animate-spin")).toBeTruthy();
  });

  it("disables controls only for the targeted session operation", () => {
    sessionStores.sessions.set([
      {
        id: "run-1",
        name: "Running one",
        status: "running",
        working_dir: "/tmp/project",
        created_at: "2026-01-01T00:00:00Z",
        updated_at: "2026-01-01T00:00:00Z",
      },
      {
        id: "run-2",
        name: "Running two",
        status: "running",
        working_dir: "/tmp/project",
        created_at: "2026-01-01T00:01:00Z",
        updated_at: "2026-01-01T00:01:00Z",
      },
    ]);
    (
      sessionStores.dashboardRows as unknown as {
        set: (value: unknown) => void;
      }
    ).set([
      {
        id: "run-1",
        name: "Running one",
        status: "Running",
        recentActivity: "1s",
        createdAt: "2026-01-01T00:00:00Z",
      },
      {
        id: "run-2",
        name: "Running two",
        status: "Running",
        recentActivity: "1s",
        createdAt: "2026-01-01T00:01:00Z",
      },
    ]);
    sessionStores.sessionOperations.set({ "run-1": "interrupting" });

    render(Sidebar);

    const rowOne = screen.getByText("Running one").closest("li");
    const rowTwo = screen.getByText("Running two").closest("li");
    const rowOneInterrupt = within(rowOne as HTMLElement)
      .getAllByRole("button")
      .find((button) => button.className.includes("border-amber-500/40"));
    const rowTwoInterrupt = within(rowTwo as HTMLElement)
      .getAllByRole("button")
      .find((button) => button.className.includes("border-amber-500/40"));

    expect(rowOneInterrupt?.hasAttribute("disabled")).toBe(true);
    expect(rowTwoInterrupt?.hasAttribute("disabled")).toBe(false);
  });

  it("shows resume controls for interrupted sessions", () => {
    sessionStores.sessions.set([
      {
        id: "resume-1",
        name: "Interrupted run",
        status: "interrupted",
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
        id: "resume-1",
        name: "Interrupted run",
        status: "Interrupted",
        recentActivity: "5s",
        createdAt: "2026-01-01T00:00:00Z",
      },
    ]);

    render(Sidebar);

    expect(screen.getByText("Interrupted")).toBeTruthy();
    expect(
      screen.getByPlaceholderText("Continue this session..."),
    ).toBeTruthy();
    expect(screen.getByRole("button", { name: "Resume" })).toBeTruthy();
  });

  it("keeps interrupt errors scoped to the affected row", () => {
    sessionStores.sessions.set([
      {
        id: "run-1",
        name: "Failed interrupt",
        status: "running",
        working_dir: "/tmp/project",
        created_at: "2026-01-01T00:00:00Z",
        updated_at: "2026-01-01T00:00:00Z",
      },
      {
        id: "run-2",
        name: "Healthy run",
        status: "running",
        working_dir: "/tmp/project",
        created_at: "2026-01-01T00:01:00Z",
        updated_at: "2026-01-01T00:01:00Z",
      },
    ]);
    (
      sessionStores.dashboardRows as unknown as {
        set: (value: unknown) => void;
      }
    ).set([
      {
        id: "run-1",
        name: "Failed interrupt",
        status: "Running",
        recentActivity: "3s",
        createdAt: "2026-01-01T00:00:00Z",
      },
      {
        id: "run-2",
        name: "Healthy run",
        status: "Running",
        recentActivity: "3s",
        createdAt: "2026-01-01T00:01:00Z",
      },
    ]);
    sessionStores.sessionErrors.set({ "run-1": "Interrupt deadline exceeded" });

    render(Sidebar);

    expect(screen.getByText("Interrupt deadline exceeded")).toBeTruthy();
    expect(screen.queryByText("Healthy run")).toBeTruthy();
  });
});

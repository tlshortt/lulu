import { fireEvent, render, screen } from "@testing-library/svelte";
import { tick } from "svelte";
import { describe, it, expect, beforeEach, vi } from "vitest";
import {
  activeSessionId,
  dashboardSelectedSessionId,
  initialSessionsLoadError,
  initialSessionsHydrated,
  initialSessionsRetryError,
  sessions,
} from "$lib/stores/sessions";
import MainArea from "./MainArea.svelte";

describe("MainArea", () => {
  beforeEach(() => {
    sessions.set([]);
    activeSessionId.set(null);
    dashboardSelectedSessionId.set(null);
    initialSessionsLoadError.set(null);
    initialSessionsRetryError.set(null);
    initialSessionsHydrated.set(true);
  });

  it("shows empty state when there are no sessions", () => {
    render(MainArea);

    expect(screen.getByText("No active sessions")).toBeTruthy();
    expect(screen.getByText(/Start a Claude Code session/)).toBeTruthy();
    expect(screen.getByText("⌘ + N")).toBeTruthy();
  });

  it("does not show empty state when a session is active but list is stale", () => {
    activeSessionId.set("pending-1");

    render(MainArea);

    expect(screen.queryByText("No active sessions")).toBeNull();
    expect(screen.getByText("Status: unknown")).toBeTruthy();
  });

  it("renders SessionOutput when sessions exist", () => {
    sessions.set([
      {
        id: "test-1",
        name: "Test Session",
        status: "running",
        working_dir: "/tmp",
        created_at: "2025-01-01T00:00:00Z",
        updated_at: "2025-01-01T00:00:00Z",
      },
    ]);
    activeSessionId.set("test-1");

    render(MainArea);

    // The empty state should NOT be shown
    expect(screen.queryByText("No active sessions")).toBeNull();
  });

  it("prompts double-click when row is selected but not opened", () => {
    sessions.set([
      {
        id: "selected-1",
        name: "Selected Session",
        status: "running",
        working_dir: "/tmp",
        created_at: "2025-01-01T00:00:00Z",
        updated_at: "2025-01-01T00:00:00Z",
      },
    ]);
    dashboardSelectedSessionId.set("selected-1");

    render(MainArea);

    expect(
      screen.getByText("Double-click to open selected session output"),
    ).toBeTruthy();
  });

  it("shows session output when active session exists before hydration completes", () => {
    initialSessionsHydrated.set(false);
    sessions.set([
      {
        id: "transient-1",
        name: "Transient Session",
        status: "running",
        working_dir: "/tmp",
        created_at: "2025-01-01T00:00:00Z",
        updated_at: "2025-01-01T00:00:00Z",
      },
    ]);
    activeSessionId.set("transient-1");

    render(MainArea);

    expect(screen.queryByText("Loading sessions...")).toBeNull();
    expect(screen.queryByText("No active sessions")).toBeNull();
    expect(
      screen.queryByText("Double-click to open selected session output"),
    ).toBeNull();
    expect(screen.getByText("Status: running")).toBeTruthy();
  });

  it("shows load error when initial fetch fails", () => {
    initialSessionsLoadError.set("Failed to load sessions.");

    render(MainArea);

    expect(screen.getByText("No active sessions")).toBeTruthy();
    expect(screen.getByText("Failed to load sessions.")).toBeTruthy();
  });

  it("lands on startup view after hydration completes with empty snapshot", async () => {
    initialSessionsHydrated.set(false);

    render(MainArea);

    expect(screen.getByText("Loading sessions...")).toBeTruthy();

    initialSessionsHydrated.set(true);
    await tick();

    expect(screen.queryByText("Loading sessions...")).toBeNull();
    expect(screen.getByText("No active sessions")).toBeTruthy();
  });

  it("shows copy error button on stalled hydration and copies retry error", async () => {
    vi.useFakeTimers();

    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(window.navigator, "clipboard", {
      configurable: true,
      value: { writeText },
    });

    initialSessionsHydrated.set(false);
    initialSessionsRetryError.set("Backend unavailable");

    render(MainArea);

    vi.advanceTimersByTime(4000);
    await tick();

    const button = screen.getByRole("button", { name: "Copy error" });
    expect(button).toBeTruthy();

    await fireEvent.click(button);

    expect(writeText).toHaveBeenCalledWith("Backend unavailable");
    expect(screen.getByText("Copied")).toBeTruthy();

    vi.useRealTimers();
  });
});

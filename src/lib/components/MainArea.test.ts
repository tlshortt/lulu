import { render, screen } from "@testing-library/svelte";
import { tick } from "svelte";
import { describe, it, expect, beforeEach } from "vitest";
import {
  activeSessionId,
  dashboardSelectedSessionId,
  initialSessionsLoadError,
  initialSessionsHydrated,
  sessions,
} from "$lib/stores/sessions";
import MainArea from "./MainArea.svelte";

describe("MainArea", () => {
  beforeEach(() => {
    sessions.set([]);
    activeSessionId.set(null);
    dashboardSelectedSessionId.set(null);
    initialSessionsLoadError.set(null);
    initialSessionsHydrated.set(true);
  });

  it("shows empty state when there are no sessions", () => {
    render(MainArea);

    expect(screen.getByText("No active sessions")).toBeTruthy();
    expect(screen.getByText(/Start a Claude Code session/)).toBeTruthy();
    expect(screen.getByText("⌘ + N")).toBeTruthy();
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

  it("suppresses transient session content before initial hydration", () => {
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

    expect(screen.getByText("Loading sessions...")).toBeTruthy();
    expect(screen.queryByText("No active sessions")).toBeNull();
    expect(
      screen.queryByText("Double-click to open selected session output"),
    ).toBeNull();
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
});

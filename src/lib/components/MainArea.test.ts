import { render, screen } from "@testing-library/svelte";
import { describe, it, expect, beforeEach } from "vitest";
import {
  activeSessionId,
  selectedSessionId,
  sessions,
} from "$lib/stores/sessions";
import MainArea from "./MainArea.svelte";

describe("MainArea", () => {
  beforeEach(() => {
    sessions.set([]);
    activeSessionId.set(null);
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

  it("keeps selectedSessionId alias compatibility with activeSessionId", () => {
    sessions.set([
      {
        id: "compat-1",
        name: "Compatibility Session",
        status: "running",
        working_dir: "/tmp",
        created_at: "2025-01-01T00:00:00Z",
        updated_at: "2025-01-01T00:00:00Z",
      },
    ]);
    selectedSessionId.set("compat-1");

    render(MainArea);

    expect(screen.queryByText("Pick a session to view output")).toBeNull();
  });
});

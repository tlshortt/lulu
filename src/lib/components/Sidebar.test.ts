import { fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import { beforeEach, describe, expect, it, vi } from "vitest";
import Sidebar from "./Sidebar.svelte";
import * as sessionStores from "$lib/stores/sessions";

vi.mock("$lib/stores/sessions", async () => {
  const { writable } = await import("svelte/store");

  return {
    activeSessionId: writable<string | null>(null),
    cliPathOverride: writable(""),
    sessions: writable([
      {
        id: "session-1",
        name: "Original Name",
        status: "completed",
        working_dir: "/tmp/project",
        created_at: "2026-01-01T00:00:00Z",
        updated_at: "2026-01-01T00:00:00Z",
      },
    ]),
    loadSessionHistory: vi.fn(async () => {}),
    renameSession: vi.fn(async () => {}),
    removeSession: vi.fn(async () => {}),
  };
});

describe("Sidebar rename", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    sessionStores.sessions.set([
      {
        id: "session-1",
        name: "Original Name",
        status: "completed",
        working_dir: "/tmp/project",
        created_at: "2026-01-01T00:00:00Z",
        updated_at: "2026-01-01T00:00:00Z",
      },
    ]);
  });

  it("renames a session on Enter after double-click", async () => {
    render(Sidebar);

    await fireEvent.doubleClick(screen.getByText("Original Name"));

    const input = screen.getByDisplayValue("Original Name");
    await fireEvent.input(input, { target: { value: "  Renamed Session  " } });
    await fireEvent.keyDown(input, { key: "Enter" });

    await waitFor(() => {
      expect(sessionStores.renameSession).toHaveBeenCalledWith(
        "session-1",
        "Renamed Session",
      );
    });
  });

  it("renames a session on blur", async () => {
    render(Sidebar);

    await fireEvent.doubleClick(screen.getByText("Original Name"));

    const input = screen.getByDisplayValue("Original Name");
    await fireEvent.input(input, { target: { value: "Blur Rename" } });
    await fireEvent.blur(input);

    await waitFor(() => {
      expect(sessionStores.renameSession).toHaveBeenCalledWith(
        "session-1",
        "Blur Rename",
      );
    });
  });

  it("cancels rename on Escape without persisting", async () => {
    render(Sidebar);

    await fireEvent.doubleClick(screen.getByText("Original Name"));

    const input = screen.getByDisplayValue("Original Name");
    await fireEvent.input(input, { target: { value: "Should Not Save" } });
    await fireEvent.keyDown(input, { key: "Escape" });

    await waitFor(() => {
      expect(screen.queryByDisplayValue("Should Not Save")).toBeNull();
    });
    expect(sessionStores.renameSession).not.toHaveBeenCalled();
  });
});

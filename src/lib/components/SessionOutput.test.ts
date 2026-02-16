import { fireEvent, render, screen } from "@testing-library/svelte";
import { beforeEach, describe, expect, it } from "vitest";
import SessionOutput from "./SessionOutput.svelte";
import {
  activeSessionId,
  sessionDebug,
  sessionEvents,
  sessions,
  showThinking,
} from "$lib/stores/sessions";
import type { SessionEvent } from "$lib/types/session";

const sessionId = "session-1";

const setSessionState = (events: SessionEvent[]) => {
  sessions.set([
    {
      id: sessionId,
      name: "Output Session",
      status: "running",
      working_dir: "/tmp/worktree",
      created_at: "2026-01-01T00:00:00Z",
      updated_at: "2026-01-01T00:00:00Z",
    },
  ]);
  activeSessionId.set(sessionId);
  sessionEvents.set({ [sessionId]: events });
};

describe("SessionOutput", () => {
  beforeEach(() => {
    activeSessionId.set(null);
    sessions.set([]);
    sessionEvents.set({});
    sessionDebug.set({});
    showThinking.set(false);
  });

  it("renders message, tool call/result, status, and error events", () => {
    setSessionState([
      {
        type: "message",
        data: {
          session_id: sessionId,
          seq: 1,
          timestamp: "2026-01-01T00:00:00Z",
          content: "stream line",
          complete: true,
        },
      },
      {
        type: "tool_call",
        data: {
          session_id: sessionId,
          seq: 2,
          timestamp: "2026-01-01T00:00:01Z",
          call_id: "call-1",
          tool_name: "bash",
          args: { command: "pwd" },
        },
      },
      {
        type: "tool_result",
        data: {
          session_id: sessionId,
          seq: 3,
          timestamp: "2026-01-01T00:00:02Z",
          call_id: "call-1",
          tool_name: "bash",
          result: { stdout: "/tmp/worktree" },
        },
      },
      {
        type: "status",
        data: {
          session_id: sessionId,
          seq: 4,
          timestamp: "2026-01-01T00:00:03Z",
          status: "completed",
        },
      },
      {
        type: "error",
        data: {
          session_id: sessionId,
          seq: 5,
          timestamp: "2026-01-01T00:00:04Z",
          error: "sample error",
        },
      },
    ]);

    render(SessionOutput);

    expect(screen.getByText("stream line")).toBeTruthy();
    expect(screen.getByText("Tool: bash")).toBeTruthy();
    expect(screen.getByText("Result")).toBeTruthy();
    expect(screen.getByText("completed")).toBeTruthy();
    expect(screen.getByText("sample error")).toBeTruthy();
  });

  it("hides thinking by default and reveals it when toggled", async () => {
    setSessionState([
      {
        type: "thinking",
        data: {
          session_id: sessionId,
          seq: 1,
          timestamp: "2026-01-01T00:00:00Z",
          content: "hidden by default",
        },
      },
    ]);

    render(SessionOutput);

    expect(screen.queryByText("hidden by default")).toBeNull();

    await fireEvent.click(
      screen.getByRole("button", { name: "Show thinking" }),
    );

    expect(screen.getByText("hidden by default")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Hide thinking" })).toBeTruthy();
  });

  it("shows a thinking hint when only hidden thinking events exist", () => {
    setSessionState([
      {
        type: "thinking",
        data: {
          session_id: sessionId,
          seq: 1,
          timestamp: "2026-01-01T00:00:00Z",
          content: "internal reasoning",
        },
      },
    ]);

    render(SessionOutput);

    expect(
      screen.getByText(
        'No visible output yet. Click "Show thinking" to view reasoning.',
      ),
    ).toBeTruthy();
  });

  it("shows waiting text while running with no events", () => {
    setSessionState([]);

    render(SessionOutput);

    expect(screen.getByText("Waiting for first output...")).toBeTruthy();
    expect(screen.getByText("Debug: spawn args + stderr tail")).toBeTruthy();
  });

  it("renders debug spawn args and stderr tail when present", async () => {
    setSessionState([]);
    sessionDebug.set({
      [sessionId]: {
        cliPath: "/home/user/.local/bin/claude",
        args: [
          "-p",
          "<prompt redacted>",
          "--verbose",
          "--output-format",
          "stream-json",
        ],
        workingDir: "/tmp/worktree",
        stderrTail: [
          "Error: sample stderr line 1",
          "Error: sample stderr line 2",
        ],
        updatedAt: "2026-01-01T00:00:00Z",
      },
    });

    render(SessionOutput);

    await fireEvent.click(screen.getByText("Debug: spawn args + stderr tail"));

    expect(screen.getByText("/home/user/.local/bin/claude")).toBeTruthy();
    expect(screen.getByText(/--output-format stream-json/)).toBeTruthy();
    expect(screen.getByText(/sample stderr line 1/)).toBeTruthy();
  });
});

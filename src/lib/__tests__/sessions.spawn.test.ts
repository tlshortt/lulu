import { beforeEach, describe, expect, it, vi } from "vitest";

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

      if (command === "spawn_session") {
        return "spawned-session";
      }

      if (command === "list_session_messages") {
        return [];
      }

      return null;
    },
  ),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: listenMock,
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: invokeMock,
}));

import {
  activeSessionId,
  dashboardSelectedSessionId,
  resetSessionEventStateForTests,
  spawnSession,
} from "$lib/stores/sessions";
import { get } from "svelte/store";

describe("spawn session launch-path behavior", () => {
  beforeEach(() => {
    resetSessionEventStateForTests();
    activeSessionId.set(null);
    dashboardSelectedSessionId.set(null);
    vi.clearAllMocks();
  });

  it("normalizes spawn timeout into actionable retry guidance", async () => {
    invokeMock.mockImplementation((command: string) => {
      if (command === "spawn_session") {
        return Promise.reject(
          new Error("spawn_session timed out after 15000ms"),
        );
      }

      if (command === "list_sessions") {
        return Promise.resolve([]);
      }

      if (command === "list_session_messages") {
        return Promise.resolve([]);
      }

      return Promise.resolve(null);
    });

    await expect(
      spawnSession("Timeout", "Prompt", "/tmp/worktree"),
    ).rejects.toThrow(
      "Session launch timed out after 15 seconds. Verify your working directory and Claude CLI, then try again.",
    );
  });

  it("keeps backend working directory validation reason user-visible", async () => {
    invokeMock.mockImplementation((command: string) => {
      if (command === "spawn_session") {
        return Promise.reject(
          new Error("Working directory does not exist: /tmp/missing"),
        );
      }

      if (command === "list_sessions") {
        return Promise.resolve([]);
      }

      if (command === "list_session_messages") {
        return Promise.resolve([]);
      }

      return Promise.resolve(null);
    });

    await expect(
      spawnSession("Missing", "Prompt", "/tmp/missing"),
    ).rejects.toThrow("Working directory does not exist: /tmp/missing");
  });

  it("returns session id even when post-launch refresh fails", async () => {
    invokeMock.mockImplementation((command: string) => {
      if (command === "spawn_session") {
        return Promise.resolve("session-123");
      }

      if (command === "list_sessions") {
        return Promise.reject(new Error("transient refresh failure"));
      }

      if (command === "list_session_messages") {
        return Promise.resolve([]);
      }

      return Promise.resolve(null);
    });

    const id = await spawnSession("Recover", "Prompt", "/tmp/worktree");
    expect(id).toBe("session-123");
    expect(get(activeSessionId)).toBe("session-123");
    expect(get(dashboardSelectedSessionId)).toBe("session-123");
  });

  it("allows retry after failed launch without stale session selection", async () => {
    invokeMock
      .mockImplementationOnce((command: string) => {
        if (command === "spawn_session") {
          return Promise.reject(
            new Error("Working directory is not a directory: /tmp/file"),
          );
        }

        if (command === "list_sessions") {
          return Promise.resolve([]);
        }

        if (command === "list_session_messages") {
          return Promise.resolve([]);
        }

        return Promise.resolve(null);
      })
      .mockImplementation((command: string) => {
        if (command === "spawn_session") {
          return Promise.resolve("session-456");
        }

        if (command === "list_sessions") {
          return Promise.resolve([]);
        }

        if (command === "list_session_messages") {
          return Promise.resolve([]);
        }

        return Promise.resolve(null);
      });

    await expect(spawnSession("First", "Prompt", "/tmp/file")).rejects.toThrow(
      "Working directory is not a directory: /tmp/file",
    );
    expect(get(activeSessionId)).toBeNull();
    expect(get(dashboardSelectedSessionId)).toBeNull();

    const retried = await spawnSession("Retry", "Prompt", "/tmp/worktree");
    expect(retried).toBe("session-456");
    expect(get(activeSessionId)).toBe("session-456");
    expect(get(dashboardSelectedSessionId)).toBe("session-456");
  });
});

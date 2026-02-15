import { fireEvent, render, screen } from "@testing-library/svelte";
import { beforeEach, describe, expect, it, vi } from "vitest";
import Page from "./+page.svelte";

const { invokeMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(async (command: string) => {
    if (command === "list_sessions") {
      return [];
    }

    return "mock-session-id";
  }),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: invokeMock,
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async () => vi.fn()),
}));

describe("page keyboard shortcuts", () => {
  beforeEach(() => {
    invokeMock.mockClear();
    window.localStorage.clear();
    Object.defineProperty(window, "innerWidth", {
      value: 1200,
      writable: true,
      configurable: true,
    });
  });

  it("opens New Session modal on Cmd+N", async () => {
    render(Page);

    expect(screen.queryByRole("dialog", { name: "New Session" })).toBeNull();

    await fireEvent.keyDown(window, { key: "n", metaKey: true });

    expect(screen.getByRole("dialog", { name: "New Session" })).toBeTruthy();
  });

  it("does not open modal when shortcut is pressed in inputs", async () => {
    render(Page);

    const input = document.createElement("input");
    document.body.appendChild(input);
    input.focus();

    await fireEvent.keyDown(input, { key: "n", metaKey: true });

    expect(screen.queryByRole("dialog", { name: "New Session" })).toBeNull();

    input.remove();
  });

  it("loads and clamps sidebar width from storage", async () => {
    window.localStorage.setItem("lulu:sidebar-width", "600");

    render(Page);

    const separator = await screen.findByRole("separator", {
      name: "Resize sidebar",
    });
    const sidebarContainer = separator.parentElement as HTMLElement;

    expect(sidebarContainer.style.width).toBe("480px");
  });

  it("resizes sidebar by drag and persists width", async () => {
    render(Page);

    const separator = await screen.findByRole("separator", {
      name: "Resize sidebar",
    });
    const sidebarContainer = separator.parentElement as HTMLElement;

    await fireEvent.pointerDown(separator, { clientX: 280 });
    await fireEvent.pointerMove(window, { clientX: 360 });
    await fireEvent.pointerUp(window);

    expect(sidebarContainer.style.width).toBe("360px");
    expect(window.localStorage.getItem("lulu:sidebar-width")).toBe("360");
  });
});

import { fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import { beforeEach, describe, expect, it, vi } from "vitest";
import NewSessionModal from "./NewSessionModal.svelte";

const { spawnSessionMock } = vi.hoisted(() => ({
  spawnSessionMock: vi.fn(),
}));

vi.mock("$lib/stores/sessions", () => ({
  spawnSession: spawnSessionMock,
}));

describe("NewSessionModal", () => {
  beforeEach(() => {
    spawnSessionMock.mockReset();
  });

  it("shows validation error when required fields are empty", async () => {
    render(NewSessionModal, {
      props: { open: true, onClose: vi.fn() },
    });

    await fireEvent.click(
      screen.getByRole("button", { name: "Start session" }),
    );

    expect(screen.getByText("Please fill out all fields.")).toBeTruthy();
    expect(spawnSessionMock).not.toHaveBeenCalled();
  });

  it("submits trimmed values and closes the modal", async () => {
    const onClose = vi.fn();
    spawnSessionMock.mockResolvedValue("session-1");

    render(NewSessionModal, {
      props: { open: true, onClose },
    });

    await fireEvent.input(screen.getByLabelText("Session name"), {
      target: { value: "  Design Review  " },
    });
    await fireEvent.input(screen.getByLabelText("Prompt"), {
      target: { value: "  Summarize latest changes  " },
    });
    await fireEvent.input(screen.getByLabelText("Working directory"), {
      target: { value: "  /tmp/project  " },
    });

    await fireEvent.click(
      screen.getByRole("button", { name: "Start session" }),
    );

    await waitFor(() => {
      expect(spawnSessionMock).toHaveBeenCalledWith(
        "Design Review",
        "Summarize latest changes",
        "/tmp/project",
      );
      expect(onClose).toHaveBeenCalledTimes(1);
    });
  });

  it("calls spawnSession exactly once while submitting", async () => {
    spawnSessionMock.mockImplementation(
      () => new Promise<string>(() => undefined),
    );

    render(NewSessionModal, {
      props: { open: true, onClose: vi.fn() },
    });

    await fireEvent.input(screen.getByLabelText("Session name"), {
      target: { value: "Single Call" },
    });
    await fireEvent.input(screen.getByLabelText("Prompt"), {
      target: { value: "Run once" },
    });
    await fireEvent.input(screen.getByLabelText("Working directory"), {
      target: { value: "/tmp/project" },
    });

    const submit = screen.getByRole("button", { name: "Start session" });
    await fireEvent.click(submit);
    await fireEvent.click(submit);

    expect(spawnSessionMock).toHaveBeenCalledTimes(1);
  });

  it("does not close when pressing space inside an input", async () => {
    const onClose = vi.fn();

    render(NewSessionModal, {
      props: { open: true, onClose },
    });

    const nameInput = screen.getByLabelText("Session name");
    await fireEvent.keyDown(nameInput, { key: " ", code: "Space" });

    expect(onClose).not.toHaveBeenCalled();
  });

  it("submits when pressing Enter in an input field", async () => {
    const onClose = vi.fn();
    spawnSessionMock.mockResolvedValue("session-2");

    render(NewSessionModal, {
      props: { open: true, onClose },
    });

    await fireEvent.input(screen.getByLabelText("Session name"), {
      target: { value: " Enter Submit " },
    });
    await fireEvent.input(screen.getByLabelText("Prompt"), {
      target: { value: " Run from enter " },
    });
    const workingDirInput = screen.getByLabelText("Working directory");
    await fireEvent.input(workingDirInput, {
      target: { value: " /tmp/enter " },
    });

    await fireEvent.keyDown(workingDirInput, { key: "Enter", code: "Enter" });

    await waitFor(() => {
      expect(spawnSessionMock).toHaveBeenCalledWith(
        "Enter Submit",
        "Run from enter",
        "/tmp/enter",
      );
      expect(onClose).toHaveBeenCalledTimes(1);
    });
  });
});

import { beforeEach, describe, expect, it } from "vitest";
import {
  resetSessionEventStateForTests,
  routeSessionEvent,
  sessionEvents,
} from "$lib/stores/sessions";
import type { SessionEvent } from "$lib/types/session";

const readEvents = () => {
  let snapshot: Record<string, SessionEvent[]> = {};
  const unsubscribe = sessionEvents.subscribe((value) => {
    snapshot = value;
  });
  unsubscribe();
  return snapshot;
};

describe("session event isolation", () => {
  beforeEach(() => {
    resetSessionEventStateForTests();
  });

  it("keeps interleaved message buffers isolated by session_id", () => {
    routeSessionEvent({
      type: "message",
      data: {
        session_id: "alpha",
        seq: 1,
        timestamp: "2026-01-01T00:00:00Z",
        content: "hello ",
        complete: false,
      },
    });

    routeSessionEvent({
      type: "message",
      data: {
        session_id: "beta",
        seq: 2,
        timestamp: "2026-01-01T00:00:01Z",
        content: "world",
        complete: true,
      },
    });

    routeSessionEvent({
      type: "message",
      data: {
        session_id: "alpha",
        seq: 3,
        timestamp: "2026-01-01T00:00:02Z",
        content: "there",
        complete: true,
      },
    });

    const events = readEvents();
    expect(events.alpha).toHaveLength(1);
    expect(events.beta).toHaveLength(1);

    expect(events.alpha[0]).toMatchObject({
      type: "message",
      data: {
        session_id: "alpha",
        content: "hello there",
      },
    });
    expect(events.beta[0]).toMatchObject({
      type: "message",
      data: {
        session_id: "beta",
        content: "world",
      },
    });
  });

  it("flushes only the target session buffer on complete status", () => {
    routeSessionEvent({
      type: "message",
      data: {
        session_id: "s1",
        seq: 1,
        timestamp: "2026-01-01T00:00:00Z",
        content: "incomplete",
        complete: false,
      },
    });

    routeSessionEvent({
      type: "message",
      data: {
        session_id: "s2",
        seq: 2,
        timestamp: "2026-01-01T00:00:01Z",
        content: "second",
        complete: false,
      },
    });

    routeSessionEvent({
      type: "status",
      data: {
        session_id: "s1",
        seq: 3,
        timestamp: "2026-01-01T00:00:02Z",
        status: "complete",
      },
    });

    const events = readEvents();
    expect(events.s1?.map((event) => event.type)).toEqual([
      "message",
      "status",
    ]);
    expect(events.s2).toBeUndefined();
  });
});

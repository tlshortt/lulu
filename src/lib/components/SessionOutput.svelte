<script lang="ts">
  import ToolCallBlock from "$lib/components/ToolCallBlock.svelte";
  import {
    activeSessionId,
    sessionEvents,
    sessions,
    showThinking,
  } from "$lib/stores/sessions";
  import type { SessionEvent, ToolResultEventData } from "$lib/types/session";

  const { sessionId = null } = $props<{ sessionId?: string | null }>();

  interface ToolCallRender {
    call: Extract<SessionEvent, { type: "tool_call" }>;
    result: ToolResultEventData | null;
  }

  type RenderItem =
    | { kind: "tool_call"; value: ToolCallRender }
    | { kind: "event"; value: SessionEvent };

  const resolveToolCalls = (events: SessionEvent[]): RenderItem[] => {
    const resultIndices = new Set<number>();
    const items: RenderItem[] = [];

    for (const [index, event] of events.entries()) {
      if (event.type !== "tool_call") {
        continue;
      }

      const resultIndex = events.findIndex((candidate, candidateIndex) => {
        if (candidateIndex <= index || resultIndices.has(candidateIndex)) {
          return false;
        }

        if (candidate.type !== "tool_result") {
          return false;
        }

        if (event.data.call_id && candidate.data.call_id) {
          return event.data.call_id === candidate.data.call_id;
        }

        return candidate.data.tool_name === event.data.tool_name;
      });

      if (resultIndex >= 0) {
        resultIndices.add(resultIndex);
      }

      const result =
        resultIndex >= 0 && events[resultIndex]?.type === "tool_result"
          ? events[resultIndex].data
          : null;

      items.push({
        kind: "tool_call",
        value: {
          call: event,
          result,
        },
      });
    }

    for (const [index, event] of events.entries()) {
      if (event.type === "tool_call") {
        continue;
      }

      if (event.type === "tool_result" && resultIndices.has(index)) {
        continue;
      }

      items.push({ kind: "event", value: event });
    }

    items.sort((a, b) => {
      const left =
        a.kind === "tool_call" ? a.value.call.data.seq : a.value.data.seq;
      const right =
        b.kind === "tool_call" ? b.value.call.data.seq : b.value.data.seq;
      return left - right;
    });

    return items;
  };

  const currentSessionId = $derived(sessionId ?? $activeSessionId);
  const session = $derived(
    currentSessionId
      ? $sessions.find((item) => item.id === currentSessionId)
      : null,
  );
  const events = $derived(
    currentSessionId ? ($sessionEvents[currentSessionId] ?? []) : [],
  );
  const renderItems = $derived(
    resolveToolCalls(
      events.filter(
        (event: SessionEvent) => $showThinking || event.type !== "thinking",
      ),
    ),
  );
</script>

{#if !currentSessionId}
  <div
    class="flex h-full flex-col items-center justify-center gap-4 px-8 text-center"
  >
    <div
      class="text-sm font-semibold uppercase tracking-[0.2em] text-foreground/40"
    >
      Sessions
    </div>
    <div class="text-2xl font-semibold">Select a session</div>
    <p class="max-w-md text-sm text-foreground/60">
      Choose a running session from the sidebar to see streaming output here.
    </p>
  </div>
{:else}
  <div class="flex h-full flex-col">
    <div
      class="flex items-center justify-between border-b border-border px-6 py-4"
    >
      <div>
        <div
          class="text-sm font-semibold uppercase tracking-[0.2em] text-foreground/40"
        >
          {session?.name ?? "Session"}
        </div>
        <div class="text-xs text-foreground/60">
          Status: {session?.status ?? "unknown"}
        </div>
      </div>

      <button
        class={`rounded-md border px-3 py-1 text-xs transition ${
          $showThinking
            ? "border-emerald-500/50 bg-emerald-500/15 text-emerald-100"
            : "border-border bg-background/50 text-foreground/70 hover:text-foreground"
        }`}
        type="button"
        onclick={() => showThinking.update((value: boolean) => !value)}
      >
        {$showThinking ? "Hide thinking" : "Show thinking"}
      </button>
    </div>

    <div class="flex-1 space-y-3 overflow-auto px-6 py-4">
      {#if renderItems.length === 0}
        <div
          class="rounded-lg border border-dashed border-border/70 bg-background/35 px-4 py-6 text-sm text-foreground/55"
        >
          No activity yet
        </div>
      {:else}
        {#each renderItems as item}
          {#if item.kind === "tool_call"}
            <ToolCallBlock
              toolName={item.value.call.data.tool_name}
              args={item.value.call.data.args}
              result={item.value.result?.result}
              timestamp={item.value.call.data.timestamp}
            />
          {:else if item.value.type === "message"}
            <div
              class="rounded-lg border border-border/70 bg-background/45 px-4 py-3 text-sm text-foreground/85"
            >
              <pre class="whitespace-pre-wrap">{item.value.data.content}</pre>
            </div>
          {:else if item.value.type === "thinking"}
            <div
              class="rounded-lg border border-border/50 bg-background/30 px-4 py-3 text-xs italic text-foreground/55"
            >
              {item.value.data.content}
            </div>
          {:else if item.value.type === "status"}
            <div class="text-xs uppercase tracking-[0.16em] text-foreground/45">
              {item.value.data.status}
            </div>
          {:else if item.value.type === "error"}
            <div
              class="rounded-lg border border-destructive/45 bg-destructive/10 px-4 py-3 text-sm text-destructive"
            >
              {item.value.data.error}
            </div>
          {:else if item.value.type === "tool_result"}
            <div
              class="rounded-lg border border-border/60 bg-background/30 px-4 py-3 text-xs text-foreground/65"
            >
              <pre class="whitespace-pre-wrap">{JSON.stringify(
                  item.value.data.result,
                  null,
                  2,
                )}</pre>
            </div>
          {/if}
        {/each}
      {/if}
    </div>
  </div>
{/if}

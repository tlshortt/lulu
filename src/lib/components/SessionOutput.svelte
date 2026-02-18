<script lang="ts">
  import ToolCallBlock from "$lib/components/ToolCallBlock.svelte";
  import {
    activeSessionId,
    interruptSession,
    resumeSession,
    sessionDebug,
    sessionErrors,
    sessionEvents,
    sessionOperations,
    sessions,
    showThinking,
  } from "$lib/stores/sessions";
  import type { SessionEvent, ToolResultEventData } from "$lib/types/session";

  const { sessionId = null } = $props<{ sessionId?: string | null }>();

  let resumePrompt = $state("");

  const normalizeStatus = (status: string) => {
    const normalized = status.toLowerCase();

    if (normalized === "complete" || normalized === "done") {
      return "completed";
    }

    if (normalized === "error") {
      return "failed";
    }

    return normalized;
  };

  const canInterruptStatus = (status: string) => {
    const normalized = normalizeStatus(status);
    return (
      normalized === "starting" ||
      normalized === "running" ||
      normalized === "resuming"
    );
  };

  const canResumeStatus = (status: string) => {
    const normalized = normalizeStatus(status);
    return normalized === "completed" || normalized === "interrupted";
  };

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
  const hiddenThinkingCount = $derived(
    events.filter((event: SessionEvent) => event.type === "thinking").length,
  );
  const debug = $derived(
    currentSessionId
      ? ($sessionDebug[currentSessionId] ?? {
          stderrTail: [],
          updatedAt: "",
        })
      : null,
  );
  const currentOperation = $derived(
    currentSessionId ? ($sessionOperations[currentSessionId] ?? null) : null,
  );
  const currentError = $derived(
    currentSessionId ? ($sessionErrors[currentSessionId] ?? null) : null,
  );
  const canInterrupt = $derived(canInterruptStatus(session?.status ?? ""));
  const canResume = $derived(canResumeStatus(session?.status ?? ""));
  const resumePromptReady = $derived(resumePrompt.trim().length > 0);

  const handleInterrupt = async () => {
    if (!currentSessionId) {
      return;
    }

    const confirmed = window.confirm("Interrupt session?");
    if (!confirmed) {
      return;
    }

    try {
      await interruptSession(currentSessionId);
    } catch (error) {
      console.error("Failed to interrupt session", error);
    }
  };

  const handleResume = async () => {
    if (!currentSessionId) {
      return;
    }

    try {
      await resumeSession(currentSessionId, resumePrompt);
      resumePrompt = "";
    } catch (error) {
      console.error("Failed to resume session", error);
    }
  };
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
  <div class="flex h-full min-w-0 flex-col">
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

      <div class="flex items-center gap-2">
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

        {#if canInterrupt}
          <button
            class="rounded-md border border-amber-500/40 bg-amber-500/10 px-3 py-1 text-xs uppercase tracking-[0.08em] text-amber-100 transition hover:bg-amber-500/20 disabled:cursor-not-allowed disabled:opacity-50"
            type="button"
            disabled={Boolean(currentOperation)}
            onclick={handleInterrupt}
          >
            {currentOperation === "interrupting"
              ? "Interrupting..."
              : "Interrupt"}
          </button>
        {/if}

        {#if canResume}
          <input
            class="w-52 rounded-md border border-border bg-background/50 px-2 py-1 text-xs text-foreground outline-none focus:border-ring disabled:cursor-not-allowed disabled:opacity-60"
            type="text"
            bind:value={resumePrompt}
            placeholder="Continue this session..."
            disabled={Boolean(currentOperation)}
            onkeydown={(event) => {
              if (event.key === "Enter") {
                event.preventDefault();
                void handleResume();
              }
            }}
          />
          <button
            class="rounded-md border border-emerald-500/35 bg-emerald-500/10 px-3 py-1 text-xs uppercase tracking-[0.08em] text-emerald-100 transition hover:bg-emerald-500/20 disabled:cursor-not-allowed disabled:opacity-50"
            type="button"
            disabled={Boolean(currentOperation) || !resumePromptReady}
            onclick={handleResume}
          >
            {currentOperation === "resuming" ? "Resuming..." : "Resume"}
          </button>
        {/if}
      </div>
    </div>

    {#if currentError}
      <div
        class="mx-6 mt-3 rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-xs text-destructive"
      >
        {currentError}
      </div>
    {/if}

    <div class="flex-1 space-y-3 overflow-y-auto overflow-x-hidden px-6 py-4">
      {#if currentSessionId && debug}
        <details
          class="rounded-lg border border-border/70 bg-background/35 px-4 py-3 text-xs text-foreground/70"
        >
          <summary
            class="cursor-pointer select-none font-semibold uppercase tracking-[0.14em] text-foreground/55"
          >
            Debug: spawn args + stderr tail
          </summary>
          <div class="mt-3 space-y-2">
            <div>
              <span class="font-semibold text-foreground/65">CLI:</span>
              <span class="ml-2 break-all font-mono"
                >{debug.cliPath ?? "(unknown)"}</span
              >
            </div>
            <div>
              <span class="font-semibold text-foreground/65">Working dir:</span>
              <span class="ml-2 break-all font-mono"
                >{debug.workingDir ?? "(unknown)"}</span
              >
            </div>
            <div>
              <span class="font-semibold text-foreground/65">Args:</span>
              <pre
                class="mt-1 max-w-full whitespace-pre-wrap break-words rounded border border-border/50 bg-background/45 px-2 py-1 font-mono">{(
                  debug.args ?? []
                ).join(" ") || "(none)"}</pre>
            </div>
            <div>
              <span class="font-semibold text-foreground/65">stderr tail:</span>
              {#if debug.stderrTail.length === 0}
                <div class="mt-1 text-foreground/50">No stderr output yet.</div>
              {:else}
                <pre
                  class="mt-1 max-h-40 max-w-full overflow-y-auto overflow-x-hidden whitespace-pre-wrap break-words rounded border border-border/50 bg-background/45 px-2 py-1 font-mono">{debug.stderrTail.join(
                    "\n",
                  )}</pre>
              {/if}
            </div>
          </div>
        </details>
      {/if}

      {#if renderItems.length === 0}
        <div
          class="rounded-lg border border-dashed border-border/70 bg-background/35 px-4 py-6 text-sm text-foreground/55"
        >
          {#if session?.status === "running"}
            {#if !$showThinking && hiddenThinkingCount > 0}
              No visible output yet. Click "Show thinking" to view reasoning.
            {:else}
              Waiting for first output...
            {/if}
          {:else}
            No activity yet
          {/if}
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
              <pre class="max-w-full whitespace-pre-wrap break-words">{item
                  .value.data.content}</pre>
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
              <pre
                class="max-w-full whitespace-pre-wrap break-words">{JSON.stringify(
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

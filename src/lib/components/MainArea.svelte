<script lang="ts">
  import SessionOutput from "$lib/components/SessionOutput.svelte";
  import {
    activeSessionId,
    dashboardSelectedSessionId,
    initialSessionsHydrated,
    sessions,
  } from "$lib/stores/sessions";
</script>

<section class="flex h-full flex-1 flex-col bg-background text-foreground">
  {#if !$initialSessionsHydrated}
    <div
      class="flex h-full flex-col items-center justify-center gap-4 px-8 text-center"
    >
      <div
        class="text-sm font-semibold uppercase tracking-[0.2em] text-foreground/40"
      >
        Sessions
      </div>
      <div class="text-2xl font-semibold">Loading sessions...</div>
      <p class="max-w-md text-sm text-foreground/60">
        Preparing your dashboard and syncing the latest session state.
      </p>
    </div>
  {:else if $sessions.length === 0}
    <div
      class="flex h-full flex-col items-center justify-center gap-4 px-8 text-center"
    >
      <div
        class="text-sm font-semibold uppercase tracking-[0.2em] text-foreground/40"
      >
        Sessions
      </div>
      <div class="text-2xl font-semibold">No active sessions</div>
      <p class="max-w-md text-sm text-foreground/60">
        Start a Claude Code session to see live output here. This space will
        render streaming logs, tool calls, and status updates in real time.
      </p>
      <div
        class="rounded-md border border-border bg-sidebar/60 px-4 py-3 font-mono text-xs text-foreground/70"
      >
        Press <span class="text-foreground">⌘ + N</span> to start a new session
      </div>
    </div>
  {:else if !$activeSessionId}
    <div
      class="flex h-full flex-col items-center justify-center gap-3 px-8 text-center"
    >
      <div
        class="text-sm font-semibold uppercase tracking-[0.2em] text-foreground/40"
      >
        Sessions
      </div>
      <div class="text-xl font-semibold">
        {#if $dashboardSelectedSessionId}
          Double-click to open selected session output
        {:else}
          Pick a session to view output
        {/if}
      </div>
      <p class="max-w-md text-sm text-foreground/60">
        {#if $dashboardSelectedSessionId}
          Single click selects a dashboard row. Double-click that row to open
          the full session stream here.
        {:else}
          Select a session from the sidebar to inspect events for that session
          only.
        {/if}
      </p>
    </div>
  {:else}
    <SessionOutput />
  {/if}
</section>

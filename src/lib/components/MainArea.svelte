<script lang="ts">
  import SessionOutput from "$lib/components/SessionOutput.svelte";
  import { get } from "svelte/store";
  import {
    activeSessionId,
    bootstrapInitialSessions,
    dashboardSelectedSessionId,
    initialSessionsLoadError,
    initialSessionsHydrated,
    initialSessionsRetryError,
    spawnRuntimeDiagnostics,
    sessions,
  } from "$lib/stores/sessions";

  const HYDRATION_STALL_MS = 4000;

  const showHydrationGate = $derived(
    !$initialSessionsHydrated && $sessions.length === 0 && !$activeSessionId,
  );
  const showStartupView = $derived(
    $initialSessionsHydrated && $sessions.length === 0 && !$activeSessionId,
  );
  const showSelectionHint = $derived(
    $initialSessionsHydrated && $sessions.length > 0 && !$activeSessionId,
  );
  const mainView = $derived(
    showHydrationGate
      ? "hydration-gate"
      : showStartupView
        ? "startup-view"
        : showSelectionHint
          ? "selection-hint"
          : "session-output",
  );
  const activeSessionExists = $derived(
    $activeSessionId
      ? $sessions.some((session) => session.id === $activeSessionId)
      : false,
  );
  const liveHasSpawnSession = $derived(
    $spawnRuntimeDiagnostics?.session_id
      ? $sessions.some(
          (session) => session.id === $spawnRuntimeDiagnostics?.session_id,
        )
      : false,
  );
  const SPAWN_DEBUG_MINIMIZED_STORAGE_KEY = "lulu:spawn-debug-minimized";
  const loadSpawnDebugMinimized = () => {
    if (typeof window === "undefined") {
      return false;
    }

    return (
      window.localStorage.getItem(SPAWN_DEBUG_MINIMIZED_STORAGE_KEY) === "true"
    );
  };

  let spawnDebugMinimized = $state(loadSpawnDebugMinimized());
  let hydrationStalled = $state(false);
  let hydrationErrorCopied = $state(false);
  let hydrationErrorCopyFailed = $state(false);

  const HYDRATION_COPY_FEEDBACK_MS = 2000;

  const resolveHydrationError = () =>
    get(initialSessionsRetryError) ?? get(initialSessionsLoadError);

  const copyHydrationError = async () => {
    hydrationErrorCopied = false;
    hydrationErrorCopyFailed = false;

    const errorMessage = resolveHydrationError();
    if (!errorMessage) {
      return;
    }

    if (
      typeof window === "undefined" ||
      !window.navigator?.clipboard?.writeText
    ) {
      hydrationErrorCopyFailed = true;
      return;
    }

    try {
      await window.navigator.clipboard.writeText(errorMessage);
      hydrationErrorCopied = true;
      window.setTimeout(() => {
        hydrationErrorCopied = false;
      }, HYDRATION_COPY_FEEDBACK_MS);
    } catch (error) {
      hydrationErrorCopyFailed = true;
      console.error("Failed to copy hydration error", error);
    }
  };

  const retryInitialSessionLoad = async () => {
    hydrationStalled = false;

    try {
      await bootstrapInitialSessions();
    } catch (error) {
      console.error("Failed to refresh initial sessions", error);
    }
  };

  $effect(() => {
    if (typeof window === "undefined") {
      return;
    }

    if (!showHydrationGate) {
      hydrationStalled = false;
      return;
    }

    hydrationStalled = false;
    const timer = window.setTimeout(() => {
      hydrationStalled = true;
    }, HYDRATION_STALL_MS);

    return () => {
      window.clearTimeout(timer);
    };
  });

  $effect(() => {
    if (typeof window === "undefined") {
      return;
    }

    window.localStorage.setItem(
      SPAWN_DEBUG_MINIMIZED_STORAGE_KEY,
      String(spawnDebugMinimized),
    );
  });
</script>

<section
  class="flex h-full min-w-0 flex-1 flex-col bg-background text-foreground"
>
  {#if showHydrationGate}
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
      {#if hydrationStalled}
        <div
          class="max-w-lg rounded-md border border-border bg-sidebar/50 px-4 py-3 text-left"
        >
          <div
            class="text-xs font-semibold uppercase tracking-[0.12em] text-foreground/55"
          >
            Still waiting on session sync
          </div>
          <p class="mt-1 text-sm text-foreground/70">
            This usually means the backend session list is still booting. You
            can retry now or start a new session from the sidebar.
          </p>
          {#if $initialSessionsRetryError}
            <div
              class="mt-2 rounded border border-destructive/40 bg-destructive/10 px-3 py-2 font-mono text-xs text-destructive"
            >
              Last backend error: {$initialSessionsRetryError}
            </div>
          {/if}
          {#if $initialSessionsLoadError}
            <div
              class="mt-2 rounded border border-destructive/40 bg-destructive/10 px-3 py-2 font-mono text-xs text-destructive"
            >
              Initial load failed: {$initialSessionsLoadError}
            </div>
          {/if}
          {#if $initialSessionsRetryError || $initialSessionsLoadError}
            <div class="mt-2 flex items-center gap-2">
              <button
                class="rounded border border-border px-3 py-1.5 text-xs font-semibold uppercase tracking-[0.08em] text-foreground transition hover:bg-background/60"
                type="button"
                onclick={() => {
                  void copyHydrationError();
                }}
              >
                Copy error
              </button>
              {#if hydrationErrorCopied}
                <span class="text-xs text-emerald-300">Copied</span>
              {:else if hydrationErrorCopyFailed}
                <span class="text-xs text-destructive">Copy failed</span>
              {/if}
            </div>
          {/if}
          <button
            class="mt-3 rounded border border-border px-3 py-1.5 text-xs font-semibold uppercase tracking-[0.08em] text-foreground transition hover:bg-background/60"
            type="button"
            onclick={() => {
              void retryInitialSessionLoad();
            }}
          >
            Retry sync
          </button>
        </div>
      {/if}
    </div>
  {:else if showStartupView}
    <div
      class="flex h-full flex-col items-center justify-center gap-4 px-8 text-center"
    >
      <div
        class="text-sm font-semibold uppercase tracking-[0.2em] text-foreground/40"
      >
        Sessions
      </div>
      <div class="text-2xl font-semibold">No active sessions</div>
      {#if $initialSessionsLoadError}
        <p class="max-w-md text-sm text-destructive">
          {$initialSessionsLoadError}
        </p>
      {:else}
        <p class="max-w-md text-sm text-foreground/60">
          Start a Claude Code session to see live output here. This space will
          render streaming logs, tool calls, and status updates in real time.
        </p>
      {/if}
      <div
        class="rounded-md border border-border bg-sidebar/60 px-4 py-3 font-mono text-xs text-foreground/70"
      >
        Press <span class="text-foreground">⌘ + N</span> to start a new session
      </div>
    </div>
  {:else if showSelectionHint}
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

{#if $spawnRuntimeDiagnostics}
  <div
    class="fixed bottom-4 right-4 z-40 max-w-md rounded-md border border-border bg-background/95 px-3 py-2 text-xs text-foreground/75 shadow-xl"
  >
    <div class="flex items-center justify-between gap-3">
      <div class="font-semibold uppercase tracking-[0.08em] text-foreground/55">
        Last spawn debug
      </div>
      <button
        class="rounded border border-border/70 bg-background/60 px-2 py-0.5 font-mono text-[10px] text-foreground/75 transition hover:text-foreground"
        type="button"
        onclick={() => {
          spawnDebugMinimized = !spawnDebugMinimized;
        }}
      >
        {spawnDebugMinimized ? "Expand" : "Minimize"}
      </button>
    </div>

    {#if spawnDebugMinimized}
      <div class="mt-1 font-mono">
        outcome={$spawnRuntimeDiagnostics.outcome} id={$spawnRuntimeDiagnostics.session_id ??
          "(none)"}
      </div>
    {:else}
      <div class="mt-1 font-mono">
        outcome={$spawnRuntimeDiagnostics.outcome} id={$spawnRuntimeDiagnostics.session_id ??
          "(none)"}
      </div>
      <div class="font-mono">view={mainView}</div>
      <div class="font-mono">
        live: sessions={$sessions.length} active={$activeSessionId ?? "(none)"} selected={$dashboardSelectedSessionId ??
          "(none)"}
      </div>
      <div class="font-mono">
        live_has_session={liveHasSpawnSession ? "yes" : "no"} refresh={$spawnRuntimeDiagnostics.refresh_status}
      </div>
      <div class="font-mono">
        active_exists={activeSessionExists ? "yes" : "no"} hydrated={$initialSessionsHydrated
          ? "yes"
          : "no"}
      </div>
    {/if}
  </div>
{/if}

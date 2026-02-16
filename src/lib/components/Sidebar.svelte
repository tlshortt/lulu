<script lang="ts">
  import { tick } from "svelte";
  import { Button } from "$lib/components/ui/button";
  import {
    activeSessionId,
    cliPathOverride,
    dashboardRows,
    dashboardSelectedSessionId,
    initialSessionsLoadError,
    initialSessionsHydrated,
    loadSessionHistory,
    renameSession,
    removeSession,
    sessions,
  } from "$lib/stores/sessions";
  import type { Session } from "$lib/stores/sessions";
  import type { DashboardStatus } from "$lib/types/session";

  const { onNewSession = () => {} } = $props<{ onNewSession?: () => void }>();

  const handleNewSession = () => {
    onNewSession();
  };

  const selectSession = (id: string, status: DashboardStatus) => {
    dashboardSelectedSessionId.set(id);

    if (status !== "Running") {
      void loadSessionHistory(id);
    }
  };

  const openSession = (id: string, status: DashboardStatus) => {
    dashboardSelectedSessionId.set(id);
    activeSessionId.set(id);

    if (status !== "Running") {
      void loadSessionHistory(id);
    }
  };

  const handleRemoveSession = async (
    sessionId: string,
    status: string,
    sessionName: string,
  ) => {
    if (status === "running") {
      const confirmed = window.confirm(
        `Kill and delete session "${sessionName}"?`,
      );

      if (!confirmed) {
        return;
      }
    }

    try {
      await removeSession(sessionId, status);
    } catch (error) {
      console.error("Failed to remove session", error);
    }
  };

  const statusBadgeClass = (status: DashboardStatus) => {
    if (status === "Running") {
      return "border-amber-400/45 bg-amber-400/10 text-amber-200";
    }

    if (status === "Completed") {
      return "border-emerald-500/40 bg-emerald-500/10 text-emerald-200";
    }

    if (status === "Failed") {
      return "border-destructive/45 bg-destructive/15 text-destructive";
    }

    return "border-amber-400/45 bg-amber-400/10 text-amber-200";
  };

  const rawStatusesBySessionId = $derived(
    new Map($sessions.map((session) => [session.id, session.status])),
  );

  const sessionsById = $derived(
    new Map($sessions.map((session) => [session.id, session])),
  );

  let editingSessionId = $state<string | null>(null);
  let editingName = $state("");
  let renameInput = $state<HTMLInputElement | null>(null);

  const startRename = (session: Session) => {
    editingSessionId = session.id;
    editingName = session.name;
    void tick().then(() => {
      renameInput?.focus();
      renameInput?.select();
    });
  };

  const cancelRename = () => {
    editingSessionId = null;
    editingName = "";
  };

  const commitRename = async (session: Session) => {
    if (editingSessionId !== session.id) {
      return;
    }

    const nextName = editingName.trim();
    if (!nextName || nextName === session.name) {
      cancelRename();
      return;
    }

    try {
      await renameSession(session.id, nextName);
    } catch (error) {
      console.error("Failed to rename session", error);
    } finally {
      cancelRename();
    }
  };
</script>

<aside
  class="flex h-full min-h-0 w-full flex-col border-r border-border bg-sidebar text-foreground"
>
  <div class="px-6 py-5">
    <div class="text-lg font-semibold tracking-tight">Lulu</div>
    <div class="text-xs text-foreground/50">Tracks all the claudes</div>
  </div>

  <div class="px-4 pb-3">
    <label
      class="block text-[11px] uppercase tracking-[0.18em] text-foreground/45"
    >
      CLI Path Override
      <input
        class="mt-2 w-full rounded-md border border-border bg-background/40 px-2 py-2 text-xs text-foreground outline-none focus:border-ring"
        value={$cliPathOverride}
        placeholder="/Users/you/.claude/bin/claude"
        oninput={(event) =>
          cliPathOverride.set((event.currentTarget as HTMLInputElement).value)}
      />
    </label>
  </div>

  <div class="min-h-0 flex-1 overflow-auto px-4">
    {#if !$initialSessionsHydrated}
      <div class="space-y-3 pb-6 text-sm text-foreground/60">
        <div
          class="rounded-md border border-border bg-background/40 px-3 py-2 font-mono"
        >
          Loading sessions...
        </div>
      </div>
    {:else if $sessions.length === 0}
      <div class="space-y-3 pb-6 text-sm text-foreground/60">
        <div
          class="rounded-md border border-border bg-background/40 px-3 py-2 font-mono"
        >
          No sessions yet
        </div>
        <div class="text-xs text-foreground/40">
          Launch a session to see it here.
        </div>
        {#if $initialSessionsLoadError}
          <div
            class="rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-xs text-destructive"
          >
            {$initialSessionsLoadError}
          </div>
        {/if}
      </div>
    {:else}
      <ul class="space-y-2 pb-6">
        {#each $dashboardRows as row (row.id)}
          <li>
            <div
              class={`relative rounded-md border border-border px-3 py-2 text-left text-sm transition ${
                $dashboardSelectedSessionId === row.id
                  ? "bg-background/70 text-foreground"
                  : "bg-background/30 text-foreground/70 hover:bg-background/50"
              }`}
            >
              <button
                class="flex w-full flex-col gap-2 pr-6 text-left"
                onclick={() => selectSession(row.id, row.status)}
                ondblclick={() => openSession(row.id, row.status)}
                type="button"
              >
                <span class="flex items-center justify-between gap-3">
                  {#if editingSessionId === row.id}
                    <input
                      bind:this={renameInput}
                      class="w-full rounded border border-border bg-background/55 px-1.5 py-1 text-sm font-medium text-foreground outline-none focus:border-ring"
                      bind:value={editingName}
                      aria-label="Rename session"
                      onblur={() => {
                        const session = sessionsById.get(row.id);
                        if (session) {
                          void commitRename(session);
                        }
                      }}
                      onkeydown={(event) => {
                        if (event.key === "Enter") {
                          event.preventDefault();
                          const session = sessionsById.get(row.id);
                          if (session) {
                            void commitRename(session);
                          }
                        }

                        if (event.key === "Escape") {
                          event.preventDefault();
                          cancelRename();
                        }

                        event.stopPropagation();
                      }}
                      onclick={(event) => event.stopPropagation()}
                    />
                  {:else}
                    <span class="truncate font-medium">{row.name}</span>
                  {/if}
                  <span class="shrink-0 text-xs text-foreground/45"
                    >{row.recentActivity}</span
                  >
                </span>
                <span class="flex items-center gap-2">
                  <span
                    class={`inline-flex items-center gap-1 rounded-full border px-2 py-0.5 text-[11px] font-semibold uppercase tracking-[0.08em] ${statusBadgeClass(
                      row.status,
                    )}`}
                  >
                    {#if row.status === "Running"}
                      <span
                        class="h-1.5 w-1.5 animate-pulse rounded-full bg-amber-300"
                      ></span>
                    {/if}
                    {row.status}
                  </span>
                  {#if row.status === "Failed" && row.failureReason}
                    <span class="truncate text-xs text-foreground/55"
                      >{row.failureReason}</span
                    >
                  {/if}
                </span>
              </button>
              {#if editingSessionId !== row.id}
                <button
                  class="absolute right-7 top-2 rounded px-1 text-xs text-foreground/40 transition hover:text-foreground"
                  aria-label={`Rename ${row.name}`}
                  title="Rename"
                  type="button"
                  onclick={(event) => {
                    event.stopPropagation();
                    const session = sessionsById.get(row.id);
                    if (session) {
                      startRename(session);
                    }
                  }}>✎</button
                >
              {/if}
              <button
                class="absolute right-2 top-2 rounded px-1 text-xs text-foreground/40 transition hover:text-destructive"
                aria-label={rawStatusesBySessionId.get(row.id) === "running"
                  ? `Kill and delete ${row.name}`
                  : `Delete ${row.name}`}
                title={rawStatusesBySessionId.get(row.id) === "running"
                  ? "Kill and delete"
                  : "Delete"}
                type="button"
                onclick={(event) => {
                  event.stopPropagation();
                  void handleRemoveSession(
                    row.id,
                    rawStatusesBySessionId.get(row.id) ?? "running",
                    row.name,
                  );
                }}>×</button
              >
            </div>
          </li>
        {/each}
      </ul>
    {/if}
  </div>

  <div class="border-t border-border/70 bg-sidebar p-4">
    <Button class="w-full" variant="secondary" onclick={handleNewSession}
      >New Session</Button
    >
  </div>
</aside>

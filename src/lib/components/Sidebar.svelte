<script lang="ts">
  import { tick } from "svelte";
  import { Button } from "$lib/components/ui/button";
  import {
    activeSessionId,
    cliPathOverride,
    loadSessionHistory,
    renameSession,
    removeSession,
    sessions,
  } from "$lib/stores/sessions";
  import type { Session } from "$lib/stores/sessions";

  const { onNewSession = () => {} } = $props<{ onNewSession?: () => void }>();

  const handleNewSession = () => {
    onNewSession();
  };

  const selectSession = (id: string, status: string) => {
    activeSessionId.set(id);

    if (status !== "running") {
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
    {#if $sessions.length === 0}
      <div class="space-y-3 pb-6 text-sm text-foreground/60">
        <div
          class="rounded-md border border-border bg-background/40 px-3 py-2 font-mono"
        >
          No sessions yet
        </div>
        <div class="text-xs text-foreground/40">
          Launch a session to see it here.
        </div>
      </div>
    {:else}
      <ul class="space-y-2 pb-6">
        {#each $sessions as session (session.id)}
          <li>
            <div
              class={`relative rounded-md border border-border px-3 py-2 text-left text-sm transition ${
                $activeSessionId === session.id
                  ? "bg-background/70 text-foreground"
                  : "bg-background/30 text-foreground/70 hover:bg-background/50"
              }`}
            >
              <button
                class="flex w-full flex-col gap-1 pr-6 text-left"
                onclick={() => selectSession(session.id, session.status)}
                ondblclick={() => startRename(session)}
                type="button"
              >
                {#if editingSessionId === session.id}
                  <input
                    bind:this={renameInput}
                    class="rounded border border-border bg-background/55 px-1.5 py-1 text-base font-medium text-foreground outline-none focus:border-ring"
                    bind:value={editingName}
                    onblur={() => {
                      void commitRename(session);
                    }}
                    onkeydown={(event) => {
                      if (event.key === "Enter") {
                        event.preventDefault();
                        void commitRename(session);
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
                  <span class="font-medium">{session.name}</span>
                {/if}
                <span
                  class="text-xs uppercase tracking-[0.2em] text-foreground/40"
                >
                  {session.status}
                </span>
              </button>
              <button
                class="absolute right-2 top-2 rounded px-1 text-xs text-foreground/40 transition hover:text-destructive"
                aria-label={session.status === "running"
                  ? `Kill and delete ${session.name}`
                  : `Delete ${session.name}`}
                title={session.status === "running"
                  ? "Kill and delete"
                  : "Delete"}
                type="button"
                onclick={(event) => {
                  event.stopPropagation();
                  void handleRemoveSession(
                    session.id,
                    session.status,
                    session.name,
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

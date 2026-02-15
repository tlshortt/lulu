<script lang="ts">
  import { Button } from "$lib/components/ui/button";
  import {
    activeSessionId,
    cliPathOverride,
    sessions,
  } from "$lib/stores/sessions";

  const { onNewSession = () => {} } = $props<{ onNewSession?: () => void }>();

  const handleNewSession = () => {
    onNewSession();
  };

  const selectSession = (id: string) => {
    activeSessionId.set(id);
  };
</script>

<aside
  class="flex h-full w-[280px] flex-col border-r border-border bg-sidebar text-foreground"
>
  <div class="px-6 py-5">
    <div class="text-lg font-semibold tracking-tight">Lulu</div>
    <div class="text-xs text-foreground/50">Mission Control</div>
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

  <div class="flex-1 overflow-auto px-4">
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
            <button
              class={`flex w-full flex-col gap-1 rounded-md border border-border px-3 py-2 text-left text-sm transition ${
                $activeSessionId === session.id
                  ? "bg-background/70 text-foreground"
                  : "bg-background/30 text-foreground/70 hover:bg-background/50"
              }`}
              onclick={() => selectSession(session.id)}
              type="button"
            >
              <span class="font-medium">{session.name}</span>
              <span
                class="text-xs uppercase tracking-[0.2em] text-foreground/40"
              >
                {session.status}
              </span>
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  </div>

  <div class="p-4">
    <Button class="w-full" variant="secondary" onclick={handleNewSession}
      >New Session</Button
    >
  </div>
</aside>

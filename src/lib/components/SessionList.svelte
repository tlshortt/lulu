<script lang="ts">
  import { sessions, selectedSessionId } from "$lib/stores/sessions";

  const selectSession = (id: string) => {
    selectedSessionId.set(id);
  };
</script>

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
            $selectedSessionId === session.id
              ? "bg-background/70 text-foreground"
              : "bg-background/30 text-foreground/70 hover:bg-background/50"
          }`}
          onclick={() => selectSession(session.id)}
          type="button"
        >
          <span class="font-medium">{session.name}</span>
          <span class="text-xs uppercase tracking-[0.2em] text-foreground/40">
            {session.status}
          </span>
        </button>
      </li>
    {/each}
  </ul>
{/if}

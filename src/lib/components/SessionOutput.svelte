<script lang="ts">
  import { sessionOutputs, sessions } from "$lib/stores/sessions";

  const { sessionId = null } = $props<{ sessionId?: string | null }>();

  const output = $derived(sessionId ? $sessionOutputs[sessionId] ?? "" : "");
  const session = $derived(
    sessionId ? $sessions.find((item) => item.id === sessionId) : null
  );
</script>

{#if !sessionId}
  <div class="flex h-full flex-col items-center justify-center gap-4 px-8 text-center">
    <div class="text-sm font-semibold uppercase tracking-[0.2em] text-foreground/40">
      Sessions
    </div>
    <div class="text-2xl font-semibold">Select a session</div>
    <p class="max-w-md text-sm text-foreground/60">
      Choose a running session from the sidebar to see streaming output here.
    </p>
  </div>
{:else}
  <div class="flex h-full flex-col">
    <div class="border-b border-border px-6 py-4">
      <div class="text-sm font-semibold uppercase tracking-[0.2em] text-foreground/40">
        {session?.name ?? "Session"}
      </div>
      <div class="text-xs text-foreground/60">Status: {session?.status ?? "unknown"}</div>
    </div>
    <div class="flex-1 overflow-auto px-6 py-4">
      <pre class="whitespace-pre-wrap text-sm text-foreground/80">{output || "Waiting for output..."}</pre>
    </div>
  </div>
{/if}

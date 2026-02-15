<script lang="ts">
  const {
    toolName,
    args,
    result = null,
    timestamp = "",
  } = $props<{
    toolName: string;
    args: unknown;
    result?: unknown;
    timestamp?: string;
  }>();

  const format = (value: unknown) => {
    if (value === null || value === undefined) {
      return "-";
    }

    if (typeof value === "string") {
      return value;
    }

    try {
      return JSON.stringify(value, null, 2);
    } catch {
      return String(value);
    }
  };
</script>

<details
  class="rounded-lg border border-border/70 bg-background/50 p-3 text-sm"
>
  <summary
    class="cursor-pointer select-none font-mono text-xs text-foreground/80"
  >
    Tool: {toolName}
    {#if timestamp}
      <span class="ml-2 text-foreground/40"
        >{new Date(timestamp).toLocaleTimeString()}</span
      >
    {/if}
  </summary>

  <div class="mt-3 space-y-3">
    <div>
      <div
        class="mb-1 text-[11px] uppercase tracking-[0.18em] text-foreground/45"
      >
        Args
      </div>
      <pre
        class="overflow-x-auto rounded-md border border-border/50 bg-background/60 p-2 text-xs">{format(
          args,
        )}</pre>
    </div>

    {#if result !== null && result !== undefined}
      <div>
        <div
          class="mb-1 text-[11px] uppercase tracking-[0.18em] text-foreground/45"
        >
          Result
        </div>
        <pre
          class="overflow-x-auto rounded-md border border-border/50 bg-background/60 p-2 text-xs">{format(
            result,
          )}</pre>
      </div>
    {/if}
  </div>
</details>

<script lang="ts">
  import { Button } from "$lib/components/ui/button";
  import { ScrollArea } from "$lib/components/ui/scroll-area";
</script>

<aside class="flex h-full w-[280px] flex-col border-r border-border bg-sidebar text-foreground">
  <div class="px-6 py-5">
    <div class="text-lg font-semibold tracking-tight">Lulu</div>
    <div class="text-xs text-foreground/50">Mission Control</div>
  </div>

  <ScrollArea class="flex-1 px-4">
    <div class="space-y-3 pb-6 text-sm text-foreground/60">
      <div class="rounded-md border border-border bg-background/40 px-3 py-2 font-mono">
        No sessions yet
      </div>
      <div class="text-xs text-foreground/40">Launch a session to see it here.</div>
    </div>
  </ScrollArea>

  <div class="p-4">
    <Button class="w-full" variant="secondary">New Session</Button>
  </div>
</aside>

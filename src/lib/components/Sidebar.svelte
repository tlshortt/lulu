<script lang="ts">
  import { Button } from "$lib/components/ui/button";
  import { ScrollArea } from "$lib/components/ui/scroll-area";
  import SessionList from "$lib/components/SessionList.svelte";
  const { onNewSession = () => {} } = $props<{ onNewSession?: () => void }>();

  const handleNewSession = () => {
    onNewSession();
  };
</script>

<aside
  class="flex h-full w-[280px] flex-col border-r border-border bg-sidebar text-foreground"
>
  <div class="px-6 py-5">
    <div class="text-lg font-semibold tracking-tight">Lulu</div>
    <div class="text-xs text-foreground/50">Mission Control</div>
  </div>

  <ScrollArea class="flex-1 px-4">
    <SessionList />
  </ScrollArea>

  <div class="p-4">
    <Button class="w-full" variant="secondary" onclick={handleNewSession}
      >New Session</Button
    >
  </div>
</aside>

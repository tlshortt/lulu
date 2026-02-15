<script lang="ts">
  import Sidebar from "$lib/components/Sidebar.svelte";
  import MainArea from "$lib/components/MainArea.svelte";
  import { onMount } from "svelte";
  import { initSessionListeners, loadSessions } from "$lib/stores/sessions";

  onMount(async () => {
    await initSessionListeners();
    await loadSessions();
  });
</script>

<div class="flex h-screen bg-background text-foreground">
  <Sidebar />
  <MainArea />
</div>

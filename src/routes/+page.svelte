<script lang="ts">
  import Sidebar from "$lib/components/Sidebar.svelte";
  import MainArea from "$lib/components/MainArea.svelte";
  import NewSessionModal from "$lib/components/NewSessionModal.svelte";
  import { initSessionListeners, loadSessions } from "$lib/stores/sessions";
  import { onMount } from "svelte";

  let newSessionOpen = $state(false);

  const openNewSession = () => {
    newSessionOpen = true;
  };

  const closeNewSession = () => {
    newSessionOpen = false;
  };

  onMount(() => {
    void initSessionListeners();
    void loadSessions();
  });
</script>

<div class="flex h-screen bg-background text-foreground">
  <Sidebar onNewSession={openNewSession} />
  <MainArea />
</div>

<NewSessionModal open={newSessionOpen} onClose={closeNewSession} />

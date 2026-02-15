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

    const handleShortcut = (event: KeyboardEvent) => {
      if (event.key.toLowerCase() !== "n") {
        return;
      }

      if (!event.metaKey && !event.ctrlKey) {
        return;
      }

      const target = event.target as HTMLElement | null;
      if (
        target instanceof HTMLInputElement ||
        target instanceof HTMLTextAreaElement ||
        target?.isContentEditable
      ) {
        return;
      }

      event.preventDefault();
      openNewSession();
    };

    window.addEventListener("keydown", handleShortcut);

    return () => {
      window.removeEventListener("keydown", handleShortcut);
    };
  });
</script>

<div class="flex h-screen bg-background text-foreground">
  <Sidebar onNewSession={openNewSession} />
  <MainArea />
</div>

<NewSessionModal open={newSessionOpen} onClose={closeNewSession} />

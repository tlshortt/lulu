<script lang="ts">
  import Sidebar from "$lib/components/Sidebar.svelte";
  import MainArea from "$lib/components/MainArea.svelte";
  import NewSessionModal from "$lib/components/NewSessionModal.svelte";
  import {
    bootstrapInitialSessions,
    beginInitialSessionsHydration,
    initSessionListeners,
  } from "$lib/stores/sessions";
  import { onMount } from "svelte";

  const SIDEBAR_STORAGE_KEY = "lulu:sidebar-width";
  const SIDEBAR_MIN_WIDTH = 240;
  const SIDEBAR_DEFAULT_WIDTH = 280;

  let newSessionOpen = $state(false);
  let sidebarWidth = $state(SIDEBAR_DEFAULT_WIDTH);
  let resizingSidebar = $state(false);

  beginInitialSessionsHydration();

  const canUseStorage = () => typeof window !== "undefined";

  const getSidebarMaxWidth = () =>
    typeof window === "undefined"
      ? SIDEBAR_DEFAULT_WIDTH
      : Math.floor(window.innerWidth * 0.4);

  const clampSidebarWidth = (width: number) =>
    Math.min(Math.max(width, SIDEBAR_MIN_WIDTH), getSidebarMaxWidth());

  const saveSidebarWidth = (width: number) => {
    if (!canUseStorage()) {
      return;
    }

    window.localStorage.setItem(SIDEBAR_STORAGE_KEY, String(width));
  };

  const loadSidebarWidth = () => {
    if (!canUseStorage()) {
      return SIDEBAR_DEFAULT_WIDTH;
    }

    const raw = window.localStorage.getItem(SIDEBAR_STORAGE_KEY);
    const parsed = raw ? Number.parseInt(raw, 10) : SIDEBAR_DEFAULT_WIDTH;
    if (!Number.isFinite(parsed)) {
      return SIDEBAR_DEFAULT_WIDTH;
    }

    return clampSidebarWidth(parsed);
  };

  const openNewSession = () => {
    newSessionOpen = true;
  };

  const closeNewSession = () => {
    newSessionOpen = false;
  };

  const startSidebarResize = (event: PointerEvent) => {
    event.preventDefault();
    resizingSidebar = true;
  };

  onMount(() => {
    sidebarWidth = loadSidebarWidth();

    void initSessionListeners().catch((error: unknown) => {
      console.error("Failed to initialize session listeners", error);
    });
    void bootstrapInitialSessions().catch((error: unknown) => {
      console.error("Failed to load sessions", error);
    });

    const handlePointerMove = (event: PointerEvent) => {
      if (!resizingSidebar) {
        return;
      }

      sidebarWidth = clampSidebarWidth(event.clientX);
    };

    const handlePointerUp = () => {
      if (!resizingSidebar) {
        return;
      }

      resizingSidebar = false;
      saveSidebarWidth(sidebarWidth);
    };

    const handleWindowResize = () => {
      sidebarWidth = clampSidebarWidth(sidebarWidth);
      saveSidebarWidth(sidebarWidth);
    };

    window.addEventListener("pointermove", handlePointerMove);
    window.addEventListener("pointerup", handlePointerUp);
    window.addEventListener("resize", handleWindowResize);

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
      window.removeEventListener("pointermove", handlePointerMove);
      window.removeEventListener("pointerup", handlePointerUp);
      window.removeEventListener("resize", handleWindowResize);
    };
  });
</script>

<div class="flex h-screen overflow-hidden bg-background text-foreground">
  <div class="relative h-full shrink-0" style={`width: ${sidebarWidth}px;`}>
    <Sidebar onNewSession={openNewSession} />
    <div
      class="group absolute inset-y-0 -right-1 z-10 w-2 cursor-col-resize"
      role="separator"
      aria-label="Resize sidebar"
      aria-orientation="vertical"
      onpointerdown={startSidebarResize}
    >
      <div
        class={`pointer-events-none absolute inset-y-3 left-1/2 w-px -translate-x-1/2 rounded-full bg-border/35 transition group-hover:bg-border/65 ${
          resizingSidebar ? "bg-border/80" : ""
        }`}
      ></div>
    </div>
  </div>
  <MainArea />
</div>

<NewSessionModal open={newSessionOpen} onClose={closeNewSession} />

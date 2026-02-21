<script lang="ts">
  import { spawnRuntimeDiagnostics, spawnSession } from "$lib/stores/sessions";

  const { open = false, onClose = () => {} } = $props<{
    open?: boolean;
    onClose?: () => void;
  }>();

  let name = $state("");
  let prompt = $state("");
  let workingDir = $state("~");
  let isSubmitting = $state(false);
  let error = $state<string | null>(null);

  const resetForm = () => {
    name = "";
    prompt = "";
    workingDir = "~";
    error = null;
  };

  const handleClose = () => {
    if (isSubmitting) return;
    resetForm();
    onClose();
  };

  const handleBackdropClick = (event: MouseEvent) => {
    if (event.currentTarget === event.target) {
      handleClose();
    }
  };

  const handleBackdropKey = (event: KeyboardEvent) => {
    if (event.target !== event.currentTarget) {
      return;
    }

    if (event.key === "Escape") {
      handleClose();
    }
  };

  const handleSubmit = async () => {
    if (isSubmitting) {
      return;
    }

    if (!name.trim() || !prompt.trim() || !workingDir.trim()) {
      error = "Please fill out all fields.";
      return;
    }

    isSubmitting = true;
    error = null;

    try {
      await spawnSession(name.trim(), prompt.trim(), workingDir.trim());
      resetForm();
      onClose();
    } catch (err) {
      const message =
        typeof err === "string"
          ? err
          : err instanceof Error
            ? err.message
            : "Failed to launch session. Verify your working directory and Claude CLI, then try again.";
      error = message;
    } finally {
      isSubmitting = false;
    }
  };

  const handleFormKeydown = (event: KeyboardEvent) => {
    if (event.key !== "Enter") {
      return;
    }

    if (event.target instanceof HTMLTextAreaElement) {
      return;
    }

    event.preventDefault();
    void handleSubmit();
  };
</script>

{#if open}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-6"
    role="button"
    tabindex="0"
    aria-label="Close new session"
    onclick={handleBackdropClick}
    onkeydown={handleBackdropKey}
  >
    <div
      class="w-full max-w-lg rounded-lg border border-border bg-background p-6 shadow-xl"
      role="dialog"
      aria-modal="true"
      aria-label="New Session"
    >
      <div class="flex items-start justify-between gap-6">
        <div>
          <div class="text-lg font-semibold">New Session</div>
          <div class="text-sm text-foreground/60">
            Launch a Claude Code session with a name, prompt, and working
            directory.
          </div>
        </div>
        <button
          class="text-sm text-foreground/60 transition hover:text-foreground"
          onclick={handleClose}
          type="button"
        >
          Close
        </button>
      </div>

      <form
        class="mt-6 space-y-4"
        onsubmit={(event) => {
          event.preventDefault();
          handleSubmit();
        }}
      >
        <label class="block text-sm font-medium">
          Session name
          <input
            class="mt-2 w-full rounded-md border border-border bg-background/40 px-3 py-2 text-sm text-foreground outline-none focus:border-ring"
            bind:value={name}
            placeholder="Design review"
            autocomplete="off"
            onkeydown={handleFormKeydown}
          />
        </label>

        <label class="block text-sm font-medium">
          Prompt
          <textarea
            class="mt-2 h-28 w-full rounded-md border border-border bg-background/40 px-3 py-2 text-sm text-foreground outline-none focus:border-ring"
            bind:value={prompt}
            placeholder="Summarize the latest changes and propose next steps."
          ></textarea>
        </label>

        <label class="block text-sm font-medium">
          Working directory
          <input
            class="mt-2 w-full rounded-md border border-border bg-background/40 px-3 py-2 text-sm text-foreground outline-none focus:border-ring"
            bind:value={workingDir}
            placeholder="~/workspace/project"
            autocomplete="off"
            onkeydown={handleFormKeydown}
          />
        </label>

        {#if error}
          <div
            class="rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive"
            role="alert"
            aria-live="assertive"
          >
            {error}
          </div>
        {/if}

        <details
          class="rounded-md border border-border bg-background/30 px-3 py-2 text-xs text-foreground/70"
        >
          <summary
            class="cursor-pointer select-none font-semibold uppercase tracking-[0.08em] text-foreground/60"
          >
            Runtime diagnostics
          </summary>
          {#if $spawnRuntimeDiagnostics}
            <div class="mt-2 space-y-1 font-mono">
              <div>Outcome: {$spawnRuntimeDiagnostics.outcome}</div>
              <div>
                Session ID: {$spawnRuntimeDiagnostics.session_id ?? "(none)"}
              </div>
              <div>Spawn: {$spawnRuntimeDiagnostics.spawn_duration_ms}ms</div>
              <div>
                Refresh: {$spawnRuntimeDiagnostics.refresh_duration_ms ?? 0}ms ({$spawnRuntimeDiagnostics.refresh_status})
              </div>
              <div>Total: {$spawnRuntimeDiagnostics.total_duration_ms}ms</div>
              <div>
                Store sessions: {$spawnRuntimeDiagnostics.sessions_count}
              </div>
              <div>
                Has session in store: {$spawnRuntimeDiagnostics.has_session_in_store
                  ? "yes"
                  : "no"}
              </div>
              <div>
                Active session: {$spawnRuntimeDiagnostics.active_session_id ??
                  "(none)"}
              </div>
              <div>
                Dashboard selected: {$spawnRuntimeDiagnostics.dashboard_selected_session_id ??
                  "(none)"}
              </div>
              {#if $spawnRuntimeDiagnostics.error_message}
                <div class="text-destructive">
                  Error: {$spawnRuntimeDiagnostics.error_message}
                </div>
              {/if}
            </div>
          {:else}
            <div class="mt-2 text-foreground/55">No attempts yet.</div>
          {/if}
        </details>

        <div class="flex items-center justify-end gap-3 pt-2">
          <button
            class="rounded-md border border-border px-4 py-2 text-sm text-foreground/70 transition hover:text-foreground"
            type="button"
            onclick={handleClose}
            disabled={isSubmitting}
          >
            Cancel
          </button>
          <button
            class="rounded-md bg-secondary px-4 py-2 text-sm font-semibold text-secondary-foreground transition hover:bg-secondary/80 disabled:opacity-60"
            type="submit"
            disabled={isSubmitting}
          >
            {isSubmitting ? "Starting..." : "Start session"}
          </button>
        </div>
      </form>
    </div>
  </div>
{/if}

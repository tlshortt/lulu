import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { writable } from "svelte/store";

export interface Session {
  id: string;
  name: string;
  status: string;
  working_dir: string;
  created_at: string;
  updated_at: string;
}

export interface SessionOutput {
  session_id: string;
  line: string;
}

export const sessions = writable<Session[]>([]);
export const sessionOutputs = writable<Record<string, string>>({});
export const selectedSessionId = writable<string | null>(null);

export async function loadSessions() {
  const data = await invoke<Session[]>("list_sessions");
  sessions.set(data);
  selectedSessionId.update((current) => current ?? data[0]?.id ?? null);
}

export async function spawnSession(
  name: string,
  prompt: string,
  workingDir: string,
) {
  const id = await invoke<string>("spawn_session", {
    name,
    prompt,
    workingDir,
  });
  await loadSessions();
  selectedSessionId.set(id);
  return id;
}

export async function initSessionListeners() {
  await listen<SessionOutput>("session-output", (event) => {
    const { session_id, line } = event.payload;
    sessionOutputs.update((outputs) => {
      const current = outputs[session_id] ?? "";
      return {
        ...outputs,
        [session_id]: `${current}${line}\n`,
      };
    });
  });

  await listen<string>("session-complete", async () => {
    await loadSessions();
  });

  await listen<string>("session-error", async () => {
    await loadSessions();
  });
}

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { AgentSource, AgentView } from "./types";

/** Gerçek kaynak: Rust tarafındaki helper köprüsü (src-tauri/src/supervisor.rs). */
export const AGENT_STATE_EVENT = "agent-state";

export function createTauriSource(): AgentSource {
  return {
    kind: "tauri",
    initial: () => invoke<AgentView>("get_agent_state"),
    subscribe: async (onChange) =>
      listen<AgentView>(AGENT_STATE_EVENT, (event) => {
        onChange(event.payload);
      }),
    refresh: () => invoke<undefined>("refresh_agent"),
  };
}

import type { AgentSource, AgentView } from "../agent/types";

export function baseView(overrides: Partial<AgentView> = {}): AgentView {
  return {
    helper: "running",
    helper_restarts: 0,
    link: "connecting",
    attempt: 0,
    retry_in_ms: null,
    link_reason: null,
    agent: null,
    last_success_ms: null,
    closing: false,
    updated_ms: 0,
    ...overrides,
  };
}

export function connectedView(overrides: Partial<AgentView> = {}): AgentView {
  return baseView({
    link: "connected",
    last_success_ms: 1_000,
    agent: {
      version: "0.1.0",
      run_mode: "SERVICE",
      uptime_secs: 125,
      connected_clients: 1,
      healthy: true,
      checks: [{ name: "ipc_listener", ok: true, detail: "accepting connections" }],
      round_trip_ms: 2,
    },
    ...overrides,
  });
}

/** Test kaynağı: olayları elle tetiklenir. Gerçek "tauri" türünü taklit eder. */
export function fakeSource(initial: AgentView) {
  let listener: ((v: AgentView) => void) | null = null;
  let refreshes = 0;
  const source: AgentSource = {
    kind: "tauri",
    initial: () => Promise.resolve(initial),
    subscribe: (cb) => {
      listener = cb;
      return Promise.resolve(() => {
        listener = null;
      });
    },
    refresh: () => {
      refreshes += 1;
      return Promise.resolve();
    },
  };
  return {
    source,
    push: (v: AgentView) => listener?.(v),
    refreshes: () => refreshes,
    subscribed: () => listener !== null,
  };
}

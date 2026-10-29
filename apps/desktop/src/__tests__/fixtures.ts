import type { AgentSource, AgentView, ScreenTimeStatus } from "../agent/types";

export function screenTime(overrides: Partial<ScreenTimeStatus> = {}): ScreenTimeStatus {
  return {
    day: "2026-10-29",
    state: "ACTIVE",
    today: {
      active_secs: 1500,
      idle_secs: 300,
      locked_secs: 0,
      suspended_secs: 0,
      unknown_secs: 0,
    },
    daily_limit_secs: 7200,
    remaining_secs: 5700,
    limit_reached: false,
    break_phase: "NORMAL",
    next_break_in_secs: 1200,
    notices: [],
    policy: {
      version: 0,
      source: "BUILTIN_DEFAULT",
      status: "OK",
      daily_limit_secs: 7200,
      break_interval_secs: 1800,
      break_duration_secs: 300,
      first_warning_secs: 300,
      final_warning_secs: 60,
      idle_threshold_secs: 300,
    },
    measurement: { source: "SESSION_HELPER", last_report_age_ms: 800 },
    storage: { persistent: true, schema_version: 1, recovered: false },
    clock_adjusted_today: false,
    computed_at_ms: 1_000,
    ...overrides,
  };
}

export function baseView(overrides: Partial<AgentView> = {}): AgentView {
  return {
    helper: "running",
    helper_restarts: 0,
    link: "connecting",
    attempt: 0,
    retry_in_ms: null,
    link_reason: null,
    agent: null,
    screen_time: null,
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

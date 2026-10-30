import type { AgentSource, AgentView, ScreenTimeStatus } from "./types";

function demoScreenTime(at: number): ScreenTimeStatus {
  return {
    day: "demo",
    state: "ACTIVE",
    today: {
      active_secs: 3180,
      idle_secs: 420,
      locked_secs: 0,
      suspended_secs: 0,
      unknown_secs: 0,
    },
    daily_limit_secs: 7200,
    remaining_secs: 4020,
    limit_reached: false,
    break_phase: "FIRST_WARNING",
    next_break_in_secs: 240,
    notices: [{ id: "demo-w1", kind: "BREAK_FIRST_WARNING" }],
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
    measurement: {
      source: "SESSION_HELPER",
      last_report_age_ms: 900,
      reporter: { kind: "BACKGROUND_LOGON_TASK", pid: 4321, session_id: 1, since_ms: at - 60_000 },
      competing_reports: 0,
    },
    storage: { persistent: false, schema_version: 1, recovered: false },
    clock_adjusted_today: false,
    computed_at_ms: at,
  };
}

/**
 * DEMO kaynağı — yalnız tarayıcıda (`npm run dev`, Tauri dışında) arayüz
 * geliştirmek için. Gerçek Agent'a bağlanmaz; ekranda açıkça "Demo" bandı
 * gösterilir. Tauri uygulamasında hiçbir zaman seçilmez (bkz. source.ts).
 */
export function createDemoSource(now: () => number = Date.now): AgentSource {
  let view: AgentView = {
    helper: "running",
    helper_restarts: 0,
    link: "disconnected",
    attempt: 1,
    retry_in_ms: 2000,
    link_reason: "demo",
    agent: null,
    screen_time: null,
    last_success_ms: null,
    closing: false,
    updated_ms: now(),
  };
  const listeners = new Set<(v: AgentView) => void>();
  const set = (next: AgentView) => {
    view = next;
    listeners.forEach((l) => {
      l(view);
    });
  };
  return {
    kind: "demo",
    initial: () => Promise.resolve(view),
    subscribe: (onChange) => {
      listeners.add(onChange);
      return Promise.resolve(() => listeners.delete(onChange));
    },
    refresh: () => {
      // Demo: "Şimdi yenile" bağlı/bağlantı yok arasında geçiş yapar.
      const connected = view.link !== "connected";
      set({
        ...view,
        link: connected ? "connected" : "disconnected",
        attempt: connected ? 0 : 1,
        retry_in_ms: connected ? null : 2000,
        agent: connected
          ? {
              version: "demo",
              run_mode: "SERVICE",
              uptime_secs: 754,
              connected_clients: 1,
              healthy: true,
              checks: [{ name: "ipc_listener", ok: true, detail: "demo" }],
              round_trip_ms: 1,
            }
          : null,
        screen_time: connected ? demoScreenTime(now()) : null,
        last_success_ms: connected ? now() : view.last_success_ms,
        updated_ms: now(),
      });
      return Promise.resolve();
    },
  };
}

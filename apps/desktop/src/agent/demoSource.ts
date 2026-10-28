import type { AgentSource, AgentView } from "./types";

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
        last_success_ms: connected ? now() : view.last_success_ms,
        updated_ms: now(),
      });
      return Promise.resolve();
    },
  };
}

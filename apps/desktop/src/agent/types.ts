// Rust tarafındaki `view::AgentView` ile birebir (serde snake_case).
// Değişiklik yapılırsa src-tauri/src/view.rs ile birlikte güncellenmelidir.

export type LinkState = "connecting" | "connected" | "disconnected" | "unauthorized";
export type HelperState = "starting" | "running" | "missing" | "exited" | "stopping";
export type RunMode = "SERVICE" | "CONSOLE";

export interface HealthCheck {
  name: string;
  ok: boolean;
  detail?: string;
}

export interface AgentInfo {
  version: string;
  run_mode: RunMode;
  uptime_secs: number;
  connected_clients: number;
  healthy: boolean;
  checks: HealthCheck[];
  round_trip_ms: number;
}

export interface AgentView {
  helper: HelperState;
  helper_restarts: number;
  helper_detail?: string;
  link: LinkState;
  attempt: number;
  retry_in_ms: number | null;
  link_reason: string | null;
  agent: AgentInfo | null;
  last_success_ms: number | null;
  closing: boolean;
  updated_ms: number;
}

/** Veri kaynağı: gerçek (Tauri + helper köprüsü) veya açıkça işaretli demo. */
export type SourceKind = "tauri" | "demo";

export interface AgentSource {
  readonly kind: SourceKind;
  initial(): Promise<AgentView>;
  subscribe(onChange: (view: AgentView) => void): Promise<() => void>;
  refresh(): Promise<void>;
}

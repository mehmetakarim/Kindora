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

// P1.C — `ipc_protocol::ScreenTimeStatus` ile birebir (agent hesaplar, arayüz yalnız gösterir).
export type UsageState = "ACTIVE" | "IDLE" | "SESSION_LOCKED" | "SUSPENDED" | "UNKNOWN";
export type BreakPhase =
  "NORMAL" | "FIRST_WARNING" | "FINAL_WARNING" | "BREAK_DUE" | "BREAK_COMPLETE";
export type NoticeKind =
  | "BREAK_FIRST_WARNING"
  | "BREAK_FINAL_WARNING"
  | "BREAK_STARTED"
  | "BREAK_COMPLETED"
  | "LIMIT_REACHED";
export type PolicySource = "BUILTIN_DEFAULT" | "DEV_FILE";
export type PolicyStatus =
  "OK" | "INVALID_KEPT_LAST" | "INVALID_USING_DEFAULT" | "VERSION_REJECTED";

export interface Notice {
  id: string;
  kind: NoticeKind;
}

export interface PolicyInfo {
  version: number;
  source: PolicySource;
  status: PolicyStatus;
  daily_limit_secs: number;
  break_interval_secs: number;
  break_duration_secs: number;
  first_warning_secs: number;
  final_warning_secs: number;
  idle_threshold_secs: number;
}

export interface ScreenTimeStatus {
  day: string;
  state: UsageState;
  today: {
    active_secs: number;
    idle_secs: number;
    locked_secs: number;
    suspended_secs: number;
    unknown_secs: number;
  };
  daily_limit_secs: number;
  remaining_secs: number;
  limit_reached: boolean;
  break_phase: BreakPhase;
  next_break_in_secs?: number;
  break_remaining_secs?: number;
  notices: Notice[];
  policy: PolicyInfo;
  measurement: {
    source: "SESSION_HELPER" | "NONE";
    last_report_age_ms?: number;
    unknown_reason?: string;
  };
  storage: { persistent: boolean; schema_version: number; recovered: boolean };
  clock_adjusted_today: boolean;
  computed_at_ms: number;
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
  /** Bağlantı yokken `null`: arayüz ölçüm yapıyormuş gibi davranmaz. */
  screen_time: ScreenTimeStatus | null;
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

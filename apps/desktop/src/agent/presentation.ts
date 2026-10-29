// Saf sunum mantığı: AgentView → ekrandaki metin, ton ve maskot anı.
// Bileşenlerden bağımsız test edilir (src/__tests__/presentation.spec.ts).

import { tr } from "../i18n/tr";
import type {
  AgentView,
  HelperState,
  LinkState,
  Notice,
  NoticeKind,
  RunMode,
  ScreenTimeStatus,
  UsageState,
} from "./types";

/** Durum çipinin tonu. Çocuk arayüzünde danger (mercan) kullanılmaz. */
export type Tone = "success" | "warning" | "info" | "neutral";

export interface ConnectionStatus {
  tone: Tone;
  label: string;
  /** Erişilebilir açıklama (renge ek olarak kelime + ikon). */
  detail: string;
}

export function connectionStatus(view: AgentView): ConnectionStatus {
  if (view.closing) {
    return { tone: "neutral", label: "Kapanıyor", detail: "Kindora güvenli şekilde kapanıyor." };
  }
  if (view.helper === "missing") {
    return {
      tone: "warning",
      label: "Bağlantı yok",
      detail: "kindora-helper.exe, Kindora ile aynı klasörde bulunamadı.",
    };
  }
  switch (view.link) {
    case "connected":
      return { tone: "success", label: "Bağlı", detail: "Kindora Agent yanıt veriyor." };
    case "unauthorized":
      return {
        tone: "warning",
        label: "Yetkilendirilmedi",
        detail: "Agent bu bağlantıyı tanımadı. Üç Kindora dosyası aynı klasörde olmalı.",
      };
    case "connecting":
      return view.last_success_ms === null && view.attempt === 0
        ? { tone: "info", label: "Bağlanıyor", detail: "Kindora Agent aranıyor." }
        : {
            tone: "info",
            label: "Yeniden bağlanıyor",
            detail: "Kindora Agent'a yeniden bağlanılıyor.",
          };
    case "disconnected":
      return { tone: "warning", label: "Bağlantı yok", detail: "Kindora Agent'a ulaşılamıyor." };
  }
}

export function runModeLabel(mode: RunMode): string {
  return mode === "SERVICE" ? "Windows servisi" : "Konsol (geliştirme)";
}

export function helperLabel(state: HelperState): string {
  const labels: Record<HelperState, string> = {
    starting: "Başlatılıyor",
    running: "Çalışıyor",
    missing: "Bulunamadı",
    exited: "Kapandı — yeniden başlatılacak",
    stopping: "Kapatılıyor",
  };
  return labels[state];
}

export function formatDuration(totalSecs: number): string {
  const s = Math.max(0, Math.floor(totalSecs));
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const sec = s % 60;
  if (h > 0) return `${String(h)} sa ${String(m)} dk`;
  if (m > 0) return `${String(m)} dk ${String(sec)} sn`;
  return `${String(sec)} sn`;
}

export function formatClock(ms: number): string {
  return new Date(ms).toLocaleTimeString("tr-TR", {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  });
}

/** "10:42:05 · 3 sn önce" */
export function formatLastSuccess(ms: number | null, now: number): string {
  if (ms === null) return "Henüz yok";
  const ago = Math.max(0, Math.round((now - ms) / 1000));
  const rel = ago < 2 ? "az önce" : `${formatDuration(ago)} önce`;
  return `${formatClock(ms)} · ${rel}`;
}

export function reconnectText(view: AgentView): string {
  if (view.link === "connected") return "Gerekmiyor";
  if (view.helper === "missing") return "Helper bulunamadı; 10 sn'de bir yeniden aranıyor";
  if (view.helper === "exited") {
    return `Helper yeniden başlatılıyor (${String(view.helper_restarts)}. kez)`;
  }
  if (view.retry_in_ms !== null && view.attempt > 0) {
    const secs = Math.max(1, Math.round(view.retry_in_ms / 1000));
    return `${String(view.attempt)}. deneme başarısız · ${String(secs)} sn içinde tekrar`;
  }
  return view.link === "connecting" ? "Bağlanılıyor…" : "Bekleniyor";
}

/** Konsept maskot çizimindeki durum adları (design Phase 1 `mascot()`). */
export type MascotPose =
  | "greeting"
  | "happy"
  | "idle"
  | "thinking"
  | "sleeping"
  | "permission"
  | "reminder"
  | "break"
  | "encourage";

export type MascotMoment =
  "welcome" | "connected" | "disconnected" | "reconnecting" | "unauthorized" | "closing";

export interface MascotLine {
  moment: MascotMoment | NoticeKind | "normal" | "quiet" | "unmeasured";
  pose: MascotPose;
  text: string;
}

const LINES: Record<MascotMoment, Omit<MascotLine, "moment">> = {
  welcome: { pose: "greeting", text: "Merhaba! Ben Filiz. Kindora'yı hazırlıyorum." },
  connected: { pose: "happy", text: "Kindora Agent'a bağlandım. Her şey yolunda." },
  disconnected: {
    pose: "idle",
    text: "Kindora Agent'a şu an ulaşamıyorum. Merak etme, kendiliğinden tekrar deneyeceğim.",
  },
  reconnecting: { pose: "thinking", text: "Yeniden bağlanmaya çalışıyorum…" },
  unauthorized: {
    pose: "permission",
    text: "Agent bu bağlantıyı tanımadı. Kindora dosyalarının aynı klasörde olması gerekiyor.",
  },
  closing: { pose: "sleeping", text: "Kindora güvenli şekilde kapanıyor. Görüşmek üzere!" },
};

export function mascotMoment(view: AgentView): MascotMoment {
  if (view.closing) return "closing";
  if (view.link === "connected") return "connected";
  if (view.link === "unauthorized") return "unauthorized";
  if (view.last_success_ms === null && view.attempt === 0 && view.helper !== "missing") {
    return "welcome";
  }
  // İlk kopuşta "bağlantı kesildi", süren denemelerde "yeniden bağlanılıyor".
  return view.link === "connecting" || view.attempt >= 2 ? "reconnecting" : "disconnected";
}

export function mascotLine(view: AgentView): MascotLine {
  const moment = mascotMoment(view);
  return { moment, ...LINES[moment] };
}

export function isLinkState(value: string): value is LinkState {
  return ["connecting", "connected", "disconnected", "unauthorized"].includes(value);
}

// ---------------------------------------------------------------------------
// P1.C — kullanım süresi ve mola sunumu. Değerler agent'tan gelir; burada
// yalnız biçimlendirilir. Mola geri sayımı iki ölçüm arasında yerel saatle
// yumuşatılır ama agent'ın verdiği değerin dışına çıkamaz.

/** "1 sa 25 dk", "12 dk", "1 dk'dan az". */
export function formatMinutes(totalSecs: number): string {
  const s = Math.max(0, Math.floor(totalSecs));
  if (s < 60) return "1 dk'dan az";
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  if (h > 0) return m > 0 ? `${String(h)} sa ${String(m)} dk` : `${String(h)} sa`;
  return `${String(m)} dk`;
}

/** "04:07" */
export function formatCountdown(totalSecs: number): string {
  const s = Math.max(0, Math.floor(totalSecs));
  return `${String(Math.floor(s / 60)).padStart(2, "0")}:${String(s % 60).padStart(2, "0")}`;
}

export function breakCountdown(st: ScreenTimeStatus, now: number): number | null {
  if (st.break_phase !== "BREAK_DUE" || st.break_remaining_secs === undefined) return null;
  const elapsed = Math.max(0, Math.floor((now - st.computed_at_ms) / 1000));
  return Math.max(0, Math.min(st.break_remaining_secs, st.break_remaining_secs - elapsed));
}

export function usageStateLabel(state: UsageState): string {
  return tr.usage[state];
}

export function usageTone(state: UsageState): Tone {
  if (state === "ACTIVE") return "success";
  if (state === "UNKNOWN") return "warning";
  return "neutral";
}

export function unknownReasonText(reason: string | undefined): string {
  if (reason === undefined) return "—";
  return tr.unknownReason[reason] ?? reason;
}

/** Halka için kullanılan oran (0–1). */
export function usedRatio(st: ScreenTimeStatus): number {
  if (st.daily_limit_secs <= 0) return 0;
  return Math.min(1, st.today.active_secs / st.daily_limit_secs);
}

export function nextBreakText(st: ScreenTimeStatus, now: number): string {
  switch (st.break_phase) {
    case "BREAK_DUE": {
      const left = breakCountdown(st, now) ?? 0;
      return `Mola zamanı · ${formatCountdown(left)} kaldı`;
    }
    case "BREAK_COMPLETE":
      return "Mola tamamlandı";
    default:
      return st.next_break_in_secs === undefined
        ? "—"
        : `${formatMinutes(st.next_break_in_secs)} kullanım sonra`;
  }
}

export function policyText(st: ScreenTimeStatus): string {
  const p = st.policy;
  const source =
    p.source === "DEV_FILE"
      ? `${tr.policySource.DEV_FILE} v${String(p.version)}`
      : tr.policySource.BUILTIN_DEFAULT;
  return p.status === "OK" ? source : `${source} · ${tr.policyStatus[p.status]}`;
}

/** Kapatılmamış ilk bildirim; aynı anda yalnız biri gösterilir. */
export function visibleNotice(
  st: ScreenTimeStatus | null,
  dismissed: ReadonlySet<string>,
): Notice | null {
  return st?.notices.find((n) => !dismissed.has(n.id)) ?? null;
}

const NOTICE_LINES: Record<Notice["kind"], { pose: MascotPose; text: string }> = {
  BREAK_FIRST_WARNING: { pose: "reminder", text: tr.filiz.breakSoon },
  BREAK_FINAL_WARNING: { pose: "reminder", text: tr.filiz.breakFinal },
  BREAK_STARTED: { pose: "break", text: tr.filiz.breakNow },
  BREAK_COMPLETED: { pose: "happy", text: tr.filiz.breakDone },
  LIMIT_REACHED: { pose: "encourage", text: tr.filiz.limitReached },
};

/**
 * Filiz'in sözü. Bağlantı/kapanış anları önceliklidir (P1.B); bağlıyken
 * kullanım süresi anları gelir. Sessiz modda yalnız mola ve süre uyarıları
 * konuşulur (normal iltifat yok → metin boş).
 */
export function filizLine(
  view: AgentView,
  dismissed: ReadonlySet<string>,
  quiet: boolean,
): MascotLine {
  const base = mascotLine(view);
  const st = view.screen_time;
  if (base.moment !== "connected" || st === null) return base;
  const notice = visibleNotice(st, dismissed);
  if (notice !== null) return { moment: notice.kind, ...NOTICE_LINES[notice.kind] };
  if (st.state === "UNKNOWN")
    return { moment: "unmeasured", pose: "thinking", text: tr.filiz.unmeasured };
  if (quiet) return { moment: "quiet", pose: "idle", text: "" };
  return {
    moment: "normal",
    pose: st.state === "ACTIVE" ? "happy" : "idle",
    text: tr.filiz.normal,
  };
}

/** Uyarı süresi agent'ın uyguladığı politikadan alınır (ör. DEV hızlı testte 1 dk). */
export function noticeText(n: Notice, st: ScreenTimeStatus): string {
  switch (n.kind) {
    case "BREAK_FIRST_WARNING":
      return tr.notice.BREAK_FIRST_WARNING(formatMinutes(st.policy.first_warning_secs));
    case "BREAK_FINAL_WARNING":
      return tr.notice.BREAK_FINAL_WARNING(formatMinutes(st.policy.final_warning_secs));
    default:
      return tr.notice[n.kind];
  }
}

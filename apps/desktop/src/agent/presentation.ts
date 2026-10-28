// Saf sunum mantığı: AgentView → ekrandaki metin, ton ve maskot anı.
// Bileşenlerden bağımsız test edilir (src/__tests__/presentation.spec.ts).

import type { AgentView, HelperState, LinkState, RunMode } from "./types";

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
export type MascotPose = "greeting" | "happy" | "idle" | "thinking" | "sleeping" | "permission";

export type MascotMoment =
  "welcome" | "connected" | "disconnected" | "reconnecting" | "unauthorized" | "closing";

export interface MascotLine {
  moment: MascotMoment;
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

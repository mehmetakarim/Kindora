import { describe, expect, it } from "vitest";
import {
  connectionStatus,
  formatDuration,
  formatLastSuccess,
  mascotLine,
  mascotMoment,
  reconnectText,
  runModeLabel,
} from "../agent/presentation";
import { baseView, connectedView } from "./fixtures";

describe("connectionStatus", () => {
  it("shows Bağlantı yok when the agent is unreachable", () => {
    const s = connectionStatus(baseView({ link: "disconnected", attempt: 2, retry_in_ms: 1000 }));
    expect(s.label).toBe("Bağlantı yok");
    expect(s.tone).toBe("warning");
  });

  it("never uses a danger tone (child-facing rule)", () => {
    const views = [
      baseView(),
      baseView({ link: "disconnected" }),
      baseView({ link: "unauthorized" }),
      baseView({ helper: "missing", link: "disconnected" }),
      connectedView(),
      connectedView({ closing: true }),
    ];
    for (const v of views) {
      expect(["success", "warning", "info", "neutral"]).toContain(connectionStatus(v).tone);
    }
  });

  it("distinguishes first connect from reconnect", () => {
    expect(connectionStatus(baseView()).label).toBe("Bağlanıyor");
    expect(connectionStatus(baseView({ attempt: 3 })).label).toBe("Yeniden bağlanıyor");
    expect(connectionStatus(baseView({ last_success_ms: 5 })).label).toBe("Yeniden bağlanıyor");
  });

  it("reports unauthorized and missing helper explicitly", () => {
    expect(connectionStatus(baseView({ link: "unauthorized" })).label).toBe("Yetkilendirilmedi");
    expect(
      connectionStatus(baseView({ helper: "missing", link: "disconnected" })).detail,
    ).toContain("kindora-helper.exe");
  });
});

describe("mascot", () => {
  it.each([
    [baseView(), "welcome", "greeting"],
    [connectedView(), "connected", "happy"],
    [baseView({ link: "disconnected", attempt: 1, retry_in_ms: 250 }), "disconnected", "idle"],
    [baseView({ link: "connecting", attempt: 2 }), "reconnecting", "thinking"],
    [baseView({ link: "disconnected", attempt: 4, retry_in_ms: 5000 }), "reconnecting", "thinking"],
    [baseView({ link: "unauthorized", attempt: 1 }), "unauthorized", "permission"],
    [connectedView({ closing: true }), "closing", "sleeping"],
  ] as const)("maps view to moment %#", (view, moment, pose) => {
    expect(mascotMoment(view)).toBe(moment);
    const line = mascotLine(view);
    expect(line.pose).toBe(pose);
    expect(line.text.length).toBeGreaterThan(10);
  });

  it("uses the design's mascot name only", () => {
    expect(mascotLine(baseView()).text).toContain("Filiz");
  });
});

describe("formatting", () => {
  it("formats durations", () => {
    expect(formatDuration(5)).toBe("5 sn");
    expect(formatDuration(125)).toBe("2 dk 5 sn");
    expect(formatDuration(3_725)).toBe("1 sa 2 dk");
  });

  it("formats last success", () => {
    expect(formatLastSuccess(null, 0)).toBe("Henüz yok");
    expect(formatLastSuccess(10_000, 10_500)).toContain("az önce");
    expect(formatLastSuccess(10_000, 40_000)).toContain("30 sn önce");
  });

  it("describes reconnect state", () => {
    expect(reconnectText(connectedView())).toBe("Gerekmiyor");
    expect(reconnectText(baseView({ link: "disconnected", attempt: 3, retry_in_ms: 2000 }))).toBe(
      "3. deneme başarısız · 2 sn içinde tekrar",
    );
    expect(
      reconnectText(baseView({ helper: "exited", helper_restarts: 2, link: "disconnected" })),
    ).toContain("2. kez");
  });

  it("labels run modes", () => {
    expect(runModeLabel("SERVICE")).toBe("Windows servisi");
    expect(runModeLabel("CONSOLE")).toBe("Konsol (geliştirme)");
  });
});

describe("theme", () => {
  it("defaults to the design's light theme and honours reduced motion", async () => {
    const { applyEnvironmentPreferences, applyTheme } = await import("../theme");
    const root = document.createElement("html");
    const win = {
      matchMedia: (q: string) => ({ matches: q.includes("reduced-motion") }) as MediaQueryList,
    };
    expect(applyEnvironmentPreferences(root, win)).toBe(false);
    expect(root.getAttribute("data-bs-theme")).toBe("light");
    expect(root.getAttribute("data-kd-motion")).toBe("reduced");
    applyTheme(root, "dark");
    expect(root.getAttribute("data-bs-theme")).toBe("dark");
  });
});

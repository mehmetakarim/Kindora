import { flushPromises, mount } from "@vue/test-utils";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import App from "../App.vue";
import {
  breakCountdown,
  filizLine,
  formatCountdown,
  formatMinutes,
  nextBreakText,
  policyText,
  usageTone,
  visibleNotice,
} from "../agent/presentation";
import { tr } from "../i18n/tr";
import { baseView, connectedView, fakeSource, screenTime } from "./fixtures";

beforeEach(() => {
  window.localStorage.clear();
});
afterEach(() => {
  window.localStorage.clear();
});

describe("screen time presentation", () => {
  it("formats minutes and countdowns", () => {
    expect(formatMinutes(0)).toBe("1 dk'dan az");
    expect(formatMinutes(59)).toBe("1 dk'dan az");
    expect(formatMinutes(720)).toBe("12 dk");
    expect(formatMinutes(7200)).toBe("2 sa");
    expect(formatMinutes(5100)).toBe("1 sa 25 dk");
    expect(formatCountdown(247)).toBe("04:07");
    expect(formatCountdown(-5)).toBe("00:00");
  });

  it("interpolates the break countdown but never exceeds agent values", () => {
    const st = screenTime({
      break_phase: "BREAK_DUE",
      break_remaining_secs: 300,
      computed_at_ms: 10_000,
    });
    delete st.next_break_in_secs;
    expect(breakCountdown(st, 10_000)).toBe(300);
    expect(breakCountdown(st, 13_500)).toBe(297);
    expect(breakCountdown(st, 9_000)).toBe(300);
    expect(breakCountdown(st, 1_000_000)).toBe(0);
    expect(nextBreakText(st, 13_500)).toBe("Mola zamanı · 04:57 kaldı");
    expect(nextBreakText(screenTime(), 0)).toBe("20 dk kullanım sonra");
    expect(breakCountdown(screenTime(), 0)).toBeNull();
  });

  it("never uses a danger tone for usage states", () => {
    for (const s of ["ACTIVE", "IDLE", "SESSION_LOCKED", "SUSPENDED", "UNKNOWN"] as const) {
      expect(["success", "warning", "info", "neutral"]).toContain(usageTone(s));
    }
  });

  it("labels test policies honestly", () => {
    expect(policyText(screenTime())).toContain("ebeveyn tercihi değil");
    const dev = screenTime();
    dev.policy = { ...dev.policy, source: "DEV_FILE", version: 3, status: "INVALID_KEPT_LAST" };
    expect(policyText(dev)).toBe(`DEV test politikası v3 · ${tr.policyStatus.INVALID_KEPT_LAST}`);
  });

  it.each([
    ["BREAK_FIRST_WARNING", tr.filiz.breakSoon, "reminder"],
    ["BREAK_FINAL_WARNING", tr.filiz.breakFinal, "reminder"],
    ["BREAK_STARTED", tr.filiz.breakNow, "break"],
    ["BREAK_COMPLETED", tr.filiz.breakDone, "happy"],
    ["LIMIT_REACHED", tr.filiz.limitReached, "encourage"],
  ] as const)("Filiz says the fixed line for %s", (kind, text, pose) => {
    const v = connectedView({ screen_time: screenTime({ notices: [{ id: "n1", kind }] }) });
    const line = filizLine(v, new Set(), false);
    expect(line.text).toBe(text);
    expect(line.pose).toBe(pose);
    // Kapatılınca normal söze döner; sessiz modda iltifat yok.
    expect(filizLine(v, new Set(["n1"]), false).text).toBe(tr.filiz.normal);
    expect(filizLine(v, new Set(["n1"]), true).text).toBe("");
    // Sessiz mod uyarıları susturmaz.
    expect(filizLine(v, new Set(), true).text).toBe(text);
  });

  it("uses the exact requested messages", () => {
    expect(tr.filiz.normal).toBe("Bugün güzel ilerliyorsun!");
    expect(tr.filiz.breakSoon).toBe("Birazdan kısa bir mola veriyoruz.");
    expect(tr.filiz.breakFinal).toBe("Çalışmanı kaydetmek için iyi bir zaman.");
    expect(tr.filiz.breakNow).toBe("Gözlerini dinlendirmeye ne dersin?");
    expect(tr.filiz.breakDone).toBe("Hazırsan devam edebiliriz!");
  });

  it("keeps connection moments ahead of screen time and reports unmeasured state", () => {
    const st = screenTime({ notices: [{ id: "x", kind: "BREAK_STARTED" }] });
    expect(
      filizLine(baseView({ link: "disconnected", attempt: 1, screen_time: st }), new Set(), false)
        .moment,
    ).toBe("disconnected");
    const unknown = connectedView({ screen_time: screenTime({ state: "UNKNOWN" }) });
    expect(filizLine(unknown, new Set(), false).text).toBe(tr.filiz.unmeasured);
  });

  it("shows only one notice at a time and skips dismissed ids", () => {
    const st = screenTime({
      notices: [
        { id: "b1-due", kind: "BREAK_STARTED" },
        { id: "d-limit", kind: "LIMIT_REACHED" },
      ],
    });
    expect(visibleNotice(st, new Set())?.id).toBe("b1-due");
    expect(visibleNotice(st, new Set(["b1-due"]))?.id).toBe("d-limit");
    expect(visibleNotice(st, new Set(["b1-due", "d-limit"]))).toBeNull();
    expect(visibleNotice(null, new Set())).toBeNull();
  });
});

describe("App screen time", () => {
  it("shows usage on the home view and diagnostics separately", async () => {
    const f = fakeSource(connectedView({ screen_time: screenTime() }));
    const w = mount(App, { props: { source: f.source, motion: false }, attachTo: document.body });
    await flushPromises();
    expect(w.get('[data-testid="active-today"]').text()).toBe("25 dk");
    expect(w.get('[data-testid="daily-limit"]').text()).toBe("2 sa");
    expect(w.get('[data-testid="remaining"]').text()).toBe("1 sa 35 dk");
    expect(w.get('[data-testid="next-break"]').text()).toBe("20 dk kullanım sonra");
    expect(w.get('[data-testid="usage-state"]').text()).toContain("Kullanılıyor");
    expect(w.get('[data-testid="filiz-bubble"]').text()).toBe(tr.filiz.normal);
    expect(w.get('[data-testid="diagnostics"]').isVisible()).toBe(false);
    await w.get('[data-testid="tab-diagnostics"]').trigger("click");
    expect(w.get('[data-testid="diagnostics"]').isVisible()).toBe(true);
    expect(w.get('[data-testid="home"]').isVisible()).toBe(false);
    expect(w.get('[data-testid="agent-version"]').text()).toBe("0.1.0");
    expect(w.get('[data-testid="diag-storage"]').text()).toContain("SQLite");
    w.unmount();
  });

  it("does not pretend to measure while the agent is unreachable", async () => {
    const f = fakeSource(connectedView({ screen_time: screenTime() }));
    const w = mount(App, { props: { source: f.source, motion: false } });
    await flushPromises();
    f.push(
      baseView({ link: "disconnected", attempt: 1, retry_in_ms: 500, last_success_ms: 1_000 }),
    );
    await flushPromises();
    expect(w.find('[data-testid="active-today"]').exists()).toBe(false);
    expect(w.get('[data-testid="usage-unavailable"]').text()).toContain("Bağlantı yok");
    w.unmount();
  });

  it("dismisses a reminder once and never shows the same event again", async () => {
    const due = screenTime({
      break_phase: "FIRST_WARNING",
      notices: [{ id: "b1-w1", kind: "BREAK_FIRST_WARNING" }],
    });
    const f = fakeSource(connectedView({ screen_time: due }));
    const w = mount(App, { props: { source: f.source, motion: false } });
    await flushPromises();
    expect(w.get('[data-testid="notice"]').attributes("data-notice-id")).toBe("b1-w1");
    expect(w.get('[data-testid="notice"]').text()).toContain("Molaya yaklaşık 5 dk kaldı.");
    expect(w.get('[data-testid="filiz-bubble"]').text()).toBe(tr.filiz.breakSoon);
    await w.get('[data-testid="notice-dismiss"]').trigger("click");
    expect(w.find('[data-testid="notice"]').exists()).toBe(false);
    // Aynı olay sonraki yoklamalarda tekrar gelir; yeniden gösterilmez.
    f.push(connectedView({ screen_time: { ...due, computed_at_ms: 3_000 } }));
    await flushPromises();
    expect(w.find('[data-testid="notice"]').exists()).toBe(false);
    // Yeni olay (son uyarı) yeni kimlikle gösterilir; tek bildirim yuvası.
    f.push(
      connectedView({
        screen_time: screenTime({
          break_phase: "FINAL_WARNING",
          notices: [{ id: "b1-w2", kind: "BREAK_FINAL_WARNING" }],
        }),
      }),
    );
    await flushPromises();
    expect(w.findAll('[data-testid="notice"]')).toHaveLength(1);
    expect(w.get('[data-testid="notice"]').attributes("data-notice-id")).toBe("b1-w2");
    w.unmount();
  });

  it("limit reached is informational and dismissible", async () => {
    const st = screenTime({
      limit_reached: true,
      remaining_secs: 0,
      today: {
        active_secs: 7300,
        idle_secs: 0,
        locked_secs: 0,
        suspended_secs: 0,
        unknown_secs: 0,
      },
      notices: [{ id: "2026-10-29-limit", kind: "LIMIT_REACHED" }],
    });
    const f = fakeSource(connectedView({ screen_time: st }));
    const w = mount(App, { props: { source: f.source, motion: false } });
    await flushPromises();
    expect(w.get('[data-testid="remaining"]').text()).toBe("Süre doldu");
    expect(w.get('[data-testid="notice"]').text()).toContain("kilitlenmez");
    await w.get('[data-testid="notice-dismiss"]').trigger("click");
    expect(w.find('[data-testid="notice"]').exists()).toBe(false);
    w.unmount();
  });

  it("quiet mode hides encouragement and persists the preference", async () => {
    const f = fakeSource(connectedView({ screen_time: screenTime() }));
    const w = mount(App, { props: { source: f.source, motion: false } });
    await flushPromises();
    await w.get('[data-testid="pref-quiet"]').setValue(true);
    expect(w.find('[data-testid="filiz-bubble"]').exists()).toBe(false);
    expect(window.localStorage.getItem("kd.prefs.v1")).toContain('"quiet":true');
    await w.get('[data-testid="pref-motion"]').setValue(true);
    expect(document.documentElement.getAttribute("data-kd-motion")).toBe("reduced");
    w.unmount();
  });
});

import { flushPromises, mount } from "@vue/test-utils";
import { afterEach, describe, expect, it, vi } from "vitest";
import App from "../App.vue";
import { createAgentSource } from "../agent/source";
import { baseView, connectedView, fakeSource } from "./fixtures";

afterEach(() => {
  vi.useRealTimers();
});

describe("App", () => {
  it("shows Bağlantı yok and no agent values when the agent is unreachable", async () => {
    const f = fakeSource(baseView({ link: "disconnected", attempt: 1, retry_in_ms: 250 }));
    const w = mount(App, { props: { source: f.source, motion: false } });
    await flushPromises();
    expect(w.get('[data-testid="no-connection-banner"]').text()).toContain("Bağlantı yok");
    expect(w.get('[data-testid="connection-chip"]').text()).toContain("Bağlantı yok");
    expect(w.get('[data-testid="run-mode"]').text()).toBe("Bağlantı yok");
    expect(w.get('[data-testid="agent-version"]').text()).toBe("Bağlantı yok");
    expect(w.find('[data-testid="demo-banner"]').exists()).toBe(false);
    w.unmount();
  });

  it("renders real agent values pushed by the source and recovers after a drop", async () => {
    const f = fakeSource(baseView());
    const w = mount(App, { props: { source: f.source, motion: false } });
    await flushPromises();
    expect(f.subscribed()).toBe(true);

    f.push(connectedView());
    await flushPromises();
    expect(w.get('[data-testid="connection-chip"]').text()).toContain("Bağlı");
    expect(w.get('[data-testid="run-mode"]').text()).toBe("Windows servisi");
    expect(w.get('[data-testid="agent-version"]').text()).toBe("0.1.0");
    expect(w.find('[data-testid="no-connection-banner"]').exists()).toBe(false);

    f.push(
      baseView({ link: "disconnected", attempt: 1, retry_in_ms: 500, last_success_ms: 1_000 }),
    );
    await flushPromises();
    expect(w.get('[data-testid="agent-version"]').text()).toBe("Bağlantı yok");
    expect(w.get('[data-testid="last-success"]').text()).not.toBe("Henüz yok");

    f.push(connectedView());
    await flushPromises();
    expect(w.get('[data-testid="connection-chip"]').text()).toContain("Bağlı");
    w.unmount();
    expect(f.subscribed()).toBe(false);
  });

  it("forwards the retry button to the source and throttles repeated clicks", async () => {
    vi.useFakeTimers();
    const f = fakeSource(baseView({ link: "disconnected", attempt: 1, retry_in_ms: 250 }));
    const w = mount(App, { props: { source: f.source, motion: false } });
    await flushPromises();
    const button = w.get('[data-testid="no-connection-banner"] button');
    await button.trigger("click");
    await button.trigger("click");
    await flushPromises();
    expect(f.refreshes()).toBe(1);
    vi.advanceTimersByTime(1100);
    await flushPromises();
    await button.trigger("click");
    await flushPromises();
    expect(f.refreshes()).toBe(2);
    w.unmount();
  });

  it("shows the closing message", async () => {
    const f = fakeSource(connectedView({ closing: true, helper: "stopping" }));
    const w = mount(App, { props: { source: f.source, motion: false } });
    await flushPromises();
    expect(w.text()).toContain("güvenli şekilde kapanıyor");
    expect(w.find('[data-testid="no-connection-banner"]').exists()).toBe(false);
    w.unmount();
  });

  it("labels demo data explicitly and never uses demo inside Tauri", async () => {
    expect(createAgentSource(true).kind).toBe("tauri");
    const demo = createAgentSource(false);
    expect(demo.kind).toBe("demo");
    const w = mount(App, { props: { source: demo, motion: false } });
    await flushPromises();
    expect(w.get('[data-testid="demo-banner"]').text()).toContain("gerçek Agent verisi değildir");
    w.unmount();
  });
});

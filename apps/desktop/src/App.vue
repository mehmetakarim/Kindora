<script setup lang="ts">
import { PhPlug } from "@phosphor-icons/vue";
import { computed, shallowRef, watchEffect } from "vue";
import { filizLine, visibleNotice } from "./agent/presentation";
import { createAgentSource } from "./agent/source";
import type { AgentSource } from "./agent/types";
import AgentStatusCard from "./components/AgentStatusCard.vue";
import BreakCard from "./components/BreakCard.vue";
import FilizSettings from "./components/FilizSettings.vue";
import KdLogo from "./components/KdLogo.vue";
import MascotPanel from "./components/MascotPanel.vue";
import MeasurementCard from "./components/MeasurementCard.vue";
import NoticeBanner from "./components/NoticeBanner.vue";
import SafetyCard from "./components/SafetyCard.vue";
import TodayCard from "./components/TodayCard.vue";
import { useAgent } from "./composables/useAgent";
import { usePreferences } from "./composables/usePreferences";

const props = withDefaults(defineProps<{ source?: AgentSource; motion?: boolean }>(), {
  source: () => createAgentSource(),
  motion: true,
});

const { view, now, refreshing, error, status, refresh, kind } = useAgent(props.source);
const { quiet, reduceMotion } = usePreferences();
const motion = computed(() => props.motion && !reduceMotion.value);
watchEffect(() => {
  if (reduceMotion.value) document.documentElement.setAttribute("data-kd-motion", "reduced");
  else if (props.motion) document.documentElement.removeAttribute("data-kd-motion");
});

type Tab = "home" | "diagnostics";
const tab = shallowRef<Tab>("home");

// Kapatılan bildirim kimlikleri: aynı olay bir daha gösterilmez (yeni olay yeni kimlik alır).
const dismissed = shallowRef<ReadonlySet<string>>(new Set());
function dismiss(id: string) {
  dismissed.value = new Set([...dismissed.value, id]);
}

const screenTime = computed(() => view.value?.screen_time ?? null);
const notice = computed(() => visibleNotice(screenTime.value, dismissed.value));
const mascot = computed(() =>
  view.value ? filizLine(view.value, dismissed.value, quiet.value) : null,
);
const disconnected = computed(
  () => view.value !== null && !view.value.closing && view.value.link !== "connected",
);
</script>

<template>
  <div class="kd-shell">
    <header class="kd-topbar">
      <KdLogo :size="36" />
      <div class="kd-topbar__title">
        <h1>Kindora</h1>
        <p>Masaüstü · P1.C önizleme</p>
      </div>
      <nav class="kd-tabs" aria-label="Görünüm">
        <button
          type="button"
          class="kd-tab"
          :class="{ 'kd-tab--active': tab === 'home' }"
          :aria-pressed="tab === 'home'"
          data-testid="tab-home"
          @click="tab = 'home'"
        >
          Ana görünüm
        </button>
        <button
          type="button"
          class="kd-tab"
          :class="{ 'kd-tab--active': tab === 'diagnostics' }"
          :aria-pressed="tab === 'diagnostics'"
          data-testid="tab-diagnostics"
          @click="tab = 'diagnostics'"
        >
          Teknik tanılama
        </button>
      </nav>
    </header>

    <main class="kd-main">
      <div
        v-if="kind === 'demo'"
        class="kd-banner kd-banner--demo"
        role="note"
        data-testid="demo-banner"
      >
        <strong>Demo</strong> — Tauri dışında çalışıyor; gösterilen değerler gerçek Agent verisi
        değildir.
      </div>

      <div
        v-if="disconnected && status"
        class="kd-banner kd-banner--warn"
        role="alert"
        data-testid="no-connection-banner"
      >
        <PhPlug :size="20" weight="bold" aria-hidden="true" />
        <span
          ><strong>{{ status.label }}.</strong> {{ status.detail }}</span
        >
        <button
          type="button"
          class="btn btn-sm btn-outline-secondary kd-banner__action"
          :disabled="refreshing"
          @click="refresh"
        >
          Yeniden dene
        </button>
      </div>

      <div v-if="error" class="kd-banner kd-banner--warn" role="alert">
        Uygulama içi hata: {{ error }}
      </div>

      <template v-if="view && mascot">
        <div v-show="tab === 'home'" class="kd-grid" data-testid="home">
          <div class="kd-col">
            <MascotPanel :line="mascot" :motion="motion" />
            <FilizSettings v-model:quiet="quiet" v-model:reduce-motion="reduceMotion" />
          </div>
          <div class="kd-col">
            <NoticeBanner
              v-if="notice && screenTime"
              :key="notice.id"
              :notice="notice"
              :status="screenTime"
              @dismiss="dismiss"
            />
            <TodayCard :status="screenTime" :now="now" />
            <BreakCard :status="screenTime" :now="now" />
          </div>
        </div>
        <div v-show="tab === 'diagnostics'" class="kd-grid" data-testid="diagnostics">
          <AgentStatusCard :view="view" :now="now" :refreshing="refreshing" @refresh="refresh" />
          <MeasurementCard :status="screenTime" :now="now" />
          <SafetyCard class="kd-grid__wide" />
        </div>
      </template>
      <p v-else-if="!error" class="kd-loading">Yükleniyor…</p>
    </main>
  </div>
</template>

<style scoped>
.kd-shell {
  min-height: 100vh;
  display: flex;
  flex-direction: column;
}
.kd-topbar {
  display: flex;
  align-items: center;
  gap: 12px;
  min-height: 72px;
  padding: 12px 28px;
  background: var(--kd-surface);
  border-bottom: 1px solid var(--kd-line);
}
.kd-topbar__title h1 {
  font-size: 20px;
  font-weight: 700;
  margin: 0;
  line-height: 1.15;
}
.kd-topbar__title p {
  margin: 0;
  font-size: 13px;
  color: var(--kd-ink-3);
}
.kd-main {
  width: 100%;
  max-width: 1320px;
  margin: 0 auto;
  padding: 24px 28px 40px;
  display: flex;
  flex-direction: column;
  gap: 18px;
}
.kd-grid {
  display: grid;
  grid-template-columns: minmax(280px, 380px) minmax(0, 1fr);
  gap: 18px;
  align-items: start;
}
.kd-col {
  display: flex;
  flex-direction: column;
  gap: 18px;
  min-width: 0;
}
.kd-tabs {
  margin-left: auto;
  display: flex;
  gap: 4px;
  padding: 4px;
  border-radius: 999px;
  background: var(--kd-surface-2);
}
.kd-tab {
  border: 0;
  background: transparent;
  color: var(--kd-ink-2);
  font-weight: 600;
  font-size: 14px;
  padding: 6px 14px;
  border-radius: 999px;
}
.kd-tab--active {
  background: var(--kd-surface);
  color: var(--kd-primary-text);
  box-shadow: 0 1px 2px rgba(26, 32, 51, 0.08);
}
.kd-grid__wide {
  grid-column: 1 / -1;
}
.kd-banner {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-wrap: wrap;
  padding: 12px 16px;
  border-radius: 10px;
  border: 1px solid transparent;
}
.kd-banner--warn {
  background: var(--kd-warning-tint);
  color: var(--kd-warning-text);
  border-color: var(--kd-warning-line);
}
.kd-banner--demo {
  background: var(--kd-secondary-tint);
  color: var(--kd-secondary-text);
  border: 1px dashed currentColor;
}
.kd-banner__action {
  margin-left: auto;
}
.kd-loading {
  color: var(--kd-ink-3);
}
@media (max-width: 900px) {
  .kd-grid {
    grid-template-columns: minmax(0, 1fr);
  }
  .kd-main {
    padding: 16px;
  }
  .kd-topbar {
    padding: 12px 16px;
    flex-wrap: wrap;
  }
}
</style>

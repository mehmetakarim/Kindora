<script setup lang="ts">
import { PhPlug } from "@phosphor-icons/vue";
import { computed } from "vue";
import { createAgentSource } from "./agent/source";
import type { AgentSource } from "./agent/types";
import AgentStatusCard from "./components/AgentStatusCard.vue";
import KdLogo from "./components/KdLogo.vue";
import MascotPanel from "./components/MascotPanel.vue";
import SafetyCard from "./components/SafetyCard.vue";
import { useAgent } from "./composables/useAgent";

const props = withDefaults(defineProps<{ source?: AgentSource; motion?: boolean }>(), {
  source: () => createAgentSource(),
  motion: true,
});

const { view, now, refreshing, error, status, mascot, refresh, kind } = useAgent(props.source);
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
        <p>Masaüstü · P1.B önizleme</p>
      </div>
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

      <div v-if="view && mascot" class="kd-grid">
        <MascotPanel :line="mascot" :motion="motion" />
        <AgentStatusCard :view="view" :now="now" :refreshing="refreshing" @refresh="refresh" />
        <SafetyCard class="kd-grid__wide" />
      </div>
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
  }
}
</style>

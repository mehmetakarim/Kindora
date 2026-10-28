<script setup lang="ts">
import { PhArrowsClockwise, PhCheckCircle, PhWarningCircle } from "@phosphor-icons/vue";
import { computed } from "vue";
import {
  connectionStatus,
  formatDuration,
  formatLastSuccess,
  helperLabel,
  reconnectText,
  runModeLabel,
} from "../agent/presentation";
import type { AgentView } from "../agent/types";
import KdStatusChip from "./KdStatusChip.vue";

const props = defineProps<{ view: AgentView; now: number; refreshing: boolean }>();
const emit = defineEmits<{ refresh: [] }>();

const status = computed(() => connectionStatus(props.view));
const agent = computed(() => props.view.agent);
const NOT_AVAILABLE = "Bağlantı yok";
</script>

<template>
  <section class="kd-card kd-status" aria-labelledby="status-title">
    <header class="kd-status__head">
      <div>
        <h2 id="status-title" class="kd-status__title">Kindora Agent</h2>
        <p class="kd-status__sub">{{ status.detail }}</p>
      </div>
      <KdStatusChip :tone="status.tone" :label="status.label" data-testid="connection-chip" />
    </header>

    <dl class="kd-status__grid">
      <dt>Çalışma modu</dt>
      <dd data-testid="run-mode">{{ agent ? runModeLabel(agent.run_mode) : NOT_AVAILABLE }}</dd>

      <dt>Agent sürümü</dt>
      <dd data-testid="agent-version">{{ agent ? agent.version : NOT_AVAILABLE }}</dd>

      <dt>Agent sağlığı</dt>
      <dd data-testid="agent-health">
        <template v-if="agent">
          <KdStatusChip
            :tone="agent.healthy ? 'success' : 'warning'"
            :label="agent.healthy ? 'Sağlıklı' : 'Sorun var'"
          />
        </template>
        <template v-else>{{ NOT_AVAILABLE }}</template>
      </dd>

      <dt>Çalışma süresi</dt>
      <dd>{{ agent ? formatDuration(agent.uptime_secs) : NOT_AVAILABLE }}</dd>

      <dt>Son başarılı iletişim</dt>
      <dd data-testid="last-success">{{ formatLastSuccess(view.last_success_ms, now) }}</dd>

      <dt>Yeniden bağlantı</dt>
      <dd data-testid="reconnect">{{ reconnectText(view) }}</dd>

      <dt>Oturum yardımcısı</dt>
      <dd>{{ helperLabel(view.helper) }}</dd>
    </dl>

    <ul v-if="agent && agent.checks.length" class="kd-checks" aria-label="Sağlık denetimleri">
      <li v-for="c in agent.checks" :key="c.name">
        <component
          :is="c.ok ? PhCheckCircle : PhWarningCircle"
          :size="16"
          weight="bold"
          :class="c.ok ? 'kd-ok' : 'kd-warn'"
          aria-hidden="true"
        />
        <span class="kd-checks__name">{{ c.name }}</span>
        <span class="kd-checks__detail"
          >{{ c.ok ? "tamam" : "sorun"
          }}<template v-if="c.detail"> · {{ c.detail }}</template></span
        >
      </li>
    </ul>

    <footer class="kd-status__foot">
      <span v-if="agent" class="kd-status__rtt">Yanıt süresi: {{ agent.round_trip_ms }} ms</span>
      <button
        type="button"
        class="btn btn-outline-primary btn-sm kd-refresh"
        :disabled="refreshing || view.closing"
        @click="emit('refresh')"
      >
        <PhArrowsClockwise :size="16" weight="bold" aria-hidden="true" />
        Şimdi yenile
      </button>
    </footer>
  </section>
</template>

<style scoped>
.kd-status {
  padding: 24px;
  display: flex;
  flex-direction: column;
  gap: 20px;
}
.kd-status__head {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: 16px;
  flex-wrap: wrap;
}
.kd-status__title {
  font-size: 20px;
  font-weight: 700;
  margin: 0 0 4px;
}
.kd-status__sub {
  margin: 0;
  color: var(--kd-ink-2);
}
.kd-status__grid {
  display: grid;
  grid-template-columns: minmax(150px, max-content) 1fr;
  gap: 12px 20px;
  margin: 0;
}
.kd-status__grid dt {
  color: var(--kd-ink-3);
  font-weight: 500;
}
.kd-status__grid dd {
  margin: 0;
  color: var(--kd-ink);
  font-weight: 600;
  overflow-wrap: anywhere;
}
.kd-checks {
  list-style: none;
  margin: 0;
  padding: 12px 14px;
  border-radius: 10px;
  background: var(--kd-surface-2);
  display: grid;
  gap: 6px;
}
.kd-checks li {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}
.kd-checks__name {
  font-family: ui-monospace, "Cascadia Mono", Menlo, monospace;
  font-size: 13px;
}
.kd-checks__detail {
  color: var(--kd-ink-3);
  font-size: 13px;
}
.kd-ok {
  color: var(--kd-success-text);
}
.kd-warn {
  color: var(--kd-warning-text);
}
.kd-status__foot {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 12px;
  flex-wrap: wrap;
}
.kd-status__rtt {
  color: var(--kd-ink-3);
  font-size: 13px;
}
.kd-refresh {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  margin-left: auto;
}
@media (max-width: 560px) {
  .kd-status__grid {
    grid-template-columns: 1fr;
    gap: 2px;
  }
  .kd-status__grid dd {
    margin-bottom: 10px;
  }
}
</style>

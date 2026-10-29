<script setup lang="ts">
import { computed } from "vue";
import {
  formatLastSuccess,
  formatMinutes,
  policyText,
  usageStateLabel,
  usageTone,
  usedRatio,
} from "../agent/presentation";
import type { ScreenTimeStatus } from "../agent/types";
import KdStatusChip from "./KdStatusChip.vue";

// Değerler agent'tan gelir. `status === null`: bağlantı yok → sayı gösterilmez.
const props = defineProps<{ status: ScreenTimeStatus | null; now: number }>();

const R = 70;
const C = 2 * Math.PI * R;
const dash = computed(() => (props.status ? C * usedRatio(props.status) : 0));
// Az kalınca amber; çocuk tarafında danger rengi yok.
const low = computed(
  () =>
    props.status !== null && props.status.remaining_secs <= props.status.daily_limit_secs * 0.15,
);
</script>

<template>
  <section class="kd-card kd-today" aria-labelledby="today-title">
    <header class="kd-today__head">
      <h2 id="today-title" class="kd-today__title">Bugünkü kullanım</h2>
      <KdStatusChip
        v-if="status"
        :tone="usageTone(status.state)"
        :label="usageStateLabel(status.state)"
        data-testid="usage-state"
      />
    </header>

    <template v-if="status">
      <div class="kd-today__body">
        <svg class="kd-ring" viewBox="0 0 160 160" width="160" height="160" aria-hidden="true">
          <circle cx="80" cy="80" :r="R" class="kd-ring__track" />
          <circle
            cx="80"
            cy="80"
            :r="R"
            class="kd-ring__value"
            :class="{ 'kd-ring__value--low': low }"
            :stroke-dasharray="`${dash} ${C}`"
            transform="rotate(-90 80 80)"
          />
        </svg>
        <dl class="kd-today__grid">
          <dt>Bugün kullanılan</dt>
          <dd data-testid="active-today">{{ formatMinutes(status.today.active_secs) }}</dd>
          <dt>Günlük limit</dt>
          <dd data-testid="daily-limit">{{ formatMinutes(status.daily_limit_secs) }}</dd>
          <dt>Kalan süre</dt>
          <dd data-testid="remaining">
            {{ status.limit_reached ? "Süre doldu" : formatMinutes(status.remaining_secs) }}
          </dd>
        </dl>
      </div>
      <p class="kd-today__meta" data-testid="policy">Kurallar: {{ policyText(status) }}</p>
      <p class="kd-today__meta" data-testid="last-measured">
        Son ölçüm: {{ formatLastSuccess(status.computed_at_ms, now) }}
      </p>
    </template>
    <p v-else class="kd-today__empty" data-testid="usage-unavailable">
      Bağlantı yok — Kindora Agent'a ulaşılamadığı için kullanım süresi gösterilemiyor.
    </p>
  </section>
</template>

<style scoped>
.kd-today {
  padding: 24px;
  display: flex;
  flex-direction: column;
  gap: 16px;
  border-radius: 28px;
}
.kd-today__head {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 12px;
  flex-wrap: wrap;
}
.kd-today__title {
  font-family: "Fredoka", "Onest", system-ui, sans-serif;
  font-weight: 600;
  font-size: 20px;
  margin: 0;
}
.kd-today__body {
  display: flex;
  align-items: center;
  gap: 24px;
  flex-wrap: wrap;
}
.kd-ring__track {
  fill: none;
  stroke: var(--kd-neutral-tint);
  stroke-width: 16;
}
.kd-ring__value {
  fill: none;
  stroke: var(--kd-primary);
  stroke-width: 16;
  stroke-linecap: round;
  transition: stroke-dasharray 400ms ease;
}
.kd-ring__value--low {
  stroke: var(--kd-warning-text);
}
.kd-today__grid {
  display: grid;
  grid-template-columns: max-content 1fr;
  gap: 8px 16px;
  margin: 0;
}
.kd-today__grid dt {
  color: var(--kd-ink-3);
  font-weight: 500;
}
.kd-today__grid dd {
  margin: 0;
  font-family: "Fredoka", "Onest", system-ui, sans-serif;
  font-weight: 600;
  font-size: 20px;
}
.kd-today__meta {
  margin: 0;
  font-size: 13px;
  color: var(--kd-ink-3);
}
.kd-today__empty {
  margin: 0;
  color: var(--kd-ink-2);
}
</style>

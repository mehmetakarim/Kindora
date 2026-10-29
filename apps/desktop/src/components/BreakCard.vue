<script setup lang="ts">
import { PhCoffee } from "@phosphor-icons/vue";
import { formatMinutes, nextBreakText } from "../agent/presentation";
import type { ScreenTimeStatus } from "../agent/types";

defineProps<{ status: ScreenTimeStatus | null; now: number }>();
</script>

<template>
  <section class="kd-card kd-break" aria-labelledby="break-title">
    <h2 id="break-title" class="kd-break__title">
      <PhCoffee :size="20" weight="bold" aria-hidden="true" />
      Sıradaki mola
    </h2>
    <template v-if="status">
      <p class="kd-break__value" data-testid="next-break">{{ nextBreakText(status, now) }}</p>
      <p class="kd-break__meta">
        Her {{ formatMinutes(status.policy.break_interval_secs) }} aktif kullanımda
        {{ formatMinutes(status.policy.break_duration_secs) }} mola. Boşta kalınan süre molayı
        yaklaştırmaz.
      </p>
    </template>
    <p v-else class="kd-break__meta">Bağlantı yok.</p>
  </section>
</template>

<style scoped>
.kd-break {
  padding: 20px 24px;
  border-radius: 28px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.kd-break__title {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 15px;
  font-weight: 600;
  color: var(--kd-secondary-text);
  margin: 0;
}
.kd-break__value {
  margin: 0;
  font-family: "Fredoka", "Onest", system-ui, sans-serif;
  font-weight: 600;
  font-size: 22px;
}
.kd-break__meta {
  margin: 0;
  font-size: 13px;
  color: var(--kd-ink-3);
}
</style>

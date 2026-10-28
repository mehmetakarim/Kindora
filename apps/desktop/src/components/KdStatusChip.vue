<script setup lang="ts">
import { PhCheckCircle, PhCircleNotch, PhInfo, PhWarningCircle } from "@phosphor-icons/vue";
import { computed } from "vue";
import type { Tone } from "../agent/presentation";

// Durum asla yalnız renkle anlatılmaz: ikon + kelime + renk (design README).
const props = defineProps<{ tone: Tone; label: string }>();

const icon = computed(() => {
  switch (props.tone) {
    case "success":
      return PhCheckCircle;
    case "warning":
      return PhWarningCircle;
    case "info":
      return PhCircleNotch;
    case "neutral":
      return PhInfo;
  }
});
</script>

<template>
  <span class="kd-chip" :class="`kd-chip--${tone}`">
    <component
      :is="icon"
      :size="16"
      weight="bold"
      aria-hidden="true"
      :class="{ 'kd-spin': tone === 'info' }"
    />
    {{ label }}
  </span>
</template>

<style scoped lang="scss">
.kd-chip {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 4px 10px;
  border-radius: 999px;
  font-size: 13px;
  font-weight: 600;
  line-height: 1.4;
  white-space: nowrap;
  border: 1px solid transparent;
}
.kd-chip--success {
  background: var(--kd-success-tint);
  color: var(--kd-success-text);
}
.kd-chip--warning {
  background: var(--kd-warning-tint);
  color: var(--kd-warning-text);
  border-color: var(--kd-warning-line);
}
.kd-chip--info {
  background: var(--kd-primary-tint);
  color: var(--kd-primary-text);
}
.kd-chip--neutral {
  background: var(--kd-neutral-tint);
  color: var(--kd-ink-2);
}
.kd-spin {
  animation: kd-spin 1.4s linear infinite;
}
@keyframes kd-spin {
  to {
    transform: rotate(360deg);
  }
}
:global([data-kd-motion="reduced"]) .kd-spin {
  animation: none;
}
</style>

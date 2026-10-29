<script setup lang="ts">
import type { MascotLine } from "../agent/presentation";
import KdMascot from "./KdMascot.vue";

defineProps<{ line: MascotLine; motion: boolean }>();
</script>

<template>
  <section class="kd-card kd-mascot-panel" aria-labelledby="mascot-title">
    <h2 id="mascot-title" class="visually-hidden">Filiz</h2>
    <KdMascot :pose="line.pose" :size="132" :motion="motion" />
    <div
      v-if="line.text"
      class="kd-bubble"
      role="status"
      aria-live="polite"
      :data-moment="line.moment"
      data-testid="filiz-bubble"
    >
      <Transition name="kd-bubble" mode="out-in">
        <p :key="line.moment" class="kd-bubble__text">{{ line.text }}</p>
      </Transition>
    </div>
    <p class="kd-mascot-panel__note">Filiz · konsept çizim (nihai değil)</p>
  </section>
</template>

<style scoped>
.kd-mascot-panel {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 16px;
  text-align: center;
  padding: 28px 24px 20px;
}
.kd-bubble {
  position: relative;
  width: 100%;
  max-width: 360px;
  padding: 14px 18px;
  border-radius: 16px;
  background: var(--kd-secondary-tint);
  color: var(--kd-ink);
  font-family: "Fredoka", "Onest", system-ui, sans-serif;
  font-weight: 500;
  font-size: 16px;
  line-height: 1.45;
  min-height: 76px;
  display: flex;
  align-items: center;
  justify-content: center;
}
.kd-bubble::before {
  content: "";
  position: absolute;
  top: -8px;
  left: 50%;
  margin-left: -8px;
  border: 8px solid transparent;
  border-top: 0;
  border-bottom-color: var(--kd-secondary-tint);
}
.kd-bubble__text {
  margin: 0;
}
.kd-mascot-panel__note {
  margin: 0;
  font-size: 12px;
  color: var(--kd-ink-3);
}
.kd-bubble-enter-active,
.kd-bubble-leave-active {
  transition:
    opacity 200ms cubic-bezier(0.2, 0.8, 0.2, 1),
    transform 200ms cubic-bezier(0.2, 0.8, 0.2, 1);
}
.kd-bubble-enter-from {
  opacity: 0;
  transform: translateY(6px);
}
.kd-bubble-leave-to {
  opacity: 0;
}
</style>

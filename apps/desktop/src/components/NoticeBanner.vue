<script setup lang="ts">
import { PhBell, PhCoffee, PhHourglassMedium, PhX } from "@phosphor-icons/vue";
import { computed } from "vue";
import { noticeText } from "../agent/presentation";
import type { Notice, ScreenTimeStatus } from "../agent/types";
import { tr } from "../i18n/tr";

// Tek bildirim yuvası: üst üste bildirim açılmaz; kapatılan kimlik bir daha gösterilmez.
const props = defineProps<{ notice: Notice; status: ScreenTimeStatus }>();
const emit = defineEmits<{ dismiss: [id: string] }>();

const icon = computed(() => {
  switch (props.notice.kind) {
    case "BREAK_STARTED":
    case "BREAK_COMPLETED":
      return PhCoffee;
    case "LIMIT_REACHED":
      return PhHourglassMedium;
    default:
      return PhBell;
  }
});
</script>

<template>
  <div
    class="kd-notice"
    :class="`kd-notice--${notice.kind.toLowerCase()}`"
    role="status"
    aria-live="polite"
    data-testid="notice"
    :data-notice-id="notice.id"
  >
    <component :is="icon" :size="22" weight="bold" aria-hidden="true" />
    <span class="kd-notice__text">{{ noticeText(notice, status) }}</span>
    <button
      type="button"
      class="btn btn-sm btn-light kd-notice__close"
      data-testid="notice-dismiss"
      @click="emit('dismiss', notice.id)"
    >
      <PhX :size="14" weight="bold" aria-hidden="true" />
      {{ tr.notice.dismiss }}
    </button>
  </div>
</template>

<style scoped>
.kd-notice {
  display: flex;
  align-items: center;
  gap: 12px;
  flex-wrap: wrap;
  padding: 12px 16px;
  border-radius: 16px;
  background: var(--kd-secondary-tint);
  color: var(--kd-ink);
  border: 1px solid var(--kd-line);
  animation: kd-notice-in 200ms cubic-bezier(0.2, 0.8, 0.2, 1);
}
.kd-notice--break_first_warning,
.kd-notice--break_final_warning {
  background: var(--kd-warning-tint);
  border-color: var(--kd-warning-line);
}
.kd-notice--break_completed {
  background: var(--kd-success-tint);
}
.kd-notice__text {
  flex: 1 1 240px;
  font-weight: 600;
}
.kd-notice__close {
  display: inline-flex;
  align-items: center;
  gap: 4px;
}
@keyframes kd-notice-in {
  from {
    opacity: 0;
    transform: translateY(6px);
  }
}
</style>

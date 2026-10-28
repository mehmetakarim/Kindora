<script setup lang="ts">
// Logo: design_files içindeki `logo()` (filiz işaretli çakıl), indigo zemin üzerinde.
withDefaults(defineProps<{ size?: number }>(), { size: 36 });
</script>

<template>
  <span
    class="kd-logo"
    :style="{ width: `${String(size)}px`, height: `${String(size)}px` }"
    aria-hidden="true"
  >
    <svg viewBox="0 0 32 32" :width="size * 0.72" :height="size * 0.72">
      <path
        d="M16 12 C24 12 27 18 27 22 C27 27 22 29 16 29 C10 29 5 27 5 22 C5 18 8 12 16 12Z"
        fill="#FFFFFF"
      />
      <path d="M16 12 L16 8" stroke="#BFF2E9" stroke-width="1.8" stroke-linecap="round" />
      <path d="M16 9 C13 5 9 5.5 8 6 C8.5 9.5 13 10.5 16 9Z" fill="#7FE3D2" />
      <path d="M16 8.5 C18.5 4 23 4 24 4.5 C23.5 8.5 19 10 16 8.5Z" fill="#BFF2E9" />
    </svg>
  </span>
</template>

<style scoped>
.kd-logo {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border-radius: 10px;
  background: var(--kd-primary);
  flex-shrink: 0;
}
</style>

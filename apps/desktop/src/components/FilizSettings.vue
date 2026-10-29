<script setup lang="ts">
// Filiz tercihleri (tasarım Phase 3 "Filiz" sekmesindeki konuşma ve hareket
// ayarlarının sade hâli). Kuralları değiştirmez.
const quiet = defineModel<boolean>("quiet", { required: true });
const reduceMotion = defineModel<boolean>("reduceMotion", { required: true });
</script>

<template>
  <section class="kd-card kd-prefs" aria-labelledby="prefs-title">
    <h2 id="prefs-title" class="kd-prefs__title">Filiz ayarları</h2>
    <div class="form-check form-switch">
      <input
        id="pref-quiet"
        v-model="quiet"
        class="form-check-input"
        type="checkbox"
        role="switch"
        data-testid="pref-quiet"
      />
      <label class="form-check-label" for="pref-quiet"
        >Sessiz mod — yalnız mola ve süre uyarıları</label
      >
    </div>
    <div class="form-check form-switch">
      <input
        id="pref-motion"
        v-model="reduceMotion"
        class="form-check-input"
        type="checkbox"
        role="switch"
        data-testid="pref-motion"
      />
      <label class="form-check-label" for="pref-motion">Hareketi azalt</label>
    </div>
    <p class="kd-prefs__note">Sesli bildirim: kapalı (bu sürümde ses yok).</p>
  </section>
</template>

<style scoped>
.kd-prefs {
  padding: 20px 24px;
  border-radius: 28px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.kd-prefs__title {
  font-size: 15px;
  font-weight: 600;
  margin: 0 0 4px;
}
.kd-prefs__note {
  margin: 0;
  font-size: 13px;
  color: var(--kd-ink-3);
}
</style>

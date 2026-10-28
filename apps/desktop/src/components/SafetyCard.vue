<script setup lang="ts">
import { PhShieldCheck } from "@phosphor-icons/vue";
</script>

<template>
  <section class="kd-card kd-safety" aria-labelledby="safety-title">
    <h2 id="safety-title" class="kd-safety__title">
      <PhShieldCheck :size="20" weight="bold" aria-hidden="true" />
      Bu sürüm hakkında
    </h2>
    <ul class="kd-safety__list">
      <li>
        Bu önizleme <strong>hiçbir kısıtlama uygulamaz</strong>: ekranı kilitlemez, uygulama veya
        site engellemez, süre sınırı koymaz.
      </li>
      <li>
        Pencereyi kapatmak güvenlidir. Kindora Agent Windows servisi olarak ayrıca yönetilir ve bu
        pencere kapanınca durmaz.
      </li>
      <li>
        Kindora, kullandığınız Windows hesabında çalışır. Hesap yönetici yetkisine sahipse Kindora
        yöneticiye karşı değiştirilemez bir kilit sağlayamaz.
      </li>
    </ul>
  </section>
</template>

<style scoped>
.kd-safety {
  padding: 20px 24px;
}
.kd-safety__title {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 16px;
  font-weight: 700;
  margin: 0 0 10px;
  color: var(--kd-ink);
}
.kd-safety__list {
  margin: 0;
  padding-left: 20px;
  color: var(--kd-ink-2);
  display: grid;
  gap: 6px;
}
</style>

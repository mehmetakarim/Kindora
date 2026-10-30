<script setup lang="ts">
import {
  backgroundText,
  formatDuration,
  formatLastSuccess,
  policyText,
  reporterText,
  unknownReasonText,
  usageStateLabel,
} from "../agent/presentation";
import type { ScreenTimeStatus } from "../agent/types";

// Teknik tanılama: ölçüm, politika ve yerel depolama ayrıntıları.
defineProps<{ status: ScreenTimeStatus | null; now: number }>();
</script>

<template>
  <section class="kd-card kd-measure" aria-labelledby="measure-title">
    <h2 id="measure-title" class="kd-measure__title">Kullanım süresi ölçümü</h2>
    <dl v-if="status" class="kd-measure__grid">
      <dt>Anlık durum</dt>
      <dd data-testid="diag-state">{{ usageStateLabel(status.state) }}</dd>
      <dt>Ölçüm kaynağı</dt>
      <dd data-testid="diag-source">
        {{
          status.measurement.source === "SESSION_HELPER"
            ? "Oturum yardımcısı (son girdi süresi)"
            : "Yok"
        }}
      </dd>
      <dt>Arka plan ölçümü</dt>
      <dd data-testid="diag-background">{{ backgroundText(status) }}</dd>
      <dt>Etkinlik raporlayıcısı</dt>
      <dd data-testid="diag-reporter">{{ reporterText(status) }}</dd>
      <dt>Son (yeniden) bağlantı</dt>
      <dd data-testid="diag-reporter-since">
        {{
          status.measurement.reporter
            ? formatLastSuccess(status.measurement.reporter.since_ms, now)
            : "—"
        }}
      </dd>
      <dt>Reddedilen ikinci raporlayıcı</dt>
      <dd data-testid="diag-competing">
        {{ String(status.measurement.competing_reports ?? 0) }} rapor (yalnız biri sayılır)
      </dd>
      <dt>Ölçülemeyen süre (bugün)</dt>
      <dd data-testid="diag-unknown">{{ formatDuration(status.today.unknown_secs) }}</dd>
      <dt>Ölçülemiyorsa nedeni</dt>
      <dd>{{ unknownReasonText(status.measurement.unknown_reason) }}</dd>
      <dt>Son helper raporu</dt>
      <dd>
        {{
          status.measurement.last_report_age_ms === undefined
            ? "Yok"
            : `${String(Math.round(status.measurement.last_report_age_ms / 1000))} sn önce`
        }}
      </dd>
      <dt>Son ölçüm</dt>
      <dd>{{ formatLastSuccess(status.computed_at_ms, now) }}</dd>
      <dt>Gün</dt>
      <dd>
        {{ status.day
        }}<template v-if="status.clock_adjusted_today"> · saat değişikliği kaydedildi</template>
      </dd>
      <dt>Bugünün dağılımı</dt>
      <dd data-testid="diag-breakdown">
        aktif {{ formatDuration(status.today.active_secs) }} · boşta
        {{ formatDuration(status.today.idle_secs) }} · kilitli
        {{ formatDuration(status.today.locked_secs) }} · uyku
        {{ formatDuration(status.today.suspended_secs) }} · ölçülemeyen
        {{ formatDuration(status.today.unknown_secs) }}
      </dd>
      <dt>Kurallar</dt>
      <dd data-testid="diag-policy">{{ policyText(status) }}</dd>
      <dt>Boşta kalma eşiği</dt>
      <dd>{{ formatDuration(status.policy.idle_threshold_secs) }}</dd>
      <dt>Uyarılar</dt>
      <dd>
        {{ formatDuration(status.policy.first_warning_secs) }} ve
        {{ formatDuration(status.policy.final_warning_secs) }} kala
      </dd>
      <dt>Yerel depolama</dt>
      <dd data-testid="diag-storage">
        {{
          status.storage.persistent
            ? `SQLite, şema v${String(status.storage.schema_version)}`
            : "Yalnız bellek — yeniden başlatmada kayıtlar korunmaz"
        }}<template v-if="status.storage.recovered"> · bozuk veritabanı kurtarıldı</template>
      </dd>
    </dl>
    <p v-else class="kd-measure__empty">Bağlantı yok — ölçüm bilgisi alınamıyor.</p>
  </section>
</template>

<style scoped>
.kd-measure {
  padding: 24px;
}
.kd-measure__title {
  font-size: 20px;
  font-weight: 700;
  margin: 0 0 16px;
}
.kd-measure__grid {
  display: grid;
  grid-template-columns: minmax(150px, max-content) 1fr;
  gap: 10px 20px;
  margin: 0;
}
.kd-measure__grid dt {
  color: var(--kd-ink-3);
  font-weight: 500;
}
.kd-measure__grid dd {
  margin: 0;
  font-weight: 600;
  overflow-wrap: anywhere;
}
.kd-measure__empty {
  margin: 0;
  color: var(--kd-ink-2);
}
@media (max-width: 560px) {
  .kd-measure__grid {
    grid-template-columns: 1fr;
    gap: 2px;
  }
  .kd-measure__grid dd {
    margin-bottom: 10px;
  }
}
</style>

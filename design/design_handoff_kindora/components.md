# Kindora — Bileşen envanteri

Durum sütunları: D = Default, H = Hover, A = Active/Selected, F = Focus, X = Disabled, L = Loading, E = Error, S = Success.
Referans görseller: `design_files/Kindora Phase 6 - Design Review.dc.html` (bölüm 02).

| Bileşen (Vue / Flutter) | Kullanım | Durumlar | Notlar |
|---|---|---|---|
| `KdSidebar` / `NavigationBar` | Ana gezinme | D H A F, rozet | Aktif: zemin #EEF0FC, metin #3F50B8 600. Mobil: 5 sekme, indicator pill 60×32 |
| `KdTopbar` | Başlık + çocuk seçici + bildirim | D | 72px; çocuk seçici dropdown, "Çocuk profili ekle" |
| `KdChildSwitcher` | Çocuk seçimi | D H A | Mobilde bottom sheet |
| `KdChildDeviceCard` | Çocuk + cihaz + mod | online / offline / unknown | Durum çipi + "Son bağlantı" (nowrap) |
| `KdModeSelector` | Serbest / Ders / Mola / Kısıtlı | D A X, **pending** | Pending: amber zemin, kesik #F1D49C çerçeve, kum saati ikonu |
| `KdUsageProgress` | Süre çubuğu | normal / az kaldı (<%15 amber) / ek süre (teal) / doldu (lavanta) | role=progressbar, aria-valuenow |
| `KdTimeRing` | Kalan süre halkası | aynı | Çocuk 200px, mobil 84–88px |
| `KdQuickAction` | Hızlı işlem kutusu | D H X + komut durumları: ready, sending, queued, pending, applied, failed, unverified | Disabled: #F4F6FA + #5D6680 + neden metni (opaklık kullanma) |
| `KdButton` | primary / secondary / danger / text | D H A F X L E S | Bkz. Phase 6 tablo. Loading'de etiket "Gönder…" + spinner |
| `KdInput` | Metin, alan adı | D F warn E L X | Uyarı (amber) ≠ hata (kırmızı) |
| `KdToggle` | Ayarlar | off on F locked L | Locked: #9AA6E6, tıklanınca açıklama toast'ı |
| `KdStatusChip` | Tüm durumlar | online offline unknown · sending queued pending applied failed unverified · study break | İkon + kelime + renk zorunlu |
| `KdPermissionRequestCard` | Talep | D L S rejected E | Süre seçici + not çipleri + serbest not |
| `KdConfirmModal` / `KdConfirmSheet` | Kritik eylem | D, danger | Kilit, eşleştirme kaldırma, profil silme |
| `KdToast` / `SnackBar` | Geri bildirim | info ok wait warn err | err ve warn (Doğrulanamadı) kapatılana kadar kalır |
| `KdBanner` | Sayfa üstü durum | offline / no-connection | Kontrol / Yeniden dene düğmesi |
| `KdRuleEditor` | Site kuralı | boş / geçerli / uyarı / hata / mükerrer | Önizleme cümlesi |
| `KdWeeklySchedule` | 7×16 ızgara | fırça seçimi, dirty | Klavye: ok tuşları + Boşluk |
| `KdActivityTimeline` | Olaylar | info / wait / err | Gün gruplu |
| `KdEmptyState` / `KdLoadingState` / `KdOfflineState` | — | — | Skeleton shimmer 1,4 sn |
| `KdMascot` | Filiz | 10 durum | `mascot_brief/` |
| `KdSpeechBubble` | Maskot balonu | info / reminder (eylemli) / waiting / access | 6 sn otomatik kapanma (eylemli hariç) |
| `KdAchievementBadge` | Rozet | earned / in-progress / locked / focus | Kesik halka = henüz kazanılmadı |
| `KdDesktopWidget` | Widget | compact / expanded / mascot-only, warn, transparent, hidden, focus | Köşeye yapışma, tam ekranda gizlenme. Klavye: ok tuşları köşe, Enter genişlet, Esc küçült |
| `KdSidebar` (dar) | < 1240 px | 72 px ikon rayı | title + aria-label zorunlu |
| `KdChartTable` | Grafik veri alternatifi | kapalı / açık | Her grafikte "Tablo olarak göster" |
| `KdBreakScreen` | Mola | timeup / running / pending request / rejected | Çalışma açık kalır |

`Mobile` / `Flutter` karşılıkları sonraki sürüm adayıdır, MVP dışıdır.

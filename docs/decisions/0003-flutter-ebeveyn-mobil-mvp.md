# ADR-0003 — Ebeveyn mobil uygulaması Flutter ile ve MVP kapsamında

- **Tarih:** 2026-10-27 (kayıt tarihi)
- **Durum:** Kabul edildi (ürün/teknoloji tercihi) — dağıtım ve push **doğrulanmadı**
- **Karar türü:** Ürün/teknoloji tercihi

## Bağlam ve problem

Ebeveyn bilgisayar başında olmayabilir; cihaz durumu görme, anlık kısıtlama, ek süre ve talep yanıtlama mobilden yapılabilmelidir. Kaynak: [context §7](../../design/KINDORA_PROJECT_CONTEXT.md#7-ebeveyn-mobil-uygulaması--flutter).

**Kaydedilen çelişki:** Claude Design teslimatı ([README](../../design/design_handoff_kindora/README.md), Acceptance Report, components.md) mobil uygulamayı "sonraki sürüm adayı, MVP dışı" ve Flutter'ı "öneri, onaylanmamış teknoloji" olarak gösterir. Daha yeni ve açık kullanıcı kararı olan [KINDORA_PROJECT_CONTEXT.md §20](../../design/KINDORA_PROJECT_CONTEXT.md#20-tasarım-referansları-ve-durumları) Flutter'ı onaylı ve **MVP kapsamında** kaydeder. Açık kullanıcı kararı esas alınır; tasarım dosyaları değiştirilmedi.

## Değerlendirilen seçenekler

Alternatif mobil teknolojilerin karşılaştırması belgelenmemiş.

## Seçilen yaklaşım ve gerekçe

Flutter (Android/iOS), MVP'ye dahil. Masaüstü ebeveynle aynı Supabase yetki modeli, komut şeması ve durum makinesi. Gerekçe (context): tek kod tabanıyla iki platform; tasarım teslimatında Flutter tema referansı mevcut. Ayrıntılı gerekçe belgelenmemiş.

## Sonuçlar ve ödünleşimler

- Dart ↔ TypeScript arasında tip paylaşımı otomatik değil; sözleşme (OpenAPI/JSON Schema vb.) gerekir.
- Push bildirim (FCM/APNs) altyapısı gerekebilir; Realtime arka plan teslimini garanti etmez.
- Mobil tasarım dosyaları eski kilit metinlerini içerir (Acceptance Report bulgusu); MVP'ye alındığı için güncelleme gerekecek.
- Mağaza hesapları/imzalama maliyeti $0 kabul edilmez.

## Bilinmeyenler / doğrulama gereken noktalar

- Push sağlayıcısı, maliyet, mağaza gereklilikleri, durum yönetimi kütüphanesi — **GATE E**.

## Güncelleme — 2026-10-27 (Phase M0.1, kullanıcı kararı A)

Kullanıcı açıkça yeniden onayladı: Flutter ebeveyn mobil uygulaması **MVP kapsamındadır** ve **iOS ile Android** ebeveyn kontrolünü sağlar. Tasarım teslimatındaki "MVP dışı / teknoloji onaysız" ifadeleri tarihsel kayıttır; güncel karar bu ADR'dir.

## İlişkili kararlar

- [ADR-0002](0002-supabase-sqlite-gorev-ayrimi.md), [ADR-0004](0004-claude-design-tasarim-yonu-kabulu.md).

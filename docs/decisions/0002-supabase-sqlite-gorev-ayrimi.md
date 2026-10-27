# ADR-0002 — Supabase merkezi backend, SQLite yerel/çevrimdışı veri

- **Tarih:** 2026-10-27 (kayıt tarihi; karar daha önce verildi, kesin tarih belgelenmemiş)
- **Durum:** Kabul edildi (teknoloji tercihi) — şema, RLS ve senkron protokolü **Önerildi**
- **Karar türü:** Ürün/teknoloji tercihi

## Bağlam ve problem

Ebeveyn istemcileri (masaüstü + mobil) ile çocuk cihazı arasında güvenli iletişim, merkezi veri ve kimlik yönetimi gerekir; aynı zamanda internet yokken kurallar çocuk cihazında uygulanmaya devam etmelidir. Kaynak: [context §10–§11](../../design/KINDORA_PROJECT_CONTEXT.md#10-merkezi-backend--supabase).

## Değerlendirilen seçenekler

- Supabase (Postgres + Auth + Edge Functions + Realtime).
- Ayrı PHP/VPS sunucusu — context'te "erken aşamada zorunluluk yok" diye anılır; ayrıntılı karşılaştırma belgelenmemiş.

## Seçilen yaklaşım ve gerekçe

- **Supabase:** Auth, aile/tenant modeli, politika sürümleri, komut + receipt kayıtları, talepler, özet raporlar, audit. Gerekçe: ayrı sunucu kurmadan başlama, ilişkisel model, RLS, sıfıra yakın başlangıç maliyeti hedefi.
- **SQLite (çocuk cihazında, Rust servis erişimiyle):** son geçerli politika, süre sayaçları, işlenmiş komut kimlikleri, bekleyen olaylar.
- **Otorite:** Politika değiştirme yetkisi sunucuda yetkilendirilmiş ebeveyn kararı; fiili cihaz durumu ve sayaç yerel ajan; çelişki sürüm + yetki tabanlı çözülür.

## Sonuçlar ve ödünleşimler

- Realtime yalnızca uyandırma sinyalidir; kesin durum kalıcı kayıt + cihaz ACK ile.
- Çevrimdışıyken verilen ebeveyn kararı cihazda hemen uygulanmış sayılmaz.
- Ücretsiz kota hedeftir, garanti değildir; lansman öncesi yeniden kontrol edilir.
- Flutter (Dart) ve TypeScript arasında ortak sözleşme için şema/codegen yaklaşımı gerekecek (seçilmedi).

## Bilinmeyenler / doğrulama gereken noktalar

- Nihai SQL şeması, RLS politikaları, migration stratejisi, Supabase bölgesi — **GATE B**.
- SQLite ACL, saat sapması, bozuk DB kurtarma — **GATE A**.

## İlişkili kararlar

- [ADR-0001](0001-kindora-kullanim-kilidi-ana-kontrol.md), [ADR-0003](0003-flutter-ebeveyn-mobil-mvp.md).

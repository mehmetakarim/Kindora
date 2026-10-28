# ADR-0010 — Paylaşılan sözleşme: JSON Schema'dan TypeScript / Dart / Rust üretimi

- **Tarih:** 2026-10-28
- **Durum:** Önerildi
- **Karar türü:** Teknik uygulama yöntemi
- **İlgili T0 belgesi:** [T0-E](../feasibility/T0-E-client-integration.md) §2, §8

## Problem ve bağlam

Parent Desktop (TypeScript), Parent Mobile (Dart), Edge Functions (TypeScript/Deno) ve Windows Agent (Rust) aynı komut, receipt, talep, politika ve hata modelini kullanmalı. Tip eşitliği diller arasında otomatik değildir (context §15).

## Değerlendirilen seçenekler

1. **JSON Schema tek kaynak + kod üretimi** (TS, Dart, Rust).
2. OpenAPI (yalnız HTTP uçları için) — IPC ve politika gövdesini kapsamaz; JSON Schema ile tamamlayıcı olabilir.
3. Protocol Buffers — güçlü tipler; Supabase/PostgREST JSON dünyasıyla sürtünme.
4. Elle senkronize modeller — sapma riski → reddedilir.

## Önerilen yaklaşım

Seçenek 1: `packages/shared-contracts/` altında sürümlü JSON Schema; her mesajda `schema_version`; Türkçe UI metinleri için ortak hata kodu sözlüğü; kırıcı değişiklik = yeni sürüm. Edge Function uçları gerekirse OpenAPI ile belgelenir ve şemaları referans alır. Kod üretici araç seçimi P5/P6 öncesi ayrı değerlendirilir.

## Avantajlar

Tek doğruluk kaynağı; IPC, HTTP ve politika gövdesini aynı şemayla doğrulama; dört dil için aynı durum makinesi.

## Riskler ve ödünleşimler

- Dart ve Rust için JSON Schema kod üreticilerinin olgunluğu değişken.
- Şema yönetimi ek süreç disiplini gerektirir.

## Açık doğrulama gereksinimleri

- Üç dilde örnek komut/receipt modelinin üretilip karşılıklı serileştirme testleri.

## İlişkili kararlar

[ADR-0003](0003-flutter-ebeveyn-mobil-mvp.md), [ADR-0007](0007-servis-helper-ipc-named-pipe.md), [ADR-0009](0009-kalici-komut-kuyrugu-realtime-uyandirma.md).

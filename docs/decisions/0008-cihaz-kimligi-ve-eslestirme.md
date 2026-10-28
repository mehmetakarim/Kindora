# ADR-0008 — Cihaz kimliği ve güvenli eşleştirme

- **Tarih:** 2026-10-28
- **Durum:** Önerildi
- **Karar türü:** Teknik uygulama yöntemi
- **İlgili T0 belgesi:** [T0-C](../feasibility/T0-C-supabase-architecture.md) §2–§3

## Problem ve bağlam

Çocuk bilgisayarı Supabase'e kendi kimliğiyle bağlanmalı, yalnız kendi komutlarını ve politikasını okuyabilmeli, iptal edilebilmeli. Paylaşılan anahtar veya istemciye gömülü `service_role` kabul edilemez (context §10.3).

## Değerlendirilen seçenekler

1. **C1 — Cihaz başına Supabase Auth kullanıcısı** + Auth Hook ile `role=device`, `device_id` claim'leri; RLS bu claim'leri kullanır ([Auth Hooks](https://supabase.com/docs/guides/auth/auth-hooks), [JWTs](https://supabase.com/docs/guides/auth/jwts)).
2. **C2 — Cihaz anahtar çifti + Edge Function'ın verdiği kısa ömürlü token.**
3. **C3 — Paylaşılan API anahtarı** — reddedilir.

## Önerilen yaklaşım

C1 temel kimlik + cihazda yerel anahtar çifti (DPAPI ile korunan) ile bağlama (C2'nin faydası). Eşleştirme: ebeveyn oturumu ile Edge Function'da üretilen 6 haneli, yalnız hash'i saklanan, 10 dk geçerli, tek kullanımlık, deneme sınırlı kod → cihaz açık anahtarını gönderir → ebeveyn cihaz adını onaylar → cihaz aktif; tüm adımlar `audit_events`'e. İptal: `revoked_at`, oturum/refresh token iptali, bekleyen komutların iptali. Çocuk için ayrı Supabase hesabı yok.

## Avantajlar

Supabase'in yerleşik oturum/yenileme altyapısı; RLS ile doğal entegrasyon; cihaz bağlama ile token kopyalamaya karşı ek katman.

## Riskler ve ödünleşimler

- Cihazlar MAU kotasına sayılır (Free 50.000 — tek aile için önemsiz).
- Auth Hook ve claim tabanlı RLS hataları aileler arası sızıntıya yol açabilir → otomatik negatif testler zorunlu.
- Çevrimdışı cihazda iptal hemen etkili olmaz.

## Açık doğrulama gereksinimleri

- Supabase'de cihaz kullanıcısı türünün (anonim vb.) uygunluğu ve claim akışı (PoC-C).
- Brute-force/oran sınırlama ve kod tekrar kullanımı testleri.
- DPAPI makine kapsamında anahtar saklama.

## İlişkili kararlar

[ADR-0002](0002-supabase-sqlite-gorev-ayrimi.md), [ADR-0009](0009-kalici-komut-kuyrugu-realtime-uyandirma.md).

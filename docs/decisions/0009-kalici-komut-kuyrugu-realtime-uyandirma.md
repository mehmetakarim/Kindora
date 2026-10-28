# ADR-0009 — Kalıcı komut kuyruğu, Realtime yalnız uyandırma sinyali

- **Tarih:** 2026-10-28
- **Durum:** Önerildi
- **Karar türü:** Teknik uygulama yöntemi
- **İlgili T0 belgesi:** [T0-C](../feasibility/T0-C-supabase-architecture.md) §4–§5, [T0-E](../feasibility/T0-E-client-integration.md) §3

## Problem ve bağlam

Ebeveyn komutları cihaz çevrimdışıyken de kaybolmamalı, bir kez uygulanmalı ve sonucu güvenilir biçimde raporlanmalı. Realtime mesajları kalıcı değildir; Postgres Changes DELETE olaylarını RLS ile filtrelemez; Broadcast yetkisi `realtime.messages` politikaları ve private kanalla sağlanır ([Realtime Authorization](https://supabase.com/docs/guides/realtime/authorization)).

## Değerlendirilen seçenekler

1. Komutu yalnız Realtime ile göndermek — kayıp riski → reddedilir.
2. **Kalıcı `device_commands` tablosu + Realtime uyandırma + periyodik çekme.**
3. Yalnız periyodik çekme (polling) — basit ama gecikmeli.

## Önerilen yaklaşım

Seçenek 2: Ebeveyn istemcisi `command_id` (UUID) üretir → Edge Function yetki/aile/cihaz/tür/hız kontrolü yapıp kalıcı kayıt oluşturur → veritabanı tetikli **private Broadcast** "yeni komut var" sinyali verir → cihaz komutu RLS'li okumayla çeker, `expires_at`/`policy_version`/tekrar kontrolü yapar, uygular, receipt gönderir. Realtime yoksa periyodik çekme (30–60 sn) ve bağlanınca senkron. 30 sn içinde receipt yoksa UI "Doğrulanamadı"; "Durumu kontrol et" yeni komut üretmez.

## Avantajlar

Çevrimdışı dayanıklılık; idempotency; Realtime kotasının düşük kullanımı; tasarımdaki 7 durumlu modelle birebir uyum.

## Riskler ve ödünleşimler

- Yanlış RLS politikası ile sinyal sızıntısı (içerik taşınmadığı için etkisi sınırlı).
- Polling aralığı ↔ gecikme ↔ kota dengesi.
- Free planda 1 haftalık hareketsizlik duraklatması senkronu durdurur.

## Açık doğrulama gereksinimleri

- Postgres Changes vs DB-tetikli Broadcast gecikme ve güvenilirlik ölçümü (PoC-C).
- Tekrar, TTL, offline kuyruk ve "Doğrulanamadı" senaryoları uçtan uca (P5).

## İlişkili kararlar

[ADR-0008](0008-cihaz-kimligi-ve-eslestirme.md), [ADR-0010](0010-paylasilan-sozlesme-json-schema.md).

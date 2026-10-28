# T0-C — Supabase ve Güvenli Cihaz İletişimi

> Durum: **Taslak / öneri** · Kapı: GATE T0-C · İlgili: [ADR-0002](../decisions/0002-supabase-sqlite-gorev-ayrimi.md), [ADR-0008](../decisions/0008-cihaz-kimligi-ve-eslestirme.md), [ADR-0009](../decisions/0009-kalici-komut-kuyrugu-realtime-uyandirma.md)
> Supabase projesi oluşturulmadı; SQL yazılmadı. Varlıklar kavramsaldır.

## 1. Varlık modeli (kavramsal)

| Varlık | Ana alanlar (öneri) | Sahiplik / RLS sınırı |
|---|---|---|
| `families` | id, name, created_by | Üye olanlar okur; yalnız owner yönetir |
| `family_members` | family_id, user_id, role (`owner`/`parent`), status | Aile üyeleri okur; owner ekler/çıkarır |
| `child_profiles` | id, family_id, display_name, birth_year(ops.), avatar | Ebeveynler okur/yazar; **çocuğun Supabase hesabı yok** (öneri, bkz. §3) |
| `devices` | id, family_id, child_profile_id, name, platform, status, last_seen_at, revoked_at | Ebeveynler okur/yönetir; cihaz yalnız kendi satırını okur, `last_seen_at`'i Edge Function üzerinden günceller |
| `device_pairings` | id, family_id, child_profile_id, code_hash, expires_at, attempts, consumed_at | Yalnız Edge Function (service role); istemci doğrudan erişmez |
| `policies` | id, child_profile_id, version, body(jsonb), signature?, created_by, created_at | Ebeveyn yazar (Edge Function); cihaz kendi çocuğunun en güncel sürümünü okur |
| `device_commands` | id(uuid, istemci üretir), device_id, type, payload, issued_by, issued_at, expires_at, policy_version, status | Ebeveyn **yalnız Edge Function ile** oluşturur; cihaz yalnız kendisine ait olanları okur |
| `command_receipts` | command_id, device_id, outcome(`applied`/`rejected`/`expired`/`failed`), detail, device_time, received_at | Yalnız cihaz yazar (Edge Function); ebeveyn okur |
| `permission_requests` | id, device_id, child_profile_id, kind(`extra_time`/`site`), reason, amount, status, decided_by, decided_at | Cihaz oluşturur; ebeveyn karar verir (Edge Function) |
| `usage_summaries` | device_id, date, active_minutes, category_minutes(jsonb) | Cihaz yazar; ebeveyn okur. **URL/içerik yok** |
| `audit_events` | id, family_id, actor_type, actor_id, action, target, at | Yalnız sunucu yazar; ebeveyn okur; değiştirilemez (append-only) |

## 2. Yetki modeli

- **Ebeveyn:** Supabase Auth kullanıcısı; `family_members` üyeliğiyle yalnız kendi ailesini görür.
- **Çocuk:** MVP'de ayrı Supabase hesabı **önerilmez**; çocuk cihaz üzerinden temsil edilir. Böylece "çocuk kendine izin verir" yolu şemada yoktur.
- **Cihaz:** Ayrı bir kimlik türü (bkz. §3). Yalnız kendi `devices` satırı, kendi komutları, kendi çocuğunun politikası.
- **Kritik yazmalar** (komut, politika, talep kararı, eşleştirme, eşleştirme kaldırma) yalnızca **Edge Functions** üzerinden; istemcilerin bu tablolara doğrudan `insert/update` RLS izni yok.
- `service_role` anahtarı yalnız Edge Functions ortamında; hiçbir istemciye gömülmez.
- **RLS test kuralı:** Her tablo için "aile A kullanıcısı aile B verisini okuyamaz/yazamaz" ve "cihaz X, cihaz Y'nin komutunu okuyamaz" otomatik testleri kabul kriteridir.

Kaynak: [Realtime Authorization](https://supabase.com/docs/guides/realtime/authorization), [JWTs](https://supabase.com/docs/guides/auth/jwts), [Auth Hooks](https://supabase.com/docs/guides/auth/auth-hooks).

## 3. Cihaz kimliği ve eşleştirme (öneri — ADR-0008)

**Seçenekler:**

| Seçenek | Açıklama | Değerlendirme |
|---|---|---|
| C1 — Cihaz başına Supabase Auth kullanıcısı (ör. anonim/e-postasız kullanıcı + custom claim `device_id`, `role=device`) | Auth Hook ile JWT'ye claim eklenir; RLS `auth.jwt()->>'device_id'` kullanır | Supabase'in yerleşik oturum/yenileme mekanizmasını kullanır; MAU sayımına girer (Free: 50.000) |
| C2 — Cihaz anahtar çifti + Edge Function'ın imzaladığı kısa ömürlü token | Cihaz özel anahtarı yerelde (DPAPI/TPM) | Daha güçlü cihaz bağlama; özel token doğrulama ve RLS entegrasyonu daha karmaşık |
| C3 — Paylaşılan API anahtarı | Tüm cihazlar aynı anahtar | **Reddedilir** |

**Öneri:** C1 + cihazda ek anahtar çifti (C2'nin bağlama özelliği) ile hibrit; kesin seçim PoC-C sonrası.

**Eşleştirme akışı (öneri):**
1. Ebeveyn (oturum açık) "Cihaz ekle" → Edge Function 6 haneli kod üretir; yalnız **hash**'i saklanır; `expires_at` = 10 dk; deneme sınırı (ör. 5); tek kullanımlık.
2. Çocuk PC kurulumu kodu girer + cihaz açık anahtarını gönderir.
3. Edge Function kodu doğrular → cihaz kimliği oluşturur → kodu tüketir → ebeveyne "cihazı doğrula: <cihaz adı>" onayı gösterir (Phase 5 eşleştirme modalı adım 3).
4. Ebeveyn onaylayınca cihaz aktif olur; `audit_events` kaydı.
5. Hız sınırlama: IP + aile başına; brute-force'a karşı kilitleme.

**İptal / kayıp cihaz:** Ebeveyn "eşleştirmeyi kaldır" → `devices.revoked_at`, cihaz oturumu/refresh token iptal, bekleyen komutlar `cancelled`. Cihaz çevrimdışıyken iptal yerelde **hemen etkili olmaz** [Çıkarım]; cihazın yerel davranışı T0-D'de.

## 4. Komut iletimi (öneri — ADR-0009)

```text
Parent (Desktop/Flutter) ── POST /issue-command {command_id, device_id, type, payload, expires_in}
        │ Edge Function: JWT doğrula → aile üyeliği → cihaz aynı aile mi → type beyaz listede mi → hız sınırı
        ▼
device_commands (status=pending) ──► Realtime (private Broadcast / Postgres Changes) = yalnız UYANDIRMA
        │
Child Agent: uyandırma VEYA periyodik çekme (ör. 30–60 sn) VEYA bağlanınca senkron
        │ GET pending commands (RLS: yalnız kendi cihazı)
        │ Doğrula: device_id, expires_at, policy_version, command_id daha önce uygulandı mı (SQLite)
        │ Uygula (yerel) → SQLite'a sonuç yaz (atomik)
        ▼
POST /command-receipt {command_id, outcome, device_time} → command_receipts + device_commands.status
        ▼
Parent UI: receipt'i okur → Uygulandı / Başarısız; süre aşımında Doğrulanamadı
```

**Kurallar:**
- Realtime mesajı **komutun kendisi değildir**; kaybolabilir. Kalıcı kayıt `device_commands` tablosudur. Realtime, Postgres Changes DELETE olaylarını RLS ile filtrelemez ve Broadcast yetkisi `realtime.messages` politikalarıyla, `private: true` kanalla sağlanır ([Resmi kaynak](https://supabase.com/docs/guides/realtime/authorization)). Öneri: veritabanı tetikli **private Broadcast** ile yalnız "yeni komut var" sinyali; içerik REST ile çekilir.
- **Idempotency:** `command_id` istemcide üretilir (UUID); sunucuda benzersiz kısıt; aynı id ile tekrar istek yeni komut üretmez; cihaz aynı id'yi bir kez uygular.
- **Replay önleme:** Komut yalnız DB'den RLS'li okumayla alınır (taşıma üzerinde ham komut yok); `expires_at`; cihazda işlenmiş id listesi; receipt'te `command_id` + cihaz kimliği. Ek olarak (C2 seçilirse) cihaz receipt'leri imzalayabilir.
- **TTL:** Komut türüne göre (ör. `APPLY_KINDORA_RESTRICTION` 10 dk, `GRANT_EXTRA_TIME` talebin geçerlilik süresi). Süresi geçen komut cihazda uygulanmaz.
- **Offline cihaz:** Komut `pending` kalır, UI "Sırada"; TTL içinde bağlanırsa uygulanır; değilse `expired`.
- **Sonuç doğrulama:** Receipt yalnız ilgili cihaz kimliğiyle kabul edilir; cihaz saati kayıt amaçlıdır, sunucu zamanı esastır.
- **Durum sorgulama (Doğrulanamadı):** Ebeveyn "Durumu kontrol et" → aynı `command_id` için sunucu durumu okunur; yeni komut üretilmez.

## 5. Bağlantı durumu

- `devices.last_seen_at` her senkronda (ör. 60 sn heartbeat) güncellenir; `online` = son görülme < 2× heartbeat. Realtime **Presence** isteğe bağlı ek sinyal; tek kaynak değil.
- UI'da "10:42 itibarıyla" etiketi (tasarım kuralı) `last_seen_at`'ten türetilir.

## 6. Maliyet ve kotalar ([Resmi kaynak](https://supabase.com/pricing), 2026-10-28)

| Kaynak | Free | Tek aile tahmini [Çıkarım] |
|---|---|---|
| Veritabanı | 500 MB | Çok altında |
| Egress | 5 GB | 60 sn heartbeat + komutlar: düşük |
| MAU | 50.000 | Cihaz başına Auth kullanıcısı seçilirse cihazlar da sayılır |
| Realtime eşzamanlı bağlantı | 200 | Cihaz + ebeveyn istemcileri; tek aile için yeterli |
| Realtime mesaj | 2 milyon / ay | Yalnız uyandırma sinyali → düşük |
| Edge Function çağrısı | 500.000 / ay | Heartbeat'i Edge Function yaparsa: 1 cihaz × 1/dk ≈ 43.200/ay; çok cihaz/ailede hızla büyür → heartbeat RLS'li doğrudan güncelleme ile yapılabilir |
| Aktif proje | 2 | Prod + test için yeterli |
| **Hareketsizlikte duraklatma** | **1 hafta** | **Risk:** kullanılmayan hafta sonu/tatil sonrası proje duraklarsa cihazlar senkron yapamaz; yerel kurallar sürer (T0-D) |
| Pro | 25 $/ay'dan | Ticari aşama veya duraklatma riski kabul edilemezse |

Fiyat/kota değişebilir; lansman öncesi yeniden doğrulanmalı.

## 7. Açık sorular

- Cihaz kimliği C1/C2 (ADR-0008).
- Postgres Changes vs DB-tetikli Broadcast (ADR-0009) — PoC-C'de ölçülecek.
- Push (FCM/APNs) Edge Function'dan mı tetiklenecek — T0-E.
- Supabase bölgesi (KVKK/gecikme) — karar bekliyor.
- Politika imzalama gerekli mi (cihazın sahte politika almasını RLS + TLS yeterince engelliyor mu) — risk R-08.

## 8. GATE T0-C kontrol listesi

- [x] Cihaz eşleştirme akışı (§3).
- [x] Yetkilendirme sınırları (§2).
- [x] RLS tasarım ilkeleri ve test kuralı (§1–2).
- [x] Komut güvenilirliği (§4).
- [ ] Kullanıcı kabulü. Gerçek Supabase testi: **NOT RUN**.

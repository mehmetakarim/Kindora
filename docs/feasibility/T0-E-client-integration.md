# T0-E — İstemci Entegrasyonu (Vue / Flutter)

> Durum: **Taslak / öneri** · Kapı: GATE T0-E · İlgili: [ADR-0003](../decisions/0003-flutter-ebeveyn-mobil-mvp.md), [ADR-0010](../decisions/0010-paylasilan-sozlesme-json-schema.md)
> Flutter/Tauri kurulumu, paketleme, mağaza ve push sağlayıcı kurulumu bu fazın kapsamı dışında.

## 1. İstemciler ve rolleri

| İstemci | Konuştuğu taraf | Yetki |
|---|---|---|
| Parent Desktop (Tauri/Vue) | Supabase (Auth, Edge Functions, RLS'li okuma, Realtime) | Ebeveyn JWT |
| Parent Mobile (Flutter) | Aynı | Aynı — **gevşek ayrı kural yok** |
| Child Desktop (Tauri/Vue) | **Yalnız yerel helper** (Supabase'e doğrudan değil) | Yok (sunum) |

Çocuk UI'nın Supabase'e doğrudan bağlanmaması, istemciye cihaz kimlik bilgisi verilmesini engeller; tüm bulut iletişimi servisten geçer.

## 2. Ortak sözleşme (öneri — ADR-0010)

- Tek kaynak: `packages/shared-contracts/` altında **JSON Schema** (komut türleri, payload'lar, receipt, talep, politika gövdesi, hata kodları).
- Üretim: TypeScript tipleri (Vue + Edge Functions/Deno), Dart modelleri (Flutter), Rust struct'ları (agent; `serde` + şema doğrulama).
- Sözleşme sürümü her mesajda (`schema_version`); kırıcı değişiklik = yeni sürüm.
- Alternatif: OpenAPI (Edge Function uçları için) — JSON Schema ile birlikte kullanılabilir. Kod üretici seçimi açık.

## 3. Komut durum makinesi (tüm ebeveyn istemcilerinde aynı)

```text
ready ──confirm──► sending ──HTTP 2xx──► pending ──receipt applied──► applied
                     │                     │  └─receipt failed/rejected─► failed
                     │                     └─timeout (30 sn öneri)────► unverified
                     └─cihaz çevrimdışı──► queued ──cihaz bağlandı──► pending
unverified ──"Durumu kontrol et" (aynı command_id)──► pending | applied | failed
failed ──"Yeniden gönder" (aynı command_id)──► sending
queued/pending ──expires_at geçti──► failed(expired)
```

UI etiketleri (tasarım): Hazır · Gönderiliyor · Sırada · Cihaz yanıtı bekleniyor · Uygulandı · Başarısız · Doğrulanamadı. **Zaman aşımı = Doğrulanamadı (amber), Başarısız değil.** Tüm komut türleri (kilit, mola, mod, ek süre, kural kaydı) aynı depoyu (`useCommandStore` / Flutter eşdeğeri) kullanır (OBS-03).

## 4. Token yönetimi

- Ebeveyn: Supabase Auth oturumu; masaüstünde refresh token işletim sistemi anahtar deposunda (Windows Credential Manager / macOS Keychain — Tauri eklentisi [PoC gerekli]); Flutter'da `flutter_secure_storage` (Keychain/Keystore) [öneri].
- Kısa JWT ömrü önerilir; Realtime politikaları JWT ile birleştiğinde önbelleğe alındığından kısa süre güvenlik açısından önemli ([Resmi kaynak](https://supabase.com/docs/guides/realtime/authorization)).
- Asimetrik JWT imzalama anahtarları (JWKS) tercih edilir ([Resmi kaynak](https://supabase.com/docs/guides/auth/jwts)).
- Mobil bildirimden onay: biyometrik doğrulama (tasarım kararı) → `local_auth` [öneri].

## 5. Push bildirim seçenekleri (karar değil)

| Seçenek | Not |
|---|---|
| FCM (Android + iOS APNs köprüsü) | Tek SDK; Google hesabı/proje gerekir |
| Doğrudan APNs + FCM | Daha fazla altyapı |
| Yalnız Realtime (uygulama açıkken) | Arka planda teslim garantisi yok — tek başına yetersiz |

Push yükü hassas veri içermez ("Yeni bir talep var"); bildirime dokununca sunucudan yenilenir. Gönderim Edge Function'dan. Seçim ve maliyet: açık karar K-06.

## 6. İzin talebi iş akışı

```text
Çocuk UI (neden ≥5 karakter, 10/15/30 dk) → helper → servis outbox → Edge Function → permission_requests(pending)
   → Realtime/push → ebeveyn (Desktop/Flutter) Onayla/Reddet + not
   → Edge Function: karar + GRANT_EXTRA_TIME komutu (aynı işlemde) → cihaz komutu uygular → receipt
   → çocuk UI: "Onaylandı (+15 dk)" / "Reddedildi: <not>" (danger rengi yok)
```

Onaysız talep limiti uzatmaz; talep süresi dolarsa `expired`.

## 7. Cihaz durumu gösterimi

`online` / `offline` / `unknown` + "Son bağlantı HH:MM"; çevrimdışıyken komut düğmeleri "Sırada" moduna geçer (komut gönderilebilir, TTL bilgisiyle) — tasarımdaki "Bağlantı gelince kullanılabilir" metni **ebeveynin kendi bağlantısı** yoksa kullanılır.

## 8. Hata mesajları (öneri sözlüğü)

Sözleşmede makine kodu + Türkçe UI metni eşlemesi: `DEVICE_REVOKED`, `COMMAND_EXPIRED`, `NOT_AUTHORIZED`, `RATE_LIMITED`, `DEVICE_OFFLINE`, `POLICY_VERSION_STALE`, `VALIDATION_FAILED`. UI metinleri tek yerde (i18n dosyası) tutulur; Vue ve Flutter aynı anahtarları kullanır.

## 9. Tasarım sistemi tutarlılığı

- Vue: `_kindora-tokens.scss` + Bootstrap 6 ([ADR-0005](../decisions/0005-bootstrap-6-hedef.md)). Bootstrap 6 durumu: **6.0.0-alpha.1, 8 Ekim 2026** ([Resmi kaynak](https://blog.getbootstrap.com/2026/10/08/bootstrap-6-alpha/)). Tarayıcı tabanı Chrome/Edge 130+, Firefox 132+, Safari 18+; Sass modülleri (`@use`/`@forward`, `@import` ile özelleştirme yok, `_variables.scss` kaldırıldı → token haritaları), responsive sınıf önekleri (`.md:d-none`), yalnız ESM JavaScript. Blog alfa öncesi beta'ya kadar "anlamlı değişiklik" olabileceğini belirtir. WebView2 kendini güncellediği için tarayıcı tabanı muhtemelen sorun değil [Çıkarım]. **Etkisi:** `_kindora-tokens.scss` Bootstrap'tan önce `@import` ile değişken override eder — v6 modül/token yapısına uyarlanması gerekir [PoC gerekli]. Alfa kararlılık riski: R-14.
- Flutter: `kindora_theme.dart` (Material 3) — aynı token değerleri.
- Mobil tasarım dosyalarındaki eski birleşik kilit metinleri güncellenmeli (OBS-07, ADR-0001).

## 10. GATE T0-E kontrol listesi

- [x] Flutter/Vue iletişim sözleşmesi yaklaşımı (§2).
- [x] Komut durumları (§3).
- [x] Hata ve bildirim akışları (§5, §8).
- [ ] Kod üretici ve push sağlayıcı seçimi (açık).
- [ ] Kullanıcı kabulü.

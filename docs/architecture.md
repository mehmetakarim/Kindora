# Kindora — Mimari

> Son güncelleme: 2026-10-27 (Europe/Istanbul) · TASK-002
> Ürün gereksinimlerinin ayrıntılı kaynağı: [KINDORA_PROJECT_CONTEXT.md](../design/KINDORA_PROJECT_CONTEXT.md) (§4–§12). Bu belge içeriği kopyalamaz; özetler ve gerçek durumu kaydeder.

Bu belge iki bölümdür: **A. Onaylanmış hedef mimari** (plan; uygulanmadı) ve **B. Mevcut uygulama durumu** (proje dosyalarından doğrulanan gerçek).

---

## A. Onaylanmış hedef mimari (planlandı — henüz uygulanmadı)

```text
Ebeveyn masaüstü (Tauri+Vue) ─┐
Ebeveyn mobil (Flutter)  ─────┴─► Supabase (Auth, Postgres+RLS, Edge Functions, Realtime)
                                        │ yetkilendirilmiş, kalıcı komut kaydı + receipt
                                        ▼
                  Çocuk Windows PC: Tauri UI + Filiz widget ◄─IPC─► Oturum yardımcısı ◄─IPC─► Rust Windows Service
                                                                                  └─ policy engine + SQLite + web filtre adaptörü
```

Temel ilke: ebeveyn arayüzleri yönetim istemcisidir; kısıtlamalar **çocuğun cihazında** uygulanır. Ayrıntı: [context §4](../design/KINDORA_PROJECT_CONTEXT.md#4-sistem-bütününde-mimari).

### A.1 Çocuk Windows uygulaması — Tauri 2 + Vue 3 + TypeScript + Bootstrap 6

- **Sorumluluk:** Sunum: kalan süre, mola akışı, ek süre/site talebi, rozetler, Filiz widget'ı (Compact / Expanded / Mascot Only), eşleştirme ekranı. Politika değerlerini değiştiremez.
- **İletişim:** Yerel IPC ile oturum yardımcısı/servis; talepler servis üzerinden Supabase'e.
- **Güvenlik sınırı:** Tauri komut yüzeyi en dar yetki setiyle; frontend komutu otomatik OS ayrıcalığına dönüşmez. Always-on-top güvenlik sınırı değildir. UI kapatılsa da denetim sürer.
- **Planlanan bileşenler:** Vue Router, Pinia, `Kd*` bileşen kütüphanesi ([components.md](../design/design_handoff_kindora/components.md)), WebView2 kabuğu.
- **Doğrulama gereken:** Bootstrap 6 kararlılığı ([ADR-0005](decisions/0005-bootstrap-6-hedef.md)); çerçevesiz/şeffaf/always-on-top widget; tam ekranda gizlenme; widget klavye odağı; %125/%150 DPI; çoklu monitör (GATE D).

### A.2 Ebeveyn masaüstü uygulaması — Tauri 2 + Vue 3 + TypeScript + Bootstrap 6

- **Sorumluluk:** Dashboard, cihazlar/eşleştirme, ekran süresi ve haftalık program, web kuralları, izin talepleri, raporlar, etkinlik geçmişi, ayarlar. Kanonik tasarım: Phase 5 + Dark varyant.
- **İletişim:** Yalnızca Supabase (Auth + Edge Functions + Realtime/okuma). Çocuk cihazıyla doğrudan konuşmaz.
- **Güvenlik sınırı:** İstemci rol göstergesi yetki değildir; yetki sunucu tarafında (RLS + Edge Functions). Gizli anahtar gömülmez.
- **Planlanan bileşenler:** `useDeviceStore`, `useCommandStore` (7 durumlu komut modeli), `useRequestStore`, `useScheduleStore`, `useWebRulesStore`, `useUiStore` (tasarım önerisi).
- **Doğrulama gereken:** 1280×800 @%150 desteği (açık karar), gerçek WebView2 ölçekleme testi, komut ACK/timeout/idempotency (GATE B, D).

### A.3 Ebeveyn mobil uygulaması — Flutter (Android/iOS) — MVP kapsamında

- **Sorumluluk:** Oturum açma, aile/çocuk seçimi, cihaz durumu, Kindora kilidi uygula/kaldır, mola, ek süre, talepleri onay/ret, bildirimler, işlem geçmişi.
- **İletişim:** Masaüstü ebeveynle **aynı** Supabase yetki modeli, komut şeması ve durum makinesi. Push (FCM/APNs) gerekebilir; Realtime arka planda push garantisi değildir.
- **Güvenlik sınırı:** Push içeriğinde hassas çocuk verisi yok; bildirime tıklayınca sunucudan yenilenir.
- **Planlanan bileşenler:** Tema referansı [kindora_theme.dart](../design/design_handoff_kindora/flutter/kindora_theme.dart); durum yönetimi seçilmedi (Riverpod/Bloc).
- **Doğrulama gereken:** Push sağlayıcısı, mağaza/imzalama gereklilikleri, gerçek cihazda Auth + komut akışı (GATE E). Ayrıca bkz. [ADR-0003](decisions/0003-flutter-ebeveyn-mobil-mvp.md).

### A.4 Windows arka plan servisi — Rust (servis + oturum yardımcısı)

- **Sorumluluk:** Açılışta çalışma, yerel policy engine, SQLite, komut/durum senkronizasyonu, web filtre adaptörü yönetimi, denetim kayıtları. Oturum yardımcısı: kullanıcı oturumuna bağlı işlemler (ör. `LockWorkStation()`), UI ile IPC.
- **İletişim:** Supabase ile kimliği doğrulanmış cihaz oturumu; UI ile yalnızca izinli IPC mesajları.
- **Güvenlik sınırı:** Beyaz listedeki komut tipleri; keyfi komut yürütme yok; aynı `command_id` bir kez uygulanır. Kurtarılamaz kilit yok.
- **Planlanan bileşenler (öneri):** `windows-agent`, `windows-session`, `policy-engine`, `network-filter` crate'leri ([context §15](../design/KINDORA_PROJECT_CONTEXT.md#15-önerilen-monorepo-organizasyonu-henüz-oluşturulmuş-değil)).
- **Doğrulama gereken:** GATE A — standart hesap yetkileri, servis/oturum başlatma, Alt+Tab/Görev Yöneticisi karşısında kısıtlama, uyku/saat değişimi, güvenli kaldırma/kurtarma. Bkz. [ADR-0001](decisions/0001-kindora-kullanim-kilidi-ana-kontrol.md).

### A.5 Merkezi backend — Supabase

- **Sorumluluk:** Auth, aile/tenant modeli, politika sürümleri, komutlar + receipt'ler, izin talepleri, kullanım özetleri, audit.
- **İletişim:** Ebeveyn istemcileri → Edge Functions (kritik komutlar); Realtime cihazı uyandırır, kesin durum kalıcı kayıt + ACK ile.
- **Güvenlik sınırı:** Her tabloda RLS; aile izolasyonu testleri zorunlu; `service_role` istemcide yok; hız sınırlaması.
- **Planlanan bileşenler:** Taslak varlıklar [context §10.2](../design/KINDORA_PROJECT_CONTEXT.md#102-kavramsal-veri-modeli--taslak-nihai-şema-değil) (nihai şema değil).
- **Doğrulama gereken:** GATE B; bölge ve kota politikası belirlenmedi. Bkz. [ADR-0002](decisions/0002-supabase-sqlite-gorev-ayrimi.md).

### A.6 Yerel veri ve çevrimdışı kurallar — SQLite

- **Sorumluluk:** Son geçerli politika kopyası, süre sayaçları, işlenmiş komut kimlikleri, bekleyen senkron olayları, cihaz konfigürasyonu.
- **İletişim:** Yalnızca Rust servis erişir; UI veriyi IPC ile okur.
- **Güvenlik sınırı:** Dosya ACL'i; sırlar düz metin değil; eski politika yeni sürümü ezmez.
- **Doğrulama gereken:** Monoton zaman + duvar saati, uyku/yeniden başlatma, bozuk DB kurtarma (GATE A).

### A.7 Web filtreleme

Hedef: tarayıcıdan bağımsız **alan adı** düzeyi. Yöntem seçilmedi (hosts / DNS / WFP / eklenti). DNS temelli yaklaşım context'te "aday" olarak geçer. GATE C ile doğrulanacak. Ayrıntı: [context §12](../design/KINDORA_PROJECT_CONTEXT.md#12-tarayıcıdan-bağımsız-web-sitesi-kontrolü).

---

## B. Mevcut uygulama durumu (2026-10-27 itibarıyla doğrulandı)

| Unsur | Durum |
|---|---|
| Git deposu | **Mevcut** (2026-10-27, TASK-002) — yerel `git init`, branch `main`, henüz commit yok |
| Uzak depo | `origin` → `https://github.com/mehmetakarim/Kindora.git` (GitHub'da boş, **public**) — henüz push yapılmadı |
| `.gitignore` | Mevcut — Node/Vue, Rust/Tauri, Flutter, Supabase sırları, SQLite, IDE/OS, log/cache kuralları |
| `.claude/settings.json` | Mevcut — yalnızca AI attribution kapatma ayarları (uygulama yapılandırması değil) |
| Kod dizinleri (`apps/`, `crates/`, `packages/`, `supabase/`) | Henüz uygulanmadı |
| Paket/bağımlılık dosyaları (`package.json`, `Cargo.toml`, `pubspec.yaml`, lock dosyaları) | Henüz uygulanmadı |
| Uygulama giriş noktaları | Henüz uygulanmadı |
| Veritabanı şemaları / migration'lar | Henüz uygulanmadı |
| Test altyapısı / CI | Henüz uygulanmadı |
| Build / geliştirme / test komutları | Yok — doğrulanabilir komut bulunmuyor |
| README.md (kök) | Yok |

**Gerçekte mevcut olanlar:**

- [design/KINDORA_PROJECT_CONTEXT.md](../design/KINDORA_PROJECT_CONTEXT.md) — ürün bağlamı v1.0.
- [design/design_handoff_kindora/](../design/design_handoff_kindora/README.md) — Claude Design teslimatı: README, components.md, `tokens/_kindora-tokens.scss`, `flutter/kindora_theme.dart` (referans tema kodu, derlenmiş bir uygulamanın parçası değil), `design_files/` altında 13 `.dc.html` prototip + destek JS dosyaları, `mascot_brief/` altında 2 `.dc.html`.
- Hafıza belgeleri: `AGENTS.md`, `CLAUDE.md`, `brain.md`, `docs/`.
- Depo altyapısı: `.gitignore`, `.claude/settings.json`.

Proje şu anda **yalnızca tasarım ve dokümantasyondan** oluşur. Kod oluşturulduğunda bu bölüm gerçek dizin, komut ve test bilgileriyle güncellenmelidir.

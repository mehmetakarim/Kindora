# Kindora — Mimari

> Son güncelleme: 2026-10-28 (Europe/Istanbul) · TASK-004 (P1.A)
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
- **P1.C (kodlandı):** kullanım süresi, günlük özet, son geçerli politika, mola durumu ve yaşam döngüsü olayları ([ADR-0014](decisions/0014-yerel-kullanim-suresi-motoru.md), [p1c-screen-time.md](screen-time/p1c-screen-time.md)). Komut kimlikleri ve senkron olayları henüz yok.

### A.7 Web filtreleme

Hedef: tarayıcıdan bağımsız **alan adı** düzeyi. Yöntem seçilmedi (hosts / DNS / WFP / eklenti). DNS temelli yaklaşım context'te "aday" olarak geçer. GATE C ile doğrulanacak. Ayrıntı: [context §12](../design/KINDORA_PROJECT_CONTEXT.md#12-tarayıcıdan-bağımsız-web-sitesi-kontrolü).

### A.8 Phase T0 bulguları (T0 dokümantasyonu kabul edildi; ADR-0006…0010 önerileri **kabul edilmedi**, hiçbiri uygulanmadı)

Ayrıntı ve kaynaklar: [docs/feasibility/](feasibility/README.md). Hiçbiri Windows/Supabase üzerinde test edilmedi.

- **Kilit güvenlik sınırı:** Kindora kilidi "caydırıcı kısıtlama + kurcalama tespiti ve tepkisi" olarak konumlanır; Ctrl+Alt+Del, UAC secure desktop, yönetici erişimi kapsam dışıdır ([T0-A](feasibility/T0-A-kindora-lock.md)).
- **Helper sonlandırılabilir:** Çocuk kimliğiyle çalışan helper aynı kullanıcı tarafından kapatılabilir [Çıkarım]; servisin helper'ı kendisinin başlatması `WTSQueryUserToken` → LocalSystem gerektirir, ancak Run anahtarı / logon görevi gibi yöntemler gerektirmez. Model seçimi PoC sonrası ([ADR-0006](decisions/0006-agent-surec-ve-hesap-modeli.md), Önerildi).
- **Assigned Access** Windows Home'da yok → ana yöntem olamaz.
- **IPC:** oturum başına, açık DACL'li named pipe ([ADR-0007](decisions/0007-servis-helper-ipc-named-pipe.md), Önerildi).
- **Bulut:** cihaz başına kimlik + tek kullanımlık eşleştirme kodu ([ADR-0008](decisions/0008-cihaz-kimligi-ve-eslestirme.md)); kalıcı komut tablosu, Realtime yalnız uyandırma ([ADR-0009](decisions/0009-kalici-komut-kuyrugu-realtime-uyandirma.md)); Free plan 1 hafta hareketsizlikte duraklar ([T0-C](feasibility/T0-C-supabase-architecture.md)).
- **Çevrimdışı:** monoton saatle kullanım ölçümü, "son ebeveyn kuralı" politikası, kurtarılamaz kilit yok ([T0-D](feasibility/T0-D-offline-policy.md)).
- **İstemciler:** JSON Schema tek kaynak sözleşme ([ADR-0010](decisions/0010-paylasilan-sozlesme-json-schema.md)); çocuk UI Supabase'e doğrudan bağlanmaz ([T0-E](feasibility/T0-E-client-integration.md)).
- **Bootstrap 6:** `6.0.0-alpha.1` (8 Eki 2026, resmi); token dosyası v6 Sass modüllerine uyarlanmalı ([ADR-0005](decisions/0005-bootstrap-6-hedef.md)).
- **Windows destek önerisi:** Windows 11 birincil, Windows 10 22H2 ESU süresince ikincil ([T0-B §6](feasibility/T0-B-windows-agent.md)).

---

## B. Mevcut uygulama durumu (2026-10-27 itibarıyla doğrulandı)

| Unsur | Durum |
|---|---|
| Git deposu | **Mevcut** — branch `main`, ilk commit `64a05ac` (2026-10-28) |
| Uzak depo | `origin` → `https://github.com/mehmetakarim/Kindora.git` (**public**); `main` = `64a05ac` push edildi |
| `.gitignore` | Mevcut — Node/Vue, Rust/Tauri, Flutter, Supabase sırları, SQLite, IDE/OS, log/cache kuralları |
| `.claude/settings.json` | Mevcut — yalnızca AI attribution kapatma ayarları (uygulama yapılandırması değil) |
| `crates/` (Rust workspace) | **Kodlandı — P1.A prototipi** (2026-10-28): `ipc-protocol`, `agent-core`, `windows-agent` (`kindora-agent`), `windows-session` (`kindora-helper`). Yalnız `PING`/`GET_STATUS`/`HEALTH_CHECK`; kilit/politika/bulut yok. macOS'ta ve GitHub-hosted Windows Server 2022'de (konsol + gerçek LocalService servisi) test edildi; **gerçek Windows 11 cihaz kabulü bekliyor**. Agent servis hesabı `NT AUTHORITY\LocalService`; helper kendi süreç DACL'ine LocalService için yalnız `PROCESS_QUERY_LIMITED_INFORMATION` ekler, böylece agent exe yolunu işletim sisteminden doğrulayabilir ([BUG-003](solutions.md)). Kindora mevcut Windows hesabında çalışır, ayrı çocuk hesabı zorunlu değildir ([ADR-0012](decisions/0012-mevcut-windows-hesabinda-calisma.md)). Ayrıntı: [p1a-windows-agent.md](agent/p1a-windows-agent.md) |
| `crates/screen-time` | **Kodlandı — P1.C** (2026-10-29): kullanım süresi motoru (aktif/boşta/kilitli/uyku/bilinmeyen, monoton saat), gün/saat değişikliği, mola durumu, DEV politika doğrulaması, SQLite (`rusqlite` 0.41.1 gömülü) migration ve kurtarma. Agent'ta 1 sn döngüyle çalışır; helper `GetLastInputInfo` ile `REPORT_ACTIVITY` gönderir; masaüstü `GET_SCREEN_TIME` verisini gösterir. **Kilit/engelleme yok.** macOS ve Windows CI'da test edildi; **Lenovo kullanım süresi kabulü bekliyor**. Ayrıntı: [p1c-screen-time.md](screen-time/p1c-screen-time.md), [ADR-0014](decisions/0014-yerel-kullanim-suresi-motoru.md) |
| `apps/desktop/` | **Kodlandı — P1.B, P1.C ile genişletildi**: Tauri 2 + Vue 3 + TS strict + Bootstrap 5.3.8 (tasarım token'larıyla). Agent'a doğrudan bağlanmaz; yanındaki `kindora-helper.exe`'yi `bridge` modunda stdio ile kullanır ([ADR-0013](decisions/0013-masaustu-kabugu-helper-koprusu.md)). P1.B **Lenovo GUI kabulü PASS** (2026-10-29). P1.C: Ana görünüm (Filiz, bugünkü kullanım, kalan süre, sıradaki mola, bildirimler) ve Teknik tanılama sekmesi. Ayrıntı: [p1b-desktop-shell.md](desktop/p1b-desktop-shell.md), [p1c-screen-time.md](screen-time/p1c-screen-time.md) |
| `packages/`, `supabase/` | Henüz uygulanmadı |
| Paket/bağımlılık dosyaları | Kök `Cargo.toml` + `Cargo.lock` (serde, serde_json, ctrlc, libc[unix], windows-sys/windows-service[windows]). Masaüstü: `apps/desktop/package.json` + `package-lock.json` (kesin sürümler; Vue 3.5, Vite 8, TS 6, Vitest 5, Bootstrap 5.3.8, @tauri-apps 2.12), `apps/desktop/src-tauri/Cargo.toml` + `Cargo.lock` (tauri 2.12.5). `pubspec.yaml`: yok |
| Uygulama giriş noktaları | `crates/windows-agent/src/main.rs` (`console`/`service`), `crates/windows-session/src/main.rs` (`run`/`bridge`/`ping`/`status`/`health`), `apps/desktop/src-tauri/src/main.rs` (Tauri). Flutter: yok |
| Veritabanı şemaları / migration'lar | Yerel SQLite şema v1 (`crates/screen-time/src/store.rs`, `PRAGMA user_version`). Supabase şeması henüz yok |
| Test altyapısı / CI | `cargo test --workspace` (32 birim + 6 Unix-dev e2e + Windows'a özgü 9 test yalnız Windows'ta). CI: [windows-p1a.yml](../.github/workflows/windows-p1a.yml), [windows-p1b.yml](../.github/workflows/windows-p1b.yml) ve [windows-p1c.yml](../.github/workflows/windows-p1c.yml) (kullanım süresi bölümü `-ScreenTime`) (GitHub-hosted `windows-2022`) + LocalService servis/köprü/masaüstü GUI duman betiği [ci/p1a-localservice-e2e.ps1](../ci/p1a-localservice-e2e.ps1) (Windows PowerShell 5.1). Ön yüz: Prettier, ESLint, vue-tsc, Vitest. Gerçek Windows 11 kabul testi değildir |
| Build / geliştirme / test komutları | Rust: [p1a-windows-agent.md §4](agent/p1a-windows-agent.md) (macOS'ta doğrulandı; servis kurulum/kaldırma komutları CI'da PowerShell 5.1 ile doğrulandı) |
| README.md (kök) | Yok |

**Gerçekte mevcut olanlar:**

- [design/KINDORA_PROJECT_CONTEXT.md](../design/KINDORA_PROJECT_CONTEXT.md) — ürün bağlamı v1.0.
- [design/design_handoff_kindora/](../design/design_handoff_kindora/README.md) — Claude Design teslimatı: README, components.md, `tokens/_kindora-tokens.scss`, `flutter/kindora_theme.dart` (referans tema kodu, derlenmiş bir uygulamanın parçası değil), `design_files/` altında 13 `.dc.html` prototip + destek JS dosyaları, `mascot_brief/` altında 2 `.dc.html`.
- Hafıza belgeleri: `AGENTS.md`, `CLAUDE.md`, `brain.md`, `docs/`.
- Depo altyapısı: `.gitignore`, `.claude/settings.json`.
- Phase T0 planlama belgeleri: [docs/feasibility/](feasibility/README.md) (yalnız doküman; uygulama kodu değil).

Proje 2026-10-28 itibarıyla tasarım, dokümantasyon ve **P1.A Rust prototipinden** oluşur. Prototipin gerçek yapısı:

```text
kindora-helper (kullanıcı oturumu) ──IPC v1──► kindora-agent (konsol / Windows servisi)
   windows-session crate                         windows-agent crate
          │                                          │
          └──────── ipc-protocol (mesaj, çerçeve, taşıma) ───┘
                                                     agent-core (yetkilendirme, işleyici, bağlantı döngüsü)
Taşıma: Windows → named pipe (kodlandı, çalıştırılmadı) · Unix → yalnız geliştirme socket'i (test edildi)
```

P1.C ile Usage Monitor, yerel politika doğrulaması ve Persistence (SQLite) **kodlandı** (kural kaynağı yalnız DEV test dosyası). Command Processor, Sync Client, ebeveyn yetkilendirmesi ve kısıtlama yüzeyi **henüz uygulanmadı**.

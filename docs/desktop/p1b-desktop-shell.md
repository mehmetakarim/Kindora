# P1.B — Kindora Desktop kabuğu (Tauri 2 + Vue 3)

> **Durum:** Kodlandı, macOS'ta ve GitHub Actions `windows-2022`'de test edildi. **Lenovo GUI Acceptance PASS** (kullanıcı raporu, 2026-10-29): GUI açıldı, LocalService bağlantısı ve gerçek agent bilgileri, servis durunca "Bağlantı yok", yeniden başlayınca otomatik bağlantı, Filiz durum değişimi, X ile kapanışta desktop + helper sonlandı. Son `sc.exe delete` kabul edildi; `1060` doğrulaması yapılmadı. P1.C ile genişletildi: [p1c-screen-time.md](../screen-time/p1c-screen-time.md).
> Kısıtlama yoktur: kilit, tam ekran/overlay, giriş engelleme, otomatik başlatma, yetki yükseltme, servis kurulumu yok.

## 1. Mimari

```text
┌──────────── kullanıcının Windows oturumu (mevcut hesap, ADR-0012) ────────────┐
│ kindora-desktop.exe (Tauri 2)                                                 │
│   WebView (Vue 3, TS strict, Bootstrap 5.3.8 + Kindora token'ları)            │
│     │  yalnız: invoke(get_agent_state | refresh_agent), listen(agent-state)   │
│   Rust: Supervisor ── stdio (JSON satırları, yalnız {"op":"refresh"}) ──┐     │
│                                                                         ▼     │
│   kindora-helper.exe bridge  (aynı klasör; alt süreç; pencere yok)            │
│     │  named pipe \\.\pipe\kindora-agent-v1 — yalnız GET_STATUS/HEALTH_CHECK  │
└─────┼─────────────────────────────────────────────────────────────────────────┘
      ▼  agent politikası DEĞİŞMEDİ: oturum ≠ 0, kindora-helper.exe, aynı klasör
kindora-agent.exe service  (Session 0, NT AUTHORITY\LocalService, start= demand)
```

Karar ve tehdit modeli: [ADR-0013](../decisions/0013-masaustu-kabugu-helper-koprusu.md).

| Katman | Konum | Sorumluluk |
|---|---|---|
| Köprü sözleşmesi | [`crates/ipc-protocol/src/bridge.rs`](../../crates/ipc-protocol/src/bridge.rs) | `BridgeRequest` (yalnız `refresh`, katı ayrıştırma), `BridgeEvent`, satır sınırı 4096 B |
| Helper köprü modu | [`crates/windows-session`](../../crates/windows-session/src/lib.rs) `run_bridge` | Bağlan → GET_STATUS + HEALTH_CHECK → 3 sn'de bir; geri çekilme 0,25→5 sn; yetkisizde 30 sn; stdin EOF → çıkış |
| Gözetim | [`apps/desktop/src-tauri/src/supervisor.rs`](../../apps/desktop/src-tauri/src/supervisor.rs) | Helper'ı exe yanından başlat (`CREATE_NO_WINDOW`), olayları oku, çökünce 1→30 sn ile yeniden başlat, kapanışta stdin kapat → 2 sn → sonlandır |
| Görünüm durumu | [`view.rs`](../../apps/desktop/src-tauri/src/view.rs) | Saf indirgeyici; bağlantı yokken agent verisi gösterilmez |
| Kayıt | [`logging.rs`](../../apps/desktop/src-tauri/src/logging.rs) | `%LOCALAPPDATA%\app.kindora.desktop.preview\logs\desktop.log`, yalnız geçişler, 1 MB'ta döner (en fazla 2 dosya) |
| Uygulama | [`lib.rs`](../../apps/desktop/src-tauri/src/lib.rs) | 2 komut, `agent-state` olayı, kapatma: mesaj → helper kapanışı → çıkış |
| İzinler | [`capabilities/main.json`](../../apps/desktop/src-tauri/capabilities/main.json), [`build.rs`](../../apps/desktop/src-tauri/build.rs) | `core:event:allow-listen/unlisten`, `allow-get-agent-state`, `allow-refresh-agent` — başka izin yok; eklenti yok |
| Ön yüz | [`apps/desktop/src`](../../apps/desktop/src) | `agent/` (tipler, saf sunum, Tauri/demo kaynakları), `components/`, `composables/useAgent.ts` |

**Ayrı Cargo workspace:** `apps/desktop/src-tauri` kendi `[workspace]` ve `Cargo.lock` dosyasına sahiptir. Böylece Tauri bağımlılıkları ve ön yüz derlemesi kök `cargo test --workspace` komutunu etkilemez. `ipc-protocol` yol bağımlılığı olarak paylaşılır.

## 2. Ekranlar

Tek ana pencere (1120×760, en küçük 720×560, normal pencere, görev çubuğunda görünür):

- **Üst çubuk:** tasarımdaki logo + "Kindora · Masaüstü · P1.B önizleme".
- **Bağlantı yok bandı** (amber, "Yeniden dene"): agent'a ulaşılamadığında, yetkilendirilmediğinde veya helper bulunamadığında görünür.
- **Filiz paneli:** tasarımdaki konsept `mascot()` SVG'si birebir aktarıldı ve "konsept çizim (nihai değil)" diye etiketlendi. Mesaj anları: hoş geldin, bağlandı, bağlantı kesildi, yeniden bağlanılıyor, yetkilendirilmedi, güvenli kapanış.
- **Kindora Agent kartı:**
  - durum çipi (ikon + kelime + renk);
  - çalışma modu, sürüm, sağlık ve denetimler, çalışma süresi;
  - son başarılı iletişim, yeniden bağlantı durumu, oturum yardımcısı;
  - yanıt süresi ve "Şimdi yenile" düğmesi.
- **Bu sürüm hakkında:** kısıtlama olmadığı, kapatmanın güvenli olduğu, yönetici hesap sınırı.
- **Tema:** açık varsayılan (tasarım); koyu token'lar `data-bs-theme="dark"` ile hazır. `prefers-reduced-motion` maskot hareketini durdurur. 900 px altında tek sütun.
- **Demo kaynağı:** yalnız Tauri dışında (tarayıcıda `npm run dev`) kullanılır ve "Demo — gerçek Agent verisi değildir" bandıyla gösterilir. Tauri içinde hiçbir zaman seçilmez (test edildi).

## 3. Komutlar

```bash
# kök: agent + helper
cargo build --workspace && cargo test --workspace
# ön yüz (Node ≥ 22.12)
cd apps/desktop && npm ci && npm run check
# masaüstü Rust (önce npm run build: frontendDist gerekir)
cd apps/desktop/src-tauri && cargo test && cargo clippy --all-targets -- -D warnings
# macOS geliştirme: agent konsol + desktop (helper yolu yalnız debug'da ortamdan)
target/debug/kindora-agent console &
(cd apps/desktop && npx vite) &
KINDORA_HELPER_PATH=$PWD/target/debug/kindora-helper apps/desktop/src-tauri/target/debug/kindora-desktop
```

## 4. Test sonuçları

### macOS (2026-10-28, arm64, Unix geliştirme taşıması — Windows davranışının kanıtı değildir)

| Kontrol | Sonuç |
|---|---|
| Kök workspace `cargo test` | PASS — 42 test (bridge sözleşmesi 3 yeni, köprü yaşam döngüsü 1 yeni) |
| `clippy -D warnings` (macOS + `x86_64-pc-windows-msvc`) | PASS |
| `bridge_lifecycle` ölçümleri | agent başladıktan sonra bağlantı 547 ms; refresh 537 ms; agent durunca tespit 2,99 sn (yoklama 3 sn); yeniden başlayınca kurtarma 248 ms; stdin kapanınca çıkış 10 ms; ilk 1,5 sn'de ≤ 4 deneme |
| Masaüstü Rust `cargo test` | PASS — 13 test (görünüm 6, kayıt 2, gözetim 5: eksik helper, çökme→yeniden başlatma, bozuk çıktı, inatçı helper sonlandırma, refresh) |
| Masaüstü `clippy -D warnings`, `fmt --check` | PASS |
| Ön yüz: Prettier, ESLint (`strictTypeChecked`), `vue-tsc` strict, Vitest | PASS — 22 test |
| Ön yüz üretim derlemesi (Vite 8) | PASS |
| Gerçek Tauri penceresi + konsol agent (elle) | PASS: Bağlı → agent durdurulunca "Bağlantı yok" + yeniden deneme → agent başlayınca kendiliğinden Bağlı; helper `SIGKILL` → 1 sn sonra yeniden başlatıldı; desktop sonlandırılınca helper EOF ile çıktı (sahipsiz süreç yok) |

### Windows CI — [windows-p1b 40754107950](https://github.com/mehmetakarim/Kindora/actions/runs/40754107950) ve [windows-p1a 40754107939](https://github.com/mehmetakarim/Kindora/actions/runs/40754107939) (commit `2fb82cf`)

Runner: Windows Server 2022, `runneradmin` (yönetici), oturum 2, Windows PowerShell 5.1.20348, WebView2 Runtime 131.0.2903.86.

| Kontrol | Sonuç |
|---|---|
| Kök workspace: fmt, build, **37 test** (agent-core 11, ipc-protocol 16, köprü yaşam döngüsü 1, pipe smoke 8, süreç e2e 1), clippy `-D warnings` | PASS |
| Köprü yaşam döngüsü (Windows, üretim pipe'ı) | PASS — bağlantı 552 ms, refresh 530 ms, agent durunca tespit 2976 ms, kurtarma 242 ms, stdin kapanınca çıkış 10 ms; farklı klasördeki helper kopyası `Unauthorized` |
| Ön yüz: `npm ci`, Prettier, ESLint, `vue-tsc`, Vitest 22 test, Vite derlemesi | PASS |
| Masaüstü Rust: fmt, clippy `-D warnings`, 8 test (görünüm, kayıt; gözetim testleri Unix'e özgü) | PASS |
| Tauri Windows paketi | PASS — `kindora-desktop.exe` (8,7 MB) + `Kindora_0.1.0_x64-setup.exe` (NSIS, 2,5 MB; yalnız CI doğrulaması) |
| P1A regresyonu + LocalService servisi (README komutları, PowerShell 5.1) | PASS — ACE'siz istemci `OpenProcess=5`, helper PING/STATUS/HEALTH, farklı klasör `ExeDirMismatch` |
| Köprü + gerçek servis | PASS — SERVICE anlık görüntüsü, refresh 555 ms, `LOCK` reddi, stdin kapanınca temiz çıkış |
| **Gerçek Tauri penceresi + gerçek servis (GUI duman testi)** | PASS — helper üzerinden bağlandı (tam 1 helper alt süreci); `sc stop` 1880 ms'de algılandı; `sc start` sonrası 438 ms'de kurtarıldı; normal üst düzey pencere; `WM_CLOSE` → 880 ms'de çıkış; helper sızıntısı yok; servis çalışmaya devam etti |
| Paket içeriği ve SHA-256 (LF) | PASS |

## 5. Windows CI kapsamı ve sınırları

Kapsam: [`windows-p1b.yml`](../../.github/workflows/windows-p1b.yml), [`ci/p1a-localservice-e2e.ps1`](../../ci/p1a-localservice-e2e.ps1).

CI'ın **doğrulayamadıkları**:
- Runner yükseltilmiş yönetici hesabıdır; Lenovo'daki yönetici olmayan pencere yolu sınanmaz.
- Windows Server 2022, Windows 11 Home/Pro etkileşimli oturumu değildir.
- Pencerenin **görsel** doğruluğu (tipografi, ölçek %125/%150, taşma) CI'da görülmez; yalnız açıldığı, bağlandığı ve kapandığı ölçülür.
- WebView2 sürümü cihazda farklı olabilir.
- Görev Yöneticisi ile helper sonlandırma (B-06) ve klavye/fare serbestliği (B-10) elle denenir.

**CI PASS, kullanıcı GUI kabulü yerine geçmez.**

## 6. Test paketi

- **Prerelease:** [`p1b-windows-test.1`](https://github.com/mehmetakarim/Kindora/releases/tag/p1b-windows-test.1) — [çalıştırma 40755951911](https://github.com/mehmetakarim/Kindora/actions/runs/40755951911), hedef `2fb82cf`, `latest` değil.
- **İçerik:** `Kindora-P1B-Windows-x64.zip` (taşınabilir) = `kindora-desktop.exe` (GUI), `kindora-agent.exe`, `kindora-helper.exe`, `README-TEST-TR.md`, `BUILD-INFO.txt`, `SHA256SUMS.txt`.
- **ZIP SHA-256:** `ab55cb4afcb5132ddc921650932263ea6727a184ba70b401d4b9d86eb460beae` (macOS'ta indirilip doğrulandı).
- **Rehber:** [packaging/p1b/README-TEST-TR.md](../../packaging/p1b/README-TEST-TR.md).
  - Kurulum: tek yönetici PowerShell bloğu (klasör + `start= demand` LocalService servisi).
  - Kabul adımları: B-01…B-11.
  - Kaldırma adımları dahil.

## 7. Açık riskler

- Lenovo GUI kabulü (WebView2, %125/%150 ölçek, gerçek oturum) yapılmadı.
- EXE'ler imzasız; SmartScreen uyarısı beklenir.
- Kurucu dağıtılmıyor (taşınabilir ZIP); NSIS kurucusu yalnız CI doğrulaması.
- Yönetici hesapta kurcalama engellenemez (ADR-0012).
- Helper ↔ desktop sözleşmesi ile Rust↔TS tipleri elle eşlenir (`types.ts` ↔ `view.rs`); ADR-0010 kod üretimi açık.

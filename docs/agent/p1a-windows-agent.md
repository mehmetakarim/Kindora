# P1.A — Windows Agent & Session Helper (ilk prototip)

> Tarih: 2026-10-28 · Görev: [TASK-004](../tasks.md) · Karar önerileri: [ADR-0006](../decisions/0006-agent-surec-ve-hesap-modeli.md), [ADR-0007](../decisions/0007-servis-helper-ipc-named-pipe.md), [ADR-0011](../decisions/0011-p1a-crate-yapisi-ve-gelistirme-tasimasi.md)
> **Kullanım kilidi yok.** Yalnız `PING`, `GET_STATUS`, `HEALTH_CHECK`. Windows'ta hiçbir şey çalıştırılmadı.

## 1. Geliştirme ortamı (2026-10-28 tespiti)

| Öğe | Durum |
|---|---|
| Geliştirme makinesi | macOS 27.0, arm64 (Apple Silicon) |
| Rust / Cargo | 1.94.0 / 1.94.0 (stable) |
| Windows hedefi | `x86_64-pc-windows-msvc` std hedefi rustup'a eklendi (yalnız `cargo check`/`clippy`; MSVC linker yok → **bağlama/çalıştırma yok**). Geri almak: `rustup target remove x86_64-pc-windows-msvc` |
| Windows test cihazı / VM | **Yok** (Parallels/VMware/UTM/VirtualBox bulunamadı) |
| Windows 11 Home/Pro — kullanıcının mevcut hesabı (Lenovo; [ADR-0012](../decisions/0012-mevcut-windows-hesabinda-calisma.md)) | Kullanıcıda; `p1a-windows-test.1` ile W-10 FAIL (BUG-003), `test.2` bekleniyor |
| Geri dönüş yöntemi (snapshot) | **Yok** |

## 2. Workspace yapısı

```text
Cargo.toml                    # workspace (resolver 2, edition 2021, MSRV 1.80)
crates/
├── ipc-protocol/             # mesaj modelleri, çerçeveleme, yerel taşıma
│   └── src/{message.rs, frame.rs, transport/{mod.rs, windows_pipe.rs, unix_dev.rs}}
├── agent-core/               # platformdan bağımsız: yetkilendirme, istek işleyici, bağlantı döngüsü
├── windows-agent/            # bin: kindora-agent (console | service)
│   └── tests/dev_transport_e2e.rs
└── windows-session/          # bin: kindora-helper (run | ping | status | health)
```

Adlandırma: helper crate'i T0-B/context §15'teki kanonik `windows-session` adını kullanır; görev metnindeki `session-helper` ile aynı bileşendir.

**Bağımlılıklar (sürüm sabit):** `serde 1.0.229`, `serde_json 1.0.154` (mesajlar), `ctrlc 3.5.2` (konsolda güvenli Ctrl+C çıkışı), `libc 0.2.191` (yalnız Unix: eş kullanıcı kimliği, test sinyali), `windows-sys 0.61.2` (yalnız Windows: named pipe, güvenlik tanımlayıcısı, süreç sorgusu), `windows-service 0.8.1` (yalnız Windows: SCM; Mullvad bakımında). Hepsi MIT/Apache-2.0.

## 3. IPC sözleşmesi (protokol v1)

- **Çerçeve:** `u32` little-endian gövde uzunluğu + UTF-8 JSON. En büyük gövde **4096 bayt**; aşan başlık gövde okunmadan reddedilir.
- **İstek:** `protocol_version` (=1), `message_id` (1–64, `[A-Za-z0-9_-]`), `command` (`PING`/`GET_STATUS`/`HEALTH_CHECK`), `timestamp_ms` (bilgi amaçlı; yetki/tekrar için kullanılmaz). Bilinmeyen alan → reddedilir.
- **Yanıt:** `protocol_version`, `message_id` (yalnız geçerli biçimde ise yankılanır), `status` (`OK`/`ERROR`), `error_code`, `timestamp_ms`, `payload` (`PONG` / `STATUS` / `HEALTH`).
- **Hata kodları:** `UNSUPPORTED_VERSION`, `UNKNOWN_COMMAND`, `INVALID_MESSAGE`, `MESSAGE_TOO_LARGE`, `DUPLICATE_MESSAGE_ID`, `UNAUTHORIZED`, `INTERNAL`.
- **Bağlantı kuralları:** tekrar `message_id` (bağlantı başına son 256) → `DUPLICATE_MESSAGE_ID`; 5 hatalı istek → bağlantı kapanır; aşırı büyük çerçeve veya yarım çerçeve → kapanır; 30 sn sessizlik → kapanır; yetkisiz eş → genel `UNAUTHORIZED` + kapanır (gerekçe istemciye söylenmez, yalnız kayda yazılır).
- **Kimlik:** yalnız işletim sisteminden — Windows: `GetNamedPipeClientProcessId`, `GetNamedPipeClientSessionId`, `QueryFullProcessImageNameW`; Unix geliştirme: `getpeereid`. Mesaj alanları kimlik sayılmaz.

### Windows yetkilendirme politikası (kod: `helper_policy()`)

İstemci: Session 0 dışında (etkileşimli oturum) **ve** dosya adı `kindora-helper.exe` **ve** agent ile aynı dizinde. Alan alınamazsa → ret.

### Windows named pipe

`\\.\pipe\kindora-agent-v1`, SDDL `D:P(A;;GA;;;SY)(A;;GA;;;BA)(A;;GA;;;OW)(A;;0x0012008B;;;IU)`, `PIPE_REJECT_REMOTE_CLIENTS`, ilk örnekte `FILE_FLAG_FIRST_PIPE_INSTANCE`, overlapped G/Ç + zaman aşımı; istemci `SECURITY_SQOS_PRESENT | SECURITY_IDENTIFICATION`. ADR-0007'den sapma (oturum başına ad + logon SID yerine tek ad + Interactive Users): bkz. [ADR-0011](../decisions/0011-p1a-crate-yapisi-ve-gelistirme-tasimasi.md).

## 4. Komutlar

macOS/Linux (geliştirme taşıması):

```bash
cargo build --workspace          # önce: süreç testleri kindora-helper ikilisini kullanır (BUG-001)
cargo test --workspace
cargo clippy --workspace --all-targets
cargo check --workspace --all-targets --target x86_64-pc-windows-msvc
cargo clippy --workspace --all-targets --target x86_64-pc-windows-msvc
target/debug/kindora-agent console
target/debug/kindora-helper run
target/debug/kindora-helper ping
```

Windows servis (yönetici PowerShell; CI'da Windows PowerShell 5.1 ile [ci/p1a-localservice-e2e.ps1](../../ci/p1a-localservice-e2e.ps1) tarafından aynı komutlarla doğrulanır — `--%` iç tırnakların bozulmadan `sc.exe`'ye gitmesi içindir):

```powershell
cargo build --release --workspace
New-Item -ItemType Directory "C:\Program Files\KindoraDev" -Force | Out-Null
Copy-Item target\release\kindora-agent.exe, target\release\kindora-helper.exe "C:\Program Files\KindoraDev\"
sc.exe --% create KindoraAgentDev binPath= "\"C:\Program Files\KindoraDev\kindora-agent.exe\" service" start= demand obj= "NT AUTHORITY\LocalService"
sc.exe start KindoraAgentDev
sc.exe query KindoraAgentDev
# Aynı hesapta, yönetici OLMAYAN PowerShell:
& "C:\Program Files\KindoraDev\kindora-helper.exe" ping
# Güvenli çıkış (yönetici):
sc.exe stop KindoraAgentDev
sc.exe delete KindoraAgentDev
```

`start= demand`: açılışta başlamaz. Helper için Run anahtarı/zamanlanmış görev **oluşturulmaz**. Servis kaydı: `C:\Windows\ServiceProfiles\LocalService\AppData\Local\Temp\kindora-agent-dev.log` (CI'da doğrulandı).

## 5. Test sonuçları (2026-10-28, macOS arm64)

### Gerçekten çalıştırılanlar

| Kontrol | Ortam | Sonuç |
|---|---|---|
| `cargo build --workspace` | macOS | PASS |
| `cargo test --workspace` — 32 birim testi (ipc-protocol 13, agent-core 19) | macOS | PASS |
| `dev_transport_e2e` — 6 süreç düzeyi test, 5 tekrar | macOS, Unix socket | PASS (5/5) |
| `cargo clippy --workspace --all-targets` | macOS | PASS (uyarı yok) |
| `cargo check` + `clippy --all-targets` | `x86_64-pc-windows-msvc` | PASS (uyarı yok) — **yalnız derleme denetimi** |

### İstenen test listesi

| # | Test | Sonuç | Kanıt / not |
|---|---|---|---|
| 1 | IPC bağlantısı | PASS (Unix dev) · NOT RUN (Windows) | `e2e_helper_cli_ping_status_health` |
| 2 | Geçerli PING | PASS (Unix dev) · NOT RUN (Windows) | birim + e2e |
| 3 | Bilinmeyen komut reddi | PASS (Unix dev) | `LOCK`, `UNLOCK`, `PROCESS_KILL` → `UNKNOWN_COMMAND` |
| 4 | Geçersiz mesaj reddi | PASS (Unix dev) | 13 hatalı biçim; bilinmeyen alan; yarım çerçeve; 1 MB başlık |
| 5 | Yetkisiz istemci reddi | PASS (birim: politika + bağlantı döngüsü) · **BLOCKED** (gerçek başka kullanıcı/oturum/exe — ikinci hesap/Windows yok) | `unauthorized_peer_gets_generic_error_and_is_dropped` |
| 6 | Helper kapanması | PASS (Unix dev): SIGKILL sonrası agent tespiti **11–12 ms** (5 tekrar) · NOT RUN (Windows Görev Yöneticisi) | `e2e_helper_kill_is_detected_by_agent` |
| 7 | Agent kapanması | PASS (Unix dev): SIGINT → düzgün çıkış **12–202 ms**, soket silindi; helper tespiti ≤ **12–405 ms** (kalp atışı 200 ms) · NOT RUN (`sc stop`) | `e2e_agent_restart_...` |
| 8 | Bağlantı kopması | PASS (Unix dev): boşta kapanma 600 ms ayar → **~1020–1030 ms** (yoklama aralığı 500 ms) | `e2e_idle_connection_is_closed_by_agent` |
| 9 | Tekrarlanan mesaj kimliği | PASS (Unix dev) | `DUPLICATE_MESSAGE_ID` |
| 10 | Kontrollü yeniden başlatma | PASS (Unix dev): agent yeniden başlayınca helper **236–243 ms** içinde yeniden bağlandı; helper SIGINT ile 0 koduyla çıktı | aynı test |

### Windows'a özgü — durum (2026-10-28, P1.A.2 sonrası)

"CI" sütunu GitHub-hosted Windows Server 2022 runner'ı ([çalıştırma 40720517755](https://github.com/mehmetakarim/Kindora/actions/runs/40720517755), `runneradmin` yönetici hesap, oturum 2, LocalService servis adımı dahil); "Cihaz" sütunu kullanıcının Windows 11 Home/Pro bilgisayarıdır. Cihaz sütunu doldurulmadan Windows kabulü verilmez.

| ID | Test | CI | Cihaz |
|---|---|---|---|
| W-01 | MSVC ile bağlama ve `.exe` üretimi | PASS (release EXE'ler, PE32+ x86-64) | NOT RUN (EXE'lerin cihazda çalışması) |
| W-02 | Konsol agent + helper, named pipe üzerinden PING/STATUS/HEALTH | PASS (yönetici, aynı kullanıcı) | NOT RUN |
| W-03 | Yönetici olmayan helper `0x0012008B` (IU) DACL ile bağlanabiliyor mu | Sınanamaz (runner yükseltilmiş yönetici → BA girişi) | NOT RUN (mevcut hesapta yönetici olmayan PowerShell) |
| W-04 | Farklı ad/dizindeki exe → `UNAUTHORIZED` | PASS (`ExeNameMismatch`; LocalService serviste farklı dizindeki kopya `ExeDirMismatch`) | NOT RUN |
| W-05 | Uzak istemci reddi (`PIPE_REJECT_REMOTE_CLIENTS`) | Sınanamaz | NOT RUN |
| W-06 | Pipe adı önceden alınmışsa agent başlamıyor | PASS (`ERROR_ACCESS_DENIED` 5; ikinci agent `agent_failed`) | NOT RUN |
| W-07 | `sc create/start/query/stop/delete` yaşam döngüsü, LocalService | PASS (PowerShell 5.1, `start= demand`, `obj= NT AUTHORITY\LocalService`, `agent_stopped`, silme → 1060) | NOT RUN |
| W-08 | Yönetici olmayan pencereden `sc stop` → erişim reddi | Sınanamaz (runner yönetici) | NOT RUN |
| W-09 | Helper sonlandırma → tespit süresi | PASS (`TerminateProcess`, ≤ 3–5 ms) | NOT RUN (Görev Yöneticisi) |
| W-10 | **LocalService agent, helper sürecini `PROCESS_QUERY_LIMITED_INFORMATION` ile açabiliyor mu** | PASS (BUG-003 düzeltmesiyle; ACE'siz istemci `OpenProcess=5` ile reddedildi, helper `os_errors=-` ile yetkilendirildi) | `test.1`: **FAIL** (`OpenProcess=5`, BUG-003) · `test.2`: **PASS** (Lenovo, kullanıcı raporu: `os_errors=-`, `outcome=PeerClosed`) |
| W-11 | Servis durdurulurken bağlı helper'ın kaybı ve yeniden bağlanma | Sınanmadı (yalnız durdurma) | NOT RUN |
| W-12 | Session 0'dan gelen istemci reddi | Sınanmadı (runner oturumu 2) | NOT RUN (kapsam dışı) |

Prerelease'ler: [`p1a-windows-test.1`](https://github.com/mehmetakarim/Kindora/releases/tag/p1a-windows-test.1) (commit `17195b5`, BUG-003'ü içerir), [`p1a-windows-test.2`](https://github.com/mehmetakarim/Kindora/releases/tag/p1a-windows-test.2) (commit `a031ab7`, BUG-003 düzeltmesi). Paket SHA-256 değerleri release notlarında ve paketteki `SHA256SUMS.txt` dosyasındadır.

Unix geliştirme taşımasındaki PASS sonuçları **Windows davranışının kanıtı değildir**; yalnız platformdan bağımsız protokol/bağlantı mantığının ve süreç ayrımının kanıtıdır.

## 5a. Windows CI ve test paketi (P1.A.1)

- Workflow: [`.github/workflows/windows-p1a.yml`](../../.github/workflows/windows-p1a.yml) — `windows-2022` runner, Rust 1.94.0 (rustup), `--locked`: `fmt --check` → `check` → `test` → `clippy -D warnings` → release derleme → paket + SHA-256 → paket doğrulama → Actions artifact. `push` yalnız artifact üretir; prerelease yalnız `workflow_dispatch` + `publish_prerelease: true` ile, **aynı çalıştırmanın** artifact'ından (`contents: write` yalnız o işte).
- Paket: `Kindora-P1A-Windows-x64.zip` = `kindora-agent.exe`, `kindora-helper.exe`, [`README-TEST-TR.md`](../../packaging/p1a/README-TEST-TR.md), `BUILD-INFO.txt`, `SHA256SUMS.txt`.
- Windows'a özgü otomatik testler (yalnız Windows'ta derlenir/çalışır):
  - `windows_pipe_smoke.rs` (süreç içi, benzersiz pipe adları): gidiş-dönüş + işletim sisteminden pid/oturum/exe, protokol retleri, overlapped zaman aşımı/iptal, istemci kopması, `accept` kapanması, `FILE_FLAG_FIRST_PIPE_INSTANCE` ile ad gaspının engellenmesi, gerçek `helper_policy()`'nin test ikilisini reddetmesi.
  - `windows_process_e2e.rs` (gerçek EXE'ler, üretim pipe adı): helper CLI PING/STATUS/HEALTH, yetkisiz istemci reddi, `TerminateProcess` sonrası tespit, ikinci agent'ın başlayamaması. Beklenen sonuç runner oturumuna göre (oturum 0 → politika reddi) açıkça doğrulanır.
  - `helper_grants_only_query_limited_to_local_service` (P1.A.2): helper DACL'ine tam olarak `(A;;0x1000;;;LS)` eklendiği.
- LocalService servis uçtan uca (P1.A.2): [`ci/p1a-localservice-e2e.ps1`](../../ci/p1a-localservice-e2e.ps1) release EXE'leri test rehberindeki komutlarla (Windows PowerShell 5.1) `KindoraAgentDev` servisi olarak kurar; ACE'siz istemci reddi (`OpenProcess=5`), helper PING/STATUS/HEALTH, farklı dizin reddi, durdurma ve silme doğrulanır. Özet satırları (`LOCALSERVICE ...`) artifact'taki `ci-test-summary.txt` ve release notlarındadır.
- **CI'ın doğrulayamadıkları:** runner hesabı yükseltilmiş yöneticidir (DACL'deki BA girişiyle bağlanır → `IU` maskesi sınanmaz, W-03; `sc stop` reddi sınanmaz, W-08); W-11 yeniden bağlanma sınanmaz; Windows Server runner etkileşimli Windows 11 oturumu değildir. Gerçek cihaz prosedürü: [README-TEST-TR.md](../../packaging/p1a/README-TEST-TR.md).

## 6. Güvenlik bulguları

1. **Varsayılan pipe DACL'i** Everyone'a okuma verir → açık SDDL ile değiştirildi; etkileşimli kullanıcıya `FILE_CREATE_PIPE_INSTANCE` verilmez.
2. **Interactive Users** grubu, ADR-0007'deki oturum logon SID'inden geniştir. Logon SID'i servis tarafında almak kullanıcı token'ı gerektirir (`WTSQueryUserToken` → LocalSystem). Oturum ve exe denetimleriyle telafi edildi; Windows PoC'de yeniden değerlendirilecek.
3. **Exe yolu denetimi** yalnız agent dizini yöneticiye özelse anlamlıdır (Program Files). Çocuk kendi helper sürecine (aynı kullanıcı) hata ayıklayıcıyla müdahale edebilir → helper **hiçbir yetkili karar veremez** (protokolde politika değiştiren komut yok).
4. **LocalService** süreç yolu sorgusu varsayılan DACL nedeniyle cihazda başarısız oldu (W-10 FAIL, [BUG-003](../solutions.md)). Düzeltme: helper kendi DACL'ine LocalService için yalnız `PROCESS_QUERY_LIMITED_INFORMATION` ekler; kimlik doğrulama gevşetilmedi (ACE'siz istemci yine reddedilir). Kalan risk: izin tüm LocalService servislerine açık; servis SID'iyle daraltma ADR-0006'da.
5. **Helper sunucuyu doğrulamıyor:** agent çalışmıyorken başka bir süreç aynı pipe adını açabilir. `SECURITY_IDENTIFICATION` taklidi engeller; ancak sahte yanıtlar mümkündür. P1.A'da etkisi düşük (yalnız durum); sonraki fazda helper, sunucu PID'inin Kindora servisi olduğunu doğrulamalı.
6. **Tekrar tespiti bağlantı başınadır** ve bellekte tutulur; yeniden bağlanınca sıfırlanır. Durum değiştiren komutlar geldiğinde kalıcı (SQLite) idempotency gerekir (ADR-0009).
7. **Zamanlama ayrıntısı:** boşta kapanma ve kapanma bayrağı yoklama aralığına (500 ms) bağlı; helper agent kaybını bir sonraki kalp atışında fark eder (≤ kalp atışı + yanıt zaman aşımı).
8. Unix geliştirme taşıması yalnız kullanıcı kimliğini denetler; **üretim taşıması değildir** ve Windows derlemesine girmez.
9. Sunucu ucunda `DisconnectNamedPipe` çağrılmaz (okunmamış veriyi, ör. `UNAUTHORIZED` yanıtını, siler); her bağlantı yeni örnek olduğu için tanıtıcıyı kapatmak yeterli [Windows'ta doğrulanacak]. Helper, yazma hatasında tamponda bekleyen `UNAUTHORIZED` yanıtını okur.
10. Kimlik alınamazsa agent kaydına Windows hata kodu yazılır (`os_errors=OpenProcess=5` gibi) — W-10 kanıtı için.
11. Parola, token, çocuk verisi, keyfi komut çalıştırma, Ctrl+Alt+Del/secure desktop müdahalesi, kalıcılaştırma **yok**.

## 7. Yorum notu

"Helper PING komutuna yanıt versin" maddesi bu prototipte şöyle uygulandı: helper istemcidir; PING gönderir, yanıtı doğrular ve kalp atışı olarak kullanır. Agent → helper yönünde istek yoktur; agent helper'ın kaybını bağlantı kapanmasıyla (ölçüldü: ~12 ms, Unix dev) ve boşta kalma zaman aşımıyla algılar. Çift yönlü istek gerekirse protokol v2 kararı gerekir.

# Kindora — Doğrulanmış Teknik Sorunlar ve Çözümler

Yalnızca **gerçekte yaşanmış ve doğrulanmış** teknik sorunlar kaydedilir. Tasarım veya kabul bulguları buraya değil, [tasks.md](tasks.md) içindeki "Kayıtlı açık bulgular" tablosuna yazılır.

## Şablon

```markdown
### BUG-NNN — Kısa başlık
- **Belirti:**
- **Ortaya çıktığı koşul:** (OS/sürüm, cihaz, adımlar)
- **Kök neden:**
- **İşe yaramayan önemli denemeler:**
- **Uygulanan çözüm:**
- **Doğrulama yöntemi:**
- **İlgili görev, dosya veya commit:**
```

## Kayıtlar

### BUG-001 — Süreç düzeyi testler `kindora-helper` ikilisini bulamıyor (temiz derleme)

- **Belirti:** GitHub Actions `windows-2022` çalıştırması [40666717224](https://github.com/mehmetakarim/Kindora/actions/runs/40666717224): `windows_process_e2e` testi `helper_bin()` doğrulamasında panik (`windows_process_e2e.rs:32`), `cargo test` çıkış kodu 101.
- **Ortaya çıktığı koşul:** Temiz `target/` dizini (CI). `windows-agent` paketinin entegrasyon testleri, **başka bir paketin** (`windows-session`) ikilisini `target/<profil>/kindora-helper[.exe]` yolunda arıyor.
- **Kök neden:** Cargo, `cargo test` sırasında yalnız aynı paketin ikililerini entegrasyon testleri için derler (`CARGO_BIN_EXE_*`). Diğer paketin `[[bin]]` hedefi derlenmez; macOS'ta geçmesinin nedeni önceden çalıştırılmış `cargo build`'in bıraktığı ikiliydi (bayat ikili riski).
- **İşe yaramayan önemli denemeler:** Yok (ilk analizde tespit edildi).
- **Uygulanan çözüm:** CI'da `cargo test`'ten önce `cargo build --workspace --locked` adımı; test hata mesajı "önce `cargo build --workspace`" diye düzeltildi. Yerel komut sırası: `cargo build --workspace && cargo test --workspace`.
- **Doğrulama yöntemi:** macOS'ta `target/debug/kindora-helper` silinip `cargo test` → aynı panik yeniden üretildi; `cargo build` sonrası 38/38 test geçti. Windows doğrulaması: düzeltme commit'inin CI çalıştırması.
- **İlgili görev, dosya veya commit:** TASK-005; `.github/workflows/windows-p1a.yml`, `crates/windows-agent/tests/{dev_transport_e2e,windows_process_e2e}.rs`; hata commit'i `3c60d42`.
- **Not:** İleride `windows-agent` testlerinin helper'ı Cargo'nun artifact dependency özelliğiyle (kararlı hâle gelirse) derletmesi değerlendirilebilir.

### BUG-002 — CI hash dosyaları CRLF ile yazılıyor; Linux/macOS `sha256sum -c` başarısız

- **Belirti:** Başarılı CI çalıştırması [40667002377](https://github.com/mehmetakarim/Kindora/actions/runs/40667002377) artifact'ında `shasum -a 256 -c Kindora-P1A-Windows-x64.zip.sha256` → `No such file or directory`. `od -c` dosya adının `...zip\r\n` ile bittiğini gösterdi. Aynı sorun `SHA256SUMS.txt` için de geçerliydi.
- **Ortaya çıktığı koşul:** Hash dosyaları Windows runner'da PowerShell `Set-Content` ile üretilip Linux/macOS'ta doğrulandığında (ör. prerelease işinin `sha256sum -c` adımı).
- **Kök neden:** Windows'ta `Set-Content` satır sonu olarak CRLF yazar; GNU/BSD `sha256sum -c` `\r`'yi dosya adının parçası sayar.
- **İşe yaramayan önemli denemeler:** Yok.
- **Uygulanan çözüm:** Paketleme adımında metin dosyaları `[IO.File]::WriteAllText` ile LF satır sonuyla yazılır; doğrulama adımı CR içeren hash dosyasında CI'ı başarısız kılar.
- **Doğrulama yöntemi:** Hatalı artifact macOS'ta `tr -d '\r'` ile doğrulandığında tüm hash'ler eşleşti (içerik sağlam, yalnız biçim hatalı). Düzeltme sonrası CI artifact'ı macOS'ta doğrudan `shasum -a 256 -c` ile doğrulanacak.
- **İlgili görev, dosya veya commit:** TASK-005; `.github/workflows/windows-p1a.yml`; hata commit'i `36e657c`.

### BUG-003 — LocalService agent helper sürecini açamıyor (`OpenProcess=5`)

- **Belirti:** Kullanıcının gerçek Windows cihazında (mevcut Lenovo hesabı, `p1a-windows-test.1`) servis olarak çalışan agent'ın kaydı: `os_errors=OpenProcess=5` ve `outcome=Unauthorized(ExePathUnknown)`. Helper `hata: unauthorized`, `exit=3`. Agent, pipe üzerinden PID ve oturum kimliğini okuyabiliyordu.
- **Ortaya çıktığı koşul:** Agent `NT AUTHORITY\LocalService` hesabıyla servis olarak çalışıyor, helper etkileşimli oturumda kullanıcı hesabıyla çalışıyor (W-10). Konsol modunda agent ve helper aynı kullanıcı olduğu için görünmüyordu; P1.A.1 CI'ı servis kurmadığı için yakalayamadı.
- **Kök neden:** PID ve oturum pipe tanıtıcısından gelir (`GetNamedPipeClientProcessId` / `GetNamedPipeClientSessionId`) ve süreç erişimi gerektirmez. Exe yolu ise `OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION)` ister ve bu çağrı helper sürecinin DACL'iyle denetlenir. Kullanıcı süreçlerinin varsayılan DACL'i (token default DACL) yalnız kullanıcıya/yöneticilere ve SYSTEM'e izin verir. CI'da ölçülen helper DACL'i: `D:(A;;0x1fffff;;;LA)(A;;0x1fffff;;;BA)(A;;0x1fffff;;;SY)`. LocalService (`S-1-5-19`) listede yok, yani `ERROR_ACCESS_DENIED` (5). LocalService'te `SeDebugPrivilege` de yoktur (olmamalıdır). Exe yolu bilinmeyince politika fail-closed reddetti.
- **Değerlendirilen ve seçilmeyen yöntemler:**
  - LocalSystem: kullanıcı talimatıyla dışlandı, en az yetki ilkesine aykırı.
  - `ImpersonateNamedPipeClient` ile kullanıcı adına açma: istemci `SECURITY_IDENTIFICATION` ile bağlanır, bu düzeyde nesne erişimi yapılamaz. Düzeyi yükseltmek sahte sunucunun kullanıcıyı taklit etmesine kapı açar.
  - `NtQuerySystemInformation(SystemProcessIdInformation)`: süreç açmadan yol verir ama belgelenmemiş bir sınıftır.
  - Toolhelp anlık görüntüsü: yalnız exe adı verir; yalnız ada güvenmek yasak.
  - Kimlik denetimini kaldırmak: yasak.
- **Uygulanan çözüm:**
  - Helper başlangıçta kendi süreç DACL'ine **yalnız** LocalService için `PROCESS_QUERY_LIMITED_INFORMATION` (0x1000) ACE'si ekler (`GetSecurityInfo` → `SetEntriesInAclW` → `SetSecurityInfo`; `ipc_protocol::transport::windows_pipe::grant_agent_process_query`).
  - Bu izin bellek erişimi, sonlandırma veya token erişimi vermez.
  - Kimlik doğrulama değişmedi: exe yolu yine işletim sisteminden okunur; oturum ≠ 0, ad ve dizin politikası uygulanır.
  - İzni vermeyen istemci yine `OpenProcess=5` ile reddedilir.
  - İzin başarısız olursa helper `process_query_grant_failed` kaydı yazar ve agent reddeder (fail closed).
- **Doğrulama yöntemi:**
  - Birim testi `helper_grants_only_query_limited_to_local_service`: DACL'e tam olarak `(A;;0x1000;;;LS)` eklendiğini, işlemin idempotent olduğunu doğrular.
  - CI adımı [`ci/p1a-localservice-e2e.ps1`](../ci/p1a-localservice-e2e.ps1), [çalıştırma 40720517755](https://github.com/mehmetakarim/Kindora/actions/runs/40720517755). Release EXE'ler test rehberindeki komutlarla (Windows PowerShell 5.1) gerçek LocalService servisi olarak kuruldu. Sonuçlar:
    - ACE'siz yabancı istemci (powershell.exe): `os_errors=OpenProcess=5 outcome=Unauthorized(ExePathUnknown)`, cihazdaki hatanın birebir yeniden üretimi.
    - Helper PING/STATUS/HEALTH: `exit=0`, `exe="C:\\Program Files\\KindoraDev\\kindora-helper.exe" os_errors=- outcome=PeerClosed`.
    - Farklı dizindeki kopya: `Unauthorized(ExeDirMismatch)`.
    - Servis durdu ve silindi.
  - Gerçek cihaz doğrulaması: `p1a-windows-test.2` ile kullanıcı testi bekleniyor.
- **İlgili görev, dosya veya commit:** TASK-006 (P1.A.2); `crates/ipc-protocol/src/transport/windows_pipe.rs`, `crates/windows-session/src/main.rs`, `crates/windows-agent/tests/windows_pipe_smoke.rs`, `ci/p1a-localservice-e2e.ps1`, `.github/workflows/windows-p1a.yml`; düzeltme commit'i `a031ab7`.
- **Kalan risk:** Aynı makinedeki diğer LocalService servisleri de helper'ın exe yolunu ve zamanlarını sorgulayabilir. İleride servis SID'i (`NT SERVICE\KindoraAgent`, `sc.exe sidtype ... restricted`) ile daraltılabilir (ADR-0006).
- **Ek bulgu (aynı faz):**
  - Önceki rehberdeki `sc.exe create ... binPath= "\"C:\Program Files\...\" service"` satırı PowerShell'de güvenilir değildir; iç tırnaklar PowerShell 5.1'de bozulur.
  - Rehber ve CI artık `sc.exe --% create ...` kullanıyor; CI'da PowerShell 5.1 ile `sc.exe qc` çıktısı doğrulandı.

### BUG-004 — Windows CI'da Prettier tüm ön yüz dosyalarını reddediyor (CRLF checkout)

- **Belirti:** [40753324808](https://github.com/mehmetakarim/Kindora/actions/runs/40753324808) çalıştırmasında `npm run format:check` başarısız oldu: "Code style issues found in 30 files". macOS'ta aynı komut geçiyordu.
- **Ortaya çıktığı koşul:** GitHub-hosted `windows-2022` runner'ında `actions/checkout`. Depoda `.gitattributes` yoktu.
- **Kök neden:** Windows'ta Git'in `core.autocrlf=true` varsayılanı metin dosyalarını CRLF ile çıkarır. Prettier'ın `endOfLine: "lf"` varsayılanı her satırı fark olarak görür. Aynı risk BUG-002'deki hash dosyası sorununa da yol açar.
- **Uygulanan çözüm:** Kök `.gitattributes` dosyası: `* text=auto eol=lf`; görsel ve font dosyaları `binary`. Depoda mevcut CRLF dosyası olmadığı `git add --renormalize .` ile doğrulandı.
- **Doğrulama yöntemi:** Düzeltme commit'inin (`2fb82cf`) `windows-p1b` çalıştırmasında format denetimi.
- **İlgili görev, dosya veya commit:** TASK-007; `.gitattributes`; hata commit'i `1695264`.

### BUG-005 — .NET `StandardInput` BOM'u köprü isteğini reddettiriyor

- **Belirti:** `windows-p1a` çalıştırması [40753324764](https://github.com/mehmetakarim/Kindora/actions/runs/40753324764), LocalService betiği adım 3b:
  - Köprü bağlandı ve SERVICE snapshot'ı geldi.
  - PowerShell'den yazılan `{"op":"refresh"}` sonrasında 3 sn içinde yeni snapshot gelmedi: `TIMEOUT: bridge line /"type":"snapshot"/`.
- **Ortaya çıktığı koşul:** Helper `System.Diagnostics.Process` ile `RedirectStandardInput` kullanılarak başlatılıyor (Windows PowerShell 5.1 / .NET Framework). Rust testleri (Rust'tan yazılan stdin) geçiyordu.
- **Kök neden:**
  - .NET'in `StandardInput` yazıcısı, giriş kodlaması UTF-8 olduğunda akışın başına BOM (`EF BB BF`) yazabilir.
  - Katı ayrıştırıcı ilk satırı `\u{feff}{"op":"refresh"}` olarak görüp reddetti.
  - Ayrıca ret olayı yalnız bir sonraki olayla birlikte yazıldığı için hata 3 sn'lik yoklamaya kadar görünmedi.
  - (Çıkarım: BOM, macOS'ta önekli satırla yeniden üretildi; Windows doğrulaması düzeltme sonrası CI'dadır.)
- **Uygulanan çözüm:**
  - `ipc_protocol::bridge` yalnız baştaki BOM'u atar; diğer her şey yine reddedilir.
  - Helper reddedilen satırı **hemen** `request_rejected` olarak bildirir.
  - Birim testine BOM'lu satır eklendi.
- **Doğrulama yöntemi:**
  - macOS: `printf '\xef\xbb\xbf{"op":"refresh"}\n{"op":"LOCK"}\n' | kindora-helper bridge` komutunda refresh kabul edildi, LOCK anında reddedildi.
  - Windows: düzeltme commit'inin LocalService betiği.
- **İlgili görev, dosya veya commit:** TASK-007; `crates/ipc-protocol/src/bridge.rs`, `crates/windows-session/src/main.rs`; hata commit'i `1695264`, düzeltme `2fb82cf`.

### NOTE-006 — macOS'ta Windows hedefli clippy, gömülü SQLite nedeniyle derlenemiyor

- **Belirti:** P1.C'de `cargo clippy --workspace --target x86_64-pc-windows-msvc` macOS'ta `libsqlite3-sys` derleme betiğinde durdu: `fatal error: 'stdlib.h' file not found`.
- **Ortaya çıktığı koşul:** `rusqlite` `bundled` özelliğiyle SQLite C kaynağını derler. macOS'ta MSVC/Windows SDK başlıkları yoktur. `LIBSQLITE3_SYS_BUNDLING=0` ve macOS SDK başlıklarıyla denemeler de başarısız oldu.
- **Kök neden:** Çapraz derleme için Windows C araç zinciri gerekir; Rust kodu değil, C bağımlılığı derlenemiyor.
- **Uygulanan çözüm:**
  - macOS'ta SQLite'a bağlı olmayan crate'ler Windows hedefiyle denetlenir: `cargo clippy -p ipc-protocol -p agent-core -p windows-session --all-targets --target x86_64-pc-windows-msvc -- -D warnings`.
  - `windows-agent` ve `screen-time`'ın Windows kodu (`platform.rs`) yalnız Windows CI'da (`windows-2022`, MSVC) derlenir ve test edilir.
- **Doğrulama yöntemi:** P1.C commit'inin `windows-p1c` çalıştırmasında `cargo build/test/clippy --workspace`.
- **İlgili görev, dosya veya commit:** TASK-008; `Cargo.toml` (`rusqlite` bundled), `crates/windows-agent/src/platform.rs`.

### NOTE-007 — `windows-sys` `WNDCLASSW` / `RegisterClassW` için `Win32_Graphics_Gdi` özelliği gerekir

- **Belirti:** P1.D'de oturum sonu penceresi (`crates/windows-session/src/session_end.rs`) Windows hedefli clippy'de `no RegisterClassW / WNDCLASSW in Win32::UI::WindowsAndMessaging` hatası verdi.
- **Kök neden:** `windows-sys` 0.61'de bu öğeler `#[cfg(feature = "Win32_Graphics_Gdi")]` arkasındadır (`HBRUSH` alanı). Yalnız `Win32_UI_WindowsAndMessaging` yetmez.
- **Uygulanan çözüm:** Kök `Cargo.toml` `windows-sys` özelliklerine `Win32_Graphics_Gdi`, `Win32_UI_WindowsAndMessaging` ve `Win32_System_LibraryLoader` eklendi.
- **Doğrulama yöntemi:** macOS'ta `cargo clippy -p windows-session --target x86_64-pc-windows-msvc --all-targets -- -D warnings` PASS; Windows CI'da `session_end` testi gerçek `WM_ENDSESSION` iletisiyle.
- **İlgili görev, dosya veya commit:** TASK-009; `Cargo.toml`, `crates/windows-session/src/session_end.rs`.

### NOTE-008 — Lenovo'da uyku süresi "uyku" değil "ölçülemeyen" sayıldı (P1.C C-08)

- **Belirti:** `p1c-windows-test.1` Lenovo testinde Windows uykusundan sonra uyku sayacı 0 kaldı, ölçülemeyen süre arttı; aktif süre artmadı.
- **Ortaya çıktığı koşul:** Gerçek Lenovo dizüstü, Başlat → Güç → Uyku. Model, Windows sürümü ve Modern Standby (S0ix) olup olmadığı raporlanmadı.
- **Kök neden:** **Doğrulanmadı.** Olası nedenler [Çıkarım]:
  - Servise `PBT_APMSUSPEND` / `PBT_APMRESUMEAUTOMATIC` gelmedi (Modern Standby'da beklenen davranış olabilir).
  - Agent döngüsü uyku boyunca kesilmeden işledi; helper raporu eskidi (`STALE_REPORT`).
  - Monoton saat uykuda ilerlemedi; aralık 5 sn'yi aşınca `UNKNOWN` yazıldı.
- **Uygulanan çözüm:** Davranış güvenli yönde (aktif sayılmıyor) ve README C-08 bunu kabul ediyor. P1.D'de doğrulanamayan uyku zorla `SUSPENDED` yapılmadı. Agent artık pil durumu değişimi dışındaki bütün `POWER_EVENT` türlerini `event=power_event event=… applied=…` olarak kaydeder.
- **Doğrulama yöntemi:** P1.D Lenovo D-11 (isteğe bağlı): uyku sonrası agent kaydındaki `power_event` satırları ve tanılamadaki ölçülemeyen nedeni.
- **İlgili görev, dosya veya commit:** TASK-008, TASK-009; `crates/windows-agent/src/service.rs`.

### NOTE-009 — `Export-ScheduledTask` varsayılan değerleri yazmaz; görev denetimi nesne özellikleriyle yapılır

- **Belirti:** P1.D ilk CI çalıştırmasında ([41043545839](https://github.com/mehmetakarim/Kindora/actions/runs/41043545839)) kurulum, görev ve helper başlatma başarılı oldu. Buna rağmen `task: LeastPrivilege + InteractiveToken` denetimi başarısız oldu.
- **Kök neden:** Dışa aktarılan XML, varsayılan olan `<RunLevel>LeastPrivilege</RunLevel>` gibi öğeleri içermiyor. Boş alan, beklenen değerle karşılaştırılınca hata verdi.
- **Uygulanan çözüm:** `ci/p1d-background-e2e.ps1` görevi `Get-ScheduledTask` nesnesinden denetler: `Principal.RunLevel=Limited`, `LogonType=Interactive`, `Settings.*`, tetikleyici sınıfları ve `Actions`.
- **Aynı incelemede düzeltilen tasarım açığı [Çıkarım]:**
  - Oturum açılışı tetikleyicisinin tekrarı (`Repetition`) yalnız tetikleyici gerçekten ateşlenince başlar. Kurulumdan veya elle başlatmadan sonra çöken helper, bir sonraki oturum açılışına kadar yeniden başlatılmazdı.
  - Yeniden denetim ayrı bir zaman tetikleyicisine taşındı. Kurulum ve kaldırma, süreçleri sonlandırmadan önce görevi durdurur (`Stop-ScheduledTask`); böylece `IgnoreNew` yeni örneği engellemez.
- **Doğrulama yöntemi:** `windows-p1d` CI §4 (helper sonlandırılır → zaman tetikleyicisi yeniden başlatır).
- **İlgili görev, dosya veya commit:** TASK-009; `packaging/p1d/install-kindora-test.ps1`, `ci/p1d-background-e2e.ps1`.

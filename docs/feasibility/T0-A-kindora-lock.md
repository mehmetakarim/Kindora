# T0-A — Kindora Kullanım Kilidi Uygulanabilirliği

> Durum: **Taslak / öneri** · Kapı: GATE T0-A · İlgili karar: [ADR-0001](../decisions/0001-kindora-kullanim-kilidi-ana-kontrol.md)
> Windows üzerinde hiçbir şey çalıştırılmadı. Tüm davranış iddiaları [Resmi kaynak] / [Çıkarım] / [PoC gerekli] etiketlidir.

## 1. Hedef akış ve bileşen eşlemesi

| # | Adım | Sorumlu bileşen (öneri) |
|---|---|---|
| 1 | Süre/mola yaklaşır | Policy Engine (servis) — yerel sayaç |
| 2 | Çocuğa önceden bildirim (5 dk / 1 dk) | Session Helper → Tauri UI / widget |
| 3 | Tam ekran mola/kısıtlama deneyimi | Session Helper (kısıtlama yüzeyi) + Tauri UI (içerik) |
| 4 | Yerel motor kısıtlamayı uygular | Policy Engine kararı → Helper uygulaması + servis denetimi |
| 5 | Çocuk ek süre ister | Tauri UI → Helper → Servis → Supabase (`permission_requests`) |
| 6 | Ebeveyn onaylar | Parent Desktop / Flutter → Edge Function |
| 7 | Cihaz onayı doğrular | Servis: imza/cihaz kimliği/TTL/policy_version kontrolü |
| 8 | Kısıtlama kaldırılır | Servis → Helper (`RELEASE_KINDORA_RESTRICTION`) |
| 9 | Geçmiş güncellenir | Servis: SQLite olay kaydı → `command_receipts` / `audit_events` |

**Kural:** Karar her zaman servis (Policy Engine) tarafındadır. UI ve Helper yalnızca **sunum ve yerel uygulama** yapar; çocuk UI'dan gelen hiçbir mesaj kısıtlamayı kaldıramaz.

## 2. Windows oturum modeli — bulgular

| Konu | Bulgu | Kanıt |
|---|---|---|
| Session 0 isolation | Vista'dan beri servisler kullanıcıyla doğrudan etkileşemez; UI için kullanıcı oturumunda ayrı süreç + IPC önerilir. Çok kullanıcılı sistemde helper `HKLM\...\Run` ile her oturumda başlatılabilir; named pipe adı oturum kimliğine göre ayrılabilir. | [Resmi kaynak](https://learn.microsoft.com/en-us/windows/win32/services/interactive-services) |
| Yükseltilmiş pencere riski | LocalSystem gibi yükseltilmiş bağlamdaki süreçler etkileşimli masaüstünde pencere oluşturmamalı; aynı masaüstündeki diğer uygulamalar o pencereyle etkileşebilir. | [Resmi kaynak](https://learn.microsoft.com/en-us/windows/win32/services/interactive-services) |
| Servisten kullanıcı oturumunda süreç başlatma | `WTSQueryUserToken` yalnızca LocalSystem + `SE_TCB_NAME` ile çağrılabilir; "yüksek güvenilirlikli servisler" içindir. **Bu gereksinim yalnız servisin helper'ı kullanıcı token'ıyla kendisinin başlattığı yöntem (M2) için geçerlidir**; HKLM Run anahtarı veya oturum açma tetikli zamanlanmış görev gibi yöntemler bu API'yi gerektirmez. | [Resmi kaynak](https://learn.microsoft.com/en-us/windows/win32/api/wtsapi32/nf-wtsapi32-wtsqueryusertoken) |
| Windows oturum kilidi | `LockWorkStation` yalnızca etkileşimli masaüstündeki süreçten çağrılabilir; asenkron çalışır, başarı dönüşü kilidin gerçekleştiğini **garanti etmez**; kilit durumunu sorgulayan fonksiyon yoktur — `WTSRegisterSessionNotification` ile izlenmelidir. Açmak için kullanıcı oturum açmalıdır. | [Resmi kaynak](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-lockworkstation) |
| Low-level klavye kancası | `WH_KEYBOARD_LL` yalnızca global kapsamlı, kurulduğu masaüstüyle sınırlı; kancalayan iş parçacığı mesaj pompalamalıdır. | [Resmi kaynak](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwindowshookexw) |
| Ctrl+Alt+Del (SAS) | Güvenli dikkat dizisi Winlogon tarafından işlenir; kullanıcı modu kancalarıyla engellenemez. | [Çıkarım] (genel Windows güvenlik modeli; PoC ile gözlemlenecek) |
| Assigned Access / kiosk | Yalnızca Pro, Enterprise/LTSC, Education, IoT Enterprise/LTSC sürümlerinde. **Windows Home desteklenmez.** | [Resmi kaynak](https://learn.microsoft.com/en-us/windows/configuration/assigned-access/overview), [kiosk yöntemleri](https://learn.microsoft.com/en-us/windows/configuration/kiosk-methods) |

### Sonuç: Kullanıcı oturumundaki helper çocuğun kendi süreci olursa öldürülebilir

Çocuğun standart hesabıyla çalışan bir süreç, aynı kullanıcı tarafından Görev Yöneticisi ile sonlandırılabilir **[Çıkarım — PoC gerekli]**. Bu nedenle "helper çalıştığı sürece kilit var" modeli tek başına yetersizdir. Üç uygulama modeli değerlendirildi:

| Model | Açıklama | Artı | Eksi |
|---|---|---|---|
| **M1 — Kullanıcı bağlamında helper** (Run anahtarı / Görev Zamanlayıcı) | Helper çocuk kimliğiyle çalışır | En az yetki; UI güvenliği basit | Çocuk sonlandırabilir; servis yeniden başlatamaz (LocalSystem değilse) |
| **M2 — Servis (LocalSystem) helper'ı çocuk oturumunda çocuk token'ıyla başlatır** | `WTSQueryUserToken` + `CreateProcessAsUser` | Servis helper ölünce yeniden başlatır (watchdog) | Servis LocalSystem olmalı; çocuk yine sonlandırabilir, yalnızca yeniden doğar (yarış penceresi) |
| **M3 — Ayrıcalıklı "kısıtlama yüzeyi"** (çocuk tarafından sonlandırılamayan kimlikle oturumda süreç) | Örn. SYSTEM token'ının oturum kimliği değiştirilerek çocuk oturumunda süreç | Çocuk sonlandıramaz | Microsoft'un yükseltilmiş pencere uyarısına aykırı risk (shatter/UI saldırı yüzeyi); en yüksek güvenlik riski |

**Öneri (ADR-0006, Önerildi):** M2 + **ikinci savunma hattı**. Helper sonlandırılırsa servis (a) helper'ı yeniden başlatır, (b) tekrarlayan sonlandırmada kısıtlama süresi içindeyse **Windows oturum kilidini veya oturum bağlantısını kesmeyi** yedek önlem olarak değerlendirir (`LOCK_WINDOWS_SESSION` ayrı komut olarak kalır; burada yalnızca kurcalama tepkisidir ve ebeveyn ayarıyla açılır/kapanır), (c) olayı `audit_events` ve ebeveyn bildirimi olarak kaydeder. M3 **önerilmez**.

> Servisten oturum kilitleme/bağlantı kesme yöntemi (`WTSDisconnectSession` vb.) ve davranışı **[PoC gerekli]** — bu belge yöntemin çalıştığını iddia etmez.

## 3. Pencere ve klavye senaryoları (beklenen davranış — doğrulanmadı)

| Senaryo | Beklenen | Etiket | Karşı önlem (aday) |
|---|---|---|---|
| Tam ekran + always-on-top kısıtlama penceresi | Diğer pencerelerin üstünde; ancak başka topmost pencereler, bildirimler, UAC (secure desktop) üstte olabilir | [Çıkarım] | Pencereyi periyodik ön plana alma; ön plan değişimini izleme (`SetWinEventHook` EVENT_SYSTEM_FOREGROUND) |
| Alt+Tab | Odak başka pencereye geçebilir | [PoC gerekli] | LL klavye kancası ile kısıtlama sırasında yutma + ön plan geri alma |
| Win+D / Win+M | Masaüstünü gösterir, pencere küçülebilir | [PoC gerekli] | Kanca + küçülmeyi engelleme / yeniden gösterme |
| Win+L | Windows kilitlenir — kısıtlamayı bozmaz, açılışta kısıtlama sürmeli | [PoC gerekli] | Oturum bildirimiyle yeniden uygulama |
| Ctrl+Alt+Del → Görev Yöneticisi | Engellenemez; çocuk kendi süreçlerini sonlandırabilir | [Çıkarım] | Helper sonlandırma tepkisi (§2); HKCU `DisableTaskMgr` politikası (Home'da etkisi [PoC gerekli]) |
| Ctrl+Shift+Esc | Görev Yöneticisi doğrudan | [PoC gerekli] | Aynı |
| Yeni oturum / kullanıcı değiştirme | Kısıtlama yalnızca çocuk oturumuna aittir; başka hesap çocuğun hesabı değildir | [Çıkarım] | Policy kullanıcı/oturum bazlı; ebeveyn hesabı kısıtlanmaz |
| Çoklu monitör | Tek pencere yalnızca bir monitörü kaplar | [Çıkarım] | Her monitör için kısıtlama penceresi; `WM_DISPLAYCHANGE` izleme |
| DPI %125/%150 | Per-monitor DPI v2 gerekli | [PoC gerekli] | Tauri/WebView2 manifest ayarı; GATE D |
| Tam ekran oyun (exclusive fullscreen) | Topmost pencere görünmeyebilir | [PoC gerekli] | Ön plan sürecini küçültme/askıya alma **yapılmaz** (veri kaybı); bunun yerine bildirim + 1 dk sonra kurcalama tepkisi |
| Uyku / uyanma | Sayaç durmalı; uyanınca kısıtlama yeniden değerlendirilmeli | [PoC gerekli] | `WM_POWERBROADCAST` / servis güç olayları |
| Yeniden başlatma | Kısıtlama SQLite'tan yüklenip helper başlar başlamaz uygulanmalı | [PoC gerekli] | Servis otomatik başlangıç; helper logon'da |

**Kabul ilkesi:** Tam ekran pencere tek başına güvenilir kilit değildir. Kindora kilidi **"caydırıcı ve açıklayıcı kullanım kısıtlaması + kurcalama tespiti ve tepkisi"** olarak konumlanır; "kırılamaz kilit" iddiası yapılmaz.

## 4. Güvenlik sınırları (açıkça tanımlı)

**Kapsam içi (hedeflenen):**
- Standart çocuk hesabıyla sıradan kısayollar (Alt+Tab, Win+D) ile kısıtlamanın atlanmasını zorlaştırmak.
- Helper sonlandırma, saat değiştirme, ağ kesme girişimlerini tespit edip politika gereği tepki vermek ve ebeveyne bildirmek.
- Yeniden başlatma/uyku sonrası kısıtlamanın sürmesi.

**Kapsam dışı (garanti verilmez):**
- Ctrl+Alt+Del, UAC secure desktop, güvenli önyükleme, BIOS/UEFI, harici önyüklenebilir ortam, Güvenli Mod.
- Çocuğun yönetici hesabına erişimi (bu durumda tüm yerel denetimler devre dışı bırakılabilir).
- Fiziksel erişimle diskin değiştirilmesi.

**Yasak yaklaşımlar (asla):** Windows parolası saklama/toplama, kimlik bilgisi sağlayıcısı/GINA değiştirme, secure desktop manipülasyonu, desteklenmeyen otomatik oturum açma, keylogger, ekran kaydı.

## 5. Standart kullanıcı ve yönetici gereksinimleri

- Çocuk: **standart kullanıcı** (zorunlu ön koşul). Kurulum sırasında çocuk hesabının yönetici olduğu tespit edilirse kurulum uyarı verir ve ebeveyni bilgilendirir.
- Kurulum: yönetici hakkı gerekir (servis kaydı, Program Files, HKLM).
- Ebeveyn: Windows'ta ayrı yönetici hesabı önerilir; kurtarma bu hesapla yapılır.
- Windows sürümü: Home dahil hedeflenmeli; bu yüzden Assigned Access **ana yöntem olamaz** (yalnızca Pro+ için isteğe bağlı güçlendirme seçeneği).

## 6. Kilit PoC tasarımı (PoC-A)

**Amaç:** Kindora kilidinin gerçek güvenilirlik düzeyini ölçmek; sonuç "kırılabilir/kırılamaz" değil, **senaryo başına tespit + tepki süresi** tablosudur.

**Ortam matrisi:** Windows 11 Home 24H2+, Windows 11 Pro, Windows 10 22H2 (ESU) — fiziksel cihaz + VM; tek ve çift monitör; %100/%125/%150 DPI. Çocuk standart hesabı + ebeveyn yönetici hesabı.

**Minimal yapı (kod yazılmadan önce onay gerekir):** Rust servis (otomatik başlangıç, kurtarma eylemleri) + Rust helper (tam ekran pencere, LL kanca, ön plan izleme, named pipe) + yerel test komutu (`APPLY/RELEASE`). Tauri/Supabase PoC'ye dahil değil.

**Test senaryoları (her biri: sonuç, tespit süresi, tepki, kanıt ekran kaydı/log):**

| ID | Senaryo |
|---|---|
| A-01 | Kısıtlama sırasında Alt+Tab, Win+Tab, Win+D, Win+M, Win key |
| A-02 | Ctrl+Shift+Esc / Ctrl+Alt+Del → Görev Yöneticisi → helper sonlandırma |
| A-03 | Helper sonlandırma → servis yeniden başlatma süresi; tekrarlı sonlandırma tepkisi |
| A-04 | Win+L, kullanıcı değiştir, oturumu kapat/aç |
| A-05 | Yeniden başlatma ve hızlı başlangıç (fast startup) sonrası kısıtlama |
| A-06 | Uyku/hazırda bekletme sırasında ve sonrasında sayaç |
| A-07 | İkinci monitör takma/çıkarma, DPI değişikliği |
| A-08 | Tam ekran oyun / video oynatıcı üzerinde kısıtlama görünürlüğü |
| A-09 | Ağ kablosu çekme / Wi-Fi kapama sırasında kısıtlama |
| A-10 | Standart kullanıcının servisi durdurma girişimi (`sc stop`, services.msc) |
| A-11 | Ebeveyn kurtarma: yerel yönetici ile kısıtlamayı kaldırma ve servis kaldırma |
| A-12 | `LockWorkStation` helper'dan çağrı + `WTS_SESSION_LOCK` bildirimi ile doğrulama |

**PoC kabul kriterleri (öneri):** A-01/A-04/A-05/A-09/A-10'da kısıtlama sürmeli; A-02/A-03'te sonlandırma ≤ 5 sn içinde tespit edilip yeniden uygulanmalı ve kayıt üretmeli; A-11 her zaman çalışmalı (kurtarılamaz kilit = FAIL). Hedef değerler kullanıcıyla netleştirilecek.

## 7. GATE T0-A kontrol listesi

- [x] Kullanım kilidi mimarisi açıklandı (§1, §2).
- [x] Güvenlik sınırları belirlendi (§4).
- [x] Çocuk standart kullanıcı hesabı dikkate alındı (§5).
- [x] Gerçek Windows test planı hazırlandı (§6).
- [ ] Kullanıcı kabulü.

Hiçbir senaryo çalıştırılmadı (**NOT RUN**).

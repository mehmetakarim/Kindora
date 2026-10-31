# ADR-0015 — Bağımsız Session Helper, tek raporlayıcı ve oturum açılışında başlatma

- **Tarih:** 2026-10-30
- **Durum:** **Kabul edildi** (kullanıcı kararı K-8, 2026-10-31).
  - P1.D'de uygulandı; P1.D.1'de kurtarma CI'da gerçek zaman tetikleyicisiyle kanıtlandı.
  - **Lenovo Acceptance PASS** (D-01…D-10, 2026-10-31).
  - Açık riskler korunur: yönetici hesap, `conhost --headless`, en çok 5 dk kurtarma, [OBS-011](../solutions.md).
- **Karar türü:** Teknik uygulama yöntemi
- **İlgili belgeler:** [P1.D raporu](../background/p1d-background-helper.md), [ADR-0006](0006-agent-surec-ve-hesap-modeli.md), [ADR-0012](0012-mevcut-windows-hesabinda-calisma.md), [ADR-0013](0013-masaustu-kabugu-helper-koprusu.md), [ADR-0014](0014-yerel-kullanim-suresi-motoru.md)

## Bağlam ve problem

- P1.C'de etkinlik sinyalini yalnız masaüstü uygulamasının köprü helper'ı gönderiyordu.
  - Pencere kapanınca süre `UNKNOWN` sayılıyordu.
  - Lenovo'da P1.C temel kabulü PASS oldu; bu sınırlama bilinen risk olarak kaldı.
- P1.D'de ölçüm masaüstünden bağımsız olmalıdır. Sınırlar:
  - Yeni Windows hesabı yok; mevcut hesap kullanılır (ADR-0012).
  - Servis LocalSystem'e yükseltilmez; oturuma süreç enjekte edilmez; çocuk arayüzü yönetici olarak başlatılmaz.
  - Otomatik başlatma yalnız açık onayla kurulur ve geri alınabilir.
  - Aynı süre iki kez yazılmaz; bağlantı yokken etkinlik uydurulmaz.

## Değerlendirilen seçenekler

### Helper'ın başlatılması

1. **Servis `WTSQueryUserToken` + `CreateProcessAsUser` ile başlatır (ADR-0006 M2).**
   - Reddedildi: LocalSystem + `SE_TCB_NAME` gerektirir. Bu, servis hesabını yükseltmek demektir.
2. **`HKCU\...\Run` anahtarı.**
   - Artılar: yönetici gerekmez; kullanıcı başınadır; kolay kaldırılır.
   - Eksiler: Windows açılış gecikmesine tabidir. Çöken helper'ı yeniden başlatmaz. Konsol penceresini gizlemek için ek araç gerekir.
3. **Başlangıç (Startup) klasörü kısayolu.**
   - Run anahtarıyla aynı eksiklere sahiptir. Ayrıca kullanıcı dosyası olarak kolay bozulur.
4. **Seçilen — kullanıcı başına Zamanlanmış Görev:**
   - **Tetikleyici:** oturum açılışı + N dk'da bir yeniden denetim. Yeniden denetim ayrı bir **zaman tetikleyicisidir**: oturum açılışı tetikleyicisinin tekrarı yalnız gerçek bir oturum açılışından sonra başlar; kurulumdan sonra veya görev elle başlatıldığında devreye girmez [Çıkarım; Task Scheduler davranışı, P1.D CI tasarım incelemesi].
   - **Çalışma:** `LeastPrivilege` + `InteractiveToken`. Kullanıcının kendi oturumunda, yöneticisiz ve parolasız çalışır.
   - **Ayarlar:** `IgnoreNew`, pilde de çalışır, süre sınırı yok, düşük öncelik.
   - **Eylem:** `conhost.exe --headless kindora-helper.exe activity --launch logon-task`. Pencere açılmaz.
   - Kurulum yönetici ister (servis zaten ister). Helper'ın kendisi yöneticisiz çalışır.
5. **Masaüstü uygulaması arka plan helper'ını başlatır.**
   - Reddedildi: pencere açmak, onaysız kalıcı arka plan süreci başlatmış olurdu. Masaüstü yalnız durumu gösterir.

### Tek raporlayıcı

- **Yalnız helper tarafında kilit.** Yetersiz: eski sürüm, tek seferlik komut ve köprü kilidi aşar.
- **Seçilen:** agent tarafında bağlantı kimliğine bağlı **kiralama (lease)** + helper tarafında oturum başına tek örnek kilidi.

## Seçilen yaklaşım

### Roller

| Mod | Rol | Yaşam döngüsü |
|---|---|---|
| `kindora-helper activity` (`run` eski adı) | Arka plan raporlayıcısı (`BACKGROUND_LOGON_TASK` / `BACKGROUND_MANUAL`) | Oturum açılışı görevi veya elle başlatılır. Masaüstünden bağımsızdır. Oturum başına tek örnek. |
| `kindora-helper bridge` | Masaüstü köprüsü (`DESKTOP_BRIDGE`) | Masaüstünün alt sürecidir. Durum okur; yalnız arka plan helper'ı yokken **yedek** raporlayıcıdır. |
| `kindora-helper report-activity` | Tek seferlik (`ONE_SHOT`) | Tanılama ve CI içindir. |

### Tek örnek

- **Windows:** `Local\Kindora.SessionHelper.Activity` mutex'i (oturum başına). İkinci örnek `already_running` kaydedip 0 koduyla çıkar.
- **Unix (geliştirme):** `flock`.

### Agent kiralaması (`crates/windows-agent/src/screen_time.rs`)

- Rapor yalnız **etkin konsol oturumundan** kabul edilir. Bu kural P1.C'den gelir.
- **Kiralamanın kimliği:**
  - Agent'ın her bağlantıya verdiği benzersiz `connection_id` (PID değil).
  - İşletim sisteminin bildirdiği oturum ve PID.
  - Helper'ın beyan ettiği rol. Rol yalnız önceliktir; **yetki vermez**.
- **Karar sırası:**
  1. Kiralama yok → al.
  2. Aynı bağlantı → yenile.
  3. Kiralayan sustu (15 sn) veya başka oturuma ait → al.
  4. Yeni raporlayıcı daha öncelikli (arka plan > köprü/tek seferlik) → devral.
  5. Aksi halde → reddet. `applied=false` döner ve `competing_reports` artar.
- **Bırakma:** kiralayan bağlantı kapanınca hemen, susunca veya konsol oturumu değişince bırakılır. Konsol oturumu değişince eski oturumun son raporu da atılır.
- **Olaylar:** `REPORTER_ACQUIRED`, `REPORTER_TAKEOVER`, `REPORTER_RELEASED` (neden: `disconnected`, `stale`, `console_session_changed`). SQLite olay tablosuna yazılır; şema değişmez (v1).

### Neden iki kez sayılamaz

- Süre yalnız agent'ın 1 sn'lik kendi döngüsünde ve monoton saatle sayılır.
- Rapor bir **ölçüm noktasıdır** ("son girdi şu an"), süre aralığı değildir.
- Tekrarlanan, geciken veya iki helper'dan gelen raporlar en çok bu noktayı günceller; aralık eklemez.
- Helper bağlantı yokken rapor biriktirmez ve yeniden bağlanınca geçmiş göndermez.

### Kurtarma

- **Helper çöker veya sonlandırılır:**
  - Bağlantı kapanır; kiralama hemen boşalır.
  - Masaüstü açıksa köprü devralır.
  - Görevin yeniden denetimi en geç N dk (varsayılan 5) içinde helper'ı yeniden başlatır.
  - Yeni örnek önceki çıkışın temiz olmadığını `previous_exit state=unclean` ile kaydeder.
  - Sürekli yeniden başlatma döngüsü yoktur: en fazla N dk'da bir.
  - **P1.D.1 düzeltmesi (CI kanıtı, [NOTE-010](../solutions.md)):** Helper sonlandırılınca `conhost --headless` 0 koduyla çıkar; görev "başarıyla tamamlandı" sayılır (olay 201/102, `LastTaskResult 0x0`). Bu yüzden `RestartOnFailure` **hiç devreye girmez**. Tek kurtarma yolu zaman tetikleyicisidir. Ayar zararsız olduğu için pakette bırakıldı; kurtarma süresi bir sonraki `NextRunTime`'a kadar, yani en çok N dk'dır.
- **Agent yeniden başlar:** helper üstel geri çekilmeyle (en çok 10 sn) yeniden bağlanır.
- **Agent çöker:** servis kurtarma eylemleri 5 sn, 60 sn ve sonra 5 dk'da bir yeniden başlatır (`sc failure`, sıfırlama 1 gün).
- **Oturum kapanır:**
  - Helper gizli bir üst düzey pencereyle `WM_ENDSESSION` alır, bağlantıyı kapatır ve iz dosyasını siler.
  - `user32` kullanan konsol süreçlerine `CTRL_LOGOFF_EVENT` gelmez.
  - Agent tarafında süre zaten WTS durumuna göre sayılır.
- **Eski sürüm helper:**
  - Kurulum betiği güncellemeden önce kurulum klasöründen çalışan Kindora süreçlerini durdurur. Windows çalışan EXE'nin üzerine yazılmasına izin vermez.
  - Yine de eski bir örnek kalırsa yeni örnek tek örnek kilidinde çıkar. Eşit öncelikte ilk gelen raporlayıcı korunur; ölçüm kaynağı titremez.
- **Birden fazla Windows oturumu:**
  - Her oturumun kendi helper'ı olabilir; yalnız etkin konsol oturumunun raporu sayılır.
  - Süre cihaz düzeyindedir; kullanıcı başına ayrım yoktur.
- **Başlangıç kaydı eksik veya bozuk:**
  - Durum betiği ve Teknik tanılama raporlayıcı olmadığını gösterir: "Arka plan ölçümü kapalı".
  - Kurulum betiğini yeniden çalıştırmak kaydı onarır (idempotent).

### Agent başlatma politikası

- Geliştirme servisi varsayılan olarak `start= demand` kalır; Windows açılışında **çalışmaz**.
- Test kurulumu yalnız `-AgentStartMode Auto` ile `start= auto` yapar.
- Üretim için önerilen davranış:
  - Agent: `auto` + kurtarma eylemleri.
  - Helper: oturum açılışı görevi.
- **Bağımlılıklar:**
  - Helper agent'tan önce açılırsa geri çekilmeyle bekler. Bu süre `UNKNOWN` sayılır.
  - Agent helper'dan önce açılırsa `NO_HELPER` → `UNKNOWN` sayar.
  - İki bileşen birbirini başlatmaz.

### Kurulum ve kaldırma

- Betikler `packaging/p1d/*.ps1`. Windows PowerShell 5.1 ile çalışır; ASCII'dir.
- Kendini yükseltmez; yönetici değilse durur.
- Yalnız Kindora kayıtlarını değiştirir: servis, `\Kindora\` görev klasörü, `C:\Program Files\KindoraDev`.
- İdempotenttir.
- Kullanım süresi veritabanını **silmez**. Kaldırmada `-DeleteData` ve `SIL` onayı (CI'da `-Yes`) olmadan veri silinmez.

## Sonuçlar ve ödünleşimler

- **Artılar:**
  - Pencere kapalıyken ölçüm sürer.
  - Servis hesabı ve helper politikası değişmedi.
  - Yeni IPC komutu yok; rapor yalnız isteğe bağlı rol alanı aldı.
  - Tek raporlayıcı sunucu tarafında zorlanır.
  - Kurtarma Windows'un kendi mekanizmalarıyla yapılır; ayrı watchdog servisi yok.
- **Eksiler / riskler:**
  - Aynı kullanıcı görevi devre dışı bırakabilir veya helper'ı sonlandırabilir. Bu tespit edilir (`UNKNOWN`, `REPORTER_RELEASED`, tanılama), engellenmez (ADR-0012).
  - `conhost --headless` belgelenmiş bir arayüz değildir [Çıkarım: Windows 10 1809+ destekler; Lenovo'da doğrulanmalı].
  - Görev yeniden denetimi en çok N dk gecikmeyle kurtarır. Bu arada süre `UNKNOWN` sayılır (eksik sayım yönünde).
  - Kullanıcı başına süre yok: hızlı kullanıcı değiştirmede süre ekrandaki oturuma, yani cihaza yazılır.
  - Rol beyanı helper'dan gelir. Aynı EXE ve oturumdaki bir süreç kendini "arka plan" ilan edebilir; bu yalnız raporlayıcı seçimini etkiler, yetki vermez.

## Bilinmeyenler / doğrulama gereken noktalar

- Gerçek oturum açılışı tetikleyicisi, oturum kapatma, Windows yeniden başlatma ve hızlı kullanıcı değiştirme (Lenovo D-08, D-09, D-12). CI'da görev yalnız elle tetiklenebilir.
- ~~`RestartOnFailure`'ın helper çıkış koduna tepkisi~~ — P1.D.1'de yanıtlandı: sonlandırmada devreye girmez (NOTE-010).
- Lenovo'da uyku sırasında hangi güç olaylarının geldiği. P1.C C-08 bulgusu: uyku 0, ölçülemeyen arttı. P1.D'de bütün güç olayları agent kaydına yazılır.

## İlişkili kararlar

- [ADR-0006](0006-agent-surec-ve-hesap-modeli.md): M1 düşük yetkili servis korunur. Helper yeniden doğurma M2 yerine Zamanlanmış Görevle yapılır.
- [ADR-0013](0013-masaustu-kabugu-helper-koprusu.md): köprü korunur ve yedek raporlayıcı olur.
- [ADR-0014](0014-yerel-kullanim-suresi-motoru.md): ölçüm modeli değişmedi. "Yalnız masaüstü açıkken ölçüm" sınırlaması bu ADR ile kaldırıldı.

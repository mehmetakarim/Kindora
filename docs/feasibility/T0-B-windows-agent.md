# T0-B — Rust Windows Agent Mimarisi

> Durum: **Taslak / öneri** · Kapı: GATE T0-B · İlgili öneriler: [ADR-0006](../decisions/0006-agent-surec-ve-hesap-modeli.md), [ADR-0007](../decisions/0007-servis-helper-ipc-named-pipe.md)
> Kod yazılmadı; Windows'ta çalıştırılmadı.

## 1. Süreç topolojisi (öneri)

```text
Session 0                                  Çocuğun oturumu (Session N)
┌───────────────────────────────┐          ┌──────────────────────────────┐
│ kindora-agent.exe (Service)   │  pipe    │ kindora-helper.exe           │
│  ├─ Policy Engine             │◄────────►│  ├─ Restriction Surface      │
│  ├─ Command Processor         │ (session │  ├─ Input guard (LL hook)    │
│  ├─ Usage Monitor (oturum/    │  başına) │  ├─ LockWorkStation çağrısı  │
│  │   aktivite olayları)       │          │  └─ UI köprüsü               │
│  ├─ Sync Client (Supabase)    │          │        ▲ yerel IPC (dar)     │
│  ├─ Persistence (SQLite)      │          │ ┌──────┴───────────────────┐ │
│  └─ Health & Recovery         │          │ │ Tauri çocuk UI + widget  │ │
└───────────────────────────────┘          │ └──────────────────────────┘ │
                                           └──────────────────────────────┘
```

Tauri UI servisle doğrudan konuşmaz; helper üzerinden yalnızca izinli mesajları iletir. UI kapanırsa denetim sürer.

## 2. Bileşen sorumlulukları

| Bileşen | Sorumluluk | Sahip süreç |
|---|---|---|
| Windows Service | Açılışta başlama, SCM yaşam döngüsü, diğer bileşenleri barındırma | agent |
| User Session Helper | Oturuma bağlı işler: kısıtlama yüzeyi, klavye koruması, `LockWorkStation`, UI köprüsü | helper |
| Local Policy Engine | Politika + sayaçlardan karar üretme (izin ver / uyar / kısıtla); saf, test edilebilir modül | agent |
| Command Processor | Uzak komutları doğrulama (cihaz, TTL, policy_version, idempotency), uygulama, receipt | agent |
| Usage Monitor | Aktif oturum süresi, boşta kalma, kilit/uyku olayları; **içerik/ekran kaydı yok** | agent (+ helper'dan aktivite sinyali) |
| IPC Layer | Servis↔helper named pipe; şema doğrulamalı, sürümlü mesajlar | agent + helper |
| Persistence | SQLite (WAL), migration, ACL'li dosya | agent |
| Health & Recovery | Helper watchdog, servis kurtarma eylemleri, bozuk DB kurtarma, kurcalama olayları | agent |

Önerilen crate ayrımı (context §15 ile uyumlu): `policy-engine` (platformdan bağımsız), `windows-agent`, `windows-session` (helper), `shared-contracts`, ileride `network-filter`.

## 3. Servis hesabı — en az yetki değerlendirmesi

| Seçenek | Gerekçe / sonuç | Kanıt |
|---|---|---|
| LocalService / NetworkService / sanal hesap | Düşük yetki; ancak `WTSQueryUserToken` çağrılamaz → helper'ı oturuma başlatamaz/yeniden doğuramaz. Program Files / HKLM yazma ve ileride ağ filtresi için ek ACL gerekir. | [Resmi kaynak](https://learn.microsoft.com/en-us/windows/win32/api/wtsapi32/nf-wtsapi32-wtsqueryusertoken), [hesaplar](https://learn.microsoft.com/en-us/windows/win32/services/service-user-accounts) |
| LocalSystem | Helper'ı servisin kendisi başlatma/yeniden doğurma (M2) mümkün; ağ filtreleme (hosts/WFP) için gerekli olabilir [PoC gerekli]. Saldırı yüzeyi büyük. | [Resmi kaynak](https://learn.microsoft.com/en-us/windows/win32/api/wtsapi32/nf-wtsapi32-wtsqueryusertoken) |
| **Bölünmüş model (öneri)** | Ana agent düşük yetkili hesap + küçük, tek işlevli "launcher/guard" bileşeni LocalSystem; ya da tek LocalSystem servis + sıkı iç sınırlar. Seçim PoC-B sonrası ADR-0006 ile. | [Çıkarım] |

**LocalSystem bütün agent için zorunlu kabul edilmedi.** `WTSQueryUserToken` gereksinimi yalnız "servis helper'ı kullanıcı token'ıyla kendisi başlatır/yeniden doğurur" (M2) yöntemine özgüdür; helper HKLM Run veya oturum açma tetikli zamanlanmış görevle başlatılırsa bu API gerekmez. Ağ filtreleme (P9) gibi başka ayrıcalık gereksinimleri ayrıca değerlendirilir. Hangi bileşenin hangi hesapla çalışacağı PoC-A/B ölçümlerinden sonra ADR-0006 ile kullanıcı kararıdır.

Sertleştirme: servis SID'i (`SERVICE_SID_TYPE_RESTRICTED` uygunsa), `SeTcbPrivilege` dışındaki gereksiz ayrıcalıkların kaldırılması (`SERVICE_CONFIG_REQUIRED_PRIVILEGES_INFO`), keyfi komut çalıştırma yok, imzalı ikililer.

## 4. Senaryolar

| Senaryo | Yaklaşım | Etiket |
|---|---|---|
| Açılışta başlama | `SERVICE_AUTO_START` (gerekirse delayed değil — kısıtlama logon'dan önce hazır olmalı) | [PoC gerekli] |
| Logon'da helper | Servis `WTS_SESSION_LOGON` bildirimiyle çocuk oturumunda helper başlatır; yedek: HKLM Run | [Resmi kaynak — öneri](https://learn.microsoft.com/en-us/windows/win32/services/interactive-services) |
| Güvenli IPC | Bkz. §5 | [Resmi kaynak + PoC] |
| Standart kullanıcı servisi durduramaz | Varsayılan servis DACL'i yerel kullanıcılara `SERVICE_STOP` vermez; yalnızca Administrators/LocalSystem | [Resmi kaynak](https://learn.microsoft.com/en-us/windows/win32/services/service-security-and-access-rights) — A-10 ile doğrulanacak |
| Komut tekrarı | `command_id` SQLite'ta benzersiz; uygulanmışsa aynı receipt yeniden gönderilir | [Çıkarım] |
| Komut süre aşımı | `expires_at` geçmişse uygulanmaz, `expired` receipt | [Çıkarım] |
| Yeniden başlatma | Politika/sayaç SQLite'tan; helper hazır olana kadar oturum "kısıtlama gerekiyorsa" oturum kilidi yedeği | [PoC gerekli] |
| Çok kullanıcılı oturum | Politika Windows kullanıcı SID ↔ child_profile eşlemesine bağlı; ebeveyn/diğer hesaplar kısıtlanmaz; oturum başına pipe | [Çıkarım] |
| Çökme sonrası toparlanma | SCM kurtarma eylemleri (yeniden başlat ×3, sıfırlama süresi); helper watchdog; açılışta tutarlılık kontrolü | [PoC gerekli] |
| Kontrollü kaldırma | Yalnızca yönetici + ebeveyn onayı (yerel PIN / çevrimiçi onay); kaldırma `audit_events`'e yazılır; kaldırma sonrası cihaz bulutta "kaldırıldı" | [Çıkarım] |
| Kurtarma | Ebeveyn yönetici hesabında yerel kurtarma aracı: kısıtlamayı kaldır, servisi durdur, logları dışa aktar. Kurtarılamaz kilit tasarlanmaz. | [Çıkarım] |

## 5. IPC güvenlik modeli (öneri — ADR-0007)

- **Taşıma:** Named pipe, oturum başına ad: `\\.\pipe\kindora\session-<id>` + rastgele sonek.
- **DACL:** Varsayılan pipe DACL'i Everyone'a okuma verir → **kullanılmaz.** Açık DACL: servis SID tam yetki + ilgili oturumun **logon SID**'i yalnızca bağlanma/okuma/yazma ([Resmi kaynak](https://learn.microsoft.com/en-us/windows/win32/ipc/named-pipe-security-and-access-rights)). `FILE_GENERIC_WRITE` yerine bireysel haklar (aynı kaynak: `FILE_CREATE_PIPE_INSTANCE` örtüşmesi).
- `PIPE_REJECT_REMOTE_CLIENTS`; `FILE_FLAG_FIRST_PIPE_INSTANCE` (pipe gaspına karşı).
- **İstemci doğrulama:** `GetNamedPipeClientProcessId` + süreç yolu/imza kontrolü; oturum kimliği eşleşmesi. [PoC gerekli]
- **Mesajlar:** sürümlü, uzunluk sınırlı, JSON Schema doğrulamalı; helper'dan servise yalnızca: `activity_ping`, `request_extra_time`, `request_site_access`, `ui_state`, `restriction_ack`. Helper'ın politika değiştiren hiçbir mesajı yok.
- Servisten helper'a: `apply_restriction`, `release_restriction`, `warn`, `lock_windows_session`, `policy_snapshot(view)`.

## 6. Windows destek matrisi (öneri)

| Sürüm | Öneri | Gerekçe |
|---|---|---|
| Windows 11 Home/Pro 23H2+ (x64) | **Birincil hedef** | Güncel; WebView2 yerleşik ([Tauri](https://v2.tauri.app/reference/webview-versions/)) |
| Windows 10 22H2 (x64) | İkincil / "en iyi çaba" | Destek 14 Eki 2025'te bitti; tüketici ESU 12 Eki 2027'ye kadar ([Microsoft](https://learn.microsoft.com/en-us/windows/whats-new/extended-security-updates)) |
| Windows 11 ARM64 | Değerlendirme | Rust/Tauri derlenebilir; LL kanca/WebView2 davranışı [PoC gerekli] |
| Windows 7/8.1 | Desteklenmez | Destek dışı |

Kurulum gereksinimleri: yönetici hakkı, WebView2 (Tauri installer sağlar), çocuk için standart hesap, ebeveyn için yönetici hesap, kod imzalama sertifikası (SmartScreen/AV için; maliyet açık karar).

## 7. GATE T0-B kontrol listesi

- [x] Servis/helper ayrımı net (§1–2).
- [x] IPC güvenlik modeli tanımlı (§5).
- [x] Yaşam döngüsü ve kurtarma planı (§4).
- [ ] Servis hesabı modeli kararı (ADR-0006 — kullanıcı onayı bekliyor).
- [ ] Kullanıcı kabulü.

# P1.C — Yerel kullanım süresi motoru ve Filiz mola deneyimi

> **Durum:** Kodlandı; macOS'ta ve GitHub Actions `windows-2022`'de test edildi. **Lenovo kullanım süresi kabulü bekliyor** (C-01…C-11).
> Karar: [ADR-0014](../decisions/0014-yerel-kullanim-suresi-motoru.md). Görev: [TASK-008](../tasks.md).

## 1. Mimari

```text
kindora-desktop (Tauri, kullanıcı)          kindora-helper bridge (kullanıcı oturumu)          kindora-agent (LocalService, Session 0)
 Ana görünüm / Teknik tanılama  ◄─stdio JSON─  GetLastInputInfo → idle_ms         ──named pipe──►  ScreenTimeRuntime (1 sn döngü)
 yalnız gösterir; kural yazmaz                 REPORT_ACTIVITY + GET_SCREEN_TIME                  ├ WTS kilit durumu (etkin konsol)
                                               (+ GET_STATUS, HEALTH_CHECK)                        ├ POWER_EVENT (uyku/uyanma)
                                                                                                   ├ DEV politika dosyası (5 sn)
                                                                                                   ├ screen-time Engine
                                                                                                   └ SQLite (LocalService profili)
```

- Ölçüm, kurallar ve kayıt **agent**'tadır. Masaüstü kapansa da agent çalışır. Helper yoksa süre `UNKNOWN` kaydedilir.
- `screen-time` crate'i işletim sisteminden bağımsızdır: Windows sinyalleri `windows-agent/src/platform.rs` içinde toplanır.
- Agent'ın helper politikası (oturum ≠ 0, exe adı ve klasörü, BUG-003 ACE) **değişmedi**.

## 2. Aktif kullanımın ölçümü

Tek kullanıcı sinyali, helper'ın kendi oturumunda `GetLastInputInfo` + `GetTickCount` ile hesapladığı `idle_ms` değeridir (sarmalı 32 bit aritmetik). Tuş, koordinat, pencere, uygulama, ekran, belge, tarayıcı, mesaj, kamera/mikrofon verisi okunmaz.

Agent her saniye bağlamı şu sırayla kurar:

1. Güç olayı uyku bildirdi → `SUSPENDED`.
2. Etkin konsol oturumu yok / kilit durumu okunamadı → `UNKNOWN` (`NO_CONSOLE_SESSION`, `LOCK_STATE_*`).
3. Oturum kilitli veya etkin değil (`WTSSessionInfoEx`: `SessionFlags`, `SessionState`) → `SESSION_LOCKED`.
4. Etkin konsol oturumundan ≤15 sn içinde rapor yok → `UNKNOWN` (`NO_HELPER` / `STALE_REPORT`); `idle_ms = null` → `UNKNOWN` (`IDLE_API_FAILED`).
5. Aksi hâlde gözlem: son girdi zamanı = rapor anı − `idle_ms` (monoton).

Gözlemde sınıflandırma ([accounting.rs](../../crates/screen-time/src/accounting.rs)):

- İki gözlenen girdi arası < boşta kalma eşiği → `ACTIVE`; değilse `IDLE`.
- Son girdiden sonra eşik dolarsa son girdiden itibaren tüm aralık **geriye dönük** `IDLE` olur (eşik süresi aktif sayılmaz).
- Başlangıç, kilit açılışı veya uyanmadan ilk girdiye kadarki süre ve engelleyen duruma geçişteki doğrulanmamış kuyruk `IDLE` sayılır.
- İki ölçüm arası > 5 sn → aralık `SUSPENDED` (güç olayı varsa) veya `UNKNOWN`.

## 3. Boşta / kilit / uyku / saat / gün

| Durum | Hesap |
|---|---|
| Boşta | Eşik dolunca son girdiden itibaren `IDLE`; molayı yaklaştırmaz. |
| Kilit (Win+L, kullanıcı değişimi) | WTS sorgusu her saniye; süre `SESSION_LOCKED`. |
| Uyku | Servis `POWER_EVENT` → `SUSPENDED`; olay kaçarsa ölçüm boşluğu `UNKNOWN`. Uyandıktan sonra eski girdi zamanı yeniden sayılmaz. |
| Servis yeniden başlatma | Kayıtlar SQLite'tan yüklenir; kapalı kalınan süre hiçbir duruma yazılmaz; son ≤15 sn kaybolabilir (eksik sayım). Mola geri sayımı duvar saatine göre en fazla kalan süre kadar ilerletilir. |
| Saat ileri/geri | Süreler monoton; duvar saati yalnız günü belirler. ±60 sn'yi aşan sapma `CLOCK_JUMP`; saat dilimi `TIMEZONE_CHANGED`; gün `clock_adjusted`. 1 gün geri: mevcut gün korunur. Daha fazlası: `DAY_CORRECTED`. |
| Gün değişimi | Değişim anından önceki parçalar önceki güne, sonrakiler yeni güne yazılır (1 sn hassasiyet). |
| Ölçüm hatası | `UNKNOWN` + neden; asla `ACTIVE` değil. |

## 4. SQLite şeması ve kurtarma

Dosya: `<servis hesabı LocalAppData>\Kindora\kindora.db`. LocalService için `C:\Windows\ServiceProfiles\LocalService\AppData\Local\Kindora`.

- Klasör `D:P(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)(A;OICI;FA;;;OW)` ile oluşturulur. Users/Interactive erişimi yoktur.
- Unix geliştirme: 0700 klasör, 0600 dosya.

Şema v1 ([store.rs](../../crates/screen-time/src/store.rs)):

| Tablo | İçerik |
|---|---|
| `daily_usage` | gün, `active/idle/locked/suspended/unknown_ms`, `clock_adjusted` |
| `usage_segments` | çalıştırma kimliği, gün, durum, başlangıç/bitiş (duvar), süre — ardışık aynı durum birleşir |
| `policy_state` | son geçerli politika (sürüm, kaynak, kanonik JSON) |
| `break_state` | mola aşaması, molaya kadar aktif süre, geri sayım, sıra no |
| `lifecycle_events` | `AGENT_START/STOP`, `USAGE_STATE`, `CLOCK_*`, `DAY_*`, `POLICY_*`, `BREAK_*`, `LIMIT_REACHED`, `DB_*` |

- **Migration:** `PRAGMA user_version`. Agent'tan yeni şema → dosyaya dokunulmaz, bellek içi.
- **Kayıt:** tek transaction, 15 sn'de bir veya olay/gün değişiminde; her parça bir kez eklenir.
- **Kurtarma:** `NOTADB`/`CORRUPT`/`quick_check ≠ ok` → `kindora.db.corrupt-<ms>` (en çok 3 kopya) + yeni veritabanı + `DB_RECOVERED`. Açılamazsa bellek içi çalışılır ve sağlık denetimi `local_storage` başarısız döner.
- **Saklama:** parçalar 14 gün, günlük özet 90 gün, olaylar son 5000 satır.
- Veriler buluta gönderilmez.

## 5. Politika motoru

| Alan | Test varsayılanı | DEV sınırları |
|---|---|---|
| `daily_limit_secs` | 7200 | 60–86400 |
| `break_interval_secs` | 1800 | 60–14400 |
| `break_duration_secs` | 300 | 30–3600 |
| `first_warning_secs` | 300 | 20–1800 |
| `final_warning_secs` | 60 | 10–600 |
| `idle_threshold_secs` | 300 | 30–3600 |

Ek kural: `final < first < break_interval`.

- **DEV dosyası:** `kindora-dev-policy.json`, agent ile aynı klasörde (Program Files → yalnız yönetici yazar). `kind: "kindora-dev-policy"`, `dev_only: true`, `version ≥ 1` zorunlu; bilinmeyen alan reddedilir.
- **Çözüm:** dosya yok → varsayılan; geçerli ve daha yeni sürüm → uygula + sakla; bozuk → son geçerli (`INVALID_KEPT_LAST`) veya varsayılan; eski/eşit sürümde farklı içerik → `VERSION_REJECTED`.
- **Limit dolunca:** yalnız `LIMIT_REACHED` bildirimi; ölçüm sürer, kilit yok.
- **Üretim gereksinimi:** kural değişikliği ebeveyn kimliğiyle yetkilendirilmiş sunucu kararından gelir; DEV dosyası yalnız testtir.

## 6. Filiz mola akışı

Aşamalar: `NORMAL → FIRST_WARNING → FINAL_WARNING → BREAK_DUE (geri sayım) → BREAK_COMPLETE (60 sn) → NORMAL`. Mola süresi kadar kesintisiz boşta/kilit/uyku doğal mola sayılır.

| Olay | Filiz | Poz (Phase 1 `mascot()`) |
|---|---|---|
| Normal | "Bugün güzel ilerliyorsun!" | happy / idle |
| İlk uyarı | "Birazdan kısa bir mola veriyoruz." | reminder |
| Son uyarı | "Çalışmanı kaydetmek için iyi bir zaman." | reminder |
| Mola zamanı | "Gözlerini dinlendirmeye ne dersin?" | break |
| Mola bitti | "Hazırsan devam edebiliriz!" | happy |
| Süre doldu | "Bugünkü planlanan süren doldu. Dilersen biraz dinlenebilirsin." | encourage |
| Ölçülemiyor | "Şu an kullanım süreni ölçemiyorum; ölçemediğim süre sayılmaz." | thinking |

- Metinler [src/i18n/tr.ts](../../apps/desktop/src/i18n/tr.ts) içindedir.
- Tek bildirim yuvası vardır. Her bildirim kapatılabilir; kapatılan kimlik tekrar gösterilmez.
- Sessiz mod iltifatı susturur, uyarıları susturmaz. Ses yoktur. "Hareketi azalt" ve işletim sistemi tercihi desteklenir.
- Agent bağlantısı yoksa kullanım sayıları **gösterilmez**.
- Ana görünüm: Filiz + ayarlar, bildirim, Bugünkü kullanım (halka, kullanılan/limit/kalan, durum, kurallar, son ölçüm), Sıradaki mola.
- Teknik tanılama: P1.B agent kartı, ölçüm/politika/depolama kartı, "Bu sürüm hakkında".

## 7. IPC komutları ve güvenlik

| Komut | İstemci | Veri | Boyut | Hata | Yetki | Test |
|---|---|---|---|---|---|---|
| `REPORT_ACTIVITY` | yalnız `kindora-helper.exe` (mevcut politika); sayım yalnız etkin konsol oturumu | `activity: {idle_ms: u64 ≤ 31 gün \| null}` (zorunlu, başka komutta yasak, ek alan yasak) | çerçeve ≤ 4 KB | `INVALID_MESSAGE`, `UNAVAILABLE`; bağlantı başına 250 ms kısma → `applied:false` | OS eş kimliği; mesajdaki kimlik iddiası yok sayılmaz, reddedilir | message.rs, handler.rs, screen_time.rs |
| `GET_SCREEN_TIME` | aynı | yanıt `SCREEN_TIME` (< 2 KB, test) | ≤ 4 KB | `UNAVAILABLE` (motor yok) | salt okunur | scenarios.rs, screen_time_e2e.rs, CI |

- Köprü (ADR-0013) değişmedi. Desktop → helper yalnız `refresh`. Snapshot'a `screen_time` eklendi (satır < 4096 bayt, test).
- `LOCK`/`UNLOCK`/`PROCESS_KILL`/kural yazma yoktur (`UNKNOWN_COMMAND`).
- Tauri yetenekleri değişmedi (iki komut + olay dinleme).

## 8. Testler ve sonuçlar

- **macOS:** kök workspace 82 test, masaüstü Rust 14, Vitest 39; ESLint, Prettier, vue-tsc, build, clippy.
  - Windows taklitli senaryolar (`scenarios.rs`) gerçek Windows kabulü değildir.
  - Gerçek Tauri penceresi macOS'ta DEV politikalı agent'a bağlandı (günlük: `agent_link state=Connected`); pencere kapanınca helper çıktı. Görsel ekran görüntüsü alınamadı (geliştirme ikilisi paket değil) — **NOT RUN**.
- **Windows CI:** §9'a bakın.
- **CI'ın doğrulayamadıkları:** gerçek kullanıcı girdisi/boşta kalma, Win+L, uyku/uyanma, hızlı kullanıcı değiştirme, %125/%150 ölçek. Runner yönetici hesaptır ve konsol oturumu Lenovo'dan farklıdır.

## 9. Windows CI sonuçları

Commit `584f4d1`, GitHub-hosted `windows-2022` (Windows Server 2022, `runneradmin`, oturum 2) — üç iş akışı da **PASS**:
[windows-p1c 40926272301](https://github.com/mehmetakarim/Kindora/actions/runs/40926272301) · [windows-p1b 40926272192](https://github.com/mehmetakarim/Kindora/actions/runs/40926272192) · [windows-p1a 40926272444](https://github.com/mehmetakarim/Kindora/actions/runs/40926272444).

- **Testler:** kök Rust 77 (Windows'ta Unix'e özgü testler yok; named pipe, süreç e2e, köprü ve kullanım süresi e2e dahil), masaüstü Rust 9, Vitest 39. fmt, clippy `-D warnings`, typecheck, lint, Tauri EXE + NSIS paketi.
- **P1.A/P1.B regresyonu:**
  - ACE'siz istemci `OpenProcess=5` ile reddedildi; helper yetkilendirildi; farklı klasördeki kopya `ExeDirMismatch` ile reddedildi.
  - Köprü refresh 556 ms.
  - GUI: servis durdurma 1456 ms'de algılandı, başlatma sonrası 216 ms'de kurtarıldı; pencere kapatılınca 973 ms'de çıkış; helper sızıntısı yok; servis çalışmaya devam etti.
- **P1.C, gerçek LocalService servisi (`-ScreenTime`):**
  - `GET_SCREEN_TIME` yanıtlandı; DEV politika v1 agent tarafında doğrulanıp uygulandı.
  - SQLite LocalService profilinde; klasör SDDL `O:LSG:LSD:P(A;OICI;FA;;;OW)(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)` (Users/Interactive yok).
  - **Gerçek API gözlemi:** `GetLastInputInfo` öncesi `idle_ms=810500`; `mouse_event` ile enjekte edilen girdiden sonra `idle_ms=328`. Helper raporu `applied=true` (etkin konsol oturumu = helper oturumu). Agent durumu `ACTIVE`, kaynak `SESSION_HELPER`.
  - Servis yeniden başlatma: toplam 21 sn → 21 sn; iki kez sayım yok; `AGENT_STOP` kaydedildi.
  - Bozuk SQLite kenara alındı ve yeniden oluşturuldu (`recovered=true`, `DB_RECOVERED`); servis çalışmaya devam etti.
  - Bozuk politika dosyası: `INVALID_KEPT_LAST`; v1 korundu; kilit yok.
- **CI sınırları:** Win+L, uyku/uyanma, gerçek kullanıcı etkinliği, yönetici olmayan hesap ve ölçek testleri yapılamaz. Enjekte girdi gerçek kullanıcı değildir. Bunlar Lenovo C-01…C-11'dedir.

## 10. Prerelease

[`p1c-windows-test.1`](https://github.com/mehmetakarim/Kindora/releases/tag/p1c-windows-test.1) — prerelease, `latest` değil; hedef `584f4d1`; [çalıştırma 40928512931](https://github.com/mehmetakarim/Kindora/actions/runs/40928512931). macOS'ta indirilip SHA-256 doğrulandı.

```text
91c8e88035e331b21bcfbcde863c26f2a0847411a87c8662a16b531ac950868a  Kindora-P1C-Windows-x64.zip
99fc212197c41cf27fffa2dea600031c6c6b88cc5f2c9e188e35f09b140d6812  kindora-agent.exe
df7d24dcf640bf3b28a060684a3f3a9b08ce04876ecc81e19611cf1df75b91b7  kindora-desktop.exe
7c31bfe3b65b97ec85b06e47cb7d9a76353ba9fb49b0484f324c2857174e2b03  kindora-helper.exe
34df037a9e8b2ccbb9d5731cf7cdf69b7552386e5843979c982dc03f284412f3  README-TEST-TR.md
61b865edfa6d2de153b245a888e1afa3c542fad637de2ba5c5056ad0dab7c406  BUILD-INFO.txt
```

Lenovo testi: [packaging/p1c/README-TEST-TR.md](../../packaging/p1c/README-TEST-TR.md) (C-01…C-11).

## 11. Açık riskler ve P1.D bağımlılıkları

- Helper yalnız masaüstü açıkken ölçer; kapalıyken süre `UNKNOWN` (eksik sayım). Oturum açılışında helper başlatma ve kurcalama tespiti (ADR-0006 M1/M2) — P1.D.
- Kurallar yalnız DEV dosyasından; ebeveyn kimlik doğrulaması, eşleştirme ve sunucu yetkilendirmesi yok (ADR-0002/0008/0009).
- Saat ileri alma / saat dilimi ile gün erken başlayabilir (kaydedilir, engellenmez); güvenilir zaman kaynağı yok.
- Mola ve limit yalnız bilgilendirir; Kindora kullanım kilidi (ADR-0001) PoC'si yapılmadı.
- Bildirimler yalnız açık pencerede; widget, always-on-top, DPI ve çoklu monitör kabulü ayrı.
- Bozuk DB kurtarmasında günün sayacı sıfırlanır; yeni şema görülürse bellek içi çalışılır.
- Veri saklama süreleri (14/90 gün) ürün/hukuk onayı bekliyor.

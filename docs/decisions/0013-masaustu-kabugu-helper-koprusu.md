# ADR-0013 — Masaüstü kabuğu agent'a Session Helper köprüsüyle bağlanır

- **Tarih:** 2026-10-28
- **Durum:** Önerildi (P1.B'de uygulandı; Lenovo GUI kabulü bekliyor)
- **Karar türü:** Teknik uygulama yöntemi
- **İlgili belgeler:** [P1.B raporu](../desktop/p1b-desktop-shell.md), [ADR-0007](0007-servis-helper-ipc-named-pipe.md), [ADR-0011](0011-p1a-crate-yapisi-ve-gelistirme-tasimasi.md), [ADR-0012](0012-mevcut-windows-hesabinda-calisma.md), [BUG-003](../solutions.md)

## Bağlam ve problem

P1.B'de ilk Kindora masaüstü uygulaması (Tauri 2 + Vue 3) agent'ın gerçek durumunu göstermelidir. Agent'ın pipe politikası yalnız şu koşulları sağlayan istemciyi kabul eder:
- istemci etkileşimli oturumdadır;
- exe adı `kindora-helper.exe`'dir;
- exe agent ile aynı klasördedir.

`kindora-desktop.exe` bu politikayı sağlamaz. Kullanıcı talimatı açıktır: politika gevşetilmez, desktop pipe'a doğrudan bağlanarak kontrolü aşmaz.

## Değerlendirilen seçenekler

1. **Politikaya `kindora-desktop.exe` eklemek.** Yasak (talimat). Ayrıca WebView içeren büyük bir süreci agent'ın güven sınırına alır.
2. **Desktop'ın helper kodunu kütüphane olarak gömüp pipe'a bağlanması.** Exe adı denetimine takılır; geçmesi için politikanın gevşetilmesi gerekir. Reddedildi.
3. **Helper'ın desktop için ikinci bir adlandırılmış pipe / yerel soket açması.** Yeni bir dinleyen uç ve yeni bir erişim denetimi gerektirir; saldırı yüzeyi büyür. Reddedildi.
4. **Seçilen: helper'ın `bridge` modu.** Desktop, yanındaki `kindora-helper.exe`'yi alt süreç olarak başlatır ve onunla yalnız anonim stdio boruları üzerinden konuşur.

## Seçilen yaklaşım ve gerekçe

- **Taşıma:** helper'ın stdin/stdout'u, satır başına bir JSON (en fazla 4096 bayt). Sözleşme `ipc_protocol::bridge` içindedir. Ağ ucu, adlandırılmış pipe veya dosya kullanılmaz.
- **Desktop → helper:** yalnız `{"op":"refresh"}` kabul edilir. Ayrıştırma katıdır: ek alan, bilinmeyen işlem veya agent komut adı reddedilir. İstek 500 ms / 1 sn ile kısılır.
- **Helper → agent:** yalnız salt okunur `GET_STATUS` + `HEALTH_CHECK` gönderilir (3 sn'de bir veya `refresh` ile). IPC protokolü, komut listesi ve politika **değişmedi**.
- **Helper → desktop:** `hello`, `link` (connecting / connected / disconnected / unauthorized + deneme sayısı ve bekleme süresi), `snapshot` (gerçek agent yanıtı), `request_rejected` olayları.
- **Yaşam döngüsü:**
  - Helper bağlantı kaybında üstel geri çekilmeyle yeniden dener (0,25 → 5 sn); yetkisiz yanıtta 30 sn bekler.
  - Desktop, helper çökünce onu 1 → 30 sn geri çekilmeyle yeniden başlatır.
  - Kapanışta desktop stdin'i kapatır ve helper temiz çıkar; 2 sn içinde çıkmazsa sonlandırılır.
  - Desktop çökerse işletim sistemi boruları kapatır ve helper EOF ile çıkar; sahipsiz süreç kalmaz.
- **Konum:** desktop, helper'ı yalnız kendi klasöründen başlatır. Yol ön yüzden veya yapılandırmadan seçilmez; geliştirme derlemesinde `KINDORA_HELPER_PATH` istisnası vardır. Agent da helper'ın kendi klasöründe olmasını şart koştuğu için **üç EXE aynı klasörde** dağıtılır.

## Tehdit modeli ve güven sınırları

| Sınır | Güven | Denetim |
|---|---|---|
| Agent (LocalService) ↔ helper | Agent, helper'ı OS kimliğiyle doğrular | Değişmedi: oturum ≠ 0, exe adı ve klasörü, `OpenProcess` için helper'ın verdiği dar izin (BUG-003) |
| Helper ↔ desktop (stdio) | Aynı kullanıcı, ebeveyn–alt süreç | Yalnız `refresh`; katı ayrıştırma; satır boyutu sınırı; çıktı yalnız durum |
| Desktop WebView ↔ Rust | Ön yüz güvenilmez kabul edilir | Yalnız 2 uygulama komutu (`get_agent_state`, `refresh_agent`) ve olay dinleme izni; shell/fs/http eklentisi yok; sıkı CSP |

- **Aynı kullanıcı saldırganı:** Bu kişi helper'ı veya desktop'ı zaten sonlandırabilir, çalıştırabilir ya da değiştirebilir. Köprü ona agent üzerinde ek bir yetki vermez: agent'a giden komutlar bugün de yalnız salt okunurdur. İleride durum değiştiren komutlar gelirse yetki **agent tarafında**, ebeveyn kimliğiyle verilmelidir; helper veya desktop yetkili karar veremez.
- **Sahte helper:** Desktop'ın klasörüne yazabilen biri sahte bir `kindora-helper.exe` koyabilir. Bu yalnız desktop'ın gösterdiği durumu yanıltır; agent sahte helper'ı yine OS kimliğiyle denetler. Program Files kurulumunda klasöre yalnız yönetici yazabilir.
- **Yanıltıcı durum:** Desktop yalnız helper'dan gelen gerçek olayları gösterir. Bağlantı yokken agent alanları "Bağlantı yok" yazar; demo kaynağı yalnız Tauri dışında ve "Demo" bandıyla kullanılır.
- **Yönetici hesap:** Kindora bir yöneticiye karşı değiştirilemez kilit sağlayamaz (ADR-0012). Bu bir güvenlik sınırıdır ve arayüzde de açıkça yazar.

## Kapanış politikası

- Pencereyi kapatmak **yalnız** desktop'ı ve onun helper alt sürecini kapatır.
- Agent Windows servisi SCM tarafından yönetilir ve çalışmaya devam eder. Desktop'ın servisi durdurma yetkisi ve komutu yoktur.

## Sonuçlar ve ödünleşimler

- **Artılar:**
  - Politika gevşemeden gerçek veri gösterilir.
  - Yeni dinleyen uç yoktur.
  - Helper, agent ile tek güvenilir istemci olarak kalır.
- **Eksiler:**
  - Desktop başına bir helper süreci daha çalışır (`kindora-helper run` ile aynı anda en fazla iki).
  - Agent durumu en geç yoklama aralığında (3 sn) güncellenir.
- **Kurucu:** NSIS kurucusu yalnız CI'da paketlemeyi doğrulamak için üretilir ve dağıtılmaz. Gerçek kurucunun servis kaydı ile helper/agent yerleşimini kapsaması, imzalama kararıyla birlikte ayrıca ele alınacak.

## Bilinmeyenler / doğrulama gereken noktalar

- Lenovo cihazında GUI kabulü (B-01…B-11), %125/%150 ölçek ve WebView2 davranışı.
- Helper'ın servis tarafından başlatılması (ADR-0006, M1/M2) seçilirse bridge modunun yerini alıp almayacağı.

## İlişkili kararlar

[ADR-0005](0005-bootstrap-6-hedef.md) (P1.B'de Bootstrap 5.3.8), [ADR-0006](0006-agent-surec-ve-hesap-modeli.md), [ADR-0007](0007-servis-helper-ipc-named-pipe.md), [ADR-0011](0011-p1a-crate-yapisi-ve-gelistirme-tasimasi.md), [ADR-0012](0012-mevcut-windows-hesabinda-calisma.md).

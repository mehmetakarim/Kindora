# ADR-0006 — Windows Agent süreç ve servis hesabı modeli

- **Tarih:** 2026-10-28
- **Durum:** Önerildi
- **Karar türü:** Teknik uygulama yöntemi
- **İlgili T0 belgesi:** [T0-A](../feasibility/T0-A-kindora-lock.md) §2, [T0-B](../feasibility/T0-B-windows-agent.md) §1–§4

## Problem ve bağlam

Servisler Session 0'da çalışır ve kullanıcıyla doğrudan etkileşemez; kısıtlama yüzeyi çocuğun oturumunda ayrı bir helper süreci gerektirir ([Microsoft](https://learn.microsoft.com/en-us/windows/win32/services/interactive-services)). Çocuğun kimliğiyle çalışan helper, aynı kullanıcı tarafından sonlandırılabilir [Çıkarım — PoC gerekli]. Helper'ın nasıl başlatılacağı ve servisin hangi hesapla çalışacağı hem güvenilirliği hem en az yetki ilkesini belirler.

## Değerlendirilen seçenekler

1. **M1 — Helper HKLM Run / oturum açma tetikli zamanlanmış görevle, çocuk kimliğiyle başlar; servis düşük yetkili hesap (LocalService / sanal hesap).** `WTSQueryUserToken` gerekmez.
2. **M2 — Servis helper'ı `WTSQueryUserToken` + `CreateProcessAsUser` ile başlatır ve sonlandırılınca yeniden doğurur.** Bu API yalnız LocalSystem + `SE_TCB_NAME` ile çalışır ([Microsoft](https://learn.microsoft.com/en-us/windows/win32/api/wtsapi32/nf-wtsapi32-wtsqueryusertoken)).
3. **Bölünmüş model — düşük yetkili ana agent + yalnız helper başlatma/izlemeden sorumlu küçük LocalSystem bileşeni.**
4. **M3 — Helper'ı çocuk tarafından sonlandırılamayan ayrıcalıklı kimlikle oturumda çalıştırmak.** Microsoft'un yükseltilmiş süreçlerin etkileşimli masaüstünde pencere oluşturmaması uyarısına aykırı → **önerilmez.**

## Önerilen yaklaşım

PoC-A/B'de M1 ve M2 (veya bölünmüş model) **ölçülerek** karşılaştırılsın: helper sonlandırıldığında tespit ve yeniden uygulama süresi, saldırı yüzeyi, kurulum karmaşıklığı. Hangi modelde olursa olsun ikinci savunma hattı: servis helper'ın yokluğunu algılar, kurcalama olayı kaydeder ve ebeveynin seçtiği yedek tepkiyi uygular. LocalSystem **tüm agent için varsayılan değildir**; yalnız ölçümle gerekçelendirilen bileşene verilir.

## Avantajlar

- En az yetki ilkesi korunur; LocalSystem yalnız gerekirse ve dar kapsamda.
- Karar varsayıma değil gerçek Windows ölçümüne dayanır.

## Riskler ve ödünleşimler

- M1: helper sonlandırılınca servis onu yeniden başlatamaz; yalnız tespit + yedek tepki.
- M2/bölünmüş: LocalSystem bileşeni yüksek değerli hedef; sertleştirme ve kod imzalama şart.
- Yedek tepki (oturum kilidi/bağlantı kesme) kullanıcı deneyimini sertleştirir; ebeveyn ayarı olmalı.

## Açık doğrulama gereksinimleri

- Standart kullanıcının helper'ı sonlandırabildiği ve servisi durduramadığı (PoC A-02, A-10).
- Her modelde yeniden uygulama süresi (A-03).
- Servisten oturum kilitleme/bağlantı kesme yönteminin çalışıp çalışmadığı.

## P1.A notu (2026-10-28)

Prototip kodu servis hesabı seçmez; `sc.exe create ... obj=` ile verilir (ilk deneme önerisi LocalService). Yeni açık soru: LocalService altındaki agent, helper sürecinin yolunu `PROCESS_QUERY_LIMITED_INFORMATION` ile sorgulayabiliyor mu? Sorgulayamazsa yetkilendirme fail-closed reddeder (W-10, [P1.A raporu](../agent/p1a-windows-agent.md)). Bu ölçüm hesap modeli kararına girdi olacak.

## P1.A.2 notu (2026-10-28) — W-10 sonucu ve BUG-003

- **Cihaz sonucu (kullanıcı, `p1a-windows-test.1`):** LocalService agent, helper'ın PID ve oturumunu okudu.
  - `OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION)` ise `5` ile reddedildi ve yetkilendirme fail-closed reddetti. W-10 FAIL.
  - Neden: kullanıcı sürecinin varsayılan DACL'i LocalService'e izin vermez ([BUG-003](../solutions.md)).
- **Düzeltme (düşük yetkili kalarak):**
  - Helper, kendi süreç DACL'ine yalnız LocalService için `PROCESS_QUERY_LIMITED_INFORMATION` ekler.
  - LocalSystem'e geçilmedi. Kimlik doğrulama (OS'den exe yolu, oturum, ad, dizin) korundu.
- **CI kanıtı:** release agent gerçek LocalService servisi olarak çalıştı ve helper'ı yetkilendirdi; ACE'siz istemci yine reddedildi.
  - Çalıştırma: [40720517755](https://github.com/mehmetakarim/Kindora/actions/runs/40720517755).
  - Kullanıcının Lenovo cihazında `p1a-windows-test.2` ile doğrulandı (2026-10-28, kullanıcı raporu): LocalService başlatma, pipe iletişimi, helper exe yolu doğrulaması, `os_errors=-` — **W-10 PASS**.
- **Bu ADR için anlamı:** M1 (düşük yetkili servis) için kimlik doğrulama engeli giderildi. Hesap modeli kararı hâlâ **Önerildi**; helper başlatma/yeniden doğurma (M1 vs M2) ölçümü açık.
- **Daraltma seçeneği:** izin bugün tüm LocalService servislerine açıktır. Servis SID'i (`sc.exe sidtype <ad> restricted` + ACE'de `NT SERVICE\<ad>`) ile yalnız Kindora servisine indirilebilir; kurulum aşamasında değerlendirilecek.
- **Hesap:** Kindora mevcut Windows hesabında çalışır ([ADR-0012](0012-mevcut-windows-hesabinda-calisma.md)). Yönetici hesapta kurcalama engellenemez, yalnız tespit edilebilir.

## İlişkili kararlar

[ADR-0001](0001-kindora-kullanim-kilidi-ana-kontrol.md), [ADR-0007](0007-servis-helper-ipc-named-pipe.md), [ADR-0012](0012-mevcut-windows-hesabinda-calisma.md).

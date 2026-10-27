# ADR-0001 — Kindora kullanım kilidi ana kontrol yöntemidir

- **Tarih:** 2026-10-27 (kayıt tarihi; karar kullanıcı tarafından daha önce verildi, kesin tarih belgelenmemiş)
- **Durum:** Kabul edildi (ürün kararı) — zorlayıcı uygulama yöntemi **Önerildi / PoC bekliyor**
- **Karar türü:** Ürün kararı

## Bağlam ve problem

Ebeveyn, bilgisayar başında olmadan çocuğun kullanımını durdurabilmeli ve izin verdiğinde Windows parolası gerekmeden yeniden açabilmelidir. Windows oturum kilidi (`LockWorkStation()`) yeniden açılışta Windows kimlik doğrulaması ister; uzaktan açma Windows güvenliğini atlatmayı gerektirir ve kapsam dışıdır. Kaynak: [context §9](../../design/KINDORA_PROJECT_CONTEXT.md#9-kindora-kullanım-kilidi-ve-windows-kilidi).

## Değerlendirilen seçenekler

1. **Kindora kullanım kilidi** — Windows oturumu açık kalır; Kindora mola/kısıtlama ekranı ve sistem denetimleriyle kullanım sınırlanır; ebeveyn onayıyla Kindora kısıtlaması kaldırılır.
2. **Windows oturum kilidi** — işletim sisteminin yerleşik kilidi; ikincil işlev olarak korunur.
3. (Tasarım teslimatındaki öneri) "Bilgisayarı kilitle" = 1 dk uyarı sonrası Windows oturum kilidi **+** Kindora kısıtlaması birlikte (Acceptance Report, karar 4). **Reddedildi** — bkz. aşağıdaki 2026-10-27 güncellemesi.

## Seçilen yaklaşım ve gerekçe

Seçenek 1 ana ürün yaklaşımıdır; seçenek 2 ayrı adlandırılmış ikincil işlevdir. Gerekçe (context'ten): mobil uygulamayla kolay yönetim, internet kafe benzeri açıklayıcı mola deneyimi, Windows parolasına dokunmama.

Komutlar ayrı adlandırılır: `APPLY_KINDORA_RESTRICTION`, `RELEASE_KINDORA_RESTRICTION`, `LOCK_WINDOWS_SESSION`.

## Sonuçlar ve ödünleşimler

- Tam ekran/always-on-top pencere güvenlik mekanizması **değildir**; gerçek kısıtlama için Windows'un desteklediği sistem sınırları araştırılmalı.
- Ctrl+Alt+Del gibi güvenli dizileri engelleme vaadi yok; bu nedenle kısıtlamanın aşılabilirlik sınırları açıkça belgelenecek.
- Kurtarılamaz kilitlenme riskine karşı yerel ebeveyn/yönetici kurtarma akışı gerekir.

## Bilinmeyenler / doğrulama gereken noktalar

- Zorlayıcı uygulama yöntemi (hangi Windows API/politika) — **GATE A PoC**.
- Değerlendirilen diğer alternatifler ve kararın tarihi belgelenmemiş.

## Güncelleme — 2026-10-27 (Phase M0.1, kullanıcı kararı B)

Kullanıcı açıkça onayladı: Kindora kullanım kilidi ana kontrol yöntemidir; **Windows oturum kilidi ek ve bağımsız bir komuttur**. İki mekanizma aynı işlem gibi ele alınmaz. Tasarım teslimatındaki birleşik "Bilgisayarı kilitle" önerisi (seçenek 3) bu nedenle uygulanmaz; tasarım dosyalarının tarihsel içeriği değiştirilmedi, uygulamada bu ADR esas alınır. UI'da kilit komutları ayrı eylemler olarak sunulur.

## İlişkili kararlar

- [ADR-0002](0002-supabase-sqlite-gorev-ayrimi.md) — kuralların yerelde uygulanması.

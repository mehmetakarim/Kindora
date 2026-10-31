# Architecture Decision Records (ADR)

Kindora'nın mimari ve teknik kararlarının geçmişi. Ürün gereksinimleri [KINDORA_PROJECT_CONTEXT.md](../../design/KINDORA_PROJECT_CONTEXT.md) içinde; bu klasör **neden** ve **hangi ödünleşimle** karar verildiğini kaydeder.

## Ne zaman ADR yazılır?

- Teknoloji, kütüphane veya sürüm seçimi (ör. Bootstrap 6 vs 5.3).
- Bileşenler arası sorumluluk, iletişim veya güvenlik sınırı değişikliği.
- PoC sonucuna göre bir uygulama yönteminin seçilmesi (ör. web filtreleme yöntemi).
- Önceki bir kararın geçersiz kılınması.

## Dosya adı

`NNNN-kisa-karar-basligi.md` — dört haneli sıra numarası, küçük harf, Türkçe karakter yerine ASCII, tire ile ayrılmış.

## Şablon

```markdown
# ADR-NNNN — Başlık

- **Tarih:** YYYY-MM-DD
- **Durum:** Önerildi | Kabul edildi | Yerine yeni karar alındı (→ ADR-XXXX)
- **Karar türü:** Ürün kararı | Teknik uygulama yöntemi

## Bağlam ve problem
## Değerlendirilen seçenekler
## Seçilen yaklaşım ve gerekçe
## Sonuçlar ve ödünleşimler
## Bilinmeyenler / doğrulama gereken noktalar
## İlişkili kararlar
```

## Kurallar

- Gerekçesi veya alternatifleri kaynaklarda açık olmayan kısımlar için ayrıntı uydurma; "Belgelenmemiş" yaz.
- Onaylı ürün kararı ile henüz öneri niteliğindeki teknik yöntemi ayır.
- ADR silinmez; geçersizleşirse durumu "Yerine yeni karar alındı" yapılır ve yeni ADR'ye bağlanır.

## Dizin

| ADR | Başlık | Durum | Tür |
|---|---|---|---|
| [0001](0001-kindora-kullanim-kilidi-ana-kontrol.md) | Kindora kullanım kilidi ana kontrol yöntemidir; Windows oturum kilidi bağımsız ek komut | Kabul edildi | Ürün kararı (uygulama yöntemi PoC bekliyor) |
| [0002](0002-supabase-sqlite-gorev-ayrimi.md) | Supabase merkezi backend, SQLite yerel/çevrimdışı veri | Kabul edildi | Ürün/teknoloji tercihi |
| [0003](0003-flutter-ebeveyn-mobil-mvp.md) | Ebeveyn mobil uygulaması Flutter ile ve MVP kapsamında (iOS + Android) | Kabul edildi | Ürün/teknoloji tercihi |
| [0004](0004-claude-design-tasarim-yonu-kabulu.md) | Claude Design genel UI/UX yönünün kabulü (maskot, Windows entegrasyonu, erişilebilirlik hariç) | Kabul edildi (kısmi) | Ürün kararı |
| [0005](0005-bootstrap-6-hedef.md) | Bootstrap 6 hedef teknolojidir; 6 kararlı değilken kullanıcı onayıyla 5.3.8 (P1.B) | Kabul edildi (hedef) | Teknoloji tercihi |
| [0006](0006-agent-surec-ve-hesap-modeli.md) | Windows Agent süreç ve servis hesabı modeli (LocalSystem varsayılan değil) | **Önerildi** | Teknik uygulama yöntemi |
| [0007](0007-servis-helper-ipc-named-pipe.md) | Servis ↔ helper IPC: oturum başına named pipe | **Önerildi** | Teknik uygulama yöntemi |
| [0008](0008-cihaz-kimligi-ve-eslestirme.md) | Cihaz kimliği ve güvenli eşleştirme | **Önerildi** | Teknik uygulama yöntemi |
| [0009](0009-kalici-komut-kuyrugu-realtime-uyandirma.md) | Kalıcı komut kuyruğu, Realtime yalnız uyandırma | **Önerildi** | Teknik uygulama yöntemi |
| [0010](0010-paylasilan-sozlesme-json-schema.md) | Paylaşılan sözleşme: JSON Schema → TS/Dart/Rust | **Önerildi** | Teknik uygulama yöntemi |
| [0011](0011-p1a-crate-yapisi-ve-gelistirme-tasimasi.md) | P1.A crate yapısı, pipe erişim modeli, Unix geliştirme taşıması | **Önerildi** | Teknik uygulama yöntemi |
| [0012](0012-mevcut-windows-hesabinda-calisma.md) | Kindora mevcut Windows hesabında çalışır; çocuk için ayrı hesap zorunlu değil | Kabul edildi | Ürün kararı |
| [0013](0013-masaustu-kabugu-helper-koprusu.md) | Masaüstü kabuğu agent'a Session Helper köprüsüyle (stdio) bağlanır; politika gevşetilmez | Kabul edildi (2026-10-31; Lenovo GUI PASS) | Teknik uygulama yöntemi |
| [0014](0014-yerel-kullanim-suresi-motoru.md) | Yerel kullanım süresi motoru: ölçüm agent'ta, girdi sinyali helper'dan, kayıt SQLite'ta; DEV politika dosyası | Kabul edildi (2026-10-31; Lenovo temel kabul PASS) | Teknik uygulama yöntemi |
| [0015](0015-bagimsiz-session-helper-ve-oturum-acilisi.md) | Bağımsız Session Helper, agent'ta tek raporlayıcı kiralaması, kullanıcı başına oturum açılışı görevi | Kabul edildi (2026-10-31; Lenovo kabul PASS) | Teknik uygulama yöntemi |
| [0016](0016-yerel-kindora-kisitlamasi-poc.md) | Yerel Kindora kısıtlaması PoC: durum/ekran ayrımı, zaman modeli, fail-open, kurtarma sınıflandırması | **Taslak** (onay bekliyor) | Teknik uygulama yöntemi |

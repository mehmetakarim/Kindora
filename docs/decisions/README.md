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
| [0005](0005-bootstrap-6-hedef.md) | Bootstrap 6 hedef teknolojidir; 5.3'e geçiş kullanıcı onayı ister | Kabul edildi (hedef) | Teknoloji tercihi |

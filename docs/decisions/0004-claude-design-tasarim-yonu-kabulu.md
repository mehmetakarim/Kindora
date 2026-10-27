# ADR-0004 — Claude Design genel UI/UX tasarım yönünün kabulü

- **Tarih:** 2026-10-27
- **Durum:** Kabul edildi (kısmi kapsam — aşağıdaki doğrulanmamış alanlar hariç)
- **Karar türü:** Ürün kararı

## Bağlam ve problem

[Claude Design teslimatı](../../design/design_handoff_kindora/README.md) ve Acceptance Report tasarımı "Kullanıcı onayı bekliyor / geliştirmeye aktarılmadı" olarak gösterir ve 10 açık karar listeler. [KINDORA_PROJECT_CONTEXT.md §20](../../design/KINDORA_PROJECT_CONTEXT.md#20-tasarım-referansları-ve-durumları) ise genel tasarım yönünün kabul edildiğini söyler. Geliştirme için hangi kısmın kabul edildiği netleştirilmeliydi.

## Değerlendirilen seçenekler

1. Tasarımı bütünüyle kabul etmek.
2. Genel UI/UX yönünü kabul etmek; nihai maskot, Windows entegrasyonları ve erişilebilirliği ayrı doğrulamaya bırakmak.
3. Tasarım aşamasını açık tutmak.

## Seçilen yaklaşım ve gerekçe

Seçenek 2 (kullanıcı kararı C, 2026-10-27). Görsel dil, tokenlar, bileşen envanteri ve kanonik prototipler (Phase 5 + Dark, Phase 3, Phase 4, Onboarding) geliştirme referansıdır.

**Kabul kapsamı dışında kalan, henüz doğrulanmamış alanlar:**

- Filiz'in nihai illüstrasyonu ve animasyonları (Mascot Review kapısı açık).
- Gerçek Windows entegrasyonları (oturum kilidi, widget always-on-top/şeffaflık, alan adı engelleme, bildirimler).
- Erişilebilirlik testleri (NVDA/Narrator, gerçek DPI ölçekleme, widget klavye odağı).

## Sonuçlar ve ödünleşimler

- Tasarım dosyalarının tarihsel içeriği değiştirilmez; çelişkiler güncel ADR'lerle çözülür ([ADR-0001](0001-kindora-kullanim-kilidi-ana-kontrol.md) kilit, [ADR-0003](0003-flutter-ebeveyn-mobil-mvp.md) mobil kapsam, [ADR-0005](0005-bootstrap-6-hedef.md) Bootstrap).
- Acceptance Report'taki diğer açık kararlar (1280×800 @%150 desteği, 30 sn komut zaman aşımı, onboarding adım sayısı, çocuk tarafı dark tema) bu ADR ile **kapatılmadı**; [docs/tasks.md](../tasks.md) açık bulgularında izlenir.

## İlişkili kararlar

- [ADR-0001](0001-kindora-kullanim-kilidi-ana-kontrol.md), [ADR-0003](0003-flutter-ebeveyn-mobil-mvp.md), [ADR-0005](0005-bootstrap-6-hedef.md).

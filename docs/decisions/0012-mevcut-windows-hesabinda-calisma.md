# ADR-0012 — Kindora mevcut Windows kullanıcı hesabında çalışır

- **Tarih:** 2026-10-28
- **Durum:** Kabul edildi
- **Karar türü:** Ürün kararı (kullanıcı kararı, P1.A.2)
- **İlgili belgeler:** [ADR-0006](0006-agent-surec-ve-hesap-modeli.md), [T0-A §5](../feasibility/T0-A-kindora-lock.md), [P1.A raporu](../agent/p1a-windows-agent.md), [solutions.md BUG-003](../solutions.md)

## Bağlam ve problem

Proje bağlamı ([KINDORA_PROJECT_CONTEXT.md](../../design/KINDORA_PROJECT_CONTEXT.md) §8 ve §14) ve T0-A §5, çocuğun ayrı bir **standart** Windows hesabı kullanmasını ön koşul olarak varsayıyordu. P1.A test rehberi de bu yüzden ayrı bir `kindora-test` hesabı açtırıyordu. Kullanıcının gerçek Windows testi mevcut hesabında (Lenovo bilgisayar) yapıldı ve LocalService kimlik doğrulama hatası (BUG-003) bu hesapta ortaya çıktı.

## Karar

- Kindora **mevcut Windows kullanıcı hesabında** çalışmalıdır.
- Çocuk adına ayrı bir Windows kullanıcı hesabı oluşturulması **zorunlu değildir**.
- Mevcut Lenovo hesabı **gerçek Windows test hesabıdır**. Test rehberi yeni hesap veya sanal makine kurulmasını zorunlu tutmaz.

Tasarım dosyası (`design/`) değiştirilmedi; standart çocuk hesabı varsayımı bu ADR ile güncellenmiştir.

## Sonuçlar ve ödünleşimler

- Agent ve helper, kullanıcının yönetici olup olmamasından bağımsız çalışmalıdır. Helper normal (yükseltilmemiş) süreç olarak çalışır; servis işlemleri yönetici onayı (UAC) ister.
- **Güvenlik sınırı (açık risk):** Hesap Administrators grubundaysa kullanıcı UAC onayıyla servisi durdurabilir veya kaldırabilir. Bu durumda Kindora kurcalamayı engelleyemez; yalnız tespit ve ebeveyne bildirim hedeflenebilir. "Kapatılamaz" iddiası yazılmaz (AGENTS.md §5.2). Yönetici hesapta koruma düzeyi ve uyarı/ebeveyn bildirimi ürün akışı ayrıca değerlendirilecek.
- Yükseltilmemiş yönetici penceresinden yapılan testler, yönetici token'ının filtrelendiği yolu (pipe'a `IU` girişiyle erişim, `sc.exe stop` reddi) sınar; bu, ayrı standart hesap testine yakın ama özdeş değildir.
- T0-A §5 ve T0-B kurulum gereksinimlerindeki "çocuk için standart hesap zorunlu" maddesi bu ADR ile geçersizdir (tarihsel belgeler değiştirilmedi).

## Bilinmeyenler / doğrulama gereken noktalar

- Yönetici hesapta kullanım kısıtlamasının ne ölçüde sürdürülebileceği (P1.B ve sonrası PoC).
- Kurulum sihirbazının hesap türünü tespit edip ebeveyne nasıl bilgi vereceği (ürün akışı — karar bekliyor).

## İlişkili kararlar

[ADR-0001](0001-kindora-kullanim-kilidi-ana-kontrol.md), [ADR-0006](0006-agent-surec-ve-hesap-modeli.md), [ADR-0011](0011-p1a-crate-yapisi-ve-gelistirme-tasimasi.md).

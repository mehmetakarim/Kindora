# ADR-0005 — Bootstrap 6 hedef teknolojidir

- **Tarih:** 2026-10-27
- **Durum:** Kabul edildi (hedef) — sürüm kararlılığı **doğrulanmadı**
- **Karar türü:** Teknoloji tercihi

## Bağlam ve problem

Context ve tasarım teslimatı Bootstrap 6'yı hedefler; Acceptance Report "Bootstrap 6 kararlı değilse 5.3 ile başla, tokenlar aynı kalsın" önerisini kullanıcı kararı olarak bekletir. Token dosyası "Bootstrap 5.3+/6 uyumlu" olarak yazılmıştır.

## Değerlendirilen seçenekler

1. Bootstrap 6 (hedef).
2. Bootstrap 5.3 + aynı tokenlar (tasarım teslimatındaki yedek öneri).

## Seçilen yaklaşım ve gerekçe

Bootstrap 6 hedef teknolojidir (kullanıcı kararı D, 2026-10-27). Mevcut kararlı sürüm ve Vue 3 / Tauri / token uyumluluğu **geliştirme başlamadan** resmi kaynaktan doğrulanır. **Kullanıcı onayı olmadan Bootstrap 5.3'e geçiş kararı verilmez.** Doğrulama Bootstrap 6'nın uygun olmadığını gösterirse durum raporlanır ve yeni ADR için kullanıcı kararı istenir.

Tailwind CSS kullanılmaz (context §3).

## Sonuçlar ve ödünleşimler

- Bootstrap 6 alfa/beta ise API değişikliği riski vardır; kesin sürüm lock dosyasıyla sabitlenmelidir.
- 5.3 yedeği yalnızca öneridir; onaylı değildir.

## Bilinmeyenler / doğrulama gereken noktalar

- Bootstrap 6'nın güncel yayın durumu (alfa/beta/kararlı), SCSS API'si ve `_kindora-tokens.scss` ile uyumu.

## İlişkili kararlar

- [ADR-0004](0004-claude-design-tasarim-yonu-kabulu.md).

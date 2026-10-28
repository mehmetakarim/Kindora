# ADR-0011 — P1.A crate yapısı, Windows pipe erişim modeli ve geliştirme taşıması

- **Tarih:** 2026-10-28
- **Durum:** Önerildi
- **Karar türü:** Teknik uygulama yöntemi
- **İlgili belgeler:** [P1.A raporu](../agent/p1a-windows-agent.md), [T0-B](../feasibility/T0-B-windows-agent.md), [ADR-0007](0007-servis-helper-ipc-named-pipe.md)

## Problem ve bağlam

P1.A ilk Rust kodunu oluşturdu. Geliştirme makinesi macOS; Windows test ortamı yok. Agent ve helper'ın ayrı süreçler olarak uçtan uca denenebilmesi ve Windows'a özgü kodun izole kalması gerekiyordu. Ayrıca ADR-0007'deki "oturum başına ad + logon SID" modeli, LocalSystem olmayan bir servis için doğrudan uygulanamıyor (logon SID kullanıcı token'ı ister).

## Değerlendirilen seçenekler

1. Windows kodu yazıp yalnız derleme denetimi yapmak (uçtan uca test yok).
2. **Taşımayı soyutlamak; Windows'ta named pipe, Unix'te yalnız geliştirme amaçlı domain socket; protokol ve bağlantı mantığını platformdan bağımsız tutmak.**
3. Platformlar arası bir IPC kütüphanesi (ör. TCP loopback) — Windows hedefinde ACL yok → reddedilir.

Pipe erişim modeli için:

- a. Oturum başına ad + logon SID (ADR-0007) — servis LocalSystem değilse logon SID alınamaz.
- b. **Tek ad + Interactive Users (yalnız okuma/yazma) + işletim sisteminden alınan oturum/exe denetimi.**

## Önerilen yaklaşım

- Crate'ler: `ipc-protocol` (modeller, çerçeve, taşıma), `agent-core` (platformdan bağımsız), `windows-agent` (bin `kindora-agent`), `windows-session` (bin `kindora-helper`; T0-B kanonik adı).
- Seçenek 2 + pipe modeli b. Unix taşıması yalnız `cfg(unix)` altında derlenir; Windows ikililerine girmez.
- ADR-0006 kararı verildiğinde (ör. LocalSystem başlatıcı bileşen) pipe modeli a'ya geçilebilir.

## Avantajlar

- Protokol, yetkilendirme kararı, zaman aşımı, hata bütçesi ve süreç yaşam döngüsü bugün gerçek süreçlerle test edilebiliyor (6 e2e test).
- Windows'a özgü kod tek modülde (`windows_pipe.rs`, `service.rs`) ve Windows PoC'de hedefli doğrulanabilir.

## Riskler ve ödünleşimler

- Unix testlerindeki PASS, Windows davranışını kanıtlamaz; yanlış güven riski → belgelerde her sonuç ortamıyla etiketlenir.
- Interactive Users, logon SID'den geniştir (aynı makinedeki her etkileşimli kullanıcı bağlanmayı deneyebilir); oturum + exe denetimi ve "helper yetkili karar veremez" ilkesiyle sınırlanır.
- Exe yolu denetimi LocalService altında çalışmayabilir (W-10).

## Açık doğrulama gereksinimleri

- W-01…W-12 Windows testleri ([P1.A raporu §5](../agent/p1a-windows-agent.md)).
- Özellikle W-03 (DACL maskesi yeterli mi) ve W-10 (LocalService ile süreç yolu sorgusu).

## İlişkili kararlar

[ADR-0006](0006-agent-surec-ve-hesap-modeli.md), [ADR-0007](0007-servis-helper-ipc-named-pipe.md).

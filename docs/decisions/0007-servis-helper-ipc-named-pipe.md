# ADR-0007 — Servis ↔ helper IPC: oturum başına named pipe

- **Tarih:** 2026-10-28
- **Durum:** Önerildi
- **Karar türü:** Teknik uygulama yöntemi
- **İlgili T0 belgesi:** [T0-B](../feasibility/T0-B-windows-agent.md) §5

## Problem ve bağlam

Session 0'daki servis ile çocuk oturumundaki helper arasında güvenli, yerel, yetki sınırlı bir kanal gerekir. Microsoft named pipe'ı önerir ve uygun ACL olmadan IPC'nin açığa çıkabileceğini belirtir ([Microsoft](https://learn.microsoft.com/en-us/windows/win32/services/interactive-services)). Named pipe'ın varsayılan güvenlik tanımlayıcısı Everyone'a okuma hakkı verir ([Microsoft](https://learn.microsoft.com/en-us/windows/win32/ipc/named-pipe-security-and-access-rights)).

## Değerlendirilen seçenekler

1. Named pipe (oturum başına, açık DACL).
2. Yerel TCP/loopback soket — ağ yığınına açık, ACL yok → önerilmez.
3. COM/RPC (ALPC) — güçlü ama karmaşık; Rust ekosisteminde daha zor.
4. Paylaşılan dosya/kayıt defteri — yarış koşulları, kurcalanabilir → önerilmez.

## Önerilen yaklaşım

Seçenek 1: `\\.\pipe\kindora\session-<id>-<rastgele>`; açık DACL (servis SID tam yetki + ilgili oturumun logon SID'i bireysel haklarla), `PIPE_REJECT_REMOTE_CLIENTS`, `FILE_FLAG_FIRST_PIPE_INSTANCE`; istemci süreç kimliği/yolu/imzası doğrulaması; sürümlü, uzunluk sınırlı, şemayla doğrulanan mesajlar. Helper'ın politika değiştiren mesajı yoktur (yalnız talep, aktivite sinyali, UI durumu, onay).

## Avantajlar

Windows'un yerleşik erişim denetimi; oturumlar arası ayrım; Microsoft'un önerdiği desen.

## Riskler ve ödünleşimler

- Yanlış DACL → başka oturumdan veya kötü niyetli süreçten bağlantı.
- Pipe adı gaspı (squatting) — ilk örnek bayrağı ve rastgele sonekle azaltılır.
- İstemci süreç doğrulaması atlatılabilir; asıl güvence helper'ın hiçbir yetkili işlem isteyememesidir.

## Açık doğrulama gereksinimleri

- Başka oturum / başka kullanıcı / sahte süreçten bağlantı denemesi reddedilmeli (PoC-B).
- Rust'ta DACL ve istemci PID doğrulamasının uygulanabilirliği.

## P1.A uygulama notu (2026-10-28)

İlk prototip tek pipe adı (`\\.\pipe\kindora-agent-v1`) + Interactive Users okuma/yazma + işletim sisteminden oturum/exe denetimi kullanır; oturum başına ad + logon SID, servis LocalSystem olmadan uygulanamadığı için ertelendi. Gerekçe ve riskler: [ADR-0011](0011-p1a-crate-yapisi-ve-gelistirme-tasimasi.md). Windows'ta doğrulanmadı.

## İlişkili kararlar

[ADR-0006](0006-agent-surec-ve-hesap-modeli.md), [ADR-0010](0010-paylasilan-sozlesme-json-schema.md).

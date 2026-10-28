# Kindora — Teknik Uygulanabilirlik (Phase T0)

> Tarih: 2026-10-28 · Görev: [TASK-003](../tasks.md) · Durum: **Dokümantasyon kabul edildi (PASS, 2026-10-28)** — teknik doğrulamalar NOT RUN
> Bu klasör **araştırma ve planlama** çıktısıdır. Hiçbir bölüm çalışan kod, gerçek Windows testi veya Supabase entegrasyonu kanıtı değildir.

## Belgeler

| Belge | Kapsam | Kapı |
|---|---|---|
| [T0-A-kindora-lock.md](T0-A-kindora-lock.md) | Kindora kullanım kilidi: oturumlar, Session 0, pencere/klavye davranışı, güvenlik sınırları, PoC tasarımı | GATE T0-A |
| [T0-B-windows-agent.md](T0-B-windows-agent.md) | Rust Windows Agent bileşenleri, servis hesabı, IPC, yaşam döngüsü, kurtarma, destek matrisi | GATE T0-B |
| [T0-C-supabase-architecture.md](T0-C-supabase-architecture.md) | Supabase veri modeli, RLS, cihaz kimliği/eşleştirme, komut iletimi, maliyet | GATE T0-C |
| [T0-D-offline-policy.md](T0-D-offline-policy.md) | SQLite modeli, çevrimdışı davranış, saat manipülasyonu, fail-open/closed politikası | GATE T0-D |
| [T0-E-client-integration.md](T0-E-client-integration.md) | Vue/Flutter ortak sözleşme, komut durum makinesi, token, push, UI durumları | GATE T0-E |
| [risk-register.md](risk-register.md) | P0–P3 teknik riskler | — |
| [implementation-roadmap.md](implementation-roadmap.md) | Önerilen geliştirme fazları (uygulama izni vermez) | — |

## Kanıt düzeyi etiketleri

Belgelerde her teknik iddia şu etiketlerden biriyle işaretlenir:

- **[Resmi kaynak]** — Microsoft Learn / Supabase / Tauri / SQLite / Bootstrap resmi belgesiyle doğrulandı (bağlantı verilir).
- **[Çıkarım]** — Resmi kaynaklardan mantıksal olarak çıkarılan, henüz test edilmemiş sonuç.
- **[PoC gerekli]** — Gerçek Windows / Supabase ortamında ölçülmeden kabul edilemez.

Araştırma macOS üzerinde yapıldı; **hiçbir Windows davranışı çalıştırılarak doğrulanmadı.**

## Kapı durumu (T0 — yalnızca dokümantasyon kabulü)

| Kapı | Belge durumu | Kullanıcı kabulü |
|---|---|---|
| GATE T0-A — Windows Lock Feasibility | Hazırlandı | PASS (yalnız plan) |
| GATE T0-B — Agent Architecture | Hazırlandı | PASS (yalnız plan) |
| GATE T0-C — Backend Security | Hazırlandı | PASS (yalnız plan) |
| GATE T0-D — Offline Resilience | Hazırlandı | PASS (yalnız plan) |
| GATE T0-E — Client Integration | Hazırlandı | PASS (yalnız plan) |

T0 kapıları yalnızca **plan ve uygulanabilirlik analizinin** kabulünü temsil eder. Uygulama fazlarında kanıta dayalı ayrı teknik kapılar kullanılır (context'teki GATE A–E; bkz. [roadmap](implementation-roadmap.md)).

## Önerilen ADR'ler (kabul edilmedi)

[ADR-0006](../decisions/0006-agent-surec-ve-hesap-modeli.md), [ADR-0007](../decisions/0007-servis-helper-ipc-named-pipe.md), [ADR-0008](../decisions/0008-cihaz-kimligi-ve-eslestirme.md), [ADR-0009](../decisions/0009-kalici-komut-kuyrugu-realtime-uyandirma.md), [ADR-0010](../decisions/0010-paylasilan-sozlesme-json-schema.md) — durumları **Önerildi**. Kullanıcı onayı olmadan "Kabul edildi" yapılmaz.

## Birincil kaynaklar

- Microsoft Learn: [Interactive Services / Session 0](https://learn.microsoft.com/en-us/windows/win32/services/interactive-services), [LockWorkStation](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-lockworkstation), [WTSQueryUserToken](https://learn.microsoft.com/en-us/windows/win32/api/wtsapi32/nf-wtsapi32-wtsqueryusertoken), [Service Security and Access Rights](https://learn.microsoft.com/en-us/windows/win32/services/service-security-and-access-rights), [Service User Accounts](https://learn.microsoft.com/en-us/windows/win32/services/service-user-accounts), [Named Pipe Security](https://learn.microsoft.com/en-us/windows/win32/ipc/named-pipe-security-and-access-rights), [SetWindowsHookEx](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwindowshookexw), [Assigned Access](https://learn.microsoft.com/en-us/windows/configuration/assigned-access/overview), [Windows 10 ESU](https://learn.microsoft.com/en-us/windows/whats-new/extended-security-updates)
- Supabase: [Pricing](https://supabase.com/pricing), [Realtime Authorization](https://supabase.com/docs/guides/realtime/authorization), [Subscribing to Database Changes](https://supabase.com/docs/guides/realtime/subscribing-to-database-changes), [JWTs / Signing keys](https://supabase.com/docs/guides/auth/jwts), [Auth Hooks](https://supabase.com/docs/guides/auth/auth-hooks)
- Tauri: [Webview versions](https://v2.tauri.app/reference/webview-versions/)
- SQLite: [Write-Ahead Logging](https://www.sqlite.org/wal.html)
- Bootstrap: [Bootstrap 6 alpha (resmi blog, 8 Ekim 2026)](https://blog.getbootstrap.com/2026/10/08/bootstrap-6-alpha/) — `6.0.0-alpha.1`
- Microsoft Learn ek: [Kiosk / Assigned Access yöntemleri](https://learn.microsoft.com/en-us/windows/configuration/kiosk-methods) — Home desteklenmez

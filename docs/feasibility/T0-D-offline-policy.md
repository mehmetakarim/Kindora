# T0-D — SQLite ve Çevrimdışı Kurallar

> Durum: **Taslak / öneri** · Kapı: GATE T0-D · İlgili: [ADR-0002](../decisions/0002-supabase-sqlite-gorev-ayrimi.md)
> Veri bütünlüğü veya kurcalama dayanıklılığı için garanti verilmez; aşağıdakiler tasarım hedefidir.

## 1. SQLite sorumlulukları ve tablo taslağı

| Tablo (öneri) | İçerik |
|---|---|
| `meta` | şema sürümü, cihaz kimliği, son başarılı senkron (sunucu zamanı + yerel monoton sayaç), son bilinen güvenilir saat |
| `policy_versions` | version, body, alındığı sunucu zamanı, (varsa) imza, `active` bayrağı |
| `usage_days` | yerel gün anahtarı, child profile, kullanılan aktif saniye, son güncelleme monoton zamanı |
| `usage_segments` | oturum başlangıç/bitiş (monoton + duvar saati), neden (aktif/boşta/kilit/uyku) |
| `breaks` | planlanan/gerçekleşen molalar |
| `grants` | geçici izinler (ek süre, site izni): kaynak komut id, verildiği sunucu zamanı, **süre (dakika)**, kalan, `expires_at` |
| `processed_commands` | command_id (UNIQUE), sonuç, uygulama zamanı, receipt gönderildi mi |
| `outbox` | bekleyen olaylar (receipt, talep, usage özeti, audit) — sıra no + idempotency anahtarı |
| `restriction_state` | aktif kısıtlama, nedeni, başlangıç |

Ayarlar: `journal_mode=WAL`, kritik yazmalarda `synchronous=FULL` ([Resmi kaynak](https://www.sqlite.org/wal.html): NORMAL'da güç kaybında son commit'ler geri alınabilir). WAL dosyası veritabanıyla birlikte taşınmalı. Dosya `C:\ProgramData\Kindora\` altında, ACL: servis hesabı + Administrators; çocuk hesabına erişim yok [PoC gerekli].

## 2. Zaman modeli

- **Kullanım süresi** monoton saatle (`QueryUnbiasedInterruptTime` / `GetTickCount64` türevi; uyku hariç) ölçülür → duvar saati değişikliği süreyi artırmaz/azaltmaz [Çıkarım — PoC gerekli].
- **Gün devri ve program** duvar saatine bağlıdır; bu yüzden saat değişikliği tespiti gerekir: son güvenilir sunucu zamanı + o andan beri geçen monoton süre ≈ beklenen zaman. Fark > eşik (ör. 5 dk) → "saat sapması" olayı.
- Çocuk standart kullanıcı olduğundan sistem saatini değiştiremez (varsayılan `SeSystemtimePrivilege` yalnız Administrators/LocalService) [Çıkarım — PoC gerekli]. Saat dilimi değişimi standart kullanıcıya açık olabilir → yerel gün sınırını etkiler; politika **cihazın kayıtlı saat dilimini** kullanmalı.

## 3. Senaryo davranışları

| # | Durum | Önerilen davranış |
|---|---|---|
| 1 | İnternet kesildi | Son geçerli politika uygulanmaya devam; talepler `outbox`'ta "Gönderilecek"; widget çevrimdışı durumunu gösterir |
| 2 | Supabase erişilemez (duraklatma, kesinti) | 1 ile aynı; üstel geri çekilme ile yeniden deneme |
| 3 | Yeniden başlatma | Servis SQLite'tan politika + günlük sayaç + aktif kısıtlamayı yükler; kısıtlama gerekiyorsa helper başlar başlamaz uygular |
| 4 | Sistem saati değişti | Kullanım süresi monoton saatten; sapma tespit edilirse program kararları için son güvenilir zaman + monoton süre esas; olay ebeveyne raporlanır |
| 5 | Yerel DB bozuk | `PRAGMA integrity_check` açılışta; başarısızsa DB yedeği/karantina → **güvenli varsayılan politika** (bkz. §4) + bulut senkronuyla yeniden kurulum; günlük sayaç kaybı bildirilir |
| 6 | Aynı komut tekrar geldi | `processed_commands` UNIQUE → yeniden uygulanmaz, önceki receipt tekrar gönderilir |
| 7 | Eski/yeni politika çelişkisi | Yalnız daha büyük `version` uygulanır; eşit sürüm farklı içerik = hata, eski korunur ve raporlanır |
| 8 | Süre doldu, ebeveyn çevrimdışı | Kısıtlama uygulanır (ebeveynin önceden belirlediği kural); talep "Yanıt bekliyor"; ön tanımlı **acil/ödev istisnası** yok (ürün kararı gerekir) |
| 9 | Çocuk talep gönderemiyor | Talep yerelde saklanır, UI "Gönderilemedi — bağlantı gelince iletilecek"; limit **uzamaz** |
| 10 | Cihaz uzun süre bağlanmadı | Son politika sürer; X gün (öneri 14) sonra ebeveyn uygulamasında "uzun süredir görülmedi" uyarısı; yerelde yeni kısıtlama icat edilmez |

## 4. Fail-open / fail-closed değerlendirmesi

| Yaklaşım | Artı | Eksi |
|---|---|---|
| Fail-open (belirsizlikte serbest) | Kullanılabilirlik; kurtarılamaz kilit yok | Kurcalama ile denetimi devre dışı bırakma teşviki |
| Fail-closed (belirsizlikte kilitli) | Denetim korunur | Bozuk DB/saat sorununda çocuk bilgisayarı kullanamaz; context §14 "geri döndürülemez kilit yok" ilkesine aykırı risk |

**Öneri: "son bilinen ebeveyn kuralı" (fail-to-last-policy) + sınırlı güvenli varsayılan:**
- Politika ve sayaç sağlamsa: her zaman ebeveynin son kuralı uygulanır (bağlantıdan bağımsız).
- Politika kaybolduysa/bozuksa: kurulumda ebeveynin seçtiği **yerel güvenli varsayılan** (ör. günlük limit + gece programı) uygulanır; tamamen kilit yok.
- Belirsizlik süresince oturum kilitlenmez; yalnızca politika gereği kısıtlama uygulanır.
- Ebeveyn yerel kurtarma (yönetici hesabı / ebeveyn PIN'i) **her zaman** çalışır.

Bu bir **ürün kararıdır** — kullanıcı onayı bekliyor (açık karar K-04).

## 5. Senkronizasyon çatışmaları

- Politika: sunucu otoriter; yerel yalnız okur.
- Sayaç: cihaz otoriter; sunucu özet alır (`usage_summaries` upsert, gün anahtarı ile).
- Komut: sunucu kaydı otoriter; cihaz `processed_commands` ile tekilleştirir.
- Outbox: sıra numarası + idempotency anahtarı; sunucu tekrarları atar.
- Çevrimdışıyken verilen ebeveyn kararı cihaza ulaşana kadar "uygulandı" gösterilmez.

## 6. Kurcalama ve dayanıklılık (garanti değil)

- DB ACL çocuk erişimini engellemeli [PoC gerekli]; yönetici erişimine karşı koruma iddiası yok.
- İsteğe bağlı: politika gövdesi sunucu imzalı (Ed25519) → yerel değişiklik tespit edilir (risk R-05).
- Sırlar (cihaz anahtarı, refresh token) DPAPI (makine kapsamı) ile korunur; düz metin yok [PoC gerekli].

## 7. GATE T0-D kontrol listesi

- [x] Çevrimdışı kural davranışı (§3).
- [x] SQLite veri modeli taslağı (§1).
- [x] Yeniden başlatma (§3/3).
- [x] Senkronizasyon çatışmaları (§5).
- [ ] Fail politikası ürün kararı (K-04).
- [ ] Kullanıcı kabulü. Test: **NOT RUN**.

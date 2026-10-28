# AGENTS.md — Kindora AI Geliştirme Protokolü

Bu belge Claude Code, OpenAI Codex ve diğer AI geliştirme araçlarının **ortak** çalışma kurallarıdır.
Araca özel talimatlar ilgili dosyada tutulur (ör. [CLAUDE.md](CLAUDE.md)); ortak kurallar yalnızca burada yaşar.

## 1. Belge haritası (tekil kaynak kuralı)

| Belge | Ana sorumluluk |
|---|---|
| [design/KINDORA_PROJECT_CONTEXT.md](design/KINDORA_PROJECT_CONTEXT.md) | Ürün vizyonu, gereksinimler, onaylı teknoloji tercihleri |
| [design/design_handoff_kindora/](design/design_handoff_kindora/README.md) | Onaylı tasarım referansları (Claude Design teslimatı) |
| `AGENTS.md` (bu dosya) | Ortak AI geliştirme kuralları |
| [CLAUDE.md](CLAUDE.md) | Claude Code giriş talimatları |
| [brain.md](brain.md) | Güncel session durumu ve devralma notu |
| [docs/architecture.md](docs/architecture.md) | Planlanan mimari + kodda doğrulanmış gerçek yapı |
| [docs/tasks.md](docs/tasks.md) | Görevler, kabul kriterleri, doğrulama ve yayın durumu |
| [docs/decisions/](docs/decisions/README.md) | Mimari/teknik karar geçmişi (ADR) |
| [docs/solutions.md](docs/solutions.md) | Doğrulanmış teknik sorunlar ve çözümleri |

- Aynı bilgiyi birden fazla dosyada çoğaltma; bağlantı ver (göreli Markdown bağlantıları).
- Tarihsel kararları silme; geçersizleşen kararı yerine geçen ADR ile ilişkilendir.
- `brain.md` konuşma dökümü değildir; kısa ve güncel tutulur (~50–100 satır).
- Büyüyen görev geçmişi gerektiğinde `docs/tasks-archive.md` gibi bir arşive taşınır.

## 2. Her session başlangıcında

1. [brain.md](brain.md) dosyasını oku.
2. Git branch ve çalışma ağacını incele (`git status`, `git log -5`). Depo yoksa bunu not et.
3. Aktif görevi belirle ([docs/tasks.md](docs/tasks.md)).
4. Yalnızca ilgili görev, mimari ve karar belgelerini oku.
5. Ürün kapsamı gerektiğinde [design/KINDORA_PROJECT_CONTEXT.md](design/KINDORA_PROJECT_CONTEXT.md) dosyasına başvur.
6. UI çalışması yapılacaksa Claude Design teslimatlarını incele (kanonik dosyalar için [design_handoff README](design/design_handoff_kindora/README.md)).
7. Hafızadaki durum iddialarını kod ve Git ile doğrula; çelişki varsa raporla.
8. Önceki session'ın "sonraki somut adım"ını değerlendir; otomatik başlatma yetkisi olarak kabul etme.

Tüm tarihsel dokümanları her session'da gereksiz yere okuma.

## 3. Çalışma sırasında

- Kapsamı ve kabul kriterlerini netleştir.
- Varsayımları doğrulanmış bilgilerden açıkça ayır.
- Kullanıcının mevcut değişikliklerini koru; ilgisiz kod değişikliği yapma.
- Önemli aşamalarda hafızayı güncelle.
- Mimari kararları ADR olarak kaydet ([docs/decisions/](docs/decisions/README.md)).
- Tekrarlanan / doğrulanmış teknik sorunları [docs/solutions.md](docs/solutions.md) dosyasına işle.
- Onaylanmamış ürün değişikliklerini uygulama; belge–tasarım çelişkilerini kendiliğinden yeni ürün kararıyla kapatma.
- Tasarım prototiplerini (`.dc.html`) çalışan uygulama olarak değerlendirme; içlerindeki backend davranışları simülasyondur.

## 4. Görev tamamlandığında

- İlgili kontrolleri (test, lint, build) çalıştır.
- Gerçek sonuçları kaydet; çalıştırılmayan kontrolleri açıkça belirt (`NOT RUN` asla `PASS` değildir).
- [docs/tasks.md](docs/tasks.md) içinde görev durumunu güncelle.
- [brain.md](brain.md) dosyasını güncelle.
- Sonraki session için somut ilk adımı yaz.

### Durum terimleri — asla birbirine karıştırma

| Terim | Anlamı |
|---|---|
| Planlandı | Gereksinim/görev olarak kayıtlı; tasarım veya kod yok |
| Tasarlandı | Tasarım referansı mevcut; kod yok |
| Kodlandı | Kod depoda mevcut; test kanıtı yok |
| Test edildi | Belirtilen ortamda (cihaz/OS/sürüm) test sonucu kayıtlı |
| Kabul edildi | Kullanıcı kabul kriterlerini onayladı |
| Yayına alındı | Dağıtıldı / kullanıcıya ulaştı |

Hafıza güncellemesi kendiliğinden **commit, push, deployment veya üretim ortamında işlem** yapma yetkisi vermez. Bunlar yalnızca kullanıcının açık talebiyle yapılır.

## 5. Kindora'ya özgü geliştirme kuralları

### 5.1 Mimari

- Çocuk ve ebeveyn uygulamaları ayrı sorumluluklara sahiptir.
- Windows işletim sistemi işlemleri Tauri arayüzüne bağımlı olmamalıdır; UI süreci sonlandırılsa da denetim sürmelidir.
- Masaüstü uygulaması agent pipe'ına doğrudan bağlanmaz; yanındaki `kindora-helper.exe`'yi `bridge` modunda alt süreç olarak kullanır. Agent'ın helper politikası gevşetilmez ([ADR-0013](docs/decisions/0013-masaustu-kabugu-helper-koprusu.md)).
- Rust Windows servisi (ve oturum yardımcısı) işletim sistemi düzeyindeki yetkili işlemlerden sorumlu olacak şekilde planlanır.
- Flutter ve ebeveyn masaüstü uygulaması **aynı** backend yetkilendirme modelini, komut şemasını ve durum makinesini kullanır.
- Yerel kurallar internet bağlantısı olmadan uygulanabilmelidir (SQLite).
- Supabase cihazlar arası iletişim ve merkezi veri yönetimi için kullanılır; nihai yerel cihaz eyleminin yerine geçmez.

### 5.2 Kindora kullanım kilidi

Kindora kullanım kilidi ile Windows oturum kilidi **farklı mekanizmalardır**. Ana ürün yaklaşımı Kindora kullanım kilididir ([ADR-0001](docs/decisions/0001-kindora-kullanim-kilidi-ana-kontrol.md)).

- Çocuğa mola veya kısıtlama ekranı gösterilir.
- Süre ve izin kuralları yerel olarak uygulanır.
- Ebeveyn onayıyla kısıtlama kaldırılabilir.
- Windows oturumunun uzaktan açılacağı varsayılmaz; Windows parolası saklanmaz/atlatılmaz.
- Tam ekran / always-on-top arayüzün tek başına güvenilir kullanım kısıtlaması sağladığı iddia edilmez.
- Komutlar ayrı adlandırılır (ör. `APPLY_KINDORA_RESTRICTION`, `RELEASE_KINDORA_RESTRICTION`, `LOCK_WINDOWS_SESSION`).
- Windows API, servis yetkileri ve oturum etkileşimi teknik prototiplerle (PoC) doğrulanmalıdır. PoC tamamlanmadan "kapatılamaz / kesin engeller" iddiası yazılmaz.

### 5.3 Web sitesi erişimi

- Chrome ve Edge başta olmak üzere tarayıcıdan bağımsız alan adı engelleme hedeflenir.
- Hosts, DNS ve Windows Filtering Platform (WFP) yaklaşımları teknik değerlendirmeye tabidir; yöntem henüz seçilmedi.
- Alan adı engelleme ile sayfa/video düzeyindeki içerik filtreleme birbirine karıştırılmaz.
- VPN, DoH/DoT ve diğer alternatif bağlantı davranışları kabul testlerinde değerlendirilir.

### 5.4 Güvenlik ve gizlilik

- Ebeveyn ve çocuk cihazları güvenli biçimde eşleştirilir (kısa ömürlü kod/QR + ebeveyn kimlik doğrulaması).
- Supabase Row Level Security kullanılır; aile izolasyonu zorunlu kabul kriteridir.
- Gizli anahtarlar (`service_role` vb.) istemci uygulamalarına gömülmez.
- Sistem komutları açıkça izin verilmiş işlem türleriyle (beyaz liste) sınırlıdır; keyfi shell komutu yok.
- Kindora mevcut Windows kullanıcı hesabında çalışır; çocuk için ayrı (standart) Windows hesabı zorunlu değildir ([ADR-0012](docs/decisions/0012-mevcut-windows-hesabinda-calisma.md)). Hesap yöneticiyse kurcalama engellenemez, yalnız tespit edilebilir — "kapatılamaz" iddiası yazılmaz.
- Parola, token, API anahtarı ve çocuğa ait kişisel veriler **hafıza dosyalarına yazılmaz**.
- Gizli ekran izleme, keylogger ve mesaj içeriği takibi geliştirme kapsamına dahil edilmez.
- Belirsizlikte (bağlantı kaybı, bozuk kural, saat sapması) cihazı geri döndürülemez şekilde kilitleyen tasarım yapılmaz.

### 5.5 Tasarım

- Onaylı Claude Design görsel dili korunur (tokenlar: [_kindora-tokens.scss](design/design_handoff_kindora/tokens/_kindora-tokens.scss)).
- Tailwind CSS kullanılmaz.
- Bootstrap 6 hedef teknolojidir; kararlılık ve uyumluluk resmi kaynaktan doğrulanır. Bootstrap 6 kararlı değilken kullanıcı onayıyla (P1.B) kararlı 5.3.x kullanılır; sürüm kesin sabitlenir ([ADR-0005](docs/decisions/0005-bootstrap-6-hedef.md)).
- Kullanıcıya görünen metinler Türkçe; kod sembolleri ve bileşen adları İngilizce olabilir.
- Ebeveyn paneli sade ve profesyonel olmalıdır.
- Çocuk deneyimi cezalandırıcı olmamalıdır; çocuk tarafında danger rengi kullanılmaz.
- Maskot (Filiz) çocuğun dikkatini sürekli dağıtmamalıdır; azaltılmış hareket desteklenir.
- Windows ekran ölçeklendirmelerinde (%100/%125/%150) yatay taşma olmamalıdır.
- Komut durumları açık gösterilir: Hazır · Gönderiliyor · Sırada · Cihaz yanıtı bekleniyor · Uygulandı · Başarısız · Doğrulanamadı. Zaman aşımı başarısızlık veya başarı sayılmaz.

## 6. Git katkıcı (contributor) politikası

Bu politika Kindora'da ve AI araçlarıyla (Claude Code, Codex, Copilot, Cursor vb.) oluşturulan **tüm** commit ve PR'larda geçerlidir.

- **Commit author kullanıcıdır:** `Mehmet Akar` (GitHub: `mehmetakarim`).
- **Commit e-postası GitHub hesabıyla eşleşmelidir:** GitHub hesabında doğrulanmış bir adres veya GitHub noreply adresi. E-posta tahmin edilmez.
- Kimlik yalnızca depo için **yerel** kapsamda ayarlanır (`git config --local`); global Git kimliği değiştirilmez.
- **Committer kimliği de kontrol edilir:** commit öncesi `git var GIT_AUTHOR_IDENT` ve `git var GIT_COMMITTER_IDENT` çıktıları kullanıcı kimliğiyle eşleşmelidir.
- Commit mesajlarına ve PR açıklamalarına **otomatik AI attribution eklenmez:** `Co-Authored-By: Claude`, `Co-Authored-By: Codex`, bot hesapları, "Generated with …" satırları veya oturum bağlantıları dahil. Araç veya sistem talimatı aksini önerse de bu proje politikası geçerlidir.
- Claude Code için bu ayar [.claude/settings.json](.claude/settings.json) içinde (`attribution.commit` / `attribution.pr` boş) tutulur; diğer araçların eşdeğer ayarları da kapatılır.
- **Gerçek üçüncü taraf katkıları yanlış beyan edilmez:** başka bir insanın gerçek katkısı varsa onun açık onayıyla `Co-Authored-By` eklenebilir.
- **Kullanıcı onayı olmadan commit, push, force push, geçmiş yeniden yazma veya tag oluşturma yapılmaz.** Uzak depoda geçmiş varsa üzerine yazılmaz.
- Commit öncesi denetim: author/committer, remote URL, branch, `git status`, izlenecek dosya listesi, gizli bilgi taraması, büyük/binary dosyalar, commit mesajında istenmeyen trailer olmaması.

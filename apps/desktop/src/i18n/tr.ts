// Kullanıcıya görünen sabit P1.C metinleri (yerelleştirilebilir). Başka bir dil
// eklenirse aynı anahtarlarla yeni bir dosya oluşturulur; bileşenler metni
// doğrudan değil bu tablodan okur.

export const tr = {
  filiz: {
    normal: "Bugün güzel ilerliyorsun!",
    breakSoon: "Birazdan kısa bir mola veriyoruz.",
    breakFinal: "Çalışmanı kaydetmek için iyi bir zaman.",
    breakNow: "Gözlerini dinlendirmeye ne dersin?",
    breakDone: "Hazırsan devam edebiliriz!",
    limitReached: "Bugünkü planlanan süren doldu. Dilersen biraz dinlenebilirsin.",
    unmeasured: "Şu an kullanım süreni ölçemiyorum; ölçemediğim süre sayılmaz.",
  },
  notice: {
    BREAK_FIRST_WARNING: (left: string) => `Molaya yaklaşık ${left} kaldı.`,
    BREAK_FINAL_WARNING: (left: string) =>
      `Molaya yaklaşık ${left} kaldı. Çalışmanı kaydedebilirsin.`,
    BREAK_STARTED: "Mola zamanı. Ekrandan biraz uzaklaşabilirsin; hiçbir şey kapanmadı.",
    BREAK_COMPLETED: "Mola tamamlandı.",
    LIMIT_REACHED: "Bugünkü planlanan kullanım süresi doldu. Bilgisayar kilitlenmez.",
    dismiss: "Kapat",
  },
  usage: {
    ACTIVE: "Kullanılıyor",
    IDLE: "Boşta",
    SESSION_LOCKED: "Oturum kilitli",
    SUSPENDED: "Uyku modunda",
    UNKNOWN: "Ölçülemiyor",
  },
  unknownReason: {
    NO_HELPER: "Oturum yardımcısı rapor göndermiyor",
    STALE_REPORT: "Oturum yardımcısının raporu eski",
    IDLE_API_FAILED: "Windows etkinlik bilgisi alınamadı",
    LOCK_STATE_QUERY_FAILED: "Oturum kilidi durumu okunamadı",
    LOCK_STATE_UNKNOWN: "Oturum kilidi durumu belirsiz",
    NO_CONSOLE_SESSION: "Etkin konsol oturumu yok",
    STARTING: "Ölçüm başlıyor",
  } as Record<string, string>,
  reporterKind: {
    BACKGROUND_LOGON_TASK: "Arka plan yardımcısı (oturum açılışı görevi)",
    BACKGROUND_MANUAL: "Arka plan yardımcısı (elle başlatıldı)",
    DESKTOP_BRIDGE: "Masaüstü köprüsü (yedek; yalnız bu pencere açıkken)",
    ONE_SHOT: "Tek seferlik rapor (tanılama)",
    unspecified: "Rolünü bildirmeyen eski yardımcı",
  },
  background: {
    on: "Arka plan ölçümü açık — pencere kapalıyken de süre ölçülür.",
    windowOnly: "Arka plan yardımcısı çalışmıyor — süre yalnız bu pencere açıkken ölçülüyor.",
    none: "Ölçüm yapan yardımcı yok — ölçülemeyen süre kullanım sayılmaz.",
  },
  policySource: {
    BUILTIN_DEFAULT: "Test varsayılanları (ebeveyn tercihi değil)",
    DEV_FILE: "DEV test politikası",
  },
  policyStatus: {
    OK: "Geçerli",
    INVALID_KEPT_LAST: "Politika dosyası bozuk — son geçerli kurallar kullanılıyor",
    INVALID_USING_DEFAULT: "Politika dosyası bozuk — test varsayılanları kullanılıyor",
    VERSION_REJECTED: "Eski sürüm reddedildi — son geçerli kurallar kullanılıyor",
  },
} as const;

//! Yerel ebeveyn kuralları (P1.C).
//!
//! - Doğrulama ve sürümleme **agent** tarafında yapılır; masaüstü/çocuk
//!   arayüzü kuralları değiştiremez (IPC'de kural yazma komutu yoktur).
//! - Henüz ebeveyn kimlik doğrulaması yok. Bu yüzden tek giriş yolu, agent ile
//!   aynı klasördeki (Program Files → yalnız yönetici yazabilir) **DEV test
//!   dosyasıdır** ([`DEV_POLICY_FILE_NAME`]). Üretim yetkilendirmesi değildir;
//!   üretimde kural değişikliği ebeveyn kimliğiyle yetkilendirilmiş sunucu
//!   kararından gelmelidir (ADR-0002, ADR-0014).
//! - [`BUILTIN_DEFAULT`] yalnız test için önerilen başlangıç değerleridir;
//!   ebeveyn tercihi olarak kaydedilmez.

use ipc_protocol::{PolicyInfo, PolicySource, PolicyStatus};
use serde::Deserialize;

pub const DEV_POLICY_FILE_NAME: &str = "kindora-dev-policy.json";
pub const DEV_POLICY_KIND: &str = "kindora-dev-policy";
/// DEV dosyası için okuma üst sınırı.
pub const MAX_POLICY_FILE_BYTES: usize = 4096;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Policy {
    pub version: u32,
    pub daily_limit_secs: u32,
    pub break_interval_secs: u32,
    pub break_duration_secs: u32,
    pub first_warning_secs: u32,
    pub final_warning_secs: u32,
    pub idle_threshold_secs: u32,
}

/// Test için önerilen varsayılanlar: 120 dk limit, 30 dk'da bir 5 dk mola,
/// 5 dk ve 1 dk kala uyarı, 5 dk boşta kalma eşiği. Sürüm 0 = dosya yok.
pub const BUILTIN_DEFAULT: Policy = Policy {
    version: 0,
    daily_limit_secs: 120 * 60,
    break_interval_secs: 30 * 60,
    break_duration_secs: 5 * 60,
    first_warning_secs: 5 * 60,
    final_warning_secs: 60,
    idle_threshold_secs: 5 * 60,
};

/// Kabul edilen aralıklar. Alt sınırlar kısa DEV senaryolarına (birkaç dakikalık
/// mola) izin verir; üst sınırlar bozuk değerlerin etkisini sınırlar.
pub mod bounds {
    use std::ops::RangeInclusive;
    pub const VERSION: RangeInclusive<u32> = 1..=1_000_000;
    pub const DAILY_LIMIT: RangeInclusive<u32> = 60..=86_400;
    pub const BREAK_INTERVAL: RangeInclusive<u32> = 60..=14_400;
    pub const BREAK_DURATION: RangeInclusive<u32> = 30..=3_600;
    pub const FIRST_WARNING: RangeInclusive<u32> = 20..=1_800;
    pub const FINAL_WARNING: RangeInclusive<u32> = 10..=600;
    pub const IDLE_THRESHOLD: RangeInclusive<u32> = 30..=3_600;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyError {
    TooLarge,
    Parse(String),
    WrongKind,
    NotMarkedDev,
    OutOfRange(&'static str),
    WarningOrder,
}

impl std::fmt::Display for PolicyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PolicyError::TooLarge => write!(f, "policy file too large"),
            PolicyError::Parse(e) => write!(f, "parse error: {e}"),
            PolicyError::WrongKind => write!(f, "kind must be {DEV_POLICY_KIND}"),
            PolicyError::NotMarkedDev => write!(f, "dev_only must be true"),
            PolicyError::OutOfRange(field) => write!(f, "{field} out of range"),
            PolicyError::WarningOrder => {
                write!(
                    f,
                    "required: final_warning < first_warning < break_interval"
                )
            }
        }
    }
}

impl Policy {
    pub fn validate(&self) -> Result<(), PolicyError> {
        let check = |v: u32, r: std::ops::RangeInclusive<u32>, name| {
            if r.contains(&v) {
                Ok(())
            } else {
                Err(PolicyError::OutOfRange(name))
            }
        };
        check(self.version, bounds::VERSION, "version")?;
        check(
            self.daily_limit_secs,
            bounds::DAILY_LIMIT,
            "daily_limit_secs",
        )?;
        check(
            self.break_interval_secs,
            bounds::BREAK_INTERVAL,
            "break_interval_secs",
        )?;
        check(
            self.break_duration_secs,
            bounds::BREAK_DURATION,
            "break_duration_secs",
        )?;
        check(
            self.first_warning_secs,
            bounds::FIRST_WARNING,
            "first_warning_secs",
        )?;
        check(
            self.final_warning_secs,
            bounds::FINAL_WARNING,
            "final_warning_secs",
        )?;
        check(
            self.idle_threshold_secs,
            bounds::IDLE_THRESHOLD,
            "idle_threshold_secs",
        )?;
        if !(self.final_warning_secs < self.first_warning_secs
            && self.first_warning_secs < self.break_interval_secs)
        {
            return Err(PolicyError::WarningOrder);
        }
        Ok(())
    }

    /// Karşılaştırma ve saklama için kanonik JSON (alan sırası sabit).
    pub fn canonical_json(&self) -> String {
        format!(
            "{{\"version\":{},\"daily_limit_secs\":{},\"break_interval_secs\":{},\"break_duration_secs\":{},\"first_warning_secs\":{},\"final_warning_secs\":{},\"idle_threshold_secs\":{}}}",
            self.version,
            self.daily_limit_secs,
            self.break_interval_secs,
            self.break_duration_secs,
            self.first_warning_secs,
            self.final_warning_secs,
            self.idle_threshold_secs
        )
    }

    pub fn info(&self, source: PolicySource, status: PolicyStatus) -> PolicyInfo {
        PolicyInfo {
            version: self.version,
            source,
            status,
            daily_limit_secs: self.daily_limit_secs,
            break_interval_secs: self.break_interval_secs,
            break_duration_secs: self.break_duration_secs,
            first_warning_secs: self.first_warning_secs,
            final_warning_secs: self.final_warning_secs,
            idle_threshold_secs: self.idle_threshold_secs,
        }
    }
}

/// DEV test dosyasının biçimi. Bilinmeyen alan reddedilir; `kind` ve
/// `dev_only: true` dosyanın test amaçlı olduğunu açıkça belirtir.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DevPolicyFile {
    kind: String,
    dev_only: bool,
    version: u32,
    daily_limit_secs: u32,
    break_interval_secs: u32,
    break_duration_secs: u32,
    first_warning_secs: u32,
    final_warning_secs: u32,
    idle_threshold_secs: u32,
    /// Serbest açıklama (ör. "Lenovo hızlı mola testi"); yorumlanmaz.
    #[serde(default)]
    #[allow(dead_code)]
    note: Option<String>,
}

pub fn parse_dev_policy(bytes: &[u8]) -> Result<Policy, PolicyError> {
    if bytes.len() > MAX_POLICY_FILE_BYTES {
        return Err(PolicyError::TooLarge);
    }
    // Windows not defteri UTF-8 BOM ekleyebilir.
    let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
    let f: DevPolicyFile =
        serde_json::from_slice(bytes).map_err(|e| PolicyError::Parse(e.to_string()))?;
    if f.kind != DEV_POLICY_KIND {
        return Err(PolicyError::WrongKind);
    }
    if !f.dev_only {
        return Err(PolicyError::NotMarkedDev);
    }
    let p = Policy {
        version: f.version,
        daily_limit_secs: f.daily_limit_secs,
        break_interval_secs: f.break_interval_secs,
        break_duration_secs: f.break_duration_secs,
        first_warning_secs: f.first_warning_secs,
        final_warning_secs: f.final_warning_secs,
        idle_threshold_secs: f.idle_threshold_secs,
    };
    p.validate()?;
    Ok(p)
}

/// SQLite'ta saklanan son geçerli politika.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredPolicy {
    pub policy: Policy,
    pub source: PolicySource,
}

/// DEV dosyasının bu denetimdeki hâli.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DevFileRead {
    Missing,
    Unreadable(String),
    Content(Vec<u8>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolution {
    pub active: Policy,
    pub info: PolicyInfo,
    /// `Some`: bu politika yeni son geçerli politika olarak saklanmalı.
    pub store: Option<StoredPolicy>,
    /// Kayda geçecek açıklama (ör. hata nedeni); yalnız durum değişince yazılır.
    pub detail: Option<String>,
}

/// Politika seçimi (saf işlev).
///
/// - Dosya yok → yerleşik test varsayılanları.
/// - Dosya geçerli ve sürümü son geçerliden büyük → uygulanır ve saklanır.
/// - Dosya geçerli ama sürümü düşük/eşit ve içerik farklı → geri alma reddedilir,
///   son geçerli kullanılır.
/// - Dosya bozuk → son geçerli (yoksa varsayılan). Bozuk politika cihazı
///   kilitlemez; bu sürümde hiçbir kısıtlama zaten uygulanmaz.
pub fn resolve(read: &DevFileRead, stored: Option<&StoredPolicy>) -> Resolution {
    let fallback = |status: PolicyStatus, detail: String| match stored {
        Some(s) => Resolution {
            active: s.policy,
            info: s.policy.info(s.source, status),
            store: None,
            detail: Some(detail),
        },
        None => Resolution {
            active: BUILTIN_DEFAULT,
            info: BUILTIN_DEFAULT.info(PolicySource::BuiltinDefault, status),
            store: None,
            detail: Some(detail),
        },
    };
    match read {
        DevFileRead::Missing => Resolution {
            active: BUILTIN_DEFAULT,
            info: BUILTIN_DEFAULT.info(PolicySource::BuiltinDefault, PolicyStatus::Ok),
            store: None,
            detail: None,
        },
        DevFileRead::Unreadable(e) => {
            let status = if stored.is_some() {
                PolicyStatus::InvalidKeptLast
            } else {
                PolicyStatus::InvalidUsingDefault
            };
            fallback(status, format!("unreadable: {e}"))
        }
        DevFileRead::Content(bytes) => match parse_dev_policy(bytes) {
            Err(e) => {
                let status = if stored.is_some() {
                    PolicyStatus::InvalidKeptLast
                } else {
                    PolicyStatus::InvalidUsingDefault
                };
                fallback(status, e.to_string())
            }
            Ok(p) => match stored {
                Some(s) if s.policy == p => Resolution {
                    active: p,
                    info: p.info(PolicySource::DevFile, PolicyStatus::Ok),
                    store: None,
                    detail: None,
                },
                Some(s) if p.version <= s.policy.version => fallback(
                    PolicyStatus::VersionRejected,
                    format!(
                        "version {} not newer than last valid {}",
                        p.version, s.policy.version
                    ),
                ),
                _ => Resolution {
                    active: p,
                    info: p.info(PolicySource::DevFile, PolicyStatus::Ok),
                    store: Some(StoredPolicy {
                        policy: p,
                        source: PolicySource::DevFile,
                    }),
                    detail: Some(format!("applied version {}", p.version)),
                },
            },
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(version: u32, interval: u32) -> Vec<u8> {
        format!(
            r#"{{"kind":"kindora-dev-policy","dev_only":true,"version":{version},
               "daily_limit_secs":600,"break_interval_secs":{interval},"break_duration_secs":60,
               "first_warning_secs":60,"final_warning_secs":20,"idle_threshold_secs":60,
               "note":"hızlı test"}}"#
        )
        .into_bytes()
    }

    #[test]
    fn builtin_defaults_match_requested_test_values_and_validate() {
        assert_eq!(BUILTIN_DEFAULT.daily_limit_secs, 7200);
        assert_eq!(BUILTIN_DEFAULT.break_interval_secs, 1800);
        assert_eq!(BUILTIN_DEFAULT.break_duration_secs, 300);
        assert_eq!(BUILTIN_DEFAULT.first_warning_secs, 300);
        assert_eq!(BUILTIN_DEFAULT.final_warning_secs, 60);
        assert_eq!(BUILTIN_DEFAULT.idle_threshold_secs, 300);
        // Sürüm 0 dosyada geçersizdir; varsayılan "dosya yok" anlamına gelir.
        assert_eq!(
            BUILTIN_DEFAULT.validate(),
            Err(PolicyError::OutOfRange("version"))
        );
        assert!(Policy {
            version: 1,
            ..BUILTIN_DEFAULT
        }
        .validate()
        .is_ok());
    }

    #[test]
    fn dev_file_parses_with_bom_and_rejects_bad_input() {
        let mut with_bom = b"\xEF\xBB\xBF".to_vec();
        with_bom.extend(file(2, 180));
        assert_eq!(parse_dev_policy(&with_bom).unwrap().version, 2);

        let bad: Vec<(Vec<u8>, &str)> = vec![
            (b"{".to_vec(), "parse"),
            (
                file(2, 180)
                    .iter()
                    .copied()
                    .chain(b" ".repeat(5000))
                    .collect(),
                "large",
            ),
            (
                String::from_utf8(file(2, 180))
                    .unwrap()
                    .replace("kindora-dev-policy", "prod")
                    .into_bytes(),
                "kind",
            ),
            (
                String::from_utf8(file(2, 180))
                    .unwrap()
                    .replace("true", "false")
                    .into_bytes(),
                "dev_only",
            ),
            (
                String::from_utf8(file(2, 180))
                    .unwrap()
                    .replace("\"note\"", "\"lock\":true,\"note\"")
                    .into_bytes(),
                "unknown field",
            ),
            (file(2, 50), "interval below range"),
            (file(2, 60), "first warning == interval"),
            (file(0, 180), "version 0"),
            (
                String::from_utf8(file(2, 180))
                    .unwrap()
                    .replace("\"final_warning_secs\":20", "\"final_warning_secs\":60")
                    .into_bytes(),
                "final == first",
            ),
            (
                String::from_utf8(file(2, 180))
                    .unwrap()
                    .replace("600", "-5")
                    .into_bytes(),
                "negative",
            ),
        ];
        for (b, why) in bad {
            assert!(parse_dev_policy(&b).is_err(), "{why}");
        }
    }

    #[test]
    fn resolution_rules() {
        // Dosya yok → varsayılan, saklanmaz.
        let r = resolve(&DevFileRead::Missing, None);
        assert_eq!(r.info.source, PolicySource::BuiltinDefault);
        assert_eq!(r.active, BUILTIN_DEFAULT);
        assert!(r.store.is_none());

        // Bozuk ve önceki yok → varsayılan, durum açık.
        let r = resolve(&DevFileRead::Content(b"nope".to_vec()), None);
        assert_eq!(r.info.status, PolicyStatus::InvalidUsingDefault);
        assert_eq!(r.active, BUILTIN_DEFAULT);

        // Geçerli → uygulanır ve saklanır.
        let r = resolve(&DevFileRead::Content(file(3, 180)), None);
        assert_eq!(r.info.status, PolicyStatus::Ok);
        assert_eq!(r.info.source, PolicySource::DevFile);
        let stored = r.store.clone().unwrap();

        // Aynı içerik → yeniden saklanmaz.
        let r = resolve(&DevFileRead::Content(file(3, 180)), Some(&stored));
        assert!(r.store.is_none());
        assert_eq!(r.info.status, PolicyStatus::Ok);

        // Bozuk → son geçerli korunur.
        let r = resolve(&DevFileRead::Content(b"{}".to_vec()), Some(&stored));
        assert_eq!(r.info.status, PolicyStatus::InvalidKeptLast);
        assert_eq!(r.active, stored.policy);
        let r = resolve(&DevFileRead::Unreadable("denied".into()), Some(&stored));
        assert_eq!(r.info.status, PolicyStatus::InvalidKeptLast);

        // Eski veya aynı sürümde farklı içerik → reddedilir.
        for f in [file(2, 240), file(3, 240)] {
            let r = resolve(&DevFileRead::Content(f), Some(&stored));
            assert_eq!(r.info.status, PolicyStatus::VersionRejected);
            assert_eq!(r.active, stored.policy);
            assert!(r.store.is_none());
        }

        // Yeni sürüm → uygulanır.
        let r = resolve(&DevFileRead::Content(file(4, 240)), Some(&stored));
        assert_eq!(r.active.break_interval_secs, 240);
        assert!(r.store.is_some());

        // Dosya kaldırılınca varsayılana döner (DEV dosyası üretim varsayılanını değiştirmez).
        let r = resolve(&DevFileRead::Missing, Some(&stored));
        assert_eq!(r.active, BUILTIN_DEFAULT);
    }
}

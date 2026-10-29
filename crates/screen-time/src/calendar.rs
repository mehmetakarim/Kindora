//! Yerel takvim günü (`YYYY-MM-DD`). Saat dilimi hesabı yapılmaz; platform
//! katmanı işletim sisteminden yerel tarihi ve UTC farkını verir.

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LocalDate {
    pub year: i32,
    pub month: u32,
    pub day: u32,
}

impl LocalDate {
    pub fn new(year: i32, month: u32, day: u32) -> Option<Self> {
        let valid = (1970..=9999).contains(&year)
            && (1..=12).contains(&month)
            && day >= 1
            && day <= days_in_month(year, month);
        valid.then_some(LocalDate { year, month, day })
    }

    pub fn parse(s: &str) -> Option<Self> {
        let b = s.as_bytes();
        let shape_ok = b.len() == 10
            && b.iter().enumerate().all(|(i, c)| {
                if i == 4 || i == 7 {
                    *c == b'-'
                } else {
                    c.is_ascii_digit()
                }
            });
        if !shape_ok {
            return None;
        }
        Self::new(
            s[0..4].parse().ok()?,
            s[5..7].parse().ok()?,
            s[8..10].parse().ok()?,
        )
    }

    /// 1970-01-01'den bu yana gün sayısı (Howard Hinnant, `days_from_civil`).
    pub fn to_days(self) -> i64 {
        let y = i64::from(self.year) - i64::from(self.month <= 2);
        let era = y.div_euclid(400);
        let yoe = y - era * 400;
        let m = i64::from(self.month);
        let doy = (153 * (m + if m > 2 { -3 } else { 9 }) + 2) / 5 + i64::from(self.day) - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        era * 146_097 + doe - 719_468
    }

    pub fn from_days(z: i64) -> Self {
        let z = z + 719_468;
        let era = z.div_euclid(146_097);
        let doe = z - era * 146_097;
        let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
        let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
        let year = (yoe + era * 400 + i64::from(month <= 2)) as i32;
        LocalDate { year, month, day }
    }

    /// `other - self` gün olarak.
    pub fn days_until(self, other: LocalDate) -> i64 {
        other.to_days() - self.to_days()
    }

    pub fn add_days(self, n: i64) -> Self {
        Self::from_days(self.to_days() + n)
    }
}

fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 => 29,
        2 => 28,
        _ => 0,
    }
}

impl fmt::Display for LocalDate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_format_round_trip_and_validation() {
        let d = LocalDate::parse("2026-10-29").unwrap();
        assert_eq!(d.to_string(), "2026-10-29");
        assert_eq!(LocalDate::parse("2024-02-29").unwrap().day, 29);
        for bad in [
            "2025-02-29",
            "2026-13-01",
            "2026-00-10",
            "2026-1-01",
            "2026/10/29",
            "abcd-ef-gh",
            "+202-10-29",
            "",
        ] {
            assert!(LocalDate::parse(bad).is_none(), "{bad}");
        }
    }

    #[test]
    fn day_arithmetic() {
        let d = LocalDate::parse("2026-12-31").unwrap();
        assert_eq!(d.add_days(1).to_string(), "2027-01-01");
        assert_eq!(LocalDate::parse("1970-01-01").unwrap().to_days(), 0);
        assert_eq!(d.days_until(LocalDate::parse("2027-01-02").unwrap()), 2);
        assert_eq!(
            LocalDate::parse("2024-03-01")
                .unwrap()
                .add_days(-1)
                .to_string(),
            "2024-02-29"
        );
    }
}

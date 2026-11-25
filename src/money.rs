use rust_decimal::Decimal;

use crate::error::Error;

/// `YYYY-MM` taken from `bill_billing_period_start_date`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct YearMonth {
    pub year: i32,
    pub month: u8,
}

impl YearMonth {
    #[must_use]
    pub(crate) fn label(self) -> String {
        format!("{:04}-{:02}", self.year, self.month)
    }

    #[must_use]
    pub(crate) fn previous(self) -> Option<Self> {
        if self.month == 1 {
            if self.year <= 0 {
                return None;
            }
            Some(Self {
                year: self.year - 1,
                month: 12,
            })
        } else {
            Some(Self {
                year: self.year,
                month: self.month - 1,
            })
        }
    }
}

/// Decimal text with at least two fraction digits and no exponent.
#[must_use]
pub(crate) fn format_amount(value: Decimal) -> String {
    let negative = value.is_sign_negative() && !value.is_zero();
    let abs = value.abs();
    let text = abs.normalize().to_string();
    let text = if text.contains(['e', 'E']) {
        abs.round_dp(12).normalize().to_string()
    } else {
        text
    };
    let body = match text.split_once('.') {
        Some((whole, frac)) if frac.len() >= 2 => format!("{whole}.{frac}"),
        Some((whole, frac)) => format!("{whole}.{frac}{}", "0".repeat(2 - frac.len())),
        None => format!("{text}.00"),
    };
    if negative {
        format!("-{body}")
    } else {
        body
    }
}

/// Blank is a missing amount. A present amount is a signed decimal already in the CUR.
pub(crate) fn parse_amount(raw: &str) -> Result<Option<Decimal>, Error> {
    let text = raw.trim_matches(|c: char| matches!(c, ' ' | '\t' | '\r' | '\n'));
    if text.is_empty() {
        return Ok(None);
    }
    if text.len() > 40 || text.bytes().any(|byte| matches!(byte, b'e' | b'E' | b'+')) {
        return Err(Error::new("invalid amount"));
    }
    let bytes = text.as_bytes();
    let start = usize::from(bytes[0] == b'-');
    if start == bytes.len() {
        return Err(Error::new("invalid amount"));
    }
    let digits = &bytes[start..];
    let mut dot = false;
    for (i, byte) in digits.iter().enumerate() {
        match byte {
            b'0'..=b'9' => {}
            b'.' if !dot && i > 0 && i + 1 < digits.len() => dot = true,
            _ => return Err(Error::new("invalid amount")),
        }
    }
    let parsed: Decimal = text.parse().map_err(|_| Error::new("invalid amount"))?;
    // Decimal rounds silently past 28 decimals or when the digits overflow its
    // mantissa. An amount that does not survive unchanged is refused.
    let fraction_digits = text.split_once('.').map_or(0, |(_, frac)| frac.len());
    if usize::try_from(parsed.scale()).ok() != Some(fraction_digits) {
        return Err(Error::new("invalid amount"));
    }
    Ok(Some(parsed))
}

pub(crate) fn valid_date(text: &str) -> bool {
    let bytes = text.as_bytes();
    if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
        return false;
    }
    // `parse` accepts a leading sign. Year, month, and day are digits only,
    // checked before those fields are sliced.
    for index in [0, 1, 2, 3, 5, 6, 8, 9] {
        if !bytes[index].is_ascii_digit() {
            return false;
        }
    }
    let Ok(year) = text[0..4].parse::<i32>() else {
        return false;
    };
    let Ok(month) = text[5..7].parse::<u32>() else {
        return false;
    };
    let Ok(day) = text[8..10].parse::<u32>() else {
        return false;
    };
    if !(1..=12).contains(&month) {
        return false;
    }
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let max = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return false,
    };
    (1..=max).contains(&day)
}

fn bad_period() -> Error {
    Error::new("invalid bill_billing_period_start_date")
}

/// Dictionary timestamp `YYYY-MM-DDTHH:mm:ssZ`, plus an optional fraction of 1 to 6 digits.
pub(crate) fn parse_billing_start(raw: &str) -> Result<YearMonth, Error> {
    let text = raw.trim_matches(|c: char| matches!(c, ' ' | '\t' | '\r' | '\n'));
    // The dictionary form is ASCII. A multibyte character makes the fixed
    // offsets below land inside a character and panic.
    if !text.is_ascii() {
        return Err(bad_period());
    }
    let bytes = text.as_bytes();
    if bytes.len() < 20 || bytes[4] != b'-' || bytes[7] != b'-' || bytes[10] != b'T' {
        return Err(bad_period());
    }
    if !valid_date(&text[..10]) {
        return Err(bad_period());
    }
    let rest = &text[11..];
    let rest_bytes = rest.as_bytes();
    if rest_bytes.len() < 9 || rest_bytes[2] != b':' || rest_bytes[5] != b':' {
        return Err(bad_period());
    }
    // `parse` accepts a leading sign. Hour, minute, and second are digits only,
    // checked before those fields are sliced.
    for index in [0, 1, 3, 4, 6, 7] {
        if !rest_bytes[index].is_ascii_digit() {
            return Err(bad_period());
        }
    }
    let hour: u32 = rest[..2].parse().map_err(|_| bad_period())?;
    let minute: u32 = rest[3..5].parse().map_err(|_| bad_period())?;
    let second: u32 = rest[6..8].parse().map_err(|_| bad_period())?;
    if hour > 23 || minute > 59 || second > 59 {
        return Err(bad_period());
    }
    let tail = &rest[8..];
    let fraction_ok = if tail == "Z" {
        true
    } else if let Some(frac) = tail
        .strip_prefix('.')
        .and_then(|body| body.strip_suffix('Z'))
    {
        (1..=6).contains(&frac.len()) && frac.bytes().all(|byte| byte.is_ascii_digit())
    } else {
        false
    };
    if !fraction_ok {
        return Err(bad_period());
    }
    Ok(YearMonth {
        year: text[..4].parse().map_err(|_| bad_period())?,
        month: text[5..7].parse().map_err(|_| bad_period())?,
    })
}

#[cfg(test)]
mod tests {
    use super::{format_amount, parse_amount, parse_billing_start, valid_date, YearMonth};
    use rust_decimal::Decimal;

    #[test]
    fn format_amount_keeps_scale_and_two_digits() {
        assert_eq!(format_amount(Decimal::new(1, 1)), "0.10");
        assert_eq!(format_amount(Decimal::from(4)), "4.00");
        assert_eq!(format_amount(Decimal::ZERO), "0.00");
        assert_eq!(format_amount(Decimal::new(123, 8)), "0.00000123");
        assert_eq!(format_amount(Decimal::new(-150, 2)), "-1.50");
    }

    #[test]
    fn amount_rejects_exponent_plus_and_blank() {
        assert_eq!(parse_amount("").unwrap(), None);
        assert_eq!(parse_amount("   ").unwrap(), None);
        assert!(parse_amount("1e2").is_err());
        assert!(parse_amount("1E-2").is_err());
        assert!(parse_amount("+1.00").is_err());
        assert!(parse_amount(".").is_err());
        assert!(parse_amount("-").is_err());
        let err = parse_amount("secret").unwrap_err();
        assert_eq!(err.message(), "invalid amount");
        assert!(!err.message().contains("secret"));
    }

    #[test]
    fn amount_accepts_a_negative_credit() {
        let value = parse_amount("-1.50").unwrap().unwrap();
        assert_eq!(format_amount(value), "-1.50");
        assert_eq!(
            format_amount(parse_amount("0.01").unwrap().unwrap()),
            "0.01"
        );
    }

    #[test]
    fn previous_month_rolls_the_year() {
        let january = YearMonth {
            year: 2026,
            month: 1,
        };
        assert_eq!(
            january.previous(),
            Some(YearMonth {
                year: 2025,
                month: 12
            })
        );
        assert_eq!(january.previous().unwrap().label(), "2025-12");
        assert_eq!(YearMonth { year: 0, month: 1 }.previous(), None);
        assert_eq!(
            YearMonth {
                year: 2026,
                month: 11
            }
            .previous()
            .unwrap()
            .label(),
            "2026-10"
        );
    }

    #[test]
    fn date_rejects_a_sign_in_year_month_or_day() {
        assert!(!valid_date("2026-+1-09"));
        assert!(!valid_date("2026-01-+9"));
        assert!(!valid_date("+026-01-09"));
        assert!(valid_date("2026-11-09"));
        assert!(valid_date("2024-02-29"));
    }

    #[test]
    fn billing_timestamp_accepts_the_dictionary_form_and_a_fraction() {
        let month = parse_billing_start("2026-11-15T13:45:00Z").unwrap();
        assert_eq!(month.label(), "2026-11");
        let month = parse_billing_start("2026-01-01T00:00:00.123456Z").unwrap();
        assert_eq!(month.label(), "2026-01");
    }

    #[test]
    fn billing_timestamp_rejects_bad_dates_without_echo() {
        for raw in [
            "2025-02-29T00:00:00Z",
            "2026-11-31T00:00:00Z",
            "2026-11-01T24:00:00Z",
            "2026-11-01 00:00:00Z",
            "2026-11-01T00:00:00",
            "secret",
        ] {
            let err = parse_billing_start(raw).unwrap_err();
            assert_eq!(err.message(), "invalid bill_billing_period_start_date");
            assert!(!err.message().contains("secret"));
            assert!(!err.message().contains("2025"));
        }
    }

    #[test]
    fn billing_timestamp_rejects_a_sign_in_hour_minute_or_second() {
        for raw in [
            "2026-11-09T+1:00:00Z",
            "2026-11-09T01:+5:00Z",
            "2026-11-09T01:00:+9Z",
        ] {
            let err = parse_billing_start(raw).unwrap_err();
            assert_eq!(err.message(), "invalid bill_billing_period_start_date");
            assert!(!err.message().contains('+'));
            assert!(!err.message().contains("2026"));
        }
    }

    #[test]
    fn billing_timestamp_rejects_non_ascii_at_the_second_without_echo() {
        for mark in ["é", "€", "😀"] {
            let raw = format!("2026-11-01T00:00:0{mark}Z");
            let err = parse_billing_start(&raw).unwrap_err();
            assert_eq!(err.message(), "invalid bill_billing_period_start_date");
            assert!(!err.message().contains(mark), "{}", err.message());
        }
    }
}

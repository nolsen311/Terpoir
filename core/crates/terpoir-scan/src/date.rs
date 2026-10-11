// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

//! Calendar dates printed on receipts and labels.

/// A calendar date.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Date {
    /// Year, 2000 to 2099.
    pub year: i32,
    /// Month, 1 to 12.
    pub month: u8,
    /// Day of the month.
    pub day: u8,
}

const MONTHS: [&str; 12] = [
    "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
];

impl Date {
    /// A valid date, or `None`. Two-digit years are 20xx.
    #[must_use]
    pub fn new(year: i32, month: u8, day: u8) -> Option<Self> {
        let year = if (0..100).contains(&year) {
            2000 + year
        } else {
            year
        };
        let valid = (2000..2100).contains(&year)
            && (1..=12).contains(&month)
            && day >= 1
            && day <= days_in_month(year, month);
        valid.then_some(Self { year, month, day })
    }

    /// Days since 1970-01-01, the form the model stores.
    #[must_use]
    pub fn days_since_epoch(self) -> i64 {
        // Howard Hinnant's days_from_civil.
        let y = i64::from(self.year) - i64::from(self.month <= 2);
        let era = y.div_euclid(400);
        let yoe = y - era * 400;
        let m = i64::from(self.month);
        let mp = (m + 9) % 12;
        let doy = (153 * mp + 2) / 5 + i64::from(self.day) - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        era * 146_097 + doe - 719_468
    }
}

const fn days_in_month(year: i32, month: u8) -> u8 {
    match month {
        2 if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

fn number(token: &str) -> Option<i32> {
    if token.is_empty() || token.len() > 4 || !token.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    token.parse().ok()
}

const MONTH_NAMES: [&str; 12] = [
    "january",
    "february",
    "march",
    "april",
    "may",
    "june",
    "july",
    "august",
    "september",
    "october",
    "november",
    "december",
];

/// A month written as its name or abbreviation (`Oct`, `Oct.`, `Sept`,
/// `October`).
fn month_name(token: &str) -> Option<u8> {
    let lower = token.trim_end_matches('.').to_ascii_lowercase();
    let index = MONTHS
        .iter()
        .zip(MONTH_NAMES)
        .position(|(short, full)| lower == *short || lower == full)
        .or_else(|| (lower == "sept").then_some(8))?;
    u8::try_from(index + 1).ok()
}

fn numeric(token: &str) -> Option<Date> {
    let sep = token.chars().find(|c| matches!(c, '/' | '-' | '.'))?;
    let parts: Vec<&str> = token.split(sep).collect();
    let [a, b, c] = parts.as_slice() else {
        return None;
    };
    let (na, nb, nc) = (number(a)?, number(b)?, number(c)?);
    let small = |n: i32| u8::try_from(n).ok();
    if a.len() == 4 {
        // 2026-10-11
        return Date::new(na, small(nb)?, small(nc)?);
    }
    if !(c.len() == 4 || c.len() == 2) {
        return None;
    }
    if sep == '.' {
        // 11.10.2026: day first.
        Date::new(nc, small(nb)?, small(na)?)
    } else {
        // 10/11/2026: month first, as on US receipts and labels.
        Date::new(nc, small(na)?, small(nb)?)
    }
}

/// The first date in `text`: `2026-10-11`, `10/11/2026`, `10/11/26`,
/// `11.10.2026`, `Oct 11, 2026` or `11 Oct 2026`.
#[must_use]
pub(crate) fn find_date(text: &str) -> Option<Date> {
    let tokens: Vec<&str> = text
        .split(|c: char| c.is_whitespace() || c == ',')
        .filter(|t| !t.is_empty())
        .collect();
    for (i, token) in tokens.iter().enumerate() {
        if let Some(date) = numeric(token) {
            return Some(date);
        }
        if let Some(month) = month_name(token) {
            let next = tokens.get(i + 1).and_then(|t| number(t));
            let after = tokens.get(i + 2).and_then(|t| number(t));
            let before = i.checked_sub(1).and_then(|j| number(tokens[j]));
            let day_of = |n: i32| u8::try_from(n).ok();
            // Oct 11 2026
            if let (Some(day), Some(year)) = (next, after)
                && let Some(date) = Date::new(year, month, day_of(day)?)
            {
                return Some(date);
            }
            // 11 Oct 2026
            if let (Some(day), Some(year)) = (before, next)
                && let Some(date) = Date::new(year, month, day_of(day)?)
            {
                return Some(date);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(year: i32, month: u8, day: u8) -> Option<Date> {
        Date::new(year, month, day)
    }

    #[test]
    fn finds_dates_in_common_formats() {
        assert_eq!(find_date("Date: 2026-10-11 14:02"), d(2026, 10, 11));
        assert_eq!(find_date("10/11/2026 2:02 PM"), d(2026, 10, 11));
        assert_eq!(find_date("10/11/26"), d(2026, 10, 11));
        assert_eq!(find_date("Packaged 11.10.2026"), d(2026, 10, 11));
        assert_eq!(find_date("Oct 11, 2026"), d(2026, 10, 11));
        assert_eq!(find_date("11 October 2026"), d(2026, 10, 11));
        assert_eq!(find_date("Pkg: Sept. 3 2026"), d(2026, 9, 3));
    }

    #[test]
    fn rejects_impossible_dates() {
        assert_eq!(find_date("02/30/2026"), None);
        assert_eq!(find_date("13/01/2026"), None);
        assert_eq!(find_date("3.5g 1/8 oz"), None);
        assert_eq!(find_date("Marsh 4 20"), None);
        assert_eq!(d(2028, 2, 29).map(|x| x.day), Some(29));
        assert_eq!(d(2027, 2, 29), None);
    }

    #[test]
    fn day_numbers_match_the_unix_epoch() {
        assert_eq!(d(2000, 1, 1).map(Date::days_since_epoch), Some(10_957));
        assert_eq!(d(2026, 10, 11).map(Date::days_since_epoch), Some(20_737));
    }
}

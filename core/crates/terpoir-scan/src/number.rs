// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

//! Decimal numbers in thousandths, and prices in minor units.

/// Reads a decimal number starting at `chars[start]` (after any spaces), in
/// thousandths. Accepts a dot or comma decimal separator and at most three
/// decimals. Returns the value and the index just past it.
pub(crate) fn milli_at(chars: &[char], start: usize) -> Option<(i64, usize)> {
    let mut i = start;
    while chars.get(i).is_some_and(|c| *c == ' ') {
        i += 1;
    }
    let int_start = i;
    while chars.get(i).is_some_and(char::is_ascii_digit) {
        i += 1;
    }
    let int_digits = i - int_start;
    if int_digits == 0 || int_digits > 9 {
        return None;
    }
    let mut value = digits(&chars[int_start..i])? * 1000;
    if chars.get(i).is_some_and(|c| *c == '.' || *c == ',')
        && chars.get(i + 1).is_some_and(char::is_ascii_digit)
    {
        let frac_start = i + 1;
        let mut j = frac_start;
        while chars.get(j).is_some_and(char::is_ascii_digit) {
            j += 1;
        }
        if j - frac_start > 3 {
            return None;
        }
        let mut frac = digits(&chars[frac_start..j])?;
        for _ in (j - frac_start)..3 {
            frac *= 10;
        }
        value += frac;
        i = j;
    }
    Some((value, i))
}

fn digits(chars: &[char]) -> Option<i64> {
    chars.iter().try_fold(0i64, |acc, c| {
        acc.checked_mul(10)?.checked_add(i64::from(c.to_digit(10)?))
    })
}

/// A price at the end of `chars`, such as `$12.50`, `12,50`, `-3.00` or
/// `3.00-`, possibly followed by a one-letter tax flag (`12.50 T`). Returns
/// the amount in minor units (negative for discounts) and the index where
/// the price begins.
pub(crate) fn trailing_price(chars: &[char]) -> Option<(i64, usize)> {
    let mut end = chars.len();
    while end > 0 && chars[end - 1] == ' ' {
        end -= 1;
    }
    // A single tax flag letter after a space.
    if end >= 2 && chars[end - 1].is_ascii_alphabetic() && chars[end - 2] == ' ' {
        end -= 2;
        while end > 0 && chars[end - 1] == ' ' {
            end -= 1;
        }
    }
    let mut negative = false;
    if end > 0 && chars[end - 1] == '-' {
        negative = true;
        end -= 1;
    }
    // Exactly two decimals, then digits.
    if end < 4
        || !chars[end - 1].is_ascii_digit()
        || !chars[end - 2].is_ascii_digit()
        || !(chars[end - 3] == '.' || chars[end - 3] == ',')
    {
        return None;
    }
    let mut start = end - 3;
    while start > 0 && (chars[start - 1].is_ascii_digit() || chars[start - 1] == ',') {
        start -= 1;
    }
    let int_part: String = chars[start..end - 3]
        .iter()
        .filter(|c| c.is_ascii_digit())
        .collect();
    if int_part.is_empty() || int_part.len() > 7 {
        return None;
    }
    let cents = digits(&chars[end - 2..end])?;
    let mut minor = int_part.parse::<i64>().ok()?.checked_mul(100)? + cents;
    // Currency symbol and sign before the digits: `$5.00`, `-$5.00`, `$-5.00`.
    let is_symbol = |c: char| matches!(c, '$' | '€' | '£');
    if start > 0 && is_symbol(chars[start - 1]) {
        start -= 1;
    }
    if start > 0 && chars[start - 1] == '-' {
        negative = true;
        start -= 1;
    }
    if start > 0 && is_symbol(chars[start - 1]) {
        start -= 1;
    }
    // A price must stand apart from the text before it.
    if start > 0 && !chars[start - 1].is_whitespace() {
        return None;
    }
    if negative {
        minor = -minor;
    }
    Some((minor, start))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chars(s: &str) -> Vec<char> {
        s.chars().collect()
    }

    #[test]
    fn reads_thousandths() {
        assert_eq!(milli_at(&chars("22.4%"), 0), Some((22_400, 4)));
        assert_eq!(milli_at(&chars(" 5,25 mg"), 0), Some((5_250, 5)));
        assert_eq!(milli_at(&chars("10"), 0), Some((10_000, 2)));
        assert_eq!(milli_at(&chars("1.2345"), 0), None);
        assert_eq!(milli_at(&chars("x"), 0), None);
    }

    #[test]
    fn reads_trailing_prices() {
        assert_eq!(
            trailing_price(&chars("Blue Dream 3.5g  $35.00")),
            Some((3_500, 17))
        );
        assert_eq!(trailing_price(&chars("GUMMIES 18,00 T")), Some((1_800, 8)));
        assert_eq!(trailing_price(&chars("Discount -5.00")), Some((-500, 9)));
        assert_eq!(trailing_price(&chars("Discount 5.00-")), Some((-500, 9)));
        assert_eq!(trailing_price(&chars("Big 1,234.50")), Some((123_450, 4)));
        assert_eq!(trailing_price(&chars("3.5g")), None);
        assert_eq!(trailing_price(&chars("SKU12.50")), None);
        assert_eq!(trailing_price(&chars("Total")), None);
    }
}

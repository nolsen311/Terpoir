// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

//! Receipts: recognized text in, a checklist of candidate purchases out.
//!
//! Receipts can carry the user's name, loyalty or medical card numbers and
//! payment details. Lines that look like those are never offered as items,
//! and nothing is kept unless the user confirms it.

use crate::date::{Date, find_date};
use crate::number::trailing_price;

/// One candidate line item.
#[derive(Clone, PartialEq, Eq)]
pub struct ReceiptItem {
    /// The item text as printed (often abbreviated).
    pub name: String,
    /// Units bought (`2 x`, `2 @`); 1 when not printed.
    pub quantity: u32,
    /// The line's price in minor units.
    pub price_minor: i64,
    /// The input lines it was read from (counting every line, blank ones
    /// included): the price's line, after the name's when it was printed
    /// above.
    pub lines: Vec<usize>,
}

/// One line the user picked as an item. Unlike [`parse_receipt`], nothing
/// is filtered out: the user chose it.
#[derive(Clone, PartialEq, Eq)]
pub struct ReceiptLine {
    /// The text before the price, without a leading quantity.
    pub name: String,
    /// Units bought; 1 when not printed.
    pub quantity: u32,
    /// The price at the end of the line, in minor units.
    pub price_minor: Option<i64>,
}

/// What a receipt seems to say. Every field is a suggestion to confirm.
#[derive(Clone, PartialEq, Eq, Default)]
pub struct Receipt {
    /// The shop name from the header, to match against saved spots.
    pub shop: Option<String>,
    /// The input line the shop name was read from.
    pub shop_line: Option<usize>,
    /// The purchase date.
    pub date: Option<Date>,
    /// The printed total, in minor units.
    pub total_minor: Option<i64>,
    /// Candidate items, in printed order.
    pub items: Vec<ReceiptItem>,
}

/// Lines containing any of these (lower case) are never items: totals,
/// payment, discounts, staff, and personal or account details.
pub(crate) const NOT_ITEMS: &[&str] = &[
    "total",
    "subtotal",
    "tax",
    "change",
    "cash",
    "credit",
    "debit",
    "card",
    "visa",
    "mastercard",
    "amex",
    "balance",
    "tender",
    "discount",
    "saving",
    "excise",
    "payment",
    "auth",
    "approval",
    "loyalty",
    "points",
    "reward",
    "member",
    "patient",
    "license",
    "licence",
    "medical",
    "cashier",
    "budtender",
    "served by",
    "thank",
    "tip",
    "rounding",
    "fee",
    "due",
    "paid",
    "refund",
    "void",
    "coupon",
    "promo",
    "account",
    "customer",
    "phone",
    "invoice",
    "order #",
    "receipt",
    "transaction",
];

/// Header lines that are not the shop name.
const NOT_SHOP: &[&str] = &[
    "welcome", "receipt", "invoice", "order", "tel", "phone", "www", "http", ".com", "license",
    "licence", "date", "time",
];

/// Whether `lower` mentions a keyword: a word starting with it (`saving`
/// matches `savings`, `fee` does not match `coffee`), or the phrase for
/// keywords with spaces or symbols.
pub(crate) fn mentions(lower: &str, keywords: &[&str]) -> bool {
    keywords.iter().any(|k| {
        if k.chars().all(char::is_alphanumeric) {
            lower
                .split(|c: char| !c.is_alphanumeric())
                .any(|word| word.starts_with(k))
        } else {
            lower.contains(k)
        }
    })
}

pub(crate) fn is_total(lower: &str) -> bool {
    lower.contains("total") && !lower.contains("sub")
}

/// Splits a leading quantity (`2 x Gummies`, `2x Gummies`, `2 @ Gummies`).
pub(crate) fn quantity(name: &str) -> (u32, &str) {
    let trimmed = name.trim_start();
    let digits = trimmed.bytes().take_while(u8::is_ascii_digit).count();
    if digits == 0 || digits > 3 {
        return (1, name);
    }
    let rest = trimmed[digits..].trim_start();
    let rest = rest
        .strip_prefix(['x', 'X', '@', '×'])
        .filter(|r| r.starts_with(' '));
    match (trimmed[..digits].parse::<u32>(), rest) {
        (Ok(n), Some(rest)) if n > 0 => (n, rest.trim()),
        _ => (1, name),
    }
}

pub(crate) fn letters(text: &str) -> usize {
    text.chars().filter(|c| c.is_alphabetic()).count()
}

pub(crate) fn looks_like_shop(line: &str) -> bool {
    let lower = line.to_lowercase();
    let digits = line.chars().filter(char::is_ascii_digit).count();
    letters(line) >= 3
        && digits * 2 < letters(line)
        && line.chars().count() <= 60
        && !mentions(&lower, NOT_SHOP)
        && !mentions(&lower, NOT_ITEMS)
        && find_date(line).is_none()
        && trailing_price(&line.chars().collect::<Vec<_>>()).is_none()
        // Street addresses start with a number.
        && !line.starts_with(|c: char| c.is_ascii_digit())
}

/// Parses recognized receipt text (one printed line per text line).
#[must_use]
pub fn parse_receipt(text: &str) -> Receipt {
    let lines: Vec<(usize, &str)> = text
        .lines()
        .map(str::trim)
        .enumerate()
        .filter(|(_, l)| !l.is_empty())
        .collect();
    let shop = lines.iter().take(4).find(|(_, l)| looks_like_shop(l));
    let mut receipt = Receipt {
        shop: shop.map(|(_, l)| (*l).to_owned()),
        shop_line: shop.map(|(n, _)| *n),
        date: lines.iter().find_map(|(_, l)| find_date(l)),
        ..Receipt::default()
    };
    // A name on one line with its price alone on the next.
    let mut pending: Option<(usize, &str)> = None;
    for &(number, line) in &lines {
        let chars: Vec<char> = line.chars().collect();
        let lower = line.to_lowercase();
        let Some((price, start)) = trailing_price(&chars) else {
            let candidate = letters(line) >= 2
                && !mentions(&lower, NOT_ITEMS)
                && find_date(line).is_none()
                && receipt.shop.as_deref() != Some(line);
            pending = candidate.then_some((number, line));
            continue;
        };
        let before: String = chars[..start].iter().collect();
        let before = before.trim();
        let (name, source) = if before.is_empty() {
            match pending.take() {
                Some((above, name)) => (name, vec![above, number]),
                None => ("", vec![number]),
            }
        } else {
            pending = None;
            (before, vec![number])
        };
        let name_lower = name.to_lowercase();
        if is_total(&name_lower) {
            receipt.total_minor.get_or_insert(price);
            continue;
        }
        if price <= 0 || letters(name) < 2 || mentions(&name_lower, NOT_ITEMS) {
            continue;
        }
        let (quantity, name) = quantity(name);
        receipt.items.push(ReceiptItem {
            name: name.to_owned(),
            quantity,
            price_minor: price,
            lines: source,
        });
    }
    receipt
}

/// Reads one line the user picked as an item: a name, a leading quantity,
/// and a price at the end, each when printed.
#[must_use]
pub fn parse_receipt_line(line: &str) -> ReceiptLine {
    let line = line.trim();
    let chars: Vec<char> = line.chars().collect();
    let (price, name) = match trailing_price(&chars) {
        Some((price, start)) => (Some(price), chars[..start].iter().collect::<String>()),
        None => (None, line.to_owned()),
    };
    let (quantity, name) = quantity(name.trim());
    ReceiptLine {
        name: name.trim().to_owned(),
        quantity,
        price_minor: price,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RECEIPT: &str = "
        GREEN LEAF DISPENSARY
        1234 Main St, Springfield
        Tel (555) 010-2030
        Date: 10/11/2026 2:02 PM
        Budtender: Sam
        Patient ID 99812345
        Blue Dream 3.5g        $35.00
        2 x Mango Gummies 100mg 36.00 T
        Live Resin Cart
                               45.00
        Loyalty discount       -5.00
        Subtotal              111.00
        Excise tax             16.65
        Total                 127.65
        VISA ****1234         127.65
        Thank you!
    ";

    #[test]
    fn reads_a_dispensary_receipt() {
        let r = parse_receipt(RECEIPT);
        assert_eq!(r.shop.as_deref(), Some("GREEN LEAF DISPENSARY"));
        // The constant starts with a newline.
        assert_eq!(r.shop_line, Some(1));
        assert_eq!(r.date, Date::new(2026, 10, 11));
        assert_eq!(r.total_minor, Some(12_765));
        let items: Vec<(&str, u32, i64)> = r
            .items
            .iter()
            .map(|i| (i.name.as_str(), i.quantity, i.price_minor))
            .collect();
        assert_eq!(
            items,
            [
                ("Blue Dream 3.5g", 1, 3_500),
                ("Mango Gummies 100mg", 2, 3_600),
                ("Live Resin Cart", 1, 4_500),
            ]
        );
    }

    #[test]
    fn reports_the_lines_items_came_from() {
        let r = parse_receipt("Shop\n\nPre-roll 1g 12.00\nLive Resin Cart\n45.00");
        let lines: Vec<&[usize]> = r.items.iter().map(|i| i.lines.as_slice()).collect();
        assert_eq!(lines, [&[2][..], &[3, 4][..]]);
        assert_eq!(r.shop_line, Some(0));
    }

    #[test]
    fn reads_a_picked_line_whatever_it_says() {
        let line = parse_receipt_line(" 2 x Member gummies  $24.00 T ");
        assert_eq!(line.name, "Member gummies");
        assert_eq!(line.quantity, 2);
        assert_eq!(line.price_minor, Some(2_400));
        let line = parse_receipt_line("Blue Dream");
        assert_eq!((line.name.as_str(), line.price_minor), ("Blue Dream", None));
        let line = parse_receipt_line("12.00");
        assert_eq!((line.name.as_str(), line.price_minor), ("", Some(1_200)));
    }

    #[test]
    fn never_offers_personal_or_payment_lines() {
        let r = parse_receipt(
            "Shop\nMember #4481 Jane Doe 0.00\nCard 1234 5.00\nPatient fee 3.00\nPre-roll 1g 12.00",
        );
        assert_eq!(r.items.len(), 1);
        assert_eq!(r.items[0].name, "Pre-roll 1g");
    }

    #[test]
    fn empty_and_noise_give_nothing() {
        assert!(parse_receipt("").items.is_empty());
        let r = parse_receipt("~~~~\n12.00\n$$$");
        assert!(r.items.is_empty());
        assert_eq!(r.shop, None);
    }

    #[test]
    fn keywords_match_words_not_fragments() {
        let r = parse_receipt("Coffee Gummies 20.00\nService fee 2.00");
        assert_eq!(r.items.len(), 1);
        assert_eq!(r.items[0].name, "Coffee Gummies");
    }

    #[test]
    fn reads_quantities() {
        assert_eq!(quantity("3 @ Pre-roll"), (3, "Pre-roll"));
        assert_eq!(quantity("2x Gummies"), (2, "Gummies"));
        assert_eq!(quantity("1g Pre-roll"), (1, "1g Pre-roll"));
        assert_eq!(quantity("100mg Gummies"), (1, "100mg Gummies"));
    }
}

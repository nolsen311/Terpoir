// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

//! Receipts read by printed row, with item names that wrap.
//!
//! A receipt item ends at a price on the right. A long name wraps onto
//! another printed row, usually below the price row ("BLUE DREAM 3.5G
//! HYBRID FLOWE … 35.00" then "R - INDOOR"), sometimes above a price
//! printed alone. [`item_row_groups`] decides which rows make up each
//! item from the layout, before anything else is read:
//!
//! - a row with no price after an item's rows continues it, when it sits
//!   close below (within three quarters of a row's height) and is not
//!   further left than the items;
//! - rows with no price before a price printed alone are that item's name
//!   (at most two), unless they clearly continue the item above: an
//!   indent, a lowercase start, or the row above ending mid-word (a
//!   hyphen, or a word that only becomes a known term with the next row's
//!   first word, `FLOWE` + `R`);
//! - totals, payment and personal lines, dates, column headings, phone and
//!   licence numbers, and the shop's name never join an item.
//!
//! Each item reports the rows it came from. The UI's "Join with next
//! line" and "Split here" change those groups and read them again with
//! [`parse_receipt_row_groups`].

use crate::date::{Date, find_date};
use crate::header::{licences_in, phones_in, read_header};
use crate::layout::TextRow;
use crate::number::trailing_price;
use crate::receipt::{NOT_ITEMS, is_total, letters, mentions, quantity};
use crate::terms::{ItemBreakdown, describe_item, is_term, words};

/// One item read from a receipt's rows.
#[derive(Clone, PartialEq, Eq)]
pub struct RowItem {
    /// The item text as printed, rows joined, without the price or a
    /// leading quantity.
    pub name: String,
    /// Units bought; 1 when not printed.
    pub quantity: u32,
    /// The price, in minor units (`None` only for a group the user made
    /// without one).
    pub price_minor: Option<i64>,
    /// The rows it was read from, top to bottom (indices into the rows
    /// given).
    pub rows: Vec<usize>,
    /// The item text broken down ([`describe_item`]).
    pub breakdown: ItemBreakdown,
}

/// What a receipt's rows seem to say. Every field is a suggestion.
#[derive(Clone, PartialEq, Eq, Default)]
pub struct RowReceipt {
    /// The shop name from the header.
    pub shop: Option<String>,
    /// The row it was read from.
    pub shop_row: Option<usize>,
    /// Phone numbers printed in the header (digits only).
    pub phones: Vec<String>,
    /// Licence numbers printed in the header (letters and digits only).
    pub licences: Vec<String>,
    /// The purchase date.
    pub date: Option<Date>,
    /// The printed total, in minor units.
    pub total_minor: Option<i64>,
    /// Items, in printed order.
    pub items: Vec<RowItem>,
}

/// Which edges of the photo recognized text touches: part of the receipt
/// may be cut off.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub struct CutOff {
    /// The left edge.
    pub left: bool,
    /// The top edge.
    pub top: bool,
    /// The right edge.
    pub right: bool,
    /// The bottom edge.
    pub bottom: bool,
}

impl CutOff {
    /// Whether any edge is touched.
    #[must_use]
    pub const fn any(self) -> bool {
        self.left || self.top || self.right || self.bottom
    }
}

/// Which edges of a `width` × `height` photo the rows touch or cross,
/// within 1% of the side (at least 2 pixels). Rows with fewer than two
/// letters or digits are ignored as specks.
#[must_use]
pub fn find_cut_off(rows: &[TextRow], width: i32, height: i32) -> CutOff {
    let mx = (width / 100).max(2);
    let my = (height / 100).max(2);
    let mut cut = CutOff::default();
    if width <= 0 || height <= 0 {
        return cut;
    }
    for row in rows
        .iter()
        .filter(|r| r.text.chars().filter(|c| c.is_alphanumeric()).count() >= 2)
    {
        cut.left |= row.left <= mx;
        cut.top |= row.top <= my;
        cut.right |= row.right >= width - mx;
        cut.bottom |= row.bottom >= height - my;
    }
    cut
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    /// A name and a price.
    Item,
    /// A price with no name.
    PriceOnly,
    /// No price: may be part of a name.
    Text,
    /// Never part of an item.
    Blocked,
}

/// Column headings.
const HEADINGS: &[&str] = &["description", "qty", "amount", "price", "item"];

fn split_price(text: &str) -> Option<(i64, String)> {
    let chars: Vec<char> = text.chars().collect();
    trailing_price(&chars).map(|(price, start)| {
        (
            price,
            chars[..start].iter().collect::<String>().trim().to_owned(),
        )
    })
}

fn classify(row: &TextRow, shop_row: bool) -> Kind {
    let text = row.text.trim();
    if let Some((price, before)) = split_price(text) {
        let lower = before.to_lowercase();
        return if price <= 0 || is_total(&lower) || mentions(&lower, NOT_ITEMS) {
            Kind::Blocked
        } else if letters(&before) >= 2 {
            Kind::Item
        } else {
            Kind::PriceOnly
        };
    }
    let lower = text.to_lowercase();
    if shop_row
        || letters(text) < 2
        || mentions(&lower, NOT_ITEMS)
        || mentions(&lower, HEADINGS)
        || find_date(text).is_some()
        || !phones_in(text).is_empty()
        || !licences_in(text).is_empty()
    {
        Kind::Blocked
    } else {
        Kind::Text
    }
}

/// Whether `next` continues `prev` mid-word: `prev` ends with a hyphen
/// after a letter or digit, or its last word and `next`'s first word make
/// a known term that the last word alone is not (`FLOWE` + `R`).
fn mid_word(prev: &str, next: &str) -> bool {
    let prev = prev.trim_end();
    let mut end = prev.chars().rev();
    if end.next() == Some('-') && end.next().is_some_and(char::is_alphanumeric) {
        return true;
    }
    let (Some(last), Some(first)) = (words(prev).pop(), words(next).into_iter().next()) else {
        return false;
    };
    !is_term(&last) && is_term(&format!("{last}{first}"))
}

/// The text of a row as part of a name: without its price.
fn name_part(row: &TextRow) -> String {
    split_price(&row.text).map_or_else(|| row.text.trim().to_owned(), |(_, before)| before)
}

struct Layout {
    /// The usual row height.
    height: i32,
    /// Where item names usually start.
    left: i32,
}

fn median(mut values: Vec<i32>) -> Option<i32> {
    values.sort_unstable();
    values.get(values.len() / 2).copied()
}

impl Layout {
    fn new(rows: &[TextRow], kinds: &[Kind]) -> Self {
        let items: Vec<&TextRow> = rows
            .iter()
            .zip(kinds)
            .filter(|(_, k)| **k == Kind::Item)
            .map(|(r, _)| r)
            .collect();
        let pool: Vec<&TextRow> = if items.is_empty() {
            rows.iter().collect()
        } else {
            items
        };
        Self {
            height: median(pool.iter().map(|r| (r.bottom - r.top).max(1)).collect()).unwrap_or(1),
            left: median(pool.iter().map(|r| r.left).collect()).unwrap_or(0),
        }
    }

    /// Whether `next` sits close enough below `prev` to be the same item.
    fn close(&self, prev: &TextRow, next: &TextRow, strong: bool) -> bool {
        let gap = next.top - prev.bottom;
        let limit = if strong {
            self.height * 3 / 2
        } else {
            self.height * 3 / 4
        };
        gap <= limit
    }

    fn not_left_of_items(&self, row: &TextRow) -> bool {
        row.left >= self.left - self.height / 2
    }

    /// An indent, a lowercase start, or `prev` ending mid-word.
    fn continues(&self, prev: &TextRow, row: &TextRow) -> bool {
        row.left - self.left >= (self.height / 2).max(1)
            || row
                .text
                .chars()
                .find(|c| c.is_alphabetic())
                .is_some_and(char::is_lowercase)
            || mid_word(&name_part(prev), &row.text)
    }
}

/// The most rows a name wraps onto.
const MAX_WRAP: usize = 2;

/// Which rows make up each item, top to bottom, by layout (see the module
/// docs). Rows are indexed as given; give them top to bottom, as
/// [`crate::group_rows`] returns them.
#[must_use]
pub fn item_row_groups(rows: &[TextRow]) -> Vec<Vec<usize>> {
    let lines: Vec<&str> = rows.iter().map(|r| r.text.as_str()).collect();
    let shop_row = read_header(&lines).name_line;
    let kinds: Vec<Kind> = rows
        .iter()
        .enumerate()
        .map(|(i, r)| classify(r, shop_row == Some(i)))
        .collect();
    let layout = Layout::new(rows, &kinds);

    let mut groups: Vec<Vec<usize>> = Vec::new();
    // The item that rows below may still join.
    let mut open: Option<usize> = None;
    let mut pending: Vec<usize> = Vec::new();

    // Moves pending rows below the open item while they fit; returns the
    // rest. With `strong_only`, only rows that clearly continue it.
    let below = |groups: &mut Vec<Vec<usize>>,
                 open: Option<usize>,
                 pending: &mut Vec<usize>,
                 strong_only: bool| {
        let Some(g) = open else { return };
        let mut taken = 0;
        for &r in pending.iter() {
            let group = &groups[g];
            let prev = &rows[*group.last().unwrap_or(&r)];
            let strong = layout.continues(prev, &rows[r]);
            let wraps = group.len() - 1;
            if wraps >= MAX_WRAP
                || (strong_only && !strong)
                || !layout.close(prev, &rows[r], strong)
                || !layout.not_left_of_items(&rows[r])
            {
                break;
            }
            groups[g].push(r);
            taken += 1;
        }
        pending.drain(..taken);
    };

    for (i, kind) in kinds.iter().enumerate() {
        match kind {
            Kind::Text => pending.push(i),
            Kind::Item => {
                below(&mut groups, open, &mut pending, false);
                pending.clear();
                groups.push(vec![i]);
                open = Some(groups.len() - 1);
            }
            Kind::PriceOnly => {
                below(&mut groups, open, &mut pending, true);
                // The name above: the last rows, close together and to the
                // price.
                let mut above: Vec<usize> = Vec::new();
                let mut next = &rows[i];
                for &r in pending.iter().rev().take(MAX_WRAP) {
                    if !layout.close(&rows[r], next, false) {
                        break;
                    }
                    above.insert(0, r);
                    next = &rows[r];
                }
                pending.clear();
                if above.is_empty() {
                    open = None;
                } else {
                    above.push(i);
                    groups.push(above);
                    open = Some(groups.len() - 1);
                }
            }
            Kind::Blocked => {
                below(&mut groups, open, &mut pending, false);
                pending.clear();
                open = None;
            }
        }
    }
    below(&mut groups, open, &mut pending, false);
    groups
}

/// Reads a receipt from its printed rows, joining item names that wrap
/// ([`item_row_groups`]).
#[must_use]
pub fn parse_receipt_rows(rows: &[TextRow]) -> RowReceipt {
    parse_receipt_row_groups(rows, &item_row_groups(rows))
}

/// Reads a receipt from its printed rows with the items' rows given, as
/// after the user joined or split items. Nothing in a group is filtered
/// out: the user chose it. Indices out of range are ignored.
#[must_use]
pub fn parse_receipt_row_groups(rows: &[TextRow], groups: &[Vec<usize>]) -> RowReceipt {
    let lines: Vec<&str> = rows.iter().map(|r| r.text.as_str()).collect();
    let header = read_header(&lines);
    let grouped: Vec<usize> = groups.iter().flatten().copied().collect();
    let total_minor = rows
        .iter()
        .enumerate()
        .filter(|(i, _)| !grouped.contains(i))
        .find_map(|(_, r)| {
            split_price(&r.text)
                .and_then(|(price, before)| is_total(&before.to_lowercase()).then_some(price))
        });
    RowReceipt {
        shop: header.name,
        shop_row: header.name_line,
        phones: header.phones,
        licences: header.licences,
        date: rows.iter().find_map(|r| find_date(&r.text)),
        total_minor,
        items: groups.iter().filter_map(|g| item(rows, g)).collect(),
    }
}

fn item(rows: &[TextRow], group: &[usize]) -> Option<RowItem> {
    let mut indices: Vec<usize> = group.iter().copied().filter(|&i| i < rows.len()).collect();
    indices.sort_unstable();
    indices.dedup();
    let price_row = indices
        .iter()
        .rev()
        .find(|&&i| split_price(&rows[i].text).is_some())
        .copied();
    let mut parts: Vec<String> = Vec::new();
    for &i in &indices {
        let part = if Some(i) == price_row {
            name_part(&rows[i])
        } else {
            rows[i].text.trim().to_owned()
        };
        // A quantity printed alone with the price (`2 @ 17.50`) leads.
        if Some(i) == price_row && letters(&part) < 2 {
            parts.insert(0, part);
        } else {
            parts.push(part);
        }
    }
    let mut text = String::new();
    for part in parts.iter().filter(|p| !p.is_empty()) {
        // A word cut at the row's end joins without a space; a hyphen
        // stays (`PRE-` + `ROLL`).
        if !text.is_empty() && !mid_word(&text, part) {
            text.push(' ');
        }
        text.push_str(part);
    }
    if indices.is_empty() || text.is_empty() && price_row.is_none() {
        return None;
    }
    let breakdown = describe_item(&text);
    let (printed, name) = quantity(&text);
    Some(RowItem {
        name: name.trim().to_owned(),
        quantity: if printed == 1 {
            breakdown.quantity.unwrap_or(1)
        } else {
            printed
        },
        price_minor: price_row
            .and_then(|i| split_price(&rows[i].text))
            .map(|(p, _)| p),
        rows: indices,
        breakdown,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use terpoir_model::{Form, StrainType};

    /// A row 30 pixels high, about 15 pixels a character.
    fn row(text: &str, left: i32, top: i32) -> TextRow {
        let width = i32::try_from(text.chars().count()).unwrap_or(0) * 15;
        TextRow {
            text: text.to_owned(),
            left,
            top,
            right: left + width,
            bottom: top + 30,
            pieces: Vec::new(),
        }
    }

    /// Rows 40 pixels apart, names at `x = 20`, prices at the right.
    fn receipt(lines: &[(&str, i32)]) -> Vec<TextRow> {
        lines
            .iter()
            .zip(0..)
            .map(|(&(text, left), n)| row(text, left, 20 + n * 40))
            .collect()
    }

    fn names(r: &RowReceipt) -> Vec<&str> {
        r.items.iter().map(|i| i.name.as_str()).collect()
    }

    fn groups(r: &RowReceipt) -> Vec<Vec<usize>> {
        r.items.iter().map(|i| i.rows.clone()).collect()
    }

    #[test]
    fn joins_a_name_wrapped_below_its_price() {
        let rows = receipt(&[
            ("GREEN LEAF", 200),
            ("Tel (555) 010-2030", 150),
            ("10/11/2026 2:02 PM", 20),
            ("BLUE DREAM 3.5G HYBRID FLOWE 35.00", 20),
            ("R - INDOOR", 20),
            ("BD PR 1G 12.00", 20),
            ("2 X MANGO GUMMIES 36.00", 20),
            ("100MG SATIVA", 20),
            ("Subtotal 83.00", 20),
            ("Total 83.00", 20),
            ("Thank you!", 150),
        ]);
        let r = parse_receipt_rows(&rows);
        assert_eq!(r.shop.as_deref(), Some("GREEN LEAF"));
        assert_eq!(r.shop_row, Some(0));
        assert_eq!(r.phones, ["5550102030"]);
        assert_eq!(r.date, Date::new(2026, 10, 11));
        assert_eq!(r.total_minor, Some(8_300));
        assert_eq!(
            names(&r),
            [
                "BLUE DREAM 3.5G HYBRID FLOWER - INDOOR",
                "BD PR 1G",
                "MANGO GUMMIES 100MG SATIVA",
            ]
        );
        assert_eq!(groups(&r), [vec![3, 4], vec![5], vec![6, 7]]);
        let blue = &r.items[0];
        assert_eq!(blue.price_minor, Some(3_500));
        assert_eq!(blue.breakdown.name, "BLUE DREAM");
        assert_eq!(blue.breakdown.form, Some(Form::Flower));
        assert_eq!(blue.breakdown.strain_type, Some(StrainType::Hybrid));
        assert_eq!(blue.breakdown.other, ["INDOOR"]);
        let gummies = &r.items[2];
        assert_eq!(gummies.quantity, 2);
        assert_eq!(gummies.breakdown.form, Some(Form::Edible));
        assert_eq!(gummies.breakdown.strain_type, Some(StrainType::Sativa));
        assert_eq!(gummies.breakdown.name, "MANGO");
    }

    #[test]
    fn reads_names_above_a_price_printed_alone() {
        let rows = receipt(&[
            ("GREEN LEAF", 200),
            ("BLUE DREAM 3.5G", 20),
            ("HYBRID FLOWER", 20),
            ("35.00", 500),
            ("LIVE RESIN CART", 20),
            ("2 @ 22.50", 400),
            ("TOTAL 80.00", 20),
        ]);
        let r = parse_receipt_rows(&rows);
        assert_eq!(
            names(&r),
            ["BLUE DREAM 3.5G HYBRID FLOWER", "LIVE RESIN CART"]
        );
        assert_eq!(groups(&r), [vec![1, 2, 3], vec![4, 5]]);
        assert_eq!(r.items[1].quantity, 2);
        assert_eq!(r.items[1].price_minor, Some(2_250));
        assert_eq!(r.items[1].breakdown.form, Some(Form::VapeCart));
    }

    #[test]
    fn a_row_that_clearly_continues_stays_with_the_item_above() {
        // The indented row and the lowercase one continue the cart; the
        // next row is the gummies' name, above their price.
        let rows = receipt(&[
            ("LIVE RESIN CART 1G 45.00", 20),
            ("   GELATO 41", 50),
            ("indoor", 20),
            ("MANGO GUMMIES", 20),
            ("18.00", 500),
        ]);
        let r = parse_receipt_rows(&rows);
        assert_eq!(groups(&r), [vec![0, 1, 2], vec![3, 4]]);
        assert_eq!(r.items[0].breakdown.name, "GELATO 41");
        // A hyphen that ends a row joins without a space.
        let rows = receipt(&[("BD PRE- 12.00", 20), ("ROLL 1G", 20)]);
        let r = parse_receipt_rows(&rows);
        assert_eq!(names(&r), ["BD PRE-ROLL 1G"]);
        assert_eq!(r.items[0].breakdown.form, Some(Form::PreRoll));
    }

    #[test]
    fn rows_far_below_or_left_of_items_do_not_join() {
        let rows = vec![
            row("BD PR 1G 12.00", 20, 20),
            // A blank line's gap below the item.
            row("SEE YOU SOON", 20, 100),
            row("GSC 3.5G 30.00", 20, 180),
            // Further left than the item names.
            row("WE LOVE OUR PATRONS", 0, 220),
        ];
        let r = parse_receipt_rows(&rows);
        assert_eq!(groups(&r), [vec![0], vec![2]]);
    }

    #[test]
    fn never_joins_personal_lines_or_the_shop() {
        let rows = receipt(&[
            ("GREEN LEAF", 20),
            ("Lic# C10-0000123-LIC", 20),
            ("ITEM QTY PRICE", 20),
            ("BD PR 1G 12.00", 20),
            ("Member Jane Doe", 20),
            ("Loyalty discount -2.00", 20),
            ("Total 10.00", 20),
        ]);
        let r = parse_receipt_rows(&rows);
        assert_eq!(groups(&r), [vec![3]]);
        assert_eq!(r.licences, ["C100000123LIC"]);
        assert_eq!(r.total_minor, Some(1_000));
    }

    #[test]
    fn rereads_groups_the_user_changed() {
        let rows = receipt(&[
            ("BLUE DREAM 3.5G HYBRID FLOWE 35.00", 20),
            ("R - INDOOR", 20),
            ("BD PR 1G 12.00", 20),
        ]);
        let auto = item_row_groups(&rows);
        assert_eq!(auto, [vec![0, 1], vec![2]]);
        // Split here: the second row on its own (no price).
        let r = parse_receipt_row_groups(&rows, &[vec![0], vec![1], vec![2]]);
        assert_eq!(
            names(&r),
            ["BLUE DREAM 3.5G HYBRID FLOWE", "R - INDOOR", "BD PR 1G"]
        );
        assert_eq!(r.items[1].price_minor, None);
        // Join with next line, in any order, ignoring bad indices.
        let r = parse_receipt_row_groups(&rows, &[vec![2, 1, 0, 9], vec![]]);
        assert_eq!(r.items.len(), 1);
        assert_eq!(r.items[0].rows, [0, 1, 2]);
        assert_eq!(r.items[0].price_minor, Some(1_200));
        assert_eq!(
            r.items[0].name,
            "BLUE DREAM 3.5G HYBRID FLOWE 35.00 R - INDOOR BD PR 1G"
        );
    }

    #[test]
    fn empty_rows_give_nothing() {
        assert!(item_row_groups(&[]).is_empty());
        let r = parse_receipt_rows(&[]);
        assert!(r.items.is_empty() && r.shop.is_none());
    }

    #[test]
    fn notices_text_at_the_photo_edge() {
        let rows = vec![row("GREEN LEAF", 200, 100), row("BD PR 1G 12.00", 20, 300)];
        let cut = find_cut_off(&rows, 1000, 2000);
        assert!(!cut.any());
        // A row running off the right edge, one at the bottom.
        let rows = vec![
            row("GREEN LEAF", 200, 100),
            TextRow {
                right: 998,
                ..row("BD PR 1G 12.00", 20, 300)
            },
            row("Total 12.00", 20, 1975),
        ];
        let cut = find_cut_off(&rows, 1000, 2000);
        assert_eq!(
            cut,
            CutOff {
                left: false,
                top: false,
                right: true,
                bottom: true
            }
        );
        // Left and top; a speck doesn't count.
        let rows = vec![row("GREEN", 0, 0), row(".", 500, 1999)];
        let cut = find_cut_off(&rows, 1000, 2000);
        assert!(cut.left && cut.top && !cut.right && !cut.bottom);
        assert!(!find_cut_off(&rows, 0, 0).any());
    }
}

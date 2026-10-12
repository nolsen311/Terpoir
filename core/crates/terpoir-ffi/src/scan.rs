// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

//! Rapid product entry: parsing of text the platform recognized on the
//! device, and of decoded barcodes. See `terpoir-scan`.
//!
//! These types carry text from the user's receipts and labels, so none
//! implements `Debug`.

use terpoir_model::TagRef;

use crate::records::{TagIntensity, TagRefData};

/// One candidate receipt line.
#[derive(Clone, PartialEq, Eq, uniffi::Record)]
pub struct ReceiptItemData {
    /// The item text as printed.
    pub name: String,
    /// Units bought; 1 when not printed.
    pub quantity: u32,
    /// The line's price in minor units.
    pub price_minor: i64,
    /// The input lines it was read from (counting blank ones).
    pub lines: Vec<u32>,
}

/// One line the user picked as a receipt item.
#[derive(Clone, PartialEq, Eq, uniffi::Record)]
pub struct ReceiptLineData {
    /// The text before the price, without a leading quantity.
    pub name: String,
    /// Units bought; 1 when not printed.
    pub quantity: u32,
    /// The price at the end of the line, in minor units.
    pub price_minor: Option<i64>,
}

/// A piece of recognized text and its box on the photo, in pixels.
#[derive(Clone, PartialEq, Eq, uniffi::Record)]
pub struct TextPieceData {
    /// The recognized text.
    pub text: String,
    /// Left edge.
    pub left: i32,
    /// Top edge.
    pub top: i32,
    /// Right edge.
    pub right: i32,
    /// Bottom edge.
    pub bottom: i32,
}

/// One printed row of recognized text.
#[derive(Clone, PartialEq, Eq, uniffi::Record)]
pub struct TextRowData {
    /// The pieces' text, left to right.
    pub text: String,
    /// Left edge of the row's box.
    pub left: i32,
    /// Top edge.
    pub top: i32,
    /// Right edge.
    pub right: i32,
    /// Bottom edge.
    pub bottom: i32,
}

/// What a receipt seems to say; every field is a suggestion to confirm.
#[derive(Clone, PartialEq, Eq, uniffi::Record)]
pub struct ReceiptScan {
    /// Shop name from the header.
    pub shop: Option<String>,
    /// The input line the shop name was read from.
    pub shop_line: Option<u32>,
    /// Purchase date, in days since 1970-01-01.
    pub date_day: Option<i64>,
    /// Printed total, in minor units.
    pub total_minor: Option<i64>,
    /// Candidate items, in printed order.
    pub items: Vec<ReceiptItemData>,
}

/// What a label seems to say; every field is a suggestion to confirm.
#[derive(Clone, PartialEq, Eq, uniffi::Record)]
pub struct LabelScanData {
    /// THC in thousandths of `thc_unit`.
    pub thc_milli: Option<i64>,
    /// `percent`, `mg_per_unit` or `mg_total`.
    pub thc_unit: Option<String>,
    /// CBD in thousandths of `cbd_unit`.
    pub cbd_milli: Option<i64>,
    /// `percent`, `mg_per_unit` or `mg_total`.
    pub cbd_unit: Option<String>,
    /// Terpenes (built-in tags) in thousandths of a percent.
    pub terpenes: Vec<TagIntensity>,
    /// Batch or lot.
    pub batch: Option<String>,
    /// Package date, in days since 1970-01-01.
    pub package_date_day: Option<i64>,
    /// Seed-to-sale package tags printed on the label.
    pub package_tags: Vec<String>,
}

/// A decoded barcode or QR code, classified.
#[derive(Clone, PartialEq, Eq, uniffi::Record)]
pub struct ScannedCodeData {
    /// The code as decoded (trimmed).
    pub code: String,
    /// `barcode`, `qr` or `package_tag`.
    pub kind: String,
    /// The code when it is a web link (such as lab results). Opening it is
    /// opt-in.
    pub link: Option<String>,
}

/// Parses text recognized from a receipt photo.
#[uniffi::export]
#[must_use]
pub fn parse_receipt(text: String) -> ReceiptScan {
    let r = terpoir_scan::parse_receipt(&text);
    ReceiptScan {
        shop: r.shop,
        shop_line: r.shop_line.map(line_number),
        date_day: r.date.map(terpoir_scan::Date::days_since_epoch),
        total_minor: r.total_minor,
        items: r
            .items
            .into_iter()
            .map(|i| ReceiptItemData {
                name: i.name,
                quantity: i.quantity,
                price_minor: i.price_minor,
                lines: i.lines.into_iter().map(line_number).collect(),
            })
            .collect(),
    }
}

fn line_number(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// Reads one receipt line the user picked as an item.
#[uniffi::export]
#[must_use]
pub fn parse_receipt_line(line: String) -> ReceiptLineData {
    let l = terpoir_scan::parse_receipt_line(&line);
    ReceiptLineData {
        name: l.name,
        quantity: l.quantity,
        price_minor: l.price_minor,
    }
}

/// Groups recognized pieces of text into printed rows, top to bottom, by
/// where they sit on the photo.
#[uniffi::export]
#[must_use]
pub fn group_text_rows(pieces: Vec<TextPieceData>) -> Vec<TextRowData> {
    let pieces: Vec<terpoir_scan::TextPiece> = pieces
        .into_iter()
        .map(|p| terpoir_scan::TextPiece {
            text: p.text,
            left: p.left,
            top: p.top,
            right: p.right,
            bottom: p.bottom,
        })
        .collect();
    terpoir_scan::group_rows(&pieces)
        .into_iter()
        .map(|r| TextRowData {
            text: r.text,
            left: r.left,
            top: r.top,
            right: r.right,
            bottom: r.bottom,
        })
        .collect()
}

/// Parses text recognized from a product label photo.
#[uniffi::export]
#[must_use]
pub fn parse_label(text: String) -> LabelScanData {
    let l = terpoir_scan::parse_label(&text);
    LabelScanData {
        thc_milli: l.thc.as_ref().map(|a| a.milli),
        thc_unit: l.thc.as_ref().map(|a| a.unit.slug().to_owned()),
        cbd_milli: l.cbd.as_ref().map(|a| a.milli),
        cbd_unit: l.cbd.as_ref().map(|a| a.unit.slug().to_owned()),
        terpenes: l
            .terpenes
            .iter()
            .map(|(terpene, milli)| TagIntensity {
                tag: TagRefData::from_model(&TagRef::Builtin(terpene.slug().to_owned())),
                value: *milli,
            })
            .collect(),
        batch: l.batch,
        package_date_day: l.package_date.map(terpoir_scan::Date::days_since_epoch),
        package_tags: l.package_tags,
    }
}

/// Classifies a decoded code. `two_dimensional` is true for QR, Data Matrix
/// and similar formats. `None` for empty, oversized or control-character
/// codes.
#[uniffi::export]
#[must_use]
pub fn classify_code(raw: String, two_dimensional: bool) -> Option<ScannedCodeData> {
    terpoir_scan::classify_code(&raw, two_dimensional).map(|c| ScannedCodeData {
        code: c.code,
        kind: c.kind.slug().to_owned(),
        link: c.link,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exports_parse_results() {
        let r = parse_receipt("Shop\n2026-10-11\nPre-roll 1g 12.00\nTotal 12.00".into());
        assert_eq!(r.date_day, Some(20_737));
        assert_eq!(r.items.len(), 1);
        assert_eq!(r.total_minor, Some(1_200));
        assert_eq!(r.items[0].lines, [2]);
        assert_eq!(r.shop_line, Some(0));

        let line = parse_receipt_line("Pre-roll 1g 12.00".into());
        assert_eq!(line.price_minor, Some(1_200));

        let piece = |text: &str, left, right| TextPieceData {
            text: text.into(),
            left,
            top: 0,
            right,
            bottom: 20,
        };
        let rows = group_text_rows(vec![piece("12.00", 300, 360), piece("Pre-roll", 0, 100)]);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].text, "Pre-roll 12.00");

        let l = parse_label("THC 22.4%\nLimonene 0.5%".into());
        assert_eq!(l.thc_unit.as_deref(), Some("percent"));
        assert!(
            l.terpenes[0].tag
                == TagRefData::Builtin {
                    slug: "limonene".into()
                }
        );

        let c = classify_code("https://lab.example/x".into(), true).unwrap();
        assert_eq!(c.kind, "qr");
        assert!(c.link.is_some());
    }
}

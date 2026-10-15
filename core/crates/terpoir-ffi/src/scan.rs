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

/// What an item's text says, broken down with the built-in vocabulary.
#[derive(Clone, PartialEq, Eq, uniffi::Record)]
pub struct ItemBreakdownData {
    /// What is left once known terms are taken out (upper case): usually
    /// the brand and strain, often abbreviated.
    pub name: String,
    /// Package size in thousandths of `size_unit` (3.5 g is 3500).
    pub size_milli: Option<i64>,
    /// `g`, `mg` or `ml` (ounces are converted to grams).
    pub size_unit: Option<String>,
    /// A product form slug (`flower`, `pre_roll`, `vape_cart`, …).
    pub form: Option<String>,
    /// `indica`, `sativa` or `hybrid`.
    pub strain_type: Option<String>,
    /// Units bought, when printed.
    pub quantity: Option<u32>,
    /// Units in one package (`5PK`), when printed.
    pub pack: Option<u32>,
    /// Other known words taken out (`INDOOR`, `THC`, `25%`), as printed.
    pub other: Vec<String>,
}

impl ItemBreakdownData {
    fn from_scan(b: terpoir_scan::ItemBreakdown) -> Self {
        Self {
            name: b.name,
            size_milli: b.size.map(|s| s.milli),
            size_unit: b.size.map(|s| s.unit.slug().to_owned()),
            form: b.form.map(|f| f.slug().to_owned()),
            strain_type: b.strain_type.map(|t| t.slug().to_owned()),
            quantity: b.quantity,
            pack: b.pack,
            other: b.other,
        }
    }
}

/// One item read from a receipt's printed rows.
#[derive(Clone, PartialEq, Eq, uniffi::Record)]
pub struct ReceiptRowItemData {
    /// The item text as printed, wrapped rows joined, without the price or
    /// a leading quantity.
    pub name: String,
    /// Units bought; 1 when not printed.
    pub quantity: u32,
    /// The price in minor units (`None` only for a group the user made
    /// without one).
    pub price_minor: Option<i64>,
    /// The rows it was read from, top to bottom (indices into the rows
    /// given).
    pub rows: Vec<u32>,
    /// The item text broken down.
    pub breakdown: ItemBreakdownData,
}

/// What a receipt's printed rows seem to say; every field is a suggestion.
#[derive(Clone, PartialEq, Eq, uniffi::Record)]
pub struct ReceiptRowsScan {
    /// Shop name from the header.
    pub shop: Option<String>,
    /// The row the shop name was read from.
    pub shop_row: Option<u32>,
    /// Phone numbers printed in the header, digits only.
    pub phones: Vec<String>,
    /// Licence numbers printed in the header, letters and digits only.
    pub licences: Vec<String>,
    /// Purchase date, in days since 1970-01-01.
    pub date_day: Option<i64>,
    /// Printed total, in minor units.
    pub total_minor: Option<i64>,
    /// Items, in printed order.
    pub items: Vec<ReceiptRowItemData>,
}

/// Which edges of the photo recognized text touches (part of the receipt
/// may be cut off: "Part of the receipt is cut off. Retake?").
#[derive(Clone, Copy, PartialEq, Eq, uniffi::Record)]
pub struct CutOffData {
    /// The left edge.
    pub left: bool,
    /// The top edge.
    pub top: bool,
    /// The right edge.
    pub right: bool,
    /// The bottom edge.
    pub bottom: bool,
}

fn text_rows(rows: Vec<TextRowData>) -> Vec<terpoir_scan::TextRow> {
    rows.into_iter()
        .map(|r| terpoir_scan::TextRow {
            text: r.text,
            left: r.left,
            top: r.top,
            right: r.right,
            bottom: r.bottom,
            pieces: Vec::new(),
        })
        .collect()
}

fn rows_scan(r: terpoir_scan::RowReceipt) -> ReceiptRowsScan {
    ReceiptRowsScan {
        shop: r.shop,
        shop_row: r.shop_row.map(line_number),
        phones: r.phones,
        licences: r.licences,
        date_day: r.date.map(terpoir_scan::Date::days_since_epoch),
        total_minor: r.total_minor,
        items: r
            .items
            .into_iter()
            .map(|i| ReceiptRowItemData {
                name: i.name,
                quantity: i.quantity,
                price_minor: i.price_minor,
                rows: i.rows.into_iter().map(line_number).collect(),
                breakdown: ItemBreakdownData::from_scan(i.breakdown),
            })
            .collect(),
    }
}

/// Reads a receipt from its printed rows (as `group_text_rows` returns
/// them), joining item names that wrap onto another row. Each item reports
/// the rows it came from.
#[uniffi::export]
#[must_use]
pub fn parse_receipt_rows(rows: Vec<TextRowData>) -> ReceiptRowsScan {
    rows_scan(terpoir_scan::parse_receipt_rows(&text_rows(rows)))
}

/// Reads a receipt from its printed rows with each item's rows given, as
/// after "Join with next line" or "Split here". Nothing in a group is
/// filtered out; indices out of range are ignored.
#[uniffi::export]
#[must_use]
pub fn parse_receipt_row_groups(rows: Vec<TextRowData>, groups: Vec<Vec<u32>>) -> ReceiptRowsScan {
    let groups: Vec<Vec<usize>> = groups
        .into_iter()
        .map(|g| {
            g.into_iter()
                .filter_map(|i| usize::try_from(i).ok())
                .collect()
        })
        .collect();
    rows_scan(terpoir_scan::parse_receipt_row_groups(
        &text_rows(rows),
        &groups,
    ))
}

/// Which edges of a `width` × `height` photo the rows touch or cross
/// (within 1% of the side), in the rows' coordinates.
#[uniffi::export]
#[must_use]
pub fn receipt_cut_off(rows: Vec<TextRowData>, width: i32, height: i32) -> CutOffData {
    let cut = terpoir_scan::find_cut_off(&text_rows(rows), width, height);
    CutOffData {
        left: cut.left,
        top: cut.top,
        right: cut.right,
        bottom: cut.bottom,
    }
}

/// Breaks an item's text into size, form, strain type, pack count,
/// quantity, and the name that is left (`BD PR 1G` leaves `BD`).
#[uniffi::export]
#[must_use]
pub fn describe_receipt_item(text: String) -> ItemBreakdownData {
    ItemBreakdownData::from_scan(terpoir_scan::describe_item(&text))
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

        let row = |text: &str, left, top| TextRowData {
            text: text.into(),
            left,
            top,
            right: left + 300,
            bottom: top + 30,
        };
        let rows = vec![
            row("Shop", 100, 20),
            row("BLUE DREAM 3.5G HYBRID FLOWE 35.00", 20, 60),
            row("R - INDOOR", 20, 100),
        ];
        let r = parse_receipt_rows(rows.clone());
        assert_eq!(r.shop_row, Some(0));
        assert_eq!(r.items.len(), 1);
        assert_eq!(r.items[0].rows, [1, 2]);
        let b = &r.items[0].breakdown;
        assert_eq!(b.name, "BLUE DREAM");
        assert_eq!(
            (b.size_milli, b.size_unit.as_deref()),
            (Some(3_500), Some("g"))
        );
        assert_eq!(b.form.as_deref(), Some("flower"));
        assert_eq!(b.strain_type.as_deref(), Some("hybrid"));
        let split = parse_receipt_row_groups(rows.clone(), vec![vec![1], vec![2]]);
        assert_eq!(split.items.len(), 2);
        assert!(!receipt_cut_off(rows.clone(), 1000, 1000).left);
        assert!(receipt_cut_off(rows, 300, 1000).right);
        let d = describe_receipt_item("2 X BD PR 1G".into());
        assert_eq!((d.name.as_str(), d.quantity), ("BD", Some(2)));
        assert_eq!(d.form.as_deref(), Some("pre_roll"));

        let c = classify_code("https://lab.example/x".into(), true).unwrap();
        assert_eq!(c.kind, "qr");
        assert!(c.link.is_some());
    }
}

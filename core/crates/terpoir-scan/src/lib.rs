// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

//! Parsing for rapid product entry. Platforms run text recognition and
//! barcode decoding on the device and pass the results here; everything
//! below is plain pattern matching, with no network and no models.
//!
//! - [`group_rows`]: printed rows from recognized pieces of text and where
//!   they sit on the photo.
//! - [`parse_receipt`]: shop, date, total and line items from receipt text;
//!   [`parse_receipt_line`] for one line the user picked.
//! - [`parse_label`]: THC, CBD, terpenes, batch, package date and package
//!   tags from a compliance label.
//! - [`classify_code`]: what kind of identifier a decoded barcode or QR
//!   code is, and whether it is a link (opening one is opt-in).
//! - [`normalize_name`]: the comparison key for duplicate detection.
//! - [`parse_receipt_rows`]: a receipt read by printed row, joining item
//!   names that wrap onto another row ([`item_row_groups`]), and
//!   [`parse_receipt_row_groups`] after the user joined or split items;
//!   [`find_cut_off`] for text touching the photo's edge.
//! - [`describe_item`]: an item's size, form, strain type, pack and
//!   quantity from the built-in vocabulary, and the name that is left.
//! - [`match_names`]: the user's products, brands and strains ranked
//!   against an item's name (initials, letters in order, cut-off names).
//! - [`read_header`] and [`match_shop`]: the shop's name, phone and
//!   licence numbers, matched against saved shops.
//!
//! Results carry text from the user's receipts and labels, so none of the
//! types implements `Debug`.

mod code;
mod date;
mod header;
mod label;
mod layout;
mod matching;
mod name;
mod number;
mod receipt;
mod terms;
mod wrap;

pub use code::{ScannedCode, classify_code, is_package_tag};
pub use date::Date;
pub use header::{
    HEADER_LINES, ShopCandidate, ShopHeader, ShopMatch, ShopMatchKind, match_shop, read_header,
};
pub use label::{LabelAmountGuess, LabelScan, parse_label};
pub use layout::{TextPiece, TextRow, group_rows};
pub use matching::{
    CLOSE, CandidateKind, MatchKind, NameCandidate, NameMatch, match_names, score_name,
};
pub use name::{names_match, normalize_name};
pub use receipt::{Receipt, ReceiptItem, ReceiptLine, parse_receipt, parse_receipt_line};
pub use terms::{ItemBreakdown, Size, SizeUnit, describe_item};
pub use wrap::{
    CutOff, RowItem, RowReceipt, find_cut_off, item_row_groups, parse_receipt_row_groups,
    parse_receipt_rows,
};

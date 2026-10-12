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
//!
//! Results carry text from the user's receipts and labels, so none of the
//! types implements `Debug`.

mod code;
mod date;
mod label;
mod layout;
mod name;
mod number;
mod receipt;

pub use code::{ScannedCode, classify_code, is_package_tag};
pub use date::Date;
pub use label::{LabelAmountGuess, LabelScan, parse_label};
pub use layout::{TextPiece, TextRow, group_rows};
pub use name::{names_match, normalize_name};
pub use receipt::{Receipt, ReceiptItem, ReceiptLine, parse_receipt, parse_receipt_line};

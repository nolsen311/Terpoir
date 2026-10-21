// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

//! The Terpoir core as seen from Kotlin (and later Swift), via UniFFI.
//!
//! Everything that touches crypto, the data model, storage or sync stays
//! in Rust; the platform layer only moves bytes between this API, the
//! platform keystore, the UI and the user's chosen sync storage.
//!
//! # Secrets crossing the boundary
//!
//! The master key crosses as bytes exactly where the platform keystore
//! has to wrap or unwrap it (setup, unlock, rotation). Rust wipes its own
//! copies; the platform must wipe its arrays right after use (Kotlin:
//! `fill(0)`). The recovery key crosses as text only to be shown once or
//! typed in. Passphrases cross as text. JVM strings cannot be wiped, so
//! the UI should hold them as briefly as possible.
//!
//! # Errors and panics
//!
//! Errors carry a kind only, never content. A panic hook that discards
//! panic messages is installed by [`init`] and by [`Journal::open`]; release
//! builds report nothing, debug builds only the source location.

#![forbid(unsafe_code)]

use std::sync::Once;

mod archive;
mod capture;
mod catalog;
mod error;
mod feedback;
mod insights;
mod journal;
mod keys;
mod lookup;
mod plaintext;
mod records;
mod scan;
mod tracking;
mod vocabulary;

pub use archive::{
    ArchiveCheck, ExportArchive, RestoredKeys, check_export_archive, keys_for_restore,
    read_export_archive, readable_blobs, sync_folder_key_file,
};
pub use capture::{capture_ready, seal_capture};
pub use catalog::{CatalogStrainData, StrainCatalog};
pub use error::{CoreError, Result};
pub use insights::{
    InsightData, InsightFeedback, ShopEffectData, ShopInsightData, TimelineComparisonData,
    TimelineInsightData,
};
pub use journal::{
    ImportResult, Journal, LabelName, NearbySpot, ProductMatch, PromptFence, ReceiptShopMatch,
    RecordType, ScanSuggestion, SyncBlob, TagSuggestion,
};
pub use keys::{
    NewKeys, PassphraseWrapped, RotatedKeys, check_recovery_key, create_keys,
    master_key_from_passphrase, master_key_from_recovery, wrap_master_key_with_passphrase,
};
pub use lookup::{
    LookupSiteData, lookup_address, lookup_template_from_example, lookup_template_valid,
};
pub use plaintext::ExportFile;
pub use records::{
    BrandData, CannabinoidAmountData, IdentifierData, LocalTimeData, PackageSizeData, ProductData,
    ProductLineData, PurchaseData, PurchaseLineData, SessionData, SnippetData, SpotData,
    StrainData, SymptomData, TagData, TagIntensity, TagProvenance, TagRefData, TerpeneLevelData,
    TimelineLevelData, TimelinePlanData,
};
pub use scan::{
    CutOffData, ItemBreakdownData, JoinedRowsData, LabelScanData, ReceiptItemData,
    ReceiptRowItemData, ReceiptRowsScan, ReceiptScan, ScannedCodeData, classify_code,
    describe_receipt_item, join_receipt_photos, package_amount_per, parse_label, parse_receipt,
    parse_receipt_row_groups, parse_receipt_rows, receipt_cut_off,
};
pub use tracking::trackable_data_points;
pub use vocabulary::{
    Vocabulary, check_in_minutes_for_methods, default_method_for_form, dose_units_for_method,
    subtype_parent, subtypes_for_form, timeline_plan_for_methods, vocabulary,
};

uniffi::setup_scaffolding!();

static PANIC_HOOK: Once = Once::new();

/// Installs a panic hook that never reports panic messages (which could
/// include data). Release builds report nothing at all; debug builds print
/// only the source location, to help development. Call once at app start;
/// [`Journal::open`] also calls it.
#[uniffi::export]
pub fn init() {
    PANIC_HOOK.call_once(|| std::panic::set_hook(Box::new(report_location)));
}

#[cfg(debug_assertions)]
#[allow(clippy::print_stderr, reason = "debug builds only, location only")]
fn report_location(info: &std::panic::PanicHookInfo<'_>) {
    if let Some(location) = info.location() {
        eprintln!("panic at {}:{}", location.file(), location.line());
    }
}

#[cfg(not(debug_assertions))]
fn report_location(_: &std::panic::PanicHookInfo<'_>) {}

/// The core's version, for the about screen and bug reports.
#[uniffi::export]
#[must_use]
pub fn core_version() -> String {
    env!("CARGO_PKG_VERSION").to_owned()
}

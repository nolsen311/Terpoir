// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

//! Terpoir's data model.
//!
//! # Documents and fields
//!
//! Every record is stored and synced as a [`Document`]: an ID, a
//! [`RecordKind`], a schema version, and a map of named fields. Each field
//! carries its own [`Stamp`] (hybrid logical clock time plus device ID), so
//! sync can merge per field with last-writer-wins (see `terpoir-sync`).
//!
//! Map-shaped data (effects, product lines, photos, tags) is flattened into
//! one field per entry, for example `effects/relaxed`, so two devices adding
//! different entries both keep theirs. Values that only make sense together
//! (a dose's amount and unit) are a single [`Value::Tuple`] and are replaced
//! as a whole. Removing an entry writes [`Value::Null`] with a fresh stamp,
//! so the removal itself merges like any other edit.
//!
//! # Typed records
//!
//! [`Session`], [`Product`], [`Strain`], [`Brand`], [`Spot`], [`Snippet`]
//! and [`Tag`] are typed views. [`Record::from_document`] reads one out of a
//! document; [`Document::update`] writes one back, stamping only the fields
//! that changed and leaving fields it does not know about (written by a
//! newer app version) untouched.
//!
//! # Privacy
//!
//! `Debug` output of documents, values and records is redacted, and
//! decrypted text is wiped when a document is dropped. Nothing here logs.

#![forbid(unsafe_code)]

mod document;
mod error;
mod records;
mod stamp;
mod value;

pub use document::{CURRENT_SCHEMA_VERSION, Document, Field, migrate};
pub use error::{Error, Result};
pub use records::{
    AmountPer, Brand, Cannabinoid, CoreEffect, DEFAULT_SPOT_RADIUS_M, Dose, DoseUnit, Flavor, Form,
    IdentifierKind, InsightFeedbackList, Intention, LabelAmount, LabelUnit, LocalTime, Location,
    MAX_RECEIPT_NAME_CHARS, MIN_SPOT_RADIUS_M, Method, Mood, PackageSize, Price, Product,
    ProductLine, Provenance, Purchase, PurchaseLine, PurchaseRef, Record, RecordKind, Session,
    SizeAmount, SizeUnit, Snippet, Social, Spot, SpotPurpose, Strain, StrainDetail, StrainType,
    Subtype, SymptomResult, Tag, TagCategory, TagRef, TargetedSymptom, Terpene, TerpeneLevel,
    Timeline, TimelinePlan, TrackedField, TrackingPreferences, check_in_minutes_for,
    receipt_name_key, timeline_plan_for, usual_method,
};
pub use stamp::{DeviceId, Hlc, Stamp};
pub use terpoir_crypto::RecordId;
pub use value::Value;

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    /// Built-in tags are stored and indexed by slug alone, so one slug must
    /// never name two things.
    #[test]
    fn builtin_tag_slugs_are_unique() {
        let effects: Vec<&str> = CoreEffect::ALL.iter().map(|e| e.slug()).collect();
        let purposes = [
            SpotPurpose::Blaze,
            SpotPurpose::Munch,
            SpotPurpose::ReUp,
            SpotPurpose::Chill,
        ]
        .map(SpotPurpose::slug);
        let all: Vec<&str> = effects
            .iter()
            .copied()
            .chain(Terpene::SLUGS.iter().copied())
            .chain(Flavor::SLUGS.iter().copied())
            .chain(Mood::SLUGS.iter().copied())
            .chain(TargetedSymptom::SLUGS.iter().copied())
            .chain(Subtype::SLUGS.iter().copied())
            .chain(purposes)
            .collect();
        let unique: HashSet<&str> = all.iter().copied().collect();
        assert_eq!(unique.len(), all.len());
    }
}

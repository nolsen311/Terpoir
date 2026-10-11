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
    Brand, CoreEffect, Dose, DoseUnit, Form, IdentifierKind, Intention, LabelAmount, LabelUnit,
    LocalTime, Location, Method, Price, Product, ProductLine, Provenance, Purchase, PurchaseLine,
    PurchaseRef, Record, RecordKind, Session, Snippet, Social, Spot, SpotPurpose, Strain,
    StrainType, SymptomResult, Tag, TagCategory, TagRef, Timeline,
};
pub use stamp::{DeviceId, Hlc, Stamp};
pub use terpoir_crypto::RecordId;
pub use value::Value;

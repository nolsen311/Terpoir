// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

//! Documents: the stored and synced form of every record.

use core::fmt;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use terpoir_crypto::RecordId;
use zeroize::Zeroizing;

use crate::records::{Record, RecordKind};
use crate::{Error, Result, Stamp, Value};

/// The schema version this build writes. Bump it together with a new entry
/// in [`MIGRATIONS`].
pub const CURRENT_SCHEMA_VERSION: u16 = 1;

/// Serialization format of [`Document::encode`] (the envelope, not the
/// schema).
const FORMAT_VERSION: u8 = 1;

/// `MIGRATIONS[i]` upgrades a document from schema `i + 1` to `i + 2`.
///
/// Migrations must be deterministic and keep field stamps unchanged, so
/// devices that migrate the same document independently agree, and a
/// migration never wins a merge over a real edit.
const MIGRATIONS: &[fn(&mut Document) -> Result<()>] = &[];

const _: () = assert!(MIGRATIONS.len() == CURRENT_SCHEMA_VERSION as usize - 1);

/// One field: its value and when it was last written.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Field {
    /// The value. [`Value::Null`] means cleared or removed.
    pub value: Value,
    /// When and where it was written.
    pub stamp: Stamp,
}

/// A record as stored and synced: ID, kind, schema version, and stamped
/// fields. Text values are wiped from memory on drop.
#[derive(Clone, PartialEq, Eq)]
pub struct Document {
    id: RecordId,
    kind: RecordKind,
    schema_version: u16,
    fields: BTreeMap<String, Field>,
}

static NULL: Value = Value::Null;

impl Document {
    /// An empty document of the current schema.
    #[must_use]
    pub const fn new(id: RecordId, kind: RecordKind) -> Self {
        Self {
            id,
            kind,
            schema_version: CURRENT_SCHEMA_VERSION,
            fields: BTreeMap::new(),
        }
    }

    /// A new document for `record` with a fresh random ID, every field
    /// stamped with `stamp`.
    ///
    /// # Errors
    ///
    /// [`Error::Random`] if ID generation fails.
    pub fn create<R: Record>(record: &R, stamp: Stamp) -> Result<Self> {
        let mut doc = Self::new(RecordId::random()?, R::KIND);
        doc.update(record, stamp)?;
        Ok(doc)
    }

    /// The record ID.
    #[must_use]
    pub const fn id(&self) -> RecordId {
        self.id
    }

    /// The record kind.
    #[must_use]
    pub const fn kind(&self) -> RecordKind {
        self.kind
    }

    /// The schema version the fields follow.
    #[must_use]
    pub const fn schema_version(&self) -> u16 {
        self.schema_version
    }

    /// All fields, including cleared ones (whose value is
    /// [`Value::Null`]), in key order.
    #[must_use]
    pub const fn fields(&self) -> &BTreeMap<String, Field> {
        &self.fields
    }

    /// One field, if it has ever been written.
    #[must_use]
    pub fn field(&self, key: &str) -> Option<&Field> {
        self.fields.get(key)
    }

    /// A field's value, or [`Value::Null`] if never written.
    #[must_use]
    pub fn value(&self, key: &str) -> &Value {
        self.fields.get(key).map_or(&NULL, |f| &f.value)
    }

    /// Non-null entries of the map stored under `prefix/`, as
    /// `(entry key, value)` pairs.
    pub fn map_entries<'a>(&'a self, prefix: &str) -> impl Iterator<Item = (&'a str, &'a Value)> {
        let start = format!("{prefix}/");
        self.fields
            .range(start.clone()..)
            .take_while(move |(k, _)| k.starts_with(&start))
            .filter(|(_, f)| !f.value.is_null())
            .filter_map(move |(k, f)| Some((k.get(prefix.len() + 1..)?, &f.value)))
    }

    /// Sets a field unconditionally, as a local edit.
    pub fn set(&mut self, key: impl Into<String>, value: Value, stamp: Stamp) {
        self.insert_field(key, Field { value, stamp });
    }

    /// Replaces a field (value and stamp) as given. Sync uses this after
    /// deciding which side wins.
    pub fn insert_field(&mut self, key: impl Into<String>, field: Field) {
        if let Some(mut old) = self.fields.insert(key.into(), field) {
            old.value.wipe();
        }
    }

    /// The newest stamp of any field.
    #[must_use]
    pub fn latest_stamp(&self) -> Option<Stamp> {
        self.fields.values().map(|f| f.stamp).max()
    }

    /// Writes a typed record into this document. Only fields whose value
    /// changed get the new `stamp`; fields the record type owns but no
    /// longer has are cleared; fields the record type does not know about
    /// (from a newer app version) are left untouched. Returns how many
    /// fields changed.
    ///
    /// # Errors
    ///
    /// [`Error::WrongKind`] if this document holds a different kind.
    pub fn update<R: Record>(&mut self, record: &R, stamp: Stamp) -> Result<usize> {
        if self.kind != R::KIND {
            return Err(Error::WrongKind);
        }
        let mut new = record.to_fields();
        new.retain(|_, v| !v.is_null());
        let mut changed = 0;
        let to_clear: Vec<String> = self
            .fields
            .iter()
            .filter(|(k, f)| R::owns_field(k) && !f.value.is_null() && !new.contains_key(*k))
            .map(|(k, _)| k.clone())
            .collect();
        for key in to_clear {
            self.set(key, Value::Null, stamp);
            changed += 1;
        }
        for (key, value) in new {
            if *self.value(&key) != value {
                self.set(key, value, stamp);
                changed += 1;
            }
        }
        Ok(changed)
    }

    /// Serializes the document. The output is plaintext: encrypt it with
    /// the record key before it goes anywhere. It is wiped when dropped.
    ///
    /// # Errors
    ///
    /// [`Error::Format`] if serialization fails (not expected).
    pub fn encode(&self) -> Result<Zeroizing<Vec<u8>>> {
        let wire = WireRef {
            id: *self.id.as_bytes(),
            kind: self.kind.code(),
            schema_version: self.schema_version,
            fields: self
                .fields
                .iter()
                .map(|(k, f)| (k.as_str(), &f.value, f.stamp))
                .collect(),
        };
        let body = Zeroizing::new(postcard::to_allocvec(&wire).map_err(|_| Error::Format)?);
        let mut out = Zeroizing::new(Vec::with_capacity(1 + body.len()));
        out.push(FORMAT_VERSION);
        out.extend_from_slice(&body);
        Ok(out)
    }

    /// Parses [`Document::encode`] output. Does not migrate; call
    /// [`migrate`] before reading typed records.
    ///
    /// # Errors
    ///
    /// [`Error::Format`] for malformed input or duplicate fields.
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        let (&format, body) = bytes.split_first().ok_or(Error::Format)?;
        if format != FORMAT_VERSION {
            return Err(Error::Format);
        }
        let (wire, rest): (WireOwned, _) =
            postcard::take_from_bytes(body).map_err(|_| Error::Format)?;
        if !rest.is_empty() {
            return Err(Error::Format);
        }
        let mut doc = Self {
            id: RecordId::from_bytes(wire.id),
            kind: RecordKind::from_code(wire.kind).ok_or(Error::Format)?,
            schema_version: wire.schema_version,
            fields: BTreeMap::new(),
        };
        for (key, value, stamp) in wire.fields {
            if doc.fields.insert(key, Field { value, stamp }).is_some() {
                return Err(Error::Format);
            }
        }
        Ok(doc)
    }
}

impl Drop for Document {
    fn drop(&mut self) {
        for field in self.fields.values_mut() {
            field.value.wipe();
        }
    }
}

impl fmt::Debug for Document {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Document")
            .field("kind", &self.kind)
            .field("schema_version", &self.schema_version)
            .field("fields", &self.fields.len())
            .finish_non_exhaustive()
    }
}

#[derive(Serialize)]
struct WireRef<'a> {
    id: [u8; 16],
    kind: u8,
    schema_version: u16,
    fields: Vec<(&'a str, &'a Value, Stamp)>,
}

#[derive(Deserialize)]
struct WireOwned {
    id: [u8; 16],
    kind: u8,
    schema_version: u16,
    fields: Vec<(String, Value, Stamp)>,
}

/// Upgrades a decoded document to [`CURRENT_SCHEMA_VERSION`].
///
/// # Errors
///
/// [`Error::NewerSchema`] if the document comes from a newer app (keep it
/// stored unchanged and ask the user to update); [`Error::UnsupportedSchema`]
/// for versions no migration covers; or whatever a migration step returns.
pub fn migrate(mut doc: Document) -> Result<Document> {
    if doc.schema_version > CURRENT_SCHEMA_VERSION {
        return Err(Error::NewerSchema);
    }
    if doc.schema_version == 0 {
        return Err(Error::UnsupportedSchema);
    }
    while doc.schema_version < CURRENT_SCHEMA_VERSION {
        let step = MIGRATIONS
            .get(usize::from(doc.schema_version - 1))
            .ok_or(Error::UnsupportedSchema)?;
        step(&mut doc)?;
        doc.schema_version += 1;
    }
    Ok(doc)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Brand, DeviceId, Hlc};

    pub(crate) fn stamp(wall_ms: u64) -> Stamp {
        Stamp {
            hlc: Hlc {
                wall_ms,
                counter: 0,
            },
            device: DeviceId::from_bytes([1; 16]),
        }
    }

    #[test]
    fn encode_decode_round_trip() {
        let mut doc = Document::new(RecordId::from_bytes([9; 16]), RecordKind::Session);
        doc.set("a", Value::Text("hello".into()), stamp(1));
        doc.set(
            "b",
            Value::Tuple(vec![Value::Int(-3), Value::Bool(true)]),
            stamp(2),
        );
        doc.set("c", Value::Id(RecordId::from_bytes([4; 16])), stamp(3));
        doc.set("d", Value::Null, stamp(4));
        let bytes = doc.encode().unwrap();
        assert_eq!(bytes[0], FORMAT_VERSION);
        assert_eq!(Document::decode(&bytes).unwrap(), doc);
    }

    #[test]
    fn decode_rejects_garbage_and_trailing_bytes() {
        let doc = Document::new(RecordId::from_bytes([9; 16]), RecordKind::Brand);
        let mut bytes = doc.encode().unwrap().to_vec();
        bytes.push(0);
        assert_eq!(Document::decode(&bytes).err(), Some(Error::Format));
        assert_eq!(Document::decode(&[]).err(), Some(Error::Format));
        assert_eq!(Document::decode(&[2, 0, 0]).err(), Some(Error::Format));
        assert_eq!(Document::decode(&[1, 0xff]).err(), Some(Error::Format));
    }

    #[test]
    fn map_entries_skip_nulls_and_other_prefixes() {
        let mut doc = Document::new(RecordId::from_bytes([1; 16]), RecordKind::Session);
        doc.set("effects/relaxed", Value::Int(3), stamp(1));
        doc.set("effects/sleepy", Value::Null, stamp(1));
        doc.set("effectsx", Value::Int(1), stamp(1));
        doc.set("effects", Value::Int(1), stamp(1));
        doc.set("f/x", Value::Int(1), stamp(1));
        let entries: Vec<_> = doc.map_entries("effects").collect();
        assert_eq!(entries, vec![("relaxed", &Value::Int(3))]);
    }

    #[test]
    fn update_stamps_only_changes_and_keeps_unknown_fields() {
        let mut brand = Brand {
            name: "Acme".into(),
        };
        let mut doc = Document::create(&brand, stamp(1)).unwrap();
        doc.set("future_field", Value::Int(7), stamp(1));

        assert_eq!(doc.update(&brand, stamp(2)).unwrap(), 0);
        assert_eq!(doc.field("name").unwrap().stamp, stamp(1));

        brand.name = "Acme Co".into();
        assert_eq!(doc.update(&brand, stamp(3)).unwrap(), 1);
        assert_eq!(doc.field("name").unwrap().stamp, stamp(3));
        assert_eq!(doc.value("future_field"), &Value::Int(7));
    }

    #[test]
    fn update_rejects_wrong_kind() {
        let mut doc = Document::new(RecordId::from_bytes([1; 16]), RecordKind::Spot);
        let brand = Brand { name: "x".into() };
        assert_eq!(doc.update(&brand, stamp(1)).err(), Some(Error::WrongKind));
    }

    #[test]
    fn migrate_checks_versions() {
        let doc = Document::new(RecordId::from_bytes([1; 16]), RecordKind::Brand);
        assert!(migrate(doc.clone()).is_ok());
        let mut newer = doc.clone();
        newer.schema_version = CURRENT_SCHEMA_VERSION + 1;
        assert_eq!(migrate(newer).err(), Some(Error::NewerSchema));
        let mut ancient = doc;
        ancient.schema_version = 0;
        assert_eq!(migrate(ancient).err(), Some(Error::UnsupportedSchema));
    }

    #[test]
    fn debug_is_redacted() {
        let mut doc = Document::new(RecordId::from_bytes([1; 16]), RecordKind::Brand);
        doc.set("name", Value::Text("secret brand".into()), stamp(1));
        assert!(!format!("{doc:?}").contains("secret"));
    }
}

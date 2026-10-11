// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

//! Typed record views over [`Document`]s.

use core::fmt;
use std::collections::{BTreeMap, BTreeSet};
use terpoir_crypto::RecordId;

use crate::{Document, Error, Result, Value};

mod catalog;
mod session;
mod snippet;
mod spot;
mod tag;

pub use catalog::{Brand, Form, LabelAmount, LabelUnit, Price, Product, Strain, StrainType};
pub use session::{
    CoreEffect, Dose, DoseUnit, Intention, Method, ProductLine, Session, Social, SymptomResult,
    Timeline,
};
pub use snippet::{Provenance, Snippet};
pub use spot::{Location, Spot, SpotPurpose};
pub use tag::{Tag, TagCategory};

/// The kind of record a document holds. The numeric codes are stored on
/// disk and synced: never change or reuse them.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[non_exhaustive]
pub enum RecordKind {
    /// A consumption session (journal entry).
    Session,
    /// A product.
    Product,
    /// A strain or cultivar.
    Strain,
    /// A brand.
    Brand,
    /// A saved spot.
    Spot,
    /// A stream snippet.
    Snippet,
    /// A user-defined tag.
    Tag,
}

impl RecordKind {
    /// Every kind.
    pub const ALL: [Self; 7] = [
        Self::Session,
        Self::Product,
        Self::Strain,
        Self::Brand,
        Self::Spot,
        Self::Snippet,
        Self::Tag,
    ];

    /// The stable numeric code.
    #[must_use]
    pub const fn code(self) -> u8 {
        match self {
            Self::Session => 1,
            Self::Product => 2,
            Self::Strain => 3,
            Self::Brand => 4,
            Self::Spot => 5,
            Self::Snippet => 6,
            Self::Tag => 7,
        }
    }

    /// The kind for a numeric code.
    #[must_use]
    pub const fn from_code(code: u8) -> Option<Self> {
        Some(match code {
            1 => Self::Session,
            2 => Self::Product,
            3 => Self::Strain,
            4 => Self::Brand,
            5 => Self::Spot,
            6 => Self::Snippet,
            7 => Self::Tag,
            _ => return None,
        })
    }
}

/// A typed view of one record kind.
pub trait Record: Sized {
    /// The kind of document this type reads and writes.
    const KIND: RecordKind;

    /// The record as flattened fields. `Null` values are ignored.
    fn to_fields(&self) -> BTreeMap<String, Value>;

    /// Reads the record from a (migrated) document.
    ///
    /// # Errors
    ///
    /// [`Error::WrongKind`] or [`Error::InvalidField`].
    fn from_document(doc: &Document) -> Result<Self>;

    /// Whether `key` is a field this type writes (a known field, or an
    /// entry of one of its maps). Fields it does not own are preserved by
    /// [`Document::update`].
    fn owns_field(key: &str) -> bool;
}

/// A local date and time: an instant plus the UTC offset where it was
/// recorded, so "10 pm" stays 10 pm after travel.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct LocalTime {
    /// Milliseconds since the Unix epoch.
    pub unix_ms: i64,
    /// Offset from UTC in minutes at the time of recording.
    pub utc_offset_min: i16,
}

impl LocalTime {
    pub(crate) fn to_value(self) -> Value {
        Value::Tuple(vec![
            Value::Int(self.unix_ms),
            Value::Int(i64::from(self.utc_offset_min)),
        ])
    }

    pub(crate) fn from_value(value: &Value, field: &'static str) -> Result<Self> {
        match value {
            Value::Tuple(items) => match items.as_slice() {
                [Value::Int(ms), Value::Int(offset)] => Ok(Self {
                    unix_ms: *ms,
                    utc_offset_min: i16::try_from(*offset)
                        .ok()
                        .filter(|o| (-18 * 60..=18 * 60).contains(o))
                        .ok_or(Error::InvalidField(field))?,
                }),
                _ => Err(Error::InvalidField(field)),
            },
            _ => Err(Error::InvalidField(field)),
        }
    }
}

/// A reference to a tag: one of the app's built-in vocabulary entries (by
/// slug, for example `relaxed` or `myrcene`) or a user-defined [`Tag`]
/// record.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum TagRef {
    /// A built-in vocabulary entry.
    Builtin(String),
    /// A user-defined tag record.
    Custom(RecordId),
}

const CUSTOM_PREFIX: &str = "custom:";

impl TagRef {
    /// The map key form: the slug, or `custom:<hex id>`.
    #[must_use]
    pub fn to_key(&self) -> String {
        match self {
            Self::Builtin(slug) => slug.clone(),
            Self::Custom(id) => format!("{CUSTOM_PREFIX}{}", id.to_hex()),
        }
    }

    /// Parses [`TagRef::to_key`] output.
    #[must_use]
    pub fn from_key(key: &str) -> Option<Self> {
        match key.strip_prefix(CUSTOM_PREFIX) {
            Some(hex) => RecordId::from_hex(hex).ok().map(Self::Custom),
            None if !key.is_empty() => Some(Self::Builtin(key.to_owned())),
            None => None,
        }
    }
}

/// Declares a vocabulary enum stored as a text slug. Unknown slugs (from a
/// newer app version) are kept as `Unknown` and written back unchanged.
macro_rules! slug_enum {
    (
        $(#[$meta:meta])*
        $name:ident { $($(#[$vmeta:meta])* $variant:ident => $slug:literal),+ $(,)? }
    ) => {
        $(#[$meta])*
        #[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
        #[non_exhaustive]
        pub enum $name {
            $($(#[$vmeta])* $variant,)+
            /// A value from a newer app version, preserved as is.
            Unknown(String),
        }

        impl $name {
            /// The stored slug.
            #[must_use]
            pub fn slug(&self) -> &str {
                match self {
                    $(Self::$variant => $slug,)+
                    Self::Unknown(slug) => slug,
                }
            }

            /// Parses a stored slug.
            #[must_use]
            pub fn from_slug(slug: &str) -> Self {
                match slug {
                    $($slug => Self::$variant,)+
                    other => Self::Unknown(other.to_owned()),
                }
            }
        }
    };
}
pub(crate) use slug_enum;

/// Implements a redacted `Debug` for a record type.
macro_rules! redacted_debug {
    ($($name:ident),+) => {
        $(impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(concat!(stringify!($name), "(<redacted>)"))
            }
        })+
    };
}
redacted_debug!(Session, Product, Strain, Brand, Spot, Snippet, Tag);

/// Builds a flattened field map.
#[derive(Default)]
pub(crate) struct Fields(BTreeMap<String, Value>);

impl Fields {
    pub(crate) fn put(&mut self, key: impl Into<String>, value: Value) -> &mut Self {
        if !value.is_null() {
            self.0.insert(key.into(), value);
        }
        self
    }

    pub(crate) fn opt<T>(
        &mut self,
        key: &str,
        value: Option<T>,
        f: impl FnOnce(T) -> Value,
    ) -> &mut Self {
        if let Some(v) = value {
            self.put(key, f(v));
        }
        self
    }

    pub(crate) fn text(&mut self, key: &str, value: Option<&String>) -> &mut Self {
        self.opt(key, value, |s| Value::Text(s.clone()))
    }

    pub(crate) fn map<K, V>(
        &mut self,
        prefix: &str,
        entries: impl IntoIterator<Item = (K, V)>,
        key: impl Fn(&K) -> String,
        value: impl Fn(V) -> Value,
    ) -> &mut Self {
        for (k, v) in entries {
            self.put(format!("{prefix}/{}", key(&k)), value(v));
        }
        self
    }

    pub(crate) fn tag_set(&mut self, prefix: &str, tags: &BTreeSet<TagRef>) -> &mut Self {
        self.map(
            prefix,
            tags.iter().map(|t| (t, ())),
            |t| t.to_key(),
            |()| Value::Bool(true),
        )
    }

    pub(crate) fn id_set(&mut self, prefix: &str, ids: &BTreeSet<RecordId>) -> &mut Self {
        self.map(
            prefix,
            ids.iter().map(|id| (id, ())),
            |id| id.to_hex(),
            |()| Value::Bool(true),
        )
    }

    pub(crate) fn finish(&mut self) -> BTreeMap<String, Value> {
        core::mem::take(&mut self.0)
    }
}

/// Typed reads from a document, naming the field in errors.
pub(crate) struct Reader<'a>(pub(crate) &'a Document);

impl<'a> Reader<'a> {
    pub(crate) fn new<R: Record>(doc: &'a Document) -> Result<Self> {
        if doc.kind() == R::KIND {
            Ok(Self(doc))
        } else {
            Err(Error::WrongKind)
        }
    }

    pub(crate) fn value(&self, key: &str) -> &'a Value {
        self.0.value(key)
    }

    pub(crate) fn text(&self, key: &'static str) -> Result<Option<String>> {
        match self.value(key) {
            Value::Null => Ok(None),
            Value::Text(s) => Ok(Some(s.clone())),
            _ => Err(Error::InvalidField(key)),
        }
    }

    pub(crate) fn required_text(&self, key: &'static str) -> Result<String> {
        self.text(key)?.ok_or(Error::InvalidField(key))
    }

    pub(crate) fn int_in<T: TryFrom<i64> + PartialOrd>(
        &self,
        key: &'static str,
        range: core::ops::RangeInclusive<T>,
    ) -> Result<Option<T>> {
        match self.value(key) {
            Value::Null => Ok(None),
            Value::Int(i) => T::try_from(*i)
                .ok()
                .filter(|v| range.contains(v))
                .map(Some)
                .ok_or(Error::InvalidField(key)),
            _ => Err(Error::InvalidField(key)),
        }
    }

    pub(crate) fn bool(&self, key: &'static str) -> Result<Option<bool>> {
        match self.value(key) {
            Value::Null => Ok(None),
            Value::Bool(b) => Ok(Some(*b)),
            _ => Err(Error::InvalidField(key)),
        }
    }

    pub(crate) fn id(&self, key: &'static str) -> Result<Option<RecordId>> {
        match self.value(key) {
            Value::Null => Ok(None),
            Value::Id(id) => Ok(Some(*id)),
            _ => Err(Error::InvalidField(key)),
        }
    }

    pub(crate) fn slug<T>(
        &self,
        key: &'static str,
        parse: impl Fn(&str) -> T,
    ) -> Result<Option<T>> {
        Ok(self.text(key)?.map(|s| parse(&s)))
    }

    pub(crate) fn entries(&self, prefix: &str) -> impl Iterator<Item = (&'a str, &'a Value)> {
        self.0.map_entries(prefix)
    }

    pub(crate) fn tag_set(&self, prefix: &'static str) -> Result<BTreeSet<TagRef>> {
        self.entries(prefix)
            .map(|(k, v)| match v {
                Value::Bool(true) => TagRef::from_key(k).ok_or(Error::InvalidField(prefix)),
                _ => Err(Error::InvalidField(prefix)),
            })
            .collect()
    }

    pub(crate) fn id_set(&self, prefix: &'static str) -> Result<BTreeSet<RecordId>> {
        self.entries(prefix)
            .map(|(k, v)| match v {
                Value::Bool(true) => RecordId::from_hex(k).map_err(|_| Error::InvalidField(prefix)),
                _ => Err(Error::InvalidField(prefix)),
            })
            .collect()
    }

    pub(crate) fn tag_map<T>(
        &self,
        prefix: &'static str,
        value: impl Fn(&Value) -> Option<T>,
    ) -> Result<BTreeMap<TagRef, T>> {
        self.entries(prefix)
            .map(|(k, v)| {
                Ok((
                    TagRef::from_key(k).ok_or(Error::InvalidField(prefix))?,
                    value(v).ok_or(Error::InvalidField(prefix))?,
                ))
            })
            .collect()
    }
}

/// Whether `key` is one of `scalars` or an entry under one of `maps`.
pub(crate) fn owns(key: &str, scalars: &[&str], maps: &[&str]) -> bool {
    scalars.contains(&key)
        || maps.iter().any(|m| {
            key.strip_prefix(m)
                .is_some_and(|rest| rest.starts_with('/'))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kind_codes_round_trip_and_are_stable() {
        for kind in RecordKind::ALL {
            assert_eq!(RecordKind::from_code(kind.code()), Some(kind));
        }
        assert_eq!(RecordKind::Session.code(), 1);
        assert_eq!(RecordKind::Tag.code(), 7);
        assert_eq!(RecordKind::from_code(0), None);
    }

    #[test]
    fn tag_ref_keys() {
        let id = RecordId::from_bytes([0xab; 16]);
        for tag in [TagRef::Builtin("relaxed".into()), TagRef::Custom(id)] {
            assert_eq!(TagRef::from_key(&tag.to_key()), Some(tag));
        }
        assert_eq!(TagRef::from_key("custom:zz"), None);
        assert_eq!(TagRef::from_key(""), None);
    }

    #[test]
    fn owns_matches_scalars_and_map_entries_only() {
        assert!(owns("name", &["name"], &["effects"]));
        assert!(owns("effects/relaxed", &["name"], &["effects"]));
        assert!(!owns("effects", &["name"], &["effects"]));
        assert!(!owns("effectsx/a", &["name"], &["effects"]));
        assert!(!owns("other", &["name"], &["effects"]));
    }

    #[test]
    fn local_time_validates_offset() {
        let t = LocalTime {
            unix_ms: 1,
            utc_offset_min: -300,
        };
        assert_eq!(LocalTime::from_value(&t.to_value(), "when").unwrap(), t);
        let bad = Value::Tuple(vec![Value::Int(1), Value::Int(5000)]);
        assert_eq!(
            LocalTime::from_value(&bad, "when").err(),
            Some(Error::InvalidField("when"))
        );
    }
}

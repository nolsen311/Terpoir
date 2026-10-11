// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

//! Field values.

use core::fmt;
use serde::{Deserialize, Serialize};
use terpoir_crypto::RecordId;
use zeroize::Zeroize;

/// A field value.
///
/// Wire compatibility: variants are serialized by position. Add new
/// variants only at the end, and never reorder or remove them.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Value {
    /// No value. Writing `Null` clears a field or removes a map entry.
    Null,
    /// A boolean.
    Bool(bool),
    /// A signed integer. Fractional quantities are stored as fixed-point
    /// integers (see the record types for scales).
    Int(i64),
    /// Text. Wiped from memory when the owning document is dropped.
    Text(String),
    /// A reference to another record.
    Id(#[serde(with = "id_bytes")] RecordId),
    /// Values that only make sense together, replaced as a whole.
    Tuple(Vec<Value>),
}

impl Value {
    /// Whether this is [`Value::Null`].
    #[must_use]
    pub const fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }

    /// Overwrites any text held by this value.
    pub(crate) fn wipe(&mut self) {
        match self {
            Self::Text(text) => text.zeroize(),
            Self::Tuple(items) => items.iter_mut().for_each(Self::wipe),
            Self::Null | Self::Bool(_) | Self::Int(_) | Self::Id(_) => {}
        }
    }
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Null => f.write_str("Null"),
            Self::Bool(_) => f.write_str("Bool(<redacted>)"),
            Self::Int(_) => f.write_str("Int(<redacted>)"),
            Self::Text(_) => f.write_str("Text(<redacted>)"),
            Self::Id(_) => f.write_str("Id(<redacted>)"),
            Self::Tuple(items) => write!(f, "Tuple(len {})", items.len()),
        }
    }
}

pub(crate) mod id_bytes {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use terpoir_crypto::RecordId;

    pub(crate) fn serialize<S: Serializer>(id: &RecordId, s: S) -> Result<S::Ok, S::Error> {
        id.as_bytes().serialize(s)
    }

    pub(crate) fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<RecordId, D::Error> {
        <[u8; 16]>::deserialize(d).map(RecordId::from_bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_is_redacted() {
        let v = Value::Tuple(vec![Value::Text("secret".into()), Value::Int(4)]);
        let shown = format!("{v:?} {:?}", Value::Text("secret".into()));
        assert!(!shown.contains("secret"));
    }

    #[test]
    fn wipe_clears_text() {
        let mut v = Value::Tuple(vec![Value::Text("secret".into())]);
        v.wipe();
        assert_eq!(v, Value::Tuple(vec![Value::Text(String::new())]));
    }
}

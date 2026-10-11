// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

//! Stream snippets: timestamped free text, during or after a session or
//! standalone. Each snippet is its own record, so snippets added on two
//! devices both survive a merge.

use std::collections::BTreeMap;
use terpoir_crypto::RecordId;

use super::{Fields, LocalTime, Reader, Record, RecordKind, TagRef, owns, slug_enum};
use crate::{Document, Result, Value};

slug_enum! {
    /// Where a confirmed tag came from.
    Provenance {
        /// Entered by the user.
        User => "user",
        /// Suggested by keyword matching, confirmed by the user.
        Keyword => "keyword",
        /// Suggested by the OS on-device model, confirmed by the user.
        OsModel => "os_model",
        /// Suggested by the bundled model, confirmed by the user.
        BundledModel => "bundled_model",
    }
}

/// A stream snippet.
#[derive(Clone, PartialEq, Eq)]
pub struct Snippet {
    /// The session it belongs to, or `None` for a standalone entry.
    pub session: Option<RecordId>,
    /// When it was written.
    pub at: LocalTime,
    /// The text.
    pub text: String,
    /// Confirmed tags and where each came from. Unconfirmed suggestions
    /// are never stored.
    pub tags: BTreeMap<TagRef, Provenance>,
}

impl Record for Snippet {
    const KIND: RecordKind = RecordKind::Snippet;

    fn to_fields(&self) -> BTreeMap<String, Value> {
        let mut f = Fields::default();
        f.opt("session", self.session, Value::Id)
            .put("at", self.at.to_value())
            .put("text", Value::Text(self.text.clone()))
            .map(
                "tags",
                &self.tags,
                |t| t.to_key(),
                |p| Value::Text(p.slug().to_owned()),
            );
        f.finish()
    }

    fn from_document(doc: &Document) -> Result<Self> {
        let r = Reader::new::<Self>(doc)?;
        Ok(Self {
            session: r.id("session")?,
            at: LocalTime::from_value(r.value("at"), "at")?,
            text: r.text("text")?.unwrap_or_default(),
            tags: r.tag_map("tags", |v| match v {
                Value::Text(s) => Some(Provenance::from_slug(s)),
                _ => None,
            })?,
        })
    }

    fn owns_field(key: &str) -> bool {
        owns(key, &["session", "at", "text"], &["tags"])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DeviceId, Hlc, Stamp};

    #[test]
    fn round_trip() {
        let snippet = Snippet {
            session: Some(RecordId::from_bytes([1; 16])),
            at: LocalTime {
                unix_ms: 5,
                utc_offset_min: 60,
            },
            text: "piney, very chill".into(),
            tags: [
                (TagRef::Builtin("pine".into()), Provenance::Keyword),
                (TagRef::Builtin("relaxed".into()), Provenance::User),
            ]
            .into(),
        };
        let stamp = Stamp {
            hlc: Hlc::default(),
            device: DeviceId::from_bytes([1; 16]),
        };
        let doc = Document::create(&snippet, stamp).unwrap();
        assert_eq!(Snippet::from_document(&doc).unwrap(), snippet);
    }
}

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
    /// Written while the journal was locked (write-only capture) and not
    /// yet kept by the user. Stored only while true.
    pub added_while_locked: bool,
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
        if self.added_while_locked {
            f.put("locked", Value::Bool(true));
        }
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
            added_while_locked: r.bool("locked")?.unwrap_or(false),
        })
    }

    fn owns_field(key: &str) -> bool {
        owns(key, &["session", "at", "text", "locked"], &["tags"])
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
            added_while_locked: false,
        };
        let stamp = Stamp {
            hlc: Hlc::default(),
            device: DeviceId::from_bytes([1; 16]),
        };
        let doc = Document::create(&snippet, stamp).unwrap();
        assert_eq!(Snippet::from_document(&doc).unwrap(), snippet);
        assert!(doc.field("locked").is_none());
    }

    #[test]
    fn keeping_a_locked_capture_clears_the_mark() {
        let stamp = |wall_ms| Stamp {
            hlc: Hlc {
                wall_ms,
                counter: 0,
            },
            device: DeviceId::from_bytes([1; 16]),
        };
        let mut snippet = Snippet {
            session: None,
            at: LocalTime {
                unix_ms: 5,
                utc_offset_min: 0,
            },
            text: "written while locked".into(),
            tags: BTreeMap::new(),
            added_while_locked: true,
        };
        let mut doc = Document::create(&snippet, stamp(1)).unwrap();
        assert!(Snippet::from_document(&doc).unwrap().added_while_locked);
        snippet.added_while_locked = false;
        doc.update(&snippet, stamp(2)).unwrap();
        assert_eq!(Snippet::from_document(&doc).unwrap(), snippet);
    }
}

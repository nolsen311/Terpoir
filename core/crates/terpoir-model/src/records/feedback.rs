// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

//! The user's feedback on insights (dismissed, marked wrong, category
//! muted). One record per journal, synced like any other, so feedback
//! given on one device holds on every device and survives export and
//! restore. Each insight or category is its own map entry, so feedback
//! given on two devices merges per entry (last writer wins).

use std::collections::BTreeMap;

use super::{Fields, Reader, Record, RecordKind, owns};
use crate::{Document, Error, Result, Value};

/// Feedback by insight key (or category, for "muted"): a state such as
/// `dismissed`, `wrong` or `muted`. States from a newer app are kept as
/// they are.
#[derive(Clone, Default, PartialEq, Eq, Debug)]
pub struct InsightFeedbackList {
    /// State by key.
    pub entries: BTreeMap<String, String>,
}

impl Record for InsightFeedbackList {
    const KIND: RecordKind = RecordKind::InsightFeedback;

    fn to_fields(&self) -> BTreeMap<String, Value> {
        let mut f = Fields::default();
        f.map(
            "feedback",
            self.entries.iter(),
            |k| (*k).clone(),
            |v| Value::Text(v.clone()),
        );
        f.finish()
    }

    fn from_document(doc: &Document) -> Result<Self> {
        let r = Reader::new::<Self>(doc)?;
        let mut entries = BTreeMap::new();
        for (key, value) in r.entries("feedback") {
            match value {
                Value::Text(state) => {
                    entries.insert(key.to_owned(), state.clone());
                }
                Value::Null => {}
                _ => return Err(Error::InvalidField("feedback")),
            }
        }
        Ok(Self { entries })
    }

    fn owns_field(key: &str) -> bool {
        owns(key, &[], &["feedback"])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DeviceId, Hlc, Stamp};

    fn stamp(wall_ms: u64) -> Stamp {
        Stamp {
            hlc: Hlc {
                wall_ms,
                counter: 0,
            },
            device: DeviceId::from_bytes([1; 16]),
        }
    }

    #[test]
    fn round_trips_and_forgetting_writes_removals() {
        let mut list = InsightFeedbackList::default();
        list.entries
            .insert("terpene:myrcene".into(), "dismissed".into());
        list.entries.insert("time".into(), "muted".into());
        list.entries.insert("dose".into(), "from_the_future".into());
        let mut doc = Document::create(&list, stamp(1)).unwrap();
        assert_eq!(InsightFeedbackList::from_document(&doc).unwrap(), list);

        let empty = InsightFeedbackList::default();
        assert_eq!(doc.update(&empty, stamp(2)).unwrap(), 3);
        assert!(doc.value("feedback/time").is_null());
        assert_eq!(InsightFeedbackList::from_document(&doc).unwrap(), empty);
    }
}

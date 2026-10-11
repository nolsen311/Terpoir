// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

//! The minimal on-disk index: record kind, references between records,
//! local dates and confirmed tags. Free text is never indexed.

use std::collections::BTreeSet;
use terpoir_model::{Document, RecordId, RecordKind, Value};

/// Map fields whose entry keys are record IDs.
const ID_KEYED_MAPS: &[&str] = &["strains"];
/// Map fields whose entry keys are tag references.
const TAG_KEYED_MAPS: &[&str] = &["effects", "symptoms", "tags", "purposes", "terpenes"];
const CUSTOM_TAG_PREFIX: &str = "custom:";
const MS_PER_DAY: i64 = 86_400_000;

/// Index rows for one record.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct IndexEntries {
    pub(crate) refs: BTreeSet<RecordId>,
    pub(crate) day: Option<i64>,
    pub(crate) tags: BTreeSet<String>,
}

impl IndexEntries {
    pub(crate) fn from_document(doc: &Document) -> Self {
        let mut entries = Self::default();
        for (key, field) in doc.fields() {
            if field.value.is_null() {
                continue;
            }
            collect_ids(&field.value, &mut entries.refs);
            let Some((map, entry)) = key.split_once('/') else {
                continue;
            };
            if ID_KEYED_MAPS.contains(&map)
                && let Ok(id) = RecordId::from_hex(entry)
            {
                entries.refs.insert(id);
            }
            if TAG_KEYED_MAPS.contains(&map) {
                if let Some(hex) = entry.strip_prefix(CUSTOM_TAG_PREFIX)
                    && let Ok(id) = RecordId::from_hex(hex)
                {
                    entries.refs.insert(id);
                }
                entries.tags.insert(entry.to_owned());
            }
        }
        entries.refs.remove(&doc.id());
        let date_field = match doc.kind() {
            RecordKind::Session => Some("when"),
            RecordKind::Snippet => Some("at"),
            RecordKind::Purchase => Some("when"),
            _ => None,
        };
        entries.day = date_field.and_then(|f| local_day(doc.value(f)));
        entries
    }
}

fn collect_ids(value: &Value, out: &mut BTreeSet<RecordId>) {
    match value {
        Value::Id(id) => {
            out.insert(*id);
        }
        Value::Tuple(items) => items.iter().for_each(|v| collect_ids(v, out)),
        _ => {}
    }
}

/// Days since the Unix epoch in the time zone where the entry was recorded.
fn local_day(value: &Value) -> Option<i64> {
    let Value::Tuple(items) = value else {
        return None;
    };
    let [Value::Int(ms), Value::Int(offset_min)] = items.as_slice() else {
        return None;
    };
    let local = ms.checked_add(offset_min.checked_mul(60_000)?)?;
    Some(local.div_euclid(MS_PER_DAY))
}

#[cfg(test)]
mod tests {
    use super::*;
    use terpoir_model::{
        CoreEffect, DeviceId, Hlc, IdentifierKind, LocalTime, Method, Product, ProductLine,
        Purchase, PurchaseLine, Session, Stamp, TagRef,
    };

    fn stamp() -> Stamp {
        Stamp {
            hlc: Hlc::default(),
            device: DeviceId::from_bytes([1; 16]),
        }
    }

    fn id(n: u8) -> RecordId {
        RecordId::from_bytes([n; 16])
    }

    #[test]
    fn session_index() {
        let mut s = Session::new(LocalTime {
            // 2026-10-11T02:00Z, which is still 2026-10-10 at UTC-7.
            unix_ms: 1_760_148_000_000,
            utc_offset_min: -420,
        });
        s.lines.insert(
            id(1),
            ProductLine {
                product: Some(id(2)),
                method: Method::Edible,
                dose: None,
                purchase: None,
            },
        );
        s.spot = Some(id(3));
        s.effects.insert(CoreEffect::Relaxed.into(), 3);
        s.effects.insert(TagRef::Custom(id(4)), 1);
        s.activity = Some("secret free text".into());
        let doc = Document::create(&s, stamp()).unwrap();
        let entries = IndexEntries::from_document(&doc);
        assert_eq!(entries.refs, [id(2), id(3), id(4)].into());
        assert_eq!(entries.day, Some(1_760_148_000_000 / MS_PER_DAY - 1));
        assert_eq!(
            entries.tags,
            ["relaxed".to_owned(), format!("custom:{}", id(4).to_hex())].into()
        );
    }

    #[test]
    fn product_strain_refs() {
        let mut p = Product::new("x".into());
        p.strains.insert(id(7));
        p.brand = Some(id(8));
        let doc = Document::create(&p, stamp()).unwrap();
        let entries = IndexEntries::from_document(&doc);
        assert_eq!(entries.refs, [id(7), id(8)].into());
        assert_eq!(entries.day, None);
    }

    #[test]
    fn purchase_index() {
        let mut p = Purchase::new(LocalTime {
            unix_ms: 1_760_148_000_000,
            utc_offset_min: 0,
        });
        p.spot = Some(id(5));
        p.lines.insert(
            id(1),
            PurchaseLine {
                product: Some(id(2)),
                batch: Some("secret batch".into()),
                price: None,
            },
        );
        let doc = Document::create(&p, stamp()).unwrap();
        let entries = IndexEntries::from_document(&doc);
        assert_eq!(entries.refs, [id(2), id(5)].into());
        assert_eq!(entries.day, Some(1_760_148_000_000 / MS_PER_DAY));
        assert!(entries.tags.is_empty());
    }

    #[test]
    fn identifiers_are_not_indexed() {
        let mut p = Product::new("x".into());
        p.identifiers
            .insert("0123456789012".into(), IdentifierKind::Barcode);
        let doc = Document::create(&p, stamp()).unwrap();
        assert_eq!(IndexEntries::from_document(&doc), IndexEntries::default());
    }

    #[test]
    fn negative_times_floor_correctly() {
        let v = Value::Tuple(vec![Value::Int(-1), Value::Int(0)]);
        assert_eq!(local_day(&v), Some(-1));
    }
}

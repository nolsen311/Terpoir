// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

//! User-defined tags (custom effects, symptoms, flavors, spot purposes).

use std::collections::BTreeMap;

use super::{Fields, Reader, Record, RecordKind, owns, slug_enum};
use crate::{Document, Result, Value};

slug_enum! {
    /// What a tag describes.
    TagCategory {
        /// An effect.
        Effect => "effect",
        /// A terpene.
        Terpene => "terpene",
        /// A flavor or aroma.
        Flavor => "flavor",
        /// A mood.
        Mood => "mood",
        /// A targeted symptom (never a condition or diagnosis).
        Symptom => "symptom",
        /// A spot purpose.
        SpotPurpose => "spot_purpose",
        /// Anything else.
        Other => "other",
    }
}

slug_enum! {
    /// Built-in flavors and aromas (the tag map also accepts custom ones).
    /// Slugs are unique across every built-in vocabulary.
    Flavor {
        /// Citrus.
        Citrus => "citrus",
        /// Berry.
        Berry => "berry",
        /// Sweet.
        Sweet => "sweet",
        /// Earthy.
        Earthy => "earthy",
        /// Pine.
        Pine => "pine",
        /// Diesel or fuel.
        Diesel => "diesel",
        /// Skunk.
        Skunk => "skunk",
        /// Spicy or peppery.
        Spicy => "spicy",
        /// Herbal.
        Herbal => "herbal",
        /// Floral.
        Floral => "floral",
        /// Woody.
        Woody => "woody",
        /// Minty.
        Minty => "minty",
    }
}

slug_enum! {
    /// Built-in moods (the tag map also accepts custom ones). Slugs are
    /// unique across every built-in vocabulary.
    Mood {
        /// Happy.
        Happy => "happy",
        /// Calm.
        Calm => "calm",
        /// Content.
        Content => "content",
        /// Energetic.
        Energetic => "energetic",
        /// Stressed.
        Stressed => "stressed",
        /// Irritable.
        Irritable => "irritable",
        /// Sad.
        Sad => "sad",
        /// Tired.
        Tired => "tired",
    }
}

/// A user-defined tag.
#[derive(Clone, PartialEq, Eq)]
pub struct Tag {
    /// Display name.
    pub name: String,
    /// What it describes.
    pub category: TagCategory,
}

impl Record for Tag {
    const KIND: RecordKind = RecordKind::Tag;

    fn to_fields(&self) -> BTreeMap<String, Value> {
        let mut f = Fields::default();
        f.put("name", Value::Text(self.name.clone()))
            .put("category", Value::Text(self.category.slug().to_owned()));
        f.finish()
    }

    fn from_document(doc: &Document) -> Result<Self> {
        let r = Reader::new::<Self>(doc)?;
        Ok(Self {
            name: r.required_text("name")?,
            category: r
                .slug("category", TagCategory::from_slug)?
                .unwrap_or(TagCategory::Other),
        })
    }

    fn owns_field(key: &str) -> bool {
        owns(key, &["name", "category"], &[])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DeviceId, Hlc, Stamp};

    #[test]
    fn round_trip() {
        let tag = Tag {
            name: "giggly".into(),
            category: TagCategory::Effect,
        };
        let stamp = Stamp {
            hlc: Hlc::default(),
            device: DeviceId::from_bytes([1; 16]),
        };
        let doc = Document::create(&tag, stamp).unwrap();
        assert_eq!(Tag::from_document(&doc).unwrap(), tag);
    }
}

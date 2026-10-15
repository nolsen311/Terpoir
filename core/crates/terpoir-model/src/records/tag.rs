// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

//! User-defined tags (custom effects, symptoms, flavors, spot purposes,
//! product subtypes).

use std::collections::BTreeMap;

use super::{Fields, Form, Method, Reader, Record, RecordKind, owns, slug_enum};
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
        /// A product subtype, under a form (since 0.6.0).
        Subtype => "subtype",
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
    /// A subtype's parent form (since 0.6.0). Stored only when set.
    pub form: Option<Form>,
    /// A subtype's usual method, when it differs from its form's (since
    /// 0.6.0). Stored only when set.
    pub method: Option<Method>,
}

impl Tag {
    /// A tag with a name and a category, and no subtype details.
    #[must_use]
    pub const fn new(name: String, category: TagCategory) -> Self {
        Self {
            name,
            category,
            form: None,
            method: None,
        }
    }
}

impl Record for Tag {
    const KIND: RecordKind = RecordKind::Tag;

    fn to_fields(&self) -> BTreeMap<String, Value> {
        let mut f = Fields::default();
        f.put("name", Value::Text(self.name.clone()))
            .put("category", Value::Text(self.category.slug().to_owned()))
            .opt("form", self.form.as_ref(), |v| {
                Value::Text(v.slug().to_owned())
            })
            .opt("method", self.method.as_ref(), |v| {
                Value::Text(v.slug().to_owned())
            });
        f.finish()
    }

    fn from_document(doc: &Document) -> Result<Self> {
        let r = Reader::new::<Self>(doc)?;
        Ok(Self {
            name: r.required_text("name")?,
            category: r
                .slug("category", TagCategory::from_slug)?
                .unwrap_or(TagCategory::Other),
            form: r.slug("form", Form::from_slug)?,
            method: r.slug("method", Method::from_slug)?,
        })
    }

    fn owns_field(key: &str) -> bool {
        owns(key, &["name", "category", "form", "method"], &[])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DeviceId, Hlc, Stamp};

    fn stamp() -> Stamp {
        Stamp {
            hlc: Hlc::default(),
            device: DeviceId::from_bytes([1; 16]),
        }
    }

    #[test]
    fn round_trip() {
        let tag = Tag::new("giggly".into(), TagCategory::Effect);
        let doc = Document::create(&tag, stamp()).unwrap();
        assert_eq!(Tag::from_document(&doc).unwrap(), tag);
        // Subtype details are stored only when set.
        assert_eq!(doc.fields().len(), 2);
    }

    #[test]
    fn a_custom_subtype_round_trips_with_its_form_and_method() {
        let tag = Tag {
            name: "Strip".into(),
            category: TagCategory::Subtype,
            form: Some(Form::Edible),
            method: Some(Method::Tincture),
        };
        let doc = Document::create(&tag, stamp()).unwrap();
        assert_eq!(doc.value("form"), &Value::Text("edible".into()));
        assert_eq!(doc.value("method"), &Value::Text("tincture".into()));
        assert_eq!(Tag::from_document(&doc).unwrap(), tag);
        // Values from a newer app version are kept.
        let newer = Tag {
            form: Some(Form::from_slug("suppository")),
            method: Some(Method::from_slug("rectal")),
            ..tag
        };
        let doc = Document::create(&newer, stamp()).unwrap();
        assert_eq!(Tag::from_document(&doc).unwrap(), newer);
        assert!(Tag::owns_field("form") && Tag::owns_field("method"));
    }
}

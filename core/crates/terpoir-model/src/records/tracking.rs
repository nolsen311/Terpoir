// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

//! "What I track": the data points the user has hidden. One record per
//! journal, synced like any other, so every device shows the same fields.
//! Each hidden data point is its own map entry, so choices made on two
//! devices merge per data point (last writer wins).

use std::collections::{BTreeMap, BTreeSet};

use super::{Fields, Reader, Record, RecordKind, owns, slug_enum};
use crate::{Document, Error, Result, Value};

slug_enum! {
    /// A data point the user can hide. When, product, method, effects and
    /// rating are always tracked, so they are not here.
    TrackedField {
        /// Brands.
        Brand => "brand",
        /// Strains and strain types.
        Strain => "strain",
        /// Prices and the shops things were bought at.
        PriceAndShop => "price_and_shop",
        /// Purchases as a whole (which also hides prices and shops).
        Purchases => "purchases",
        /// THC and CBD amounts.
        Potency => "potency",
        /// Terpenes.
        Terpenes => "terpenes",
        /// Batch and package date.
        Batch => "batch",
        /// Dose.
        Dose => "dose",
        /// Onset, peak and duration.
        Timeline => "timeline",
        /// Mood before and after.
        Mood => "mood",
        /// Intention.
        Intention => "intention",
        /// Solo or with others.
        Social => "social",
    }
}

impl TrackedField {
    /// Every data point this app version knows, in display order.
    #[must_use]
    pub fn all() -> Vec<Self> {
        Self::SLUGS
            .iter()
            .map(|slug| Self::from_slug(slug))
            .collect()
    }
}

slug_enum! {
    /// How much a strain records (docs/design-decisions.md, "Richer
    /// strains"). One choice for the whole journal; a lower step hides the
    /// extra fields and never deletes them.
    StrainDetail {
        /// Indica, sativa or hybrid only.
        Simple => "simple",
        /// Also which terpenes the strain is known for.
        Terpenes => "terpenes",
        /// Also terpene levels, parent strains and further cannabinoids.
        Detailed => "detailed",
    }
}

/// The hidden data points. Empty (everything tracked) by default.
#[derive(Clone, Default, PartialEq, Eq, Debug)]
pub struct TrackingPreferences {
    /// Hidden data points, including any from a newer app version, which
    /// are kept as they are.
    pub hidden: BTreeSet<TrackedField>,
    /// The strain detail step, if the user chose one; see
    /// [`strain_detail`](Self::strain_detail).
    pub strain_detail: Option<StrainDetail>,
}

impl TrackingPreferences {
    /// The strain detail step: Terpenes until the user chooses, which is
    /// what strains recorded before the setting existed. A step from a
    /// newer app version counts as Detailed, so nothing is hidden.
    #[must_use]
    pub fn strain_detail(&self) -> StrainDetail {
        match &self.strain_detail {
            None => StrainDetail::Terpenes,
            Some(StrainDetail::Unknown(_)) => StrainDetail::Detailed,
            Some(step) => step.clone(),
        }
    }

    /// Whether `field` is tracked (shown). Hiding purchases also hides
    /// prices and shops.
    #[must_use]
    pub fn tracks(&self, field: &TrackedField) -> bool {
        if *field == TrackedField::PriceAndShop && self.hidden.contains(&TrackedField::Purchases) {
            return false;
        }
        !self.hidden.contains(field)
    }
}

impl Record for TrackingPreferences {
    const KIND: RecordKind = RecordKind::Tracking;

    fn to_fields(&self) -> BTreeMap<String, Value> {
        let mut f = Fields::default();
        f.map(
            "hidden",
            self.hidden.iter().map(|h| (h, ())),
            |h| h.slug().to_owned(),
            |()| Value::Bool(true),
        )
        .opt("strain_detail", self.strain_detail.as_ref(), |d| {
            Value::Text(d.slug().to_owned())
        });
        f.finish()
    }

    fn from_document(doc: &Document) -> Result<Self> {
        let r = Reader::new::<Self>(doc)?;
        let mut hidden = BTreeSet::new();
        for (slug, value) in r.entries("hidden") {
            match value {
                Value::Bool(true) => {
                    hidden.insert(TrackedField::from_slug(slug));
                }
                Value::Null | Value::Bool(false) => {}
                _ => return Err(Error::InvalidField("hidden")),
            }
        }
        Ok(Self {
            hidden,
            strain_detail: r.slug("strain_detail", StrainDetail::from_slug)?,
        })
    }

    fn owns_field(key: &str) -> bool {
        owns(key, &["strain_detail"], &["hidden"])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DeviceId, Hlc, Stamp};

    fn stamp(wall_ms: u64, device: u8) -> Stamp {
        Stamp {
            hlc: Hlc {
                wall_ms,
                counter: 0,
            },
            device: DeviceId::from_bytes([device; 16]),
        }
    }

    #[test]
    fn round_trip_keeps_unknown_fields() {
        let prefs = TrackingPreferences {
            hidden: [
                TrackedField::Brand,
                TrackedField::Mood,
                TrackedField::from_slug("vibes"),
            ]
            .into(),
            strain_detail: Some(StrainDetail::Detailed),
        };
        let doc = Document::create(&prefs, stamp(1, 1)).unwrap();
        assert_eq!(TrackingPreferences::from_document(&doc).unwrap(), prefs);
    }

    #[test]
    fn showing_again_writes_a_removal() {
        let mut prefs = TrackingPreferences {
            hidden: [TrackedField::Brand, TrackedField::Dose].into(),
            strain_detail: None,
        };
        let mut doc = Document::create(&prefs, stamp(1, 1)).unwrap();
        prefs.hidden.remove(&TrackedField::Brand);
        assert_eq!(doc.update(&prefs, stamp(2, 1)).unwrap(), 1);
        assert!(doc.value("hidden/brand").is_null());
        assert_eq!(TrackingPreferences::from_document(&doc).unwrap(), prefs);
    }

    #[test]
    fn hiding_purchases_hides_prices_and_shops() {
        let prefs = TrackingPreferences {
            hidden: [TrackedField::Purchases].into(),
            strain_detail: None,
        };
        assert!(!prefs.tracks(&TrackedField::PriceAndShop));
        assert!(!prefs.tracks(&TrackedField::Purchases));
        assert!(prefs.tracks(&TrackedField::Brand));
        assert!(TrackingPreferences::default().tracks(&TrackedField::PriceAndShop));
    }

    #[test]
    fn strain_detail_defaults_to_terpenes_and_never_hides_on_unknown() {
        let mut prefs = TrackingPreferences::default();
        assert_eq!(prefs.strain_detail(), StrainDetail::Terpenes);
        prefs.strain_detail = Some(StrainDetail::Simple);
        assert_eq!(prefs.strain_detail(), StrainDetail::Simple);
        // A step a newer app added shows everything this app knows.
        prefs.strain_detail = Some(StrainDetail::from_slug("lab_grade"));
        assert_eq!(prefs.strain_detail(), StrainDetail::Detailed);
        let doc = Document::create(&prefs, stamp(1, 1)).unwrap();
        assert_eq!(TrackingPreferences::from_document(&doc).unwrap(), prefs);
    }

    #[test]
    fn every_known_field_has_a_unique_slug() {
        let all = TrackedField::all();
        assert_eq!(all.len(), TrackedField::SLUGS.len());
        assert!(all.iter().all(|f| !matches!(f, TrackedField::Unknown(_))));
    }
}

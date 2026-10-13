// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

//! Record types as the platform sees them, and conversions to and from
//! the core model. IDs are 32-character hex strings; vocabulary values
//! (methods, forms, effects, ...) are slugs, and slugs this version does
//! not know are kept unchanged.
//!
//! These types carry journal content, so none implements `Debug`.

use std::collections::{BTreeMap, BTreeSet};
use terpoir_model::{
    Brand, Dose, DoseUnit, Form, IdentifierKind, Intention, LabelAmount, LabelUnit, LocalTime,
    Location, Method, Price, Product, ProductLine, Provenance, Purchase, PurchaseLine, PurchaseRef,
    RecordId, Session, Snippet, Social, Spot, Strain, StrainType, SymptomResult, Tag, TagCategory,
    TagRef, Timeline,
};

use crate::{CoreError, Result};

pub(crate) fn parse_id(id: &str) -> Result<RecordId> {
    RecordId::from_hex(id).map_err(|_| CoreError::InvalidInput)
}

fn parse_ids(ids: &[String]) -> Result<BTreeSet<RecordId>> {
    ids.iter().map(|id| parse_id(id)).collect()
}

fn ids_to_strings(ids: &BTreeSet<RecordId>) -> Vec<String> {
    ids.iter().map(RecordId::to_hex).collect()
}

fn opt_id(id: Option<&String>) -> Result<Option<RecordId>> {
    id.map(|id| parse_id(id)).transpose()
}

fn in_range(value: Option<u8>, range: core::ops::RangeInclusive<u8>) -> Result<Option<u8>> {
    match value {
        Some(v) if !range.contains(&v) => Err(CoreError::InvalidInput),
        other => Ok(other),
    }
}

/// A local date and time.
#[derive(Clone, Copy, PartialEq, Eq, uniffi::Record)]
pub struct LocalTimeData {
    /// Milliseconds since the Unix epoch.
    pub unix_ms: i64,
    /// UTC offset in minutes where it was recorded.
    pub utc_offset_min: i16,
}

impl From<LocalTime> for LocalTimeData {
    fn from(t: LocalTime) -> Self {
        Self {
            unix_ms: t.unix_ms,
            utc_offset_min: t.utc_offset_min,
        }
    }
}

impl From<LocalTimeData> for LocalTime {
    fn from(t: LocalTimeData) -> Self {
        Self {
            unix_ms: t.unix_ms,
            utc_offset_min: t.utc_offset_min,
        }
    }
}

/// A tag: a built-in vocabulary slug or a custom tag record.
#[derive(Clone, PartialEq, Eq, uniffi::Enum)]
pub enum TagRefData {
    /// Built-in vocabulary entry, by slug (for example `relaxed`).
    Builtin {
        /// The slug.
        slug: String,
    },
    /// A custom tag record.
    Custom {
        /// The tag record's ID.
        id: String,
    },
}

impl TagRefData {
    fn to_model(&self) -> Result<TagRef> {
        match self {
            Self::Builtin { slug } if !slug.is_empty() && !slug.starts_with("custom:") => {
                Ok(TagRef::Builtin(slug.clone()))
            }
            Self::Builtin { .. } => Err(CoreError::InvalidInput),
            Self::Custom { id } => Ok(TagRef::Custom(parse_id(id)?)),
        }
    }

    pub(crate) fn from_model(tag: &TagRef) -> Self {
        match tag {
            TagRef::Builtin(slug) => Self::Builtin { slug: slug.clone() },
            TagRef::Custom(id) => Self::Custom { id: id.to_hex() },
        }
    }
}

fn tags_to_model(tags: &[TagRefData]) -> Result<BTreeSet<TagRef>> {
    tags.iter().map(TagRefData::to_model).collect()
}

fn tags_from_model(tags: &BTreeSet<TagRef>) -> Vec<TagRefData> {
    tags.iter().map(TagRefData::from_model).collect()
}

/// A tag with an intensity or amount.
#[derive(Clone, PartialEq, Eq, uniffi::Record)]
pub struct TagIntensity {
    /// The tag.
    pub tag: TagRefData,
    /// Effects: intensity 0 to 5. Terpenes: thousandths of a percent.
    pub value: i64,
}

/// A targeted symptom and its outcome.
#[derive(Clone, PartialEq, Eq, uniffi::Record)]
pub struct SymptomData {
    /// The symptom.
    pub tag: TagRefData,
    /// `helped` or `did_not_help`.
    pub result: String,
}

/// A confirmed tag on a snippet and where it came from.
#[derive(Clone, PartialEq, Eq, uniffi::Record)]
pub struct TagProvenance {
    /// The tag.
    pub tag: TagRefData,
    /// `user`, `keyword`, `os_model` or `bundled_model`.
    pub provenance: String,
}

/// One product consumed in a session.
#[derive(Clone, PartialEq, Eq, uniffi::Record)]
pub struct ProductLineData {
    /// The line's ID; `None` for a new line.
    pub line_id: Option<String>,
    /// The product's ID, if recorded.
    pub product: Option<String>,
    /// Method slug, for example `edible`.
    pub method: String,
    /// Dose in thousandths of `dose_unit` (2.5 mg is 2500).
    pub dose_milli: Option<i64>,
    /// Dose unit slug, for example `mg`. Set with `dose_milli` or not at all.
    pub dose_unit: Option<String>,
    /// The purchase this unit came from. Set with `purchase_line_id` or not
    /// at all.
    pub purchase_id: Option<String>,
    /// The line within that purchase.
    pub purchase_line_id: Option<String>,
    /// The Re-up Spot it was bought at, when the user picked one.
    pub shop: Option<String>,
}

/// A session (journal entry).
#[derive(Clone, PartialEq, Eq, uniffi::Record)]
pub struct SessionData {
    /// When it happened.
    pub when: LocalTimeData,
    /// Products consumed.
    pub lines: Vec<ProductLineData>,
    /// What the user was doing.
    pub activity: Option<String>,
    /// Intention slug.
    pub intention: Option<String>,
    /// Mood before, 1 to 5.
    pub mood_before: Option<u8>,
    /// `solo` or `with_others`.
    pub social: Option<String>,
    /// Effects with intensity 0 to 5.
    pub effects: Vec<TagIntensity>,
    /// Minutes to onset.
    pub onset_min: Option<u32>,
    /// Minutes to peak.
    pub peak_min: Option<u32>,
    /// Total minutes.
    pub duration_min: Option<u32>,
    /// Mood after, 1 to 5.
    pub mood_after: Option<u8>,
    /// Rating, 1 to 5.
    pub rating: Option<u8>,
    /// Would repeat.
    pub would_repeat: Option<bool>,
    /// Attached spot's ID.
    pub spot: Option<String>,
    /// Targeted symptoms.
    pub symptoms: Vec<SymptomData>,
    /// Attachment IDs.
    pub attachments: Vec<String>,
}

impl SessionData {
    pub(crate) fn to_model(&self) -> Result<Session> {
        let mut lines = BTreeMap::new();
        for line in &self.lines {
            let id = match &line.line_id {
                Some(id) => parse_id(id)?,
                None => RecordId::random()?,
            };
            let dose = match (line.dose_milli, &line.dose_unit) {
                (None, None) => None,
                (Some(amount_milli), Some(unit)) => Some(Dose {
                    amount_milli,
                    unit: DoseUnit::from_slug(unit),
                }),
                _ => return Err(CoreError::InvalidInput),
            };
            let purchase = match (&line.purchase_id, &line.purchase_line_id) {
                (None, None) => None,
                (Some(purchase), Some(line)) => Some(PurchaseRef {
                    purchase: parse_id(purchase)?,
                    line: parse_id(line)?,
                }),
                _ => return Err(CoreError::InvalidInput),
            };
            let line = ProductLine {
                product: opt_id(line.product.as_ref())?,
                method: Method::from_slug(&line.method),
                dose,
                purchase,
                shop: opt_id(line.shop.as_ref())?,
            };
            if lines.insert(id, line).is_some() {
                return Err(CoreError::InvalidInput);
            }
        }
        let effects = self
            .effects
            .iter()
            .map(|e| {
                let intensity = u8::try_from(e.value)
                    .ok()
                    .filter(|v| *v <= 5)
                    .ok_or(CoreError::InvalidInput)?;
                Ok((e.tag.to_model()?, intensity))
            })
            .collect::<Result<_>>()?;
        let symptoms = self
            .symptoms
            .iter()
            .map(|s| Ok((s.tag.to_model()?, SymptomResult::from_slug(&s.result))))
            .collect::<Result<_>>()?;
        Ok(Session {
            when: self.when.into(),
            lines,
            activity: self.activity.clone(),
            intention: self.intention.as_deref().map(Intention::from_slug),
            mood_before: in_range(self.mood_before, 1..=5)?,
            social: self.social.as_deref().map(Social::from_slug),
            effects,
            timeline: Timeline {
                onset_min: self.onset_min,
                peak_min: self.peak_min,
                duration_min: self.duration_min,
            },
            mood_after: in_range(self.mood_after, 1..=5)?,
            rating: in_range(self.rating, 1..=5)?,
            would_repeat: self.would_repeat,
            spot: opt_id(self.spot.as_ref())?,
            symptoms,
            attachments: parse_ids(&self.attachments)?,
        })
    }

    pub(crate) fn from_model(s: &Session) -> Self {
        Self {
            when: s.when.into(),
            lines: s
                .lines
                .iter()
                .map(|(id, line)| ProductLineData {
                    line_id: Some(id.to_hex()),
                    product: line.product.map(|p| p.to_hex()),
                    method: line.method.slug().to_owned(),
                    dose_milli: line.dose.as_ref().map(|d| d.amount_milli),
                    dose_unit: line.dose.as_ref().map(|d| d.unit.slug().to_owned()),
                    purchase_id: line.purchase.map(|r| r.purchase.to_hex()),
                    purchase_line_id: line.purchase.map(|r| r.line.to_hex()),
                    shop: line.shop.map(|s| s.to_hex()),
                })
                .collect(),
            activity: s.activity.clone(),
            intention: s.intention.as_ref().map(|i| i.slug().to_owned()),
            mood_before: s.mood_before,
            social: s.social.as_ref().map(|v| v.slug().to_owned()),
            effects: s
                .effects
                .iter()
                .map(|(tag, v)| TagIntensity {
                    tag: TagRefData::from_model(tag),
                    value: i64::from(*v),
                })
                .collect(),
            onset_min: s.timeline.onset_min,
            peak_min: s.timeline.peak_min,
            duration_min: s.timeline.duration_min,
            mood_after: s.mood_after,
            rating: s.rating,
            would_repeat: s.would_repeat,
            spot: s.spot.map(|id| id.to_hex()),
            symptoms: s
                .symptoms
                .iter()
                .map(|(tag, r)| SymptomData {
                    tag: TagRefData::from_model(tag),
                    result: r.slug().to_owned(),
                })
                .collect(),
            attachments: ids_to_strings(&s.attachments),
        }
    }
}

/// A product.
#[derive(Clone, PartialEq, Eq, uniffi::Record)]
pub struct ProductData {
    /// Name.
    pub name: String,
    /// Form slug, for example `flower`.
    pub form: Option<String>,
    /// Brand ID.
    pub brand: Option<String>,
    /// Strain IDs.
    pub strains: Vec<String>,
    /// THC amount in thousandths of `thc_unit`.
    pub thc_milli: Option<i64>,
    /// `percent`, `mg_per_unit` or `mg_total`.
    pub thc_unit: Option<String>,
    /// CBD amount in thousandths of `cbd_unit`.
    pub cbd_milli: Option<i64>,
    /// `percent`, `mg_per_unit` or `mg_total`.
    pub cbd_unit: Option<String>,
    /// Label terpenes, in thousandths of a percent.
    pub terpenes: Vec<TagIntensity>,
    /// Batch or lot.
    pub batch: Option<String>,
    /// Package or harvest date, in days since 1970-01-01.
    pub package_date_day: Option<i64>,
    /// Barcodes, QR payloads and package tags read from the packaging.
    pub identifiers: Vec<IdentifierData>,
    /// Re-up Spot IDs it is bought at ("Where I buy it").
    pub shops: Vec<String>,
}

/// A code read from a product's packaging.
#[derive(Clone, PartialEq, Eq, uniffi::Record)]
pub struct IdentifierData {
    /// The code exactly as read. Must not be empty.
    pub code: String,
    /// `barcode`, `qr` or `package_tag`.
    pub kind: String,
}

fn label(milli: Option<i64>, unit: Option<&String>) -> Result<Option<LabelAmount>> {
    match (milli, unit) {
        (None, None) => Ok(None),
        (Some(milli), Some(unit)) => Ok(Some(LabelAmount {
            milli,
            unit: LabelUnit::from_slug(unit),
        })),
        _ => Err(CoreError::InvalidInput),
    }
}

impl ProductData {
    pub(crate) fn to_model(&self) -> Result<Product> {
        let mut identifiers = BTreeMap::new();
        for i in &self.identifiers {
            if i.code.is_empty() {
                return Err(CoreError::InvalidInput);
            }
            identifiers.insert(i.code.clone(), IdentifierKind::from_slug(&i.kind));
        }
        Ok(Product {
            name: self.name.clone(),
            form: self.form.as_deref().map(Form::from_slug),
            brand: opt_id(self.brand.as_ref())?,
            strains: parse_ids(&self.strains)?,
            thc: label(self.thc_milli, self.thc_unit.as_ref())?,
            cbd: label(self.cbd_milli, self.cbd_unit.as_ref())?,
            terpenes: self
                .terpenes
                .iter()
                .map(|t| Ok((t.tag.to_model()?, t.value)))
                .collect::<Result<_>>()?,
            batch: self.batch.clone(),
            package_date: self.package_date_day,
            identifiers,
            shops: parse_ids(&self.shops)?,
        })
    }

    pub(crate) fn from_model(p: &Product) -> Self {
        Self {
            name: p.name.clone(),
            form: p.form.as_ref().map(|f| f.slug().to_owned()),
            brand: p.brand.map(|id| id.to_hex()),
            strains: ids_to_strings(&p.strains),
            thc_milli: p.thc.as_ref().map(|l| l.milli),
            thc_unit: p.thc.as_ref().map(|l| l.unit.slug().to_owned()),
            cbd_milli: p.cbd.as_ref().map(|l| l.milli),
            cbd_unit: p.cbd.as_ref().map(|l| l.unit.slug().to_owned()),
            terpenes: p
                .terpenes
                .iter()
                .map(|(tag, v)| TagIntensity {
                    tag: TagRefData::from_model(tag),
                    value: *v,
                })
                .collect(),
            batch: p.batch.clone(),
            package_date_day: p.package_date,
            identifiers: p
                .identifiers
                .iter()
                .map(|(code, kind)| IdentifierData {
                    code: code.clone(),
                    kind: kind.slug().to_owned(),
                })
                .collect(),
            shops: ids_to_strings(&p.shops),
        }
    }
}

/// A strain.
#[derive(Clone, PartialEq, Eq, uniffi::Record)]
pub struct StrainData {
    /// Name.
    pub name: String,
    /// Lineage.
    pub lineage: Option<String>,
    /// Dominant terpenes.
    pub terpenes: Vec<TagRefData>,
    /// `indica`, `sativa` or `hybrid`.
    pub strain_type: Option<String>,
}

impl StrainData {
    pub(crate) fn to_model(&self) -> Result<Strain> {
        Ok(Strain {
            name: self.name.clone(),
            lineage: self.lineage.clone(),
            terpenes: tags_to_model(&self.terpenes)?,
            strain_type: self.strain_type.as_deref().map(StrainType::from_slug),
        })
    }

    pub(crate) fn from_model(s: &Strain) -> Self {
        Self {
            name: s.name.clone(),
            lineage: s.lineage.clone(),
            terpenes: tags_from_model(&s.terpenes),
            strain_type: s.strain_type.as_ref().map(|t| t.slug().to_owned()),
        }
    }
}

/// A brand.
#[derive(Clone, PartialEq, Eq, uniffi::Record)]
pub struct BrandData {
    /// Name.
    pub name: String,
}

/// A saved spot.
#[derive(Clone, PartialEq, Eq, uniffi::Record)]
pub struct SpotData {
    /// User-chosen name.
    pub name: String,
    /// Purposes (`blaze`, `munch`, `re_up`, `chill`, or custom).
    pub purposes: Vec<TagRefData>,
    /// Latitude in 1e-7 degrees. Set all three location fields or none.
    pub lat_e7: Option<i32>,
    /// Longitude in 1e-7 degrees.
    pub lon_e7: Option<i32>,
    /// Accuracy radius in metres.
    pub accuracy_m: Option<u32>,
    /// Geofence radius in metres.
    pub radius_m: Option<u32>,
    /// Prompt to log a purchase on arrival (Re-up Spots; off by default).
    pub purchase_prompts: bool,
}

impl SpotData {
    pub(crate) fn to_model(&self) -> Result<Spot> {
        let location = match (self.lat_e7, self.lon_e7, self.accuracy_m) {
            (None, None, None) => None,
            (Some(lat_e7), Some(lon_e7), Some(accuracy_m))
                if lat_e7.unsigned_abs() <= 900_000_000
                    && lon_e7.unsigned_abs() <= 1_800_000_000 =>
            {
                Some(Location {
                    lat_e7,
                    lon_e7,
                    accuracy_m,
                })
            }
            _ => return Err(CoreError::InvalidInput),
        };
        Ok(Spot {
            name: self.name.clone(),
            purposes: tags_to_model(&self.purposes)?,
            location,
            radius_m: self.radius_m,
            purchase_prompts: self.purchase_prompts,
        })
    }

    pub(crate) fn from_model(s: &Spot) -> Self {
        Self {
            name: s.name.clone(),
            purposes: tags_from_model(&s.purposes),
            lat_e7: s.location.map(|l| l.lat_e7),
            lon_e7: s.location.map(|l| l.lon_e7),
            accuracy_m: s.location.map(|l| l.accuracy_m),
            radius_m: s.radius_m,
            purchase_prompts: s.purchase_prompts,
        }
    }
}

/// One product bought in a purchase.
#[derive(Clone, PartialEq, Eq, uniffi::Record)]
pub struct PurchaseLineData {
    /// The line's ID; `None` for a new line.
    pub line_id: Option<String>,
    /// The product's ID, if recorded.
    pub product: Option<String>,
    /// Batch or lot of this unit.
    pub batch: Option<String>,
    /// Price in minor units (cents).
    pub price_minor_units: Option<i64>,
    /// ISO 4217 currency code. Set with the price or not at all.
    pub price_currency: Option<String>,
}

/// A purchase.
#[derive(Clone, PartialEq, Eq, uniffi::Record)]
pub struct PurchaseData {
    /// When it was bought.
    pub when: LocalTimeData,
    /// The Re-up Spot's ID, if recorded.
    pub spot: Option<String>,
    /// What was bought.
    pub lines: Vec<PurchaseLineData>,
}

impl PurchaseData {
    pub(crate) fn to_model(&self) -> Result<Purchase> {
        let mut lines = BTreeMap::new();
        for line in &self.lines {
            let id = match &line.line_id {
                Some(id) => parse_id(id)?,
                None => RecordId::random()?,
            };
            let price = match (line.price_minor_units, &line.price_currency) {
                (None, None) => None,
                (Some(minor_units), Some(currency)) if !currency.is_empty() => Some(Price {
                    minor_units,
                    currency: currency.clone(),
                }),
                _ => return Err(CoreError::InvalidInput),
            };
            let line = PurchaseLine {
                product: opt_id(line.product.as_ref())?,
                batch: line.batch.clone(),
                price,
            };
            if lines.insert(id, line).is_some() {
                return Err(CoreError::InvalidInput);
            }
        }
        Ok(Purchase {
            when: self.when.into(),
            spot: opt_id(self.spot.as_ref())?,
            lines,
        })
    }

    pub(crate) fn from_model(p: &Purchase) -> Self {
        Self {
            when: p.when.into(),
            spot: p.spot.map(|id| id.to_hex()),
            lines: p
                .lines
                .iter()
                .map(|(id, line)| PurchaseLineData {
                    line_id: Some(id.to_hex()),
                    product: line.product.map(|p| p.to_hex()),
                    batch: line.batch.clone(),
                    price_minor_units: line.price.as_ref().map(|p| p.minor_units),
                    price_currency: line.price.as_ref().map(|p| p.currency.clone()),
                })
                .collect(),
        }
    }
}

/// A stream snippet.
#[derive(Clone, PartialEq, Eq, uniffi::Record)]
pub struct SnippetData {
    /// Session ID, or `None` for a standalone entry.
    pub session: Option<String>,
    /// When it was written.
    pub at: LocalTimeData,
    /// The text.
    pub text: String,
    /// Confirmed tags.
    pub tags: Vec<TagProvenance>,
    /// Written while the journal was locked and not yet kept: show it for
    /// review. Save it with `false` to keep it; delete it to discard it.
    #[uniffi(default = false)]
    pub added_while_locked: bool,
}

impl SnippetData {
    pub(crate) fn to_model(&self) -> Result<Snippet> {
        Ok(Snippet {
            session: opt_id(self.session.as_ref())?,
            at: self.at.into(),
            text: self.text.clone(),
            tags: self
                .tags
                .iter()
                .map(|t| Ok((t.tag.to_model()?, Provenance::from_slug(&t.provenance))))
                .collect::<Result<_>>()?,
            added_while_locked: self.added_while_locked,
        })
    }

    pub(crate) fn from_model(s: &Snippet) -> Self {
        Self {
            session: s.session.map(|id| id.to_hex()),
            at: s.at.into(),
            text: s.text.clone(),
            tags: s
                .tags
                .iter()
                .map(|(tag, p)| TagProvenance {
                    tag: TagRefData::from_model(tag),
                    provenance: p.slug().to_owned(),
                })
                .collect(),
            added_while_locked: s.added_while_locked,
        }
    }
}

/// A custom tag.
#[derive(Clone, PartialEq, Eq, uniffi::Record)]
pub struct TagData {
    /// Display name.
    pub name: String,
    /// `effect`, `terpene`, `flavor`, `mood`, `symptom`, `spot_purpose` or
    /// `other`.
    pub category: String,
}

impl TagData {
    pub(crate) fn to_model(&self) -> Tag {
        Tag {
            name: self.name.clone(),
            category: TagCategory::from_slug(&self.category),
        }
    }

    pub(crate) fn from_model(t: &Tag) -> Self {
        Self {
            name: t.name.clone(),
            category: t.category.slug().to_owned(),
        }
    }
}

impl BrandData {
    pub(crate) fn to_model(&self) -> Brand {
        Brand {
            name: self.name.clone(),
        }
    }

    pub(crate) fn from_model(b: &Brand) -> Self {
        Self {
            name: b.name.clone(),
        }
    }
}

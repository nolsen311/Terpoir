// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

//! Products, strains and brands.

use std::collections::{BTreeMap, BTreeSet};
use terpoir_crypto::RecordId;

use super::{Fields, Method, Reader, Record, RecordKind, TagRef, owns, slug_enum};
use crate::{Document, Error, Result, Value};

slug_enum! {
    /// Product form.
    Form {
        /// Flower.
        Flower => "flower",
        /// Pre-roll.
        PreRoll => "pre_roll",
        /// Vape cartridge or disposable.
        VapeCart => "vape_cart",
        /// Concentrate.
        Concentrate => "concentrate",
        /// Edible.
        Edible => "edible",
        /// Tincture.
        Tincture => "tincture",
        /// Beverage.
        Beverage => "beverage",
        /// Topical.
        Topical => "topical",
    }
}

slug_enum! {
    /// Unit of a cannabinoid label amount.
    LabelUnit {
        /// Percent by weight.
        Percent => "percent",
        /// Milligrams per serving or unit.
        MgPerUnit => "mg_per_unit",
        /// Milligrams in the whole package.
        MgTotal => "mg_total",
    }
}

slug_enum! {
    /// Common terpenes, as built-in tags for product label amounts (the map
    /// also accepts custom terpene tags).
    Terpene {
        /// Myrcene.
        Myrcene => "myrcene",
        /// Limonene.
        Limonene => "limonene",
        /// Caryophyllene (beta-caryophyllene).
        Caryophyllene => "caryophyllene",
        /// Linalool.
        Linalool => "linalool",
        /// Alpha-pinene.
        AlphaPinene => "alpha_pinene",
        /// Beta-pinene.
        BetaPinene => "beta_pinene",
        /// Humulene.
        Humulene => "humulene",
        /// Terpinolene.
        Terpinolene => "terpinolene",
        /// Ocimene.
        Ocimene => "ocimene",
        /// Bisabolol.
        Bisabolol => "bisabolol",
        /// Nerolidol.
        Nerolidol => "nerolidol",
        /// Terpineol.
        Terpineol => "terpineol",
    }
}

slug_enum! {
    /// What kind of code a scanned product identifier came from.
    IdentifierKind {
        /// A retail barcode (UPC/EAN).
        Barcode => "barcode",
        /// A QR code on the package.
        Qr => "qr",
        /// A seed-to-sale package tag (for example a state tracking ID).
        PackageTag => "package_tag",
    }
}

slug_enum! {
    /// The indica/sativa/hybrid label. Kept because packaging uses it;
    /// insights rely on terpene and cannabinoid profiles instead.
    StrainType {
        /// Indica.
        Indica => "indica",
        /// Sativa.
        Sativa => "sativa",
        /// Hybrid.
        Hybrid => "hybrid",
    }
}

slug_enum! {
    /// Roughly how much of a terpene a strain has (docs/design-decisions.md,
    /// "Richer strains"). Never an exact percentage: those stay on products,
    /// from their labels.
    TerpeneLevel {
        /// The strain's main terpene.
        Dominant => "dominant",
        /// Clearly present, behind the dominant one.
        Secondary => "secondary",
        /// A little.
        Trace => "trace",
    }
}

slug_enum! {
    /// Cannabinoids besides THC and CBD that a strain can be known for.
    Cannabinoid {
        /// Cannabinol.
        Cbn => "cbn",
        /// Cannabigerol.
        Cbg => "cbg",
        /// Tetrahydrocannabinolic acid.
        Thca => "thca",
        /// Cannabidivarin.
        Cbdv => "cbdv",
    }
}

impl Form {
    /// The usual way this form is consumed, as a session's default method.
    /// `None` for a form from a newer app version.
    #[must_use]
    pub fn default_method(&self) -> Option<Method> {
        Some(match self {
            Self::Flower | Self::PreRoll => Method::Smoked,
            Self::VapeCart => Method::VapeCart,
            Self::Concentrate => Method::Dabbed,
            Self::Edible => Method::Edible,
            Self::Tincture => Method::Tincture,
            Self::Beverage => Method::Beverage,
            Self::Topical => Method::Topical,
            Self::Unknown(_) => return None,
        })
    }
}

/// A cannabinoid amount from a label.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct LabelAmount {
    /// Amount in thousandths of `unit` (22.4 % is 22400).
    pub milli: i64,
    /// The unit.
    pub unit: LabelUnit,
}

fn label_value(amount: &LabelAmount) -> Value {
    Value::Tuple(vec![
        Value::Int(amount.milli),
        Value::Text(amount.unit.slug().to_owned()),
    ])
}

fn label_from(value: &Value, field: &'static str) -> Result<Option<LabelAmount>> {
    match value {
        Value::Null => Ok(None),
        Value::Tuple(items) => match items.as_slice() {
            [Value::Int(milli), Value::Text(unit)] => Ok(Some(LabelAmount {
                milli: *milli,
                unit: LabelUnit::from_slug(unit),
            })),
            _ => Err(Error::InvalidField(field)),
        },
        _ => Err(Error::InvalidField(field)),
    }
}

/// A product.
#[derive(Clone, PartialEq, Eq)]
pub struct Product {
    /// Name.
    pub name: String,
    /// Form.
    pub form: Option<Form>,
    /// Brand.
    pub brand: Option<RecordId>,
    /// Strains it contains.
    pub strains: BTreeSet<RecordId>,
    /// THC from the label.
    pub thc: Option<LabelAmount>,
    /// CBD from the label.
    pub cbd: Option<LabelAmount>,
    /// Top terpenes from the label, in thousandths of a percent.
    pub terpenes: BTreeMap<TagRef, i64>,
    /// Batch or lot number.
    pub batch: Option<String>,
    /// The package date printed on the label, as days since 1970-01-01
    /// (a calendar date; no time zone).
    pub package_date: Option<i64>,
    /// Scanned codes (barcode, QR, package tag), by their value. Where and
    /// for how much it was bought lives on [`crate::Purchase`] records.
    pub identifiers: BTreeMap<String, IdentifierKind>,
    /// Re-up Spots the user buys it at ("Where I buy it"): added when a
    /// receipt from a spot lists it, and editable at any time.
    pub shops: BTreeSet<RecordId>,
}

impl Product {
    /// A product with only a name.
    #[must_use]
    pub const fn new(name: String) -> Self {
        Self {
            name,
            form: None,
            brand: None,
            strains: BTreeSet::new(),
            thc: None,
            cbd: None,
            terpenes: BTreeMap::new(),
            batch: None,
            package_date: None,
            identifiers: BTreeMap::new(),
            shops: BTreeSet::new(),
        }
    }
}

impl Record for Product {
    const KIND: RecordKind = RecordKind::Product;

    fn to_fields(&self) -> BTreeMap<String, Value> {
        let mut f = Fields::default();
        f.put("name", Value::Text(self.name.clone()))
            .opt("form", self.form.as_ref(), |v| {
                Value::Text(v.slug().to_owned())
            })
            .opt("brand", self.brand, Value::Id)
            .id_set("strains", &self.strains)
            .opt("thc", self.thc.as_ref(), label_value)
            .opt("cbd", self.cbd.as_ref(), label_value)
            .map(
                "terpenes",
                &self.terpenes,
                |t| t.to_key(),
                |v| Value::Int(*v),
            )
            .text("batch", self.batch.as_ref())
            .opt("package_date", self.package_date, Value::Int)
            .map(
                "identifiers",
                &self.identifiers,
                |code| (*code).clone(),
                |kind| Value::Text(kind.slug().to_owned()),
            )
            .id_set("shops", &self.shops);
        f.finish()
    }

    fn from_document(doc: &Document) -> Result<Self> {
        let r = Reader::new::<Self>(doc)?;
        let identifiers = r
            .entries("identifiers")
            .map(|(code, kind)| match kind {
                Value::Text(kind) => Ok((code.to_owned(), IdentifierKind::from_slug(kind))),
                _ => Err(Error::InvalidField("identifiers")),
            })
            .collect::<Result<_>>()?;
        Ok(Self {
            name: r.required_text("name")?,
            form: r.slug("form", Form::from_slug)?,
            brand: r.id("brand")?,
            strains: r.id_set("strains")?,
            thc: label_from(r.value("thc"), "thc")?,
            cbd: label_from(r.value("cbd"), "cbd")?,
            terpenes: r.tag_map("terpenes", |v| match v {
                Value::Int(i) => Some(*i),
                _ => None,
            })?,
            batch: r.text("batch")?,
            package_date: r.int_in("package_date", i64::MIN..=i64::MAX)?,
            identifiers,
            shops: r.id_set("shops")?,
        })
    }

    fn owns_field(key: &str) -> bool {
        owns(
            key,
            &[
                "name",
                "form",
                "brand",
                "thc",
                "cbd",
                "batch",
                "package_date",
            ],
            &["strains", "terpenes", "identifiers", "shops"],
        )
    }
}

/// A strain or cultivar.
#[derive(Clone, PartialEq, Eq)]
pub struct Strain {
    /// Name.
    pub name: String,
    /// Lineage, free text (for example "OG Kush × Durban Poison").
    pub lineage: Option<String>,
    /// Terpenes the strain is known for.
    pub terpenes: BTreeSet<TagRef>,
    /// Indica/sativa/hybrid label.
    pub strain_type: Option<StrainType>,
    /// Rough levels for some of [`terpenes`](Self::terpenes); a terpene
    /// without one is unrated. A level for a terpene no longer in the set
    /// (after merging edits from two devices) is ignored: see
    /// [`level`](Self::level).
    pub terpene_levels: BTreeMap<TagRef, TerpeneLevel>,
    /// Parent strains (lineage), as strain records. The free-text
    /// [`lineage`](Self::lineage) stays as a note.
    pub parents: BTreeSet<RecordId>,
    /// Cannabinoids besides THC and CBD the strain is known for.
    pub cannabinoids: BTreeSet<Cannabinoid>,
}

impl Strain {
    /// A strain with only a name, as created when a parent is typed in.
    #[must_use]
    pub fn new(name: String) -> Self {
        Self {
            name,
            lineage: None,
            terpenes: BTreeSet::new(),
            strain_type: None,
            terpene_levels: BTreeMap::new(),
            parents: BTreeSet::new(),
            cannabinoids: BTreeSet::new(),
        }
    }

    /// The level of `terpene`, if it is one of this strain's terpenes and
    /// has been rated.
    #[must_use]
    pub fn level(&self, terpene: &TagRef) -> Option<&TerpeneLevel> {
        self.terpenes
            .contains(terpene)
            .then(|| self.terpene_levels.get(terpene))
            .flatten()
    }

    /// The strain's dominant terpene, for insights when a product's label
    /// lists none: the one rated dominant (the first, in a stable order,
    /// if several are), or else the only terpene listed.
    #[must_use]
    pub fn dominant_terpene(&self) -> Option<&TagRef> {
        self.terpenes
            .iter()
            .find(|t| self.level(t) == Some(&TerpeneLevel::Dominant))
            .or_else(|| {
                let mut only = self.terpenes.iter();
                match (only.next(), only.next()) {
                    (Some(t), None) => Some(t),
                    _ => None,
                }
            })
    }
}

impl Record for Strain {
    const KIND: RecordKind = RecordKind::Strain;

    fn to_fields(&self) -> BTreeMap<String, Value> {
        let mut f = Fields::default();
        f.put("name", Value::Text(self.name.clone()))
            .text("lineage", self.lineage.as_ref())
            .tag_set("terpenes", &self.terpenes)
            .opt("type", self.strain_type.as_ref(), |t| {
                Value::Text(t.slug().to_owned())
            })
            .map(
                "terpene_levels",
                &self.terpene_levels,
                |t| t.to_key(),
                |l| Value::Text(l.slug().to_owned()),
            )
            .id_set("parents", &self.parents)
            .map(
                "cannabinoids",
                self.cannabinoids.iter().map(|c| (c, ())),
                |c| c.slug().to_owned(),
                |()| Value::Bool(true),
            );
        f.finish()
    }

    fn from_document(doc: &Document) -> Result<Self> {
        let r = Reader::new::<Self>(doc)?;
        let mut cannabinoids = BTreeSet::new();
        for (slug, value) in r.entries("cannabinoids") {
            match value {
                Value::Bool(true) => {
                    cannabinoids.insert(Cannabinoid::from_slug(slug));
                }
                _ => return Err(Error::InvalidField("cannabinoids")),
            }
        }
        Ok(Self {
            name: r.required_text("name")?,
            lineage: r.text("lineage")?,
            terpenes: r.tag_set("terpenes")?,
            strain_type: r.slug("type", StrainType::from_slug)?,
            terpene_levels: r.tag_map("terpene_levels", |v| match v {
                Value::Text(slug) => Some(TerpeneLevel::from_slug(slug)),
                _ => None,
            })?,
            parents: r.id_set("parents")?,
            cannabinoids,
        })
    }

    fn owns_field(key: &str) -> bool {
        owns(
            key,
            &["name", "lineage", "type"],
            &["terpenes", "terpene_levels", "parents", "cannabinoids"],
        )
    }
}

/// A brand.
#[derive(Clone, PartialEq, Eq)]
pub struct Brand {
    /// Name.
    pub name: String,
}

impl Record for Brand {
    const KIND: RecordKind = RecordKind::Brand;

    fn to_fields(&self) -> BTreeMap<String, Value> {
        let mut f = Fields::default();
        f.put("name", Value::Text(self.name.clone()));
        f.finish()
    }

    fn from_document(doc: &Document) -> Result<Self> {
        let r = Reader::new::<Self>(doc)?;
        Ok(Self {
            name: r.required_text("name")?,
        })
    }

    fn owns_field(key: &str) -> bool {
        key == "name"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DeviceId, Hlc, Stamp};

    #[test]
    fn every_form_has_a_default_method() {
        for slug in Form::SLUGS {
            let method = Form::from_slug(slug).default_method();
            assert!(
                matches!(&method, Some(m) if !matches!(m, Method::Unknown(_))),
                "{slug}"
            );
        }
        assert_eq!(Form::PreRoll.default_method(), Some(Method::Smoked));
        assert_eq!(Form::from_slug("suppository").default_method(), None);
    }

    fn stamp() -> Stamp {
        Stamp {
            hlc: Hlc::default(),
            device: DeviceId::from_bytes([1; 16]),
        }
    }

    fn round_trip<R: Record + PartialEq + core::fmt::Debug>(record: &R) {
        let doc = Document::create(record, stamp()).unwrap();
        let bytes = doc.encode().unwrap();
        let back = crate::migrate(Document::decode(&bytes).unwrap()).unwrap();
        assert_eq!(&R::from_document(&back).unwrap(), record);
    }

    #[test]
    fn product_round_trips() {
        let mut p = Product::new("Blue Dream 3.5g".into());
        p.form = Some(Form::Flower);
        p.brand = Some(RecordId::from_bytes([1; 16]));
        p.strains.insert(RecordId::from_bytes([2; 16]));
        p.thc = Some(LabelAmount {
            milli: 22_400,
            unit: LabelUnit::Percent,
        });
        p.cbd = Some(LabelAmount {
            milli: 100,
            unit: LabelUnit::Unknown("future_unit".into()),
        });
        p.terpenes.insert(TagRef::Builtin("myrcene".into()), 1_200);
        p.batch = Some("LOT-7".into());
        p.package_date = Some(20_370);
        p.shops.insert(RecordId::from_bytes([3; 16]));
        p.identifiers
            .insert("0850012345678".into(), IdentifierKind::Barcode);
        p.identifiers.insert(
            "1A4060300003B01000012345".into(),
            IdentifierKind::PackageTag,
        );
        // A code containing the key separator still round-trips.
        p.identifiers
            .insert("https://example.test/coa/42".into(), IdentifierKind::Qr);
        round_trip(&p);
        round_trip(&Product::new("bare".into()));
    }

    #[test]
    fn strain_and_brand_round_trip() {
        round_trip(&Strain {
            name: "Blue Dream".into(),
            lineage: Some("Blueberry × Haze".into()),
            terpenes: [TagRef::Builtin("myrcene".into())].into(),
            strain_type: Some(StrainType::Hybrid),
            ..Strain::new(String::new())
        });
        let myrcene = TagRef::Builtin("myrcene".into());
        let pinene = TagRef::Builtin("alpha_pinene".into());
        round_trip(&Strain {
            name: "Jack Herer".into(),
            terpenes: [
                myrcene.clone(),
                pinene.clone(),
                TagRef::Custom(RecordId::from_bytes([7; 16])),
            ]
            .into(),
            terpene_levels: [
                (myrcene, TerpeneLevel::Secondary),
                (pinene, TerpeneLevel::Dominant),
            ]
            .into(),
            parents: [RecordId::from_bytes([4; 16]), RecordId::from_bytes([5; 16])].into(),
            cannabinoids: [
                Cannabinoid::Cbg,
                Cannabinoid::Cbdv,
                Cannabinoid::from_slug("thcv"),
            ]
            .into(),
            ..Strain::new(String::new())
        });
        round_trip(&Brand {
            name: "Acme".into(),
        });
    }

    #[test]
    fn levels_count_only_for_listed_terpenes() {
        let myrcene = TagRef::Builtin("myrcene".into());
        let limonene = TagRef::Builtin("limonene".into());
        let mut s = Strain::new("x".into());
        s.terpenes.insert(myrcene.clone());
        // Left by a merge after another device removed limonene.
        s.terpene_levels
            .insert(limonene.clone(), TerpeneLevel::Dominant);
        assert_eq!(s.level(&limonene), None);
        // The only terpene listed is the dominant one, rated or not.
        assert_eq!(s.dominant_terpene(), Some(&myrcene));
        s.terpenes.insert(limonene.clone());
        assert_eq!(s.dominant_terpene(), Some(&limonene));
        s.terpene_levels.clear();
        // Two unrated terpenes: no dominant one.
        assert_eq!(s.dominant_terpene(), None);
    }

    #[test]
    fn a_strain_from_before_levels_reads_with_none() {
        // A strain saved by 0.3 or earlier: no levels, parents or
        // cannabinoids. Its terpenes stay, unrated.
        let mut doc = Document::new(RecordId::from_bytes([9; 16]), RecordKind::Strain);
        doc.set("name", Value::Text("Old".into()), stamp());
        doc.set("terpenes/myrcene", Value::Bool(true), stamp());
        let s = Strain::from_document(&doc).unwrap();
        assert_eq!(s.terpenes.len(), 1);
        assert!(s.terpene_levels.is_empty() && s.parents.is_empty() && s.cannabinoids.is_empty());
    }

    #[test]
    fn name_is_required() {
        let doc = Document::new(RecordId::from_bytes([1; 16]), RecordKind::Brand);
        assert_eq!(
            Brand::from_document(&doc).err(),
            Some(Error::InvalidField("name"))
        );
    }

    #[test]
    fn wrong_kind_is_rejected() {
        let doc = Document::create(&Brand { name: "x".into() }, stamp()).unwrap();
        assert_eq!(Strain::from_document(&doc).err(), Some(Error::WrongKind));
    }
}

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

/// A price the user paid.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Price {
    /// Amount in the currency's minor unit (cents).
    pub minor_units: i64,
    /// ISO 4217 currency code.
    pub currency: String,
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
    /// Where it was bought (a spot), if the user records it.
    pub bought_at: Option<RecordId>,
    /// What it cost, if the user records it.
    pub price: Option<Price>,
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
            bought_at: None,
            price: None,
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
            .opt("bought_at", self.bought_at, Value::Id)
            .opt("price", self.price.as_ref(), |p| {
                Value::Tuple(vec![
                    Value::Int(p.minor_units),
                    Value::Text(p.currency.clone()),
                ])
            });
        f.finish()
    }

    fn from_document(doc: &Document) -> Result<Self> {
        let r = Reader::new::<Self>(doc)?;
        let price = match r.value("price") {
            Value::Null => None,
            Value::Tuple(items) => match items.as_slice() {
                [Value::Int(minor_units), Value::Text(currency)] => Some(Price {
                    minor_units: *minor_units,
                    currency: currency.clone(),
                }),
                _ => return Err(Error::InvalidField("price")),
            },
            _ => return Err(Error::InvalidField("price")),
        };
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
            bought_at: r.id("bought_at")?,
            price,
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
                "bought_at",
                "price",
            ],
            &["strains", "terpenes"],
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
    /// Dominant terpenes.
    pub terpenes: BTreeSet<TagRef>,
    /// Indica/sativa/hybrid label.
    pub strain_type: Option<StrainType>,
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
            });
        f.finish()
    }

    fn from_document(doc: &Document) -> Result<Self> {
        let r = Reader::new::<Self>(doc)?;
        Ok(Self {
            name: r.required_text("name")?,
            lineage: r.text("lineage")?,
            terpenes: r.tag_set("terpenes")?,
            strain_type: r.slug("type", StrainType::from_slug)?,
        })
    }

    fn owns_field(key: &str) -> bool {
        owns(key, &["name", "lineage", "type"], &["terpenes"])
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
        p.bought_at = Some(RecordId::from_bytes([3; 16]));
        p.price = Some(Price {
            minor_units: 3_500,
            currency: "USD".into(),
        });
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
        });
        round_trip(&Brand {
            name: "Acme".into(),
        });
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

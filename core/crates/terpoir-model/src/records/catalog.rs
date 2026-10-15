// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

//! Products, strains and brands.

use std::collections::{BTreeMap, BTreeSet};
use terpoir_crypto::RecordId;

use super::{
    Fields, Method, PackageSize, Reader, Record, RecordKind, Tag, TagCategory, TagRef, owns,
    slug_enum,
};
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
    /// Built-in product subtypes (docs/design-decisions.md, "Product
    /// subtypes"): each under exactly one [`Form`], its parent. A product's
    /// subtype is one of these or a custom tag of
    /// [`TagCategory::Subtype`]. Slugs are unique across every built-in
    /// vocabulary.
    Subtype {
        /// A joint (pre-roll).
        Joint => "joint",
        /// A blunt (pre-roll).
        Blunt => "blunt",
        /// An infused pre-roll.
        InfusedPreRoll => "infused_pre_roll",
        /// Mini pre-rolls.
        MiniPreRolls => "mini_pre_rolls",
        /// Shatter (concentrate).
        Shatter => "shatter",
        /// Wax or badder (concentrate).
        Wax => "wax",
        /// Crumble (concentrate).
        Crumble => "crumble",
        /// Live resin (concentrate).
        LiveResin => "live_resin",
        /// Rosin (concentrate).
        Rosin => "rosin",
        /// Bubble hash (concentrate).
        BubbleHash => "bubble_hash",
        /// Dry sift or kief (concentrate).
        DrySift => "dry_sift",
        /// Pressed hash (concentrate).
        PressedHash => "pressed_hash",
        /// Diamonds (concentrate).
        Diamonds => "diamonds",
        /// Sauce (concentrate).
        Sauce => "sauce",
        /// Distillate (concentrate).
        Distillate => "distillate",
        /// RSO, Rick Simpson oil (concentrate).
        Rso => "rso",
        /// A cartridge (vape).
        Cartridge => "cartridge",
        /// A disposable (vape).
        Disposable => "disposable",
        /// An all-in-one, AIO (vape).
        AllInOne => "all_in_one",
        /// A pod (vape).
        Pod => "pod",
        /// A gummy (edible).
        Gummy => "gummy",
        /// Chocolate (edible).
        Chocolate => "chocolate",
        /// A baked good (edible).
        BakedGood => "baked_good",
        /// A mint (edible).
        Mint => "mint",
        /// A capsule (edible).
        Capsule => "capsule",
        /// Soda (beverage).
        Soda => "soda",
        /// Seltzer (beverage).
        Seltzer => "seltzer",
        /// Tea or coffee (beverage).
        TeaCoffee => "tea_coffee",
        /// A shot (beverage).
        Shot => "shot",
        /// A balm or lotion (topical).
        Balm => "balm",
        /// A transdermal patch (topical).
        Patch => "patch",
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
    /// Cannabinoids besides THC and CBD: a strain can be known for them, and
    /// a product's label can state their amounts.
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

impl Subtype {
    /// The form this subtype is under. `None` for a subtype from a newer
    /// app version.
    #[must_use]
    pub fn parent(&self) -> Option<Form> {
        Some(match self {
            Self::Joint | Self::Blunt | Self::InfusedPreRoll | Self::MiniPreRolls => Form::PreRoll,
            Self::Shatter
            | Self::Wax
            | Self::Crumble
            | Self::LiveResin
            | Self::Rosin
            | Self::BubbleHash
            | Self::DrySift
            | Self::PressedHash
            | Self::Diamonds
            | Self::Sauce
            | Self::Distillate
            | Self::Rso => Form::Concentrate,
            Self::Cartridge | Self::Disposable | Self::AllInOne | Self::Pod => Form::VapeCart,
            Self::Gummy | Self::Chocolate | Self::BakedGood | Self::Mint | Self::Capsule => {
                Form::Edible
            }
            Self::Soda | Self::Seltzer | Self::TeaCoffee | Self::Shot => Form::Beverage,
            Self::Balm | Self::Patch => Form::Topical,
            Self::Unknown(_) => return None,
        })
    }

    /// The subtype's own usual method, when it differs from its form's:
    /// only a patch has one (transdermal). `None` means the form's
    /// [`default_method`](Form::default_method).
    #[must_use]
    pub const fn suggested_method(&self) -> Option<Method> {
        match self {
            Self::Patch => Some(Method::Transdermal),
            _ => None,
        }
    }

    /// The built-in subtypes under `form`, in display order.
    pub fn of(form: &Form) -> impl Iterator<Item = Self> + '_ {
        Self::SLUGS
            .iter()
            .map(|slug| Self::from_slug(slug))
            .filter(move |s| s.parent().as_ref() == Some(form))
    }
}

/// The usual method for a product of `form` and `subtype`
/// (docs/design-decisions.md, "Product subtypes"): the subtype's own
/// method when its parent is the form (a built-in subtype's
/// [`suggested_method`](Subtype::suggested_method), or a custom subtype's
/// [`Tag::method`]), else the form's [`default_method`](Form::default_method).
/// A product with a subtype but no form takes the subtype's parent as its
/// form.
///
/// `custom` is the tag record of a custom `subtype`, if it could be read;
/// a tag of another category counts as no subtype.
#[must_use]
pub fn usual_method(
    form: Option<&Form>,
    subtype: Option<&TagRef>,
    custom: Option<&Tag>,
) -> Option<Method> {
    let (parent, own) = match subtype {
        Some(TagRef::Builtin(slug)) => {
            let s = Subtype::from_slug(slug);
            (s.parent(), s.suggested_method())
        }
        Some(TagRef::Custom(_)) => match custom {
            Some(tag) if tag.category == TagCategory::Subtype => {
                (tag.form.clone(), tag.method.clone())
            }
            _ => (None, None),
        },
        None => (None, None),
    };
    let form = form.or(parent.as_ref())?;
    match own {
        Some(method) if parent.as_ref() == Some(form) => Some(method),
        _ => form.default_method(),
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
    /// Subtype under the form: a built-in [`Subtype`] slug or a custom tag
    /// of [`TagCategory::Subtype`]. Since 0.6.0; a scalar field, so an
    /// older app keeps it untouched.
    pub subtype: Option<TagRef>,
    /// Brand.
    pub brand: Option<RecordId>,
    /// Strains it contains.
    pub strains: BTreeSet<RecordId>,
    /// THC from the label.
    pub thc: Option<LabelAmount>,
    /// CBD from the label.
    pub cbd: Option<LabelAmount>,
    /// Other cannabinoids from the label (CBN, CBG, THCa, CBDV), in the
    /// same units as THC and CBD. Since 0.5.0; stored per entry, so an older
    /// app keeps them untouched.
    pub cannabinoids: BTreeMap<Cannabinoid, LabelAmount>,
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
    /// The usual package size, from its label: pre-fills a new purchase
    /// of it (since 0.6.0; a scalar field, so an older app keeps it
    /// untouched).
    pub default_size: Option<PackageSize>,
    /// Learned receipt names (docs/design-decisions.md, "Learned receipt
    /// names"): the exact receipt line texts the user linked to this
    /// product, normalised by [`receipt_name_key`]. A receipt line with
    /// one of these texts links to it automatically. Since 0.6.0; stored
    /// per entry (`receipt_names/<text>`), so an older app keeps them
    /// untouched and two devices' names merge. Never indexed.
    pub receipt_names: BTreeSet<String>,
}

/// The longest learned receipt name kept, in characters.
pub const MAX_RECEIPT_NAME_CHARS: usize = 200;

/// A receipt line's text as a learned receipt name: upper case, with
/// single spaces and nothing else changed, so a line links on its own
/// only when it prints exactly the same words (`bd  3.5g hyb` and
/// `BD 3.5G HYB` are one name; `BD 3.5C HYB` is another). `None` for a
/// line with no text, or longer than [`MAX_RECEIPT_NAME_CHARS`].
#[must_use]
pub fn receipt_name_key(text: &str) -> Option<String> {
    let key = text
        .split_whitespace()
        .map(str::to_uppercase)
        .collect::<Vec<_>>()
        .join(" ");
    (!key.is_empty() && key.chars().count() <= MAX_RECEIPT_NAME_CHARS).then_some(key)
}

impl Product {
    /// A product with only a name.
    #[must_use]
    pub const fn new(name: String) -> Self {
        Self {
            name,
            form: None,
            subtype: None,
            brand: None,
            strains: BTreeSet::new(),
            thc: None,
            cbd: None,
            cannabinoids: BTreeMap::new(),
            terpenes: BTreeMap::new(),
            batch: None,
            package_date: None,
            identifiers: BTreeMap::new(),
            shops: BTreeSet::new(),
            default_size: None,
            receipt_names: BTreeSet::new(),
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
            .opt("subtype", self.subtype.as_ref(), |t| {
                Value::Text(t.to_key())
            })
            .opt("brand", self.brand, Value::Id)
            .id_set("strains", &self.strains)
            .opt("thc", self.thc.as_ref(), label_value)
            .opt("cbd", self.cbd.as_ref(), label_value)
            .map(
                "cannabinoid_amounts",
                &self.cannabinoids,
                |c| c.slug().to_owned(),
                label_value,
            )
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
            .id_set("shops", &self.shops)
            .opt(
                "default_size",
                self.default_size.as_ref(),
                PackageSize::to_value,
            )
            .text_set("receipt_names", &self.receipt_names);
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
            subtype: r
                .text("subtype")?
                .map(|key| TagRef::from_key(&key).ok_or(Error::InvalidField("subtype")))
                .transpose()?,
            brand: r.id("brand")?,
            strains: r.id_set("strains")?,
            thc: label_from(r.value("thc"), "thc")?,
            cbd: label_from(r.value("cbd"), "cbd")?,
            cannabinoids: r
                .entries("cannabinoid_amounts")
                .filter_map(|(slug, value)| {
                    label_from(value, "cannabinoid_amounts")
                        .map(|amount| amount.map(|a| (Cannabinoid::from_slug(slug), a)))
                        .transpose()
                })
                .collect::<Result<_>>()?,
            terpenes: r.tag_map("terpenes", |v| match v {
                Value::Int(i) => Some(*i),
                _ => None,
            })?,
            batch: r.text("batch")?,
            package_date: r.int_in("package_date", i64::MIN..=i64::MAX)?,
            identifiers,
            shops: r.id_set("shops")?,
            default_size: match r.value("default_size") {
                Value::Null => None,
                v => Some(PackageSize::from_value(v).ok_or(Error::InvalidField("default_size"))?),
            },
            receipt_names: r.text_set("receipt_names")?,
        })
    }

    fn owns_field(key: &str) -> bool {
        owns(
            key,
            &[
                "name",
                "form",
                "subtype",
                "brand",
                "thc",
                "cbd",
                "batch",
                "package_date",
                "default_size",
            ],
            &[
                "strains",
                "cannabinoid_amounts",
                "terpenes",
                "identifiers",
                "shops",
                "receipt_names",
            ],
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
        p.subtype = Some(TagRef::Custom(RecordId::from_bytes([4; 16])));
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
        let mut patch = Product::new("Patch".into());
        patch.form = Some(Form::Topical);
        patch.subtype = Some(TagRef::Builtin("patch".into()));
        round_trip(&patch);
        // A subtype from a newer app version round-trips as is.
        patch.subtype = Some(TagRef::Builtin("microdose_strip".into()));
        round_trip(&patch);
    }

    #[test]
    fn a_product_from_before_subtypes_reads_with_none_and_keeps_newer_fields() {
        // Saved by 0.5 or earlier: no subtype.
        let mut doc = Document::new(RecordId::from_bytes([9; 16]), RecordKind::Product);
        doc.set("name", Value::Text("Old".into()), stamp());
        doc.set("form", Value::Text("edible".into()), stamp());
        let p = Product::from_document(&doc).unwrap();
        assert_eq!(p.subtype, None);
        // An app without subtypes (here, one that doesn't own a field a
        // newer app wrote) leaves it as written when it saves the product.
        doc.set("future_field", Value::Text("x".into()), stamp());
        let mut edited = p;
        edited.name = "Renamed".into();
        doc.update(&edited, stamp()).unwrap();
        assert_eq!(doc.value("future_field"), &Value::Text("x".into()));
        // The subtype is a scalar field holding the tag key.
        let mut with = edited.clone();
        with.subtype = Some(TagRef::Builtin("gummy".into()));
        doc.update(&with, stamp()).unwrap();
        assert_eq!(doc.value("subtype"), &Value::Text("gummy".into()));
        assert!(Product::owns_field("subtype"));
        // A malformed one is refused.
        doc.set("subtype", Value::Text("custom:zz".into()), stamp());
        assert_eq!(
            Product::from_document(&doc).err(),
            Some(Error::InvalidField("subtype"))
        );
    }

    #[test]
    fn default_size_and_receipt_names_round_trip_and_sit_beside_older_fields() {
        use super::super::{SizeAmount, SizeUnit};
        let mut p = Product::new("Blue Dream".into());
        p.default_size = Some(PackageSize {
            amount: Some(SizeAmount {
                milli: 3_500,
                unit: SizeUnit::Gram,
            }),
            pack: None,
        });
        p.receipt_names.insert("BD 3.5G HYB".into());
        p.receipt_names.insert("BD 1/8 HYB".into());
        round_trip(&p);
        let doc = Document::create(&p, stamp()).unwrap();
        assert_eq!(doc.value("receipt_names/BD 1/8 HYB"), &Value::Bool(true));
        assert!(Product::owns_field("default_size"));
        assert!(Product::owns_field("receipt_names/BD 3.5G HYB"));
        // Saved before 0.6.0: none of either.
        let mut old = Document::new(RecordId::from_bytes([9; 16]), RecordKind::Product);
        old.set("name", Value::Text("Old".into()), stamp());
        let read = Product::from_document(&old).unwrap();
        assert_eq!(read.default_size, None);
        assert!(read.receipt_names.is_empty());
        // A malformed size is refused.
        old.set("default_size", Value::Int(1), stamp());
        assert_eq!(
            Product::from_document(&old).err(),
            Some(Error::InvalidField("default_size"))
        );
    }

    #[test]
    fn receipt_name_keys_fold_only_case_and_spaces() {
        assert_eq!(
            receipt_name_key("  bd   3.5g\tHyb ").as_deref(),
            Some("BD 3.5G HYB")
        );
        // Recognition confusions and punctuation are kept: exact only.
        assert_ne!(
            receipt_name_key("BD 3.5C HYB"),
            receipt_name_key("BD 3.5G HYB")
        );
        assert_ne!(receipt_name_key("BD-1G"), receipt_name_key("BD 1G"));
        assert_eq!(receipt_name_key(" \t "), None);
        assert_eq!(
            receipt_name_key(&"A".repeat(MAX_RECEIPT_NAME_CHARS + 1)),
            None
        );
        assert!(receipt_name_key(&"A".repeat(MAX_RECEIPT_NAME_CHARS)).is_some());
    }

    #[test]
    fn every_subtype_has_one_parent_form() {
        for slug in Subtype::SLUGS {
            let s = Subtype::from_slug(slug);
            assert!(!matches!(s, Subtype::Unknown(_)), "{slug}");
            assert!(
                matches!(s.parent(), Some(f) if !matches!(f, Form::Unknown(_))),
                "{slug}"
            );
        }
        // Every subtype is listed under exactly its parent.
        let listed: usize = Form::SLUGS
            .iter()
            .map(|f| Subtype::of(&Form::from_slug(f)).count())
            .sum();
        assert_eq!(listed, Subtype::SLUGS.len());
        assert_eq!(
            Subtype::of(&Form::Topical).collect::<Vec<_>>(),
            [Subtype::Balm, Subtype::Patch]
        );
        assert_eq!(Subtype::of(&Form::Flower).count(), 0);
        assert_eq!(Subtype::from_slug("strip").parent(), None);
        // Only the patch has a method of its own.
        for slug in Subtype::SLUGS {
            let s = Subtype::from_slug(slug);
            let expected = (s == Subtype::Patch).then_some(Method::Transdermal);
            assert_eq!(s.suggested_method(), expected, "{slug}");
        }
    }

    #[test]
    fn usual_method_comes_from_the_subtype_then_the_form() {
        let builtin = |s: &str| TagRef::Builtin(s.into());
        let topical = Some(&Form::Topical);
        // A patch is transdermal; a balm, like any topical, topical.
        assert_eq!(
            usual_method(topical, Some(&builtin("patch")), None),
            Some(Method::Transdermal)
        );
        assert_eq!(
            usual_method(topical, Some(&builtin("balm")), None),
            Some(Method::Topical)
        );
        assert_eq!(usual_method(topical, None, None), Some(Method::Topical));
        // A subtype under another form: the form decides.
        assert_eq!(
            usual_method(Some(&Form::Edible), Some(&builtin("patch")), None),
            Some(Method::Edible)
        );
        // No form: the subtype's parent stands in.
        assert_eq!(
            usual_method(None, Some(&builtin("patch")), None),
            Some(Method::Transdermal)
        );
        assert_eq!(
            usual_method(None, Some(&builtin("rosin")), None),
            Some(Method::Dabbed)
        );
        assert_eq!(usual_method(None, None, None), None);
        // A subtype from a newer app version: the form's.
        assert_eq!(
            usual_method(topical, Some(&builtin("future")), None),
            Some(Method::Topical)
        );
        // A custom subtype with its own method, under its parent only.
        let custom = TagRef::Custom(RecordId::from_bytes([3; 16]));
        let strip = Tag {
            name: "Strip".into(),
            category: TagCategory::Subtype,
            form: Some(Form::Edible),
            method: Some(Method::Tincture),
        };
        assert_eq!(
            usual_method(Some(&Form::Edible), Some(&custom), Some(&strip)),
            Some(Method::Tincture)
        );
        assert_eq!(
            usual_method(Some(&Form::Beverage), Some(&custom), Some(&strip)),
            Some(Method::Beverage)
        );
        // One that keeps its form's method, or is missing, or isn't a
        // subtype tag.
        let kept = Tag {
            method: None,
            ..strip.clone()
        };
        assert_eq!(
            usual_method(Some(&Form::Edible), Some(&custom), Some(&kept)),
            Some(Method::Edible)
        );
        assert_eq!(
            usual_method(Some(&Form::Edible), Some(&custom), None),
            Some(Method::Edible)
        );
        let effect = Tag {
            category: TagCategory::Effect,
            ..strip
        };
        assert_eq!(
            usual_method(Some(&Form::Edible), Some(&custom), Some(&effect)),
            Some(Method::Edible)
        );
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

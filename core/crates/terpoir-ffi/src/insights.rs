// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

//! Insights over the journal (see `terpoir-insights`): each session's
//! facts, the comparisons that clear the evidence thresholds, and the
//! user's feedback on them (dismissed, marked wrong, category muted), kept
//! in the encrypted store.

use std::collections::{BTreeMap, BTreeSet};

use terpoir_insights::{
    Buy, Level, SessionFacts, ShopSessionFacts, insights, shop_insights, time_of_day,
};
use terpoir_model::{
    DoseUnit, Product, ProductLine, Purchase, RecordId, Session, Spot, Strain, TagRef,
    TrackedField, TrackingPreferences,
};

use crate::error::Result;

/// A difference in the user's ratings worth showing, with its evidence. The
/// app words it ("your sessions suggest…"); it is never advice.
#[derive(Clone, PartialEq, Eq, uniffi::Record)]
pub struct InsightData {
    /// Stable key for feedback, such as `terpene:myrcene`.
    pub key: String,
    /// `terpene`, `method`, `form`, `strain_type` or `time`.
    pub category: String,
    /// What was compared: a built-in slug (`myrcene`, `vape_cart`,
    /// `evening`), or a custom tag's name when [`Self::custom`].
    pub subject: String,
    /// Whether [`Self::subject`] is a custom tag's name.
    pub custom: bool,
    /// Average rating with it, in thousandths of a star.
    pub with_avg_milli: i64,
    /// Average rating without it.
    pub without_avg_milli: i64,
    /// The sessions with it, for tap-through.
    pub with_sessions: Vec<String>,
    /// How many rated sessions are without it.
    pub without_count: u32,
    /// Likely confounders: `method`, `time` or `dose`.
    pub confounders: Vec<String>,
}

/// What the user said about an insight.
#[derive(Clone, Copy, PartialEq, Eq, Debug, uniffi::Enum)]
pub enum InsightFeedback {
    /// Hide this insight.
    Dismissed,
    /// Hide it, and note that the user thinks it is wrong.
    Wrong,
    /// Hide every insight of this category (`key` is the category).
    Muted,
}

impl InsightFeedback {
    pub(crate) const fn slug(self) -> &'static str {
        match self {
            Self::Dismissed => "dismissed",
            Self::Wrong => "wrong",
            Self::Muted => "muted",
        }
    }
}

/// What the insights read from the journal.
pub(crate) trait Source {
    fn sessions(&self) -> Result<Vec<(RecordId, Session)>>;
    fn product(&self, id: &RecordId) -> Result<Option<Product>>;
    fn strain(&self, id: &RecordId) -> Result<Option<Strain>>;
    fn tag_name(&self, id: &RecordId) -> Result<Option<String>>;
    /// Feedback state by insight key or category.
    fn feedback(&self) -> Result<BTreeMap<String, String>>;
    fn purchases(&self) -> Result<Vec<(RecordId, Purchase)>>;
    fn spot(&self, id: &RecordId) -> Result<Option<Spot>>;
    fn brand_name(&self, id: &RecordId) -> Result<Option<String>>;
    /// "What I track": insights skip what is built on a hidden data point.
    fn tracking(&self) -> Result<TrackingPreferences>;
}

/// A shop's numbers (see `terpoir_insights::shop_insights`), with their
/// evidence. Only shops with enough rated sessions.
#[derive(Clone, PartialEq, Eq, uniffi::Record)]
pub struct ShopInsightData {
    /// The Re-up Spot's ID.
    pub spot: String,
    /// Its name.
    pub name: String,
    /// Sessions with a product bought here, for tap-through.
    pub sessions: Vec<String>,
    /// Average rating of the rated ones, in thousandths of a star.
    pub avg_rating_milli: Option<i64>,
    /// Hits (4★ or more, or "would repeat") among judged sessions, in percent.
    pub hit_rate_percent: Option<u32>,
    /// Average age of products at purchase here, in days, when enough
    /// package dates are known.
    pub avg_age_days: Option<i64>,
    /// Brands, products and strains rated differently here than at other
    /// shops: brands first, then products, then strains.
    pub effects: Vec<ShopEffectData>,
}

/// A brand, product or strain rated differently depending on the shop it
/// came from.
#[derive(Clone, PartialEq, Eq, uniffi::Record)]
pub struct ShopEffectData {
    /// `brand`, `product` or `strain`.
    pub level: String,
    /// The brand's, product's or strain's ID.
    pub subject: String,
    /// Its display label (for a product: brand, then name).
    pub label: String,
    /// Average rating when bought here, in thousandths of a star.
    pub here_avg_milli: i64,
    /// Average rating when bought elsewhere.
    pub elsewhere_avg_milli: i64,
    /// Rated sessions with it bought here.
    pub here_sessions: u32,
    /// Rated sessions with it bought elsewhere.
    pub elsewhere_sessions: u32,
}

/// Where a session line's product came from: the shop the user picked on
/// the line, else the shop of the purchase it names, else the product's
/// only known shop ("Where I buy it" and purchases). Otherwise unknown,
/// and the line is left out. The purchase day is known only from a named
/// purchase.
fn line_shop(
    line: &ProductLine,
    product: &Product,
    purchases: &BTreeMap<RecordId, Purchase>,
    bought_at: Option<&BTreeSet<RecordId>>,
) -> Option<(RecordId, Option<i64>)> {
    let named = line
        .purchase
        .as_ref()
        .and_then(|r| purchases.get(&r.purchase))
        .and_then(|p| p.spot.map(|spot| (spot, local_day(p))));
    if let Some(shop) = line.shop {
        let day = named.filter(|(s, _)| *s == shop).map(|(_, d)| d);
        return Some((shop, day));
    }
    if let Some((shop, day)) = named {
        return Some((shop, Some(day)));
    }
    let mut known: BTreeSet<RecordId> = product.shops.clone();
    known.extend(bought_at.into_iter().flatten());
    match known.len() {
        1 => known.pop_first().map(|s| (s, None)),
        _ => None,
    }
}

/// Shop insights over the journal; see [`line_shop`] for how a line gets
/// its shop.
pub(crate) fn shops(source: &dyn Source) -> Result<Vec<ShopInsightData>> {
    // Shop insights are built on prices and shops; brands, strains and
    // package dates within them only while those are tracked too.
    let tracking = source.tracking()?;
    if !tracking.tracks(&TrackedField::PriceAndShop) {
        return Ok(Vec::new());
    }
    let show_brand = tracking.tracks(&TrackedField::Brand);
    let show_strain = tracking.tracks(&TrackedField::Strain);
    let show_age = tracking.tracks(&TrackedField::Batch);
    let sessions = source.sessions()?;
    let purchases: BTreeMap<RecordId, Purchase> = source.purchases()?.into_iter().collect();
    // The shops each product appears on receipts from.
    let mut bought_at: BTreeMap<RecordId, BTreeSet<RecordId>> = BTreeMap::new();
    for purchase in purchases.values() {
        let Some(shop) = purchase.spot else {
            continue;
        };
        for product in purchase.lines.values().filter_map(|l| l.product) {
            bought_at.entry(product).or_default().insert(shop);
        }
    }
    let mut products: BTreeMap<RecordId, Option<Product>> = BTreeMap::new();
    let mut facts = Vec::with_capacity(sessions.len());
    for (index, (_, session)) in sessions.iter().enumerate() {
        let mut buys = Vec::new();
        for line in session.lines.values() {
            let Some(product_id) = line.product else {
                continue;
            };
            if let std::collections::btree_map::Entry::Vacant(slot) = products.entry(product_id) {
                slot.insert(source.product(&product_id)?);
            }
            let Some(Some(product)) = products.get(&product_id) else {
                continue;
            };
            let Some((shop, day)) =
                line_shop(line, product, &purchases, bought_at.get(&product_id))
            else {
                continue;
            };
            buys.push(Buy {
                shop: shop.to_hex(),
                product: product_id.to_hex(),
                brand: product.brand.filter(|_| show_brand).map(|b| b.to_hex()),
                strains: if show_strain {
                    product.strains.iter().map(RecordId::to_hex).collect()
                } else {
                    Vec::new()
                },
                age_days: day
                    .zip(product.package_date)
                    .filter(|_| show_age)
                    .map(|(d, p)| d - p)
                    .filter(|age| *age >= 0),
            });
        }
        facts.push(ShopSessionFacts {
            index,
            rating: session.rating,
            would_repeat: session.would_repeat,
            buys,
        });
    }

    let mut out = Vec::new();
    for summary in shop_insights(&facts) {
        let Ok(spot_id) = RecordId::from_hex(&summary.shop) else {
            continue;
        };
        let name = source.spot(&spot_id)?.map(|s| s.name).unwrap_or_default();
        let mut effects = Vec::new();
        for effect in &summary.effects {
            let Ok(id) = RecordId::from_hex(&effect.subject) else {
                continue;
            };
            let (level, label) = match effect.level {
                Level::Brand => ("brand", source.brand_name(&id)?),
                Level::Strain => ("strain", source.strain(&id)?.map(|s| s.name)),
                Level::Product => (
                    "product",
                    match products.get(&id).cloned().flatten() {
                        Some(p) => Some(product_label(source, &p, show_brand)?),
                        None => None,
                    },
                ),
            };
            let Some(label) = label else {
                continue;
            };
            effects.push(ShopEffectData {
                level: level.to_owned(),
                subject: effect.subject.clone(),
                label,
                here_avg_milli: effect.here_avg_milli,
                elsewhere_avg_milli: effect.elsewhere_avg_milli,
                here_sessions: u32::try_from(effect.here_sessions).unwrap_or(u32::MAX),
                elsewhere_sessions: u32::try_from(effect.elsewhere_sessions).unwrap_or(u32::MAX),
            });
        }
        out.push(ShopInsightData {
            spot: summary.shop.clone(),
            name,
            sessions: summary
                .sessions
                .iter()
                .filter_map(|i| sessions.get(*i).map(|(id, _)| id.to_hex()))
                .collect(),
            avg_rating_milli: summary.avg_rating_milli,
            hit_rate_percent: summary.hit_rate_percent,
            avg_age_days: summary.avg_age_days,
            effects,
        });
    }
    Ok(out)
}

/// A product's display label: brand, then name (unless the name already
/// starts with the brand, or brands are hidden).
fn product_label(source: &dyn Source, p: &Product, show_brand: bool) -> Result<String> {
    let brand = match p.brand.filter(|_| show_brand) {
        Some(b) => source.brand_name(&b)?,
        None => None,
    };
    Ok(match brand {
        Some(b) if !p.name.to_lowercase().starts_with(&b.to_lowercase()) => {
            format!("{b} {}", p.name)
        }
        _ => p.name.clone(),
    })
}

/// The purchase's local calendar day, in days since 1970-01-01.
fn local_day(purchase: &Purchase) -> i64 {
    (purchase.when.unix_ms.div_euclid(60_000) + i64::from(purchase.when.utc_offset_min))
        .div_euclid(1440)
}

/// The insights to show: computed, then filtered by the user's feedback.
pub(crate) fn current(source: &dyn Source) -> Result<Vec<InsightData>> {
    let tracking = source.tracking()?;
    let show_terpenes = tracking.tracks(&TrackedField::Terpenes);
    let show_strain = tracking.tracks(&TrackedField::Strain);
    let show_dose = tracking.tracks(&TrackedField::Dose);
    let sessions = source.sessions()?;
    let mut products: BTreeMap<RecordId, Option<Product>> = BTreeMap::new();
    let mut strains: BTreeMap<RecordId, Option<Strain>> = BTreeMap::new();
    let mut facts = Vec::with_capacity(sessions.len());
    for (index, (_, session)) in sessions.iter().enumerate() {
        let mut features = BTreeSet::new();
        let mut doses = Vec::new();
        let mut all_mg = !session.lines.is_empty();
        for line in session.lines.values() {
            features.insert(format!("method:{}", line.method.slug()));
            match &line.dose {
                Some(d) if d.unit == DoseUnit::Milligrams => doses.push(d.amount_milli),
                _ => all_mg = false,
            }
            let Some(product_id) = line.product else {
                continue;
            };
            if let std::collections::btree_map::Entry::Vacant(slot) = products.entry(product_id) {
                slot.insert(source.product(&product_id)?);
            }
            let Some(Some(product)) = products.get(&product_id) else {
                continue;
            };
            if let Some(form) = &product.form {
                features.insert(format!("form:{}", form.slug()));
            }
            // The product's dominant terpene (ties: the first in key order).
            if show_terpenes
                && let Some((tag, _)) = product.terpenes.iter().fold(
                    None::<(&TagRef, i64)>,
                    |best, (t, v)| match best {
                        Some((_, b)) if b >= *v => best,
                        _ => Some((t, *v)),
                    },
                )
            {
                features.insert(format!("terpene:{}", tag_key(tag)));
            }
            for strain_id in product.strains.iter().filter(|_| show_strain) {
                if let std::collections::btree_map::Entry::Vacant(slot) = strains.entry(*strain_id)
                {
                    slot.insert(source.strain(strain_id)?);
                }
                if let Some(Some(strain)) = strains.get(strain_id)
                    && let Some(kind) = &strain.strain_type
                {
                    features.insert(format!("strain_type:{}", kind.slug()));
                }
            }
        }
        let local_min =
            session.when.unix_ms.div_euclid(60_000) + i64::from(session.when.utc_offset_min);
        let hour = u32::try_from(local_min.rem_euclid(1440) / 60).unwrap_or(0);
        features.insert(time_of_day(hour).to_owned());
        facts.push(SessionFacts {
            index,
            rating: session.rating,
            features,
            dose_mg_milli: (show_dose && all_mg && !doses.is_empty()).then(|| doses.iter().sum()),
        });
    }

    let feedback = source.feedback()?;
    let mut out = Vec::new();
    for insight in insights(&facts) {
        let category = insight.category().to_owned();
        if feedback.contains_key(&insight.feature)
            || feedback.get(&category).is_some_and(|s| s == "muted")
        {
            continue;
        }
        let value = insight.feature.split_once(':').map_or("", |(_, v)| v);
        let (subject, custom) = match value.strip_prefix("custom:") {
            Some(hex) => match RecordId::from_hex(hex).ok() {
                Some(id) => (source.tag_name(&id)?.unwrap_or_default(), true),
                None => continue,
            },
            None => (value.to_owned(), false),
        };
        out.push(InsightData {
            key: insight.feature.clone(),
            category,
            subject,
            custom,
            with_avg_milli: insight.with_avg_milli,
            without_avg_milli: insight.without_avg_milli,
            with_sessions: insight
                .with_sessions
                .iter()
                .filter_map(|i| sessions.get(*i).map(|(id, _)| id.to_hex()))
                .collect(),
            without_count: u32::try_from(insight.without_count).unwrap_or(u32::MAX),
            confounders: insight.confounders.clone(),
        });
    }
    Ok(out)
}

fn tag_key(tag: &TagRef) -> String {
    match tag {
        TagRef::Builtin(slug) => slug.clone(),
        TagRef::Custom(id) => format!("custom:{}", id.to_hex()),
    }
}

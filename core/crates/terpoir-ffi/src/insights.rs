// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

//! Insights over the journal (see `terpoir-insights`): each session's
//! facts, the comparisons that clear the evidence thresholds, and the
//! user's feedback on them (dismissed, marked wrong, category muted), kept
//! in the encrypted store.

use std::collections::{BTreeMap, BTreeSet};

use terpoir_insights::{
    Buy, Level, SessionFacts, ShopSessionFacts, TimelineFacts, insights, shop_insights,
    time_of_day, timeline_patterns,
};
use terpoir_model::{
    DoseUnit, Product, ProductLine, Purchase, RecordId, Session, Spot, Strain, StrainDetail,
    TagRef, TrackedField, TrackingPreferences,
};

use crate::error::Result;

/// A difference in the user's ratings worth showing, with its evidence. The
/// app words it ("your sessions suggest…"); it is never advice.
#[derive(Clone, PartialEq, Eq, uniffi::Record)]
pub struct InsightData {
    /// Stable key for feedback, such as `terpene:myrcene`.
    pub key: String,
    /// `terpene`, `method`, `form`, `strain_type`, `lineage` or `time`.
    pub category: String,
    /// What was compared: a built-in slug (`myrcene`, `vape_cart`,
    /// `evening`), a custom tag's name when [`Self::custom`], or for
    /// `lineage` the shared parent strain's name.
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
    /// For a terpene insight: how many of [`Self::with_sessions`] have the
    /// terpene only from a strain's profile, because their product's label
    /// lists none (docs/design-decisions.md, "Richer strains"). The app
    /// says so; it is weaker evidence than a label.
    pub from_strain_profiles: u32,
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
    /// Every strain, to find which are parents of others.
    fn strains(&self) -> Result<Vec<(RecordId, Strain)>>;
    fn tag_name(&self, id: &RecordId) -> Result<Option<String>>;
    /// Feedback state by insight key or category.
    fn feedback(&self) -> Result<BTreeMap<String, String>>;
    fn purchases(&self) -> Result<Vec<(RecordId, Purchase)>>;
    fn spot(&self, id: &RecordId) -> Result<Option<Spot>>;
    fn brand_name(&self, id: &RecordId) -> Result<Option<String>>;
    /// "What I track": insights skip what is built on a hidden data point.
    fn tracking(&self) -> Result<TrackingPreferences>;
}

/// "How it usually goes" for a method, product or strain: its typical
/// timeline from the points the user set (see
/// `terpoir_insights::timeline_patterns`). Opt-in with the other insights,
/// and empty while the timeline is hidden in "What I track".
#[derive(Clone, PartialEq, Eq, uniffi::Record)]
pub struct TimelineInsightData {
    /// `method`, `product` or `strain`.
    pub category: String,
    /// A method's slug, or the product's or strain's ID.
    pub subject: String,
    /// The product's or strain's display name; a method's slug.
    pub name: String,
    /// Sessions with set points, for tap-through.
    pub sessions: Vec<String>,
    /// Median minutes until it reached Mild or stronger, when at least 3
    /// sessions show it.
    pub onset_min: Option<u32>,
    /// Median minutes until the strongest point.
    pub peak_min: u32,
    /// Median strongest level, 0 (None) to 5 (Intense).
    pub peak_level: u8,
    /// Median minutes until it dropped below Mild after the peak, when at
    /// least 3 sessions show it.
    pub eases_min: Option<u32>,
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
    /// Brands, products, strains, strain families and dominant terpenes
    /// rated differently here than at other shops, in that order.
    pub effects: Vec<ShopEffectData>,
}

/// A brand, product, strain, strain family or dominant terpene rated
/// differently depending on the shop it came from.
#[derive(Clone, PartialEq, Eq, uniffi::Record)]
pub struct ShopEffectData {
    /// `brand`, `product`, `strain`, `lineage` (a parent strain and its
    /// offspring) or `terpene` (products with it as the dominant terpene).
    pub level: String,
    /// The brand's, product's or strain's ID (for `lineage`, the parent
    /// strain's); for `terpene`, its key: a built-in slug, or `custom:` and
    /// a tag ID.
    pub subject: String,
    /// Its display label (for a product: brand, then name; for a built-in
    /// terpene, its slug; for a custom one, the tag's name).
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
    let show_terpenes = tracking.tracks(&TrackedField::Terpenes);
    // Families need parents, shown only at the Detailed step; a strain's
    // terpene stands in for a label's above Simple, as in the insights.
    let show_lineage = show_strain && tracking.strain_detail() == StrainDetail::Detailed;
    let strain_terpenes =
        show_terpenes && show_strain && tracking.strain_detail() != StrainDetail::Simple;
    let parents = if show_lineage {
        parents_of_others(&source.strains()?)
    } else {
        BTreeSet::new()
    };
    let mut strains: BTreeMap<RecordId, Option<Strain>> = BTreeMap::new();
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
            let mut lineages = BTreeSet::new();
            let mut strain_terpene = None;
            for strain_id in product.strains.iter().filter(|_| show_strain) {
                if let std::collections::btree_map::Entry::Vacant(slot) = strains.entry(*strain_id)
                {
                    slot.insert(source.strain(strain_id)?);
                }
                if let Some(Some(strain)) = strains.get(strain_id) {
                    if show_lineage {
                        lineages.extend(families(strain_id, strain, &parents).map(|f| f.to_hex()));
                    }
                    if strain_terpene.is_none() {
                        strain_terpene = strain.dominant_terpene().map(tag_key);
                    }
                }
            }
            let terpene = label_terpene(product)
                .map(tag_key)
                .or(strain_terpene.filter(|_| strain_terpenes))
                .filter(|_| show_terpenes);
            buys.push(Buy {
                shop: shop.to_hex(),
                product: product_id.to_hex(),
                brand: product.brand.filter(|_| show_brand).map(|b| b.to_hex()),
                strains: if show_strain {
                    product.strains.iter().map(RecordId::to_hex).collect()
                } else {
                    Vec::new()
                },
                lineages: lineages.into_iter().collect(),
                terpene,
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
            let id = RecordId::from_hex(&effect.subject).ok();
            let (level, label) = match (effect.level, id) {
                (Level::Terpene, _) => ("terpene", terpene_label(source, &effect.subject)?),
                (_, None) => continue,
                (Level::Brand, Some(id)) => ("brand", source.brand_name(&id)?),
                (Level::Strain, Some(id)) => ("strain", source.strain(&id)?.map(|s| s.name)),
                (Level::Lineage, Some(id)) => ("lineage", source.strain(&id)?.map(|s| s.name)),
                (Level::Product, Some(id)) => (
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
    // A strain's terpenes stand in for a label's only while the journal
    // shows them: terpenes and strains tracked, above the Simple step.
    let strain_terpenes =
        show_terpenes && show_strain && tracking.strain_detail() != StrainDetail::Simple;
    // Strain families (a parent and its offspring) need parents, which the
    // journal shows only at the Detailed step.
    let show_lineage = show_strain && tracking.strain_detail() == StrainDetail::Detailed;
    let parents = if show_lineage {
        parents_of_others(&source.strains()?)
    } else {
        BTreeSet::new()
    };
    // (session index, feature) pairs that come only from a strain profile.
    let mut from_profile: BTreeSet<(usize, String)> = BTreeSet::new();
    let sessions = source.sessions()?;
    let mut products: BTreeMap<RecordId, Option<Product>> = BTreeMap::new();
    let mut strains: BTreeMap<RecordId, Option<Strain>> = BTreeMap::new();
    let mut facts = Vec::with_capacity(sessions.len());
    for (index, (_, session)) in sessions.iter().enumerate() {
        let mut features = BTreeSet::new();
        let mut from_labels = BTreeSet::new();
        let mut from_strains = BTreeSet::new();
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
            let label_terpene = label_terpene(product);
            if show_terpenes && let Some(tag) = label_terpene {
                from_labels.insert(format!("terpene:{}", tag_key(tag)));
            }
            let mut strain_terpene = None;
            for strain_id in product.strains.iter().filter(|_| show_strain) {
                if let std::collections::btree_map::Entry::Vacant(slot) = strains.entry(*strain_id)
                {
                    slot.insert(source.strain(strain_id)?);
                }
                if let Some(Some(strain)) = strains.get(strain_id) {
                    if let Some(kind) = &strain.strain_type {
                        features.insert(format!("strain_type:{}", kind.slug()));
                    }
                    if show_lineage {
                        for family in families(strain_id, strain, &parents) {
                            features.insert(format!("lineage:{}", family.to_hex()));
                        }
                    }
                    // No terpenes on the label: the first of its strains
                    // (in ID order) with a dominant terpene stands in.
                    if strain_terpene.is_none() {
                        strain_terpene = strain.dominant_terpene().map(tag_key);
                    }
                }
            }
            if strain_terpenes
                && label_terpene.is_none()
                && let Some(key) = strain_terpene
            {
                from_strains.insert(format!("terpene:{key}"));
            }
        }
        for feature in from_strains.difference(&from_labels) {
            from_profile.insert((index, feature.clone()));
        }
        features.extend(from_labels);
        features.extend(from_strains);
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
        let lineage = (category == "lineage")
            .then(|| RecordId::from_hex(value).ok())
            .flatten();
        let (subject, custom) = match value.strip_prefix("custom:") {
            _ if category == "lineage" => match lineage {
                Some(id) => match source.strain(&id)? {
                    Some(strain) => (strain.name, false),
                    None => continue,
                },
                None => continue,
            },
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
            from_strain_profiles: u32::try_from(
                insight
                    .with_sessions
                    .iter()
                    .filter(|i| from_profile.contains(&(**i, insight.feature.clone())))
                    .count(),
            )
            .unwrap_or(u32::MAX),
        });
    }
    Ok(out)
}

/// A product's dominant terpene from its label (ties: the first in key
/// order).
fn label_terpene(product: &Product) -> Option<&TagRef> {
    product
        .terpenes
        .iter()
        .fold(None::<(&TagRef, i64)>, |best, (t, v)| match best {
            Some((_, b)) if b >= *v => best,
            _ => Some((t, *v)),
        })
        .map(|(tag, _)| tag)
}

/// The strains that are a parent of another strain in the journal.
fn parents_of_others(strains: &[(RecordId, Strain)]) -> BTreeSet<RecordId> {
    strains
        .iter()
        .flat_map(|(_, s)| s.parents.iter().copied())
        .collect()
}

/// The strain families a strain belongs to, each named by its shared
/// parent: one per parent, and its own when it is a parent of others.
fn families<'a>(
    id: &RecordId,
    strain: &'a Strain,
    parents: &BTreeSet<RecordId>,
) -> impl Iterator<Item = RecordId> + 'a {
    let own = parents.contains(id).then_some(*id);
    strain.parents.iter().copied().chain(own)
}

/// A terpene key's display label: a built-in slug as is, a custom tag's
/// name; `None` for a tag that is gone.
fn terpene_label(source: &dyn Source, key: &str) -> Result<Option<String>> {
    match key.strip_prefix("custom:") {
        Some(hex) => match RecordId::from_hex(hex) {
            Ok(id) => source.tag_name(&id),
            Err(_) => Ok(None),
        },
        None => Ok(Some(key.to_owned())),
    }
}

fn tag_key(tag: &TagRef) -> String {
    match tag {
        TagRef::Builtin(slug) => slug.clone(),
        TagRef::Custom(id) => format!("custom:{}", id.to_hex()),
    }
}

/// Typical timelines by method, product and strain (see
/// [`TimelineInsightData`]): methods first, then products and strains by
/// name.
pub(crate) fn timelines(source: &dyn Source) -> Result<Vec<TimelineInsightData>> {
    let tracking = source.tracking()?;
    if !tracking.tracks(&TrackedField::Timeline) {
        return Ok(Vec::new());
    }
    let show_strain = tracking.tracks(&TrackedField::Strain);
    let show_brand = tracking.tracks(&TrackedField::Brand);
    let sessions = source.sessions()?;
    let mut products: BTreeMap<RecordId, Option<Product>> = BTreeMap::new();
    let mut facts = Vec::new();
    for (index, (_, session)) in sessions.iter().enumerate() {
        if session.timeline_levels.is_empty() {
            continue;
        }
        let mut groups = BTreeSet::new();
        for line in session.lines.values() {
            groups.insert(format!("method:{}", line.method.slug()));
            let Some(product_id) = line.product else {
                continue;
            };
            groups.insert(format!("product:{}", product_id.to_hex()));
            if let std::collections::btree_map::Entry::Vacant(slot) = products.entry(product_id) {
                slot.insert(source.product(&product_id)?);
            }
            if show_strain && let Some(Some(product)) = products.get(&product_id) {
                for strain in &product.strains {
                    groups.insert(format!("strain:{}", strain.to_hex()));
                }
            }
        }
        facts.push(TimelineFacts {
            index,
            groups,
            levels: session.timeline_levels.clone(),
        });
    }
    let mut out = Vec::new();
    for pattern in timeline_patterns(&facts) {
        let (category, subject) = pattern.group.split_once(':').unwrap_or(("", ""));
        let name = match category {
            "method" => Some(subject.to_owned()),
            "product" => match RecordId::from_hex(subject).ok() {
                Some(id) => match products.get(&id).cloned().flatten() {
                    Some(p) => Some(product_label(source, &p, show_brand)?),
                    None => None,
                },
                None => None,
            },
            "strain" => match RecordId::from_hex(subject).ok() {
                Some(id) => source.strain(&id)?.map(|s| s.name),
                None => None,
            },
            _ => None,
        };
        // A deleted product or strain has nothing to show.
        let Some(name) = name else { continue };
        out.push(TimelineInsightData {
            category: category.to_owned(),
            subject: subject.to_owned(),
            name,
            sessions: pattern
                .sessions
                .iter()
                .map(|&i| sessions[i].0.to_hex())
                .collect(),
            onset_min: pattern.onset_min,
            peak_min: pattern.peak_min,
            peak_level: pattern.peak_level,
            eases_min: pattern.eases_min,
        });
    }
    let order = |c: &str| match c {
        "method" => 0,
        "product" => 1,
        _ => 2,
    };
    out.sort_by(|a, b| {
        order(&a.category)
            .cmp(&order(&b.category))
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    Ok(out)
}

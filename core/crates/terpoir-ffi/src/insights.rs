// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

//! Insights over the journal (see `terpoir-insights`): each session's
//! facts, the comparisons that clear the evidence thresholds, and the
//! user's feedback on them (dismissed, marked wrong, category muted), kept
//! in the encrypted store.

use std::collections::{BTreeMap, BTreeSet};

use terpoir_insights::{Buy, SessionFacts, ShopSessionFacts, insights, shop_insights, time_of_day};
use terpoir_model::{DoseUnit, Product, Purchase, RecordId, Session, Spot, Strain, TagRef};

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
    const fn slug(self) -> &'static str {
        match self {
            Self::Dismissed => "dismissed",
            Self::Wrong => "wrong",
            Self::Muted => "muted",
        }
    }
}

/// The store key of the feedback list (`key\tstate` lines).
pub(crate) const META_FEEDBACK: &str = "insight_feedback";

/// What the insights read from the journal.
pub(crate) trait Source {
    fn sessions(&self) -> Result<Vec<(RecordId, Session)>>;
    fn product(&self, id: &RecordId) -> Result<Option<Product>>;
    fn strain(&self, id: &RecordId) -> Result<Option<Strain>>;
    fn tag_name(&self, id: &RecordId) -> Result<Option<String>>;
    fn feedback(&self) -> Result<Vec<u8>>;
    fn purchases(&self) -> Result<Vec<(RecordId, Purchase)>>;
    fn spot(&self, id: &RecordId) -> Result<Option<Spot>>;
    fn brand_name(&self, id: &RecordId) -> Result<Option<String>>;
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
    /// Products rated differently here than at other shops.
    pub same_product: Vec<ShopEffectData>,
}

/// One product rated differently depending on the shop it came from.
#[derive(Clone, PartialEq, Eq, uniffi::Record)]
pub struct ShopEffectData {
    /// The product's ID.
    pub product: String,
    /// Its display label (brand, then name).
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

/// Shop insights over the journal. A session line counts for a shop when
/// it names the purchase it came from, or when its product was only ever
/// bought at one shop; otherwise its shop is unknown and it is left out.
pub(crate) fn shops(source: &dyn Source) -> Result<Vec<ShopInsightData>> {
    let sessions = source.sessions()?;
    let purchases: BTreeMap<RecordId, Purchase> = source.purchases()?.into_iter().collect();
    // Where each product was bought: (shop, purchase day) per purchase line.
    let mut bought: BTreeMap<RecordId, Vec<(RecordId, i64)>> = BTreeMap::new();
    for purchase in purchases.values() {
        let Some(shop) = purchase.spot else {
            continue;
        };
        for line in purchase.lines.values() {
            if let Some(product) = line.product {
                bought
                    .entry(product)
                    .or_default()
                    .push((shop, local_day(purchase)));
            }
        }
    }
    let mut package_dates: BTreeMap<RecordId, Option<i64>> = BTreeMap::new();
    let mut facts = Vec::with_capacity(sessions.len());
    for (index, (_, session)) in sessions.iter().enumerate() {
        let mut buys = Vec::new();
        for line in session.lines.values() {
            let Some(product) = line.product else {
                continue;
            };
            // The purchase this line names, or the product's only purchase
            // place (and, with one purchase, its day).
            let (shop, day) = match line
                .purchase
                .as_ref()
                .and_then(|r| purchases.get(&r.purchase))
            {
                Some(p) => match p.spot {
                    Some(shop) => (shop, Some(local_day(p))),
                    None => continue,
                },
                None => {
                    let places = bought.get(&product).map_or(&[][..], Vec::as_slice);
                    let shops: BTreeSet<RecordId> = places.iter().map(|(s, _)| *s).collect();
                    if shops.len() != 1 {
                        continue;
                    }
                    let day = (places.len() == 1).then(|| places[0].1);
                    (places[0].0, day)
                }
            };
            if let std::collections::btree_map::Entry::Vacant(slot) = package_dates.entry(product) {
                slot.insert(source.product(&product)?.and_then(|p| p.package_date));
            }
            let package = package_dates.get(&product).copied().flatten();
            buys.push(Buy {
                shop: shop.to_hex(),
                product: product.to_hex(),
                age_days: day.zip(package).map(|(d, p)| d - p).filter(|age| *age >= 0),
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
        let mut same_product = Vec::new();
        for effect in &summary.same_product {
            let Ok(product_id) = RecordId::from_hex(&effect.product) else {
                continue;
            };
            let label = match source.product(&product_id)? {
                Some(p) => {
                    let brand = match p.brand {
                        Some(b) => source.brand_name(&b)?,
                        None => None,
                    };
                    match brand {
                        Some(b) if !p.name.to_lowercase().starts_with(&b.to_lowercase()) => {
                            format!("{b} {}", p.name)
                        }
                        _ => p.name,
                    }
                }
                None => continue,
            };
            same_product.push(ShopEffectData {
                product: effect.product.clone(),
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
            same_product,
        });
    }
    Ok(out)
}

/// The purchase's local calendar day, in days since 1970-01-01.
fn local_day(purchase: &Purchase) -> i64 {
    (purchase.when.unix_ms.div_euclid(60_000) + i64::from(purchase.when.utc_offset_min))
        .div_euclid(1440)
}

/// The insights to show: computed, then filtered by the user's feedback.
pub(crate) fn current(source: &dyn Source) -> Result<Vec<InsightData>> {
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
            if let Some((tag, _)) =
                product
                    .terpenes
                    .iter()
                    .fold(None::<(&TagRef, i64)>, |best, (t, v)| match best {
                        Some((_, b)) if b >= *v => best,
                        _ => Some((t, *v)),
                    })
            {
                features.insert(format!("terpene:{}", tag_key(tag)));
            }
            for strain_id in &product.strains {
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
            dose_mg_milli: (all_mg && !doses.is_empty()).then(|| doses.iter().sum()),
        });
    }

    let feedback = parse_feedback(&source.feedback()?);
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

fn parse_feedback(bytes: &[u8]) -> BTreeMap<String, String> {
    String::from_utf8_lossy(bytes)
        .lines()
        .filter_map(|l| l.split_once('\t'))
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
        .collect()
}

/// The feedback list with `key` set to `feedback`.
pub(crate) fn with_feedback(current: &[u8], key: &str, feedback: InsightFeedback) -> Vec<u8> {
    let mut all = parse_feedback(current);
    // Keys never hold tabs or newlines; refuse to corrupt the list if one does.
    if key.contains(['\t', '\n']) {
        return current.to_vec();
    }
    all.insert(key.to_owned(), feedback.slug().to_owned());
    all.iter()
        .map(|(k, v)| format!("{k}\t{v}\n"))
        .collect::<String>()
        .into_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feedback_round_trips() {
        let list = with_feedback(b"", "terpene:myrcene", InsightFeedback::Dismissed);
        let list = with_feedback(&list, "time", InsightFeedback::Muted);
        let list = with_feedback(&list, "terpene:myrcene", InsightFeedback::Wrong);
        let parsed = parse_feedback(&list);
        assert_eq!(
            parsed.get("terpene:myrcene").map(String::as_str),
            Some("wrong")
        );
        assert_eq!(parsed.get("time").map(String::as_str), Some("muted"));
        assert_eq!(
            with_feedback(&list, "bad\tkey", InsightFeedback::Dismissed),
            list
        );
    }
}

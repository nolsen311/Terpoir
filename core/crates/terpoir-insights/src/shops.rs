// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

//! Shop insights (docs/design-decisions.md, "Shop insights"): sessions
//! grouped by the Re-up Spot each product came from.
//!
//! - Per shop: the average rating and the **hit rate** (the share of
//!   sessions rated 4★ or more, or marked "would repeat").
//! - **Shop effect**, kept apart from selection: the same brand, product,
//!   strain, strain family (a parent and its offspring) or dominant
//!   terpene rated differently depending on where it was bought. Only for
//!   ones bought at two or more shops. Brand is the main level: individual
//!   products often appear at one shop only, while a brand's are usually
//!   consistent across shops.
//! - **Freshness:** how old products were (from the label's package date)
//!   when bought at each shop, from purchase-dated lines only.
//!
//! The same evidence thresholds as every insight ([`MIN_SESSIONS`],
//! [`MIN_DIFFERENCE`]).

use std::collections::{BTreeMap, BTreeSet};

use crate::{MIN_DIFFERENCE, MIN_SESSIONS};

/// What a shop effect compares.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Level {
    /// The same brand.
    Brand,
    /// The same product.
    Product,
    /// The same strain.
    Strain,
    /// Strains sharing a parent: the parent and its offspring.
    Lineage,
    /// Products with the same dominant terpene.
    Terpene,
}

/// One product in a session, and where it was bought.
#[derive(Clone, PartialEq, Eq, Default)]
pub struct Buy {
    /// The shop (the caller's key for the spot).
    pub shop: String,
    /// The product (the caller's key).
    pub product: String,
    /// The product's brand, if known.
    pub brand: Option<String>,
    /// The product's strains.
    pub strains: Vec<String>,
    /// The strain families the product belongs to (the caller's key for
    /// each shared parent).
    pub lineages: Vec<String>,
    /// The product's dominant terpene (the caller's key), if known.
    pub terpene: Option<String>,
    /// Days from the package date to the purchase, if both are known.
    pub age_days: Option<i64>,
}

impl Buy {
    /// The keys this buy has at `level`.
    fn keys(&self, level: Level) -> Vec<&str> {
        match level {
            Level::Brand => self.brand.as_deref().into_iter().collect(),
            Level::Product => vec![self.product.as_str()],
            Level::Strain => self.strains.iter().map(String::as_str).collect(),
            Level::Lineage => self.lineages.iter().map(String::as_str).collect(),
            Level::Terpene => self.terpene.as_deref().into_iter().collect(),
        }
    }
}

/// What one session contributes to shop insights.
#[derive(Clone, Default)]
pub struct ShopSessionFacts {
    /// The caller's handle for the session.
    pub index: usize,
    /// Overall rating, 1 to 5.
    pub rating: Option<u8>,
    /// Would do it again.
    pub would_repeat: Option<bool>,
    /// The session's products whose shop is known.
    pub buys: Vec<Buy>,
}

/// The same brand, product or strain, rated differently depending on where
/// it was bought.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ShopEffect {
    /// What is compared.
    pub level: Level,
    /// The brand, product or strain (the caller's key).
    pub subject: String,
    /// Its average rating when bought here, in thousandths of a star.
    pub here_avg_milli: i64,
    /// Its average rating when bought elsewhere.
    pub elsewhere_avg_milli: i64,
    /// Rated sessions with it bought here.
    pub here_sessions: usize,
    /// Rated sessions with it bought elsewhere.
    pub elsewhere_sessions: usize,
}

/// One shop's numbers, with their evidence.
#[derive(Clone, PartialEq, Eq)]
pub struct ShopSummary {
    /// The shop.
    pub shop: String,
    /// Sessions with a product bought here (the caller's indices).
    pub sessions: Vec<usize>,
    /// Average rating of the rated ones, in thousandths of a star.
    pub avg_rating_milli: Option<i64>,
    /// Share of judged sessions (rated, or marked would-repeat or not)
    /// that were hits, in percent.
    pub hit_rate_percent: Option<u32>,
    /// Average age of products at purchase here, in days, when enough
    /// package dates are known.
    pub avg_age_days: Option<i64>,
    /// Brands, products, strains, strain families and dominant terpenes
    /// rated differently here than elsewhere, in that order.
    pub effects: Vec<ShopEffect>,
}

fn is_hit(s: &ShopSessionFacts) -> Option<bool> {
    match (s.rating, s.would_repeat) {
        (None, None) => None,
        (rating, repeat) => Some(rating.is_some_and(|r| r >= 4) || repeat == Some(true)),
    }
}

fn average(ratings: &[u8]) -> Option<i64> {
    let n = i64::try_from(ratings.len()).ok().filter(|n| *n > 0)?;
    Some(ratings.iter().map(|r| i64::from(*r) * 1000).sum::<i64>() / n)
}

/// The shop effects at `level` for `shop`. A session with the subject
/// bought both here and elsewhere says nothing about either, so it is left
/// out of both sides.
fn effects(sessions: &[ShopSessionFacts], shop: &str, level: Level) -> Vec<ShopEffect> {
    // Where each subject was bought, across all sessions.
    let mut places: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for s in sessions {
        for b in &s.buys {
            for key in b.keys(level) {
                places.entry(key).or_default().insert(b.shop.as_str());
            }
        }
    }
    let mut out = Vec::new();
    for (subject, at) in &places {
        if at.len() < 2 || !at.contains(shop) {
            continue;
        }
        let mut here = Vec::new();
        let mut elsewhere = Vec::new();
        for s in sessions {
            let Some(rating) = s.rating else { continue };
            let with = s.buys.iter().filter(|b| b.keys(level).contains(subject));
            let (mut in_here, mut in_else) = (false, false);
            for b in with {
                if b.shop == shop {
                    in_here = true;
                } else {
                    in_else = true;
                }
            }
            match (in_here, in_else) {
                (true, false) => here.push(rating),
                (false, true) => elsewhere.push(rating),
                _ => {}
            }
        }
        if here.len() < MIN_SESSIONS || elsewhere.len() < MIN_SESSIONS {
            continue;
        }
        let (Some(a), Some(b)) = (average(&here), average(&elsewhere)) else {
            continue;
        };
        if (a - b).abs() >= MIN_DIFFERENCE {
            out.push(ShopEffect {
                level,
                subject: (*subject).to_owned(),
                here_avg_milli: a,
                elsewhere_avg_milli: b,
                here_sessions: here.len(),
                elsewhere_sessions: elsewhere.len(),
            });
        }
    }
    out
}

/// Per-shop summaries for shops with enough rated sessions, most-used
/// first.
#[must_use]
pub fn shop_insights(sessions: &[ShopSessionFacts]) -> Vec<ShopSummary> {
    let shops: BTreeSet<&str> = sessions
        .iter()
        .flat_map(|s| s.buys.iter().map(|b| b.shop.as_str()))
        .collect();

    let mut out = Vec::new();
    for shop in shops {
        let here: Vec<&ShopSessionFacts> = sessions
            .iter()
            .filter(|s| s.buys.iter().any(|b| b.shop == shop))
            .collect();
        let ratings: Vec<u8> = here.iter().filter_map(|s| s.rating).collect();
        if ratings.len() < MIN_SESSIONS {
            continue;
        }
        let judged: Vec<bool> = here.iter().filter_map(|s| is_hit(s)).collect();
        let hits = judged.iter().filter(|h| **h).count();
        let hit_rate = u32::try_from(hits * 100 / judged.len().max(1))
            .ok()
            .filter(|_| !judged.is_empty());
        let ages: Vec<i64> = here
            .iter()
            .flat_map(|s| s.buys.iter())
            .filter(|b| b.shop == shop)
            .filter_map(|b| b.age_days)
            .collect();
        let avg_age = (ages.len() >= MIN_SESSIONS)
            .then(|| ages.iter().sum::<i64>() / i64::try_from(ages.len()).unwrap_or(1));

        out.push(ShopSummary {
            shop: shop.to_owned(),
            sessions: here.iter().map(|s| s.index).collect(),
            avg_rating_milli: average(&ratings),
            hit_rate_percent: hit_rate,
            avg_age_days: avg_age,
            effects: [
                Level::Brand,
                Level::Product,
                Level::Strain,
                Level::Lineage,
                Level::Terpene,
            ]
            .into_iter()
            .flat_map(|level| effects(sessions, shop, level))
            .collect(),
        });
    }
    out.sort_by_key(|s| std::cmp::Reverse(s.sessions.len()));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn buy(shop: &str, product: &str, age: Option<i64>) -> Buy {
        Buy {
            shop: shop.into(),
            product: product.into(),
            age_days: age,
            ..Buy::default()
        }
    }

    fn branded(shop: &str, product: &str, brand: &str, strain: &str) -> Buy {
        Buy {
            shop: shop.into(),
            product: product.into(),
            brand: Some(brand.into()),
            strains: vec![strain.into()],
            ..Buy::default()
        }
    }

    fn session(
        index: usize,
        rating: Option<u8>,
        repeat: Option<bool>,
        buys: Vec<Buy>,
    ) -> ShopSessionFacts {
        ShopSessionFacts {
            index,
            rating,
            would_repeat: repeat,
            buys,
        }
    }

    #[test]
    fn shops_need_enough_rated_sessions() {
        let mut all = Vec::new();
        for i in 0..4 {
            all.push(session(i, Some(4), None, vec![buy("a", "p", None)]));
        }
        assert!(shop_insights(&all).is_empty());
        all.push(session(4, Some(2), None, vec![buy("a", "p", None)]));
        let found = shop_insights(&all);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].avg_rating_milli, Some(3_600));
        assert_eq!(found[0].sessions, vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn hit_rate_counts_four_stars_or_would_repeat() {
        let all: Vec<_> = (0..5)
            .map(|i| {
                let (rating, repeat) = match i {
                    0 => (Some(5), None),
                    1 => (Some(2), Some(true)),
                    2 => (Some(3), Some(false)),
                    3 => (Some(2), None),
                    _ => (Some(4), Some(false)),
                };
                session(i, rating, repeat, vec![buy("a", "p", None)])
            })
            .collect();
        assert_eq!(shop_insights(&all)[0].hit_rate_percent, Some(60));
    }

    #[test]
    fn freshness_needs_enough_package_dates() {
        let mut all: Vec<_> = (0..5)
            .map(|i| {
                session(
                    i,
                    Some(3),
                    None,
                    vec![buy("a", "p", Some(30 * (i as i64 + 1)))],
                )
            })
            .collect();
        assert_eq!(shop_insights(&all)[0].avg_age_days, Some(90));
        all[0].buys[0].age_days = None;
        assert_eq!(shop_insights(&all)[0].avg_age_days, None);
    }

    #[test]
    fn the_same_product_rated_higher_at_one_shop_is_a_shop_effect() {
        let mut all = Vec::new();
        for i in 0..5 {
            all.push(session(
                i,
                Some(5),
                None,
                vec![buy("a", "blue dream", None)],
            ));
            all.push(session(
                10 + i,
                Some(3),
                None,
                vec![buy("b", "blue dream", None)],
            ));
            // A product only ever bought at one shop is selection, not a shop effect.
            all.push(session(
                20 + i,
                Some(1),
                None,
                vec![buy("a", "gummies", None)],
            ));
        }
        let found = shop_insights(&all);
        let a = found.iter().find(|s| s.shop == "a").unwrap();
        assert_eq!(a.effects.len(), 1);
        let effect = &a.effects[0];
        assert_eq!(
            (effect.level, effect.subject.as_str()),
            (Level::Product, "blue dream")
        );
        assert_eq!(
            (effect.here_avg_milli, effect.elsewhere_avg_milli),
            (5_000, 3_000)
        );
        assert_eq!((effect.here_sessions, effect.elsewhere_sessions), (5, 5));
        let b = found.iter().find(|s| s.shop == "b").unwrap();
        assert_eq!(b.effects[0].here_avg_milli, 3_000);
        // Most-used shop first.
        assert_eq!(found[0].shop, "a");
    }

    #[test]
    fn a_brand_rated_higher_at_one_shop_is_a_shop_effect_across_products() {
        // Each shop carries different products of the same brand.
        let mut all = Vec::new();
        for i in 0..5 {
            all.push(session(
                i,
                Some(5),
                None,
                vec![branded("a", &format!("a{i}"), "acme", "og")],
            ));
            all.push(session(
                10 + i,
                Some(2),
                None,
                vec![branded("b", &format!("b{i}"), "acme", "og")],
            ));
        }
        let found = shop_insights(&all);
        let a = found.iter().find(|s| s.shop == "a").unwrap();
        let levels: Vec<_> = a
            .effects
            .iter()
            .map(|e| (e.level, e.subject.as_str()))
            .collect();
        // Brand first; no product effect (no product at both shops).
        assert_eq!(levels, vec![(Level::Brand, "acme"), (Level::Strain, "og")]);
        assert_eq!(
            (
                a.effects[0].here_avg_milli,
                a.effects[0].elsewhere_avg_milli
            ),
            (5_000, 2_000)
        );
    }

    #[test]
    fn strain_families_and_terpenes_are_compared_across_strains() {
        // Different strains at each shop, sharing a parent and a terpene:
        // no strain effect, but a family and a terpene one.
        let bought = |shop: &str, i: usize| Buy {
            shop: shop.into(),
            product: format!("{shop}{i}"),
            strains: vec![format!("{shop}-strain")],
            lineages: vec!["blueberry".into()],
            terpene: Some("myrcene".into()),
            ..Buy::default()
        };
        let mut all = Vec::new();
        for i in 0..5 {
            all.push(session(i, Some(5), None, vec![bought("a", i)]));
            all.push(session(10 + i, Some(2), None, vec![bought("b", i)]));
        }
        let found = shop_insights(&all);
        let a = found.iter().find(|s| s.shop == "a").unwrap();
        let levels: Vec<_> = a
            .effects
            .iter()
            .map(|e| (e.level, e.subject.as_str()))
            .collect();
        assert_eq!(
            levels,
            vec![(Level::Lineage, "blueberry"), (Level::Terpene, "myrcene")]
        );
    }

    #[test]
    fn a_session_with_both_shops_counts_on_neither_side() {
        let mut all = Vec::new();
        for i in 0..5 {
            all.push(session(
                i,
                Some(5),
                None,
                vec![branded("a", "p", "acme", "og")],
            ));
            all.push(session(
                10 + i,
                Some(2),
                None,
                vec![branded("b", "p", "acme", "og")],
            ));
        }
        all.push(session(
            20,
            Some(1),
            None,
            vec![
                branded("a", "p", "acme", "og"),
                branded("b", "p", "acme", "og"),
            ],
        ));
        let found = shop_insights(&all);
        let a = found.iter().find(|s| s.shop == "a").unwrap();
        assert!(
            a.effects
                .iter()
                .all(|e| e.here_sessions == 5 && e.elsewhere_sessions == 5)
        );
    }
}

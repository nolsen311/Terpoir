// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

//! Shop insights (docs/design-decisions.md, "Shop insights"): sessions
//! grouped by the Re-up Spot each product was bought at.
//!
//! - Per shop: the average rating and the **hit rate** (the share of
//!   sessions rated 4★ or more, or marked "would repeat").
//! - **Shop effect**, kept apart from selection: the same product rated
//!   differently depending on where it was bought. Only for products bought
//!   at two or more shops.
//! - **Freshness:** how old products were (from the label's package date)
//!   when bought at each shop.
//!
//! The same evidence thresholds as every insight ([`MIN_SESSIONS`],
//! [`MIN_DIFFERENCE`]).

use std::collections::{BTreeMap, BTreeSet};

use crate::{MIN_DIFFERENCE, MIN_SESSIONS};

/// One product in a session, and where it was bought.
#[derive(Clone, PartialEq, Eq)]
pub struct Buy {
    /// The shop (the caller's key for the spot).
    pub shop: String,
    /// The product (the caller's key).
    pub product: String,
    /// Days from the package date to the purchase, if both are known.
    pub age_days: Option<i64>,
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

/// The same product, rated differently depending on where it was bought.
#[derive(Clone, PartialEq, Eq)]
pub struct ShopEffect {
    /// The product.
    pub product: String,
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
    /// Products rated differently here than elsewhere.
    pub same_product: Vec<ShopEffect>,
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

/// Per-shop summaries for shops with enough rated sessions, most-used
/// first.
#[must_use]
pub fn shop_insights(sessions: &[ShopSessionFacts]) -> Vec<ShopSummary> {
    let shops: BTreeSet<&str> = sessions
        .iter()
        .flat_map(|s| s.buys.iter().map(|b| b.shop.as_str()))
        .collect();
    // Where each product was bought, across all sessions.
    let mut product_shops: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for s in sessions {
        for b in &s.buys {
            product_shops
                .entry(b.product.as_str())
                .or_default()
                .insert(b.shop.as_str());
        }
    }

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

        let mut same_product = Vec::new();
        for (product, at) in &product_shops {
            if at.len() < 2 || !at.contains(shop) {
                continue;
            }
            let bought = |s: &&ShopSessionFacts, here: bool| {
                s.buys
                    .iter()
                    .any(|b| b.product == *product && (b.shop == shop) == here)
            };
            let here_ratings: Vec<u8> = sessions
                .iter()
                .filter(|s| bought(s, true))
                .filter_map(|s| s.rating)
                .collect();
            let else_ratings: Vec<u8> = sessions
                .iter()
                .filter(|s| bought(s, false))
                .filter_map(|s| s.rating)
                .collect();
            if here_ratings.len() < MIN_SESSIONS || else_ratings.len() < MIN_SESSIONS {
                continue;
            }
            let (Some(a), Some(b)) = (average(&here_ratings), average(&else_ratings)) else {
                continue;
            };
            if (a - b).abs() >= MIN_DIFFERENCE {
                same_product.push(ShopEffect {
                    product: (*product).to_owned(),
                    here_avg_milli: a,
                    elsewhere_avg_milli: b,
                    here_sessions: here_ratings.len(),
                    elsewhere_sessions: else_ratings.len(),
                });
            }
        }

        out.push(ShopSummary {
            shop: shop.to_owned(),
            sessions: here.iter().map(|s| s.index).collect(),
            avg_rating_milli: average(&ratings),
            hit_rate_percent: hit_rate,
            avg_age_days: avg_age,
            same_product,
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
        assert_eq!(a.same_product.len(), 1);
        let effect = &a.same_product[0];
        assert_eq!(effect.product, "blue dream");
        assert_eq!(
            (effect.here_avg_milli, effect.elsewhere_avg_milli),
            (5_000, 3_000)
        );
        assert_eq!((effect.here_sessions, effect.elsewhere_sessions), (5, 5));
        let b = found.iter().find(|s| s.shop == "b").unwrap();
        assert_eq!(b.same_product[0].here_avg_milli, 3_000);
        // Most-used shop first.
        assert_eq!(found[0].shop, "a");
    }
}

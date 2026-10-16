// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

//! Insights: plain statistics over the user's own sessions, never a model,
//! so every conclusion can be checked (docs/design-decisions.md, "Insights
//! engine").
//!
//! Each session is described by its rating and a set of **features** such
//! as `terpene:myrcene` or `method:vape_cart`. For every feature, the rated
//! sessions that have it are compared with those that don't. A comparison
//! becomes an [`Insight`] only with enough evidence on both sides
//! ([`MIN_SESSIONS`]) and a difference worth noticing ([`MIN_DIFFERENCE`]).
//! Each insight lists its sessions, and flags likely **confounders**: other
//! things that differ between the two groups (method, time of day, dose),
//! which might explain the difference instead.
//!
//! Nothing here sees text, places or identities: only the facts below.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};

mod shops;
mod timeline;

pub use shops::{Buy, Level, ShopEffect, ShopSessionFacts, ShopSummary, shop_insights};
pub use timeline::{MIN_TIMELINE_SESSIONS, ON, TimelineFacts, TimelinePattern, timeline_patterns};

/// Sessions needed on each side of a comparison (maintainer, 2026-10-12).
pub const MIN_SESSIONS: usize = 5;

/// The smallest difference in average rating that surfaces, in thousandths
/// of a star (maintainer, 2026-10-12: one star).
pub const MIN_DIFFERENCE: i64 = 1_000;

/// How much more common another feature must be on one side (in
/// percentage points) to be flagged as a likely confounder.
const CONFOUNDER_SHARE_PP: usize = 50;

/// How much higher one side's average dose must be (in percent of the
/// lower one) to flag dose as a likely confounder.
const CONFOUNDER_DOSE_PERCENT: i64 = 50;

/// The most insights returned, strongest first.
pub const MAX_INSIGHTS: usize = 12;

/// What one session contributes. Identified by an opaque index the caller
/// maps back to its record.
#[derive(Clone, Default)]
pub struct SessionFacts {
    /// The caller's handle for the session.
    pub index: usize,
    /// Overall rating, 1 to 5; unrated sessions are left out.
    pub rating: Option<u8>,
    /// Features, as `category:value` (`terpene:myrcene`, `method:edible`,
    /// `time:evening`).
    pub features: BTreeSet<String>,
    /// Total dose in milligrams (thousandths), when every line was dosed
    /// in mg; other units are not comparable, so they are left out.
    pub dose_mg_milli: Option<i64>,
}

/// A difference in ratings worth showing, with its evidence.
#[derive(Clone, PartialEq, Eq)]
pub struct Insight {
    /// The feature compared, such as `terpene:myrcene`.
    pub feature: String,
    /// Average rating of the sessions with it, in thousandths of a star.
    pub with_avg_milli: i64,
    /// Average rating of the sessions without it.
    pub without_avg_milli: i64,
    /// The sessions with it (the caller's indices), for tap-through.
    pub with_sessions: Vec<usize>,
    /// How many rated sessions are without it.
    pub without_count: usize,
    /// Likely confounders: `method`, `time` or `dose`.
    pub confounders: Vec<String>,
}

impl Insight {
    /// The feature's category, such as `terpene`.
    #[must_use]
    pub fn category(&self) -> &str {
        category(&self.feature)
    }
}

fn category(feature: &str) -> &str {
    feature.split_once(':').map_or(feature, |(c, _)| c)
}

/// The time-of-day feature for a local hour (0 to 23).
#[must_use]
pub fn time_of_day(hour: u32) -> &'static str {
    match hour {
        5..=10 => "time:morning",
        11..=16 => "time:afternoon",
        17..=21 => "time:evening",
        _ => "time:night",
    }
}

/// Every comparison that clears the thresholds, strongest first.
#[must_use]
pub fn insights(sessions: &[SessionFacts]) -> Vec<Insight> {
    let rated: Vec<&SessionFacts> = sessions.iter().filter(|s| s.rating.is_some()).collect();
    let features: BTreeSet<&String> = rated.iter().flat_map(|s| s.features.iter()).collect();
    let mut found = Vec::new();
    for feature in features {
        let (with, without): (Vec<&SessionFacts>, Vec<&SessionFacts>) =
            rated.iter().partition(|s| s.features.contains(feature));
        if with.len() < MIN_SESSIONS || without.len() < MIN_SESSIONS {
            continue;
        }
        let with_avg = average_rating(&with);
        let without_avg = average_rating(&without);
        if (with_avg - without_avg).abs() < MIN_DIFFERENCE {
            continue;
        }
        found.push(Insight {
            feature: feature.clone(),
            with_avg_milli: with_avg,
            without_avg_milli: without_avg,
            with_sessions: with.iter().map(|s| s.index).collect(),
            without_count: without.len(),
            confounders: confounders(category(feature), &with, &without),
        });
    }
    found.sort_by_key(|i| std::cmp::Reverse((i.with_avg_milli - i.without_avg_milli).abs()));
    found.truncate(MAX_INSIGHTS);
    found
}

/// Mean rating in thousandths of a star.
fn average_rating(sessions: &[&SessionFacts]) -> i64 {
    let total: i64 = sessions
        .iter()
        .filter_map(|s| s.rating)
        .map(|r| i64::from(r) * 1000)
        .sum();
    // Callers pass at least MIN_SESSIONS sessions, all rated.
    total / i64::try_from(sessions.len().max(1)).unwrap_or(1)
}

/// Other differences between the groups that might explain the result.
fn confounders(own: &str, with: &[&SessionFacts], without: &[&SessionFacts]) -> Vec<String> {
    let mut found = BTreeSet::new();
    for watched in ["method", "time"] {
        if watched == own {
            continue;
        }
        let share = |group: &[&SessionFacts]| -> BTreeMap<String, usize> {
            let mut counts = BTreeMap::new();
            for s in group {
                for f in s.features.iter().filter(|f| category(f) == watched) {
                    *counts.entry(f.clone()).or_insert(0) += 1;
                }
            }
            counts
                .into_iter()
                .map(|(f, n)| (f, n * 100 / group.len().max(1)))
                .collect()
        };
        let a = share(with);
        let b = share(without);
        let keys: BTreeSet<&String> = a.keys().chain(b.keys()).collect();
        if keys.into_iter().any(|k| {
            let x = a.get(k).copied().unwrap_or(0);
            let y = b.get(k).copied().unwrap_or(0);
            x.abs_diff(y) >= CONFOUNDER_SHARE_PP
        }) {
            found.insert(watched.to_owned());
        }
    }
    let dose = |group: &[&SessionFacts]| -> Option<i64> {
        let doses: Vec<i64> = group.iter().filter_map(|s| s.dose_mg_milli).collect();
        // Half the group at least, or the average says little.
        (doses.len() * 2 >= group.len() && !doses.is_empty())
            .then(|| doses.iter().sum::<i64>() / i64::try_from(doses.len()).unwrap_or(1))
    };
    if let (Some(a), Some(b)) = (dose(with), dose(without)) {
        let (low, high) = (a.min(b), a.max(b));
        if low > 0 && (high - low) * 100 >= low * CONFOUNDER_DOSE_PERCENT {
            found.insert("dose".to_owned());
        }
    }
    found.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session(index: usize, rating: u8, features: &[&str], dose: Option<i64>) -> SessionFacts {
        SessionFacts {
            index,
            rating: Some(rating),
            features: features.iter().map(|f| (*f).to_owned()).collect(),
            dose_mg_milli: dose,
        }
    }

    #[test]
    fn a_clear_difference_with_enough_sessions_surfaces() {
        let mut all = Vec::new();
        for i in 0..6 {
            all.push(session(
                i,
                2,
                &["terpene:myrcene", "method:smoked", "time:evening"],
                None,
            ));
        }
        for i in 6..20 {
            all.push(session(
                i,
                4,
                &["terpene:limonene", "method:smoked", "time:evening"],
                None,
            ));
        }
        let found = insights(&all);
        let myrcene = found
            .iter()
            .find(|i| i.feature == "terpene:myrcene")
            .unwrap();
        assert_eq!(myrcene.with_avg_milli, 2_000);
        assert_eq!(myrcene.without_avg_milli, 4_000);
        assert_eq!(myrcene.with_sessions, (0..6).collect::<Vec<_>>());
        assert_eq!(myrcene.without_count, 14);
        assert!(myrcene.confounders.is_empty());
        assert_eq!(myrcene.category(), "terpene");
        // Shared by every session: nothing to compare.
        assert!(found.iter().all(|i| i.feature != "method:smoked"));
    }

    #[test]
    fn too_few_sessions_or_too_small_a_difference_stays_quiet() {
        let mut few = Vec::new();
        for i in 0..4 {
            few.push(session(i, 1, &["form:edible"], None));
        }
        for i in 4..20 {
            few.push(session(i, 5, &["form:flower"], None));
        }
        assert!(insights(&few).iter().all(|i| i.feature != "form:edible"));

        let mut close = Vec::new();
        for i in 0..10 {
            close.push(session(
                i,
                if i % 2 == 0 { 3 } else { 4 },
                &["form:edible"],
                None,
            ));
        }
        for i in 10..20 {
            close.push(session(i, 4, &["form:flower"], None));
        }
        assert!(insights(&close).is_empty());
    }

    #[test]
    fn unrated_sessions_are_left_out() {
        let mut all = Vec::new();
        for i in 0..5 {
            all.push(session(i, 1, &["method:edible"], None));
        }
        for i in 5..10 {
            all.push(session(i, 5, &["method:smoked"], None));
        }
        all.push(SessionFacts {
            index: 99,
            rating: None,
            features: ["method:edible".to_owned()].into(),
            dose_mg_milli: None,
        });
        let found = insights(&all);
        let edible = found.iter().find(|i| i.feature == "method:edible").unwrap();
        assert!(!edible.with_sessions.contains(&99));
    }

    #[test]
    fn confounders_are_flagged() {
        let mut all = Vec::new();
        // Myrcene sessions are all late-night edibles at high doses.
        for i in 0..6 {
            all.push(session(
                i,
                2,
                &["terpene:myrcene", "method:edible", "time:night"],
                Some(20_000),
            ));
        }
        for i in 6..16 {
            all.push(session(
                i,
                4,
                &["terpene:limonene", "method:smoked", "time:afternoon"],
                Some(5_000),
            ));
        }
        let found = insights(&all);
        let myrcene = found
            .iter()
            .find(|i| i.feature == "terpene:myrcene")
            .unwrap();
        assert_eq!(myrcene.confounders, ["dose", "method", "time"]);
        // A method insight never flags method itself.
        let edible = found.iter().find(|i| i.feature == "method:edible").unwrap();
        assert!(!edible.confounders.contains(&"method".to_owned()));
    }

    #[test]
    fn strongest_first() {
        let mut all = Vec::new();
        for i in 0..5 {
            all.push(session(i, 1, &["form:edible", "terpene:pinene"], None));
        }
        for i in 5..10 {
            all.push(session(i, 3, &["form:edible", "terpene:limonene"], None));
        }
        for i in 10..15 {
            all.push(session(i, 5, &["form:flower", "terpene:limonene"], None));
        }
        let found = insights(&all);
        let gaps: Vec<i64> = found
            .iter()
            .map(|i| (i.with_avg_milli - i.without_avg_milli).abs())
            .collect();
        assert!(gaps.windows(2).all(|w| w[0] >= w[1]));
    }

    #[test]
    fn hours_fall_into_four_parts_of_the_day() {
        assert_eq!(time_of_day(4), "time:night");
        assert_eq!(time_of_day(5), "time:morning");
        assert_eq!(time_of_day(12), "time:afternoon");
        assert_eq!(time_of_day(20), "time:evening");
        assert_eq!(time_of_day(23), "time:night");
    }
}

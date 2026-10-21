// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

//! "How it usually goes": typical timelines from the points the user set
//! (docs/design-decisions.md, "Session timeline").
//!
//! Sessions are grouped (by method, product or strain); a group with
//! enough sessions that have set points gets its typical shape: when it
//! comes on (the first point at [`ON`] or stronger), when it peaks (the
//! strongest point, the earliest if tied) and how strong, and when it eases
//! off (the first point after the peak below [`ON`]). Each is the median
//! over the sessions that show it, and appears only when enough of them
//! do. Only points the user set are used, never the chart's guesses.

use std::collections::{BTreeMap, BTreeSet};

/// The level a session counts as "on" from: Mild (0 None, 1 Faint, 2 Mild,
/// 3 Clear, 4 Strong, 5 Intense).
pub const ON: u8 = 2;

/// Sessions with set points a group needs, and sessions that must show a
/// moment (coming on, easing off) for it to appear.
pub const MIN_TIMELINE_SESSIONS: usize = 3;

/// One session's timeline, by the caller's handle.
#[derive(Clone, Default)]
pub struct TimelineFacts {
    /// The caller's handle for the session.
    pub index: usize,
    /// Groups it belongs to, as `category:value` (`method:edible`,
    /// `product:<id>`, `strain:<id>`).
    pub groups: BTreeSet<String>,
    /// Levels the user set, by minutes after consumption.
    pub levels: BTreeMap<u32, u8>,
}

/// A group's typical timeline.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct TimelinePattern {
    /// The group, such as `method:edible`.
    pub group: String,
    /// The sessions with set points in it (the caller's indices).
    pub sessions: Vec<usize>,
    /// Median minutes until it comes on, when enough sessions show it.
    pub onset_min: Option<u32>,
    /// Median minutes until the peak.
    pub peak_min: u32,
    /// Median peak level.
    pub peak_level: u8,
    /// Median minutes until it eases off below [`ON`] after the peak, when
    /// enough sessions show it.
    pub eases_min: Option<u32>,
}

/// One session's shape: (onset, peak minute, peak level, eases).
type Shape = (Option<u32>, u32, u8, Option<u32>);

/// A measure, how to read it from a shape, and the smallest difference
/// worth stating.
type MeasureRule = (Measure, fn(&Shape) -> Option<u32>, u32);

fn shape(levels: &BTreeMap<u32, u8>) -> Option<Shape> {
    let (&peak_min, &peak_level) =
        levels
            .iter()
            .fold(None::<(&u32, &u8)>, |best, point| match best {
                Some(b) if b.1 >= point.1 => Some(b),
                _ => Some(point),
            })?;
    let onset = levels.iter().find(|(_, l)| **l >= ON).map(|(m, _)| *m);
    let eases = (peak_level >= ON)
        .then(|| {
            levels
                .range(peak_min + 1..)
                .find(|(_, l)| **l < ON)
                .map(|(m, _)| *m)
        })
        .flatten();
    Some((onset, peak_min, peak_level, eases))
}

fn median<T: Ord + Copy>(mut values: Vec<T>) -> Option<T> {
    if values.is_empty() {
        return None;
    }
    values.sort_unstable();
    // The lower middle for an even count: an actual observed value.
    Some(values[(values.len() - 1) / 2])
}

/// Every group with at least [`MIN_TIMELINE_SESSIONS`] sessions that have
/// set points, by group.
#[must_use]
pub fn timeline_patterns(sessions: &[TimelineFacts]) -> Vec<TimelinePattern> {
    let mut groups: BTreeMap<&str, Vec<(usize, Shape)>> = BTreeMap::new();
    for s in sessions {
        let Some(shape) = shape(&s.levels) else {
            continue;
        };
        for group in &s.groups {
            groups.entry(group).or_default().push((s.index, shape));
        }
    }
    groups
        .into_iter()
        .filter(|(_, members)| members.len() >= MIN_TIMELINE_SESSIONS)
        .filter_map(|(group, members)| {
            let enough = |v: Vec<u32>| {
                (v.len() >= MIN_TIMELINE_SESSIONS)
                    .then(|| median(v))
                    .flatten()
            };
            let onsets = members.iter().filter_map(|(_, s)| s.0).collect();
            let eases = members.iter().filter_map(|(_, s)| s.3).collect();
            Some(TimelinePattern {
                group: group.to_owned(),
                sessions: members.iter().map(|(i, _)| *i).collect(),
                onset_min: enough(onsets),
                peak_min: median(members.iter().map(|(_, s)| s.1).collect())?,
                peak_level: median(members.iter().map(|(_, s)| s.2).collect())?,
                eases_min: enough(eases),
            })
        })
        .collect()
}

/// What a comparison measures.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Measure {
    /// Minutes until it comes on.
    Onset,
    /// Minutes until the peak.
    PeakTime,
    /// The peak's level.
    PeakLevel,
    /// Minutes until it eases off after the peak.
    Eases,
}

impl Measure {
    /// `onset`, `peak_time`, `peak_level` or `eases`.
    #[must_use]
    pub const fn slug(self) -> &'static str {
        match self {
            Self::Onset => "onset",
            Self::PeakTime => "peak_time",
            Self::PeakLevel => "peak_level",
            Self::Eases => "eases",
        }
    }
}

/// "Comes on faster than your other edibles": a product's or strain's
/// timeline against the user's other sessions with the same method.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct TimelineComparison {
    /// The group, such as `product:<id>`.
    pub group: String,
    /// The method compared within, such as `edible`.
    pub method: String,
    /// What differs.
    pub measure: Measure,
    /// The group's median (minutes, or a level for [`Measure::PeakLevel`]).
    pub this: u32,
    /// The other sessions' median.
    pub others: u32,
    /// The group's sessions that show the measure (the caller's indices).
    pub sessions: Vec<usize>,
    /// How many other sessions show it.
    pub others_count: usize,
}

/// The share of a group's sessions that must sit on the same side of the
/// others' median, in thirds: two thirds.
const SAME_SIDE_THIRDS: usize = 2;

/// Comparisons worth stating, for every product and strain group
/// (`product:…`, `strain:…`) whose sessions all share a method.
///
/// Sparse points could mislead (docs/design-decisions.md, "Session
/// timeline"), so a comparison appears only when:
/// - both sides have at least [`MIN_TIMELINE_SESSIONS`] sessions showing
///   the measure: the group's, and the method's sessions outside the group;
/// - the medians differ by at least one point of the method's timeline
///   (`steps`: minutes between points, by method slug) for a time, or by a
///   whole level for the peak's strength, since a smaller gap is below what
///   the points can tell apart;
/// - at least two thirds of the group's sessions lie on the same side of the
///   others' median, so one unusual session can't make the difference.
#[must_use]
pub fn timeline_comparisons(
    sessions: &[TimelineFacts],
    steps: &BTreeMap<String, u32>,
) -> Vec<TimelineComparison> {
    let shaped: Vec<(&TimelineFacts, Shape)> = sessions
        .iter()
        .filter_map(|s| shape(&s.levels).map(|sh| (s, sh)))
        .collect();
    let mut groups: BTreeSet<&str> = BTreeSet::new();
    for (s, _) in &shaped {
        for g in &s.groups {
            if g.starts_with("product:") || g.starts_with("strain:") {
                groups.insert(g);
            }
        }
    }
    let mut out = Vec::new();
    for group in groups {
        let members: Vec<&(&TimelineFacts, Shape)> = shaped
            .iter()
            .filter(|(s, _)| s.groups.contains(group))
            .collect();
        if members.len() < MIN_TIMELINE_SESSIONS {
            continue;
        }
        // The first method every session of the group shares.
        let Some(method) = members[0].0.groups.iter().find(|g| {
            g.starts_with("method:") && members.iter().all(|(s, _)| s.groups.contains(*g))
        }) else {
            continue;
        };
        let slug = method.trim_start_matches("method:");
        let Some(&step) = steps.get(slug) else {
            continue;
        };
        let others: Vec<&(&TimelineFacts, Shape)> = shaped
            .iter()
            .filter(|(s, _)| s.groups.contains(method) && !s.groups.contains(group))
            .collect();
        let measures: [MeasureRule; 4] = [
            (Measure::Onset, |s| s.0, step),
            (Measure::PeakTime, |s| Some(s.1), step),
            (Measure::PeakLevel, |s| Some(u32::from(s.2)), 1),
            (Measure::Eases, |s| s.3, step),
        ];
        for (measure, value, threshold) in measures {
            let mine: Vec<(usize, u32)> = members
                .iter()
                .filter_map(|(s, sh)| value(sh).map(|v| (s.index, v)))
                .collect();
            let theirs: Vec<u32> = others.iter().filter_map(|(_, sh)| value(sh)).collect();
            if mine.len() < MIN_TIMELINE_SESSIONS || theirs.len() < MIN_TIMELINE_SESSIONS {
                continue;
            }
            let (Some(this), Some(other)) = (
                median(mine.iter().map(|(_, v)| *v).collect()),
                median(theirs.clone()),
            ) else {
                continue;
            };
            if this.abs_diff(other) < threshold.max(1) {
                continue;
            }
            let same_side = mine
                .iter()
                .filter(|(_, v)| if this < other { *v < other } else { *v > other })
                .count();
            if same_side * 3 < mine.len() * SAME_SIDE_THIRDS {
                continue;
            }
            out.push(TimelineComparison {
                group: group.to_owned(),
                method: slug.to_owned(),
                measure,
                this,
                others: other,
                sessions: mine.iter().map(|(i, _)| *i).collect(),
                others_count: theirs.len(),
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts(index: usize, groups: &[&str], points: &[(u32, u8)]) -> TimelineFacts {
        TimelineFacts {
            index,
            groups: groups.iter().map(|g| (*g).to_owned()).collect(),
            levels: points.iter().copied().collect(),
        }
    }

    #[test]
    fn typical_shape_from_set_points() {
        let sessions = [
            facts(
                0,
                &["method:edible"],
                &[(48, 1), (96, 3), (144, 4), (240, 1)],
            ),
            facts(1, &["method:edible"], &[(48, 2), (96, 4), (192, 1)]),
            facts(2, &["method:edible"], &[(48, 1), (144, 4), (240, 2)]),
            facts(3, &["method:dabbed"], &[(20, 4)]),
        ];
        let found = timeline_patterns(&sessions);
        assert_eq!(found.len(), 1, "dabs have too few sessions");
        let edible = &found[0];
        assert_eq!(edible.group, "method:edible");
        assert_eq!(edible.sessions, [0, 1, 2]);
        // Onsets 96, 48, 144 → 96.
        assert_eq!(edible.onset_min, Some(96));
        // Peaks 144 (4), 96 (4), 144 (4) → 144 at 4.
        assert_eq!(edible.peak_min, 144);
        assert_eq!(edible.peak_level, 4);
        // Easing off: 240 and 192 only (the third never drops below Mild).
        assert_eq!(edible.eases_min, None);
    }

    #[test]
    fn eases_off_once_enough_sessions_show_it() {
        let sessions: Vec<TimelineFacts> = (0..4)
            .map(|i| facts(i, &["product:a"], &[(30, 3), (60, 1)]))
            .collect();
        let p = &timeline_patterns(&sessions)[0];
        assert_eq!(
            (p.onset_min, p.peak_min, p.peak_level, p.eases_min),
            (Some(30), 30, 3, Some(60))
        );
    }

    #[test]
    fn sessions_without_points_are_left_out() {
        let sessions = [
            facts(0, &["method:smoked"], &[]),
            facts(1, &["method:smoked"], &[(30, 2)]),
            facts(2, &["method:smoked"], &[(30, 2)]),
        ];
        assert!(timeline_patterns(&sessions).is_empty());
    }

    #[test]
    fn a_faint_session_has_no_onset() {
        let sessions: Vec<TimelineFacts> = (0..3)
            .map(|i| facts(i, &["method:topical"], &[(40, 1), (80, 1)]))
            .collect();
        let p = &timeline_patterns(&sessions)[0];
        assert_eq!(
            (p.onset_min, p.peak_min, p.peak_level, p.eases_min),
            (None, 40, 1, None)
        );
    }

    fn steps() -> BTreeMap<String, u32> {
        [("edible".to_owned(), 48)].into()
    }

    #[test]
    fn compares_a_product_with_the_methods_other_sessions() {
        let mut sessions = Vec::new();
        // The gummies come on at 48 min, peak at 96 at Strong (4).
        for i in 0..3 {
            sessions.push(facts(
                i,
                &["method:edible", "product:gummy"],
                &[(48, 2), (96, 4), (192, 1)],
            ));
        }
        // Other edibles come on at 144 min, peak at 192 at Clear (3).
        for i in 3..7 {
            sessions.push(facts(
                i,
                &["method:edible", "product:brownie"],
                &[(48, 0), (144, 2), (192, 3), (240, 1)],
            ));
        }
        let found = timeline_comparisons(&sessions, &steps());
        let gummy: Vec<(Measure, u32, u32)> = found
            .iter()
            .filter(|c| c.group == "product:gummy")
            .map(|c| (c.measure, c.this, c.others))
            .collect();
        assert_eq!(
            gummy,
            [
                (Measure::Onset, 48, 144),
                (Measure::PeakTime, 96, 192),
                (Measure::PeakLevel, 4, 3),
                // Eases off at 192 against 240: exactly one point apart.
                (Measure::Eases, 192, 240),
            ]
        );
        let onset = found
            .iter()
            .find(|c| c.group == "product:gummy" && c.measure == Measure::Onset)
            .unwrap();
        assert_eq!(onset.method, "edible");
        assert_eq!(onset.sessions, [0, 1, 2]);
        assert_eq!(onset.others_count, 4);
    }

    #[test]
    fn small_or_scattered_differences_say_nothing() {
        let mut sessions = Vec::new();
        // Less than one point apart: 48 against 72 is under a 48 min step.
        for i in 0..3 {
            sessions.push(facts(i, &["method:edible", "product:a"], &[(48, 3)]));
        }
        for i in 3..6 {
            sessions.push(facts(i, &["method:edible"], &[(72, 3)]));
        }
        assert!(timeline_comparisons(&sessions, &steps()).is_empty());

        // A median far apart, but only one of three sessions on that side.
        let mut scattered = Vec::new();
        for (i, onset) in [(0, 48), (1, 192), (2, 48)] {
            scattered.push(facts(i, &["method:edible", "product:a"], &[(onset, 3)]));
        }
        scattered[1].levels = [(48, 0), (192, 3)].into();
        for i in 3..6 {
            scattered.push(facts(i, &["method:edible"], &[(48, 0), (96, 0), (144, 3)]));
        }
        let found = timeline_comparisons(&scattered, &steps());
        assert!(found.iter().all(|c| c.measure != Measure::PeakLevel));
        // Two of three come on at 48 < 144: that one stands.
        assert!(
            found
                .iter()
                .any(|c| c.measure == Measure::Onset && c.this == 48)
        );

        // Too few other sessions to compare with.
        let few: Vec<TimelineFacts> = (0..5)
            .map(|i| facts(i, &["method:edible", "product:a"], &[(48, 3)]))
            .collect();
        assert!(timeline_comparisons(&few, &steps()).is_empty());
    }
}

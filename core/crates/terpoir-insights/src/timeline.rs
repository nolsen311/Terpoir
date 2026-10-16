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
}

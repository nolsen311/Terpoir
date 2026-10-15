// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

//! Sessions: the journal entry.

use std::collections::{BTreeMap, BTreeSet};
use terpoir_crypto::RecordId;

use super::{Fields, LocalTime, Reader, Record, RecordKind, TagRef, owns, slug_enum};
use crate::{Document, Error, Result, Value};

slug_enum! {
    /// How the product was consumed.
    Method {
        /// Smoked flower (joint, pipe, bong).
        Smoked => "smoked",
        /// Vaporized flower.
        VapedFlower => "vaped_flower",
        /// Vape cartridge or disposable.
        VapeCart => "vape_cart",
        /// Dabbed concentrate.
        Dabbed => "dabbed",
        /// Eaten.
        Edible => "edible",
        /// Tincture or other sublingual.
        Tincture => "tincture",
        /// Drink.
        Beverage => "beverage",
        /// Applied to the skin.
        Topical => "topical",
        /// A transdermal patch: through the skin into the bloodstream, for
        /// hours (unlike a topical).
        Transdermal => "transdermal",
    }
}

slug_enum! {
    /// Unit of a dose; method-specific.
    DoseUnit {
        /// Hits or pulls (flower).
        Hits => "hits",
        /// Fraction of a bowl (flower), amount in thousandths.
        Bowls => "bowls",
        /// Puffs (carts).
        Puffs => "puffs",
        /// Dabs (concentrates).
        Dabs => "dabs",
        /// Milligrams.
        Milligrams => "mg",
        /// Millilitres.
        Millilitres => "ml",
    }
}

slug_enum! {
    /// Why the session happened.
    Intention {
        /// To sleep.
        Sleep => "sleep",
        /// To relax.
        Relax => "relax",
        /// To create.
        Create => "create",
        /// For relief.
        Relief => "relief",
    }
}

slug_enum! {
    /// Alone or with others. Deliberately no place or relationship labels.
    Social {
        /// Alone.
        Solo => "solo",
        /// With other people.
        WithOthers => "with_others",
    }
}

slug_enum! {
    /// Outcome for a targeted symptom.
    SymptomResult {
        /// 👍 helped.
        Helped => "helped",
        /// 👎 did not help.
        DidNotHelp => "did_not_help",
        /// Targeted, but the user was unsure and skipped rating it.
        NotRated => "not_rated",
    }
}

slug_enum! {
    /// Built-in targeted symptoms: symptoms, never conditions or diagnoses
    /// (the symptom map also accepts custom ones). Slugs are unique across
    /// every built-in tag vocabulary.
    TargetedSymptom {
        /// Sleep.
        Sleep => "sleep",
        /// Pain.
        Pain => "pain",
        /// Anxiety.
        Anxiety => "anxiety",
        /// Appetite.
        Appetite => "appetite",
        /// Nausea.
        Nausea => "nausea",
        /// Focus.
        Focus => "focus",
    }
}

impl Method {
    /// The dose units that make sense for this method, the first being the
    /// usual one. Empty when doses are not measured (topicals); every unit
    /// for a method from a newer app version.
    #[must_use]
    pub fn dose_units(&self) -> &'static [DoseUnit] {
        match self {
            Self::Smoked | Self::VapedFlower => &[DoseUnit::Hits, DoseUnit::Bowls],
            Self::VapeCart => &[DoseUnit::Puffs],
            Self::Dabbed => &[DoseUnit::Dabs],
            Self::Edible | Self::Beverage | Self::Transdermal => &[DoseUnit::Milligrams],
            Self::Tincture => &[DoseUnit::Millilitres, DoseUnit::Milligrams],
            Self::Topical => &[],
            Self::Unknown(_) => &ALL_DOSE_UNITS,
        }
    }

    /// The default delay of a follow-up check-in after this method, in
    /// minutes, or `None` for no check-in by default: edibles and drinks
    /// take longest to come on, patches act over hours, tinctures come on
    /// sooner; inhaled and topical need none, nor does a method from a
    /// newer app version.
    #[must_use]
    pub const fn check_in_minutes(&self) -> Option<u32> {
        match self {
            Self::Edible | Self::Beverage => Some(90),
            Self::Transdermal => Some(120),
            Self::Tincture => Some(45),
            Self::Smoked
            | Self::VapedFlower
            | Self::VapeCart
            | Self::Dabbed
            | Self::Topical
            | Self::Unknown(_) => None,
        }
    }
}

/// The default check-in delay for a session with these methods: the
/// longest of theirs; `None` when none of them asks for one.
pub fn check_in_minutes_for<'a>(methods: impl IntoIterator<Item = &'a Method>) -> Option<u32> {
    methods
        .into_iter()
        .filter_map(Method::check_in_minutes)
        .max()
}

/// How a method's timeline is asked for: its length, how many points it
/// has (evenly spaced unless [`at`](Self::at) places them, the last at the
/// end), and how many of them the first gesture sets
/// (docs/design-decisions.md, "Session timeline").
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct TimelinePlan {
    /// Length in minutes.
    pub minutes: u32,
    /// Points along it.
    pub points: u32,
    /// Points the first gesture sets.
    pub gesture_points: u32,
    /// The points' times in minutes, for uneven spacing (ascending, as
    /// many as [`points`](Self::points), the last at the end). Empty for
    /// evenly spaced points.
    pub at: &'static [u32],
}

impl TimelinePlan {
    /// The points' times in minutes: [`at`](Self::at) when set, else
    /// evenly spaced, the last at the end.
    #[must_use]
    pub fn point_minutes(&self) -> Vec<u32> {
        if !self.at.is_empty() {
            return self.at.to_vec();
        }
        (1..=self.points)
            .map(|i| self.minutes * i / self.points.max(1))
            .collect()
    }
}

/// A patch's points: 1, 2, 4, 8 and 12 hours (maintainer, 2026-10-15).
const TRANSDERMAL_POINTS: &[u32] = &[60, 120, 240, 480, 720];

impl Method {
    /// The timeline asked for after this method (maintainer, 2026-10-15).
    /// A method from a newer app version gets the smoked one.
    #[must_use]
    pub const fn timeline_plan(&self) -> TimelinePlan {
        let (minutes, points, gesture_points, at): (u32, u32, u32, &'static [u32]) = match self {
            Self::Dabbed => (60, 3, 1, &[]),
            Self::Smoked | Self::VapedFlower | Self::VapeCart | Self::Unknown(_) => {
                (120, 4, 2, &[])
            }
            Self::Edible | Self::Beverage | Self::Tincture => (240, 5, 2, &[]),
            Self::Topical => (120, 3, 2, &[]),
            Self::Transdermal => (720, 5, 2, TRANSDERMAL_POINTS),
        };
        TimelinePlan {
            minutes,
            points,
            gesture_points,
            at,
        }
    }
}

const ALL_DOSE_UNITS: [DoseUnit; 6] = [
    DoseUnit::Hits,
    DoseUnit::Bowls,
    DoseUnit::Puffs,
    DoseUnit::Dabs,
    DoseUnit::Milligrams,
    DoseUnit::Millilitres,
];

/// The fixed core effects vocabulary. Custom effects are [`TagRef::Custom`].
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[non_exhaustive]
pub enum CoreEffect {
    /// Relaxed.
    Relaxed,
    /// Euphoric.
    Euphoric,
    /// Focused.
    Focused,
    /// Creative.
    Creative,
    /// Sleepy.
    Sleepy,
    /// Hungry.
    Hungry,
    /// Anxious.
    Anxious,
    /// Paranoid.
    Paranoid,
    /// Dry mouth.
    DryMouth,
    /// Couch-lock.
    CouchLock,
}

impl CoreEffect {
    /// Every core effect, in display order.
    pub const ALL: [Self; 10] = [
        Self::Relaxed,
        Self::Euphoric,
        Self::Focused,
        Self::Creative,
        Self::Sleepy,
        Self::Hungry,
        Self::Anxious,
        Self::Paranoid,
        Self::DryMouth,
        Self::CouchLock,
    ];

    /// The stored slug.
    #[must_use]
    pub const fn slug(self) -> &'static str {
        match self {
            Self::Relaxed => "relaxed",
            Self::Euphoric => "euphoric",
            Self::Focused => "focused",
            Self::Creative => "creative",
            Self::Sleepy => "sleepy",
            Self::Hungry => "hungry",
            Self::Anxious => "anxious",
            Self::Paranoid => "paranoid",
            Self::DryMouth => "dry_mouth",
            Self::CouchLock => "couch_lock",
        }
    }
}

impl From<CoreEffect> for TagRef {
    fn from(effect: CoreEffect) -> Self {
        Self::Builtin(effect.slug().to_owned())
    }
}

/// An amount in a method-specific unit.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Dose {
    /// Amount in thousandths of `unit` (2.5 mg is 2500).
    pub amount_milli: i64,
    /// The unit.
    pub unit: DoseUnit,
}

/// A reference to one line of a [`crate::Purchase`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct PurchaseRef {
    /// The purchase.
    pub purchase: RecordId,
    /// The line within it.
    pub line: RecordId,
}

/// One product consumed in a session.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ProductLine {
    /// The product, if recorded.
    pub product: Option<RecordId>,
    /// How it was consumed.
    pub method: Method,
    /// How much, if recorded.
    pub dose: Option<Dose>,
    /// The purchase line this came from, if recorded.
    pub purchase: Option<PurchaseRef>,
    /// The Re-up Spot it was bought at, when the user picked one (for a
    /// product bought at several). Stored in the session's `line_shops`
    /// map, so apps that predate it keep it untouched.
    pub shop: Option<RecordId>,
}

impl ProductLine {
    fn to_value(&self) -> Value {
        Value::Tuple(vec![
            self.product.map_or(Value::Null, Value::Id),
            Value::Text(self.method.slug().to_owned()),
            self.dose
                .as_ref()
                .map_or(Value::Null, |d| Value::Int(d.amount_milli)),
            self.dose
                .as_ref()
                .map_or(Value::Null, |d| Value::Text(d.unit.slug().to_owned())),
            self.purchase.map_or(Value::Null, |p| Value::Id(p.purchase)),
            self.purchase.map_or(Value::Null, |p| Value::Id(p.line)),
        ])
    }

    fn from_value(value: &Value) -> Option<Self> {
        let Value::Tuple(items) = value else {
            return None;
        };
        // Lines written before purchases existed have four items.
        let (product, method, amount, unit, purchase, line) = match items.as_slice() {
            [product, Value::Text(method), amount, unit] => {
                (product, method, amount, unit, &Value::Null, &Value::Null)
            }
            [product, Value::Text(method), amount, unit, purchase, line] => {
                (product, method, amount, unit, purchase, line)
            }
            _ => return None,
        };
        let purchase = match (purchase, line) {
            (Value::Null, Value::Null) => None,
            (Value::Id(purchase), Value::Id(line)) => Some(PurchaseRef {
                purchase: *purchase,
                line: *line,
            }),
            _ => return None,
        };
        let product = match product {
            Value::Null => None,
            Value::Id(id) => Some(*id),
            _ => return None,
        };
        let dose = match (amount, unit) {
            (Value::Null, Value::Null) => None,
            (Value::Int(amount_milli), Value::Text(unit)) => Some(Dose {
                amount_milli: *amount_milli,
                unit: DoseUnit::from_slug(unit),
            }),
            _ => return None,
        };
        Some(Self {
            product,
            method: Method::from_slug(method),
            dose,
            purchase,
            shop: None,
        })
    }
}

/// Onset, peak and duration, in minutes after consumption.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub struct Timeline {
    /// Minutes until first noticed.
    pub onset_min: Option<u32>,
    /// Minutes until strongest.
    pub peak_min: Option<u32>,
    /// Total minutes.
    pub duration_min: Option<u32>,
}

/// A consumption session: the journal entry.
#[derive(Clone, PartialEq, Eq)]
pub struct Session {
    /// When it happened (editable after the fact).
    pub when: LocalTime,
    /// Products consumed, keyed by line ID so lines merge independently.
    pub lines: BTreeMap<RecordId, ProductLine>,
    /// What the user was doing.
    pub activity: Option<String>,
    /// Why.
    pub intention: Option<Intention>,
    /// Mood before, 1 to 5.
    pub mood_before: Option<u8>,
    /// Alone or with others.
    pub social: Option<Social>,
    /// Effect intensities, 0 to 5.
    pub effects: BTreeMap<TagRef, u8>,
    /// Onset, peak and duration (asked for before 0.5.0; kept as saved).
    pub timeline: Timeline,
    /// How strong it felt over time: minutes after consumption → 0 to 5.
    /// Only the points the user set; nothing guessed is stored.
    pub timeline_levels: BTreeMap<u32, u8>,
    /// When the timeline levels were last changed, in Unix milliseconds.
    pub timeline_updated_ms: Option<i64>,
    /// Mood after, 1 to 5.
    pub mood_after: Option<u8>,
    /// Overall rating, 1 to 5.
    pub rating: Option<u8>,
    /// Would do it again.
    pub would_repeat: Option<bool>,
    /// Where, if the user attached a spot.
    pub spot: Option<RecordId>,
    /// Targeted symptoms (opt-in section).
    pub symptoms: BTreeMap<TagRef, SymptomResult>,
    /// Attached photo and voice-clip file IDs.
    pub attachments: BTreeSet<RecordId>,
}

impl Session {
    /// The timeline plan for the session's methods: the longest of them
    /// (on a tie, the one with more points); smoked's with no lines.
    #[must_use]
    pub fn timeline_plan(&self) -> TimelinePlan {
        timeline_plan_for(self.lines.values().map(|l| &l.method))
    }

    /// A session at `when` with nothing else filled in.
    #[must_use]
    pub const fn new(when: LocalTime) -> Self {
        Self {
            when,
            lines: BTreeMap::new(),
            activity: None,
            intention: None,
            mood_before: None,
            social: None,
            effects: BTreeMap::new(),
            timeline: Timeline {
                onset_min: None,
                peak_min: None,
                duration_min: None,
            },
            timeline_levels: BTreeMap::new(),
            timeline_updated_ms: None,
            mood_after: None,
            rating: None,
            would_repeat: None,
            spot: None,
            symptoms: BTreeMap::new(),
            attachments: BTreeSet::new(),
        }
    }
}

/// The timeline plan for a set of methods; see [`Session::timeline_plan`].
pub fn timeline_plan_for<'a>(methods: impl IntoIterator<Item = &'a Method>) -> TimelinePlan {
    methods
        .into_iter()
        .map(Method::timeline_plan)
        .max_by_key(|p| (p.minutes, p.points))
        .unwrap_or_else(|| Method::Smoked.timeline_plan())
}

const SCALARS: &[&str] = &[
    "when",
    "activity",
    "intention",
    "mood_before",
    "social",
    "onset_min",
    "peak_min",
    "duration_min",
    "timeline_updated",
    "mood_after",
    "rating",
    "would_repeat",
    "spot",
];
const MAPS: &[&str] = &[
    "lines",
    "line_shops",
    "effects",
    "timeline_levels",
    "symptoms",
    "attachments",
];

impl Record for Session {
    const KIND: RecordKind = RecordKind::Session;

    fn to_fields(&self) -> BTreeMap<String, Value> {
        let int = |v: u32| Value::Int(i64::from(v));
        let small = |v: u8| Value::Int(i64::from(v));
        let mut f = Fields::default();
        f.put("when", self.when.to_value())
            .text("activity", self.activity.as_ref())
            .opt("intention", self.intention.as_ref(), |i| {
                Value::Text(i.slug().to_owned())
            })
            .opt("mood_before", self.mood_before, small)
            .opt("social", self.social.as_ref(), |s| {
                Value::Text(s.slug().to_owned())
            })
            .opt("onset_min", self.timeline.onset_min, int)
            .opt("peak_min", self.timeline.peak_min, int)
            .opt("duration_min", self.timeline.duration_min, int)
            .opt("timeline_updated", self.timeline_updated_ms, Value::Int)
            .opt("mood_after", self.mood_after, small)
            .opt("rating", self.rating, small)
            .opt("would_repeat", self.would_repeat, Value::Bool)
            .opt("spot", self.spot, Value::Id)
            .map("lines", &self.lines, |id| id.to_hex(), |l| l.to_value())
            .map(
                "line_shops",
                self.lines
                    .iter()
                    .filter_map(|(id, l)| l.shop.map(|shop| (id, shop))),
                |id| id.to_hex(),
                Value::Id,
            )
            .map("effects", &self.effects, |t| t.to_key(), |v| small(*v))
            .map(
                "timeline_levels",
                &self.timeline_levels,
                ToString::to_string,
                |v| small(*v),
            )
            .map(
                "symptoms",
                &self.symptoms,
                |t| t.to_key(),
                |r| Value::Text(r.slug().to_owned()),
            )
            .id_set("attachments", &self.attachments);
        f.finish()
    }

    fn from_document(doc: &Document) -> Result<Self> {
        let r = Reader::new::<Self>(doc)?;
        let mut lines: BTreeMap<RecordId, ProductLine> = r
            .entries("lines")
            .map(|(k, v)| {
                Ok((
                    RecordId::from_hex(k).map_err(|_| Error::InvalidField("lines"))?,
                    ProductLine::from_value(v).ok_or(Error::InvalidField("lines"))?,
                ))
            })
            .collect::<Result<_>>()?;
        // A shop for a line that is gone (removed on another device) is
        // ignored.
        for (k, v) in r.entries("line_shops") {
            let (Ok(id), Value::Id(shop)) = (RecordId::from_hex(k), v) else {
                return Err(Error::InvalidField("line_shops"));
            };
            if let Some(line) = lines.get_mut(&id) {
                line.shop = Some(*shop);
            }
        }
        let timeline_levels = r
            .entries("timeline_levels")
            .map(|(k, v)| {
                let minute = k.parse::<u32>().ok();
                let level = match v {
                    Value::Int(i) => u8::try_from(*i).ok().filter(|l| *l <= 5),
                    _ => None,
                };
                minute
                    .zip(level)
                    .ok_or(Error::InvalidField("timeline_levels"))
            })
            .collect::<Result<_>>()?;
        Ok(Self {
            when: LocalTime::from_value(r.value("when"), "when")?,
            lines,
            activity: r.text("activity")?,
            intention: r.slug("intention", Intention::from_slug)?,
            mood_before: r.int_in("mood_before", 1..=5)?,
            social: r.slug("social", Social::from_slug)?,
            effects: r.tag_map("effects", |v| match v {
                Value::Int(i) => u8::try_from(*i).ok().filter(|i| *i <= 5),
                _ => None,
            })?,
            timeline: Timeline {
                onset_min: r.int_in("onset_min", 0..=u32::MAX)?,
                peak_min: r.int_in("peak_min", 0..=u32::MAX)?,
                duration_min: r.int_in("duration_min", 0..=u32::MAX)?,
            },
            timeline_levels,
            timeline_updated_ms: r.int_in("timeline_updated", i64::MIN..=i64::MAX)?,
            mood_after: r.int_in("mood_after", 1..=5)?,
            rating: r.int_in("rating", 1..=5)?,
            would_repeat: r.bool("would_repeat")?,
            spot: r.id("spot")?,
            symptoms: r.tag_map("symptoms", |v| match v {
                Value::Text(s) => Some(SymptomResult::from_slug(s)),
                _ => None,
            })?,
            attachments: r.id_set("attachments")?,
        })
    }

    fn owns_field(key: &str) -> bool {
        owns(key, SCALARS, MAPS)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DeviceId, Hlc, Stamp};

    #[test]
    fn every_measured_method_has_a_usual_unit() {
        for slug in Method::SLUGS {
            let method = Method::from_slug(slug);
            assert!(!matches!(method, Method::Unknown(_)), "{slug}");
            if method == Method::Topical {
                assert!(method.dose_units().is_empty());
            } else {
                assert!(!method.dose_units().is_empty(), "{slug}");
            }
        }
        assert_eq!(Method::Smoked.dose_units()[0], DoseUnit::Hits);
        assert_eq!(Method::VapeCart.dose_units(), &[DoseUnit::Puffs]);
        assert_eq!(Method::Transdermal.dose_units(), &[DoseUnit::Milligrams]);
        // A method from a newer app version offers every unit.
        assert_eq!(
            Method::from_slug("nebulized").dose_units().len(),
            DoseUnit::SLUGS.len()
        );
    }

    #[test]
    fn slug_lists_round_trip() {
        for slug in DoseUnit::SLUGS
            .iter()
            .chain(Intention::SLUGS)
            .chain(Social::SLUGS)
        {
            assert!(
                [
                    DoseUnit::from_slug(slug).slug(),
                    Intention::from_slug(slug).slug(),
                    Social::from_slug(slug).slug()
                ]
                .contains(slug)
            );
        }
        assert!(!matches!(
            DoseUnit::from_slug(DoseUnit::SLUGS[0]),
            DoseUnit::Unknown(_)
        ));
    }

    fn stamp(ms: u64, device: u8) -> Stamp {
        Stamp {
            hlc: Hlc {
                wall_ms: ms,
                counter: 0,
            },
            device: DeviceId::from_bytes([device; 16]),
        }
    }

    fn id(n: u8) -> RecordId {
        RecordId::from_bytes([n; 16])
    }

    pub(crate) fn full_session() -> Session {
        let mut s = Session::new(LocalTime {
            unix_ms: 1_760_000_000_000,
            utc_offset_min: -420,
        });
        s.lines.insert(
            id(1),
            ProductLine {
                product: Some(id(2)),
                method: Method::Edible,
                dose: Some(Dose {
                    amount_milli: 2500,
                    unit: DoseUnit::Milligrams,
                }),
                purchase: Some(PurchaseRef {
                    purchase: id(4),
                    line: id(5),
                }),
                shop: Some(id(6)),
            },
        );
        s.lines.insert(
            id(3),
            ProductLine {
                product: None,
                method: Method::Unknown("future_method".into()),
                dose: None,
                purchase: None,
                shop: None,
            },
        );
        s.activity = Some("hiking".into());
        s.intention = Some(Intention::Relax);
        s.mood_before = Some(2);
        s.social = Some(Social::WithOthers);
        s.effects.insert(CoreEffect::Relaxed.into(), 4);
        s.effects.insert(TagRef::Custom(id(9)), 0);
        s.timeline = Timeline {
            onset_min: Some(45),
            peak_min: Some(90),
            duration_min: None,
        };
        s.timeline_levels.insert(30, 2);
        s.timeline_levels.insert(60, 4);
        s.timeline_updated_ms = Some(1_760_000_000_000);
        s.mood_after = Some(4);
        s.rating = Some(5);
        s.would_repeat = Some(true);
        s.spot = Some(id(4));
        s.symptoms
            .insert(TagRef::Builtin("sleep".into()), SymptomResult::Helped);
        s.attachments.insert(id(5));
        s
    }

    #[test]
    fn round_trip_through_document_and_bytes() {
        let session = full_session();
        let doc = Document::create(&session, stamp(1, 1)).unwrap();
        let decoded = crate::migrate(Document::decode(&doc.encode().unwrap()).unwrap()).unwrap();
        assert_eq!(Session::from_document(&decoded).unwrap(), session);
    }

    #[test]
    fn minimal_session_round_trips() {
        let session = Session::new(LocalTime {
            unix_ms: 0,
            utc_offset_min: 0,
        });
        let doc = Document::create(&session, stamp(1, 1)).unwrap();
        assert_eq!(doc.fields().len(), 1);
        assert_eq!(Session::from_document(&doc).unwrap(), session);
    }

    #[test]
    fn removing_a_map_entry_writes_null_with_new_stamp() {
        let mut session = full_session();
        let mut doc = Document::create(&session, stamp(1, 1)).unwrap();
        session.effects.remove(&TagRef::from(CoreEffect::Relaxed));
        session.rating = None;
        assert_eq!(doc.update(&session, stamp(2, 1)).unwrap(), 2);
        let removed = doc.field("effects/relaxed").unwrap();
        assert_eq!(removed.value, Value::Null);
        assert_eq!(removed.stamp, stamp(2, 1));
        assert_eq!(doc.value("rating"), &Value::Null);
        assert_eq!(Session::from_document(&doc).unwrap(), session);
    }

    #[test]
    fn timeline_plans_follow_the_method() {
        assert_eq!(Method::Dabbed.timeline_plan().point_minutes(), [20, 40, 60]);
        assert_eq!(Method::Dabbed.timeline_plan().gesture_points, 1);
        assert_eq!(
            Method::Smoked.timeline_plan().point_minutes(),
            [30, 60, 90, 120]
        );
        assert_eq!(
            Method::Tincture.timeline_plan().point_minutes(),
            [48, 96, 144, 192, 240]
        );
        assert_eq!(
            Method::Topical.timeline_plan().point_minutes(),
            [40, 80, 120]
        );
        // A patch's points are uneven.
        let patch = Method::Transdermal.timeline_plan();
        assert_eq!(patch.point_minutes(), [60, 120, 240, 480, 720]);
        assert_eq!(
            (patch.minutes, patch.points, patch.gesture_points),
            (720, 5, 2)
        );
        // Every plan has as many points as it says, ascending, the last at
        // the end.
        for slug in Method::SLUGS {
            let plan = Method::from_slug(slug).timeline_plan();
            let points = plan.point_minutes();
            assert_eq!(points.len(), plan.points as usize, "{slug}");
            assert!(points.windows(2).all(|w| w[0] < w[1]), "{slug}");
            assert_eq!(points.last(), Some(&plan.minutes), "{slug}");
        }
        // A session takes its longest method's plan; with none, smoked's.
        let mut s = Session::new(LocalTime {
            unix_ms: 0,
            utc_offset_min: 0,
        });
        assert_eq!(s.timeline_plan(), Method::Smoked.timeline_plan());
        for method in [Method::Dabbed, Method::Beverage, Method::Smoked] {
            s.lines.insert(
                RecordId::random().unwrap(),
                ProductLine {
                    product: None,
                    method,
                    dose: None,
                    purchase: None,
                    shop: None,
                },
            );
        }
        assert_eq!(s.timeline_plan(), Method::Beverage.timeline_plan());
        // The longest plan wins: a patch's.
        s.lines.insert(
            RecordId::random().unwrap(),
            ProductLine {
                product: None,
                method: Method::Transdermal,
                dose: None,
                purchase: None,
                shop: None,
            },
        );
        assert_eq!(s.timeline_plan(), Method::Transdermal.timeline_plan());
    }

    #[test]
    fn check_ins_follow_the_slowest_method() {
        assert_eq!(Method::Edible.check_in_minutes(), Some(90));
        assert_eq!(Method::Beverage.check_in_minutes(), Some(90));
        assert_eq!(Method::Tincture.check_in_minutes(), Some(45));
        assert_eq!(Method::Transdermal.check_in_minutes(), Some(120));
        for method in [
            Method::Smoked,
            Method::VapedFlower,
            Method::VapeCart,
            Method::Dabbed,
            Method::Topical,
            Method::from_slug("nebulized"),
        ] {
            assert_eq!(method.check_in_minutes(), None);
        }
        assert_eq!(
            check_in_minutes_for(&[Method::Tincture, Method::Edible, Method::Smoked]),
            Some(90)
        );
        assert_eq!(
            check_in_minutes_for(&[Method::Edible, Method::Transdermal]),
            Some(120)
        );
        assert_eq!(
            check_in_minutes_for(&[Method::Smoked, Method::Topical]),
            None
        );
        assert_eq!(check_in_minutes_for(&[]), None);
    }

    #[test]
    fn timeline_levels_are_one_field_per_point() {
        let mut session = full_session();
        let mut doc = Document::create(&session, stamp(1, 1)).unwrap();
        assert_eq!(doc.value("timeline_levels/30"), &Value::Int(2));
        session.timeline_levels.insert(90, 1);
        assert_eq!(doc.update(&session, stamp(2, 1)).unwrap(), 1);
        assert_eq!(Session::from_document(&doc).unwrap(), session);
        // A level above 5 is refused.
        session.timeline_levels.insert(120, 6);
        doc.update(&session, stamp(3, 1)).unwrap();
        assert!(Session::from_document(&doc).is_err());
    }

    #[test]
    fn effects_are_one_field_per_entry() {
        let doc = Document::create(&full_session(), stamp(1, 1)).unwrap();
        assert_eq!(doc.value("effects/relaxed"), &Value::Int(4));
        assert!(
            doc.field(&format!("effects/custom:{}", id(9).to_hex()))
                .is_some()
        );
    }

    #[test]
    fn out_of_range_values_are_rejected() {
        let mut doc = Document::create(&full_session(), stamp(1, 1)).unwrap();
        doc.set("rating", Value::Int(6), stamp(2, 1));
        assert_eq!(
            Session::from_document(&doc).err(),
            Some(Error::InvalidField("rating"))
        );
        let mut doc = Document::create(&full_session(), stamp(1, 1)).unwrap();
        doc.set("effects/relaxed", Value::Int(9), stamp(2, 1));
        assert_eq!(
            Session::from_document(&doc).err(),
            Some(Error::InvalidField("effects"))
        );
    }

    #[test]
    fn missing_when_is_rejected() {
        let doc = Document::new(id(1), RecordKind::Session);
        assert_eq!(
            Session::from_document(&doc).err(),
            Some(Error::InvalidField("when"))
        );
    }

    #[test]
    fn debug_is_redacted() {
        assert_eq!(format!("{:?}", full_session()), "Session(<redacted>)");
    }
}

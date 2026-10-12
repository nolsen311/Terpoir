// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

//! Saved spots: places described by purpose, never by relationship.

use std::collections::{BTreeMap, BTreeSet};

use super::{Fields, Reader, Record, RecordKind, TagRef, owns};
use crate::{Document, Error, Result, Value};

/// Built-in spot purposes. Custom purposes are [`TagRef::Custom`]. There
/// are deliberately no relational labels (home, work, a friend's place).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[non_exhaustive]
pub enum SpotPurpose {
    /// Where you consume.
    Blaze,
    /// Where you eat after.
    Munch,
    /// Where you buy.
    ReUp,
    /// Scenic or outdoor places worth returning to.
    Chill,
}

impl SpotPurpose {
    /// The stored slug.
    #[must_use]
    pub const fn slug(self) -> &'static str {
        match self {
            Self::Blaze => "blaze",
            Self::Munch => "munch",
            Self::ReUp => "re_up",
            Self::Chill => "chill",
        }
    }
}

impl From<SpotPurpose> for TagRef {
    fn from(purpose: SpotPurpose) -> Self {
        Self::Builtin(purpose.slug().to_owned())
    }
}

/// Coordinates, only ever stored inside encrypted records.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Location {
    /// Latitude in units of 1e-7 degrees.
    pub lat_e7: i32,
    /// Longitude in units of 1e-7 degrees.
    pub lon_e7: i32,
    /// Accuracy radius in metres. Coarse locations have large values.
    pub accuracy_m: u32,
}

/// Mean Earth radius (IUGG), metres.
const EARTH_RADIUS_M: f64 = 6_371_008.8;

/// The radius a spot uses when it has none set, and the least any spot
/// uses, in metres, so "near" never shrinks to a doorstep (design doc,
/// "Location and saved spots": a minimum radius).
pub const DEFAULT_SPOT_RADIUS_M: u32 = 150;
/// The smallest radius a spot may use.
pub const MIN_SPOT_RADIUS_M: u32 = 100;

impl Location {
    /// Great-circle distance in metres (haversine; ample for spots).
    #[must_use]
    pub fn distance_m(&self, other: &Self) -> f64 {
        let rad = |e7: i32| f64::from(e7) * 1e-7_f64.to_radians();
        let (lat1, lat2) = (rad(self.lat_e7), rad(other.lat_e7));
        let dlat = lat2 - lat1;
        let dlon = rad(other.lon_e7) - rad(self.lon_e7);
        let h = (dlat / 2.0).sin().powi(2) + lat1.cos() * lat2.cos() * (dlon / 2.0).sin().powi(2);
        2.0 * EARTH_RADIUS_M * h.sqrt().min(1.0).asin()
    }
}

impl Spot {
    /// The radius used to decide "near": the spot's own, at least
    /// [`MIN_SPOT_RADIUS_M`], or [`DEFAULT_SPOT_RADIUS_M`].
    #[must_use]
    pub fn effective_radius_m(&self) -> u32 {
        self.radius_m
            .map_or(DEFAULT_SPOT_RADIUS_M, |r| r.max(MIN_SPOT_RADIUS_M))
    }

    /// Whether it is a Re-up Spot (where the user buys).
    #[must_use]
    pub fn is_re_up(&self) -> bool {
        self.purposes.contains(&SpotPurpose::ReUp.into())
    }

    /// Whether leaving it should prompt "Quick log?": the user opted in on
    /// a Re-up Spot that has coordinates.
    #[must_use]
    pub fn prompts_purchases(&self) -> bool {
        self.purchase_prompts && self.location.is_some() && self.is_re_up()
    }

    /// The distance to `here` in metres when `here` is near this spot: within
    /// its radius, plus the two fixes' accuracy capped at that radius, so a
    /// coarse fix never makes far-away spots "near". `None` otherwise, or
    /// when the spot has no coordinates.
    #[must_use]
    pub fn nearness_m(&self, here: &Location) -> Option<f64> {
        let spot = self.location?;
        let radius = f64::from(self.effective_radius_m());
        let slack = (f64::from(spot.accuracy_m) + f64::from(here.accuracy_m)).min(radius);
        let d = spot.distance_m(here);
        (d <= radius + slack).then_some(d)
    }
}

const LAT_MAX: i64 = 900_000_000;
const LON_MAX: i64 = 1_800_000_000;

impl Location {
    fn to_value(self) -> Value {
        Value::Tuple(vec![
            Value::Int(i64::from(self.lat_e7)),
            Value::Int(i64::from(self.lon_e7)),
            Value::Int(i64::from(self.accuracy_m)),
        ])
    }

    fn from_value(value: &Value) -> Result<Option<Self>> {
        let invalid = Error::InvalidField("location");
        match value {
            Value::Null => Ok(None),
            Value::Tuple(items) => match items.as_slice() {
                [Value::Int(lat), Value::Int(lon), Value::Int(acc)]
                    if (-LAT_MAX..=LAT_MAX).contains(lat) && (-LON_MAX..=LON_MAX).contains(lon) =>
                {
                    Ok(Some(Self {
                        lat_e7: i32::try_from(*lat).map_err(|_| invalid)?,
                        lon_e7: i32::try_from(*lon).map_err(|_| invalid)?,
                        accuracy_m: u32::try_from(*acc).map_err(|_| invalid)?,
                    }))
                }
                _ => Err(invalid),
            },
            _ => Err(invalid),
        }
    }
}

/// A saved spot.
#[derive(Clone, PartialEq, Eq)]
pub struct Spot {
    /// User-chosen name ("Spot 7" by default; never reverse-geocoded
    /// without opt-in).
    pub name: String,
    /// What the spot is for.
    pub purposes: BTreeSet<TagRef>,
    /// Where it is, if recorded.
    pub location: Option<Location>,
    /// Geofence radius in metres for "Notify me", if set.
    pub radius_m: Option<u32>,
    /// "Remind me to log purchases here" (Re-up Spots with coordinates).
    pub purchase_prompts: bool,
}

impl Record for Spot {
    const KIND: RecordKind = RecordKind::Spot;

    fn to_fields(&self) -> BTreeMap<String, Value> {
        let mut f = Fields::default();
        f.put("name", Value::Text(self.name.clone()))
            .tag_set("purposes", &self.purposes)
            .opt("location", self.location, Location::to_value)
            .opt("radius_m", self.radius_m, |r| Value::Int(i64::from(r)))
            // Off is the default and is stored as absent.
            .opt(
                "purchase_prompts",
                self.purchase_prompts.then_some(true),
                Value::Bool,
            );
        f.finish()
    }

    fn from_document(doc: &Document) -> Result<Self> {
        let r = Reader::new::<Self>(doc)?;
        Ok(Self {
            name: r.required_text("name")?,
            purposes: r.tag_set("purposes")?,
            location: Location::from_value(r.value("location"))?,
            radius_m: r.int_in("radius_m", 0..=u32::MAX)?,
            purchase_prompts: r.bool("purchase_prompts")?.unwrap_or(false),
        })
    }

    fn owns_field(key: &str) -> bool {
        owns(
            key,
            &["name", "location", "radius_m", "purchase_prompts"],
            &["purposes"],
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DeviceId, Hlc, Stamp};

    fn stamp() -> Stamp {
        Stamp {
            hlc: Hlc::default(),
            device: DeviceId::from_bytes([1; 16]),
        }
    }

    #[test]
    fn round_trip() {
        let spot = Spot {
            name: "Spot 7".into(),
            purposes: [SpotPurpose::Chill.into(), SpotPurpose::Munch.into()].into(),
            location: Some(Location {
                lat_e7: -337_000_000,
                lon_e7: 1_512_000_000,
                accuracy_m: 1_000,
            }),
            radius_m: Some(150),
            purchase_prompts: true,
        };
        let doc = Document::create(&spot, stamp()).unwrap();
        assert_eq!(Spot::from_document(&doc).unwrap(), spot);
    }

    #[test]
    fn only_opted_in_re_up_spots_with_coordinates_prompt() {
        let mut spot = Spot {
            name: "Spot 2".into(),
            purposes: [SpotPurpose::ReUp.into()].into(),
            location: Some(at(1, 1, 10)),
            radius_m: None,
            purchase_prompts: true,
        };
        assert!(spot.is_re_up() && spot.prompts_purchases());
        spot.location = None;
        assert!(!spot.prompts_purchases());
        spot.location = Some(at(1, 1, 10));
        spot.purposes = [SpotPurpose::Chill.into()].into();
        assert!(!spot.is_re_up() && !spot.prompts_purchases());
        spot.purposes = [SpotPurpose::ReUp.into()].into();
        spot.purchase_prompts = false;
        assert!(!spot.prompts_purchases());
    }

    fn at(lat_e7: i32, lon_e7: i32, accuracy_m: u32) -> Location {
        Location {
            lat_e7,
            lon_e7,
            accuracy_m,
        }
    }

    #[test]
    fn distances_are_great_circle_metres() {
        // One degree of latitude is about 111.2 km.
        let d = at(0, 0, 0).distance_m(&at(10_000_000, 0, 0));
        assert!((d - 111_195.0).abs() < 50.0, "{d}");
        // 0.001° of latitude is about 111 m.
        let d = at(377_000_000, -1_224_000_000, 0).distance_m(&at(377_010_000, -1_224_000_000, 0));
        assert!((d - 111.2).abs() < 1.0, "{d}");
        assert!(at(5, 5, 0).distance_m(&at(5, 5, 0)).abs() < 1e-6);
    }

    #[test]
    fn nearness_uses_the_radius_and_capped_accuracy() {
        let mut spot = Spot {
            name: "x".into(),
            purposes: BTreeSet::new(),
            location: Some(at(377_000_000, -1_224_000_000, 10)),
            radius_m: None,
            purchase_prompts: false,
        };
        // ~111 m away: inside the default 150 m.
        assert!(
            spot.nearness_m(&at(377_010_000, -1_224_000_000, 10))
                .is_some()
        );
        // ~334 m away: outside 150 m + (20 m of accuracy).
        assert!(
            spot.nearness_m(&at(377_030_000, -1_224_000_000, 10))
                .is_none()
        );
        // A coarse 2 km fix only stretches "near" by the radius (to 300 m).
        assert!(
            spot.nearness_m(&at(377_030_000, -1_224_000_000, 2_000))
                .is_none()
        );
        assert!(
            spot.nearness_m(&at(377_025_000, -1_224_000_000, 2_000))
                .is_some()
        );
        // A tiny radius is raised to the minimum.
        spot.radius_m = Some(10);
        assert_eq!(spot.effective_radius_m(), MIN_SPOT_RADIUS_M);
        // No coordinates: never near.
        spot.location = None;
        assert!(
            spot.nearness_m(&at(377_000_000, -1_224_000_000, 10))
                .is_none()
        );
    }

    #[test]
    fn rejects_impossible_coordinates() {
        let spot = Spot {
            name: "x".into(),
            purposes: BTreeSet::new(),
            location: None,
            radius_m: None,
            purchase_prompts: false,
        };
        let mut doc = Document::create(&spot, stamp()).unwrap();
        doc.set(
            "location",
            Value::Tuple(vec![Value::Int(LAT_MAX + 1), Value::Int(0), Value::Int(1)]),
            stamp(),
        );
        assert_eq!(
            Spot::from_document(&doc).err(),
            Some(Error::InvalidField("location"))
        );
    }
}

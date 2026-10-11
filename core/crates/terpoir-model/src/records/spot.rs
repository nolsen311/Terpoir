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
}

impl Record for Spot {
    const KIND: RecordKind = RecordKind::Spot;

    fn to_fields(&self) -> BTreeMap<String, Value> {
        let mut f = Fields::default();
        f.put("name", Value::Text(self.name.clone()))
            .tag_set("purposes", &self.purposes)
            .opt("location", self.location, Location::to_value)
            .opt("radius_m", self.radius_m, |r| Value::Int(i64::from(r)));
        f.finish()
    }

    fn from_document(doc: &Document) -> Result<Self> {
        let r = Reader::new::<Self>(doc)?;
        Ok(Self {
            name: r.required_text("name")?,
            purposes: r.tag_set("purposes")?,
            location: Location::from_value(r.value("location"))?,
            radius_m: r.int_in("radius_m", 0..=u32::MAX)?,
        })
    }

    fn owns_field(key: &str) -> bool {
        owns(key, &["name", "location", "radius_m"], &["purposes"])
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
        };
        let doc = Document::create(&spot, stamp()).unwrap();
        assert_eq!(Spot::from_document(&doc).unwrap(), spot);
    }

    #[test]
    fn rejects_impossible_coordinates() {
        let spot = Spot {
            name: "x".into(),
            purposes: BTreeSet::new(),
            location: None,
            radius_m: None,
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

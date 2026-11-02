// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

//! Address search for spots (docs/design-decisions.md, "Address search"):
//! reads the results of an OpenStreetMap search (Photon's GeoJSON) into
//! places the user can pick. The app sends the search and hands the reply
//! here; nothing is stored but the coordinates of the place picked.

use serde_json::Value as Json;

/// The most places one search shows.
const MAX_PLACES: usize = 10;

/// The narrowest and widest accuracy given to a place, in metres.
const MIN_ACCURACY_M: u32 = 10;
const MAX_ACCURACY_M: u32 = 50_000;

/// Accuracy for a point with no outline: an address or a business, or an
/// area such as a town.
const POINT_ACCURACY_M: u32 = 25;
const AREA_ACCURACY_M: u32 = 2_000;

/// Metres per degree of latitude.
const M_PER_DEGREE: f64 = 111_320.0;

/// A search result.
#[derive(Clone, PartialEq, Eq, Debug, uniffi::Record)]
pub struct PlaceData {
    /// What it's called: a business's name, or the street address.
    pub name: String,
    /// Where it is, for telling results apart (street, town, region).
    pub detail: String,
    /// Latitude in units of 1e-7 degrees.
    pub lat_e7: i32,
    /// Longitude in units of 1e-7 degrees.
    pub lon_e7: i32,
    /// How far from the point the place may reach, in metres.
    pub accuracy_m: u32,
}

/// The places in a Photon search reply, in its order; empty when the reply
/// isn't one.
#[uniffi::export]
#[must_use]
pub fn parse_place_search(json: String) -> Vec<PlaceData> {
    let Ok(reply) = serde_json::from_str::<Json>(&json) else {
        return Vec::new();
    };
    let Some(features) = reply.get("features").and_then(Json::as_array) else {
        return Vec::new();
    };
    let mut places: Vec<PlaceData> = Vec::new();
    // OpenStreetMap can hold the same place twice (a node and a building).
    for found in features.iter().filter_map(place) {
        if !places.contains(&found) {
            places.push(found);
        }
    }
    places.truncate(MAX_PLACES);
    places
}

fn place(feature: &Json) -> Option<PlaceData> {
    let coordinates = feature.pointer("/geometry/coordinates")?.as_array()?;
    let lon = coordinates.first()?.as_f64()?;
    let lat = coordinates.get(1)?.as_f64()?;
    if !(-90.0..=90.0).contains(&lat) || !(-180.0..=180.0).contains(&lon) {
        return None;
    }
    let props = feature.get("properties")?;
    let text = |key: &str| {
        props
            .get(key)
            .and_then(Json::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
    };
    let street = match (text("housenumber"), text("street")) {
        (Some(number), Some(street)) => Some(format!("{number} {street}")),
        (None, Some(street)) => Some(street.to_owned()),
        _ => None,
    };
    let name = text("name").map(str::to_owned).or_else(|| street.clone())?;
    let mut detail: Vec<String> = Vec::new();
    for part in [
        street,
        text("city").or_else(|| text("district")).map(str::to_owned),
        text("state").map(str::to_owned),
        text("country").map(str::to_owned),
    ]
    .into_iter()
    .flatten()
    {
        if part != name && !detail.contains(&part) {
            detail.push(part);
        }
    }
    let accuracy_m = props
        .get("extent")
        .and_then(Json::as_array)
        .and_then(|e| extent_radius_m(e, lat))
        .unwrap_or_else(|| {
            if text("housenumber").is_some()
                || !matches!(text("osm_key"), Some("place" | "boundary"))
            {
                POINT_ACCURACY_M
            } else {
                AREA_ACCURACY_M
            }
        })
        .clamp(MIN_ACCURACY_M, MAX_ACCURACY_M);
    Some(PlaceData {
        name,
        detail: detail.join(", "),
        lat_e7: to_e7(lat),
        lon_e7: to_e7(lon),
        accuracy_m,
    })
}

/// Half the diagonal of an outline `[west, north, east, south]`, in metres.
fn extent_radius_m(extent: &[Json], lat: f64) -> Option<u32> {
    let [west, north, east, south] = [0, 1, 2, 3].map(|i| extent.get(i).and_then(Json::as_f64));
    let (west, north, east, south) = (west?, north?, east?, south?);
    let height = (north - south).abs() * M_PER_DEGREE;
    let width = (east - west).abs() * M_PER_DEGREE * lat.to_radians().cos();
    let radius = (height.hypot(width) / 2.0).round();
    // Bounded by the clamp in `place`, so the cast cannot truncate.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    Some(radius.min(f64::from(MAX_ACCURACY_M)) as u32)
}

fn to_e7(degrees: f64) -> i32 {
    // Within ±180°, so within i32 once scaled.
    #[allow(clippy::cast_possible_truncation)]
    let e7 = (degrees * 1e7).round() as i32;
    e7
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reply(features: &str) -> String {
        format!(r#"{{"type":"FeatureCollection","features":[{features}]}}"#)
    }

    #[test]
    fn a_business_has_its_name_and_address() {
        let json = reply(
            r#"{"type":"Feature","properties":{"osm_key":"shop","osm_value":"mall","housenumber":"1450",
            "name":"Ala Moana Center","street":"Ala Moana Boulevard","district":"Ala Moana","city":"Honolulu",
            "state":"Hawaii","country":"United States","extent":[-157.8475966,21.2936742,-157.8399794,21.2892927]},
            "geometry":{"type":"Point","coordinates":[-157.8437756,21.2914679]}}"#,
        );
        let places = parse_place_search(json);
        assert_eq!(places.len(), 1);
        let p = &places[0];
        assert_eq!(p.name, "Ala Moana Center");
        assert_eq!(
            p.detail,
            "1450 Ala Moana Boulevard, Honolulu, Hawaii, United States"
        );
        assert_eq!((p.lat_e7, p.lon_e7), (212_914_679, -1_578_437_756));
        // The mall is about 790 m by 490 m across.
        assert!((400..500).contains(&p.accuracy_m), "{}", p.accuracy_m);
    }

    #[test]
    fn an_address_without_a_name_is_named_by_its_street() {
        let json = reply(
            r#"{"properties":{"osm_key":"building","housenumber":"12","street":"Main Street","city":"Hilo"},
            "geometry":{"coordinates":[-155.08,19.72]}}"#,
        );
        let p = &parse_place_search(json)[0];
        assert_eq!(p.name, "12 Main Street");
        assert_eq!(p.detail, "Hilo");
        assert_eq!(p.accuracy_m, POINT_ACCURACY_M);
    }

    #[test]
    fn a_town_without_an_outline_is_vague() {
        let json = reply(
            r#"{"properties":{"osm_key":"place","name":"Kona"},"geometry":{"coordinates":[-155.99,19.64]}}"#,
        );
        assert_eq!(parse_place_search(json)[0].accuracy_m, AREA_ACCURACY_M);
    }

    #[test]
    fn a_tiny_outline_keeps_the_minimum_and_a_huge_one_the_maximum() {
        let tiny = reply(
            r#"{"properties":{"name":"Kiosk","extent":[0.0,0.00001,0.00001,0.0]},"geometry":{"coordinates":[0.000005,0.000005]}}"#,
        );
        assert_eq!(parse_place_search(tiny)[0].accuracy_m, MIN_ACCURACY_M);
        let huge = reply(
            r#"{"properties":{"name":"Hawaii","extent":[-178.4,28.5,-154.7,18.9]},"geometry":{"coordinates":[-157.0,20.5]}}"#,
        );
        assert_eq!(parse_place_search(huge)[0].accuracy_m, MAX_ACCURACY_M);
    }

    #[test]
    fn results_without_a_name_or_point_are_skipped() {
        let json = reply(
            r#"{"properties":{"osm_key":"highway"},"geometry":{"coordinates":[1.0,2.0]}},
            {"properties":{"name":"No point"},"geometry":{}},
            {"properties":{"name":"Off the map"},"geometry":{"coordinates":[200.0,95.0]}},
            {"properties":{"name":"Kept"},"geometry":{"coordinates":[1.0,2.0]}}"#,
        );
        let names: Vec<String> = parse_place_search(json)
            .into_iter()
            .map(|p| p.name)
            .collect();
        assert_eq!(names, ["Kept"]);
    }

    #[test]
    fn the_same_place_twice_shows_once() {
        let one =
            r#"{"properties":{"name":"Shop","city":"Hilo"},"geometry":{"coordinates":[1.0,2.0]}}"#;
        let elsewhere =
            r#"{"properties":{"name":"Shop","city":"Kona"},"geometry":{"coordinates":[3.0,4.0]}}"#;
        let json = reply(&[one, one, elsewhere].join(","));
        let details: Vec<String> = parse_place_search(json)
            .into_iter()
            .map(|p| p.detail)
            .collect();
        assert_eq!(details, ["Hilo", "Kona"]);
    }

    #[test]
    fn at_most_ten_places() {
        let json = reply(
            &(0..15)
                .map(|i| format!(r#"{{"properties":{{"name":"Shop {i}"}},"geometry":{{"coordinates":[1.0,2.0]}}}}"#))
                .collect::<Vec<_>>()
                .join(","),
        );
        assert_eq!(parse_place_search(json).len(), MAX_PLACES);
    }

    #[test]
    fn anything_else_gives_no_places() {
        for json in [
            "",
            "not json",
            "[]",
            r#"{"features":"no"}"#,
            r#"{"message":"rate limited"}"#,
        ] {
            assert!(parse_place_search(json.to_owned()).is_empty(), "{json}");
        }
    }
}

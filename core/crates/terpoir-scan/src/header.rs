// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

//! The receipt's header: which shop it is from.
//!
//! The header is the lines above the first priced line (at most
//! [`HEADER_LINES`]). It is read for the shop's name, phone numbers and
//! licence numbers, and matched against the user's saved shops
//! (docs/design-decisions.md, "Simpler review"): a phone or licence
//! number the shop is known by is a strong match, a name only a guess.
//! Location is never used.

use crate::matching::score_name;
use crate::name::names_match;
use crate::number::trailing_price;
use crate::receipt::looks_like_shop;

/// The most lines read as the header.
pub const HEADER_LINES: usize = 10;

/// What the header says about the shop.
#[derive(Clone, PartialEq, Eq, Default)]
pub struct ShopHeader {
    /// The line that looks most like the shop's name.
    pub name: Option<String>,
    /// The line it was read from.
    pub name_line: Option<usize>,
    /// Phone numbers, digits only, without a leading country code `1` on
    /// an 11-digit number.
    pub phones: Vec<String>,
    /// Licence numbers, upper-case letters and digits only.
    pub licences: Vec<String>,
}

/// A saved shop to match against.
#[derive(Clone, PartialEq, Eq)]
pub struct ShopCandidate {
    /// Its name.
    pub name: String,
    /// Phone numbers it is known by (normalised like [`ShopHeader::phones`]).
    pub phones: Vec<String>,
    /// Licence numbers it is known by (normalised like
    /// [`ShopHeader::licences`]).
    pub licences: Vec<String>,
}

/// How a shop was recognised.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ShopMatchKind {
    /// By a phone number: strong.
    Phone,
    /// By a licence number: strong.
    Licence,
    /// By name only: a guess until the user confirms it.
    Name,
}

impl ShopMatchKind {
    /// `phone`, `licence` or `name`.
    #[must_use]
    pub const fn slug(self) -> &'static str {
        match self {
            Self::Phone => "phone",
            Self::Licence => "licence",
            Self::Name => "name",
        }
    }

    /// Whether the match is strong enough to show as recognised ("✓")
    /// rather than guessed ("?").
    #[must_use]
    pub const fn is_strong(self) -> bool {
        matches!(self, Self::Phone | Self::Licence)
    }
}

/// The saved shop a header most likely belongs to.
#[derive(Clone, PartialEq, Eq)]
pub struct ShopMatch {
    /// Its index in the candidates.
    pub index: usize,
    /// How it was recognised.
    pub kind: ShopMatchKind,
    /// The header line that matched.
    pub line: usize,
}

const PHONE_WORDS: &[&str] = &["tel", "phone", "ph", "call", "fax"];
const LICENCE_WORDS: &[&str] = &["lic", "license", "licence", "permit"];

fn has_word(lower: &str, words: &[&str]) -> bool {
    lower
        .split(|c: char| !c.is_alphanumeric())
        .any(|w| words.contains(&w))
}

/// Phone numbers in a line: digit runs joined by spaces, dots, dashes and
/// brackets, 10 or 11 digits (or 7 after a word such as `Tel`).
pub(crate) fn phones_in(line: &str) -> Vec<String> {
    let lower = line.to_lowercase();
    if has_word(&lower, LICENCE_WORDS) {
        return Vec::new();
    }
    let labelled = has_word(&lower, PHONE_WORDS);
    let mut out = Vec::new();
    let mut digits = String::new();
    let mut flush = |digits: &mut String| {
        let number = match digits.len() {
            11 if digits.starts_with('1') => Some(digits[1..].to_owned()),
            10 => Some(digits.clone()),
            7 if labelled => Some(digits.clone()),
            _ => None,
        };
        out.extend(number);
        digits.clear();
    };
    for c in line.chars() {
        if c.is_ascii_digit() {
            digits.push(c);
        } else if !(matches!(c, ' ' | '.' | '-' | '(' | ')' | '+') || digits.is_empty() && c == ':')
        {
            flush(&mut digits);
        }
    }
    flush(&mut digits);
    out
}

/// Licence numbers in a line: after a word such as `Lic` or `License`,
/// the words with at least three digits; elsewhere, words mixing letters
/// and at least five digits with a dash (`C10-0000123-LIC`).
pub(crate) fn licences_in(line: &str) -> Vec<String> {
    let lower = line.to_lowercase();
    let labelled = has_word(&lower, LICENCE_WORDS);
    line.split_whitespace()
        .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()))
        .filter(|w| {
            let digits = w.chars().filter(char::is_ascii_digit).count();
            let letters = w.chars().filter(char::is_ascii_alphabetic).count();
            if labelled {
                digits >= 3
            } else {
                digits >= 5 && letters >= 1 && w.contains('-')
            }
        })
        .map(|w| {
            w.chars()
                .filter(char::is_ascii_alphanumeric)
                .flat_map(char::to_uppercase)
                .collect::<String>()
        })
        .filter(|w| w.len() >= 4)
        .collect()
}

/// The header's lines: those before the first priced line, at most
/// [`HEADER_LINES`].
fn header_lines<'a>(lines: &[&'a str]) -> impl Iterator<Item = (usize, &'a str)> {
    lines
        .iter()
        .copied()
        .enumerate()
        .take_while(|(_, l)| trailing_price(&l.chars().collect::<Vec<_>>()).is_none())
        .take(HEADER_LINES)
}

/// Reads the shop's name, phone and licence numbers from a receipt's top
/// lines.
#[must_use]
pub fn read_header(lines: &[&str]) -> ShopHeader {
    let mut header = ShopHeader::default();
    for (i, line) in header_lines(lines) {
        let line = line.trim();
        for phone in phones_in(line) {
            if !header.phones.contains(&phone) {
                header.phones.push(phone);
            }
        }
        for licence in licences_in(line) {
            if !header.licences.contains(&licence) {
                header.licences.push(licence);
            }
        }
        if header.name.is_none() && i < 4 && looks_like_shop(line) {
            header.name = Some(line.to_owned());
            header.name_line = Some(i);
        }
    }
    header
}

/// The saved shop a receipt's header belongs to: one whose phone or
/// licence number is printed there, else one whose name is (the same name
/// first, then one containing the other, earlier lines first).
#[must_use]
pub fn match_shop(lines: &[&str], candidates: &[ShopCandidate]) -> Option<ShopMatch> {
    let header: Vec<(usize, &str)> = header_lines(lines).collect();
    for &(line, text) in &header {
        let (phones, licences) = (phones_in(text), licences_in(text));
        for (index, c) in candidates.iter().enumerate() {
            if c.phones.iter().any(|p| phones.contains(p)) {
                return Some(ShopMatch {
                    index,
                    kind: ShopMatchKind::Phone,
                    line,
                });
            }
            if c.licences.iter().any(|l| licences.contains(l)) {
                return Some(ShopMatch {
                    index,
                    kind: ShopMatchKind::Licence,
                    line,
                });
            }
        }
    }
    let mut best: Option<(u8, ShopMatch)> = None;
    for &(line, text) in &header {
        for (index, c) in candidates.iter().enumerate() {
            let rank = match score_name(text, &c.name) {
                Some((crate::MatchKind::Exact, _)) => 2,
                _ if names_match(text, &c.name) => 1,
                _ => continue,
            };
            if best.as_ref().is_none_or(|(r, _)| rank > *r) {
                best = Some((
                    rank,
                    ShopMatch {
                        index,
                        kind: ShopMatchKind::Name,
                        line,
                    },
                ));
            }
        }
    }
    best.map(|(_, m)| m)
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEADER: &[&str] = &[
        "GREEN LEAF",
        "DISPENSARY",
        "1234 Main St, Springfield",
        "Tel (555) 010-2030",
        "Lic# C10-0000123-LIC",
        "10/11/2026 2:02 PM",
        "BD 3.5G HYB 35.00",
        "Call 555 999 0000 for deals 1.00",
    ];

    fn shop(name: &str, phones: &[&str], licences: &[&str]) -> ShopCandidate {
        ShopCandidate {
            name: name.into(),
            phones: phones.iter().map(|&p| p.into()).collect(),
            licences: licences.iter().map(|&l| l.into()).collect(),
        }
    }

    #[test]
    fn reads_the_header() {
        let h = read_header(HEADER);
        assert_eq!(h.name.as_deref(), Some("GREEN LEAF"));
        assert_eq!(h.name_line, Some(0));
        assert_eq!(h.phones, ["5550102030"]);
        assert_eq!(h.licences, ["C100000123LIC"]);
    }

    #[test]
    fn reads_phone_numbers() {
        assert_eq!(phones_in("+1 (555) 010-2030"), ["5550102030"]);
        assert_eq!(phones_in("555.010.2030"), ["5550102030"]);
        assert_eq!(phones_in("Phone: 010-2030"), ["0102030"]);
        // Seven digits alone, dates and licences are not phone numbers.
        assert!(phones_in("010-2030").is_empty());
        assert!(phones_in("2026-10-11").is_empty());
        assert!(phones_in("License 4021234567").is_empty());
    }

    #[test]
    fn reads_licence_numbers() {
        assert_eq!(
            licences_in("License: OCM-AUCP-23-000123"),
            ["OCMAUCP23000123"]
        );
        assert_eq!(licences_in("LIC # 402R-00456"), ["402R00456"]);
        assert_eq!(licences_in("MED C10-0000123-LIC"), ["C100000123LIC"]);
        assert!(licences_in("1234 Main St").is_empty());
    }

    #[test]
    fn a_known_phone_number_is_a_strong_match() {
        let shops = [
            shop("Green Leaf", &[], &[]),
            shop("GL Downtown", &["5550102030"], &[]),
        ];
        let m = match_shop(HEADER, &shops).unwrap();
        assert_eq!((m.index, m.kind, m.line), (1, ShopMatchKind::Phone, 3));
        assert!(m.kind.is_strong());
        let shops = [shop("Other", &[], &["C100000123LIC"])];
        let m = match_shop(HEADER, &shops).unwrap();
        assert_eq!((m.index, m.kind, m.line), (0, ShopMatchKind::Licence, 4));
        // A number printed below the header doesn't count.
        let shops = [shop("Deals", &["5559990000"], &[])];
        assert!(match_shop(HEADER, &shops).is_none());
    }

    #[test]
    fn a_name_is_a_guess() {
        let shops = [
            shop("Leaf", &[], &[]),
            shop("Green Leaf Dispensary", &[], &[]),
            shop("Green Leaf", &[], &[]),
        ];
        let m = match_shop(HEADER, &shops).unwrap();
        assert_eq!((m.index, m.kind, m.line), (2, ShopMatchKind::Name, 0));
        assert!(!m.kind.is_strong());
        // Recognition confusions are tolerated.
        let m = match_shop(&["GREEN LEAF", "8LUE SKY"], &[shop("Blue Sky", &[], &[])]).unwrap();
        assert_eq!(m.line, 1);
        // Containment, when nothing is the same.
        let m = match_shop(HEADER, &shops[1..2]).unwrap();
        assert_eq!((m.index, m.line), (0, 0));
        assert!(match_shop(HEADER, &[shop("Elsewhere", &[], &[])]).is_none());
        assert!(match_shop(&[], &shops).is_none());
    }
}

// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

//! Item breakdown: the built-in receipt and label vocabulary, and
//! [`describe_item`], which takes a line's size, form, strain type, pack
//! count and quantity out of its text and leaves the name.
//!
//! The vocabulary is fixed and lives here, like the terpene list
//! (docs/design-decisions.md, "Simpler review"). Words are compared in
//! upper case and, for terms, after folding the usual recognition
//! confusions (`0/O`, `1/I/l`, `5/S`, `8/B`), so `HY8` reads as `HYB`.
//!
//! | Kind | Printed as | Becomes |
//! |---|---|---|
//! | Form | `PR`, `PREROLL`, `PRE-ROLL` | pre-roll |
//! | Form | `VAPE` | vape cart |
//! | Form | `FLWR`, `FLOWER`, `BUD`, `SMALLS`, `SHAKE` | flower |
//! | Form | `EDIBLE`, `CHEWS` | edible |
//! | Form | `CONC`, `RESIN` | concentrate |
//! | Form | `TINCT`, `TINCTURE`, `DROPS` | tincture |
//! | Form | `TOPICAL`, `SALVE` | topical |
//! | Form | `DRINK`, `BEV`, `BEVERAGE` | beverage |
//! | Subtype | `JOINT`; `BLUNT`; `INFUSED PR`, `INFUSED PRE ROLL` | joint, blunt, infused pre-roll (pre-roll) |
//! | Subtype | `CART`, `CARTRIDGE`; `DISPO`, `DISPOSABLE`; `AIO`, `ALL IN ONE`; `POD` | cartridge, disposable, all-in-one, pod (vape cart) |
//! | Subtype | `GUMMY`, `GUMMIES`; `CHOC`, `CHOCOLATE` | gummy, chocolate (edible) |
//! | Subtype | `SHATTER`; `WAX`, `BADDER`, `BUDDER`; `CRUMBLE`; `LR`, `LIVE RESIN`; `ROSIN`; `BUBBLE HASH`; `HASH`; `KIEF`; `DIAMONDS`; `SAUCE`; `DISTILLATE`; `RSO` | shatter, wax, crumble, live resin, rosin, bubble hash, pressed hash, dry sift, diamonds, sauce, distillate, RSO (concentrate) |
//! | Subtype | `SODA`; `SELTZER` | soda, seltzer (beverage) |
//! | Subtype | `BALM`, `LOTION`; `PATCH` | balm, patch (topical) |
//! | Strain type | `HYB`, `HYBRID`; `IND`, `INDICA`; `SAT`, `SATIVA` | hybrid, indica, sativa |
//! | Strain type | `IND DOM`, `INDICA HYBRID`, `I/H` (and the sativa forms) | the side it leans to |
//! | Strain type | `50/50`, `60/40` (two numbers adding up to 100) | hybrid, unless a type word follows |
//! | Size | `0.5G`, `1G`, `3.5 G`, `7G`, `14G`, `28G`, `100MG`, `10ML` | grams, mg or ml |
//! | Size | `1/8`, `1/4`, `1/2` (optionally `OZ`), `1 OZ`, `OZ`, `EIGHTH`, `QTR` | grams (an ounce is 28 g) |
//! | Pack | `5PK`, `10 PACK`, `20CT` | units in the package |
//! | Quantity | `2 X`, `2X`, `X2`, `QTY 2`, `2 @`, a leading count (`2 BD PR`) | units bought |
//! | Other | `INDOOR`, `OUTDOOR`, `GREENHOUSE`, `SUNGROWN`, `INFUSED`, `THC`, `CBD`, `25%` | kept aside |
//!
//! A subtype also gives its form. When a line names several forms, the
//! more specific wins: edible, beverage, tincture, topical, vape cart,
//! pre-roll, concentrate, flower (`LIVE RESIN CART` is a cart,
//! `INFUSED PR` with `KIEF` a pre-roll). The subtype is the first one the
//! line names under the form that won (`LIVE RESIN CART` is a cartridge).
//! An ounce is counted as 28 g and an eighth as 3.5 g, as dispensaries do.
//! In a size, a `C` read for a `G` is accepted only for the usual flower
//! weights (`3.5C` is 3.5 g), and letters inside a number only after a
//! real digit (`1O0MG` is 100 mg; `1O` alone is not a size).

use terpoir_model::{Form, PackageSize, SizeAmount, StrainType, Subtype};

use crate::number::trailing_price;

/// The unit of a package size.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SizeUnit {
    /// Grams (ounces and their fractions are converted to grams).
    Gram,
    /// Milligrams (edibles, usually of THC).
    Milligram,
    /// Millilitres.
    Millilitre,
}

impl SizeUnit {
    /// `g`, `mg` or `ml`.
    #[must_use]
    pub const fn slug(self) -> &'static str {
        match self {
            Self::Gram => "g",
            Self::Milligram => "mg",
            Self::Millilitre => "ml",
        }
    }
}

impl From<SizeUnit> for terpoir_model::SizeUnit {
    /// The purchase line's unit: the same slug.
    fn from(unit: SizeUnit) -> Self {
        match unit {
            SizeUnit::Gram => Self::Gram,
            SizeUnit::Milligram => Self::Milligram,
            SizeUnit::Millilitre => Self::Millilitre,
        }
    }
}

/// A package size.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Size {
    /// The amount in thousandths of `unit` (3.5 g is 3500).
    pub milli: i64,
    /// The unit.
    pub unit: SizeUnit,
}

/// What a line's text says about the item. Every field is a suggestion.
#[derive(Clone, PartialEq, Eq, Default)]
pub struct ItemBreakdown {
    /// What is left once the known terms are taken out, in upper case with
    /// single spaces: usually the brand and strain, often abbreviated.
    pub name: String,
    /// The package size.
    pub size: Option<Size>,
    /// The product form.
    pub form: Option<Form>,
    /// The subtype under [`form`](Self::form), when the line names one.
    pub subtype: Option<Subtype>,
    /// The strain type.
    pub strain_type: Option<StrainType>,
    /// Units bought, when printed.
    pub quantity: Option<u32>,
    /// Units in one package (`5PK`), when printed.
    pub pack: Option<u32>,
    /// Other known words taken out (`INDOOR`, `THC`, `25%`, a second size),
    /// as printed.
    pub other: Vec<String>,
}

impl ItemBreakdown {
    /// The size and pack count as a purchase line or product stores them
    /// ([`PackageSize`]), or `None` when the line prints neither. With both
    /// an amount and a pack, the amount is read as each piece's unless that
    /// is implausible for the line's form ([`crate::amount_per`]).
    #[must_use]
    pub fn package_size(&self) -> Option<PackageSize> {
        let mut size = PackageSize {
            amount: self.size.map(|s| SizeAmount {
                milli: s.milli,
                unit: s.unit.into(),
            }),
            pack: self.pack,
            per: None,
        };
        size.per = crate::amount_per(&size, self.form.as_ref());
        size.is_valid().then_some(size)
    }
}

/// A built-in term and what it means.
#[derive(Clone, Copy)]
enum Meaning {
    Form(&'static str),
    /// A subtype (slug), which also gives its parent form.
    Subtype(&'static str),
    Type(&'static str),
    Other,
}

/// The built-in phrases, as upper-case words. Matched longest first.
const TERMS: &[(&str, Meaning)] = &[
    // Pre-rolls.
    ("PR", Meaning::Form("pre_roll")),
    ("PRS", Meaning::Form("pre_roll")),
    ("PREROLL", Meaning::Form("pre_roll")),
    ("PREROLLS", Meaning::Form("pre_roll")),
    ("PRE ROLL", Meaning::Form("pre_roll")),
    ("PRE ROLLS", Meaning::Form("pre_roll")),
    ("JOINT", Meaning::Subtype("joint")),
    ("JOINTS", Meaning::Subtype("joint")),
    ("BLUNT", Meaning::Subtype("blunt")),
    ("BLUNTS", Meaning::Subtype("blunt")),
    ("INFUSED PR", Meaning::Subtype("infused_pre_roll")),
    ("INFUSED PRS", Meaning::Subtype("infused_pre_roll")),
    ("INFUSED PREROLL", Meaning::Subtype("infused_pre_roll")),
    ("INFUSED PREROLLS", Meaning::Subtype("infused_pre_roll")),
    ("INFUSED PRE ROLL", Meaning::Subtype("infused_pre_roll")),
    ("INFUSED PRE ROLLS", Meaning::Subtype("infused_pre_roll")),
    // Vapes.
    ("CART", Meaning::Subtype("cartridge")),
    ("CARTS", Meaning::Subtype("cartridge")),
    ("CARTRIDGE", Meaning::Subtype("cartridge")),
    ("CARTRIDGES", Meaning::Subtype("cartridge")),
    ("VAPE", Meaning::Form("vape_cart")),
    ("VAPES", Meaning::Form("vape_cart")),
    ("DISPO", Meaning::Subtype("disposable")),
    ("DISPOSABLE", Meaning::Subtype("disposable")),
    ("POD", Meaning::Subtype("pod")),
    ("PODS", Meaning::Subtype("pod")),
    ("DISPOS", Meaning::Subtype("disposable")),
    ("DISPOSABLES", Meaning::Subtype("disposable")),
    ("AIO", Meaning::Subtype("all_in_one")),
    ("ALL IN ONE", Meaning::Subtype("all_in_one")),
    // Flower.
    ("FLWR", Meaning::Form("flower")),
    ("FLW", Meaning::Form("flower")),
    ("FLWER", Meaning::Form("flower")),
    ("FLOWR", Meaning::Form("flower")),
    ("FLOWER", Meaning::Form("flower")),
    ("FLOWERS", Meaning::Form("flower")),
    ("BUD", Meaning::Form("flower")),
    ("BUDS", Meaning::Form("flower")),
    ("SMALLS", Meaning::Form("flower")),
    ("SHAKE", Meaning::Form("flower")),
    // Edibles.
    ("EDIBLE", Meaning::Form("edible")),
    ("EDIBLES", Meaning::Form("edible")),
    ("EDBL", Meaning::Form("edible")),
    ("GUMMY", Meaning::Subtype("gummy")),
    ("GUMMIES", Meaning::Subtype("gummy")),
    ("GUMMIE", Meaning::Subtype("gummy")),
    ("GUMMI", Meaning::Subtype("gummy")),
    ("GUMM", Meaning::Subtype("gummy")),
    ("GUMMS", Meaning::Subtype("gummy")),
    ("CHOCOLATE", Meaning::Subtype("chocolate")),
    ("CHOC", Meaning::Subtype("chocolate")),
    ("CHEW", Meaning::Form("edible")),
    ("CHEWS", Meaning::Form("edible")),
    // Concentrates.
    ("CONC", Meaning::Form("concentrate")),
    ("CONCENTRATE", Meaning::Form("concentrate")),
    ("WAX", Meaning::Subtype("wax")),
    ("SHATTER", Meaning::Subtype("shatter")),
    ("BADDER", Meaning::Subtype("wax")),
    ("BATTER", Meaning::Subtype("wax")),
    ("BUDDER", Meaning::Subtype("wax")),
    ("ROSIN", Meaning::Subtype("rosin")),
    ("RESIN", Meaning::Form("concentrate")),
    ("LR", Meaning::Subtype("live_resin")),
    ("LIVE RESIN", Meaning::Subtype("live_resin")),
    ("LIVE ROSIN", Meaning::Subtype("rosin")),
    ("HASH", Meaning::Subtype("pressed_hash")),
    ("KIEF", Meaning::Subtype("dry_sift")),
    ("DIAMONDS", Meaning::Subtype("diamonds")),
    ("SAUCE", Meaning::Subtype("sauce")),
    ("CRUMBLE", Meaning::Subtype("crumble")),
    ("BUBBLE HASH", Meaning::Subtype("bubble_hash")),
    ("DISTILLATE", Meaning::Subtype("distillate")),
    ("DISTY", Meaning::Subtype("distillate")),
    ("RSO", Meaning::Subtype("rso")),
    // Tinctures, topicals, drinks.
    ("TINC", Meaning::Form("tincture")),
    ("TINCT", Meaning::Form("tincture")),
    ("TINCTURE", Meaning::Form("tincture")),
    ("TINCTURES", Meaning::Form("tincture")),
    ("DROPS", Meaning::Form("tincture")),
    ("TOPICAL", Meaning::Form("topical")),
    ("TOPICALS", Meaning::Form("topical")),
    ("BALM", Meaning::Subtype("balm")),
    ("LOTION", Meaning::Subtype("balm")),
    ("SALVE", Meaning::Form("topical")),
    ("PATCH", Meaning::Subtype("patch")),
    ("PATCHES", Meaning::Subtype("patch")),
    ("DRINK", Meaning::Form("beverage")),
    ("DRINKS", Meaning::Form("beverage")),
    ("BEV", Meaning::Form("beverage")),
    ("BEVERAGE", Meaning::Form("beverage")),
    ("SODA", Meaning::Subtype("soda")),
    ("SELTZER", Meaning::Subtype("seltzer")),
    // Strain types.
    ("HY", Meaning::Type("hybrid")),
    ("HYB", Meaning::Type("hybrid")),
    ("HYBR", Meaning::Type("hybrid")),
    ("HYBRID", Meaning::Type("hybrid")),
    ("IND", Meaning::Type("indica")),
    ("INDI", Meaning::Type("indica")),
    ("INDICA", Meaning::Type("indica")),
    ("SAT", Meaning::Type("sativa")),
    ("SATI", Meaning::Type("sativa")),
    ("SATIVA", Meaning::Type("sativa")),
    ("I/H", Meaning::Type("indica")),
    ("S/H", Meaning::Type("sativa")),
    ("IND DOM", Meaning::Type("indica")),
    ("INDICA DOM", Meaning::Type("indica")),
    ("INDICA DOMINANT", Meaning::Type("indica")),
    ("IND HYB", Meaning::Type("indica")),
    ("INDICA HYBRID", Meaning::Type("indica")),
    ("SAT DOM", Meaning::Type("sativa")),
    ("SATIVA DOM", Meaning::Type("sativa")),
    ("SATIVA DOMINANT", Meaning::Type("sativa")),
    ("SAT HYB", Meaning::Type("sativa")),
    ("SATIVA HYBRID", Meaning::Type("sativa")),
    // Kept aside.
    ("INDOOR", Meaning::Other),
    ("OUTDOOR", Meaning::Other),
    ("GREENHOUSE", Meaning::Other),
    ("SUNGROWN", Meaning::Other),
    ("SUN GROWN", Meaning::Other),
    ("LIGHT DEP", Meaning::Other),
    ("INFUSED", Meaning::Other),
    ("THC", Meaning::Other),
    ("THCA", Meaning::Other),
    ("CBD", Meaning::Other),
    ("EA", Meaning::Other),
    ("EACH", Meaning::Other),
];

/// Forms by how specific they are: the first one a line names wins.
const FORM_ORDER: &[&str] = &[
    "edible",
    "beverage",
    "tincture",
    "topical",
    "vape_cart",
    "pre_roll",
    "concentrate",
    "flower",
];

/// Weights a `C` may stand for a `G` in, in milligrams.
const USUAL_GRAMS: &[i64] = &[500, 1_000, 2_000, 3_500, 7_000, 14_000, 28_000];

/// Folds the usual recognition confusions in a word, for comparing terms:
/// `0`→`O`, `1`/`L`/`|`→`I`, `5`→`S`, `8`→`B`. Words are upper case.
fn fold_term(word: &str) -> String {
    word.chars()
        .map(|c| match c {
            '0' => 'O',
            '1' | 'L' | '|' => 'I',
            '5' => 'S',
            '8' => 'B',
            other => other,
        })
        .collect()
}

/// Upper-case words of `text`. Punctuation separates words, except a
/// decimal point or comma inside a number (`3.5G`), a slash between
/// letters or digits (`1/8`, `I/H`), a colon between digits (`1:1`) and a
/// percent sign after a number (`25%`).
pub(crate) fn words(text: &str) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    let mut current = String::new();
    for (i, &c) in chars.iter().enumerate() {
        let prev = i.checked_sub(1).map(|p| chars[p]);
        let next = chars.get(i + 1).copied();
        let keep = if c.is_alphanumeric() {
            true
        } else {
            match c {
                '.' | ',' => {
                    next.is_some_and(|n| n.is_ascii_digit())
                        && prev.is_none_or(|p| !p.is_alphabetic())
                        || prev.is_some_and(|p| p.is_ascii_digit())
                            && next.is_some_and(char::is_alphanumeric)
                }
                '/' => {
                    prev.is_some_and(char::is_alphanumeric)
                        && next.is_some_and(char::is_alphanumeric)
                }
                ':' => {
                    prev.is_some_and(|p| p.is_ascii_digit())
                        && next.is_some_and(|n| n.is_ascii_digit())
                }
                '%' => prev.is_some_and(|p| p.is_ascii_digit()),
                _ => false,
            }
        };
        if keep {
            current.extend(c.to_uppercase());
        } else if !current.is_empty() {
            out.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        out.push(current);
    }
    out
}

/// A plain number in thousandths: digits with at most one decimal point
/// or comma, or a leading point (`.5`).
fn number_milli(text: &str) -> Option<i64> {
    let (int, frac) = match text.find(['.', ',']) {
        Some(at) => (&text[..at], &text[at + 1..]),
        None => (text, ""),
    };
    if int.is_empty() && frac.is_empty()
        || int.len() > 6
        || frac.len() > 3
        || !int.bytes().chain(frac.bytes()).all(|b| b.is_ascii_digit())
    {
        return None;
    }
    let int: i64 = if int.is_empty() { 0 } else { int.parse().ok()? };
    let mut frac_milli: i64 = if frac.is_empty() {
        0
    } else {
        frac.parse().ok()?
    };
    for _ in frac.len()..3 {
        frac_milli *= 10;
    }
    Some(int * 1000 + frac_milli)
}

/// A number with recognition confusions folded: letters count as digits
/// only after a real digit (`1O0` is 100, `O1` is not a number).
fn folded_number_milli(text: &str) -> Option<i64> {
    if let Some(n) = number_milli(text) {
        return Some(n);
    }
    let mut seen_digit = false;
    let mut folded = String::new();
    for c in text.chars() {
        let d = match c {
            '0'..='9' => {
                seen_digit = true;
                c
            }
            '.' | ',' => c,
            'O' if seen_digit => '0',
            'I' | 'L' | '|' if seen_digit => '1',
            'S' if seen_digit => '5',
            'B' if seen_digit => '8',
            _ => return None,
        };
        folded.push(d);
    }
    number_milli(&folded)
}

/// An integer count of 1 to 999.
fn count(text: &str) -> Option<u32> {
    (text.len() <= 3 && text.bytes().all(|b| b.is_ascii_digit()))
        .then(|| text.parse().ok())
        .flatten()
        .filter(|n| *n > 0)
}

fn unit(text: &str) -> Option<(SizeUnit, i64)> {
    // The factor converts thousandths of the printed unit to thousandths
    // of the stored unit, times 1000.
    Some(match text {
        "G" | "GR" | "GM" | "GRAM" | "GRAMS" => (SizeUnit::Gram, 1_000),
        "MG" => (SizeUnit::Milligram, 1_000),
        "ML" => (SizeUnit::Millilitre, 1_000),
        "OZ" => (SizeUnit::Gram, 28_000),
        _ => return None,
    })
}

fn size_of(milli: i64, unit: SizeUnit, factor: i64) -> Option<Size> {
    let milli = milli.checked_mul(factor)? / 1_000;
    (milli > 0).then_some(Size { milli, unit })
}

/// A fraction of an ounce: `1/8`, `1/4`, `1/2`.
fn ounce_fraction(text: &str) -> Option<Size> {
    let milli = match text {
        "1/8" => 3_500,
        "1/4" => 7_000,
        "1/2" => 14_000,
        _ => return None,
    };
    Some(Size {
        milli,
        unit: SizeUnit::Gram,
    })
}

/// A size in one word: `3.5G`, `100MG`, `1/8OZ`, `EIGHTH`, `3.5C`.
fn size_word(word: &str) -> Option<Size> {
    match word {
        "EIGHTH" | "8TH" => return ounce_fraction("1/8"),
        "QUARTER" | "QTR" => return ounce_fraction("1/4"),
        "OZ" => {
            return Some(Size {
                milli: 28_000,
                unit: SizeUnit::Gram,
            });
        }
        _ => {}
    }
    if let Some(size) = ounce_fraction(word.strip_suffix("OZ").unwrap_or(word)) {
        return Some(size);
    }
    if !word.starts_with(|c: char| c.is_ascii_digit() || c == '.') {
        return None;
    }
    // Longest number first, plain before folded.
    for fold in [false, true] {
        for (at, _) in word.char_indices().rev().filter(|(at, _)| *at > 0) {
            let (number, rest) = word.split_at(at);
            let milli = if fold {
                folded_number_milli(number)
            } else {
                number_milli(number)
            };
            let Some(milli) = milli else { continue };
            if let Some((unit, factor)) = unit(rest) {
                return size_of(milli, unit, factor);
            }
            if rest == "C" && USUAL_GRAMS.contains(&milli) {
                return size_of(milli, SizeUnit::Gram, 1_000);
            }
        }
    }
    None
}

/// A size across one or two words, and how many words it takes.
fn size_at(words: &[String], i: usize) -> Option<(Size, usize)> {
    let word = words[i].as_str();
    let next = words.get(i + 1).map(String::as_str);
    // `3.5 G`, `100 MG`, `1 OZ`, `1/8 OZ`.
    if let Some(next) = next {
        if let Some(fraction) = ounce_fraction(word).filter(|_| next == "OZ") {
            return Some((fraction, 2));
        }
        if let (Some(milli), Some((unit, factor))) = (folded_number_milli(word), unit(next))
            && let Some(size) = size_of(milli, unit, factor)
        {
            return Some((size, 2));
        }
    }
    size_word(word).map(|s| (s, 1))
}

/// A pack count: `5PK`, `10 PACK`, `20CT`.
fn pack_at(words: &[String], i: usize) -> Option<(u32, usize)> {
    let is_pack = |w: &str| matches!(w, "PK" | "PACK" | "CT" | "COUNT");
    let word = words[i].as_str();
    if let Some(n) = count(word)
        && words.get(i + 1).is_some_and(|w| is_pack(w))
    {
        return Some((n, 2));
    }
    let digits = word.bytes().take_while(u8::is_ascii_digit).count();
    (digits > 0 && is_pack(&word[digits..]))
        .then(|| count(&word[..digits]).map(|n| (n, 1)))
        .flatten()
}

/// A quantity bought: `2 X`, `2X`, `X2`, `QTY 2`.
fn quantity_at(words: &[String], i: usize) -> Option<(u32, usize)> {
    let word = words[i].as_str();
    let next = words.get(i + 1).map(String::as_str);
    if let Some(n) = count(word)
        && next == Some("X")
    {
        return Some((n, 2));
    }
    if word == "QTY" {
        return next.and_then(count).map(|n| (n, 2));
    }
    if let Some(rest) = word.strip_prefix("QTY") {
        return count(rest).map(|n| (n, 1));
    }
    if let Some(n) = word.strip_suffix('X').and_then(count) {
        return Some((n, 1));
    }
    word.strip_prefix('X').and_then(count).map(|n| (n, 1))
}

/// A ratio such as `50/50` or `70/30` (adding up to 100).
fn is_type_ratio(word: &str) -> bool {
    word.split_once('/')
        .is_some_and(|(a, b)| matches!((count(a), count(b)), (Some(a), Some(b)) if a + b == 100))
}

/// The longest built-in phrase starting at `words[i]` whose words are all
/// unused, and how many words it takes.
fn term_at(folded: &[String], used: &[bool], i: usize) -> Option<(Meaning, usize)> {
    TERMS
        .iter()
        .filter_map(|&(phrase, meaning)| {
            let parts: Vec<&str> = phrase.split(' ').collect();
            let n = parts.len();
            let fits = i + n <= folded.len()
                && (i..i + n).all(|j| !used[j])
                && parts
                    .iter()
                    .zip(&folded[i..i + n])
                    .all(|(p, w)| fold_term(p) == *w);
            fits.then_some((meaning, n))
        })
        .max_by_key(|&(_, n)| n)
}

/// Whether `word` (upper case) is a built-in term on its own, compared
/// after folding.
pub(crate) fn is_term(word: &str) -> bool {
    let folded = fold_term(&word.to_uppercase());
    TERMS
        .iter()
        .any(|(phrase, _)| !phrase.contains(' ') && fold_term(phrase) == folded)
}

/// What [`describe_item`] found, with the words of the name before terms
/// were taken out (sizes, packs and quantities already removed).
pub(crate) struct Described {
    pub(crate) breakdown: ItemBreakdown,
    /// The name with only amounts taken out: `HASH PLANT 1G` keeps `HASH
    /// PLANT`, which matters when a strain's name contains a term.
    pub(crate) with_terms: String,
}

pub(crate) fn describe(text: &str) -> Described {
    let text = text.trim();
    let chars: Vec<char> = text.chars().collect();
    let text: String = match trailing_price(&chars) {
        Some((_, start)) => chars[..start].iter().collect(),
        None => text.to_owned(),
    };
    let words = words(&text);
    let mut used = vec![false; words.len()];
    let mut out = ItemBreakdown::default();

    // Amounts first, so `1G` is never read as a term.
    let mut i = 0;
    while i < words.len() {
        if let Some((size, n)) = size_at(&words, i) {
            if out.size.is_none() {
                out.size = Some(size);
            } else {
                out.other.push(words[i..i + n].join(" "));
            }
            used[i..i + n].fill(true);
            i += n;
        } else if let Some((pack, n)) = pack_at(&words, i) {
            out.pack.get_or_insert(pack);
            used[i..i + n].fill(true);
            i += n;
        } else if let Some((quantity, n)) = quantity_at(&words, i) {
            out.quantity.get_or_insert(quantity);
            used[i..i + n].fill(true);
            i += n;
        } else if words[i].ends_with('%') || words[i].contains(':') {
            out.other.push(words[i].clone());
            used[i] = true;
            i += 1;
        } else {
            i += 1;
        }
    }
    // A leading count: `2 BD PR`, or `2 @ Gummies` (the `@` is dropped).
    if out.quantity.is_none()
        && !used.first().copied().unwrap_or(true)
        && used.get(1).is_some_and(|u| !u)
        && let Some(n) = count(&words[0]).filter(|n| *n < 100)
    {
        out.quantity = Some(n);
        used[0] = true;
    }
    let with_terms = join_unused(&words, &used);

    // Then the terms.
    let folded: Vec<String> = words.iter().map(|w| fold_term(w)).collect();
    let mut forms: Vec<&str> = Vec::new();
    let mut subtypes: Vec<Subtype> = Vec::new();
    let mut i = 0;
    while i < words.len() {
        if used[i] {
            i += 1;
            continue;
        }
        if is_type_ratio(&words[i]) {
            out.other.push(words[i].clone());
            used[i] = true;
            i += 1;
            continue;
        }
        let Some((meaning, n)) = term_at(&folded, &used, i) else {
            i += 1;
            continue;
        };
        match meaning {
            Meaning::Form(slug) => forms.push(slug),
            Meaning::Subtype(slug) => {
                let subtype = Subtype::from_slug(slug);
                if let Some(parent) = subtype.parent() {
                    forms.push(form_slug(&parent));
                }
                subtypes.push(subtype);
            }
            Meaning::Type(slug) => {
                out.strain_type
                    .get_or_insert_with(|| StrainType::from_slug(slug));
            }
            Meaning::Other => out.other.push(words[i..i + n].join(" ")),
        }
        used[i..i + n].fill(true);
        i += n;
    }
    // A bare ratio means a hybrid.
    if out.strain_type.is_none() && out.other.iter().any(|o| is_type_ratio(o)) {
        out.strain_type = Some(StrainType::Hybrid);
    }
    out.form = FORM_ORDER
        .iter()
        .find(|slug| forms.contains(slug))
        .map(|slug| Form::from_slug(slug));
    out.subtype = subtypes
        .into_iter()
        .find(|s| out.form.is_some() && s.parent() == out.form);
    out.name = join_unused(&words, &used);
    Described {
        breakdown: out,
        with_terms,
    }
}

/// A known form's slug, borrowed from the form order.
fn form_slug(form: &Form) -> &'static str {
    FORM_ORDER
        .iter()
        .find(|slug| **slug == form.slug())
        .copied()
        .unwrap_or("")
}

fn join_unused(words: &[String], used: &[bool]) -> String {
    words
        .iter()
        .zip(used)
        .filter(|(_, u)| !**u)
        .map(|(w, _)| w.as_str())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Breaks a receipt line's or label's item text into size, form, strain
/// type, pack count, quantity, and the name that is left. A price at the
/// end is ignored.
#[must_use]
pub fn describe_item(text: &str) -> ItemBreakdown {
    describe(text).breakdown
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grams(milli: i64) -> Option<Size> {
        Some(Size {
            milli,
            unit: SizeUnit::Gram,
        })
    }

    #[test]
    fn splits_words() {
        assert_eq!(
            words("Blue Dream 3.5g (Hyb) - PRE-ROLL 1/8 I/H x2 25% 1:1"),
            [
                "BLUE", "DREAM", "3.5G", "HYB", "PRE", "ROLL", "1/8", "I/H", "X2", "25%", "1:1"
            ]
        );
        assert_eq!(words(".5G, 3,5 g."), [".5G", "3,5", "G"]);
    }

    #[test]
    fn both_abbreviations_leave_the_strain() {
        let a = describe_item("BD PR 1G");
        assert_eq!(a.name, "BD");
        assert_eq!(a.form, Some(Form::PreRoll));
        assert_eq!(a.size, grams(1_000));
        assert_eq!(a.strain_type, None);

        let b = describe_item("BD 3.5G HYB");
        assert_eq!(b.name, "BD");
        assert_eq!(b.form, None);
        assert_eq!(b.size, grams(3_500));
        assert_eq!(b.strain_type, Some(StrainType::Hybrid));
    }

    #[test]
    fn reads_a_wrapped_line() {
        let d = describe_item("BLUE DREAM 3.5G HYBRID FLOWER - INDOOR 35.00");
        assert_eq!(d.name, "BLUE DREAM");
        assert_eq!(d.form, Some(Form::Flower));
        assert_eq!(d.strain_type, Some(StrainType::Hybrid));
        assert_eq!(d.size, grams(3_500));
        assert_eq!(d.other, ["INDOOR"]);
    }

    #[test]
    fn package_sizes_map_onto_the_purchase_line() {
        let size = describe_item("BD PR 5PK 0.5G").package_size().unwrap();
        // Five half-gram pre-rolls: plausible, so each piece's.
        assert_eq!(size.per, Some(terpoir_model::AmountPer::Each));
        let amount = size.amount.unwrap();
        assert_eq!(
            (amount.milli, amount.unit, size.pack),
            (500, terpoir_model::SizeUnit::Gram, Some(5))
        );
        let size = describe_item("GUMMIES 100MG").package_size().unwrap();
        assert_eq!(
            size.amount.unwrap().unit,
            terpoir_model::SizeUnit::Milligram
        );
        assert_eq!(size.per, None);
        // 20 gummies of 100 mg would be 2000 mg: the pack's total.
        let size = describe_item("GUMMIES 20PK 100MG").package_size().unwrap();
        assert_eq!(size.per, Some(terpoir_model::AmountPer::Total));
        assert_eq!(describe_item("BD HYB").package_size(), None);
        // The units' slugs are the purchase line's.
        for unit in [SizeUnit::Gram, SizeUnit::Milligram, SizeUnit::Millilitre] {
            assert_eq!(terpoir_model::SizeUnit::from(unit).slug(), unit.slug());
        }
    }

    #[test]
    fn reads_sizes() {
        for (text, size) in [
            ("X 0.5G", grams(500)),
            ("X .5G", grams(500)),
            ("X 1G", grams(1_000)),
            ("X 1 G", grams(1_000)),
            ("X 3,5 GRAMS", grams(3_500)),
            ("X 7G", grams(7_000)),
            ("X 14G", grams(14_000)),
            ("X 28G", grams(28_000)),
            ("X 1/8", grams(3_500)),
            ("X 1/8OZ", grams(3_500)),
            ("X 1/4 OZ", grams(7_000)),
            ("X 1/2", grams(14_000)),
            ("X 1 OZ", grams(28_000)),
            ("X OZ", grams(28_000)),
            ("X EIGHTH", grams(3_500)),
            (
                "X 100MG",
                Some(Size {
                    milli: 100_000,
                    unit: SizeUnit::Milligram,
                }),
            ),
            (
                "X 10 ML",
                Some(Size {
                    milli: 10_000,
                    unit: SizeUnit::Millilitre,
                }),
            ),
        ] {
            let d = describe_item(text);
            assert!(d.size == size, "{text}");
            assert_eq!(d.name, "X", "{text}");
        }
    }

    #[test]
    fn folds_recognition_confusions_carefully() {
        // `C` for `G` in a usual weight.
        assert_eq!(describe_item("BD 3.5C").size, grams(3_500));
        assert_eq!(describe_item("BD 1C").size, grams(1_000));
        // Not in an unusual one.
        assert_eq!(describe_item("BD 4.2C").size, None);
        // Letters inside a number, after a real digit.
        assert_eq!(
            describe_item("GUMMIES 1O0MG").size,
            Some(Size {
                milli: 100_000,
                unit: SizeUnit::Milligram
            })
        );
        assert_eq!(describe_item("BD 3.SG").size, grams(3_500));
        assert_eq!(describe_item("BD 1OG").size, grams(10_000));
        // `1O` alone is not a size, and an ounce is not ten.
        let d = describe_item("BD 1O");
        assert_eq!((d.size, d.name.as_str()), (None, "BD 1O"));
        assert_eq!(describe_item("BD 1OZ").size, grams(28_000));
        // Terms: `HY8` is `HYB`, `1ND` is `IND`, `5AT` is `SAT`.
        assert_eq!(
            describe_item("BD HY8").strain_type,
            Some(StrainType::Hybrid)
        );
        assert_eq!(
            describe_item("GSC 1ND").strain_type,
            Some(StrainType::Indica)
        );
        assert_eq!(
            describe_item("GSC 5AT").strain_type,
            Some(StrainType::Sativa)
        );
        assert_eq!(describe_item("FLWR BD").form, Some(Form::Flower));
        assert_eq!(describe_item("F1WR BD").form, Some(Form::Flower));
    }

    #[test]
    fn reads_quantities_and_packs() {
        for text in [
            "2 X BD PR",
            "2X BD PR",
            "BD PR X2",
            "BD PR QTY 2",
            "BD PR QTY:2",
        ] {
            let d = describe_item(text);
            assert_eq!(d.quantity, Some(2), "{text}");
            assert_eq!(d.name, "BD", "{text}");
        }
        let d = describe_item("2 @ MANGO GUMMIES 100MG");
        assert_eq!((d.quantity, d.name.as_str()), (Some(2), "MANGO"));
        let d = describe_item("3 BD PR 0.5G 5PK");
        assert_eq!(d.quantity, Some(3));
        assert_eq!(d.pack, Some(5));
        assert_eq!(d.size, grams(500));
        assert_eq!(d.name, "BD");
        // A size is not a quantity.
        let d = describe_item("1G BD PR");
        assert_eq!((d.quantity, d.size), (None, grams(1_000)));
        // A number alone is kept: it can be part of a strain's name.
        let d = describe_item("GELATO 41 3.5G");
        assert_eq!((d.quantity, d.name.as_str()), (None, "GELATO 41"));
    }

    #[test]
    fn reads_forms_and_types() {
        let d = describe_item("Live Resin Cart 1g");
        assert_eq!(d.form, Some(Form::VapeCart));
        assert_eq!(d.name, "");
        assert_eq!(describe_item("LR BADDER 1G").form, Some(Form::Concentrate));
        assert_eq!(describe_item("Mango Gummies").form, Some(Form::Edible));
        assert_eq!(describe_item("PRE-ROLL GG4").form, Some(Form::PreRoll));
        assert_eq!(describe_item("CBD TINCT 30ML").form, Some(Form::Tincture));
        assert_eq!(describe_item("Relief balm").form, Some(Form::Topical));
        assert_eq!(describe_item("Lemon BEV 10mg").form, Some(Form::Beverage));
        assert_eq!(describe_item("BD BLUNT 1G").form, Some(Form::PreRoll));
        let d = describe_item("GDP INDICA DOMINANT 1/8");
        assert_eq!(d.strain_type, Some(StrainType::Indica));
        assert_eq!(d.name, "GDP");
        assert_eq!(
            describe_item("Jack I/H").strain_type,
            Some(StrainType::Indica)
        );
        let d = describe_item("GSC 50/50 3.5G");
        assert_eq!(
            (d.strain_type, d.name.as_str()),
            (Some(StrainType::Hybrid), "GSC")
        );
        let d = describe_item("GSC 70/30 SAT");
        assert_eq!(d.strain_type, Some(StrainType::Sativa));
        let d = describe_item("BD THC 25% 1:1");
        assert_eq!(d.name, "BD");
        assert_eq!(d.other, ["25%", "1:1", "THC"]);
    }

    #[test]
    fn reads_subtypes_with_their_forms() {
        let read = |text: &str| {
            let d = describe_item(text);
            (d.form, d.subtype, d.name)
        };
        for (text, form, subtype, name) in [
            ("GG4 ROSIN 1G", Form::Concentrate, Subtype::Rosin, "GG4"),
            (
                "Live Resin Sugar 1g",
                Form::Concentrate,
                Subtype::LiveResin,
                "SUGAR",
            ),
            ("LR BADDER 1G", Form::Concentrate, Subtype::LiveResin, ""),
            ("Sherb Badder", Form::Concentrate, Subtype::Wax, "SHERB"),
            ("BUDDER", Form::Concentrate, Subtype::Wax, ""),
            ("SHATTER", Form::Concentrate, Subtype::Shatter, ""),
            ("CRUMBLE", Form::Concentrate, Subtype::Crumble, ""),
            ("Diamonds", Form::Concentrate, Subtype::Diamonds, ""),
            ("Sauce", Form::Concentrate, Subtype::Sauce, ""),
            ("Distillate", Form::Concentrate, Subtype::Distillate, ""),
            ("RSO 1G", Form::Concentrate, Subtype::Rso, ""),
            ("Bubble Hash", Form::Concentrate, Subtype::BubbleHash, ""),
            (
                "Temple Ball Hash",
                Form::Concentrate,
                Subtype::PressedHash,
                "TEMPLE BALL",
            ),
            ("KIEF 1G", Form::Concentrate, Subtype::DrySift, ""),
            ("BD AIO 1G", Form::VapeCart, Subtype::AllInOne, "BD"),
            ("All In One Vape", Form::VapeCart, Subtype::AllInOne, ""),
            ("BD DISPO 0.5G", Form::VapeCart, Subtype::Disposable, "BD"),
            ("Disposable", Form::VapeCart, Subtype::Disposable, ""),
            ("Live Resin Cart 1g", Form::VapeCart, Subtype::Cartridge, ""),
            ("Pod", Form::VapeCart, Subtype::Pod, ""),
            ("INFUSED PR 1G", Form::PreRoll, Subtype::InfusedPreRoll, ""),
            (
                "Infused Pre-Roll Kief",
                Form::PreRoll,
                Subtype::InfusedPreRoll,
                "",
            ),
            ("BD BLUNT 1G", Form::PreRoll, Subtype::Blunt, "BD"),
            ("JOINT", Form::PreRoll, Subtype::Joint, ""),
            ("Mango Gummies", Form::Edible, Subtype::Gummy, "MANGO"),
            ("Dark CHOC 100MG", Form::Edible, Subtype::Chocolate, "DARK"),
            ("Chocolate", Form::Edible, Subtype::Chocolate, ""),
            ("Relief PATCH 20MG", Form::Topical, Subtype::Patch, "RELIEF"),
            ("Relief balm", Form::Topical, Subtype::Balm, "RELIEF"),
            ("Lime Seltzer", Form::Beverage, Subtype::Seltzer, "LIME"),
        ] {
            assert_eq!(
                read(text),
                (Some(form), Some(subtype), name.to_owned()),
                "{text}"
            );
        }
        // A form named without a subtype has none.
        assert_eq!(read("PR GG4").1, None);
        assert_eq!(read("CONC").1, None);
        // INFUSED alone is still kept aside.
        assert_eq!(describe_item("INFUSED GUMMY").other, ["INFUSED"]);
    }

    #[test]
    fn keeps_a_name_with_a_term_in_it() {
        let d = describe("HASH PLANT 1G");
        assert_eq!(d.breakdown.name, "PLANT");
        assert_eq!(d.with_terms, "HASH PLANT");
        assert!(is_term("hash"));
        assert!(!is_term("PLANT"));
    }

    #[test]
    fn empty_text_gives_nothing() {
        let d = describe_item("   ");
        assert!(d.name.is_empty() && d.size.is_none() && d.quantity.is_none());
        assert!(describe_item("35.00").name.is_empty());
    }
}

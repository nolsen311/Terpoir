// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

//! Compliance labels: recognized text in, product fields to confirm out.

use terpoir_model::{LabelUnit, Terpene};

use crate::code::is_package_tag;
use crate::date::{Date, find_date};
use crate::number::milli_at;

/// A cannabinoid amount read from a label.
#[derive(Clone, PartialEq, Eq)]
pub struct LabelAmountGuess {
    /// Thousandths of `unit`.
    pub milli: i64,
    /// Percent, mg per unit or mg in the package.
    pub unit: LabelUnit,
}

/// What a label seems to say. Every field is a suggestion to confirm.
#[derive(Clone, PartialEq, Eq, Default)]
pub struct LabelScan {
    /// THC (total THC when the label gives it).
    pub thc: Option<LabelAmountGuess>,
    /// CBD (total CBD when the label gives it).
    pub cbd: Option<LabelAmountGuess>,
    /// Terpenes in thousandths of a percent, in label order.
    pub terpenes: Vec<(Terpene, i64)>,
    /// Batch or lot.
    pub batch: Option<String>,
    /// Package date.
    pub package_date: Option<Date>,
    /// Seed-to-sale package tags printed on the label.
    pub package_tags: Vec<String>,
}

/// A line as characters, with a lower-case copy at the same indices.
pub(crate) struct Line {
    pub(crate) chars: Vec<char>,
    pub(crate) lower: Vec<char>,
}

impl Line {
    pub(crate) fn new(text: &str) -> Self {
        let chars: Vec<char> = text.chars().collect();
        let lower = chars
            .iter()
            .map(|c| c.to_lowercase().next().unwrap_or(*c))
            .collect();
        Self { chars, lower }
    }

    /// Indices where `word` starts as a whole word.
    fn find_word(&self, word: &str) -> Vec<usize> {
        let pat: Vec<char> = word.chars().collect();
        let n = pat.len();
        (0..=self.lower.len().saturating_sub(n))
            .filter(|&i| {
                self.lower.get(i..i + n) == Some(&pat[..])
                    && (i == 0 || !self.lower[i - 1].is_alphanumeric())
                    && self.lower.get(i + n).is_none_or(|c| !c.is_alphanumeric())
            })
            .collect()
    }

    fn rest(&self, from: usize) -> String {
        self.chars.get(from..).unwrap_or_default().iter().collect()
    }

    fn lower_from(&self, from: usize, len: usize) -> String {
        self.lower.iter().skip(from).take(len).collect()
    }
}

fn skip(line: &Line, mut i: usize, separators: &[char]) -> usize {
    while line.chars.get(i).is_some_and(|c| separators.contains(c)) {
        i += 1;
    }
    i
}

/// The unit right after a number: `%`, `mg/g`, or `mg` (per unit when the
/// label says so nearby).
fn unit_at(line: &Line, i: usize) -> Option<(LabelUnit, i64, usize)> {
    let i = skip(line, i, &[' ']);
    if line.chars.get(i) == Some(&'%') {
        return Some((LabelUnit::Percent, 1, i + 1));
    }
    if line.lower_from(i, 2) != "mg" {
        return None;
    }
    if line.lower_from(i, 4) == "mg/g" {
        // 10 mg/g is 1 %.
        return Some((LabelUnit::Percent, 10, i + 4));
    }
    let nearby = line.lower_from(i + 2, 24);
    let per_unit = [
        "per", "/", "each", "serving", "piece", "unit", "gummy", "dose",
    ]
    .iter()
    .any(|w| nearby.contains(w));
    let unit = if per_unit {
        LabelUnit::MgPerUnit
    } else {
        LabelUnit::MgTotal
    };
    Some((unit, 1, i + 2))
}

/// An amount for `keyword` on this line: `THC: 22.4%`, `THC 10mg per
/// piece`, or `22.4% THC`.
fn amount_for(line: &Line, keyword: &str) -> Option<LabelAmountGuess> {
    for at in line.find_word(keyword) {
        let after = skip(line, at + keyword.chars().count(), &[' ', ':', '=', '-']);
        if let Some((milli, end)) = milli_at(&line.chars, after)
            && let Some((unit, scale, _)) = unit_at(line, end)
        {
            return guess(milli / scale, unit);
        }
        // Number before the keyword, on the same line, so the per-unit
        // check still sees the words after it.
        let mut end = at;
        while end > 0 && line.chars[end - 1] == ' ' {
            end -= 1;
        }
        let head = line.lower_from(0, end);
        let suffix = if head.ends_with('%') {
            1
        } else if head.ends_with("mg/g") {
            4
        } else if head.ends_with("mg") {
            2
        } else {
            continue;
        };
        let mut number_end = end - suffix;
        while number_end > 0 && line.chars[number_end - 1] == ' ' {
            number_end -= 1;
        }
        let mut start = number_end;
        while start > 0
            && (line.chars[start - 1].is_ascii_digit()
                || matches!(line.chars[start - 1], '.' | ','))
        {
            start -= 1;
        }
        if let Some((milli, number)) = milli_at(&line.chars, start)
            && number == number_end
            && let Some((unit, scale, _)) = unit_at(line, number)
        {
            return guess(milli / scale, unit);
        }
    }
    None
}

fn guess(milli: i64, unit: LabelUnit) -> Option<LabelAmountGuess> {
    let max = match unit {
        LabelUnit::Percent => 100_000,
        _ => 100_000_000,
    };
    (0..=max)
        .contains(&milli)
        .then_some(LabelAmountGuess { milli, unit })
}

fn cannabinoid(lines: &[Line], name: &str) -> Option<LabelAmountGuess> {
    let total = format!("total {name}");
    lines
        .iter()
        .find_map(|l| amount_for(l, &total))
        .or_else(|| lines.iter().find_map(|l| amount_for(l, name)))
}

pub(crate) const TERPENES: &[(&str, Terpene)] = &[
    ("myrcene", Terpene::Myrcene),
    ("limonene", Terpene::Limonene),
    ("caryophyllene", Terpene::Caryophyllene),
    ("linalool", Terpene::Linalool),
    ("humulene", Terpene::Humulene),
    ("terpinolene", Terpene::Terpinolene),
    ("ocimene", Terpene::Ocimene),
    ("bisabolol", Terpene::Bisabolol),
    ("nerolidol", Terpene::Nerolidol),
    ("terpineol", Terpene::Terpineol),
    ("pinene", Terpene::AlphaPinene),
];

pub(crate) fn terpene_at(line: &Line, at: usize, name: &str, default: &Terpene) -> Terpene {
    if name != "pinene" {
        return default.clone();
    }
    let before: String = line.lower[at.saturating_sub(6)..at].iter().collect();
    let before = before.trim_end_matches(['-', ' ']);
    if before.ends_with("beta") || before.ends_with('b') || before.ends_with('β') {
        Terpene::BetaPinene
    } else {
        Terpene::AlphaPinene
    }
}

pub(crate) fn terpenes(lines: &[Line]) -> Vec<(Terpene, i64)> {
    let mut found: Vec<(usize, usize, Terpene, i64)> = Vec::new();
    for (n, line) in lines.iter().enumerate() {
        for (name, terpene) in TERPENES {
            for at in find_terpene(line, name) {
                let after = skip(line, at + name.chars().count(), &[' ', ':', '-', '=']);
                let Some((milli, end)) = milli_at(&line.chars, after) else {
                    continue;
                };
                let Some((LabelUnit::Percent, scale, _)) = unit_at(line, end) else {
                    continue;
                };
                let terpene = terpene_at(line, at, name, terpene);
                if milli / scale <= 100_000 && !found.iter().any(|f| f.2 == terpene) {
                    found.push((n, at, terpene, milli / scale));
                }
            }
        }
    }
    found.sort_by_key(|f| (f.0, f.1));
    found.into_iter().map(|f| (f.2, f.3)).collect()
}

/// Terpene names may carry a prefix (`β-caryophyllene`, `trans-nerolidol`).
pub(crate) fn find_terpene(line: &Line, name: &str) -> Vec<usize> {
    let pat: Vec<char> = name.chars().collect();
    let n = pat.len();
    (0..=line.lower.len().saturating_sub(n))
        .filter(|&i| {
            line.lower.get(i..i + n) == Some(&pat[..])
                && (i == 0 || !line.lower[i - 1].is_alphabetic())
                && line.lower.get(i + n).is_none_or(|c| !c.is_alphanumeric())
        })
        .collect()
}

fn batch(lines: &[Line]) -> Option<String> {
    for line in lines {
        for keyword in ["batch", "lot"] {
            for at in line.find_word(keyword) {
                let mut i = skip(line, at + keyword.len(), &[' ', ':', '#', '/', '.']);
                // `Batch No.`, `Lot Number`, `Batch ID`, `Batch/Lot`.
                for filler in ["number", "no", "id", "lot", "code"] {
                    if line.lower_from(i, filler.len()) == filler
                        && line
                            .lower
                            .get(i + filler.len())
                            .is_none_or(|c| !c.is_alphanumeric())
                    {
                        i = skip(line, i + filler.len(), &[' ', ':', '#', '.', '/']);
                    }
                }
                let token: String = line.chars[i.min(line.chars.len())..]
                    .iter()
                    .take_while(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
                    .collect();
                let token = token.trim_end_matches(['.', '-']);
                if token.len() >= 2 && token.chars().any(|c| c.is_ascii_digit()) {
                    return Some(token.to_owned());
                }
            }
        }
    }
    None
}

fn package_date(lines: &[Line]) -> Option<Date> {
    for line in lines {
        for keyword in [
            "packaged",
            "package date",
            "pkg date",
            "pkg. date",
            "packed",
            "pkg",
        ] {
            if let Some(&at) = line.find_word(keyword).first()
                && let Some(date) = find_date(&line.rest(at + keyword.chars().count()))
            {
                return Some(date);
            }
        }
    }
    None
}

fn package_tags(text: &str) -> Vec<String> {
    let mut tags: Vec<String> = Vec::new();
    for token in text.split(|c: char| !c.is_ascii_alphanumeric()) {
        if is_package_tag(token) && !tags.iter().any(|t| t == token) {
            tags.push(token.to_owned());
        }
    }
    tags
}

/// Parses recognized label text.
#[must_use]
pub fn parse_label(text: &str) -> LabelScan {
    let lines: Vec<Line> = text.lines().map(Line::new).collect();
    LabelScan {
        thc: cannabinoid(&lines, "thc"),
        cbd: cannabinoid(&lines, "cbd"),
        terpenes: terpenes(&lines),
        batch: batch(&lines),
        package_date: package_date(&lines),
        package_tags: package_tags(text),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn amount(milli: i64, unit: LabelUnit) -> Option<LabelAmountGuess> {
        Some(LabelAmountGuess { milli, unit })
    }

    #[test]
    fn reads_a_flower_label() {
        let label = parse_label(
            "BLUE DREAM  3.5g\n\
             THCA: 25.1%  Total THC: 22.4%\n\
             Total CBD: <0.1%  CBD 0.08 %\n\
             Terpenes: β-Myrcene 0.82%  Limonene 0.41%\n\
             beta-Pinene 0.12% alpha-pinene 0.2 % Caryophyllene 3.1 mg/g\n\
             Batch No.: BD-1042\n\
             Packaged on 09/28/2026\n\
             1A4060300002EE1000012345",
        );
        assert!(label.thc == amount(22_400, LabelUnit::Percent));
        assert!(label.cbd == amount(80, LabelUnit::Percent));
        assert!(
            label.terpenes
                == [
                    (Terpene::Myrcene, 820),
                    (Terpene::Limonene, 410),
                    (Terpene::BetaPinene, 120),
                    (Terpene::AlphaPinene, 200),
                    (Terpene::Caryophyllene, 310),
                ]
        );
        assert_eq!(label.batch.as_deref(), Some("BD-1042"));
        assert_eq!(label.package_date, Date::new(2026, 9, 28));
        assert_eq!(label.package_tags, ["1A4060300002EE1000012345"]);
    }

    #[test]
    fn reads_an_edible_label() {
        let label = parse_label(
            "Mango Gummies\n10mg THC per piece\n100 mg THC total\nLot # 7781A\nPkg Date: Oct 1, 2026",
        );
        assert!(label.thc == amount(10_000, LabelUnit::MgPerUnit));
        assert!(label.cbd.is_none());
        assert_eq!(label.batch.as_deref(), Some("7781A"));
        assert_eq!(label.package_date, Date::new(2026, 10, 1));
    }

    #[test]
    fn number_before_keyword() {
        let label = parse_label("22.4% THC | 0.5% CBD");
        assert!(label.thc == amount(22_400, LabelUnit::Percent));
        assert!(label.cbd == amount(500, LabelUnit::Percent));
    }

    #[test]
    fn ignores_lookalikes() {
        let label = parse_label("THCV 1%\nPilot batchless\nHarvest date 09/01/2026\nTHC: 140%");
        assert!(label.thc.is_none());
        assert_eq!(label.batch, None);
        assert_eq!(label.package_date, None);
    }
}

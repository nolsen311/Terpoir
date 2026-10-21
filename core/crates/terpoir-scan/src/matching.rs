// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

//! Matching a receipt item's name against the user's own products, brands
//! and strains, without an alias list (docs/design-decisions.md, "Simpler
//! review"). Every result is a suggestion: nothing here links on its own.
//!
//! Names are compared after [`normalize_name`] and after folding the usual
//! recognition confusions (`0/O`, `1/I/l`, `5/S`, `8/B`, `rn/m`). In order
//! of strength:
//!
//! | Kind | Example | Score |
//! |---|---|---|
//! | Exact | `BLUE DREAM` → Blue Dream | 100 |
//! | Words | `ACME BLUE DREAM` → Blue Dream (85); `DREAM` → Blue Dream (75) | 75–85 |
//! | Prefix | `BLUE DRE`, `BLU DRE` → Blue Dream (cut off) | 60–70 |
//! | Initials | `BD` → Blue Dream, `GSC` → Girl Scout Cookies | 50 |
//! | Letters | `BDM`, `BLU DRM`, `BLUDRM` → Blue Dream (in order, each word entered at its start) | 20–40 |
//!
//! A product whose brand the text names too gets 2 more points (`ACME BD`
//! puts Acme Blue Dream before the strain Blue Dream). A candidate the
//! user buys at the receipt's shop gets up to 9 more points, enough to order two matches of one kind (BD: Blue Dream or
//! Bubba Diesel) but never to pass a stronger kind. Matches within
//! [`CLOSE`] points of the best are marked close, so the UI offers them
//! as a choice.

use std::collections::HashSet;

use crate::name::normalize_name;
use crate::terms::describe;

/// How far below the best a match may score and still be offered as an
/// equal choice.
pub const CLOSE: u32 = 10;

/// The most points a shop's history adds.
const MAX_BOOST: u32 = 9;

/// Points for a product whose brand the text names too.
const BRAND_NAMED: u32 = 2;

/// How a name matched.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum MatchKind {
    /// The same name.
    Exact,
    /// All of one name's words inside the other.
    Words,
    /// A name cut off: each word the start of the candidate's word.
    Prefix,
    /// The candidate's initials.
    Initials,
    /// The candidate's letters in order, each word entered at its start.
    Letters,
}

impl MatchKind {
    /// `exact`, `words`, `prefix`, `initials` or `letters`.
    #[must_use]
    pub const fn slug(self) -> &'static str {
        match self {
            Self::Exact => "exact",
            Self::Words => "words",
            Self::Prefix => "prefix",
            Self::Initials => "initials",
            Self::Letters => "letters",
        }
    }
}

/// What a candidate is.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CandidateKind {
    /// One of the user's products.
    Product,
    /// One of the user's brands.
    Brand,
    /// One of the user's strains.
    Strain,
}

/// A record to match against.
#[derive(Clone, PartialEq, Eq)]
pub struct NameCandidate {
    /// What it is.
    pub kind: CandidateKind,
    /// Its name.
    pub name: String,
    /// How often the user bought it (or, for a brand or strain, products
    /// of it) at the receipt's shop: purchases there plus "Where I buy it".
    pub at_shop: u32,
}

/// A suggested candidate.
#[derive(Clone, PartialEq, Eq)]
pub struct NameMatch {
    /// Its index in the candidates.
    pub index: usize,
    /// How it matched.
    pub kind: MatchKind,
    /// The match's strength (see the module docs), with the shop boost.
    pub score: u32,
    /// Whether the shop's history added to it.
    pub at_shop: bool,
    /// Within [`CLOSE`] of the best match: offer it as an equal choice.
    pub close: bool,
}

/// Folded words: lower case, letters and digits, recognition confusions
/// folded.
fn fold_words(name: &str) -> Vec<String> {
    normalize_name(name)
        .replace("rn", "m")
        .split(' ')
        .filter(|w| !w.is_empty())
        .map(|w| {
            w.chars()
                .map(|c| match c {
                    '0' => 'o',
                    '1' | 'i' | '|' => 'l',
                    '5' => 's',
                    '8' => 'b',
                    other => other,
                })
                .collect()
        })
        .collect()
}

fn letter_count(words: &[String]) -> usize {
    words.iter().map(|w| w.chars().count()).sum()
}

/// Whether `inner` appears in `outer` as consecutive whole words.
fn contains_words(outer: &[String], inner: &[String]) -> bool {
    !inner.is_empty() && outer.windows(inner.len()).any(|w| w == inner)
}

/// A share of 10 points: how much of the candidate the query covers.
fn coverage(query: usize, candidate: usize) -> u32 {
    u32::try_from((query * 10 / candidate.max(1)).min(10)).unwrap_or(10)
}

/// Scores one query against one name.
#[must_use]
pub fn score_name(query: &str, name: &str) -> Option<(MatchKind, u32)> {
    score_words(&fold_words(query), &fold_words(name))
}

fn score_words(q: &[String], c: &[String]) -> Option<(MatchKind, u32)> {
    let (q_letters, c_letters) = (letter_count(q), letter_count(c));
    if q.is_empty() || c.is_empty() || q_letters < 2 {
        return None;
    }
    if q == c {
        return Some((MatchKind::Exact, 100));
    }
    if c_letters >= 3 && contains_words(q, c) {
        return Some((MatchKind::Words, 85));
    }
    if q_letters >= 3 && contains_words(c, q) {
        return Some((MatchKind::Words, 75));
    }
    // Cut off: each query word starts the candidate's word.
    if q_letters >= 3
        && q.len() <= c.len()
        && q.iter().zip(c).all(|(a, b)| b.starts_with(a.as_str()))
    {
        return Some((MatchKind::Prefix, 60 + coverage(q_letters, c_letters)));
    }
    let joined: String = q.concat();
    if c.len() >= 2
        && joined.chars().count() == c.len()
        && joined.chars().zip(c).all(|(a, w)| w.starts_with(a))
    {
        return Some((MatchKind::Initials, 50));
    }
    if q_letters >= 3
        && let Some(reached) = letters_in_order(q, c)
    {
        let skipped = u32::try_from(c.len() - reached).unwrap_or(u32::MAX);
        let score = (30 + coverage(q_letters, c_letters)).saturating_sub(5 * skipped);
        return Some((MatchKind::Letters, score.max(20)));
    }
    None
}

/// Whether the query's letters appear in the candidate in order, starting
/// with its first letter, entering each candidate word at its first letter
/// and skipping no word but trailing ones; each query word must also start
/// a candidate word. Returns how many candidate words were reached.
fn letters_in_order(q: &[String], c: &[String]) -> Option<usize> {
    // The query as (letter, starts a query word).
    let query: Vec<(char, bool)> = q
        .iter()
        .flat_map(|w| w.chars().enumerate().map(|(i, ch)| (ch, i == 0)))
        .collect();
    let words: Vec<Vec<char>> = c.iter().map(|w| w.chars().collect()).collect();
    let mut failed = HashSet::new();
    // The state: next query letter, current word, next letter in it (0
    // when the word is not entered yet).
    fn walk(
        query: &[(char, bool)],
        words: &[Vec<char>],
        state: (usize, usize, usize),
        failed: &mut HashSet<(usize, usize, usize)>,
    ) -> Option<usize> {
        let (qi, w, ci) = state;
        if qi == query.len() {
            return Some(w + 1);
        }
        if failed.contains(&state) {
            return None;
        }
        let (letter, word_start) = query[qi];
        // Further along the current word (not at a query word's start).
        if ci > 0 && !word_start {
            for (at, &ch) in words[w].iter().enumerate().skip(ci) {
                if ch == letter
                    && let Some(n) = walk(query, words, (qi + 1, w, at + 1), failed)
                {
                    return Some(n);
                }
            }
        }
        // Into the next word, at its first letter.
        let next = if ci == 0 { w } else { w + 1 };
        if words.get(next).and_then(|word| word.first()) == Some(&letter)
            && let Some(n) = walk(query, words, (qi + 1, next, 1), failed)
        {
            return Some(n);
        }
        failed.insert(state);
        None
    }
    walk(&query, &words, (0, 0, 0), &mut failed)
}

/// Removes `inner`'s words from `outer` where they appear together.
fn without_words(outer: &[String], inner: &[String]) -> Vec<String> {
    match outer.windows(inner.len()).position(|w| w == inner) {
        Some(at) if !inner.is_empty() => outer[..at]
            .iter()
            .chain(&outer[at + inner.len()..])
            .cloned()
            .collect(),
        _ => outer.to_vec(),
    }
}

/// Ranks `candidates` against a receipt item's text, best first.
///
/// The text is broken down first ([`crate::describe_item`]), and the name
/// that is left is matched, so `BD PR 1G` and `BD 3.5G HYB` both look for
/// `BD`. The name with its terms kept is tried too (a strain called Hash
/// Plant), and, once a brand is found in the name, the rest of the name
/// without it (`ACME BD` looks for `BD` among strains). Products are also
/// compared by their own names' broken-down form.
#[must_use]
pub fn match_names(text: &str, candidates: &[NameCandidate]) -> Vec<NameMatch> {
    let described = describe(text);
    let mut queries: Vec<Vec<String>> = vec![fold_words(&described.breakdown.name)];
    let with_terms = fold_words(&described.with_terms);
    if with_terms != queries[0] {
        queries.push(with_terms);
    }
    // A brand named in full: also try what's left without it, against
    // strains and against products' names without the brand.
    let leftover = queries[0].clone();
    let mut brands: Vec<Vec<String>> = Vec::new();
    for candidate in candidates.iter().filter(|c| c.kind == CandidateKind::Brand) {
        let brand = fold_words(&candidate.name);
        if letter_count(&brand) >= 3 && contains_words(&leftover, &brand) {
            let rest = without_words(&leftover, &brand);
            if !rest.is_empty() && !queries.contains(&rest) {
                queries.push(rest);
            }
            brands.push(brand);
        }
    }

    let mut found: Vec<NameMatch> = candidates
        .iter()
        .enumerate()
        .filter_map(|(index, candidate)| {
            let mut names = vec![fold_words(&candidate.name)];
            // A product whose brand the text also names.
            let mut brand_named = false;
            if candidate.kind == CandidateKind::Product {
                let plain = fold_words(&describe(&candidate.name).breakdown.name);
                for brand in &brands {
                    let rest = without_words(&plain, brand);
                    if rest.len() < plain.len() {
                        brand_named = true;
                        if !rest.is_empty() {
                            names.push(rest);
                        }
                    }
                }
                if !plain.is_empty() && plain != names[0] {
                    names.push(plain);
                }
            }
            let (kind, score) = queries
                .iter()
                .flat_map(|q| names.iter().filter_map(move |n| score_words(q, n)))
                .max_by_key(|&(kind, score)| (score, std::cmp::Reverse(kind)))?;
            let boost = if candidate.at_shop > 0 {
                (5 + candidate.at_shop.min(4)).min(MAX_BOOST)
            } else {
                0
            };
            Some(NameMatch {
                index,
                kind,
                score: score + boost + if brand_named { BRAND_NAMED } else { 0 },
                at_shop: boost > 0,
                close: false,
            })
        })
        .collect();
    found.sort_by(|a, b| {
        b.score
            .cmp(&a.score)
            .then(a.kind.cmp(&b.kind))
            .then(a.index.cmp(&b.index))
    });
    if let Some(best) = found.first().map(|m| m.score) {
        for m in &mut found {
            m.close = best - m.score <= CLOSE;
        }
    }
    found
}

/// Which of `names` a label names in full: each found name's index, in the
/// order of `names`.
///
/// A name counts when one of the label's lines (or the label's text run
/// together, for a name wrapped over two lines) contains all its words in
/// order, after the same folding as the receipt matcher (`0/O`, `1/l/I`,
/// …). Names under three letters never count. A name whose words lie
/// inside another found name's is dropped (`Dream` when `Blue Dream` is
/// found), so a label for Blue Dream doesn't also suggest Dream.
///
/// Only names in full: initials or a cut-off name on a label are too weak
/// to fill a field (docs/design-decisions.md, "Scan recognition from the
/// journal").
#[must_use]
pub fn names_in_label(text: &str, names: &[&str]) -> Vec<usize> {
    let mut lines: Vec<Vec<String>> = text
        .lines()
        .map(fold_words)
        .filter(|l| !l.is_empty())
        .collect();
    let joined: Vec<String> = lines.concat();
    lines.push(joined);
    let folded: Vec<Vec<String>> = names.iter().map(|n| fold_words(n)).collect();
    let found: Vec<usize> = folded
        .iter()
        .enumerate()
        .filter(|(_, name)| {
            letter_count(name) >= 3 && lines.iter().any(|line| contains_words(line, name))
        })
        .map(|(i, _)| i)
        .collect();
    found
        .iter()
        .copied()
        .filter(|&i| {
            !found.iter().any(|&j| {
                j != i
                    && folded[j].len() > folded[i].len()
                    && contains_words(&folded[j], &folded[i])
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_in_label_need_whole_names() {
        let label = "ACME FARMS\nBLUE DREAM 3.5G\nTHC 22.1%\nBatch 0G-17";
        let names = [
            "Acme Farms",
            "Blue Dream",
            "Dream",
            "Blue",
            "OG Kush",
            "BD",
            "Ac",
        ];
        assert_eq!(names_in_label(label, &names), [0, 1]);
        // Recognition confusions fold, and a wrapped name is found.
        assert_eq!(
            names_in_label("BLUE DRE4M\nB1ue\nDream", &["Blue Dream"]),
            [0]
        );
        assert_eq!(
            names_in_label("GIRL SCOUT\nC00KIES", &["Girl Scout Cookies"]),
            [0]
        );
        assert!(names_in_label("", &["Blue Dream"]).is_empty());
    }

    fn kind(query: &str, name: &str) -> Option<MatchKind> {
        score_name(query, name).map(|(k, _)| k)
    }

    fn candidates(list: &[(CandidateKind, &str, u32)]) -> Vec<NameCandidate> {
        list.iter()
            .map(|&(kind, name, at_shop)| NameCandidate {
                kind,
                name: name.into(),
                at_shop,
            })
            .collect()
    }

    fn names<'a>(found: &[NameMatch], list: &'a [NameCandidate]) -> Vec<&'a str> {
        found.iter().map(|m| list[m.index].name.as_str()).collect()
    }

    #[test]
    fn scores_each_kind() {
        assert_eq!(kind("BLUE DREAM", "Blue Dream"), Some(MatchKind::Exact));
        assert_eq!(
            kind("BLUE DREAM", "Acme Blue Dream"),
            Some(MatchKind::Words)
        );
        assert_eq!(
            kind("ACME BLUE DREAM", "Blue Dream"),
            Some(MatchKind::Words)
        );
        assert_eq!(kind("BLUE DRE", "Blue Dream"), Some(MatchKind::Prefix));
        assert_eq!(kind("BLU DRE", "Blue Dream"), Some(MatchKind::Prefix));
        assert_eq!(kind("BD", "Blue Dream"), Some(MatchKind::Initials));
        assert_eq!(kind("GSC", "Girl Scout Cookies"), Some(MatchKind::Initials));
        assert_eq!(kind("BDM", "Blue Dream"), Some(MatchKind::Letters));
        assert_eq!(kind("BLU DRM", "Blue Dream"), Some(MatchKind::Letters));
        assert_eq!(kind("BLUDRM", "Blue Dream"), Some(MatchKind::Letters));
        assert_eq!(
            kind("GSCOOK", "Girl Scout Cookies"),
            Some(MatchKind::Letters)
        );
    }

    #[test]
    fn rejects_what_does_not_follow_the_name() {
        // Not starting at the first word.
        assert_eq!(kind("DM", "Blue Dream"), None);
        assert_eq!(kind("DRM", "Blue Dream"), None);
        // A word entered in its middle.
        assert_eq!(kind("BRM", "Blue Dream"), None);
        // A query word not at a word start.
        assert_eq!(kind("BL EAM", "Blue Dream"), None);
        // Skipping a middle word.
        assert_eq!(kind("GCK", "Girl Scout Cookies"), None);
        // Too short to mean anything.
        assert_eq!(kind("B", "Blue Dream"), None);
        assert_eq!(kind("", "Blue Dream"), None);
        assert_eq!(kind("BD", "Bluedream"), None);
    }

    #[test]
    fn tolerates_recognition_confusions() {
        assert_eq!(kind("8LUE DREAM", "Blue Dream"), Some(MatchKind::Exact));
        assert_eq!(kind("G5C", "Girl Scout Cookies"), Some(MatchKind::Initials));
        assert_eq!(kind("GELAT0 41", "Gelato 41"), Some(MatchKind::Exact));
        assert_eq!(kind("CORN DOG", "Com Dog"), Some(MatchKind::Exact));
    }

    #[test]
    fn both_abbreviations_suggest_blue_dream() {
        let list = candidates(&[
            (CandidateKind::Strain, "Blue Dream", 0),
            (CandidateKind::Strain, "Girl Scout Cookies", 0),
            (CandidateKind::Strain, "Sour Diesel", 0),
        ]);
        for text in [
            "BD PR 1G",
            "BD 3.5G HYB",
            "BDM 3.5G",
            "BLU DRM FLWR",
            "BLUE DRE 1/8",
        ] {
            let found = match_names(text, &list);
            assert_eq!(names(&found, &list), ["Blue Dream"], "{text}");
        }
        let found = match_names("GSC 1/8 IND", &list);
        assert_eq!(names(&found, &list), ["Girl Scout Cookies"]);
        assert_eq!(found[0].kind, MatchKind::Initials);
    }

    #[test]
    fn ambiguous_initials_are_ranked_by_the_shop() {
        let list = candidates(&[
            (CandidateKind::Strain, "Bubba Diesel", 0),
            (CandidateKind::Strain, "Blue Dream", 3),
        ]);
        let found = match_names("BD 3.5G HYB", &list);
        assert_eq!(names(&found, &list), ["Blue Dream", "Bubba Diesel"]);
        assert!(
            found
                .iter()
                .all(|m| m.close && m.kind == MatchKind::Initials)
        );
        assert!(found[0].at_shop && !found[1].at_shop);
        assert_eq!((found[0].score, found[1].score), (58, 50));

        // Without history, both are equal choices, in the user's order.
        let list = candidates(&[
            (CandidateKind::Strain, "Bubba Diesel", 0),
            (CandidateKind::Strain, "Blue Dream", 0),
        ]);
        let found = match_names("BD PR 1G", &list);
        assert_eq!(names(&found, &list), ["Bubba Diesel", "Blue Dream"]);
        assert!(found.iter().all(|m| m.close));
    }

    #[test]
    fn the_shop_never_beats_a_stronger_kind() {
        let list = candidates(&[
            (CandidateKind::Strain, "Blue Dream", 0),
            (CandidateKind::Strain, "Blue Dream Haze", 9),
        ]);
        let found = match_names("BLUE DREAM", &list);
        assert_eq!(names(&found, &list), ["Blue Dream", "Blue Dream Haze"]);
        assert!(!found[1].close);
    }

    #[test]
    fn finds_the_brand_and_the_strain() {
        let list = candidates(&[
            (CandidateKind::Brand, "Acme", 0),
            (CandidateKind::Strain, "Blue Dream", 0),
            (CandidateKind::Product, "Acme Blue Dream 3.5g", 0),
        ]);
        let found = match_names("ACME BD 3.5G", &list);
        let got: Vec<(&str, MatchKind)> = found
            .iter()
            .map(|m| (list[m.index].name.as_str(), m.kind))
            .collect();
        assert_eq!(
            got,
            [
                ("Acme", MatchKind::Words),
                ("Acme Blue Dream 3.5g", MatchKind::Initials),
                ("Blue Dream", MatchKind::Initials),
            ]
        );
        assert_eq!(found[1].score, 52);
        assert!(found[0].close && !found[1].close && !found[2].close);
        // The product is found by its name without its size.
        let found = match_names("ACME BLUE DREAM 1/8", &list);
        assert_eq!(list[found[0].index].name, "Acme Blue Dream 3.5g");
        assert_eq!(found[0].kind, MatchKind::Exact);
    }

    #[test]
    fn keeps_a_strain_named_with_a_term() {
        let list = candidates(&[(CandidateKind::Strain, "Hash Plant", 0)]);
        let found = match_names("HASH PLANT 1G", &list);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].kind, MatchKind::Exact);
    }

    #[test]
    fn nothing_matches_nothing() {
        let list = candidates(&[(CandidateKind::Strain, "Blue Dream", 0)]);
        assert!(match_names("", &list).is_empty());
        assert!(match_names("3.5G HYB 35.00", &list).is_empty());
        assert!(match_names("ZKITTLEZ", &list).is_empty());
        assert!(match_names("BD", &[]).is_empty());
    }
}

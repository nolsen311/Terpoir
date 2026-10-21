// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

//! Reading a strain's web page that the user shared to Terpoir, as text
//! or as a screenshot already read on the phone (docs/design-decisions.md,
//! "Sharing back what you found"). Strain pages have no common layout, so
//! this is a careful guess: everything it finds is offered as a suggestion
//! the user confirms.
//!
//! - **Terpenes:** named on the lines under a "terpene" heading (the next
//!   few lines), or anywhere when the page has no such heading. With
//!   percentages they are ordered by amount; otherwise in page order. The
//!   dominant one is the one a "dominant terpene" line names, else the
//!   first.
//! - **Type:** the first line naming indica, sativa or hybrid; a line
//!   naming two (an "indica-dominant hybrid") reads as hybrid.
//! - **Parents:** a line about lineage, genetics, parents or a cross, split
//!   on `×` or ` x `.
//! - **Breeder:** a line about the breeder or "bred by".

use terpoir_model::{StrainType, Terpene};

use crate::label::{Line, TERPENES, find_terpene, terpene_at, terpenes};

/// How many lines after a "terpene" heading are read for terpenes.
const TERPENE_LINES: usize = 6;

/// At most this many terpenes, parents.
const MAX_TERPENES: usize = 5;
const MAX_PARENTS: usize = 3;

/// The longest parent or breeder name taken, in characters.
const MAX_NAME_CHARS: usize = 40;

/// What a shared strain page seems to say.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct StrainPage {
    /// Terpenes, the most prominent first.
    pub terpenes: Vec<Terpene>,
    /// The dominant one, if the page shows one.
    pub dominant: Option<Terpene>,
    /// Indica, sativa or hybrid.
    pub strain_type: Option<StrainType>,
    /// Parent strains, by name.
    pub parents: Vec<String>,
    /// Who bred it.
    pub breeder: Option<String>,
}

fn has(line: &str, words: &[&str]) -> bool {
    words.iter().any(|w| line.contains(w))
}

/// Terpenes named on `line`, in order.
fn named_terpenes(line: &Line) -> Vec<(usize, Terpene)> {
    let mut found = Vec::new();
    for (name, terpene) in TERPENES {
        for at in find_terpene(line, name) {
            found.push((at, terpene_at(line, at, name, terpene)));
        }
    }
    found.sort_by_key(|(at, _)| *at);
    found
}

fn read_terpenes(lines: &[Line], lower: &[String]) -> (Vec<Terpene>, Option<Terpene>) {
    let headings: Vec<usize> = lower
        .iter()
        .enumerate()
        .filter(|(_, l)| l.contains("terpene"))
        .map(|(n, _)| n)
        .collect();
    let in_scope = |n: usize| {
        headings.is_empty() || headings.iter().any(|&h| n >= h && n <= h + TERPENE_LINES)
    };
    let scoped: Vec<Line> = lines
        .iter()
        .enumerate()
        .filter(|(n, _)| in_scope(*n))
        .map(|(_, l)| Line::new(&l.chars.iter().collect::<String>()))
        .collect();
    // With amounts: by amount, the largest first.
    let mut amounts = terpenes(&scoped);
    let mut order: Vec<Terpene> = if amounts.len() >= 2 {
        amounts.sort_by_key(|a| std::cmp::Reverse(a.1));
        amounts.into_iter().map(|(t, _)| t).collect()
    } else {
        Vec::new()
    };
    for line in &scoped {
        for (_, t) in named_terpenes(line) {
            if !order.contains(&t) {
                order.push(t);
            }
        }
    }
    order.truncate(MAX_TERPENES);
    let dominant = lines
        .iter()
        .zip(lower)
        .filter(|(_, l)| l.contains("dominant terpene") || l.contains("main terpene"))
        .find_map(|(line, _)| named_terpenes(line).first().map(|(_, t)| t.clone()))
        .or_else(|| order.first().cloned());
    (order, dominant)
}

fn read_type(lower: &[String]) -> Option<StrainType> {
    lower.iter().find_map(|l| {
        let words: Vec<&str> = l
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w| !w.is_empty())
            .collect();
        let indica = words.contains(&"indica");
        let sativa = words.contains(&"sativa");
        let hybrid = words.contains(&"hybrid");
        match (indica, sativa, hybrid) {
            (_, _, true) | (true, true, _) => Some(StrainType::Hybrid),
            (true, false, false) => Some(StrainType::Indica),
            (false, true, false) => Some(StrainType::Sativa),
            _ => None,
        }
    })
}

/// A name as found on a page: trimmed of punctuation, a few words, not too
/// long, with a letter in it.
fn clean_name(text: &str) -> Option<String> {
    let name = text
        .trim()
        .trim_matches(|c: char| !c.is_alphanumeric() && c != '#')
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let words = name.split(' ').count();
    (!name.is_empty()
        && name.chars().count() <= MAX_NAME_CHARS
        && words <= 5
        && name.chars().any(char::is_alphabetic))
    .then_some(name)
}

/// The text after a label (`Lineage: …`), or after `of` / `by` / `from`.
fn after_label<'a>(text: &'a str, lower: &str, markers: &[&str]) -> &'a str {
    if let Some(at) = text.find(':') {
        return &text[at + 1..];
    }
    for marker in markers {
        if let Some(at) = lower.find(marker) {
            return &text[at + marker.len()..];
        }
    }
    text
}

fn read_parents(lines: &[Line], lower: &[String]) -> Vec<String> {
    for (line, l) in lines.iter().zip(lower) {
        let labelled = has(
            l,
            &[
                "lineage",
                "genetics",
                "parents",
                "cross of",
                "crossing",
                "bred from",
            ],
        );
        let text: String = line.chars.iter().collect();
        let rest = if labelled {
            after_label(
                &text,
                l,
                &[
                    "cross of",
                    "crossing",
                    "bred from",
                    "genetics",
                    "lineage",
                    "parents",
                ],
            )
        } else {
            &text
        };
        let parts: Vec<&str> = rest
            .split(['×', '✕'])
            .flat_map(|p| p.split(" x ").flat_map(|q| q.split(" X ")))
            .flat_map(|p| {
                if labelled {
                    p.split(" and ").collect()
                } else {
                    vec![p]
                }
            })
            .collect();
        if parts.len() < 2 && !(labelled && parts.len() == 1 && rest.contains(',')) {
            continue;
        }
        let parts: Vec<&str> = if parts.len() < 2 {
            rest.split(',').collect()
        } else {
            parts
        };
        let names: Vec<String> = parts.iter().filter_map(|p| clean_name(p)).collect();
        if names.len() >= 2 && names.len() == parts.len() {
            return names.into_iter().take(MAX_PARENTS).collect();
        }
    }
    Vec::new()
}

fn read_breeder(lines: &[Line], lower: &[String]) -> Option<String> {
    lines.iter().zip(lower).find_map(|(line, l)| {
        if !has(l, &["breeder", "bred by"]) {
            return None;
        }
        let text: String = line.chars.iter().collect();
        let rest = after_label(&text, l, &["bred by", "breeder"]);
        let first = rest.split(['.', ',', ';', '(', '|']).next().unwrap_or("");
        clean_name(first).filter(|n| !n.eq_ignore_ascii_case("unknown"))
    })
}

/// Reads a shared strain page's text.
#[must_use]
pub fn parse_strain_page(text: &str) -> StrainPage {
    let lines: Vec<Line> = text
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(Line::new)
        .collect();
    let lower: Vec<String> = lines.iter().map(|l| l.lower.iter().collect()).collect();
    let (terpenes, dominant) = read_terpenes(&lines, &lower);
    StrainPage {
        terpenes,
        dominant,
        strain_type: read_type(&lower),
        parents: read_parents(&lines, &lower),
        breeder: read_breeder(&lines, &lower),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_a_typical_page() {
        let page = "Blue Dream\n\
            Hybrid · 18% THC\n\
            Blue Dream is a sativa-leaning hybrid.\n\
            Lineage: Blueberry × Haze\n\
            Breeder: DJ Short (Oregon)\n\
            Top terpenes\n\
            Myrcene  Pinene  Caryophyllene\n\
            Learn more about limonene and our newsletter";
        let read = parse_strain_page(page);
        assert_eq!(read.strain_type, Some(StrainType::Hybrid));
        assert_eq!(read.parents, ["Blueberry", "Haze"]);
        assert_eq!(read.breeder.as_deref(), Some("DJ Short"));
        assert_eq!(
            read.terpenes,
            [
                Terpene::Myrcene,
                Terpene::AlphaPinene,
                Terpene::Caryophyllene,
                Terpene::Limonene
            ]
        );
        assert_eq!(read.dominant, Some(Terpene::Myrcene));
    }

    #[test]
    fn amounts_and_a_dominant_line_decide_the_order() {
        let page = "Terpenes\n\
            Limonene 0.3%\n\
            β-Caryophyllene 0.9%\n\
            Myrcene 0.5%\n\
            Dominant terpene: Caryophyllene\n\
            Indica\n\
            A cross of OG Kush and Durban Poison.\n\
            Bred by Cookie Fam Genetics, California";
        let read = parse_strain_page(page);
        assert_eq!(
            read.terpenes,
            [Terpene::Caryophyllene, Terpene::Myrcene, Terpene::Limonene]
        );
        assert_eq!(read.dominant, Some(Terpene::Caryophyllene));
        assert_eq!(read.strain_type, Some(StrainType::Indica));
        assert_eq!(read.parents, ["OG Kush", "Durban Poison"]);
        assert_eq!(read.breeder.as_deref(), Some("Cookie Fam Genetics"));
    }

    #[test]
    fn says_nothing_it_cannot_find() {
        let read = parse_strain_page("Sign in to see this page.\nBreeder: Unknown");
        assert_eq!(read, StrainPage::default());
        // A long sentence with "x" in it isn't a lineage.
        let read =
            parse_strain_page("Rated 4 x by our reviewers who loved the flavour of this one a lot");
        assert!(read.parents.is_empty());
    }
}

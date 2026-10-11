// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

//! Keyword tag suggestions for stream snippets: the first, default tier of
//! tag extraction (docs/design-decisions.md, "Tag extraction"). A curated
//! vocabulary of phrases per built-in tag, plus the user's custom tag
//! names, matched as whole words. No model, no network; every rule is in
//! this file, so suggestions are auditable.
//!
//! Suggestions are only suggestions: each carries the span of the phrase
//! it came from, and nothing is stored until the user confirms it.
//!
//! Matching: case-insensitive, whole words, with a few English endings on
//! a phrase's last word (`relax` matches `relaxed` and `relaxing`). The
//! longest phrase wins where two overlap; each tag is suggested once, at
//! its first mention. A mention right after a negation (`not anxious`,
//! `no paranoia`, `wasn't sleepy at all`) is skipped.

use terpoir_model::{CoreEffect, Flavor, Mood, TagCategory, TagRef, Terpene};

/// A tag suggested for a span of text.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Suggestion {
    /// The tag.
    pub tag: TagRef,
    /// What kind of tag it is.
    pub category: TagCategory,
    /// Start of the matched phrase, in UTF-16 code units (as platform
    /// strings index).
    pub start: usize,
    /// End of the matched phrase (exclusive), in UTF-16 code units.
    pub end: usize,
}

/// A custom tag the user made, matched by its name.
pub struct CustomTag {
    /// The tag.
    pub tag: TagRef,
    /// Its category.
    pub category: TagCategory,
    /// Its display name.
    pub name: String,
}

/// Endings allowed on a phrase's last word.
const ENDINGS: &[&str] = &["", "s", "es", "d", "ed", "ing", "y"];

/// Words that negate a mention up to this many words later.
const NEGATIONS: &[&str] = &[
    "not", "no", "never", "without", "zero", "isnt", "wasnt", "werent", "didnt", "dont", "doesnt",
    "nor", "hardly", "barely",
];
const NEGATION_REACH: usize = 3;

fn effect_phrases(effect: CoreEffect) -> &'static [&'static str] {
    match effect {
        CoreEffect::Relaxed => &["relaxed", "relax", "chill", "mellow", "unwind", "laid back"],
        CoreEffect::Euphoric => &[
            "euphoric", "euphoria", "giggly", "giggle", "elated", "blissful", "bliss",
        ],
        CoreEffect::Focused => &["focused", "focus", "clear headed", "productive", "alert"],
        CoreEffect::Creative => &["creative", "creativity", "inspired"],
        CoreEffect::Sleepy => &[
            "sleepy",
            "drowsy",
            "knocked out",
            "dozed off",
            "fell asleep",
        ],
        CoreEffect::Hungry => &["hungry", "munchies", "snacky", "hunger"],
        CoreEffect::Anxious => &["anxious", "anxiety", "nervous", "jittery", "on edge"],
        CoreEffect::Paranoid => &["paranoid", "paranoia"],
        CoreEffect::DryMouth => &["dry mouth", "cottonmouth", "cotton mouth"],
        CoreEffect::CouchLock => &[
            "couch lock",
            "couchlock",
            "couch locked",
            "glued to the couch",
        ],
        // A newer core effect without phrases yet; a test requires every
        // built-in tag to have some.
        _ => &[],
    }
}

fn terpene_phrases(terpene: &Terpene) -> &'static [&'static str] {
    match terpene {
        Terpene::Myrcene => &["myrcene"],
        Terpene::Limonene => &["limonene"],
        Terpene::Caryophyllene => &["caryophyllene"],
        Terpene::Linalool => &["linalool"],
        Terpene::AlphaPinene => &["pinene", "alpha pinene", "a pinene"],
        Terpene::BetaPinene => &["beta pinene", "b pinene"],
        Terpene::Humulene => &["humulene"],
        Terpene::Terpinolene => &["terpinolene"],
        Terpene::Ocimene => &["ocimene"],
        Terpene::Bisabolol => &["bisabolol"],
        Terpene::Nerolidol => &["nerolidol"],
        Terpene::Terpineol => &["terpineol"],
        // Unknown slugs and newer variants: no phrases.
        _ => &[],
    }
}

fn flavor_phrases(flavor: &Flavor) -> &'static [&'static str] {
    match flavor {
        Flavor::Citrus => &[
            "citrus",
            "citrusy",
            "lemon",
            "lemony",
            "orange",
            "lime",
            "grapefruit",
        ],
        Flavor::Berry => &["berry", "berries", "blueberry", "strawberry", "grape"],
        Flavor::Sweet => &["sweet", "sugary", "candy"],
        Flavor::Earthy => &["earthy", "musky", "musk"],
        Flavor::Pine => &["pine", "piney", "piny"],
        Flavor::Diesel => &["diesel", "fuel", "gassy"],
        Flavor::Skunk => &["skunk", "skunky", "dank"],
        Flavor::Spicy => &["spicy", "pepper", "peppery"],
        Flavor::Herbal => &["herbal", "herby", "grassy"],
        Flavor::Floral => &["floral", "flowery", "lavender"],
        Flavor::Woody => &["woody", "cedar"],
        Flavor::Minty => &["minty", "mint"],
        // Unknown slugs and newer variants: no phrases.
        _ => &[],
    }
}

fn mood_phrases(mood: &Mood) -> &'static [&'static str] {
    match mood {
        Mood::Happy => &["happy", "cheerful", "good mood"],
        Mood::Calm => &["calm", "peaceful", "at ease"],
        Mood::Content => &["content", "satisfied"],
        Mood::Energetic => &["energetic", "energized", "energised"],
        Mood::Stressed => &["stressed", "stress"],
        Mood::Irritable => &["irritable", "irritated", "grumpy", "annoyed"],
        Mood::Sad => &["sad", "gloomy"],
        Mood::Tired => &["tired", "exhausted", "fatigued"],
        // Unknown slugs and newer variants: no phrases.
        _ => &[],
    }
}

/// Every built-in tag with its phrases.
fn builtin() -> Vec<(TagRef, TagCategory, Vec<String>)> {
    let owned = |p: &[&str]| p.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>();
    let mut all = Vec::new();
    for e in CoreEffect::ALL {
        all.push((e.into(), TagCategory::Effect, owned(effect_phrases(e))));
    }
    for slug in Terpene::SLUGS {
        let t = Terpene::from_slug(slug);
        all.push((
            TagRef::Builtin((*slug).to_owned()),
            TagCategory::Terpene,
            owned(terpene_phrases(&t)),
        ));
    }
    for slug in Flavor::SLUGS {
        let f = Flavor::from_slug(slug);
        all.push((
            TagRef::Builtin((*slug).to_owned()),
            TagCategory::Flavor,
            owned(flavor_phrases(&f)),
        ));
    }
    for slug in Mood::SLUGS {
        let m = Mood::from_slug(slug);
        all.push((
            TagRef::Builtin((*slug).to_owned()),
            TagCategory::Mood,
            owned(mood_phrases(&m)),
        ));
    }
    all
}

/// A word of the text: lower-cased letters and digits (apostrophes
/// dropped, so `wasn't` is `wasnt`), and its UTF-16 span.
struct Word {
    text: String,
    start: usize,
    end: usize,
    /// Which clause it is in: punctuation and line breaks start a new
    /// one, and a negation never reaches past its clause.
    clause: usize,
}

/// Characters that end a clause.
fn ends_clause(c: char) -> bool {
    matches!(
        c,
        '.' | ',' | ';' | ':' | '!' | '?' | '\n' | '(' | ')' | '…'
    )
}

fn words(text: &str) -> Vec<Word> {
    let mut out = Vec::new();
    let mut current: Option<Word> = None;
    let mut pos = 0;
    let mut clause = 0;
    for c in text.chars() {
        let len = c.len_utf16();
        if c.is_alphanumeric() {
            let w = current.get_or_insert(Word {
                text: String::new(),
                start: pos,
                end: pos,
                clause,
            });
            w.text.extend(c.to_lowercase());
            w.end = pos + len;
        } else if (c == '\'' || c == '’') && current.is_some() {
            // Inside a word: keep going, without the apostrophe.
        } else {
            out.extend(current.take());
            if ends_clause(c) {
                clause += 1;
            }
        }
        pos += len;
    }
    out.extend(current);
    out
}

fn phrase_words(phrase: &str) -> Vec<String> {
    words(phrase).into_iter().map(|w| w.text).collect()
}

/// Whether `phrase` matches the words starting at `at`.
fn matches(text: &[Word], at: usize, phrase: &[String]) -> bool {
    let Some(last) = phrase.len().checked_sub(1) else {
        return false;
    };
    phrase.iter().enumerate().all(|(i, p)| {
        let Some(w) = text.get(at + i) else {
            return false;
        };
        if i == last {
            ENDINGS
                .iter()
                .any(|e| w.text.strip_suffix(e) == Some(p.as_str()))
        } else {
            w.text == *p
        }
    })
}

fn negated(text: &[Word], at: usize) -> bool {
    text[at.saturating_sub(NEGATION_REACH)..at]
        .iter()
        .any(|w| w.clause == text[at].clause && NEGATIONS.contains(&w.text.as_str()))
}

/// Suggests tags for `text`: built-in vocabulary plus `custom` tags by
/// name. Ordered by position.
#[must_use]
pub fn suggest(text: &str, custom: &[CustomTag]) -> Vec<Suggestion> {
    let text_words = words(text);
    // Custom tags first: on an equal match, the user's own tag wins over a
    // built-in synonym (a custom "Giggly" over the built-in "euphoric").
    let mut candidates: Vec<(TagRef, TagCategory, Vec<Vec<String>>)> = custom
        .iter()
        .map(|c| {
            (
                c.tag.clone(),
                c.category.clone(),
                vec![phrase_words(&c.name)],
            )
        })
        .filter(|(_, _, p)| !p[0].is_empty())
        .collect();
    candidates.extend(
        builtin().into_iter().map(|(tag, cat, phrases)| {
            (tag, cat, phrases.iter().map(|p| phrase_words(p)).collect())
        }),
    );

    // Every match: (first word, word count, candidate index).
    let mut found: Vec<(usize, usize, usize)> = Vec::new();
    for at in 0..text_words.len() {
        for (i, (_, _, phrases)) in candidates.iter().enumerate() {
            if let Some(len) = phrases
                .iter()
                .filter(|p| matches(&text_words, at, p))
                .map(Vec::len)
                .max()
            {
                found.push((at, len, i));
            }
        }
    }
    // Longest first, then earliest, so overlaps keep the longer phrase.
    found.sort_by_key(|&(at, len, _)| (core::cmp::Reverse(len), at));
    let mut taken = vec![false; text_words.len()];
    let mut chosen: Vec<(usize, usize, usize)> = Vec::new();
    for (at, len, i) in found {
        if taken[at..at + len].iter().any(|t| *t)
            || negated(&text_words, at)
            || chosen
                .iter()
                .any(|&(_, _, j)| candidates[j].0 == candidates[i].0)
        {
            continue;
        }
        taken[at..at + len].iter_mut().for_each(|t| *t = true);
        chosen.push((at, len, i));
    }
    chosen.sort_by_key(|&(at, _, _)| at);
    chosen
        .into_iter()
        .map(|(at, len, i)| Suggestion {
            tag: candidates[i].0.clone(),
            category: candidates[i].1.clone(),
            start: text_words[at].start,
            end: text_words[at + len - 1].end,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    use terpoir_model::RecordId;

    fn slugs(text: &str) -> Vec<String> {
        suggest(text, &[])
            .into_iter()
            .map(|s| match s.tag {
                TagRef::Builtin(slug) => slug,
                TagRef::Custom(id) => id.to_hex(),
            })
            .collect()
    }

    #[test]
    fn finds_effects_flavors_and_moods() {
        assert_eq!(
            slugs("Super relaxing, kinda giggly. Tasted lemony and piney; got the munchies."),
            ["relaxed", "euphoric", "citrus", "pine", "hungry"]
        );
        assert_eq!(
            slugs("Felt calm and creative, a bit tired later"),
            ["calm", "creative", "tired"]
        );
    }

    #[test]
    fn longest_phrase_wins_and_tags_are_suggested_once() {
        // "dry mouth" is one effect, not "dry" + anything; "beta pinene" is
        // beta-pinene, not alpha.
        assert_eq!(slugs("dry mouth, then cottonmouth"), ["dry_mouth"]);
        assert_eq!(slugs("strong beta-pinene nose"), ["beta_pinene"]);
        assert_eq!(slugs("couch-locked all night"), ["couch_lock"]);
    }

    #[test]
    fn negated_mentions_are_skipped() {
        assert!(slugs("not anxious at all, no paranoia").is_empty());
        assert_eq!(slugs("wasn't sleepy, but very focused"), ["focused"]);
        assert_eq!(slugs("Didn't feel hungry"), Vec::<String>::new());
    }

    #[test]
    fn a_negation_ends_with_its_clause() {
        // "piney" is three words after "not", but in the next clause.
        assert_eq!(
            slugs("so relaxed, not anxious, tasted piney"),
            ["relaxed", "pine"]
        );
        assert_eq!(slugs("not anxious. Hungry later"), ["hungry"]);
        // Within one clause it still applies.
        assert!(slugs("not really very anxious").is_empty());
    }

    #[test]
    fn spans_are_utf16() {
        let text = "😀 chill vibes";
        let s = &suggest(text, &[])[0];
        // The emoji is two UTF-16 units, then a space.
        assert_eq!((s.start, s.end), (3, 8));
        let units: Vec<u16> = text.encode_utf16().collect();
        assert_eq!(String::from_utf16(&units[s.start..s.end]).unwrap(), "chill");
    }

    #[test]
    fn whole_words_only() {
        // Words that merely contain a phrase are not mentions of it.
        assert!(slugs("pinecone, spinach, mintage, sweetener").is_empty());
    }

    #[test]
    fn custom_tags_match_by_name() {
        let id = RecordId::from_bytes([7; 16]);
        let custom = [CustomTag {
            tag: TagRef::Custom(id),
            category: TagCategory::Effect,
            name: "Tingly feet".into(),
        }];
        let s = suggest("Had tingly feet for an hour", &custom);
        assert_eq!(s.len(), 1);
        assert!(s[0].tag == TagRef::Custom(id));
        assert_eq!((s[0].start, s[0].end), (4, 15));
    }

    #[test]
    fn a_custom_tag_wins_over_a_builtin_synonym() {
        let id = RecordId::from_bytes([9; 16]);
        let custom = [CustomTag {
            tag: TagRef::Custom(id),
            category: TagCategory::Effect,
            name: "Giggly".into(),
        }];
        let s = suggest("so giggly", &custom);
        assert_eq!(s.len(), 1);
        assert!(s[0].tag == TagRef::Custom(id));
    }

    #[test]
    fn builtin_slugs_are_unique_across_vocabularies() {
        let all = builtin();
        let unique: BTreeSet<_> = all.iter().map(|(t, _, _)| t.clone()).collect();
        assert_eq!(unique.len(), all.len());
        assert!(all.iter().all(|(_, _, p)| !p.is_empty()));
    }
}

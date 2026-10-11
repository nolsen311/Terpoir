// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

//! Name comparison for duplicate detection and shop matching.

/// Lower case, letters and digits only, single spaces: `Blue  Dream (3.5g)`
/// becomes `blue dream 3 5g`.
#[must_use]
pub fn normalize_name(name: &str) -> String {
    name.split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_lowercase)
        .collect::<Vec<_>>()
        .join(" ")
}

/// Whether two names refer to the same thing: equal once normalized, or one
/// contains the other as whole words (a receipt's `Green Leaf` and a saved
/// spot `Green Leaf Dispensary`). Names under three characters never match.
#[must_use]
pub fn names_match(a: &str, b: &str) -> bool {
    let (a, b) = (normalize_name(a), normalize_name(b));
    if a.len() < 3 || b.len() < 3 {
        return false;
    }
    let (short, long) = if a.len() <= b.len() { (a, b) } else { (b, a) };
    format!(" {long} ").contains(&format!(" {short} "))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes() {
        assert_eq!(normalize_name("  Blue  Dream (3.5g) "), "blue dream 3 5g");
        assert_eq!(normalize_name("Café Ñ"), "café ñ");
    }

    #[test]
    fn matches_whole_words() {
        assert!(names_match("BLUE DREAM", "Blue Dream"));
        assert!(names_match("Green Leaf", "GREEN LEAF DISPENSARY"));
        assert!(!names_match("Leaf", "Leafy Greens"));
        assert!(!names_match("ab", "ab"));
        assert!(!names_match("", "Green"));
    }
}

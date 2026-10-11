// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

//! Decoded barcodes and QR codes.

use terpoir_model::IdentifierKind;

/// Longest code kept as an identifier.
const MAX_CODE: usize = 2048;

/// A decoded code, classified.
#[derive(Clone, PartialEq, Eq)]
pub struct ScannedCode {
    /// The code exactly as decoded (trimmed).
    pub code: String,
    /// Retail barcode, QR or package tag.
    pub kind: IdentifierKind,
    /// The code itself when it is a web link, such as a lab-results (COA)
    /// page. Opening it is opt-in: the site learns the user's IP address
    /// and which batch they bought.
    pub link: Option<String>,
}

/// A seed-to-sale package tag: `1A` followed by 22 digits or capital
/// letters (24 characters, as printed by Metrc).
#[must_use]
pub fn is_package_tag(text: &str) -> bool {
    text.len() == 24
        && text.starts_with("1A")
        && text
            .bytes()
            .all(|b| b.is_ascii_digit() || b.is_ascii_uppercase())
}

fn is_link(code: &str) -> bool {
    let lower = code.get(..8).map(str::to_ascii_lowercase);
    let scheme = lower
        .as_deref()
        .is_some_and(|l| l.starts_with("https://") || l.starts_with("http://"));
    scheme && !code.chars().any(char::is_whitespace)
}

/// Classifies a decoded code. `two_dimensional` is true for QR, Data Matrix
/// and similar formats, false for linear barcodes. Returns `None` for
/// empty, oversized or control-character codes.
#[must_use]
pub fn classify_code(raw: &str, two_dimensional: bool) -> Option<ScannedCode> {
    let code = raw.trim();
    if code.is_empty() || code.len() > MAX_CODE || code.chars().any(char::is_control) {
        return None;
    }
    let kind = if is_package_tag(code) {
        IdentifierKind::PackageTag
    } else if two_dimensional {
        IdentifierKind::Qr
    } else {
        IdentifierKind::Barcode
    };
    let link = (two_dimensional && is_link(code)).then(|| code.to_owned());
    Some(ScannedCode {
        code: code.to_owned(),
        kind,
        link,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_codes() {
        let ean = classify_code(" 0123456789012 ", false).unwrap();
        assert_eq!(
            (ean.code.as_str(), ean.kind, ean.link),
            ("0123456789012", IdentifierKind::Barcode, None)
        );

        let tag = classify_code("1A4060300002EE1000012345", false).unwrap();
        assert_eq!(tag.kind, IdentifierKind::PackageTag);

        let coa = classify_code("https://lab.example/coa?batch=BD-1042", true).unwrap();
        assert_eq!(coa.kind, IdentifierKind::Qr);
        assert_eq!(
            coa.link.as_deref(),
            Some("https://lab.example/coa?batch=BD-1042")
        );

        let text = classify_code("HTTPS://x.example", true).unwrap();
        assert!(text.link.is_some());
        assert!(
            classify_code("javascript:alert(1)", true)
                .unwrap()
                .link
                .is_none()
        );
        assert!(
            classify_code("https://a.example/ b", true)
                .unwrap()
                .link
                .is_none()
        );
        // A link in a linear barcode is not offered.
        assert!(
            classify_code("http://x.example", false)
                .unwrap()
                .link
                .is_none()
        );
    }

    #[test]
    fn rejects_bad_codes() {
        assert!(classify_code("   ", false).is_none());
        assert!(classify_code("a\u{0}b", true).is_none());
        assert!(classify_code(&"9".repeat(MAX_CODE + 1), false).is_none());
    }

    #[test]
    fn package_tags() {
        assert!(is_package_tag("1A4FF0100000022000000123"));
        assert!(!is_package_tag("1A4ff0100000022000000123"));
        assert!(!is_package_tag("1A40603000"));
    }
}

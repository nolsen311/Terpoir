// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

//! Address templates for "Look up on the web" (docs/design-decisions.md,
//! "Catalog lookups"). The app fetches nothing: it opens the user's
//! browser on a saved site's address with the strain's name filled in.
//!
//! A template is an `https` address holding one placeholder, which says how
//! the site wants the name written:
//!
//! | Placeholder | Blue Dream #3 becomes |
//! |---|---|
//! | `{strain}` | `Blue%20Dream%20%233` |
//! | `{strain+}` | `Blue+Dream+%233` |
//! | `{strain-}` | `blue-dream-3` |
//!
//! Users rarely write one by hand: [`template_from_example`] makes one
//! from the address of a search they did on the site.

use terpoir_scan::normalize_name;

/// The placeholders, in the order an example address is tried.
const PLACEHOLDERS: [&str; 3] = ["{strain+}", "{strain}", "{strain-}"];

/// The longest template accepted.
pub const MAX_TEMPLATE_CHARS: usize = 1000;

/// Why a template was refused.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TemplateError {
    /// Not an `https://` address with a host.
    NotHttps,
    /// No placeholder, or more than one.
    Placeholder,
    /// Spaces, control characters, or longer than [`MAX_TEMPLATE_CHARS`].
    Malformed,
}

/// Checks a template: `https`, a host the placeholder isn't part of, one
/// placeholder, no spaces or control characters.
///
/// # Errors
///
/// [`TemplateError`] when it can't be used.
pub fn check_template(template: &str) -> Result<(), TemplateError> {
    if template.chars().count() > MAX_TEMPLATE_CHARS
        || template
            .chars()
            .any(|c| c.is_whitespace() || c.is_control())
    {
        return Err(TemplateError::Malformed);
    }
    let Some(rest) = template
        .get(..8)
        .filter(|s| s.eq_ignore_ascii_case("https://"))
        .map(|_| &template[8..])
    else {
        return Err(TemplateError::NotHttps);
    };
    let host_end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let host = &rest[..host_end];
    if host.is_empty()
        || !host
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | ':'))
    {
        return Err(TemplateError::NotHttps);
    }
    let count: usize = PLACEHOLDERS
        .iter()
        .map(|p| template.matches(p).count())
        .sum();
    if count != 1 {
        return Err(TemplateError::Placeholder);
    }
    Ok(())
}

/// Percent-encodes everything but RFC 3986's unreserved characters.
fn encode(text: &str) -> String {
    let mut out = String::new();
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            out.push(char::from(byte));
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

fn words(strain: &str) -> Vec<&str> {
    strain.split_whitespace().collect()
}

/// The name as each placeholder writes it.
fn written(placeholder: &str, strain: &str) -> String {
    match placeholder {
        "{strain+}" => words(strain)
            .iter()
            .map(|w| encode(w))
            .collect::<Vec<_>>()
            .join("+"),
        "{strain-}" => normalize_name(strain).replace(' ', "-"),
        _ => encode(&words(strain).join(" ")),
    }
}

/// The address to open for `strain`, or `None` for a template that fails
/// [`check_template`] or a name with nothing to search for.
#[must_use]
pub fn lookup_address(template: &str, strain: &str) -> Option<String> {
    check_template(template).ok()?;
    if normalize_name(strain).is_empty() {
        return None;
    }
    PLACEHOLDERS
        .iter()
        .find(|p| template.contains(*p))
        .map(|p| template.replacen(p, &written(p, strain), 1))
}

/// A template from the address of a search the user did on the site for
/// `example` (`https://site/search?q=blue+dream` searched for Blue Dream
/// gives `https://site/search?q={strain+}`). The name is found however the
/// site wrote it (`blue+dream`, `Blue%20Dream`, `blue-dream`), ignoring
/// case. `None` when the address doesn't hold the name, or the result fails
/// [`check_template`].
#[must_use]
pub fn template_from_example(address: &str, example: &str) -> Option<String> {
    let address = address.trim();
    if normalize_name(example).is_empty() || address.contains('{') {
        return None;
    }
    let lower = address.to_ascii_lowercase();
    for placeholder in PLACEHOLDERS {
        let form = written(placeholder, example).to_ascii_lowercase();
        if form.is_empty() {
            continue;
        }
        if let Some(at) = lower.find(&form) {
            let template = format!(
                "{}{placeholder}{}",
                &address[..at],
                &address[at + form.len()..]
            );
            if check_template(&template).is_ok() {
                return Some(template);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checks_templates() {
        assert!(
            check_template("https://www.allbud.com/marijuana-strains/search?q={strain+}").is_ok()
        );
        assert!(check_template("https://example.org/strains/{strain-}").is_ok());
        assert_eq!(
            check_template("http://example.org/{strain}"),
            Err(TemplateError::NotHttps)
        );
        assert_eq!(
            check_template("https://{strain}.example.org/"),
            Err(TemplateError::NotHttps)
        );
        assert_eq!(
            check_template("https:///x?q={strain}"),
            Err(TemplateError::NotHttps)
        );
        assert_eq!(
            check_template("https://example.org/"),
            Err(TemplateError::Placeholder)
        );
        assert_eq!(
            check_template("https://example.org/{strain}/{strain-}"),
            Err(TemplateError::Placeholder)
        );
        assert_eq!(
            check_template("https://example.org/a b{strain}"),
            Err(TemplateError::Malformed)
        );
        assert_eq!(
            check_template("javascript:alert(1){strain}"),
            Err(TemplateError::NotHttps)
        );
    }

    #[test]
    fn fills_in_the_name() {
        let name = "Blue Dream #3";
        assert_eq!(
            lookup_address("https://a.example/s?q={strain}", name).as_deref(),
            Some("https://a.example/s?q=Blue%20Dream%20%233")
        );
        assert_eq!(
            lookup_address("https://a.example/s?q={strain+}", name).as_deref(),
            Some("https://a.example/s?q=Blue+Dream+%233")
        );
        assert_eq!(
            lookup_address("https://a.example/strains/{strain-}", name).as_deref(),
            Some("https://a.example/strains/blue-dream-3")
        );
        // Characters that could break out of the address are encoded.
        assert_eq!(
            lookup_address("https://a.example/s?q={strain}", "a&b=c/d").as_deref(),
            Some("https://a.example/s?q=a%26b%3Dc%2Fd")
        );
        assert!(lookup_address("https://a.example/s?q={strain}", " - ").is_none());
        assert!(lookup_address("http://a.example/s?q={strain}", name).is_none());
    }

    #[test]
    fn learns_a_template_from_a_search() {
        assert_eq!(
            template_from_example(
                "https://www.allbud.com/marijuana-strains/search?q=blue+dream",
                "Blue Dream"
            )
            .as_deref(),
            Some("https://www.allbud.com/marijuana-strains/search?q={strain+}")
        );
        assert_eq!(
            template_from_example(
                "https://a.example/search?term=Blue%20Dream&page=1",
                "blue dream"
            )
            .as_deref(),
            Some("https://a.example/search?term={strain}&page=1")
        );
        assert_eq!(
            template_from_example("https://a.example/strains/blue-dream", "Blue Dream").as_deref(),
            Some("https://a.example/strains/{strain-}")
        );
        // A one-word name has no separator to tell the forms apart.
        assert_eq!(
            template_from_example("https://a.example/s?q=Haze", "haze").as_deref(),
            Some("https://a.example/s?q={strain+}")
        );
        assert!(template_from_example("https://a.example/s?q=og+kush", "Blue Dream").is_none());
        assert!(template_from_example("http://a.example/s?q=blue+dream", "Blue Dream").is_none());
        assert!(template_from_example("https://a.example/s?q=blue+dream", "").is_none());
    }
}

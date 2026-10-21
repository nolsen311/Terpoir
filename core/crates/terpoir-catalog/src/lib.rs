// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

//! The offline strain catalog (docs/design-decisions.md, "Catalog lookups").
//!
//! The catalog ships with the app and is searched on the device, so a
//! lookup tells no one what the user is looking at. What it finds is a
//! suggestion: the user confirms it before anything is saved.
//!
//! The data is a tab-separated table (`data/strains-kushy.tsv`, from
//! Kushy's MIT-licensed dataset): a header line, then name, type, breeder
//! and parents (names separated by ` | `). The platform reads the file from
//! its bundled assets and hands the text to [`StrainCatalog::parse`].

#![forbid(unsafe_code)]

use terpoir_model::StrainType;
use terpoir_scan::{MatchKind, normalize_name, score_name};

/// The header the catalog file starts with.
const HEADER: &str = "name\ttype\tbreeder\tparents";

/// Separates parent names in the parents column.
const PARENT_SEPARATOR: &str = " | ";

/// A strain as the catalog knows it.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct CatalogStrain {
    /// Name.
    pub name: String,
    /// Indica or sativa. The catalog never says hybrid: the source marks
    /// nearly every strain so, which says nothing.
    pub strain_type: Option<StrainType>,
    /// Who bred it.
    pub breeder: Option<String>,
    /// Parent strains, by name.
    pub parents: Vec<String>,
}

/// A catalog file that could not be read.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ParseError {
    /// The first line is not the expected header.
    Header,
    /// A line (counting from 1, the header included) has the wrong number
    /// of columns, an empty name, or a type that isn't one.
    Line(usize),
}

/// The offline strain catalog.
#[derive(Clone, Debug)]
pub struct StrainCatalog {
    /// Strains in file order, with their normalized names.
    strains: Vec<(CatalogStrain, String)>,
}

/// A catalog strain found by [`StrainCatalog::search`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CatalogMatch<'a> {
    /// The strain.
    pub strain: &'a CatalogStrain,
    /// How its name matched the query.
    pub kind: MatchKind,
}

impl StrainCatalog {
    /// Reads a catalog file.
    ///
    /// # Errors
    ///
    /// [`ParseError`] when the file isn't a catalog.
    pub fn parse(text: &str) -> Result<Self, ParseError> {
        let mut lines = text.lines();
        if lines.next().map(str::trim_end) != Some(HEADER) {
            return Err(ParseError::Header);
        }
        let mut strains = Vec::new();
        for (index, line) in lines.enumerate() {
            let number = index + 2;
            if line.is_empty() {
                continue;
            }
            let columns: Vec<&str> = line.split('\t').collect();
            let [name, strain_type, breeder, parents] = columns[..] else {
                return Err(ParseError::Line(number));
            };
            let name = name.trim();
            if name.is_empty() {
                return Err(ParseError::Line(number));
            }
            let strain_type = match strain_type.trim() {
                "" => None,
                "indica" => Some(StrainType::Indica),
                "sativa" => Some(StrainType::Sativa),
                _ => return Err(ParseError::Line(number)),
            };
            let breeder = Some(breeder.trim())
                .filter(|b| !b.is_empty())
                .map(str::to_owned);
            let parents = parents
                .split(PARENT_SEPARATOR)
                .map(str::trim)
                .filter(|p| !p.is_empty())
                .map(str::to_owned)
                .collect();
            let strain = CatalogStrain {
                name: name.to_owned(),
                strain_type,
                breeder,
                parents,
            };
            strains.push((strain, normalize_name(name)));
        }
        Ok(Self { strains })
    }

    /// How many strains the catalog holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.strains.len()
    }

    /// Whether the catalog is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.strains.is_empty()
    }

    /// The strain with this name, ignoring case, spacing and punctuation.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&CatalogStrain> {
        let wanted = normalize_name(name);
        if wanted.is_empty() {
            return None;
        }
        self.strains
            .iter()
            .find(|(_, normalized)| *normalized == wanted)
            .map(|(strain, _)| strain)
    }

    /// Up to `limit` strains whose names match `query`, best first, as the
    /// receipt matcher scores names (`terpoir_scan::score_name`: the same
    /// name, whole words, a cut-off name, initials, letters in order).
    /// Equal scores keep the shorter name first, then the catalog's order.
    #[must_use]
    pub fn search(&self, query: &str, limit: usize) -> Vec<CatalogMatch<'_>> {
        if limit == 0 || normalize_name(query).is_empty() {
            return Vec::new();
        }
        let mut found: Vec<(u32, usize, usize, CatalogMatch<'_>)> = self
            .strains
            .iter()
            .enumerate()
            .filter_map(|(order, (strain, normalized))| {
                score_name(query, &strain.name).map(|(kind, score)| {
                    (
                        score,
                        normalized.len(),
                        order,
                        CatalogMatch { strain, kind },
                    )
                })
            })
            .collect();
        found.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));
        found.truncate(limit);
        found.into_iter().map(|(.., m)| m).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "name\ttype\tbreeder\tparents\n\
        Blue Dream\tsativa\t\tBlueberry | Haze\n\
        Blueberry\tindica\tDJ Short\t\n\
        Blue Dragon\t\t\t\n\
        Girl Scout Cookies\t\tCookie Fam Genetics\tOG Kush | Durban Poison\n\
        Haze\tsativa\t\t\n";

    fn names<'a>(found: &'a [CatalogMatch<'a>]) -> Vec<&'a str> {
        found.iter().map(|m| m.strain.name.as_str()).collect()
    }

    #[test]
    fn parses_columns() {
        let catalog = StrainCatalog::parse(SAMPLE).unwrap();
        assert_eq!(catalog.len(), 5);
        let dream = catalog.get("blue  dream").unwrap();
        assert_eq!(dream.strain_type, Some(StrainType::Sativa));
        assert_eq!(dream.breeder, None);
        assert_eq!(dream.parents, ["Blueberry", "Haze"]);
        let gsc = catalog.get("Girl Scout Cookies").unwrap();
        assert_eq!(gsc.strain_type, None);
        assert_eq!(gsc.breeder.as_deref(), Some("Cookie Fam Genetics"));
        assert!(catalog.get("Unknown").is_none());
        assert!(catalog.get(" - ").is_none());
    }

    #[test]
    fn rejects_other_files() {
        assert_eq!(
            StrainCatalog::parse("hello\n").unwrap_err(),
            ParseError::Header
        );
        assert_eq!(
            StrainCatalog::parse("name\ttype\tbreeder\tparents\nA\tsativa\n").unwrap_err(),
            ParseError::Line(2)
        );
        assert_eq!(
            StrainCatalog::parse("name\ttype\tbreeder\tparents\nA\thybrid\t\t\n").unwrap_err(),
            ParseError::Line(2)
        );
        assert_eq!(
            StrainCatalog::parse("name\ttype\tbreeder\tparents\n\t\t\t\n").unwrap_err(),
            ParseError::Line(2)
        );
    }

    #[test]
    fn searches_best_first() {
        let catalog = StrainCatalog::parse(SAMPLE).unwrap();
        let found = catalog.search("Blue Dream", 5);
        assert_eq!(names(&found)[0], "Blue Dream");
        assert_eq!(found[0].kind, MatchKind::Exact);
        // A cut-off name: the shorter of equal matches first.
        assert_eq!(
            names(&catalog.search("blue dr", 5)),
            ["Blue Dream", "Blue Dragon"]
        );
        assert_eq!(names(&catalog.search("GSC", 5)), ["Girl Scout Cookies"]);
        assert_eq!(catalog.search("Blue", 1).len(), 1);
        assert!(catalog.search("", 5).is_empty());
        assert!(catalog.search("Blue", 0).is_empty());
        assert!(catalog.search("zzz", 5).is_empty());
    }

    #[test]
    fn bundled_catalog_reads() {
        let catalog = StrainCatalog::parse(include_str!("../data/strains-kushy.tsv")).unwrap();
        assert!(catalog.len() > 9_000);
        let ak = catalog.get("AK-47").unwrap();
        assert_eq!(ak.breeder.as_deref(), Some("Serious Seeds"));
        // Nothing in the bundled data claims hybrid, and no name repeats.
        let mut seen = std::collections::HashSet::new();
        for (strain, normalized) in &catalog.strains {
            assert_ne!(strain.strain_type, Some(StrainType::Hybrid));
            assert!(seen.insert(normalized.clone()), "{}", strain.name);
        }
    }
}

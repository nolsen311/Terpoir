// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

//! The user's saved lookup sites for "Look up on the web"
//! (docs/design-decisions.md, "Catalog lookups"). One record per journal,
//! kept encrypted and synced like any other, because the sites someone
//! looks strains up on say something about them. Each site's name and
//! address are their own map entries, so edits on two devices merge per
//! site (last writer wins).

use std::collections::BTreeMap;

use super::{Fields, Reader, Record, RecordKind, owns};
use crate::{Document, Error, Result, Value};

/// A saved site: a name and an address template holding a strain
/// placeholder (see `terpoir_catalog::lookup`).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct LookupSite {
    /// What the user calls it ("AllBud").
    pub name: String,
    /// The address with a placeholder for the strain name.
    pub template: String,
}

/// Saved sites by ID (random, made by the core when a site is added).
#[derive(Clone, Default, PartialEq, Eq, Debug)]
pub struct LookupSites {
    /// Sites by ID. A site missing its name or address (a half-merged
    /// edit) is left out when read.
    pub sites: BTreeMap<String, LookupSite>,
}

impl Record for LookupSites {
    const KIND: RecordKind = RecordKind::LookupSites;

    fn to_fields(&self) -> BTreeMap<String, Value> {
        let mut f = Fields::default();
        f.map(
            "site_names",
            self.sites.iter(),
            |id| (*id).clone(),
            |s| Value::Text(s.name.clone()),
        )
        .map(
            "site_templates",
            self.sites.iter(),
            |id| (*id).clone(),
            |s| Value::Text(s.template.clone()),
        );
        f.finish()
    }

    fn from_document(doc: &Document) -> Result<Self> {
        let r = Reader::new::<Self>(doc)?;
        let mut names = BTreeMap::new();
        for (id, value) in r.entries("site_names") {
            match value {
                Value::Text(name) => {
                    names.insert(id.to_owned(), name.clone());
                }
                Value::Null => {}
                _ => return Err(Error::InvalidField("site_names")),
            }
        }
        let mut sites = BTreeMap::new();
        for (id, value) in r.entries("site_templates") {
            match value {
                Value::Text(template) => {
                    if let Some(name) = names.remove(id) {
                        sites.insert(
                            id.to_owned(),
                            LookupSite {
                                name,
                                template: template.clone(),
                            },
                        );
                    }
                }
                Value::Null => {}
                _ => return Err(Error::InvalidField("site_templates")),
            }
        }
        Ok(Self { sites })
    }

    fn owns_field(key: &str) -> bool {
        owns(key, &[], &["site_names", "site_templates"])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DeviceId, Hlc, Stamp};

    fn stamp(wall_ms: u64) -> Stamp {
        Stamp {
            hlc: Hlc {
                wall_ms,
                counter: 0,
            },
            device: DeviceId::from_bytes([1; 16]),
        }
    }

    fn site(name: &str, template: &str) -> LookupSite {
        LookupSite {
            name: name.into(),
            template: template.into(),
        }
    }

    #[test]
    fn round_trip_and_removal() {
        let mut list = LookupSites::default();
        list.sites.insert(
            "a1".into(),
            site("AllBud", "https://www.allbud.com/s?q={strain+}"),
        );
        list.sites
            .insert("b2".into(), site("Mine", "https://example.org/{strain-}"));
        let mut doc = Document::create(&list, stamp(1)).unwrap();
        assert_eq!(LookupSites::from_document(&doc).unwrap(), list);

        list.sites.remove("a1");
        doc.update(&list, stamp(2)).unwrap();
        assert!(doc.value("site_names/a1").is_null());
        assert!(doc.value("site_templates/a1").is_null());
        assert_eq!(LookupSites::from_document(&doc).unwrap(), list);

        doc.update(&LookupSites::default(), stamp(3)).unwrap();
        assert!(LookupSites::from_document(&doc).unwrap().sites.is_empty());
    }
}

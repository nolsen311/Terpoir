// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

//! The offline strain catalog (`terpoir_catalog`), searched on the device.
//! The platform reads the bundled catalog file and hands its text to
//! [`StrainCatalog::open`]; what a search finds is only ever a suggestion.

use std::sync::Arc;

use terpoir_catalog::CatalogStrain;
use terpoir_scan::MatchKind;

use crate::{CoreError, Result};

/// The offline strain catalog.
#[derive(uniffi::Object)]
pub struct StrainCatalog {
    catalog: terpoir_catalog::StrainCatalog,
}

impl core::fmt::Debug for StrainCatalog {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("StrainCatalog(..)")
    }
}

/// A strain from the catalog.
#[derive(Clone, PartialEq, Eq, Debug, uniffi::Record)]
pub struct CatalogStrainData {
    /// Name.
    pub name: String,
    /// `indica` or `sativa`; the catalog never says hybrid.
    pub strain_type: Option<String>,
    /// Who bred it.
    pub breeder: Option<String>,
    /// Parent strains, by name.
    pub parents: Vec<String>,
    /// How the name matched the search: `exact`, `words`, `prefix`,
    /// `initials` or `letters`.
    pub match_kind: String,
}

#[uniffi::export]
impl StrainCatalog {
    /// Reads the bundled catalog file's text.
    ///
    /// # Errors
    ///
    /// `InvalidInput` if the text isn't a catalog.
    #[uniffi::constructor]
    pub fn open(text: String) -> Result<Arc<Self>> {
        crate::init();
        let catalog =
            terpoir_catalog::StrainCatalog::parse(&text).map_err(|_| CoreError::InvalidInput)?;
        Ok(Arc::new(Self { catalog }))
    }

    /// How many strains the catalog holds.
    #[must_use]
    pub fn size(&self) -> u32 {
        u32::try_from(self.catalog.len()).unwrap_or(u32::MAX)
    }

    /// Catalog strains a scanned label names in full, longest name first,
    /// at most `limit`, leaving out names in `exclude` (the journal's own
    /// strains, offered already). Two words or six letters at least.
    #[must_use]
    pub fn find_in_label(
        &self,
        text: String,
        exclude: Vec<String>,
        limit: u32,
    ) -> Vec<CatalogStrainData> {
        self.catalog
            .find_in_label(
                &text,
                &exclude,
                usize::try_from(limit).unwrap_or(usize::MAX),
            )
            .into_iter()
            .map(|s| data(s, MatchKind::Words))
            .collect()
    }

    /// Up to `limit` strains whose names match `query`, best first.
    #[must_use]
    pub fn search(&self, query: String, limit: u32) -> Vec<CatalogStrainData> {
        self.catalog
            .search(&query, usize::try_from(limit).unwrap_or(usize::MAX))
            .into_iter()
            .map(|m| data(m.strain, m.kind))
            .collect()
    }
}

fn data(strain: &CatalogStrain, kind: MatchKind) -> CatalogStrainData {
    CatalogStrainData {
        name: strain.name.clone(),
        strain_type: strain.strain_type.as_ref().map(|t| t.slug().to_owned()),
        breeder: strain.breeder.clone(),
        parents: strain.parents.clone(),
        match_kind: kind.slug().to_owned(),
    }
}

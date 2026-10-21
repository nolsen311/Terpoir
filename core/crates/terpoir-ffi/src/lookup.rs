// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

//! "Look up on the web" (docs/design-decisions.md, "Catalog lookups"): the
//! user's saved lookup sites, one synced record per journal addressed like
//! "What I track", and the addresses the app hands to the browser. The app
//! fetches nothing itself.

use terpoir_catalog::lookup;
use terpoir_crypto::RecordId;
use terpoir_model::{Document, LookupSite, LookupSites, Record, migrate};

use crate::journal::Unlocked;
use crate::tracking::{rotate_singleton, singleton_id};
use crate::{CoreError, Result};

const ID_PURPOSE: &str = "lookup-sites";

/// The longest site name accepted.
const MAX_NAME_CHARS: usize = 100;

/// A saved lookup site.
#[derive(Clone, PartialEq, Eq, Debug, uniffi::Record)]
pub struct LookupSiteData {
    /// Its ID.
    pub id: String,
    /// What the user calls it.
    pub name: String,
    /// Its address template (see `terpoir_catalog::lookup`).
    pub template: String,
}

fn load(u: &Unlocked) -> Result<Option<Document>> {
    let id = singleton_id(&u.master, ID_PURPOSE)?;
    Ok(u.store.get(&u.master, &id)?.map(migrate).transpose()?)
}

fn current(u: &Unlocked) -> Result<LookupSites> {
    Ok(load(u)?
        .map(|doc| LookupSites::from_document(&doc))
        .transpose()?
        .unwrap_or_default())
}

fn save(u: &mut Unlocked, sites: &LookupSites, stamp_ms: u64) -> Result<()> {
    let id = singleton_id(&u.master, ID_PURPOSE)?;
    let mut doc = load(u)?.unwrap_or_else(|| Document::new(id, LookupSites::KIND));
    let stamp = u.clock.stamp(stamp_ms);
    doc.update(sites, stamp)?;
    u.store.put(&u.master, &doc)?;
    u.save_clock()?;
    Ok(())
}

/// The saved sites, by name.
pub(crate) fn list(u: &Unlocked) -> Result<Vec<LookupSiteData>> {
    let mut out: Vec<LookupSiteData> = current(u)?
        .sites
        .into_iter()
        .map(|(id, s)| LookupSiteData {
            id,
            name: s.name,
            template: s.template,
        })
        .collect();
    out.sort_by_key(|a| a.name.to_lowercase());
    Ok(out)
}

/// Adds a site (`id` `None`) or changes one; returns its ID.
pub(crate) fn put(
    u: &mut Unlocked,
    id: Option<String>,
    name: &str,
    template: &str,
    now_ms: u64,
) -> Result<String> {
    let name = name.trim();
    let template = template.trim();
    if name.is_empty()
        || name.chars().count() > MAX_NAME_CHARS
        || name.chars().any(char::is_control)
        || lookup::check_template(template).is_err()
    {
        return Err(CoreError::InvalidInput);
    }
    let mut sites = current(u)?;
    let id = match id {
        Some(id) if sites.sites.contains_key(&id) => id,
        Some(_) => return Err(CoreError::NotFound),
        None => RecordId::random()?.to_hex(),
    };
    sites.sites.insert(
        id.clone(),
        LookupSite {
            name: name.to_owned(),
            template: template.to_owned(),
        },
    );
    save(u, &sites, now_ms)?;
    Ok(id)
}

/// Removes a site; an unknown ID changes nothing.
pub(crate) fn remove(u: &mut Unlocked, id: &str, now_ms: u64) -> Result<()> {
    let mut sites = current(u)?;
    if sites.sites.remove(id).is_some() {
        save(u, &sites, now_ms)?;
    }
    Ok(())
}

/// Removes every site, on every device (the removals sync): the toggle's
/// off-action.
pub(crate) fn forget(u: &mut Unlocked, now_ms: u64) -> Result<()> {
    if load(u)?.is_some() {
        save(u, &LookupSites::default(), now_ms)?;
    }
    Ok(())
}

/// For master-key rotation (see `tracking::rotate`).
pub(crate) fn rotate(u: &mut Unlocked, new_master: &terpoir_crypto::MasterKey) -> Result<()> {
    rotate_singleton(u, new_master, ID_PURPOSE)
}

/// The address to open in the browser for `strain` on a site with
/// `template`, or `None` when the template can't be used or the name has
/// nothing to search for.
#[uniffi::export]
#[must_use]
pub fn lookup_address(template: String, strain: String) -> Option<String> {
    lookup::lookup_address(&template, &strain)
}

/// A template from the address of a search the user did on a site for
/// `example`, or `None` when the address doesn't hold that name.
#[uniffi::export]
#[must_use]
pub fn lookup_template_from_example(address: String, example: String) -> Option<String> {
    lookup::template_from_example(&address, &example)
}

/// Whether `template` can be saved as typed (an `https` address with one
/// placeholder).
#[uniffi::export]
#[must_use]
pub fn lookup_template_valid(template: String) -> bool {
    lookup::check_template(template.trim()).is_ok()
}

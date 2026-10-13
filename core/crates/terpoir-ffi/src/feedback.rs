// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

//! Insight feedback (docs/design-decisions.md, "Insights"): one synced
//! record per journal, addressed by an ID derived from the master key like
//! "What I track". Feedback kept by earlier versions in this device's local
//! metadata moves into the record on unlock.

use std::collections::BTreeMap;

use terpoir_model::{Document, InsightFeedbackList, Record, migrate};

use crate::Result;
use crate::journal::Unlocked;
use crate::tracking::{rotate_singleton, singleton_id};

const ID_PURPOSE: &str = "insight-feedback";

/// Where earlier versions kept feedback, on this device only (`key\tstate`
/// lines).
const LEGACY_META: &str = "insight_feedback";

fn load(u: &Unlocked) -> Result<Option<Document>> {
    let id = singleton_id(&u.master, ID_PURPOSE)?;
    Ok(u.store.get(&u.master, &id)?.map(migrate).transpose()?)
}

fn legacy(u: &Unlocked) -> Result<BTreeMap<String, String>> {
    let bytes = u.store.meta(LEGACY_META)?.unwrap_or_default();
    Ok(String::from_utf8_lossy(&bytes)
        .lines()
        .filter_map(|l| l.split_once('\t'))
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
        .collect())
}

/// State by insight key or category.
pub(crate) fn current(u: &Unlocked) -> Result<BTreeMap<String, String>> {
    let synced = load(u)?
        .map(|doc| InsightFeedbackList::from_document(&doc))
        .transpose()?
        .unwrap_or_default()
        .entries;
    // Not yet moved into the record: still applies.
    let mut all = legacy(u)?;
    all.extend(synced);
    Ok(all)
}

fn save(u: &mut Unlocked, entries: BTreeMap<String, String>, stamp_ms: u64) -> Result<()> {
    let id = singleton_id(&u.master, ID_PURPOSE)?;
    let mut doc = load(u)?.unwrap_or_else(|| Document::new(id, InsightFeedbackList::KIND));
    let stamp = u.clock.stamp(stamp_ms);
    doc.update(&InsightFeedbackList { entries }, stamp)?;
    u.store.put(&u.master, &doc)?;
    u.save_clock()?;
    Ok(())
}

/// Sets the feedback for `key`. Keys are insight keys or categories, which
/// never hold control characters; one that does is ignored.
pub(crate) fn set(u: &mut Unlocked, key: &str, state: &str, now_ms: u64) -> Result<()> {
    if key.is_empty() || key.chars().any(char::is_control) {
        return Ok(());
    }
    let mut entries = current(u)?;
    if entries.get(key).map(String::as_str) == Some(state) {
        return Ok(());
    }
    entries.insert(key.to_owned(), state.to_owned());
    save(u, entries, now_ms)?;
    u.store.delete_meta(LEGACY_META)?;
    Ok(())
}

/// Forgets all feedback, on every device (the removals sync).
pub(crate) fn forget(u: &mut Unlocked, now_ms: u64) -> Result<()> {
    if load(u)?.is_some() {
        save(u, BTreeMap::new(), now_ms)?;
    }
    u.store.delete_meta(LEGACY_META)?;
    Ok(())
}

/// Moves feedback from earlier versions' local metadata into the synced
/// record. Entries already in the record win.
pub(crate) fn migrate_local(u: &mut Unlocked) -> Result<()> {
    if legacy(u)?.is_empty() {
        return Ok(());
    }
    let entries = current(u)?;
    let at = u.clock.last().wall_ms;
    save(u, entries, at)?;
    u.store.delete_meta(LEGACY_META)?;
    Ok(())
}

/// For master-key rotation (see `tracking::rotate`).
pub(crate) fn rotate(u: &mut Unlocked, new_master: &terpoir_crypto::MasterKey) -> Result<()> {
    rotate_singleton(u, new_master, ID_PURPOSE)
}

// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

//! "What I track" (docs/design-decisions.md): the data points the user has
//! hidden, kept in one synced record, and deleting a data point's values
//! from every record.
//!
//! The record's ID is derived from the master key (HKDF with a fixed
//! label), so devices that each create it address the same record and its
//! per-data-point stamps merge. Master-key rotation moves it to the new ID.

use terpoir_crypto::{MasterKey, RecordId};
use terpoir_model::{
    Brand, Document, Product, Purchase, Record, Session, Strain, Timeline, TrackedField,
    TrackingPreferences, migrate,
};

use crate::journal::{Journal, Unlocked};
use crate::{CoreError, Result};

const ID_PURPOSE: &str = "tracking-preferences";

/// The ID of a one-per-journal record under `master`: derived from the
/// master key with a fixed `purpose`, so every device addresses the same
/// record.
pub(crate) fn singleton_id(master: &MasterKey, purpose: &str) -> Result<RecordId> {
    let key = master.derive_subkey(purpose)?;
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&key.expose_secret()[..16]);
    Ok(RecordId::from_bytes(bytes))
}

/// The tracking-preferences record's ID under `master`.
pub(crate) fn record_id(master: &MasterKey) -> Result<RecordId> {
    singleton_id(master, ID_PURPOSE)
}

fn load(u: &Unlocked) -> Result<Option<Document>> {
    let id = record_id(&u.master)?;
    Ok(u.store.get(&u.master, &id)?.map(migrate).transpose()?)
}

pub(crate) fn preferences(u: &Unlocked) -> Result<TrackingPreferences> {
    load(u)?
        .map(|doc| TrackingPreferences::from_document(&doc))
        .transpose()
        .map(Option::unwrap_or_default)
        .map_err(Into::into)
}

/// For master-key rotation, after the store is re-wrapped to `new_master`
/// and before the journal switches to it: the record moves to the ID
/// derived from the new key, fields and stamps unchanged, and the old ID is
/// deleted.
pub(crate) fn rotate(u: &mut Unlocked, new_master: &MasterKey) -> Result<()> {
    rotate_singleton(u, new_master, ID_PURPOSE)
}

/// [`rotate`] for any one-per-journal record (see [`singleton_id`]).
pub(crate) fn rotate_singleton(
    u: &mut Unlocked,
    new_master: &MasterKey,
    purpose: &str,
) -> Result<()> {
    let old_id = singleton_id(&u.master, purpose)?;
    let Some(old) = u.store.get(new_master, &old_id)?.map(migrate).transpose()? else {
        return Ok(());
    };
    let mut moved = Document::new(singleton_id(new_master, purpose)?, old.kind());
    for (key, field) in old.fields() {
        moved.insert_field(key.clone(), field.clone());
    }
    u.store.put(new_master, &moved)?;
    u.store.delete(&old.id())?;
    Ok(())
}

fn parse(slug: &str) -> Result<TrackedField> {
    match TrackedField::from_slug(slug) {
        TrackedField::Unknown(_) => Err(CoreError::InvalidInput),
        field => Ok(field),
    }
}

/// Applies `edit` to every record of type `R`, saving those it changed.
/// Returns how many changed.
fn edit_all<R: Record + Clone + PartialEq>(
    u: &mut Unlocked,
    now_ms: u64,
    mut edit: impl FnMut(&mut R),
) -> Result<u32> {
    let mut changed = 0u32;
    for id in u.store.ids_of_kind(R::KIND)? {
        let Some(record) = u.store.get_record::<R>(&u.master, &id)? else {
            continue;
        };
        let mut edited = record.clone();
        edit(&mut edited);
        if edited != record {
            u.save(Some(id.to_hex()), &edited, now_ms)?;
            changed = changed.saturating_add(1);
        }
    }
    Ok(changed)
}

/// Deletes every record of `kind`. Returns how many.
fn delete_all<R: Record>(u: &mut Unlocked) -> Result<u32> {
    let mut deleted = 0u32;
    for id in u.store.ids_of_kind(R::KIND)? {
        if u.store.delete(&id)? {
            deleted = deleted.saturating_add(1);
        }
    }
    Ok(deleted)
}

fn delete_values(u: &mut Unlocked, field: &TrackedField, now_ms: u64) -> Result<u32> {
    let n = match field {
        TrackedField::Brand => {
            edit_all(u, now_ms, |p: &mut Product| p.brand = None)? + delete_all::<Brand>(u)?
        }
        TrackedField::Strain => {
            edit_all(u, now_ms, |p: &mut Product| p.strains.clear())? + delete_all::<Strain>(u)?
        }
        TrackedField::PriceAndShop => {
            edit_all(u, now_ms, |p: &mut Purchase| {
                p.spot = None;
                for line in p.lines.values_mut() {
                    line.price = None;
                }
            })? + edit_all(u, now_ms, |s: &mut Session| {
                for line in s.lines.values_mut() {
                    line.shop = None;
                }
            })? + edit_all(u, now_ms, |p: &mut Product| p.shops.clear())?
        }
        // Purchases go as a whole, and with them prices and shops.
        TrackedField::Purchases => {
            edit_all(u, now_ms, |s: &mut Session| {
                for line in s.lines.values_mut() {
                    line.purchase = None;
                }
            })? + delete_all::<Purchase>(u)?
                + delete_values(u, &TrackedField::PriceAndShop, now_ms)?
        }
        TrackedField::Potency => edit_all(u, now_ms, |p: &mut Product| {
            p.thc = None;
            p.cbd = None;
        })?,
        TrackedField::Terpenes => {
            edit_all(u, now_ms, |p: &mut Product| p.terpenes.clear())?
                + edit_all(u, now_ms, |s: &mut Strain| s.terpenes.clear())?
        }
        TrackedField::Batch => {
            edit_all(u, now_ms, |p: &mut Product| {
                p.batch = None;
                p.package_date = None;
            })? + edit_all(u, now_ms, |p: &mut Purchase| {
                for line in p.lines.values_mut() {
                    line.batch = None;
                }
            })?
        }
        TrackedField::Dose => edit_all(u, now_ms, |s: &mut Session| {
            for line in s.lines.values_mut() {
                line.dose = None;
            }
        })?,
        TrackedField::Timeline => edit_all(u, now_ms, |s: &mut Session| {
            s.timeline = Timeline::default()
        })?,
        TrackedField::Mood => edit_all(u, now_ms, |s: &mut Session| {
            s.mood_before = None;
            s.mood_after = None;
        })?,
        TrackedField::Intention => edit_all(u, now_ms, |s: &mut Session| s.intention = None)?,
        TrackedField::Social => edit_all(u, now_ms, |s: &mut Session| s.social = None)?,
        _ => return Err(CoreError::InvalidInput),
    };
    Ok(n)
}

/// Every data point the user can hide, in display order.
#[uniffi::export]
#[must_use]
pub fn trackable_data_points() -> Vec<String> {
    TrackedField::SLUGS
        .iter()
        .map(|s| (*s).to_owned())
        .collect()
}

#[uniffi::export]
impl Journal {
    /// The data points the user has hidden (from `trackable_data_points`;
    /// hiding purchases also hides `price_and_shop`). Empty by default.
    ///
    /// # Errors
    ///
    /// `Locked` or `Storage`.
    pub fn hidden_data_points(&self) -> Result<Vec<String>> {
        self.with(|u| {
            let prefs = preferences(u)?;
            Ok(TrackedField::all()
                .into_iter()
                .filter(|f| !prefs.tracks(f))
                .map(|f| f.slug().to_owned())
                .collect())
        })
    }

    /// Shows (`tracked`) or hides a data point. Hiding keeps its values,
    /// unchanged; `delete_data_point` removes them. The choice syncs.
    ///
    /// # Errors
    ///
    /// `InvalidInput` for an unknown data point; `Locked` or `Storage`.
    pub fn set_data_point_tracked(
        &self,
        data_point: String,
        tracked: bool,
        now_ms: u64,
    ) -> Result<()> {
        let field = parse(&data_point)?;
        self.with(|u| {
            let id = record_id(&u.master)?;
            let mut prefs = preferences(u)?;
            let changed = if tracked {
                prefs.hidden.remove(&field)
            } else {
                prefs.hidden.insert(field)
            };
            if !changed {
                return Ok(());
            }
            let stamp = u.clock.stamp(now_ms);
            let mut doc = load(u)?.unwrap_or_else(|| Document::new(id, TrackingPreferences::KIND));
            doc.update(&prefs, stamp)?;
            u.store.put(&u.master, &doc)?;
            u.save_clock()?;
            Ok(())
        })
    }

    /// Deletes a data point's values from every record (for "Delete" when
    /// it is turned off): fields are cleared, and brands, strains or
    /// purchases as records are deleted (crypto-erased). The removals sync
    /// like edits. Returns how many records changed or were deleted.
    ///
    /// # Errors
    ///
    /// `InvalidInput` for an unknown data point; `Locked` or `Storage`.
    pub fn delete_data_point(&self, data_point: String, now_ms: u64) -> Result<u32> {
        let field = parse(&data_point)?;
        self.with(|u| delete_values(u, &field, now_ms))
    }
}

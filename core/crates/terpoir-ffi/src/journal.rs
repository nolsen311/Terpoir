// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

//! The unlocked journal.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard};
use terpoir_crypto::{MasterKey, RecoveryKey, WrappedKey};
use terpoir_model::{
    Brand, DeviceId, Document, Form, Hlc, Location, Product, Purchase, Record, RecordId,
    RecordKind, Session, Snippet, Spot, Strain, Tag, TagCategory, TagRef, TrackingPreferences,
    migrate, receipt_name_key, usual_method,
};
use terpoir_scan::{names_match, normalize_name};
use terpoir_store::Store;
use terpoir_sync::{Clock, apply_pending, export_blobs, import_blob};
use terpoir_tagging::CustomTag;
use zeroize::Zeroizing;

use crate::insights::{self, InsightData, InsightFeedback, ShopInsightData, TimelineInsightData};
use crate::keys::RotatedKeys;
use crate::plaintext::{self, ExportFile};
use crate::records::{
    BrandData, ProductData, PurchaseData, SessionData, SnippetData, SpotData, StrainData, TagData,
    TagRefData, parse_id,
};
use crate::{CoreError, Result};

const META_DEVICE: &str = "device_id";
const META_CLOCK: &str = "clock";
const META_SEQUENCE: &str = "sync_sequence";
const META_MAPTILER_KEY: &str = "maptiler_key";
/// Longest MapTiler key accepted. Real keys are 20 characters.
const MAX_MAPTILER_KEY_LEN: usize = 128;

/// An existing product that may be the one being entered.
#[derive(Clone, PartialEq, Eq, Debug, uniffi::Record)]
pub struct ProductMatch {
    /// The product's ID.
    pub id: String,
    /// True when a scanned code matched (a sure match); false for a name
    /// match ("Same product, new batch?").
    pub by_code: bool,
}

/// A tag suggested for part of a snippet's text (keyword matching).
#[derive(Clone, PartialEq, Eq, uniffi::Record)]
pub struct TagSuggestion {
    /// The tag.
    pub tag: TagRefData,
    /// `effect`, `terpene`, `flavor` or `mood`.
    pub category: String,
    /// Start of the phrase it came from, in UTF-16 code units.
    pub start: u32,
    /// End of that phrase (exclusive), in UTF-16 code units.
    pub end: u32,
}

/// One of the user's products, brands or strains suggested for a receipt
/// item. Always a suggestion ("?"): nothing links on its own.
#[derive(Clone, PartialEq, Eq, uniffi::Record)]
pub struct ScanSuggestion {
    /// The record's ID.
    pub id: String,
    /// `product`, `brand` or `strain`.
    pub record_type: String,
    /// Its name.
    pub name: String,
    /// How it matched: `exact`, `words`, `prefix`, `initials` or
    /// `letters` (strongest first).
    pub match_kind: String,
    /// The match's strength, with the shop's boost; for ranking only.
    pub score: u32,
    /// Whether the user's purchases at the receipt's shop raised it.
    pub at_shop: bool,
    /// Within a few points of the best suggestion: offer it as an equal
    /// choice (BD: Blue Dream or Bubba Diesel).
    pub close: bool,
    /// A learned receipt name (since 0.6.0): the line's text is exactly
    /// one this product learned (`Journal::learn_receipt_name`), so it
    /// links automatically, shown with a "matched" chip that one tap
    /// changes or undoes. Learned products come first, with `match_kind`
    /// `learned`; when two products learned the same text (possible
    /// after a sync), both are `close`, to be offered as equal choices
    /// instead. Every other suggestion is a "?". Last, with a default, so
    /// callers that predate it compile.
    #[uniffi(default = false)]
    pub learned: bool,
}

/// The score of a learned receipt name, above any ranked suggestion.
const LEARNED_SCORE: u32 = 1_000;

/// The saved spot a receipt's header most likely belongs to, and what the
/// header says.
#[derive(Clone, PartialEq, Eq, uniffi::Record)]
pub struct ReceiptShopMatch {
    /// The spot's ID, if one matched.
    pub spot: Option<String>,
    /// `phone`, `licence` or `name`.
    pub kind: Option<String>,
    /// A phone or licence number match ("Green Leaf ✓"); false for a name
    /// only ("Green Leaf?").
    pub strong: bool,
    /// The header row that matched.
    pub row: Option<u32>,
    /// The header row that looks like the shop's name.
    pub shop: Option<String>,
    /// That row.
    pub shop_row: Option<u32>,
    /// Phone numbers printed in the header, digits only.
    pub phones: Vec<String>,
    /// Licence numbers printed in the header, letters and digits only.
    pub licences: Vec<String>,
}

/// A saved spot near a location fix.
#[derive(Clone, PartialEq, Eq, uniffi::Record)]
pub struct NearbySpot {
    /// The spot's ID.
    pub id: String,
    /// Its name.
    pub name: String,
    /// Distance from the fix, in whole metres.
    pub distance_m: u32,
    /// Whether it is a Re-up Spot, so "Show me" can offer logging a
    /// purchase.
    pub re_up: bool,
}

/// A Re-up Spot to watch for purchase prompts: where, and nothing else.
#[derive(Clone, PartialEq, Eq, uniffi::Record)]
pub struct PromptFence {
    /// Latitude in units of 1e-7 degrees.
    pub lat_e7: i32,
    /// Longitude in units of 1e-7 degrees.
    pub lon_e7: i32,
    /// Geofence radius in metres (the spot's own, at least the minimum).
    pub radius_m: u32,
}

/// A spot with coordinates, to watch for "Notify me", and what an alert
/// about it may show at the user's notification tier.
#[derive(Clone, PartialEq, Eq, uniffi::Record)]
pub struct SpotAlert {
    /// Latitude in units of 1e-7 degrees.
    pub lat_e7: i32,
    /// Longitude in units of 1e-7 degrees.
    pub lon_e7: i32,
    /// Geofence radius in metres (the spot's own, at least the minimum).
    pub radius_m: u32,
    /// The spot's name (Named and Rich tiers).
    pub name: String,
    /// Sessions logged here (Rich tier).
    pub sessions: u32,
    /// Average rating of those sessions, in tenths of a star, if any were
    /// rated (Rich tier).
    pub average_rating_tenths: Option<u32>,
    /// The product of the latest session here, if recorded (Rich tier).
    pub last_product: Option<String>,
}

/// Record types, for listing.
#[derive(Clone, Copy, PartialEq, Eq, Debug, uniffi::Enum)]
pub enum RecordType {
    /// Sessions.
    Session,
    /// Products.
    Product,
    /// Strains.
    Strain,
    /// Brands.
    Brand,
    /// Spots.
    Spot,
    /// Stream snippets.
    Snippet,
    /// Custom tags.
    Tag,
    /// Purchases.
    Purchase,
}

impl From<RecordType> for RecordKind {
    fn from(t: RecordType) -> Self {
        match t {
            RecordType::Session => Self::Session,
            RecordType::Product => Self::Product,
            RecordType::Strain => Self::Strain,
            RecordType::Brand => Self::Brand,
            RecordType::Spot => Self::Spot,
            RecordType::Snippet => Self::Snippet,
            RecordType::Tag => Self::Tag,
            RecordType::Purchase => Self::Purchase,
        }
    }
}

/// An encrypted sync blob to upload under `name`.
#[derive(uniffi::Record)]
pub struct SyncBlob {
    /// Random file name (32 hex characters).
    pub name: String,
    /// Encrypted contents.
    pub bytes: Vec<u8>,
}

/// What importing a sync blob did.
#[derive(Clone, Copy, PartialEq, Eq, Debug, uniffi::Record)]
pub struct ImportResult {
    /// Records added or merged.
    pub records: u64,
    /// Deletions applied.
    pub deletions: u64,
    /// Records from a newer app version. They are kept sealed, carried in
    /// exports, and applied by `apply_pending_records` after updating, so
    /// the blob need not be imported again.
    pub deferred: u64,
}

pub(crate) struct Unlocked {
    pub(crate) store: Store,
    pub(crate) master: MasterKey,
    pub(crate) clock: Clock,
    wrapped_database_key: WrappedKey,
}

impl plaintext::Source for Unlocked {
    fn ids(&self, kind: RecordKind) -> Result<Vec<RecordId>> {
        Ok(self.store.ids_of_kind(kind)?)
    }

    fn document(&self, id: &RecordId) -> Result<Option<Document>> {
        self.store
            .get(&self.master, id)?
            .map(migrate)
            .transpose()
            .map_err(Into::into)
    }
}

impl insights::Source for Unlocked {
    fn sessions(&self) -> Result<Vec<(RecordId, Session)>> {
        let mut out = Vec::new();
        for id in self.store.ids_of_kind(RecordKind::Session)? {
            if let Some(session) = self.store.get_record::<Session>(&self.master, &id)? {
                out.push((id, session));
            }
        }
        Ok(out)
    }

    fn product(&self, id: &RecordId) -> Result<Option<Product>> {
        Ok(self.store.get_record::<Product>(&self.master, id)?)
    }

    fn strain(&self, id: &RecordId) -> Result<Option<Strain>> {
        Ok(self.store.get_record::<Strain>(&self.master, id)?)
    }

    fn strains(&self) -> Result<Vec<(RecordId, Strain)>> {
        let mut out = Vec::new();
        for id in self.store.ids_of_kind(RecordKind::Strain)? {
            if let Some(strain) = self.store.get_record::<Strain>(&self.master, &id)? {
                out.push((id, strain));
            }
        }
        Ok(out)
    }

    fn tag_name(&self, id: &RecordId) -> Result<Option<String>> {
        Ok(self
            .store
            .get_record::<Tag>(&self.master, id)?
            .map(|t| t.name))
    }

    fn purchases(&self) -> Result<Vec<(RecordId, Purchase)>> {
        let mut out = Vec::new();
        for id in self.store.ids_of_kind(RecordKind::Purchase)? {
            if let Some(purchase) = self.store.get_record::<Purchase>(&self.master, &id)? {
                out.push((id, purchase));
            }
        }
        Ok(out)
    }

    fn spot(&self, id: &RecordId) -> Result<Option<Spot>> {
        Ok(self.store.get_record::<Spot>(&self.master, id)?)
    }

    fn brand_name(&self, id: &RecordId) -> Result<Option<String>> {
        Ok(self
            .store
            .get_record::<Brand>(&self.master, id)?
            .map(|b| b.name))
    }

    fn tracking(&self) -> Result<TrackingPreferences> {
        crate::tracking::preferences(self)
    }

    fn feedback(&self) -> Result<BTreeMap<String, String>> {
        crate::feedback::current(self)
    }
}

impl Unlocked {
    pub(crate) fn save_clock(&mut self) -> Result<()> {
        let last = self.clock.last();
        let mut bytes = [0u8; 12];
        bytes[..8].copy_from_slice(&last.wall_ms.to_be_bytes());
        bytes[8..].copy_from_slice(&last.counter.to_be_bytes());
        self.store.set_meta(META_CLOCK, &bytes)?;
        Ok(())
    }

    pub(crate) fn save<R: Record>(
        &mut self,
        id: Option<String>,
        record: &R,
        now_ms: u64,
    ) -> Result<String> {
        let stamp = self.clock.stamp(now_ms);
        let doc = match id {
            Some(id) => {
                let id = parse_id(&id)?;
                let mut doc = migrate(
                    self.store
                        .get(&self.master, &id)?
                        .ok_or(CoreError::NotFound)?,
                )?;
                doc.update(record, stamp)?;
                doc
            }
            None => Document::create(record, stamp)?,
        };
        self.store.put(&self.master, &doc)?;
        self.save_clock()?;
        Ok(doc.id().to_hex())
    }

    fn get<R: Record>(&self, id: &str) -> Result<Option<R>> {
        Ok(self.store.get_record::<R>(&self.master, &parse_id(id)?)?)
    }
}

/// An unlocked journal. [`Journal::lock`] wipes the master key and closes
/// the database; every later call fails with `Locked` until the app opens
/// the journal again.
#[derive(uniffi::Object)]
pub struct Journal {
    state: Mutex<Option<Unlocked>>,
}

impl core::fmt::Debug for Journal {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("Journal(..)")
    }
}

impl Journal {
    fn unlocked(&self) -> Result<MutexGuard<'_, Option<Unlocked>>> {
        self.state.lock().map_err(|_| CoreError::Internal)
    }

    pub(crate) fn with<T>(&self, f: impl FnOnce(&mut Unlocked) -> Result<T>) -> Result<T> {
        let mut guard = self.unlocked()?;
        let unlocked = guard.as_mut().ok_or(CoreError::Locked)?;
        f(unlocked)
    }
}

fn ids(list: Vec<RecordId>) -> Vec<String> {
    list.iter().map(RecordId::to_hex).collect()
}

#[uniffi::export]
impl Journal {
    /// Opens (or creates) the journal database and unlocks it.
    /// `master_key` comes from the platform keystore (or recovery /
    /// passphrase); wipe the array after this call.
    ///
    /// # Errors
    ///
    /// `WrongKey` if the master key does not match; `NewerVersion` or
    /// `Storage` otherwise.
    #[uniffi::constructor]
    pub fn open(
        db_path: String,
        files_dir: String,
        master_key: Vec<u8>,
        wrapped_database_key: Vec<u8>,
    ) -> Result<Arc<Self>> {
        crate::init();
        let master_key = Zeroizing::new(master_key);
        let master = MasterKey::from_slice(&master_key)?;
        let wrapped_database_key = WrappedKey::from_slice(&wrapped_database_key)?;
        let database_key = master.unwrap_database_key(&wrapped_database_key)?;
        let mut store = Store::open(Path::new(&db_path), Path::new(&files_dir), &database_key)?;

        let device = match store.meta(META_DEVICE)? {
            Some(bytes) => DeviceId::from_bytes(bytes.try_into().map_err(|_| CoreError::Internal)?),
            None => {
                let device = DeviceId::random()?;
                store.set_meta(META_DEVICE, device.as_bytes())?;
                device
            }
        };
        let last = match store.meta(META_CLOCK)? {
            Some(bytes) if bytes.len() == 12 => Hlc {
                wall_ms: u64::from_be_bytes(
                    bytes[..8].try_into().map_err(|_| CoreError::Internal)?,
                ),
                counter: u32::from_be_bytes(
                    bytes[8..].try_into().map_err(|_| CoreError::Internal)?,
                ),
            },
            _ => Hlc::default(),
        };
        let journal = Arc::new(Self {
            state: Mutex::new(Some(Unlocked {
                store,
                master,
                clock: Clock::new(device, last),
                wrapped_database_key,
            })),
        });
        // An index built by an older app's rules is rebuilt with this
        // version's. A failure must not keep the journal locked either: the
        // old index still answers, and the rebuild is retried next unlock.
        let _ = journal.with(|u| Ok(u.store.refresh_index(&u.master)?));
        // Feedback earlier versions kept on this device joins the synced
        // record. A failure must not keep the journal locked: it is retried
        // on the next unlock, and the feedback applies meanwhile.
        let _ = journal.with(crate::feedback::migrate_local);
        Ok(journal)
    }

    /// Locks the journal: wipes the master key from memory and closes the
    /// database.
    pub fn lock(&self) {
        if let Ok(mut guard) = self.state.lock() {
            *guard = None;
        }
    }

    /// Whether [`Journal::lock`] has been called.
    #[must_use]
    pub fn is_locked(&self) -> bool {
        self.state.lock().map_or(true, |g| g.is_none())
    }

    /// Creates (`id` = `None`) or updates a session. Returns its ID.
    ///
    /// # Errors
    ///
    /// `InvalidInput`, `NotFound`, `Locked` or `Storage`.
    pub fn save_session(
        &self,
        id: Option<String>,
        session: SessionData,
        now_ms: u64,
    ) -> Result<String> {
        let record = session.to_model()?;
        self.with(|u| u.save(id, &record, now_ms))
    }

    /// A session, if it exists.
    ///
    /// # Errors
    ///
    /// `WrongType` if `id` is another kind of record.
    pub fn get_session(&self, id: String) -> Result<Option<SessionData>> {
        self.with(|u| Ok(u.get::<Session>(&id)?.as_ref().map(SessionData::from_model)))
    }

    /// Creates or updates a product. Returns its ID.
    ///
    /// # Errors
    ///
    /// `InvalidInput`, `NotFound`, `Locked` or `Storage`.
    pub fn save_product(
        &self,
        id: Option<String>,
        product: ProductData,
        now_ms: u64,
    ) -> Result<String> {
        let record = product.to_model()?;
        self.with(|u| u.save(id, &record, now_ms))
    }

    /// A product, if it exists.
    ///
    /// # Errors
    ///
    /// `WrongType` if `id` is another kind of record.
    pub fn get_product(&self, id: String) -> Result<Option<ProductData>> {
        self.with(|u| Ok(u.get::<Product>(&id)?.as_ref().map(ProductData::from_model)))
    }

    /// Creates or updates a strain. Returns its ID.
    ///
    /// # Errors
    ///
    /// `InvalidInput`, `NotFound`, `Locked` or `Storage`.
    pub fn save_strain(
        &self,
        id: Option<String>,
        strain: StrainData,
        now_ms: u64,
    ) -> Result<String> {
        let record = strain.to_model()?;
        self.with(|u| u.save(id, &record, now_ms))
    }

    /// A strain, if it exists.
    ///
    /// # Errors
    ///
    /// `WrongType` if `id` is another kind of record.
    pub fn get_strain(&self, id: String) -> Result<Option<StrainData>> {
        self.with(|u| Ok(u.get::<Strain>(&id)?.as_ref().map(StrainData::from_model)))
    }

    /// Creates or updates a brand. Returns its ID.
    ///
    /// # Errors
    ///
    /// `InvalidInput`, `NotFound`, `Locked` or `Storage`.
    pub fn save_brand(&self, id: Option<String>, brand: BrandData, now_ms: u64) -> Result<String> {
        let record = brand.to_model();
        self.with(|u| u.save(id, &record, now_ms))
    }

    /// A brand, if it exists.
    ///
    /// # Errors
    ///
    /// `WrongType` if `id` is another kind of record.
    pub fn get_brand(&self, id: String) -> Result<Option<BrandData>> {
        self.with(|u| Ok(u.get::<Brand>(&id)?.as_ref().map(BrandData::from_model)))
    }

    /// Creates or updates a spot. Returns its ID.
    ///
    /// # Errors
    ///
    /// `InvalidInput`, `NotFound`, `Locked` or `Storage`.
    pub fn save_spot(&self, id: Option<String>, spot: SpotData, now_ms: u64) -> Result<String> {
        let record = spot.to_model()?;
        self.with(|u| u.save(id, &record, now_ms))
    }

    /// A spot, if it exists.
    ///
    /// # Errors
    ///
    /// `WrongType` if `id` is another kind of record.
    pub fn get_spot(&self, id: String) -> Result<Option<SpotData>> {
        self.with(|u| Ok(u.get::<Spot>(&id)?.as_ref().map(SpotData::from_model)))
    }

    /// Creates or updates a purchase. Returns its ID. When it has a Re-up
    /// Spot, the spot is added to each listed product's shops.
    ///
    /// # Errors
    ///
    /// `InvalidInput`, `NotFound`, `Locked` or `Storage`.
    pub fn save_purchase(
        &self,
        id: Option<String>,
        purchase: PurchaseData,
        now_ms: u64,
    ) -> Result<String> {
        let record = purchase.to_model()?;
        self.with(|u| {
            let saved = u.save(id, &record, now_ms)?;
            // The receipt's shop joins each product's "Where I buy it".
            if let Some(spot) = record.spot {
                let products: BTreeSet<RecordId> =
                    record.lines.values().filter_map(|l| l.product).collect();
                for product_id in products {
                    // A line naming a missing or non-product record is skipped.
                    let Ok(Some(mut product)) =
                        u.store.get_record::<Product>(&u.master, &product_id)
                    else {
                        continue;
                    };
                    if product.shops.insert(spot) {
                        u.save(Some(product_id.to_hex()), &product, now_ms)?;
                    }
                }
            }
            Ok(saved)
        })
    }

    /// A purchase, if it exists.
    ///
    /// # Errors
    ///
    /// `WrongType` if `id` is another kind of record.
    pub fn get_purchase(&self, id: String) -> Result<Option<PurchaseData>> {
        self.with(|u| {
            Ok(u.get::<Purchase>(&id)?
                .as_ref()
                .map(PurchaseData::from_model))
        })
    }

    /// For each of `names`, the journal's strain with that name (ignoring
    /// case, spacing and punctuation), or an empty string when there is
    /// none: so a catalog suggestion's parents can be linked to strains the
    /// user already has.
    ///
    /// # Errors
    ///
    /// `Locked` or `Storage`.
    pub fn strain_ids_named(&self, names: Vec<String>) -> Result<Vec<String>> {
        self.with(|u| {
            let mut strains = Vec::new();
            for id in u.store.ids_of_kind(RecordKind::Strain)? {
                if let Some(strain) = u.store.get_record::<Strain>(&u.master, &id)? {
                    strains.push((id, strain));
                }
            }
            Ok(names
                .iter()
                .map(|name| {
                    let wanted = normalize_name(name);
                    strains
                        .iter()
                        .filter(|_| !wanted.is_empty())
                        .find(|(_, s)| normalize_name(&s.name) == wanted)
                        .map(|(id, _)| id.to_hex())
                        .unwrap_or_default()
                })
                .collect())
        })
    }

    /// Products that may be the one being entered: those sharing any of
    /// `codes`, then those with the same name and, when both have one, the
    /// same brand. Code matches come first.
    ///
    /// # Errors
    ///
    /// `InvalidInput` for a malformed brand ID, `Locked` or `Storage`.
    pub fn find_products(
        &self,
        name: Option<String>,
        brand: Option<String>,
        codes: Vec<String>,
    ) -> Result<Vec<ProductMatch>> {
        let brand = brand.map(|b| parse_id(&b)).transpose()?;
        let name = name.map(|n| normalize_name(&n)).filter(|n| !n.is_empty());
        self.with(|u| {
            let mut by_code = Vec::new();
            let mut by_name = Vec::new();
            for id in u.store.ids_of_kind(RecordKind::Product)? {
                let Some(product) = u.store.get_record::<Product>(&u.master, &id)? else {
                    continue;
                };
                if codes
                    .iter()
                    .any(|c| product.identifiers.contains_key(c.trim()))
                {
                    by_code.push(id);
                } else if name.as_ref() == Some(&normalize_name(&product.name))
                    && (brand.is_none() || product.brand.is_none() || brand == product.brand)
                {
                    by_name.push(id);
                }
            }
            Ok(by_code
                .iter()
                .map(|id| ProductMatch {
                    id: id.to_hex(),
                    by_code: true,
                })
                .chain(by_name.iter().map(|id| ProductMatch {
                    id: id.to_hex(),
                    by_code: false,
                }))
                .collect())
        })
    }

    /// Keyword tag suggestions for a snippet's text: the built-in
    /// vocabulary plus the user's custom effect, terpene, flavor and mood
    /// tags, matched on the device. Nothing is stored; the user confirms
    /// each suggestion first.
    ///
    /// # Errors
    ///
    /// `Locked` or `Storage`.
    pub fn suggest_tags(&self, text: String) -> Result<Vec<TagSuggestion>> {
        self.with(|u| {
            let mut custom = Vec::new();
            for id in u.store.ids_of_kind(RecordKind::Tag)? {
                let Some(tag) = u.store.get_record::<Tag>(&u.master, &id)? else {
                    continue;
                };
                if matches!(
                    tag.category,
                    TagCategory::Effect
                        | TagCategory::Terpene
                        | TagCategory::Flavor
                        | TagCategory::Mood
                ) {
                    custom.push(CustomTag {
                        tag: TagRef::Custom(id),
                        category: tag.category,
                        name: tag.name,
                    });
                }
            }
            Ok(terpoir_tagging::suggest(&text, &custom)
                .into_iter()
                .map(|s| TagSuggestion {
                    tag: TagRefData::from_model(&s.tag),
                    category: s.category.slug().to_owned(),
                    start: u32::try_from(s.start).unwrap_or(u32::MAX),
                    end: u32::try_from(s.end).unwrap_or(u32::MAX),
                })
                .collect())
        })
    }

    /// Saved spots near a location fix, closest first ("Show me"). Runs on
    /// the device over the encrypted spots; the fix is not stored. A spot
    /// is near when within its radius plus the fixes' accuracy, capped
    /// (`Spot::nearness_m`).
    ///
    /// # Errors
    ///
    /// `InvalidInput` for impossible coordinates, `Locked` or `Storage`.
    pub fn spots_near(&self, lat_e7: i32, lon_e7: i32, accuracy_m: u32) -> Result<Vec<NearbySpot>> {
        if lat_e7.unsigned_abs() > 900_000_000 || lon_e7.unsigned_abs() > 1_800_000_000 {
            return Err(CoreError::InvalidInput);
        }
        let here = Location {
            lat_e7,
            lon_e7,
            accuracy_m,
        };
        self.with(|u| {
            let mut near = Vec::new();
            for id in u.store.ids_of_kind(RecordKind::Spot)? {
                let Some(spot) = u.store.get_record::<Spot>(&u.master, &id)? else {
                    continue;
                };
                if let Some(d) = spot.nearness_m(&here) {
                    let re_up = spot.is_re_up();
                    near.push((d, id, spot.name, re_up));
                }
            }
            near.sort_by(|a, b| a.0.total_cmp(&b.0));
            Ok(near
                .into_iter()
                .map(|(d, id, name, re_up)| NearbySpot {
                    id: id.to_hex(),
                    name,
                    re_up,
                    // Rounded; nearness is bounded by a radius, far below u32::MAX.
                    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                    distance_m: d.round() as u32,
                })
                .collect())
        })
    }

    /// The Re-up Spots that prompt "Quick log?" when the user leaves after a
    /// stop: opted in, with coordinates. Only where they are; the prompt
    /// never names the spot.
    ///
    /// # Errors
    ///
    /// `Locked` or `Storage`.
    pub fn purchase_prompt_fences(&self) -> Result<Vec<PromptFence>> {
        self.with(|u| {
            let mut fences = Vec::new();
            for id in u.store.ids_of_kind(RecordKind::Spot)? {
                let Some(spot) = u.store.get_record::<Spot>(&u.master, &id)? else {
                    continue;
                };
                if !spot.prompts_purchases() {
                    continue;
                }
                if let Some(at) = spot.location {
                    fences.push(PromptFence {
                        lat_e7: at.lat_e7,
                        lon_e7: at.lon_e7,
                        radius_m: spot.effective_radius_m(),
                    });
                }
            }
            Ok(fences)
        })
    }

    /// Every spot with coordinates, for "Notify me" geofences, with what
    /// an alert about it may show. Computed inside the core over the
    /// encrypted journal; the caller keeps only what its tier allows.
    ///
    /// # Errors
    ///
    /// `Locked` or `Storage`.
    pub fn spot_alerts(&self) -> Result<Vec<SpotAlert>> {
        self.with(|u| {
            let sessions: std::collections::HashSet<RecordId> = u
                .store
                .ids_of_kind(RecordKind::Session)?
                .into_iter()
                .collect();
            let mut alerts = Vec::new();
            for id in u.store.ids_of_kind(RecordKind::Spot)? {
                let Some(spot) = u.store.get_record::<Spot>(&u.master, &id)? else {
                    continue;
                };
                let Some(at) = spot.location else {
                    continue;
                };
                let mut count = 0u32;
                let mut ratings = Vec::new();
                let mut latest: Option<Session> = None;
                for rid in u.store.referencing(&id)? {
                    if !sessions.contains(&rid) {
                        continue;
                    }
                    let Some(session) = u.store.get_record::<Session>(&u.master, &rid)? else {
                        continue;
                    };
                    if session.spot != Some(id) {
                        continue;
                    }
                    count += 1;
                    ratings.extend(session.rating.map(u32::from));
                    if latest
                        .as_ref()
                        .is_none_or(|l| session.when.unix_ms > l.when.unix_ms)
                    {
                        latest = Some(session);
                    }
                }
                let last_product = match latest
                    .as_ref()
                    .and_then(|s| s.lines.values().find_map(|line| line.product))
                {
                    Some(product) => u
                        .store
                        .get_record::<Product>(&u.master, &product)?
                        .map(|p| p.name),
                    None => None,
                };
                // At most a few thousand sessions per spot, far below u32::MAX.
                #[allow(clippy::cast_possible_truncation)]
                let average_rating_tenths = (!ratings.is_empty()).then(|| {
                    let n = ratings.len() as u32;
                    (ratings.iter().sum::<u32>() * 10 + n / 2) / n
                });
                alerts.push(SpotAlert {
                    lat_e7: at.lat_e7,
                    lon_e7: at.lon_e7,
                    radius_m: spot.effective_radius_m(),
                    name: spot.name,
                    sessions: count,
                    average_rating_tenths,
                    last_product,
                });
            }
            Ok(alerts)
        })
    }

    /// The saved spot a receipt's shop name most likely refers to: an
    /// exact name match, else one name containing the other as whole
    /// words. Matching happens on the device; nothing is looked up.
    ///
    /// # Errors
    ///
    /// `Locked` or `Storage`.
    pub fn match_spot(&self, shop: String) -> Result<Option<String>> {
        let wanted = normalize_name(&shop);
        self.with(|u| {
            let mut partial = None;
            for id in u.store.ids_of_kind(RecordKind::Spot)? {
                let Some(spot) = u.store.get_record::<Spot>(&u.master, &id)? else {
                    continue;
                };
                if !wanted.is_empty() && normalize_name(&spot.name) == wanted {
                    return Ok(Some(id.to_hex()));
                }
                if partial.is_none() && names_match(&spot.name, &shop) {
                    partial = Some(id.to_hex());
                }
            }
            Ok(partial)
        })
    }

    /// The user's products, brands and strains that a receipt item's text
    /// may name, best first (at most 10). A product that learned the
    /// line's exact text (case and spacing aside) comes first, marked
    /// `learned`: the only automatic link. The text is broken down first
    /// (`BD PR 1G` looks for `BD`) and matched by name, initials, letters
    /// in order and cut-off names, tolerant of recognition errors. With
    /// the receipt's `spot`, what the user buys there ranks higher:
    /// purchases at that spot and products whose "Where I buy it" includes
    /// it. Matching happens on the device; nothing is stored.
    ///
    /// # Errors
    ///
    /// `InvalidInput` for a malformed spot ID, `Locked` or `Storage`.
    pub fn suggest_for_receipt_item(
        &self,
        text: String,
        spot: Option<String>,
    ) -> Result<Vec<ScanSuggestion>> {
        let spot = spot.map(|s| parse_id(&s)).transpose()?;
        self.with(|u| {
            let mut products = Vec::new();
            for id in u.store.ids_of_kind(RecordKind::Product)? {
                if let Some(product) = u.store.get_record::<Product>(&u.master, &id)? {
                    products.push((id, product));
                }
            }
            // How often each product was bought at the spot.
            let mut bought: BTreeMap<RecordId, u32> = BTreeMap::new();
            if let Some(spot) = spot {
                for id in u.store.ids_of_kind(RecordKind::Purchase)? {
                    let Some(purchase) = u.store.get_record::<Purchase>(&u.master, &id)? else {
                        continue;
                    };
                    if purchase.spot == Some(spot) {
                        for product in purchase.lines.values().filter_map(|l| l.product) {
                            *bought.entry(product).or_default() += 1;
                        }
                    }
                }
                for (id, product) in &products {
                    if product.shops.contains(&spot) {
                        *bought.entry(*id).or_default() += 1;
                    }
                }
            }
            let mut by_record: BTreeMap<RecordId, u32> = BTreeMap::new();
            for (id, product) in &products {
                let n = bought.get(id).copied().unwrap_or(0);
                if n == 0 {
                    continue;
                }
                for related in product.strains.iter().chain(&product.brand) {
                    *by_record.entry(*related).or_default() += n;
                }
            }

            // Products that learned this exact line.
            let learned: Vec<(RecordId, String)> = receipt_name_key(&text)
                .map(|key| {
                    products
                        .iter()
                        .filter(|(_, p)| p.receipt_names.contains(&key))
                        .map(|(id, p)| (*id, p.name.clone()))
                        .collect()
                })
                .unwrap_or_default();
            let ambiguous = learned.len() > 1;
            let mut suggestions: Vec<ScanSuggestion> = learned
                .iter()
                .map(|(id, name)| ScanSuggestion {
                    id: id.to_hex(),
                    record_type: "product".to_owned(),
                    name: name.clone(),
                    match_kind: "learned".to_owned(),
                    score: LEARNED_SCORE,
                    at_shop: bought.get(id).copied().unwrap_or(0) > 0,
                    close: ambiguous,
                    learned: true,
                })
                .collect();

            let mut ids = Vec::new();
            let mut candidates = Vec::new();
            let mut add = |id: RecordId, kind, name: String, at_shop: u32| {
                ids.push(id);
                candidates.push(terpoir_scan::NameCandidate {
                    kind,
                    name,
                    at_shop,
                });
            };
            for (id, product) in products {
                let n = bought.get(&id).copied().unwrap_or(0);
                add(id, terpoir_scan::CandidateKind::Product, product.name, n);
            }
            for id in u.store.ids_of_kind(RecordKind::Brand)? {
                if let Some(brand) = u.store.get_record::<Brand>(&u.master, &id)? {
                    let n = by_record.get(&id).copied().unwrap_or(0);
                    add(id, terpoir_scan::CandidateKind::Brand, brand.name, n);
                }
            }
            for id in u.store.ids_of_kind(RecordKind::Strain)? {
                if let Some(strain) = u.store.get_record::<Strain>(&u.master, &id)? {
                    let n = by_record.get(&id).copied().unwrap_or(0);
                    add(id, terpoir_scan::CandidateKind::Strain, strain.name, n);
                }
            }
            let ranked = terpoir_scan::match_names(&text, &candidates)
                .into_iter()
                .filter(|m| !learned.iter().any(|(id, _)| *id == ids[m.index]))
                .map(|m| ScanSuggestion {
                    id: ids[m.index].to_hex(),
                    record_type: match candidates[m.index].kind {
                        terpoir_scan::CandidateKind::Product => "product",
                        terpoir_scan::CandidateKind::Brand => "brand",
                        terpoir_scan::CandidateKind::Strain => "strain",
                    }
                    .to_owned(),
                    name: candidates[m.index].name.clone(),
                    match_kind: m.kind.slug().to_owned(),
                    score: m.score,
                    at_shop: m.at_shop,
                    close: m.close,
                    learned: false,
                });
            suggestions.extend(ranked);
            suggestions.truncate(10);
            Ok(suggestions)
        })
    }

    /// Learns a receipt name (docs/design-decisions.md, "Learned receipt
    /// names"): the user linked a receipt line to `product`, so the
    /// product keeps the line's text (upper case, single spaces; nothing
    /// else changed), and a receipt printing exactly that text links to it
    /// automatically next time. Another product that had learned the same
    /// text gives it up, since the user just linked it elsewhere. Kept in
    /// the encrypted product and synced; never indexed.
    ///
    /// # Errors
    ///
    /// `InvalidInput` for a malformed ID or an empty text (or one over 200
    /// characters), `NotFound` if the product doesn't exist, `WrongType`
    /// if `product` is another kind of record, `Locked` or `Storage`.
    pub fn learn_receipt_name(
        &self,
        product: String,
        line_text: String,
        now_ms: u64,
    ) -> Result<()> {
        let key = receipt_name_key(&line_text).ok_or(CoreError::InvalidInput)?;
        let product_id = parse_id(&product)?;
        self.with(|u| {
            let mut record = u.get::<Product>(&product)?.ok_or(CoreError::NotFound)?;
            for id in u.store.ids_of_kind(RecordKind::Product)? {
                if id == product_id {
                    continue;
                }
                let Some(mut other) = u.store.get_record::<Product>(&u.master, &id)? else {
                    continue;
                };
                if other.receipt_names.remove(&key) {
                    u.save(Some(id.to_hex()), &other, now_ms)?;
                }
            }
            if record.receipt_names.insert(key) {
                u.save(Some(product), &record, now_ms)?;
            }
            Ok(())
        })
    }

    /// Forgets a learned receipt name of `product` (the "matched" chip's
    /// undo, or the delete on the product's page). The text is normalised
    /// as when it was learned. Returns whether the product had it.
    ///
    /// # Errors
    ///
    /// `InvalidInput` for a malformed ID, `NotFound`, `WrongType`,
    /// `Locked` or `Storage`.
    pub fn forget_receipt_name(
        &self,
        product: String,
        line_text: String,
        now_ms: u64,
    ) -> Result<bool> {
        self.with(|u| {
            let mut record = u.get::<Product>(&product)?.ok_or(CoreError::NotFound)?;
            let Some(key) = receipt_name_key(&line_text) else {
                return Ok(false);
            };
            if !record.receipt_names.remove(&key) {
                return Ok(false);
            }
            u.save(Some(product), &record, now_ms)?;
            Ok(true)
        })
    }

    /// Which of two products to keep when merging them (docs/design-decisions.md,
    /// "Simpler review"): the one recording more (fields set, and entries in
    /// its sets and maps), else the one more sessions and purchases refer
    /// to, else `first`. Returns its ID.
    ///
    /// # Errors
    ///
    /// `InvalidInput` for a malformed ID, `NotFound` if either doesn't
    /// exist, `WrongType` if either is another kind of record, `Locked` or
    /// `Storage`.
    pub fn product_to_keep(&self, first: String, second: String) -> Result<String> {
        let (a_id, b_id) = (parse_id(&first)?, parse_id(&second)?);
        self.with(|u| {
            let a = u.get::<Product>(&first)?.ok_or(CoreError::NotFound)?;
            let b = u.get::<Product>(&second)?.ok_or(CoreError::NotFound)?;
            let rank = |p: &Product, id: &RecordId| -> Result<(usize, usize)> {
                Ok((p.detail_count(), u.store.referencing(id)?.len()))
            };
            Ok(if rank(&b, &b_id)? > rank(&a, &a_id)? {
                second
            } else {
                first
            })
        })
    }

    /// Merges two products into one (docs/design-decisions.md, "Simpler
    /// review": two products that learned the same receipt name, or any two
    /// the user says are one), keeping the most information from each:
    /// `keep`'s empty fields take `other`'s, its sets and maps (strains,
    /// shops, scanned codes, terpenes, cannabinoids, learned receipt names)
    /// take the union, every session line and purchase line naming `other`
    /// names `keep` instead, and then `other` is deleted the usual way,
    /// leaving its deletion marker. It is ordinary edits and a deletion, so
    /// it syncs like any: per field, last writer wins. Returns how many
    /// sessions and purchases were changed.
    ///
    /// # Errors
    ///
    /// `InvalidInput` for a malformed ID or `keep` equal to `other`,
    /// `NotFound` if either doesn't exist, `WrongType` if either is another
    /// kind of record, `Locked` or `Storage`.
    pub fn merge_products(&self, keep: String, other: String, now_ms: u64) -> Result<u32> {
        let keep_id = parse_id(&keep)?;
        let other_id = parse_id(&other)?;
        if keep_id == other_id {
            return Err(CoreError::InvalidInput);
        }
        self.with(|u| {
            let mut kept = u.get::<Product>(&keep)?.ok_or(CoreError::NotFound)?;
            let gone = u.get::<Product>(&other)?.ok_or(CoreError::NotFound)?;
            if kept.absorb(&gone) {
                u.save(Some(keep), &kept, now_ms)?;
            }
            // Sessions and purchases are what name a product (the index's
            // references); each line naming `other` is re-pointed.
            let mut changed = 0u32;
            for id in u.store.referencing(&other_id)? {
                let Some(doc) = u.store.get(&u.master, &id)? else {
                    continue;
                };
                let doc = migrate(doc)?;
                let repointed = match doc.kind() {
                    RecordKind::Session => {
                        let mut session = Session::from_document(&doc)?;
                        let mut any = false;
                        for line in session.lines.values_mut() {
                            if line.product == Some(other_id) {
                                line.product = Some(keep_id);
                                any = true;
                            }
                        }
                        any && {
                            u.save(Some(id.to_hex()), &session, now_ms)?;
                            true
                        }
                    }
                    RecordKind::Purchase => {
                        let mut purchase = Purchase::from_document(&doc)?;
                        let mut any = false;
                        for line in purchase.lines.values_mut() {
                            if line.product == Some(other_id) {
                                line.product = Some(keep_id);
                                any = true;
                            }
                        }
                        any && {
                            u.save(Some(id.to_hex()), &purchase, now_ms)?;
                            true
                        }
                    }
                    _ => false,
                };
                changed += u32::from(repointed);
            }
            u.store.delete(&other_id)?;
            Ok(changed)
        })
    }

    /// Remembers a confirmed shop's numbers from its receipt: the phone and
    /// licence numbers printed in the header of `rows` (as
    /// `match_receipt_shop` reads them) are added to `spot`, so its next
    /// receipt is recognised strongly ("Green Leaf ✓"). Call it when the
    /// user confirms or picks the shop. Returns how many numbers were new.
    ///
    /// # Errors
    ///
    /// `InvalidInput` for a malformed ID, `NotFound` if the spot doesn't
    /// exist, `WrongType` if `spot` is another kind of record, `Locked` or
    /// `Storage`.
    pub fn remember_receipt_shop(
        &self,
        spot: String,
        rows: Vec<String>,
        now_ms: u64,
    ) -> Result<u32> {
        let lines: Vec<&str> = rows.iter().map(String::as_str).collect();
        let header = terpoir_scan::read_header(&lines);
        self.with(|u| {
            let mut record = u.get::<Spot>(&spot)?.ok_or(CoreError::NotFound)?;
            let mut added = 0u32;
            for phone in header.phones {
                added += u32::from(record.phones.insert(phone));
            }
            for licence in header.licences {
                added += u32::from(record.licences.insert(licence));
            }
            if added > 0 {
                u.save(Some(spot), &record, now_ms)?;
            }
            Ok(added)
        })
    }

    /// The saved spot a receipt's header belongs to, from its rows top to
    /// bottom (the rows above the first priced row are read): by a phone
    /// or licence number the spot is known by (strong), else by name (a
    /// guess). Also returns the name, phone and licence numbers the header
    /// prints. A spot is known by the numbers it stores (`SpotData.phones`
    /// and `licences`, added by `remember_receipt_shop` or the spot
    /// editor). Location is never used.
    ///
    /// # Errors
    ///
    /// `Locked` or `Storage`.
    pub fn match_receipt_shop(&self, rows: Vec<String>) -> Result<ReceiptShopMatch> {
        let lines: Vec<&str> = rows.iter().map(String::as_str).collect();
        let header = terpoir_scan::read_header(&lines);
        self.with(|u| {
            let mut ids = Vec::new();
            let mut candidates = Vec::new();
            for id in u.store.ids_of_kind(RecordKind::Spot)? {
                if let Some(spot) = u.store.get_record::<Spot>(&u.master, &id)? {
                    ids.push(id);
                    candidates.push(terpoir_scan::ShopCandidate {
                        name: spot.name,
                        phones: spot.phones.into_iter().collect(),
                        licences: spot.licences.into_iter().collect(),
                    });
                }
            }
            let found = terpoir_scan::match_shop(&lines, &candidates);
            let row = |n: usize| u32::try_from(n).unwrap_or(u32::MAX);
            Ok(ReceiptShopMatch {
                spot: found.as_ref().map(|m| ids[m.index].to_hex()),
                kind: found.as_ref().map(|m| m.kind.slug().to_owned()),
                strong: found.as_ref().is_some_and(|m| m.kind.is_strong()),
                row: found.as_ref().map(|m| row(m.line)),
                shop: header.name,
                shop_row: header.name_line.map(row),
                phones: header.phones,
                licences: header.licences,
            })
        })
    }

    /// Creates or updates a stream snippet. Returns its ID.
    ///
    /// # Errors
    ///
    /// `InvalidInput`, `NotFound`, `Locked` or `Storage`.
    pub fn save_snippet(
        &self,
        id: Option<String>,
        snippet: SnippetData,
        now_ms: u64,
    ) -> Result<String> {
        let record = snippet.to_model()?;
        self.with(|u| u.save(id, &record, now_ms))
    }

    /// A snippet, if it exists.
    ///
    /// # Errors
    ///
    /// `WrongType` if `id` is another kind of record.
    pub fn get_snippet(&self, id: String) -> Result<Option<SnippetData>> {
        self.with(|u| Ok(u.get::<Snippet>(&id)?.as_ref().map(SnippetData::from_model)))
    }

    /// Creates or updates a custom tag. Returns its ID.
    ///
    /// # Errors
    ///
    /// `NotFound`, `Locked` or `Storage`.
    pub fn save_tag(&self, id: Option<String>, tag: TagData, now_ms: u64) -> Result<String> {
        let record = tag.to_model();
        self.with(|u| u.save(id, &record, now_ms))
    }

    /// The usual method (slug) for a product of `form` and `subtype`, as a
    /// new session line's default (docs/design-decisions.md, "Product
    /// subtypes"): the subtype's own method when it is under that form (a
    /// patch is transdermal; a custom subtype's chosen method), else the
    /// form's usual one. Reads a custom subtype's tag; a missing one counts
    /// as no subtype. `None` with neither, or for a form from a newer app
    /// version.
    ///
    /// # Errors
    ///
    /// `InvalidInput` for a malformed subtype, `WrongType` if a custom
    /// subtype's ID is another kind of record, `Locked` or `Storage`.
    pub fn usual_method(
        &self,
        form: Option<String>,
        subtype: Option<TagRefData>,
    ) -> Result<Option<String>> {
        let subtype = subtype.as_ref().map(TagRefData::to_model).transpose()?;
        let form = form.as_deref().map(Form::from_slug);
        self.with(|u| {
            let custom = match &subtype {
                Some(TagRef::Custom(id)) => u.store.get_record::<Tag>(&u.master, id)?,
                _ => None,
            };
            Ok(
                usual_method(form.as_ref(), subtype.as_ref(), custom.as_ref())
                    .map(|m| m.slug().to_owned()),
            )
        })
    }

    /// A custom tag, if it exists.
    ///
    /// # Errors
    ///
    /// `WrongType` if `id` is another kind of record.
    pub fn get_tag(&self, id: String) -> Result<Option<TagData>> {
        self.with(|u| Ok(u.get::<Tag>(&id)?.as_ref().map(TagData::from_model)))
    }

    /// Deletes a record and its attachments (crypto-erasure). Returns
    /// whether it existed. The deletion syncs to other devices.
    ///
    /// # Errors
    ///
    /// `InvalidInput`, `Locked` or `Storage`.
    pub fn delete_record(&self, id: String) -> Result<bool> {
        let id = parse_id(&id)?;
        self.with(|u| Ok(u.store.delete(&id)?))
    }

    /// The whole journal in plaintext: `terpoir.json` with every record,
    /// and `sessions.csv`, `products.csv` and `purchases.csv`. Unencrypted:
    /// the app offers it only behind a strong warning, saved where the user
    /// picks.
    ///
    /// # Errors
    ///
    /// `Locked` or `Storage`.
    pub fn export_plaintext(&self, now_ms: u64) -> Result<Vec<ExportFile>> {
        self.with(|u| plaintext::export(&*u, now_ms))
    }

    /// Deletes every targeted symptom: each session's symptom ratings and
    /// the user's own symptom tags (the off-action's "delete" choice for
    /// targeted symptoms). The removals sync like edits. Returns how many
    /// sessions changed.
    ///
    /// # Errors
    ///
    /// `Locked` or `Storage`.
    pub fn delete_symptoms(&self, now_ms: u64) -> Result<u32> {
        self.with(|u| {
            let mut changed = 0u32;
            for id in u.store.ids_of_kind(RecordKind::Session)? {
                let Some(mut session) = u.store.get_record::<Session>(&u.master, &id)? else {
                    continue;
                };
                if session.symptoms.is_empty() {
                    continue;
                }
                session.symptoms.clear();
                u.save(Some(id.to_hex()), &session, now_ms)?;
                changed += 1;
            }
            for id in u.store.ids_of_kind(RecordKind::Tag)? {
                let tag = u.store.get_record::<Tag>(&u.master, &id)?;
                if tag.is_some_and(|t| t.category == TagCategory::Symptom) {
                    u.store.delete(&id)?;
                }
            }
            Ok(changed)
        })
    }

    /// Insights over the user's rated sessions (opt-in: the app calls this
    /// only while insights are on), strongest first, minus those the user
    /// dismissed, marked wrong, or muted by category.
    ///
    /// # Errors
    ///
    /// `Locked` or `Storage`.
    pub fn insights(&self) -> Result<Vec<InsightData>> {
        self.with(|u| insights::current(&*u))
    }

    /// Shop insights: per Re-up Spot, the average rating, hit rate and
    /// freshness of what was bought there, and products rated differently
    /// there than elsewhere. Opt-in like all insights; computed when called.
    ///
    /// # Errors
    ///
    /// `Locked` or `Storage`.
    pub fn shop_insights(&self) -> Result<Vec<ShopInsightData>> {
        self.with(|u| insights::shops(&*u))
    }

    /// "How it usually goes": typical timelines by method, product and
    /// strain, from the points the user set. Opt-in with the other
    /// insights; empty while the timeline is hidden.
    ///
    /// # Errors
    ///
    /// `Locked` or `Storage`.
    pub fn timeline_insights(&self) -> Result<Vec<TimelineInsightData>> {
        self.with(|u| insights::timelines(&*u))
    }

    /// Records the user's feedback on an insight (`key` is the insight's
    /// key, or its category for [`InsightFeedback::Muted`]). Kept in a
    /// synced record, so it holds on every device and in exports.
    ///
    /// # Errors
    ///
    /// `Locked` or `Storage`.
    pub fn set_insight_feedback(
        &self,
        key: String,
        feedback: InsightFeedback,
        now_ms: u64,
    ) -> Result<()> {
        self.with(|u| crate::feedback::set(u, &key, feedback.slug(), now_ms))
    }

    /// Forgets all insight feedback (the insights toggle's off-action), on
    /// every device: the removals sync.
    ///
    /// # Errors
    ///
    /// `Locked` or `Storage`.
    pub fn forget_insight_feedback(&self, now_ms: u64) -> Result<()> {
        self.with(|u| crate::feedback::forget(u, now_ms))
    }

    /// IDs of all records of a type.
    ///
    /// # Errors
    ///
    /// `Locked` or `Storage`.
    pub fn list(&self, record_type: RecordType) -> Result<Vec<String>> {
        self.with(|u| Ok(ids(u.store.ids_of_kind(record_type.into())?)))
    }

    /// IDs of sessions and snippets dated within `first_day..=last_day`
    /// (days since 1970-01-01 in the time zone where each was recorded).
    ///
    /// # Errors
    ///
    /// `Locked` or `Storage`.
    pub fn on_days(&self, first_day: i64, last_day: i64) -> Result<Vec<String>> {
        self.with(|u| Ok(ids(u.store.ids_on_days(first_day, last_day)?)))
    }

    /// IDs of records that reference `id`.
    ///
    /// # Errors
    ///
    /// `InvalidInput`, `Locked` or `Storage`.
    pub fn referencing(&self, id: String) -> Result<Vec<String>> {
        let id = parse_id(&id)?;
        self.with(|u| Ok(ids(u.store.referencing(&id)?)))
    }

    /// IDs of records carrying a confirmed tag.
    ///
    /// # Errors
    ///
    /// `InvalidInput`, `Locked` or `Storage`.
    pub fn with_tag(&self, tag: TagRefData) -> Result<Vec<String>> {
        let key = match tag {
            TagRefData::Builtin { slug } => slug,
            TagRefData::Custom { id } => format!("custom:{}", parse_id(&id)?.to_hex()),
        };
        self.with(|u| Ok(ids(u.store.ids_with_tag(&key)?)))
    }

    /// Encrypts a photo or voice clip and attaches it to a session.
    /// Returns the attachment ID.
    ///
    /// # Errors
    ///
    /// `NotFound` if the session does not exist; `WrongType` if `session`
    /// is not a session.
    pub fn add_attachment(&self, session: String, bytes: Vec<u8>, now_ms: u64) -> Result<String> {
        let bytes = Zeroizing::new(bytes);
        let session_id = parse_id(&session)?;
        self.with(|u| {
            let mut record = u
                .store
                .get_record::<Session>(&u.master, &session_id)?
                .ok_or(CoreError::NotFound)?;
            let file = u
                .store
                .write_file(&u.master, &session_id, bytes.as_slice())?;
            record.attachments.insert(file);
            u.save(Some(session), &record, now_ms)?;
            Ok(file.to_hex())
        })
    }

    /// Decrypts an attachment.
    ///
    /// # Errors
    ///
    /// `NotFound`, or `WrongKey` if the file was tampered with.
    pub fn read_attachment(&self, id: String) -> Result<Vec<u8>> {
        let id = parse_id(&id)?;
        self.with(|u| {
            let mut out = Vec::new();
            u.store.read_file(&u.master, &id, &mut out)?;
            Ok(out)
        })
    }

    /// Deletes an attachment and removes it from its session.
    ///
    /// # Errors
    ///
    /// `NotFound`, `Locked` or `Storage`.
    pub fn delete_attachment(&self, session: String, id: String, now_ms: u64) -> Result<bool> {
        let session_id = parse_id(&session)?;
        let file = parse_id(&id)?;
        self.with(|u| {
            let mut record = u
                .store
                .get_record::<Session>(&u.master, &session_id)?
                .ok_or(CoreError::NotFound)?;
            let existed = u.store.delete_file(&file)?;
            if record.attachments.remove(&file) {
                u.save(Some(session), &record, now_ms)?;
            }
            Ok(existed)
        })
    }

    /// Writes the whole journal as encrypted sync blobs to upload.
    ///
    /// # Errors
    ///
    /// `Locked` or `Storage`.
    pub fn export_sync_blobs(&self) -> Result<Vec<SyncBlob>> {
        self.with(|u| {
            let first = u
                .store
                .meta(META_SEQUENCE)?
                .and_then(|b| b.try_into().ok())
                .map_or(1, u64::from_be_bytes);
            let blobs = export_blobs(&u.store, &u.master, u.clock.device(), first)?;
            let next = first.saturating_add(u64::try_from(blobs.len()).unwrap_or(u64::MAX));
            u.store.set_meta(META_SEQUENCE, &next.to_be_bytes())?;
            Ok(blobs
                .into_iter()
                .map(|(name, bytes)| SyncBlob { name, bytes })
                .collect())
        })
    }

    /// The whole journal as an encrypted export archive (see
    /// `read_export_archive`): `master_wrapped_by_recovery` (as stored on
    /// this device) plus every sync blob, so only the recovery key opens it.
    ///
    /// # Errors
    ///
    /// `Locked` or `Storage`.
    pub fn export_archive(&self, master_wrapped_by_recovery: Vec<u8>) -> Result<Vec<u8>> {
        let blobs: Vec<Vec<u8>> = self
            .export_sync_blobs()?
            .into_iter()
            .map(|b| b.bytes)
            .collect();
        crate::archive::write(&master_wrapped_by_recovery, &blobs)
    }

    /// Checks an export of this journal before importing it: what it holds
    /// and whether any of it needs a newer app. Nothing is changed.
    ///
    /// # Errors
    ///
    /// `InvalidSyncData` if it is damaged or from another journal.
    pub fn check_export_archive(
        &self,
        archive: crate::archive::ExportArchive,
    ) -> Result<crate::archive::ArchiveCheck> {
        self.with(|u| crate::archive::check(&u.master, &archive))
    }

    /// Applies a sync blob downloaded from the user's storage. Safe to
    /// repeat.
    ///
    /// # Errors
    ///
    /// `InvalidSyncData` if it is malformed or from another journal;
    /// `NewerVersion` if a newer app wrote it in a format this version
    /// can't parse (keep it and import it again after updating).
    pub fn import_sync_blob(&self, bytes: Vec<u8>, now_ms: u64) -> Result<ImportResult> {
        self.with(|u| {
            let summary = import_blob(&mut u.store, &u.master, &mut u.clock, &bytes, now_ms)?;
            u.save_clock()?;
            let n = |v: usize| u64::try_from(v).unwrap_or(u64::MAX);
            Ok(ImportResult {
                records: n(summary.records),
                deletions: n(summary.deletions),
                deferred: n(summary.deferred),
            })
        })
    }

    /// Applies records kept for a newer app version (see
    /// [`ImportResult::deferred`]) that this version can now read. Call
    /// after unlocking. Returns how many were applied.
    ///
    /// # Errors
    ///
    /// `Locked` or `Storage`.
    pub fn apply_pending_records(&self, now_ms: u64) -> Result<u64> {
        self.with(|u| {
            let applied = apply_pending(&mut u.store, &u.master, &mut u.clock, now_ms)?;
            u.save_clock()?;
            Ok(u64::try_from(applied).unwrap_or(u64::MAX))
        })
    }

    /// How many records are kept for a newer app version.
    ///
    /// # Errors
    ///
    /// `Locked` or `Storage`.
    pub fn pending_record_count(&self) -> Result<u64> {
        self.with(|u| Ok(u.store.pending_count()?))
    }

    /// The user's own MapTiler API key for the online map, if they saved
    /// one. Kept in this device's encrypted local metadata; never synced or
    /// exported.
    ///
    /// # Errors
    ///
    /// `Locked` or `Storage`.
    pub fn maptiler_key(&self) -> Result<Option<String>> {
        self.with(|u| {
            Ok(u.store
                .meta(META_MAPTILER_KEY)?
                .and_then(|bytes| String::from_utf8(bytes).ok()))
        })
    }

    /// Saves the user's MapTiler key (`Some`) or deletes it (`None`).
    ///
    /// # Errors
    ///
    /// `InvalidInput` unless the key is 1 to 128 ASCII letters and digits;
    /// `Locked` or `Storage`.
    pub fn set_maptiler_key(&self, key: Option<String>) -> Result<()> {
        if let Some(key) = &key
            && (key.is_empty()
                || key.len() > MAX_MAPTILER_KEY_LEN
                || !key.bytes().all(|b| b.is_ascii_alphanumeric()))
        {
            return Err(CoreError::InvalidInput);
        }
        self.with(|u| {
            match key {
                Some(key) => u.store.set_meta(META_MAPTILER_KEY, key.as_bytes())?,
                None => {
                    u.store.delete_meta(META_MAPTILER_KEY)?;
                }
            }
            Ok(())
        })
    }

    /// Housekeeping: folds and truncates the write-ahead log and compacts
    /// the database. Run occasionally (for example when charging).
    ///
    /// # Errors
    ///
    /// `Locked` or `Storage`.
    pub fn maintenance(&self) -> Result<()> {
        self.with(|u| {
            u.store.vacuum()?;
            Ok(())
        })
    }

    /// Rotates the master key: re-wraps every record key and the database
    /// key, and wraps the new master key under the recovery key. The
    /// recovery key is checked against `master_wrapped_by_recovery` first.
    /// Store every returned value in place of the old ones, rewrite sync
    /// blobs afterwards, and call `merge_captures` so the inbox gets the
    /// new capture public key.
    ///
    /// # Errors
    ///
    /// `WrongKey` if the recovery key does not belong to this journal.
    pub fn rotate_master_key(
        &self,
        recovery_key: String,
        master_wrapped_by_recovery: Vec<u8>,
    ) -> Result<RotatedKeys> {
        let text = Zeroizing::new(recovery_key);
        let recovery = RecoveryKey::parse(&text)?;
        let kek = recovery.key_encryption_key()?;
        let current =
            kek.unwrap_master_key(&WrappedKey::from_slice(&master_wrapped_by_recovery)?)?;
        self.with(|u| {
            if current.expose_secret() != u.master.expose_secret() {
                return Err(CoreError::WrongKey);
            }
            let new_master = MasterKey::generate()?;
            let database_key = u.master.unwrap_database_key(&u.wrapped_database_key)?;
            let wrapped_database_key = new_master.wrap_database_key(&database_key)?;
            u.store.rewrap_all(&u.master, &new_master)?;
            crate::capture::rotate(u, &new_master)?;
            crate::tracking::rotate(u, &new_master)?;
            crate::feedback::rotate(u, &new_master)?;
            let rotated = RotatedKeys {
                master_key: new_master.expose_secret().to_vec(),
                master_wrapped_by_recovery: kek.wrap_master_key(&new_master)?.as_bytes().to_vec(),
                wrapped_database_key: wrapped_database_key.as_bytes().to_vec(),
            };
            u.master = new_master;
            u.wrapped_database_key = wrapped_database_key;
            Ok(rotated)
        })
    }
}

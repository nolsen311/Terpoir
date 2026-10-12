// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

//! The unlocked journal.

use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard};
use terpoir_crypto::{MasterKey, RecoveryKey, WrappedKey};
use terpoir_model::{
    Brand, DeviceId, Document, Hlc, Location, Product, Purchase, Record, RecordId, RecordKind,
    Session, Snippet, Spot, Strain, Tag, TagCategory, TagRef, migrate,
};
use terpoir_scan::{names_match, normalize_name};
use terpoir_store::Store;
use terpoir_sync::{Clock, export_blobs, import_blob};
use terpoir_tagging::CustomTag;
use zeroize::Zeroizing;

use crate::insights::{self, InsightData, InsightFeedback, ShopInsightData};
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

/// A saved spot near a location fix.
#[derive(Clone, PartialEq, Eq, uniffi::Record)]
pub struct NearbySpot {
    /// The spot's ID.
    pub id: String,
    /// Its name.
    pub name: String,
    /// Distance from the fix, in whole metres.
    pub distance_m: u32,
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
    /// Records from a newer app version, skipped; import the blob again
    /// after updating.
    pub deferred: u64,
}

struct Unlocked {
    store: Store,
    master: MasterKey,
    clock: Clock,
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

    fn feedback(&self) -> Result<Vec<u8>> {
        Ok(self
            .store
            .meta(insights::META_FEEDBACK)?
            .unwrap_or_default())
    }
}

impl Unlocked {
    fn save_clock(&mut self) -> Result<()> {
        let last = self.clock.last();
        let mut bytes = [0u8; 12];
        bytes[..8].copy_from_slice(&last.wall_ms.to_be_bytes());
        bytes[8..].copy_from_slice(&last.counter.to_be_bytes());
        self.store.set_meta(META_CLOCK, &bytes)?;
        Ok(())
    }

    fn save<R: Record>(&mut self, id: Option<String>, record: &R, now_ms: u64) -> Result<String> {
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

    fn with<T>(&self, f: impl FnOnce(&mut Unlocked) -> Result<T>) -> Result<T> {
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
        Ok(Arc::new(Self {
            state: Mutex::new(Some(Unlocked {
                store,
                master,
                clock: Clock::new(device, last),
                wrapped_database_key,
            })),
        }))
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

    /// Creates or updates a purchase. Returns its ID.
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
        self.with(|u| u.save(id, &record, now_ms))
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
                    near.push((d, id, spot.name));
                }
            }
            near.sort_by(|a, b| a.0.total_cmp(&b.0));
            Ok(near
                .into_iter()
                .map(|(d, id, name)| NearbySpot {
                    id: id.to_hex(),
                    name,
                    // Rounded; nearness is bounded by a radius, far below u32::MAX.
                    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                    distance_m: d.round() as u32,
                })
                .collect())
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

    /// Records the user's feedback on an insight (`key` is the insight's
    /// key, or its category for [`InsightFeedback::Muted`]). Kept in the
    /// encrypted store, on this device only.
    ///
    /// # Errors
    ///
    /// `Locked` or `Storage`.
    pub fn set_insight_feedback(&self, key: String, feedback: InsightFeedback) -> Result<()> {
        self.with(|u| {
            let current = u.store.meta(insights::META_FEEDBACK)?.unwrap_or_default();
            let next = insights::with_feedback(&current, &key, feedback);
            u.store.set_meta(insights::META_FEEDBACK, &next)?;
            Ok(())
        })
    }

    /// Forgets all insight feedback (the insights toggle's off-action).
    ///
    /// # Errors
    ///
    /// `Locked` or `Storage`.
    pub fn forget_insight_feedback(&self) -> Result<()> {
        self.with(|u| Ok(u.store.set_meta(insights::META_FEEDBACK, b"")?))
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

    /// Applies a sync blob downloaded from the user's storage. Safe to
    /// repeat.
    ///
    /// # Errors
    ///
    /// `InvalidSyncData` if it is malformed or from another journal.
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
    /// Store every returned value in place of the old ones, and rewrite
    /// sync blobs afterwards.
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

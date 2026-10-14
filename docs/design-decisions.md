# Terpoir — Design Decisions

Working name: **Terpoir** (terpene + terroir). Domain: **terpoir.app**.
Repository: https://github.com/nolsen311/Terpoir
Status: pre-development, high-level design. Last updated 2026-10-12.

## Purpose
A mobile app for cannabis consumers to journal experiences with strains, products, brands, delivery methods, and anything cannabis-related, and to understand how those affect them personally.

## Guiding principles
- **Privacy first.** Protect user data from platform vendors, storage providers, network observers, and anyone without the user's consent.
- **Consent first.** Every practice that affects the user is an explicit, informed, revocable choice. Defaults are the most private option.
- **Honest disclosure.** When a feature leaks information, say exactly what, to whom, and how to avoid it.
- **Voice and tone.** Playful and warm inside the app: cannabis can be fun, and people are sharing something personal, so laughing is allowed. Neutral wording on discreet surfaces (Discreet-tier notifications, discreet-icon mode). Humor never pressures a privacy choice: any playful nudge is one tap to dismiss with no extra step and no repeat nagging.
- **The user's threat model, the user's choice.** Privacy options are as granular as practical, so each person can match the app to what they actually fear. Someone in a jurisdiction where they have little to fear can trade privacy for convenience, one toggle at a time or through YOLO Mode (see Privacy Center), with every trade disclosed. The default is still the most private option.
- **A lean app.** The base app carries only the essentials; heavy components (text-recognition engines, language data, models) are optional downloads where the store allows it (see Optional components).
- **Recorded exceptions to "most private default":**
  - **Keyboard choice.** The keyboard is the user's own configured tool; the app defaults to it unchanged, keeps only zero-cost protections, and discloses the risks (see Keyboards and voice input).
  - **Auto-lock timing (2026-10-12, maintainer).** The app locks with the device by default, like the user's other apps, instead of on its own timer; shorter in-app timeouts are opt-in (see Memory hygiene and app unlock).

## Platforms and architecture
- Native apps on both Android and iOS. **Android first, iOS second.**
- **Shared core in Rust**, exposed to both platforms via UniFFI (Kotlin and Swift bindings). The core owns encryption, the data model, sync, and merge logic. Platform code handles only UI and OS integration. One implementation to audit.
  - Rationale: mature, audited crypto libraries; `zeroize` for wiping key material from memory (not reliably possible on the JVM); production-grade CRDT libraries available if ever needed.
  - Android UI: Kotlin + Jetpack Compose. Build via cargo-ndk; iOS via xcframework.
- **FFI surface (decided, `core/crates/terpoir-ffi`):** UniFFI proc-macro bindings; Kotlin package `app.terpoir.core`, Swift module `TerpoirCore`. The platform gets key setup/recovery/passphrase functions and a `Journal` object (typed save/get per record type, queries, attachments, sync blob export/import, maintenance, master-key rotation, `lock()`). The master key crosses the boundary as bytes only where the platform keystore must wrap or unwrap it; the platform wipes its copy right after. Errors expose a kind only. A panic hook never reports panic messages (release builds report nothing; debug builds the source location only). The binding generator is a separate build-only crate, so its dependencies never ship. CI compiles the generated Kotlin with a smoke test and runs it on the JVM against the host library.
- **No operator-run backend.** The developer never hosts user data and is never a subpoena or compromise target.
- Local-first: the encrypted on-device store is the source of truth.

## Sync and storage
- Bring-your-own storage only: iCloud Drive, Google Drive, WebDAV/Nextcloud, S3-compatible buckets, Syncthing folders, or encrypted export/import files.
- Everything uploaded is a client-side-encrypted blob; providers see only ciphertext.
- Metadata defense on the client: padding, batched writes, opaque file names.
- Device pairing is direct (QR code or local transfer); no third party involved.
- Account sign-in (Google, Apple), if offered at all, is transport only. Keys are never derived from or escrowed with the account provider.

### Folder sync: Android implementation (2026-10-12)
- **Toggle:** "Sync through a folder" (Sync and keys, off by default, in neither preset). While it is on, the Privacy Center's data card offers "Choose folder" (the system folder picker; the app keeps a persisted permission for that folder only) and "Sync now".
- **When:** on unlock, after edits, and on "Sync now", only while the app is open. No background sync, and no INTERNET permission: whatever syncs the folder (Syncthing, or a cloud drive app's document provider) moves the files.
- **Each sync (`FolderSync`):** imports every `<32 hex>.terpoir` blob not seen before, then writes this device's full state as new blobs. Each blob is written under a `.partial` name and renamed when complete; on providers that cannot rename, it is written directly and a reader that catches it half-written retries next time. Then this device's previous blobs are deleted, since the new ones supersede them. Merging is the core's (`import_sync_blob`, idempotent).
- **Local state:** the folder URI, the names of blobs already imported, and this device's own current blob names, in app-private storage. Names are random and carry nothing.
- **Off-action:** gives up the folder permission and forgets that state. Files already in the folder stay there, encrypted; the card says so.
- **Newer apps:** a blob in a newer app's container format is left unread until this app updates. Records a newer app wrote inside a readable blob are kept by the core (see "Schema upgrades"), so the blob counts as imported.
- **Not yet:** a reinstall forgets this device's own blob names, so its old blobs stay in the folder until deleted by hand.
- **Restore from a sync folder (maintainer, 2026-10-13):** each sync also keeps a `terpoir.key` in the folder: the master key wrapped by the recovery key, in the export format with no blobs (`sync_folder_key_file`). With the folder's blobs it makes an export, so a new device restores from the folder with the recovery key, through the export path.
  - **Accepted exposure:** whoever holds the folder holds the master key wrapped by the recovery key. That is what an export file already carries; the recovery key's 256 bits stay the protection. The disclosure card says so.
  - **Writing it:** a device writes the file when the folder has none. Wrapping is randomized, so each device's copy differs; a device replaces only the copy it wrote itself (it remembers its hash), and only when its own copy changed. Otherwise devices would overwrite each other at every sync.
  - **Restoring:** blobs that don't open under the folder's master key are left out (`readable_blobs`): one still being written, say, or one from before a master-key rotation. If none open, the restore is refused and the user is asked to sync once from a device that has the journal. Blobs in a newer format still refuse the restore, as for an export. Afterwards the device keeps syncing through that folder: the folder sync toggle is turned on (the restore screen says so), and every blob is imported again at the first sync, which changes nothing already there.

### Merge strategy (decided)
- **Per-field last-writer-wins.** Each field carries a hybrid logical clock (HLC) timestamp plus device ID; on conflict the later write wins, ties broken by device ID. Every device reaches the same result independently.
- **New entries merge as a set union.**
- **Deletions sync as a minimal marker** carrying only the entry ID (no content). Markers are cleared once all paired devices have seen them.
- Concurrent edits to the *same field* lose one side; accepted as rare for a personal journal.
- CRDT text (e.g. for the notes field) deferred; revisit only if same-field conflicts become a real problem. CRDT operation history conflicts with crypto-erasure, so it would need compaction.

### Sync implementation (decided, `core/crates/terpoir-sync`)
- **Hybrid logical clock:** stamps follow wall time when clocks are sane and stay causally ordered when they are not. A remote reading can pull the local clock at most 1 hour past local wall time, so a device with a wildly wrong clock cannot drag everyone's stamps into the future.
- **Merge:** per field, the greater stamp wins (time, counter, then device ID). Documents must be at the same schema version; a document from a newer app is kept aside, not merged, until this app can read it. Unknown records are added (set union); a deletion marker always wins, so a deleted record never returns.
- **Against the store:** new records are stored exactly as received (same wrapped key and ciphertext), so a record has one key on every device and its files open everywhere; existing records are merged and re-sealed under their existing key. Deleting a record leaves only its ID as a marker (`deletions` table). Export is full-state for now (always correct because merging is idempotent); incremental export is an optimization for later.
- **Marker cleanup:** a marker can be dropped once every paired device has seen it **and** no sync blob still carries the record (old blobs are rewritten first); otherwise an old blob could resurrect it on a newly paired device.
- **Blob format:** a batch of sealed records (the same ciphertext as the local store) plus deletion IDs, padded to a size bucket (4 KiB minimum, powers of two to 1 MiB, then whole MiB), encrypted under a container key derived from the master key, and stored under a random 128-bit hex name. Providers learn neither record counts, IDs, kinds, nor exact sizes. Master-key rotation changes the container key, so blobs are rewritten after rotation.
- **Verified by property tests:** merge is commutative, associative and idempotent; three devices with clocks skewed by up to a day, random edits, deletions and partial syncs converge to identical journals for every delivery order, including redelivery.

## Keys and recovery
- Key hierarchy: recovery key → master key → **per-entry data keys**. Rotation re-wraps keys instead of re-encrypting the journal.
- **Per-entry keys (decided):** each entry, with its photos, gets its own random data key, wrapped by the master key.
- **Crypto-erasure on delete:** deleting an entry destroys its wrapped key, making any leftover ciphertext (flash remnants, old sync blobs, provider history) unreadable.
- **No auto-expiry.** Entries persist until the user deletes them.
- **Provider versioning warning:** crypto-erasure only holds if every copy of the wrapped key is gone. Warn users that Google Drive revision history, S3 versioning, Syncthing file versioning, and similar can retain old copies, and how to disable or purge them.
- **Periodic master-key rotation** invalidates stale wrapped-key copies that may linger in provider history.
- Recovery key is human-handleable (word list or grouped Base32 with checksum) plus a printable QR code.
- Backup options, user's choice:
  - Password manager (platform credential APIs where supported, or auto-expiring, sensitive-marked clipboard). Disclose that the password manager's sync becomes part of the trust chain.
  - Local key file, ideally wrapped with a separate passphrase.
  - Paper.
- **Colocation warning:** a key stored next to the data it protects is no protection. Warn (blocking where detectable) if the key is saved to the same provider used for sync. Explain the nuance that E2E-encrypted password managers are far better than plain files, but a single account takeover still gets closer to both.
- Onboarding requires confirming the backup before sync is enabled, with a deliberate "skip, I accept data loss" path.
- No escrow: losing all devices and the recovery key means losing the data. Onboarding makes this clear.
- In-app and on the website: **"We will never ask for your recovery key."**
- **Android onboarding (implemented):**
  - **Brand intro (maintainer, 2026-10-14):** the first-run screens (age gate, new or existing journal, restore, recovery-key backup, the Google Maps choice) sit under a brand header: the mark, "Terpoir" in Fraunces Black and the tagline on the icon's green, as in the Play feature graphic (`onboarding/BrandFrame.kt`, `res/drawable/logo_mark.xml`). On the first of them after a launch it fills the screen for under a second, then slides up into an 88 dp band while the page fades in below; later pages show the band at once. No intro with animations turned off in system settings. The Privacy Center tour and the unlock screen don't use it. The wordmark is sized in dp, so a large font setting can't push it off screen.
  - **The key is shown once.** It comes from `createKeys`, is held in process memory only (a JVM string cannot be wiped, so it is dropped as soon as the backup is resolved), and is never written anywhere. The screen is under screen protection.
  - **Backup options in this build:**
    - Paper.
    - Copy for a password manager, with the disclosure. The copy is marked sensitive and cleared after a minute.
    - Later: the Credential Manager path, an encrypted key file and a printable QR code.
  - **Confirming the backup** means typing the whole key back. The core unwraps the master key with it (`masterKeyFromRecovery`) and the result is wiped at once, so a confirmed copy is proven to open this journal, not just to look right. Typos and another journal's key are told apart.
  - **Skipping** means "Skip, I accept data loss", behind a dialog that says the key won't be shown again.
  - **The journal is blocked until the backup is resolved.** If the process dies before that, the next start discards the new journal and its Keystore key and restarts setup. The journal is still empty, and its only recovery key died with the process.

### Crypto details (decided, `core/crates/terpoir-crypto`)
- **Key hierarchy in code:** a recovery key or app passphrase yields a key-encryption key, which wraps the master key; the platform keystore also wraps the master key. The master key wraps only other keys: one random key per record and the SQLCipher database key. Subkeys (sync container and similar) come from HKDF-SHA-256 over the master key with fixed purpose labels.
- **Domain separation:** every XChaCha20-Poly1305 operation binds a purpose label (record, wrapped master/record/database key, file stream) and its context in the associated data. A wrapped record key is bound to its record ID and will not unwrap for another record.
- **Formats are versioned:** sealed data is `version || 192-bit random nonce || ciphertext || tag`; wrapped keys are 73 bytes.
- **Files** (photos, clips) use the STREAM construction (`aead-stream`, 64 KiB chunks) with a per-file key derived from the owning record key and the file ID, so destroying the record key erases its files.
- **Recovery key:** 256 bits, written as 56 characters of Crockford Base32 in 14 groups of 4 (version byte, key, 16-bit SHA-256 checksum). Input ignores case, spaces and hyphens and reads I/L as 1 and O as 0. Chosen over a word list: no word list to ship or translate, and it fits a QR code and a password manager equally well.
- **App passphrase:** Argon2id, 64 MiB, 3 iterations, 1 lane, 128-bit random salt; parameters are stored with the wrapped key and capped (1 GiB, 16 iterations) so tampered parameters cannot hang unlock.
- **Capture key pair (write-only capture, 2026-10-13):** HPKE (RFC 9180) in base mode with DHKEM(X25519, HKDF-SHA-256), HKDF-SHA-256 and ChaCha20-Poly1305, from the `hpke` crate (rozbb/rust-hpke). Chosen over a libsodium-style sealed box because HPKE is an IETF standard with published test vectors that the crate checks, and the crate reuses the RustCrypto crates already in the core (adding only `x25519-dalek` and `curve25519-dalek`); a sealed box would mean a pre-release `crypto_box`, an older one that duplicates those crates, or libsodium bindings. The private half is stored as a 32-byte seed (RFC 9180 `DeriveKeyPair`) wrapped by the master key with its own purpose label, bound to the public key. A sealed note is `version || recipient public key || encapsulated key || ciphertext || tag`, with HPKE `info` `terpoir/capture/v1` and the note's record ID in the associated data.
- **Not yet done:** locking key pages against swap (best effort per platform) is left to the FFI and platform layers.

## Location and saved spots
- Location is opt-in, per entry, coarse by default; precise only on request.
- Raw coordinates never leave the device except inside encrypted blobs.
- "Near a saved spot" detection runs on-device:
  - Android: GeofencingClient (Play services), with an AOSP LocationManager fallback for de-Googled devices.
  - iOS: Core Location region monitoring (~20 regions; rotate nearest set).
- Disclosed tradeoff: to geofence, the OS location daemon holds spot coordinates in plaintext on-device.
- Two modes: **Notify me** (geofences, "Always" permission, high consent bar) and **Show me** ("While Using" only; check nearby spots on app open; nothing registered with the OS).
- Minimum radius and cooldown so alerts don't become a visit log; dismissed alerts are not persisted.

- **Android implementation, part 1 (decided): spots and "Show me".**
  - **Spot editor:**
    - The name defaults to the first free "Spot N".
    - Purposes are the four built-ins plus the user's own; there are no relational suggestions.
    - Coordinates are optional. "Use approximate location" asks only for `ACCESS_COARSE_LOCATION`, so the system blurs the fix. "Use precise location" asks for fine location. Either can be removed.
  - **One-shot fixes:** taken through the platform `LocationManager` (fused provider on Android 12+, else network, and GPS only with precise permission), the same on both builds. Only while the app is open; nothing is registered with the OS.
  - **"Near" is decided in the core** (`Spot::nearness_m`, `Journal::spots_near`). A fix is near a spot when the great-circle distance is within the spot's radius plus the two fixes' accuracy, with that allowance capped at the radius, so a coarse fix never makes far-away spots near. The radius is the spot's own with a 100 m minimum, or 150 m when unset.
  - **"Show me"** is a Privacy Center toggle, off by default. With it on, opening the journal takes one fix and shows a banner for the nearest saved spot, with "Log a session here", which pre-attaches the spot. Without location permission it offers to ask. The fix is not kept. Off has nothing to clean up.
  - **Sessions** can be attached to a spot in the session editor.
  - Address lookup stays out (it would send coordinates to the OS geocoder); it remains a future opt-in.
- **Geofence alerts and notification tiers (decided 2026-10-11, revised the same day):** alerts are discreet by default ("Did you hit a spot?", never a spot, purpose or product). The maintainer chose full tiers: Named shows the spot's name, Rich adds the average rating, session count and last product. Both are opt-in, and both read a small **notification cache** outside the encrypted journal, as "Lock-state tradeoff" below describes. This replaces the earlier decision that every tier stays discreet until unlock. The lock screen always gets the discreet version (Android private visibility with a redacted public version).
- **Android implementation, part 2 (decided): "Notify me".**
  - **Toggle:** "Notify me at my spots" (Location, off by default). It needs precise location, location "all the time" and, on Android 13+, notifications. The journal asks for them one at a time, only after the toggle is on. Its off-action removes every geofence, deletes the notification cache and the quiet period, and clears a showing alert.
  - **Geofences:** registered only while the journal is unlocked, because they need the spots' coordinates. That happens on every unlock and after edits, so after a reboot alerts resume when the journal is next opened. No boot receiver and no plaintext copy of coordinates outside the journal and the OS location service. The 90 spots with the most sessions are watched, with each spot's radius (at least 100 m). Geofence IDs are opaque ("s0", "s1", ...), never record IDs or names.
    - **Play build:** `GeofencingClient`, alerting after a 3-minute stop (dwell), so driving past doesn't alert.
    - **foss build, and Play without Play services:** `LocationManager` proximity alerts, which alert on entering.
  - **What an alert shows** comes from the core (`Journal::spot_alerts`) at registration and is cached only as far as the tier allows. With no cache entry, the alert stays discreet. Tapping it opens the app; after unlock, the journal shows which saved spot is near, from a fresh fix, as "Show me" does.
  - **Cooldown:** at most one alert every 4 hours, whichever spot. Only the end of the quiet period is stored, not which spot fired. Dismissed alerts leave nothing behind.
  - **Tier toggles** (Notifications): "Spot names in alerts" (Named) and "Details in alerts" (Rich), both off by default. On rebuilds the cache now if the journal is open, else at the next unlock. Off narrows or deletes the cache, which works while locked.
  - **Play Console:** location "all the time" on Google Play needs the background-location declaration and a demonstration video before release.
  - **Left out of the first Play release (maintainer, 2026-10-13):** the Play build doesn't request `ACCESS_BACKGROUND_LOCATION` (its manifest removes it, and the permission allowlist allows it in the foss build only, so a Play manifest that requests it fails the build). `Distribution.BACKGROUND_LOCATION` is false there: "Notify me", purchase reminders and the two alert tiers aren't in its Privacy Center, and their stored settings read as off, so any fences an earlier build registered are cleared at the next unlock. "Show me" (while the app is open) is unchanged, as is the foss build. An instrumented test checks that the toggles match the manifest. Bringing it to Play later needs the declaration and video above.

### Spots: purpose types, never relationships
- Every saved location is a **Spot** with one or more purpose types describing what it is *for*, not whose it is:
  - **Blaze Spot:** where you consume.
  - **Munch Spot:** where you eat after.
  - **Re-up Spot:** where you buy.
  - **Chill Spot:** scenic or outdoor places worth returning to.
  - Custom types allowed; the app never suggests relational labels (home, friend's house, work).
- Rationale: relational labels reveal a residence and a social graph; purpose types are more useful for insights anyway.
- **No automatic place names.** Reverse geocoding sends coordinates to Google or Apple via the OS geocoder. Spots get user-chosen names with a neutral default ("Spot 7"). Address lookup is an opt-in Privacy Center toggle with that disclosure.
- Playful type names stay inside the app. Discreet-tier notifications and the discreet-icon mode use neutral wording ("a saved spot").
- **Coordinates are optional on every spot.** A Re-up Spot can be just a name (from a receipt or typed in); plenty of users won't want shop locations stored at all.

### Map picker (decided 2026-10-12, map providers revised the same day)
- **Pick a spot's location on a map,** as well as from the phone's location. No location permission is needed, and the phone's position is never read for it.
- **The map is online and opt-in.** An offline map download and handing off to a map app were considered; an in-app online map was chosen.
- **Toggle:** "Online map" (Location, off by default; in YOLO Mode). Turning it on, from the Privacy Center or the spot editor, shows the chosen provider's card first. Off stops every map request and deletes what the map stored. Picked locations stay with their spots.
- **Providers (maintainer, 2026-10-12): one choice among three, never combined.** The toggle says whether there is a map; the provider says whose. No project-held key is shared in the default path.
  - **OpenFreeMap (default, every build):** OpenStreetMap vector tiles from OpenFreeMap's public instance (`tiles.openfreemap.org`), drawn by MapLibre Native (BSD-2-Clause, no telemetry; its OpenGL build, since the default requires Vulkan). No API key, registration or cookies, and commercial use is allowed. Its card says that OpenFreeMap sees the IP address and the areas viewed, and that looking around a spot shows roughly where it is.
    - **Accepted risk:** the public instance is run by one person on donations, with no SLA. If it goes away, the default map stops working until the app changes provider. A user-entered tile server URL (for self-hosting) is a possible later addition; the project itself never runs one, under the "no project-run endpoint" hard line.
    - **Attribution:** always shows "© OpenMapTiles, data from © OpenStreetMap contributors", drawn over the map, not behind MapLibre's info button.
  - **MapTiler, with the user's own key:** the user pastes a MapTiler API key in the Privacy Center (Location). The key is kept in the encrypted store's metadata, on this device only, and never syncs. With it, the gateway also allows `api.maptiler.com` and MapLibre draws MapTiler's street map. Removing the key deletes it and switches back to OpenFreeMap. Its card names MapTiler as seeing the IP address, the areas viewed and the key (which identifies the user's MapTiler account). Attribution: "© MapTiler © OpenStreetMap contributors". Each user is on their own MapTiler plan, so the project needs no paid plan.
  - **Google Maps (Play only, an active choice):** the Maps SDK for Android, through Play services. The foss build never offers it (`verifyFossDependencies`; the existing §7 permission for Google Play libraries covers `com.google.android.gms` in the Play flavor).
    - **Offered early in onboarding** on Play builds that carry a Google key, as a choice with neither option preselected (like Speak/Type), with Google's card shown. Also a provider option in the Privacy Center and part of YOLO Mode on Play.
    - **Card:** Google sees the IP address and the areas viewed, through Play services, which is tied more closely to the device and Google account than the other providers. The SDK does its own networking, outside the gateway. Map styling stays basic (no map ID).
    - **Enforcement:** no Google map view is ever created unless the map is on and Google is the provider; the gateway cannot apply to it. Off stops creating map views, but cannot clear what Play services cached in its own process; the card says so, as ML Kit's does.
    - **The key:** a project-held Google Maps Platform key, restricted to Maps SDK for Android, the package `app.terpoir` and the SHA-1 fingerprints of the signing certificates (CI debug, local debug, the maintainer's release key and Play App Signing). Never committed. **Two keys (maintainer, 2026-10-13):** a debug key restricted to the debug certificates (CI and local), from the CI secret `GOOGLE_MAPS_API_KEY_DEBUG` or the `terpoir.googleMapsDebugKey` Gradle property; and a release key restricted to the release and Play App Signing certificates, from `GOOGLE_MAPS_API_KEY_RELEASE` or `terpoir.googleMapsReleaseKey`, which never reaches pull-request CI. Each goes into its build type's Play manifest (`com.google.android.geo.API_KEY`). Creating, restricting and rotating both keys is planned for Terpoir-infra (Terraform). A Play build without it doesn't offer Google. Mobile map loads without a map ID are not billed, so a leaked key restricted to the SDK carries little cost. **Cost (maintainer, 2026-10-13): keep Google Maps, guarded.** The map stays a basic map with no map ID (billed under the "Maps SDK" SKU, formerly "Mobile Native Dynamic Maps": free usage cap Unlimited, no price at any volume, per Google's pricing page read 2026-10-13) and uses no paid Google service (Places, Geocoding, Directions); adding any of these needs the maintainer's decision, since a map ID alone moves loads to Dynamic Maps: 10,000 free a month, then $7.00 per 1,000. MapTiler is only ever the user's own key, never a project key. Terpoir-infra keeps a $1 monthly budget alert on the Maps project (its D6).
- **Nothing is cached** for the MapLibre providers: MapLibre's tile cache is set to zero, since cached tiles would show which areas were viewed. Off deletes what MapLibre stored (its tile database and settings file).
- **The gateway** (`MapGateway`, the "one gateway" under "Android project"): MapLibre's only HTTP client. Before any connection it refuses every request unless the map is on and the request goes over HTTPS to the current provider's hosts: `tiles.openfreemap.org` for OpenFreeMap (the live style, its TileJSON's tiles, fonts, sprites and shaded relief all come from that one host, checked 2026-10-13), and `api.maptiler.com` only while MapTiler is the chosen provider. It follows no redirects, keeps no cache or cookies, and sends the User-Agent `Terpoir` instead of MapLibre's own (which names the phone model and Android version).
- **Permissions:** MapLibre's ACCESS_WIFI_STATE is removed everywhere, and its ACCESS_NETWORK_STATE in foss (Play keeps it for ML Kit); the app tells MapLibre it is online instead. Its native code logs only through MapLibre's Java logger, which the app silences.
- **Accuracy:** a pick records the ground the crosshair covers at that zoom (5 m to 50 km), so a pick made zoomed out is as vague as it looks.
- **Size:** MapLibre adds about 13 MB of native code per ABI, in every build.
- **Android (implemented 2026-10-13):**
  - **Provider:** a setting (`MapProvider`), OpenFreeMap unless the user picks another. The gateway asks for the toggle and the provider on every request, so a change applies at once. Style: OpenFreeMap's "Liberty".
  - **MapTiler key:** pasted in the Privacy Center's "Map provider" card under MapTiler's disclosure, kept by the core in the journal's local metadata (`Journal::set_maptiler_key`), never synced or exported. Saving it chooses MapTiler; removing it deletes it and chooses OpenFreeMap. A device-only flag records that a key exists, so YOLO Mode can choose MapTiler; it is corrected whenever the journal turns out to have no key (after a reset or a restore on another phone), and the map then falls back to OpenFreeMap.
  - **Toggle:** on chooses MapTiler if a key is saved, else OpenFreeMap (as YOLO Mode does); off also returns the provider to OpenFreeMap and keeps a saved key.
  - **Build:** INTERNET is in every build's manifest and allowlist; the project MapTiler key, its build config field and the keyless "offline" manifest are gone.
  - **Google Maps (Play):** `play-services-maps` in the Play flavor only, behind `Distribution.googleMaps`, which is null in foss and in Play builds without a key. Each build type gets its own key (see "The key" above). The map picker creates a Google `MapView` only while the online map is on and Google is the provider; with Google chosen the gateway lets nothing out for MapLibre. A basic map: no map ID, and buildings, indoor maps, the map toolbar, rotation and tilt off. Google draws its own credits.
  - **Choosing Google:** only an active choice. Onboarding asks once, after the recovery-key backup, on Play builds with a key ("Use Google Maps" or "Not now", neither preselected, with Google's card); "Use Google Maps" turns the map on with Google. The Map provider card offers Google after showing its card. YOLO Mode chooses Google on such builds (a preset can carry a choice through `PrivacyToggle.presetOnAction`); turning the map on by hand never does. Journals set up before this are not asked.
- **Superseded the same day:** the earlier plan of a project-held MapTiler key in every build with a map (`TERPOIR_MAPTILER_KEY`, `terpoir.maptilerKey`, the offline manifest for keyless builds, and a paid MapTiler plan for a release with the paid unlock).
- **F-Droid:** OpenFreeMap is free software end to end and MapTiler is opt-in with the user's own key, so the foss build should carry no NonFreeNet anti-feature; confirm in the F-Droid recipe review.

### Purchase prompts (decided)
- **Per-spot opt-in** on a Re-up Spot: "Remind me to log purchases here." Requires that spot to have coordinates; uses "Notify me" geofencing, so the Play build doesn't offer it yet (no background location; see "Notify me").
- **Trigger: leaving after a real stop**, not driving past: a minimum dwell inside the spot (a few minutes), then exit. Android geofences support dwell directly; on iOS the app times entry to exit.
- **Discreet at every notification tier:** "Quick log?" with no shop name. Tapping opens purchase capture, receipt scan first.
- **No visit log:** the entry time is held only until exit (in the encrypted store if the app is killed meanwhile), then discarded. Dismissed prompts leave no trace. At most one prompt per spot per day.
- **"Show me" alternative** (no "Always" permission): a banner when the app is opened while at a Re-up Spot; nothing registered with the OS.
- **Off-action:** removes purchase-prompt geofences and clears pending state.
- **Android implementation (2026-10-12):**
  - **Toggle:** "Purchase reminders" (Location, off by default, in neither preset). While it is on, the spot editor offers "Remind me to log purchases here" on Re-up Spots with a location. The core lists the opted-in spots (`Journal::purchase_prompt_fences`: coordinates and radius only).
  - **Fences:** prompt fences ("p0", "p1", …, at most 20) share one registration with "Notify me" fences ("s…", the rest of the 90). They need the same permissions, so the journal's setup card shows for either feature.
  - **Dwell, then exit:** on Play, Play services reports a 3-minute dwell and the exit. With proximity alerts (foss, or no Play services) the app times entry to exit: 3 minutes inside counts as a stop.
  - **No visit log (changed from the above):** the encrypted store is locked when the OS reports an event, so while inside a fence the app keeps only its opaque ID and the monotonic time of entry, in app-private storage, dropped on leaving. To keep to one prompt per spot per day, it keeps the opaque IDs prompted today until the day changes.
  - **The prompt:** "Quick log?" / "Tap to scan the receipt.", the same at every tier and public on the lock screen. Tapping it opens receipt scan after unlock.
  - **Off with the journal locked:** turning the toggle off removes every fence at once; "Notify me" fences return at the next unlock.
  - **"Show me" alternative:** at a Re-up Spot, the nearby banner also offers "Log a purchase".

## Notifications
- Local notifications only; nothing via APNs or FCM.
- Privacy tiers, default Discreet:
  - **Discreet:** "Did you hit a spot?"
  - **Named:** includes spot name.
  - **Rich:** ratings, last product, notes.
- Rich opt-in discloses where content lands: lock screen, notification history, notification-listener apps, wearables, OS summarization.
- Middle option: rich content only when unlocked (Android private visibility with redacted public version; iOS hidden-previews placeholder).
- Same tiering to apply to widgets and Live Activities later.

### Follow-up check-ins (decided)
- Opt-in per session; a local reminder after a method-based default delay (about 90 min for edibles, 45 for tinctures; none by default for inhaled), user-adjustable.
- Notification text stays generic at every tier: "Quick check-in?" Tapping opens a one-screen update (effect ratings, a stream snippet).
- Disclosure: a scheduled notification sits in the OS scheduler and reveals timing, not content.
- Off-action cancels all pending check-ins.
- **Android implementation (decided 2026-10-11):**
  - **Toggle:** "Follow-up check-ins" (Notifications, off by default). With it on, the session editor offers "Check in later" with a delay in minutes. The default comes from the session's methods: 90 minutes for edibles and drinks, 45 for tinctures, and no check-in by default for inhaled or topical use. The editor asks for the notification permission when it's missing.
  - **Scheduling:** an inexact `AlarmManager` alarm (`setAndAllowWhileIdle`), so no exact-alarm permission is needed. The alarm carries nothing. One check-in is pending at a time, and a newer one replaces it. A reboot drops a pending check-in (no boot receiver).
  - **Session ID:** waits in the app's private, no-backup preferences. It is an opaque random ID, meaningless without the journal, and is removed once the check-in is opened or cancelled.
  - **Reminder:** "Quick check-in?" at every tier, public on the lock screen because it says nothing.
  - **Check-in screen:** tapping the reminder opens it after unlock. It updates the session's effects, mood after and rating, and can add a note to its stream.

## Keyboards and voice input (decided)

### Keyboards
- **Default: the user's normal keyboard experience.** The user knows their own input preferences; the app trusts that choice.
- **Zero-cost protections stay on by default** because they don't change typing: on Android, the no-personalized-learning flag on all fields and opting fields out of autofill. iOS has no equivalent free protection.
- **Opt-in protections (Privacy Center), iOS:** block third-party keyboards app-wide; disable autocorrect and predictions on journal fields. Each with a disclosure card.
- **Android implementation:** all app content is wrapped in `PrivateTextInput`, a text-input interceptor that adds `IME_FLAG_NO_PERSONALIZED_LEARNING` to every text field, so a new field cannot forget it. Autofill is off app-wide: the window's root view is `IMPORTANT_FOR_AUTOFILL_NO_EXCLUDE_DESCENDANTS`.
- **Info card:** keyboards see everything typed before encryption; third-party keyboards can log and transmit keystrokes (on iOS when granted Full Access); system keyboards learn new words, and the iOS keyboard dictionary can sync via iCloud. On Android the app cannot control which keyboard is used, and third-party keyboards may ignore the no-learning request. Words already learned stay until the user resets the keyboard dictionary in system settings (link provided).

### Voice-first new notes
- The **new-note FAB opens voice mode by default**: speak, and the transcript becomes a stream snippet ready for tag suggestions. A keyboard button in the capture sheet switches to typing; the FAB can be set to text-first.
- **First-use card:** transcription happens on the phone and is never sent anywhere; keyboards can see typing, on-device voice can't be seen by anyone else; others nearby can hear you. The user then picks voice or text as the default.
- **Engine tiers:**
  1. **OS on-device recognition** (default where available): iOS must require on-device recognition; Android must explicitly use the on-device recognizer.
  2. **Bundled Whisper-class model** (~75–150 MB) in the Rust core; works on de-Googled devices; can share the download path with the bundled tagging model.
- **Never fall back to cloud recognition.** If on-device recognition is unavailable, say so and offer the bundled model.
- **Audio is discarded after transcription** by default; optionally kept as an encrypted attachment.
- Microphone permission is "while using" only; the OS recording indicator stays visible.
- Public-speaking caveat appears in the first-use card and voice-mode info text; one swipe switches to text.
- **Android implementation (decided):**
  - **Recognizer:** `SpeechRecognizer.createOnDeviceSpeechRecognizer` only, and only where `isOnDeviceRecognitionAvailable` says it exists (Android 12+). There is no code path to the general recognizer, which may use the network. Elsewhere (Android 11, or phones without one) the note screen says on-device recognition isn't available and offers typing. The bundled model is Phase 5.
  - **No audio reaches Terpoir:** the system recognizer returns text only, so there is nothing to discard or keep. "Keep voice clips" waits for the bundled model, which handles audio itself.
  - **Notes:** the main button on the Journal tab is "New note". It listens at once in voice mode and saves a standalone stream snippet. "New session" is the smaller button beside it. Notes show in the journal timeline with sessions, newest first.
  - **First-use card:** the first time the button is used, the user picks "Speak" or "Type". Either is a choice; neither is pre-selected. The card states on-device processing, that keyboards see typing, and that people nearby can hear.
  - **Permissions:** `RECORD_AUDIO`, asked on the first spoken note. A `<queries>` entry lets the app see whether a recognition service exists.

### Privacy Center entries
- **Keyboard** (iOS options above). Off-action: future typing only; card links to keyboard dictionary reset.
- **New note default:** voice or text. Android: the "New notes start with voice" toggle, in a "Keyboard and voice" section. Voice is its private state, as the design default, since on-device voice is seen by no one else while keyboards see typing. Neither side creates anything to clean up.
- **Voice engine:** OS on-device or bundled. Off-action deletes the bundled model.
- **Keep voice clips.** Off-action: delete stored clips (with confirmation) or keep existing ones.

## Quick capture (decided 2026-10-12)
Making a new note should take one tap from outside the app. Every entry point opens the same capture sheet in the user's default mode (voice or text, the "New notes start with voice" setting).

### Entry points
| Entry point | Default | Notes |
|---|---|---|
| App shortcut (long-press the icon): "New note" | On | Static, no content, like "Lock now". Optional pair "Voice note" / "Text note", the user's default listed first. Android can pin it to the home screen. |
| Android Quick Settings tile | Off | One swipe from anywhere. |
| Home-screen widget | Off | One "New note" button and nothing else: no last session, ratings or other journal content. |
| iOS Control Center / Lock Screen control, Action Button | Off | Same rules as the widget. |
| Voice assistants (App Actions, App Intents / Siri) | Off | The trigger phrase goes through Google's or Apple's assistant, revealing that Terpoir is used (not note content). Conflicts with "donate no activities", so it sits behind its own toggle and card. |

- **Discreet consistency:** every surface follows the discreet name and icon setting ("New note" with the neutral icon) and never shows journal content, so it reveals no more than the app icon already does.
- **Notification icon (maintainer, 2026-10-14):** notifications use a one-colour silhouette of the app icon (the open journal and its bubbles, `ic_notification`), which shows no more than the app icon already does. When the discreet icon setting exists, notifications follow it and use the neutral glyph.
- **Write-only capture (decided 2026-10-12, maintainer):** a quick capture works while the journal is locked, with no unlock step, and nothing it writes can be read until the journal is unlocked.
  - **Design:** the core keeps a capture key pair. The public key is stored in app storage (it is not secret); the private key is wrapped by the master key like a record key. A capture is transcribed on-device and sealed to the public key with a vetted public-key construction (HPKE or a sealed box, chosen at implementation), then written to a pending-captures inbox. On the next unlock the core opens the inbox, adds each capture as a stream snippet, and securely deletes the sealed files.
  - **Review before merge:** someone holding an unlocked phone could add notes they cannot read, so inbox captures are marked "added while locked" and the user keeps or discards them.
  - **Known costs:** a new crypto path in the core (key pair, versioned inbox format, re-keyed with master-key rotation); the inbox's file count and sizes are visible on disk while pending, inside the app's protected storage; tag suggestions wait for unlock. Audio is discarded after transcription as usual, or sealed with the note if the user keeps voice clips.
  - **Scope:** works while the device is unlocked and the journal is locked. Capture from the device lock screen is a further opt-in with its own card.
  - **Implementation (2026-10-13):** HPKE, recorded under "Crypto details". The capture key pair lives in the journal database's local metadata, so it never syncs and each device has its own. The inbox is `journal/inbox/` in no-backup storage: `capture.pub` (rewritten on every unlock) and one `<record ID>.cap` file per note, written atomically. A note holds its text and local time, padded to a multiple of 512 bytes. On unlock each note becomes a standalone snippet with that record ID, marked "added while locked" (a synced field, so every device shows the mark until it is cleared); a note already in the journal, or already discarded, is never added twice. Merged and unreadable files are overwritten with zeros and deleted (best effort on flash storage); files from a newer app version wait for an update. Master-key rotation makes a new pair and keeps the previous one, re-wrapped, so notes sealed to it in between still open. Review is on the journal timeline: a count at the top, and "Keep" or "Discard" on each marked note. Discarding is an ordinary deletion. Notes are text only, as no audio reaches Terpoir yet.
  - **Before the first unlock with this version** there is no public key, so entry points fall back to unlocking first.
- **Privacy Center entries:** Quick Settings tile, home-screen widget, lock-screen/Control Center control, voice assistant. Off-actions remove the tile, widget or control (or ask the user to, where the OS requires) and, for assistants, delete any donated intents. The app shortcut is covered by the existing quick-lock shortcuts and needs no toggle; it can be hidden in settings.
- **Android (implemented 2026-10-12, write-only capture 2026-10-13):** every entry point launches the app with a "new note" intent. While the journal is locked it opens the capture sheet: voice or typing in the user's default mode, no tags, with "Unlock instead"; saving seals the note and returns to the lock screen. While unlocked, the full note screen opens. The "New note" app shortcut is dynamic (not in the manifest), so the Shortcuts card in the Privacy Center can hide it, pinned copies included. The tile (`TileService`) and widget (`AppWidgetProvider`, one button) ship as disabled components; their toggles (OS integration) enable and disable them, which removes the tile from Quick Settings and the widget from the picker. Neither is in YOLO Mode: each only helps once the user places it.

## Visual design (decided)
- **Theme: Terroir**, Material 3, in light and dark following the system setting:
  - colors: moss green (primary), sage (secondary), amber resin (tertiary) on linen by day; the same hues lifted on a forest-floor dark by night;
  - type: Fraunces (a soft serif) for display, headline and large titles; Work Sans for everything else;
  - corners: chips 8, fields 12, cards 16, sheets 28.
- **Accessibility:** every text color meets WCAG AA (4.5:1) on every surface it is used on, in both modes. `ThemeContrastTest` enforces it.
- **The brand palette is used as is.** Wallpaper-based dynamic color is not applied; offering it as an option is undecided.
- **Fonts are bundled, never downloaded.** Downloadable Google Fonts go through Play services, which FOSS lacks, and they contact Google. Provenance: `android/fonts/README.md`.
- **Kept as ideas** from the same design exploration:
  - **Night Garden:** dark-first, sage and violet, Sora and Figtree. The maintainer's personal favorite.
  - **Terpene Pop:** bright violet, rounder shapes, effect chips colored by terpene, Bricolage Grotesque and DM Sans.

  Either could return, for example as an alternate theme or as accents.

## Accessibility and screen capture (decided)
- **Always on (zero cost):**
  - Android 14+: journal content marked accessibility-data-sensitive, so it is exposed only to services declared as genuine accessibility tools (e.g. TalkBack), not to other apps holding accessibility permission.
  - Both platforms: app-switcher blur.
- **"Screen protection" toggle, default on:**
  - Android: FLAG_SECURE on journal screens (blocks screenshots, recording, casting; blanks Recents). Screen readers, Switch Access, and Voice Access keep working.
  - iOS: blur content while the screen is recorded, mirrored, or AirPlayed. (iOS cannot block screenshots.)
  - Off-action allows screenshots and screen sharing; disclosure: captures land in the photo gallery and possibly cloud backups, and casting shows content on other screens.
- Rationale: unlike keyboards, this default protects against other software rather than overriding the user's own tool.
- Sharing has a deliberate path via share/export (see open decisions), so screenshots aren't the only option.
- **Android implementation (`app.terpoir.privacy.ScreenPrivacy`):** applied to the whole window. The app is a single activity and every screen shows or leads to journal content.
  - App-switcher blur: Android 13+ disables the Recents snapshot (`setRecentsScreenshotEnabled(false)`). Android 11 and 12 have no such API, so FLAG_SECURE is set whenever the app is not in front (`onPause`), even with screen protection off.
  - Accessibility-data-sensitive (14+): set on the window's root view, which covers the Compose UI's virtual accessibility nodes.
  - Screen protection is stored in `PrivacySettings` (default on) and applied on every resume, so a change takes effect at once. It is the first Privacy Center toggle (Device Security). Turning it off only clears FLAG_SECURE; nothing else needs cleaning up.

## Local storage format (decided)
- **Two layers:**
  1. **Per-record encryption inside:** each record is serialized and encrypted with its own key and stored as a blob. The same ciphertext is what syncs, so sync never re-encrypts and destroying a record key erases it everywhere.
  2. **SQLCipher outside:** the whole SQLite database is encrypted with a database key wrapped by the master key and held in Keystore / Keychain, hiding structure, row counts, and index columns.
- **Minimal on-disk indexes** (inside SQLCipher only): record IDs, record type, references between records, dates, confirmed tag IDs. **Free text is never indexed on disk**; search and insights build working indexes in memory after unlock.
- **Deletion hygiene:** `PRAGMA secure_delete=ON`; write-ahead log checkpointed and truncated regularly; periodic VACUUM.
- **Photos and voice clips:** separate encrypted files using streaming encryption under the owning session's key; erased when that key is destroyed.
- **Implementation (decided, `core/crates/terpoir-store`):**
  - SQLCipher via `rusqlite` with SQLCipher and OpenSSL built from source (no system crypto library needed on Android or iOS). Raw 256-bit database key (no SQLCipher KDF); `cipher_memory_security` on so SQLCipher wipes freed memory; WAL mode, `secure_delete`, in-memory temp store.
  - SQLCipher's own logging is compiled out (`SQLCIPHER_OMIT_LOG`, set in `core/.cargo/config.toml`); it would otherwise write to logcat or the Apple console.
  - Tables: records (ID, kind, wrapped record key, ciphertext), and index tables for references, local day, confirmed tag keys and file ownership. The sealed record is bound to its ID and kind, so rows cannot be swapped.
  - A record's key is created on first save and kept for its lifetime (its files' keys derive from it). Sync imports sealed records unchanged after verifying they decrypt and match their ID and kind.
  - Deleting a record removes its wrapped key, ciphertext, index rows and files, then checkpoints and truncates the WAL. Master-key rotation re-wraps every record key in one transaction.
  - The store never holds the master key; it is passed per call, so locking is dropping the master key and closing the store.
- **Primitives (vetted Rust crates only):** XChaCha20-Poly1305 for records, files, and key wrapping; Argon2id for passphrase-derived keys; HKDF for subkeys.
- **Lock-state tradeoff:** strongest OS file protection makes data unreadable while locked. Discreet notifications and check-ins need no journal data. **Named and Rich notifications use a small separate notification cache** (spot names, summary lines) readable after first unlock. The Rich/Named disclosure cards mention it; their off-action deletes it.

## Logs and crash reports (decided)
- **Release builds log nothing.** Logging is compiled out, not silenced:
  - Rust: release max level off; custom panic handler reports only the code location, never panic message contents.
  - Android: R8 strips logging calls.
  - iOS: logging compiled out of release builds.
  - CI lints fail the build on stray debug prints (e.g. `dbg!`, `println!`). Rust (`.github/workflows/rust.yml`): `cargo clippy -D warnings` on all targets, plus `dbg_macro` and `todo` denied everywhere; `print_stdout`, `print_stderr` and `unwrap_used` denied in library and binary code (tests may print and unwrap).
- **Debug builds follow the no-content rule:** sensitive values use wrapper types that print as `<redacted>`.
- **Android implementation:**
  - **App code does not log at all, in any build.** `verifyNoLoggingInSource` (part of `check`) fails on `android.util.Log`, `println`/`print`, `printStackTrace`, `System.out`/`err` or `java.util.logging` in the app's sources. Tests may print.
  - **R8 strips logging from release builds** (`proguard-rules.pro`), the app's and every library's (AndroidX, Compose, JNA):
    - `android.util.Log`, with `isLoggable` forced to false;
    - `java.util.logging.Logger` and `Throwable.printStackTrace()`;
    - Kotlin's `print`/`println`;
    - printing to any `PrintStream`, because Compose and JNA keep debug paths that print to `System.out`/`err` and Android sends those to logcat. The app never writes files through `PrintStream`; the core owns storage.
  - **The shipped bytecode is checked:** `verify<Variant>NoLogging` (release variants, part of `check`) runs `dexdump` on the release APK and fails if any of those calls survived R8. `printStackTrace(PrintWriter)` and `Log.getStackTraceString` are allowed: they only build a string (the UniFFI bindings and CameraX use them); a call that would log it is caught on its own. Int-returning `Log` calls are assumed to return 0, so R8 removes them even where Kotlin uses the result.
  - **Backstop:** a non-debuggable build points `System.out` and `System.err` at a stream that discards everything, for anything that reaches them some other way.
- **Recorded exception (accepted 2026-10-11): Tesseract's JitPack libraries.** The prebuilt Tesseract4Android libraries (text recognition, see "Rapid product entry") call Android's native logger directly, which R8 and the dexdump check cannot reach. They log fixed status lines only: "Initialized Tesseract API with language=eng" on each engine start, and a few error and assertion messages. Never image content, recognized text, keys, coordinates or identifiers.
  - **Closed for F-Droid's build:** the from-source libraries (`android/ocr-native/`) have logging compiled out, and the `foss` flavor uses them with `-Pterpoir.ocr.aar`. The OCR workflow's `foss-app` job checks the shipped libraries.
  - **Still open for JitPack builds** (Play, and app CI). It closes when every release build uses the from-source libraries (tracked in `docs/follow-ups.md`). No other native logging is allowed.
- **No crash-reporting SDKs and no analytics, ever.**
- **Disclosure:** Apple and Google collect OS-level crash reports (stack traces, device info, no journal data) when the user has opted in to share diagnostics with developers in device settings. The app cannot disable this; the privacy policy says so.
- **Opt-in diagnostics mode** (Privacy Center): records a local, encrypted, redacted log; auto-off after 24 hours. Sharing requires the user to review the full text, then send it via the share sheet. Nothing is sent automatically. Off-action deletes all diagnostic logs.

## Sharing and export (decided)

### Share an entry (session, product, or insight)
- User picks fields; private-leaning defaults:
  - Included: product, strain, ratings, effects.
  - Excluded by default: stream text, spot, targeted symptoms, exact times (date only or none), photos.
- Preview shows exactly what will be shared (rendered card or plain text).
- The card is a temporary in-app file handed to the share sheet and deleted afterward; it **never touches the photo gallery**. Included photos are already EXIF-stripped.
- Copy to clipboard marks content sensitive and sets it to expire.
- **Small, tasteful "made with Terpoir" mark included by default**, with a one-tap toggle in the share preview to remove it. Toggle label: **"C'mon, be cool, bud!"** Turning it off is instant and gets a friendly "No worries, we get it ✌️" — no extra step, no repeat nag. Rationale: the user is already sharing cannabis content, and word of mouth is how a no-ads, no-tracking app gets discovered.
- Notice: "Once shared, this is outside Terpoir's protection."
- **Android implementation (2026-10-12):** a Share button on saved sessions and products opens the share screen. It shares what is saved, not unsaved edits.
  - **Fields:** sessions offer product, strain, rating and effects (on), plus method and dose, intention, timeline, spot and notes (off). The date is a separate choice of none (the default), date only, or date and time. Products offer strain, form, THC and CBD, and terpenes (on), and batch (off). Targeted symptoms are offered (excluded by default) only while that section is on and the session has some; insights join once there are insights.
  - **Formats:** a card image or plain text, with a live preview of exactly what goes out. The "Made with Terpoir" mark and its toggle work as described above.
  - **The card is never written to storage.** Instead of a temporary file, an in-app content provider (`ShareProvider`, not exported) serves the PNG from memory to the app the user picks, with a one-URI read grant. The card is dropped when the share screen closes, so there is nothing to delete and nothing reaches the gallery.
  - **Copy text** uses the sensitive clipboard (hidden from previews on Android 13+, cleared after a minute).

### Full export
- **Encrypted export (default):** whole journal as an archive encrypted under the recovery key; same format as export/import sync.
- **Plaintext export (opt-in):** JSON + CSV + photos for portability, with a strong warning; saved only through the system file picker, noting that cloud-synced folders will upload it.
  - **Implementation (2026-10-12):** built in the core (`Journal::export_plaintext`). `terpoir.json` holds every record with every field, read generically from the stored documents so nothing is left out as the model grows. `sessions.csv`, `products.csv` and `purchases.csv` use names instead of record IDs. CSV cells that a spreadsheet would run as a formula get a leading `'`. JSON goes through `serde_json`, already in the dependency graph.
  - **Android:** "Export as plain files (unencrypted)…" in the Privacy Center's data card. A warning dialog comes first; the ZIP is then saved where the user picks, and the plaintext buffers are zeroed after writing. Photos join the export once the app has photos.

### Import
- From Terpoir encrypted exports. CSV import from other apps later.

### Encrypted export, import and restore: implementation (decided 2026-10-12)
- **Archive** (`terpoir-ffi`, `archive.rs`): the 8-byte magic `TERPOIR1`, then the master key wrapped by the recovery key (as the device already stores it), then the journal's full set of sync blobs, each length-prefixed. It adds no cryptography: the blobs are the sync blobs, encrypted under a key derived from the master key, so only the recovery key opens an export. Malformed, truncated or padded files are refused before any decryption.
- **Export** (Privacy tab, "Your data"): saved only where the user picks in the system file picker, as `terpoir-<date>.terpoir`. The card tells the user to keep the file and the recovery key apart.
- **Import** merges an export of the same journal back in (sync merge rules). An export of another journal is refused.
- **First run (maintainer, 2026-10-13):** right after the age gate, "Do you already have a Terpoir journal?", with "Start a new journal" and "I have a journal" as equal choices. There is nowhere to look a journal up (no account or server, app data left out of phone backups, and no access to the user's files without the picker), so the app asks. "I have a journal" offers an export file, a sync folder (see "Folder sync"), and the user's other phone, shown as "coming soon" until direct pairing exists.
- **Restore** (from an export file, on the first-run screen): the user picks the file and types the recovery key, which is checked against the export first. After the user authenticates, the master key is unwrapped (`keys_for_restore`), wrapped with a new Keystore key, and paired with a new database key for this device; then every blob is imported. Onboarding counts the recovery key as confirmed. A failure leaves no journal behind.
- **Tested (2026-10-13):** `terpoir-ffi/tests/export_round_trip.rs` fills a journal with every record type (every optional field set somewhere, references, an edit, a deletion), exports it, restores it on a new device, and compares every field of every record through the plaintext export. It also covers importing the same export twice, an older export merged into a journal that changed since (nothing is undone), a journal large enough to span several blobs, and damaged or foreign files. On Android, an instrumented test saves the export through the ContentResolver, reads it back, restores it into a new journal and compares the same way; the Keystore step of a real restore needs a fingerprint or PIN and is covered by the vault tests.
- **Not in an export, by design:** this device's own metadata: the MapTiler key, the capture key pair, the device ID and clock. (Insight feedback is a synced record, so it is in every export.)
- **Not yet:** photos and voice clips. The core can already attach encrypted files to a session (`add_attachment`), but an export carries records only, so attached files must be added to the archive before the app offers photos, or a restore would lose them. Also share cards and the plaintext export.

### Future enhancement (approved, post-launch)
- Terpoir-to-Terpoir sharing: a product or strain record travels as a small file or QR code that a friend imports into their own journal. No server.

## Memory hygiene and app unlock (decided)
- **Key material (Rust core):** held in self-zeroing types (`zeroize`, `secrecy`); avoid copies; lock key pages against swap/compression where the platform allows (best effort).
- **Decrypted content:** decrypt on demand, hold only what is on screen, drop promptly. Known limit: Kotlin and Swift strings cannot be reliably wiped, so the UI briefly holds plaintext; view state is cleared whenever the app goes to the background.
- **Auto-lock:** journal content cleared from view immediately on background (the app-switcher blur is unchanged); when the app locks, the master key is wiped from memory and reopening requires authentication.
  - **Default: lock with the device (decided 2026-10-12, maintainer).** No in-app timer: the app locks when the device locks or the screen turns off, and when its process ends. This matches how the user's other apps behave; a 1-minute default made every return a second unlock.
  - **Stricter timers, opt-in:** immediately, 1, 5, or 15 minutes after leaving the app, each with a short card. Locking the device still locks the app at once, whatever the setting.
  - **Disclosure (on the setting's card):** with the default, an unlocked phone handed to someone can open the journal if it was unlocked recently; the master key stays in memory while the phone is unlocked and the app is in the background. Users who share their phone can pick "Immediately" or an app passphrase.
- **Biometric unlock (crypto-bound):** the master-key wrapping key lives in Android Keystore / iOS Keychain and is usable only after OS-verified authentication, so a patched app cannot skip the prompt.
  - Default: **biometric or device passcode** (strong biometrics on Android; Face ID / Touch ID on iOS), which survives new biometric enrollments.
  - Optional strict mode: key bound to the currently enrolled biometric set; enrolling a new face or fingerprint invalidates it (re-unlock with passcode or recovery key).
- **Android implementation (decided):**
  - The wrapping key is an AES-256-GCM Android Keystore key, StrongBox-backed where the device has StrongBox (falls back to the TEE otherwise). It requires user authentication **on every use** (`setUserAuthenticationParameters(0, AUTH_BIOMETRIC_STRONG | AUTH_DEVICE_CREDENTIAL)`; strict mode: `AUTH_BIOMETRIC_STRONG` only and invalidated by new enrollments) and works only while the device is unlocked (`setUnlockedDeviceRequired`). Unwrapping happens through the system BiometricPrompt with a `CryptoObject`, so the OS, not the app, gates the key.
  - The platform BiometricPrompt is used directly (Android 11+ has everything needed), so there is no androidx.biometric dependency; the app holds the normal `USE_BIOMETRIC` permission.
  - **Prompt on open (maintainer, 2026-10-14):** a locked journal shows the unlock prompt by itself each time the app comes into view (launch, or back from the background), instead of waiting for an "Unlock" tap. Once per appearance: cancelling leaves the locked screen, whose button asks again, and a restart of the activity by the prompt's own PIN screen doesn't count as coming back, so a cancel can't loop. Opening a quick note while locked still goes straight to write-only capture. The key still needs authentication on every use; only the extra tap is gone.
  - The unwrapped master key goes straight into the core (`Journal.open`) and the Kotlin array is wiped; Kotlin never keeps it. Wrapped keys live in no-backup app storage.
  - Auto-lock: leaving the app hides journal content at once; with an opt-in timeout, the master key is wiped after it by a timer, and the elapsed time is checked again on return because Android may freeze or kill a backgrounded app before the timer fires. Screen-off locks immediately, and returning to a locked device locks too.
  - The choice is an **App lock** card in the Privacy Center (implemented 2026-10-12): "When my phone locks (default)", "As soon as I leave Terpoir", or 1, 5 or 15 minutes, with the default's disclosure shown while it is selected. It is a setting, not a privacy toggle: presets and "Restore maximum privacy" leave it alone, as the recorded exception allows.
  - Known limit: while a backgrounded process is frozen, the master key stays in its memory until the process resumes or is killed; device lock and screen-off are the reliable triggers.

## Age gating (decided)
- **Store rating at the top tier:** 18+ on the App Store, Mature on Google Play. Parental controls block installs on supervised devices.
- **Platform age signals, checked on-device:** Apple Declared Age Range API and Google Play Age Signals API (verify current availability). If the signal indicates a minor, show only a "Terpoir is for adults" screen. Read the category only; never store or transmit it. No server involved.
- **Self-attestation on first launch:** "I'm of legal age to use cannabis where I live." One local yes/no; no date of birth. Sole gate where age signals are unavailable (older OS versions, de-Googled devices).
- **Disclosure:** the age category comes from Apple or Google; Terpoir only reads it.
- Context: state app store accountability laws (Texas, enjoined; Utah, Louisiana, California upcoming) require runtime use of these APIs; re-check status before launch.
- **Android implementation (`app.terpoir.onboarding`):** one age gate in both flavors.
  - **The platform signal (`AgeSignals`) comes first, and it can only block.** An upper age bound under 18 shows only "Terpoir is for adults".
  - **It never passes anyone.** Google's top range is 18+, and the legal age for cannabis is often higher (21 in most US states, 19 in most Canadian provinces). Everyone not blocked answers the self-attestation.
  - **Stored:** only the pass, never the signal's range or a "no", so a "no" is asked again next launch.
  - **Play flavor:** Google Play Age Signals (`checkAgeSignals`), through IPC to the Play Store; the app opens no network connection of its own for it. A disclosure on the age screen says the range comes from Google and is checked on the phone only.
    - Terpoir never calls `requestAgeSignalsAccess`, which would prompt the user to share their age with the app.
    - Re-check before launch whether state laws require that prompt.
  - **FOSS flavor:** no signal; self-attestation only.

## Device security features (decided)
- **Unlock options** (default remains biometric or device passcode):
  - **Device passcode only:** disables biometrics (some users prefer this given US courts' greater willingness to compel biometrics than passcodes).
  - **App passphrase:** separate from the device passcode, so someone who knows the phone's passcode still can't open the journal. Argon2id-derived; escalating delays on failed attempts.
- **Quick-lock:** in-app lock button and a "Lock now" app-icon shortcut, plus lock-on-device-lock. No gesture triggers (they misfire).
- **Panic wipe** (both behind confirmation):
  - **Wipe this device:** destroys the local master key, crypto-erasing the journal on this phone; synced encrypted copies remain and can be restored with the recovery key.
  - **Destroy everything:** also deletes synced blobs. Irreversible.
- **No duress passcode.** Rejected: easily triggered by mistake, and destroying data under legal compulsion can itself be a crime (obstruction, evidence tampering). Panic wipe stays a calm, deliberate action.
- **Discreet icon and name:**
  - Android: icon and launcher name can change (e.g. "Journal" with an abstract icon).
  - iOS: icon only; the name stays "Terpoir" (platform limit, stated on the option's card).
  - Neutral alternatives only; never impersonate a real app.
  - Disclosure: the app still appears as Terpoir in system settings, the app store, and storage listings.

## Builds and releases (decided)

### Distribution channels
- **Google Play** (mainstream Android).
- **F-Droid** (de-Googled users; builds from source and can publish the maintainer-signed APK when builds match byte for byte). Requires a **FOSS build flavor** without Play services: AOSP location fallback, no Gemini Nano tier, no Play age signals, no Play Billing, no Google Maps.
- **GitHub Releases:** maintainer-signed APKs (works with update tools such as Obtainium).
- **App Store** (iOS).

### Android project (decided, `android/`)
- **Toolchain:** Android Gradle Plugin 9 with built-in Kotlin and the Compose compiler plugin; Gradle wrapper pinned by SHA-256 (CI also checks the wrapper JAR against Gradle's published checksums); versions in `gradle/libs.versions.toml`, each with its reason.
- **SDK levels (decided):** `minSdk 30` (Android 11), `targetSdk`/`compileSdk 37`.
  - Android 11 is the oldest release where a Keystore key can require a strong biometric **or** the device credential on every use (`setUserAuthenticationParameters` with `AUTH_BIOMETRIC_STRONG | AUTH_DEVICE_CREDENTIAL`), and where the unlock prompt can offer both for a crypto-bound operation. That is exactly the crypto-bound "biometric or device passcode" unlock; on Android 10 it would need a weaker time-window key or a separate passcode path. Accepted cost: Android 10 devices are not supported.
  - Newer hardening is used at runtime where the device has it, with a safe fallback, rather than by raising the minimum: hiding other apps' overlays on sensitive screens (12+), approximate-location choice (12+), sensitive clipboard flag (13+), accessibility-data-sensitive marking (14+), screenshot and screen-recording callbacks (14+/15+).
- **Flavors:** `play` and `foss` (dimension `distribution`), same application ID `app.terpoir`. Google-dependent features are listed as `Capability` values; only `play` may offer them. `verifyFossDependencies` fails the build if the FOSS release classpath contains Play services, Firebase, Play Billing, Play Core/age signals, ML Kit, Google Maps or on-device Gemini libraries.
- **Permissions:** every variant's merged manifest is checked against an allowlist in `app/build.gradle.kts`, so a library cannot add one silently. INTERNET is allowed in every build, for the opt-in online map (OpenFreeMap needs no key).
  - **INTERNET arrives with the first feature that needs it (decided),** not earlier. That feature is the opt-in online map (2026-10-12, "Map picker"). Other downloads still go through the store (Play Feature Delivery and Play Asset Delivery run in the Play Store app, over IPC). With INTERNET:
    - All network access goes through one gateway that refuses any request whose Privacy Center toggle is off, so with the defaults the app opens no connection at all. Today that is `MapGateway`, MapLibre's only client; unit tests check it. A device test that the app opens no connection with the defaults is still to come (`docs/follow-ups.md`).
    - **Exception: ML Kit's usage metrics on Play (maintainer decision, 2026-10-12).** Google Data Transport sends them itself, not through the gateway, and only while "Google text recognition" is on; ML Kit is not used otherwise. Its card discloses this.
    - **Exception: Google Maps on Play (maintainer decision, 2026-10-12).** The Maps SDK and Play services do their own networking, not through the gateway, and only while the map is on with Google as its provider. Its card discloses this.
    - Android does not let users revoke INTERNET (some systems, such as GrapheneOS, do); the permission's card says so and points to the per-toggle controls.
    - Syncthing folders and Google Drive through the system file picker need no INTERNET permission in the app.
- **Backups:** `allowBackup=false`, plus full-backup and data-extraction rules that exclude every domain from cloud backup and device-to-device transfer.
- **Dependency verification:** `gradle/verification-metadata.xml` pins a SHA-256 for every artifact; regenerate it with `./gradlew --write-verification-metadata sha256 assemble check` when dependencies change (Dependabot does not).
  - **Two trusted hashes for one file (decided 2026-10-11):** the Kotlin Compose plugin's marker POM is published by both Maven Central and the Gradle Plugin Portal with the same content, formatted differently. Gradle falls back to the Portal when Maven Central rate-limits CI, so both hashes are trusted (`also-trust`). The marker only names the plugin artifact, whose own files are pinned to a single hash. Any other second hash needs the same review.
- **Release builds** are minified with R8 and never signed in the build; the dependency-info blob that AGP normally embeds for Google is disabled.
- **Rust core in the app (decided):** Gradle cross-compiles `core/` (`terpoir-ffi`) with cargo-ndk for `arm64-v8a` and `x86_64` only (64-bit; no 32-bit devices), and generates the Kotlin bindings from an unstripped host build with the core's own `uniffi-bindgen` (release libraries are stripped, which removes UniFFI's interface metadata). Nothing generated is committed. The bindings call the library through JNA (Apache-2.0). The native libraries are 16 KB page-aligned and link only `libc`, `libm` and `libdl`: no `liblog`, so the core cannot write to logcat.
- **On-device tests:** CI runs the instrumented tests on an Android 11 (minSdk) emulator image without Google services, through plain `adb`/`am instrument` so no extra test-runner artifacts are fetched at run time.

### Signing keys
- **Play App Signing:** Google holds the key that signs Play builds. Disclosed; F-Droid and GitHub builds are signed with the maintainer's own key, and both certificate fingerprints are published on terpoir.app and in the README.
- **Maintainer Android signing key on YubiKey:** generated once on an offline machine, imported into **two YubiKeys** (primary and backup, stored separately) via PIV, one encrypted offline backup kept, machine copy wiped. Signing through `apksigner` with Yubico's PKCS#11 library; PIN and touch required. APK signature scheme v3 key rotation available if ever needed.
- **Play upload key:** may also live on a YubiKey (lower stakes; Google can reset it).
- **Apple distribution certificate:** dedicated Mac keychain (Apple re-signs App Store builds; certificates are quickly revocable).
- **Git tags and release manifests:** signed with an SSH `ed25519-sk` key on the YubiKey.
- Signing material never in CI or the repo.
- **Release pipeline (2026-10-13; runbook in `docs/releasing.md`):** a `v*` tag on `main` (matching `versionName`) runs `release.yml`, which builds the unsigned foss and Play APKs and the Play bundle in the `release` environment, runs every check, and puts them with `UNSIGNED-SHA256SUMS` in a draft GitHub release. The maintainer signs on their own machine with `android/scripts/sign-release.sh`, through the YubiKey's PKCS#11 module (`libykcs11`): the APKs with the app key (APK Signature Scheme v3, as minSdk is 30; no v4 `.idsig`), the bundle with the Play upload key. The script checks the downloads against CI's hashes, verifies every signature, refuses an APK whose certificate differs from `android/release/app-certificate.sha256` once that is set, writes `SHA256SUMS`, and can replace the draft's unsigned files with the signed ones; publishing stays manual. The signed APK is CI's build plus the signature, which is what F-Droid's reproducibility check compares. The upload key is generated on the primary YubiKey (slot `82`, say), since Google can reset it.
  - **GitHub offers only the foss APK; Play builds go to Google Play (maintainer, 2026-10-13).** A Play APK installed from GitHub has no way to get text recognition, which only Google Play delivers, so the release workflow no longer builds or attaches it. `android/scripts/play-upload.sh` sends the bundle to Google Play from the maintainer's machine, where it is signed: one Google Play Developer API edit (bundle, R8 mapping, track), committed only when every step succeeded. Default: the internal track, as a draft for the maintainer to roll out. No key file anywhere: the maintainer's gcloud login gets a 10-minute token of a Play publisher service account (Terpoir-infra) from the IAM Credentials API, scoped to the Play Developer API; the service account can only release to the tracks the Play Console grants it. The first release and the app itself are set up by hand in the Play Console, as the API requires.
- **Release wrapper (2026-10-13):** `android/scripts/release.sh vX.Y.Z` chains the manual steps on the maintainer's machine: signs and pushes the tag, waits for the Release workflow, runs `sign-release.sh --upload`, and asks before publishing. Considered and rejected: a self-hosted runner holding the YubiKey (needs the PIN as a CI secret, and any workflow code on it can request signatures) and a cloud KMS (moves the key off the YubiKeys). Signing stays offline.
- **iOS caveat (disclosed):** App Store binaries cannot be matched byte for byte to the source because Apple encrypts and re-signs them.

### Build pipeline (GitHub Actions)
- Reproducible Rust and Android builds, with a CI job that rebuilds and compares hashes.
  - Core (decided): `core/scripts/reproducible-build.sh` builds the committed source (`git archive`) at a fixed path with a fixed Cargo home, the pinned toolchain and `SOURCE_DATE_EPOCH` set to the commit time, because OpenSSL compiles in its install path and build date. The `Reproducible build` workflow runs it from two differently placed checkouts with a clean cache and requires identical hashes. Android targets join the same check in Phase 4.
  - OCR native libraries (decided): the open text-recognition engine is Tesseract through [Tesseract4Android](https://github.com/adaptech-cz/Tesseract4Android) (Apache-2.0), whose JitPack AAR carries natives built by JitPack, which F-Droid does not accept. `android/ocr-native/build.sh` rebuilds that AAR from pinned upstream release tarballs (Tesseract, Leptonica, libpng, IJG libjpeg; from Tesseract4Android only its CMake files, JNI wrapper and Java API), each checked by SHA-256, with the app's pinned NDK for `arm64-v8a` and `x86_64`, 16 KB-aligned, and with the wrapper's logcat calls compiled out (no `liblog`). The `OCR native libraries` workflow builds it on two runners from different paths and requires identical outputs matching the committed `android/ocr-native/SHA256SUMS`. It is asynchronous (weekly, on demand, and when the pins or recipe change) and never a required check, so app CI is no slower. App CI keeps the JitPack AAR; a Gradle property (`terpoir.ocr.aar`) points the `foss` flavor at the from-source AAR instead, and F-Droid's recipe sets it (`android/ocr-native/README.md`). With the property set, JitPack is not declared at all. The workflow's `foss-app` job builds the `foss` release that way and checks that the shipped OCR libraries don't log. Its `foss-device` job builds the `foss` debug app the same way and runs `BundledTextEngineTest` on the Android 11 emulator, so the from-source libraries are shown to load and read text on a device.
- Lockfiles committed; Gradle dependency verification; `cargo-deny` (licenses, advisories); `cargo-vet` (audits).
  - `cargo-deny` policy (`core/deny.toml`): fail on known vulnerabilities and on yanked, unmaintained, or unsound crates; licenses limited to a GPL-3.0-compatible allow list (Apache-2.0, BSD, CC0, ISC, MIT, MPL-2.0, Unicode-3.0, Zlib), extended only with a stated reason; crates.io as the only source; crash-reporting and telemetry crates banned.
  - `cargo-vet`: every Rust dependency audited, imported from a trusted auditor, or explicitly exempted (`core/supply-chain/`).
- Actions pinned by commit hash; release builds only from protected `main` and signed tags.
- **CI cost (decided 2026-10-11):**
  - A new push to a pull request cancels that pull request's older runs.
  - Pull requests skip the jobs that check nothing they change (`.github/scripts/changes.sh`): Android and CodeQL run for `android/`, `core/` or `.github/` changes; Rust, Reproducible build and Supply chain for `core/` or `.github/`. Markdown-only changes run none of them. Pushes to `main` and scheduled runs always run everything. A skipped job still reports, so it can be a required check.
- **Test builds (decided 2026-10-12):** the Android job uploads its debug APKs (`terpoir-debug-apks`, kept 14 days) so the maintainer can install the app before releases exist. They are debug-signed and keep logging, so they are for testing only. The foss build is the one to install, because the Play debug build lacks text recognition, which only Google Play can install for it. The throwaway debug signing key is cached between runs (saved only from `main`) so each build installs over the last. It is never a release key: release signing stays offline on the maintainer's hardware key. A release-signed build will not install over a debug one (uninstall first, which deletes the test journal).
  - **Development builds on GitHub Releases (maintainer, 2026-10-12):** each push to `main` also publishes the two debug APKs as one rolling pre-release tagged `dev-build`, titled "Development build <commit> (unstable)" and never marked latest. Its notes warn that it is buggy, say which APK to install, and give the commit; a `SHA256SUMS` file lists the APKs' hashes. Each push replaces it, so it is always at the same address. A separate job publishes it, the only one allowed to write releases; it downloads the APKs the Android job built and checks out no code.
    - The default map needs no key. The Play APK carries the Google Maps key, which is restricted to Maps SDK for Android, the package and the signing certificates' SHA-1s; mobile map loads are not billed, so extracting it gains little.
  - Downloaded dependencies (Gradle modules and wrapper, the Cargo registry, cargo-ndk) are cached. Only the Android job on `main` saves the cache, so a pull request cannot seed it, and dependency verification and `Cargo.lock` checksums still apply to cached files. Build outputs are never cached, and the reproducible-build jobs use no cache.
- SBOM published with each release.

### Transparency and updates
- Release hashes published on terpoir.app and signed; later, logged to Sigstore's public transparency log.
  - **Published so far (2026-10-14):** terpoir.app ("Verify your download") and the README carry both certificate fingerprints and the release-signing SSH key, also committed as `android/release/allowed_signers` (principal: the tagger's GitHub noreply address, namespaces `file` and `git`, so one file verifies both `SHA256SUMS.sig` and `git verify-tag`). The hashes themselves stay in each GitHub release's signed `SHA256SUMS` rather than being pasted onto the site by hand, which would go stale; publishing them on terpoir.app is automated in 0.7.0. A second key from the backup YubiKey is added to the same places when it exists.
- No in-app updater; stores and F-Droid handle updates (preserves "the app never contacts terpoir.app").

## Monetization (decided)
- **Principle: never paywall privacy or security.** Paying unlocks fun, never safety.
- **Free core forever:** journaling, sync, insights, and every privacy feature.
- **One-time "Connoisseur" unlock** on store builds (roughly $4.99–$9.99): alternate icons and themes, extra share-card styles, fun extras. Purchases via Apple/Google billing, so the maintainer learns of a sale, not a person.
- **Tip jar** in-app purchases on store builds (unlock nothing).
- **Donations** for FOSS builds (GitHub Sponsors or Liberapay links); F-Droid and GitHub builds include all features under the GPL.
- **Merch later:** stickers and print-on-demand apparel.
- **Ruled out:** dispensary partnerships, affiliate links, sponsored strains, ads, and data sales.

## App store policy (decided)

### Apple
- **Guideline 5.1.1(ix):** apps providing services in highly regulated fields, explicitly including legal cannabis use, should be submitted by a legal entity, not an individual. Apple organization enrollment excludes sole proprietorships and DBAs; it requires a legal entity, D-U-N-S number, work email on the organization's domain, and a functional public website.
- **Plan:** submit iOS as an individual with review notes (private personal journal; sells, delivers, and promotes nothing; stores no user data). If rejected under 5.1.1(ix), decide then between forming an LLC (assign trademark and copyright to it; replace the terpoir.app null MX with mail forwarding for the organization email) or remaining Android-only. **iOS is a stretch goal, not a commitment;** Android-only is an acceptable outcome.
- **1.4.3:** no encouraging consumption. The app records and reflects; playful copy may joke about sharing the app, never "consume more."
- **2.3.8:** store metadata (icon, screenshots, previews) must be all-ages appropriate even though the app is 18+. No explicit drug imagery; consistent with discreet branding.
- **App icon (maintainer, 2026-10-13):** an open journal with amber bubbles rising from its spine: the notebook, and terpenes, which are mostly liquid at room temperature. It replaces the placeholder leaf, which read as anatomical. Colours: background `#2E4A3B`, pages `#E8E2D0`, bubbles `#D9A441`. The spine is a gap, not a line, so the one-colour themed icon keeps both pages. The Play icon and feature graphic are rendered from the same shapes (`android/release/store/render.py`).
- **2.3.6:** honest age-rating answers (see Age gating).

### Google Play
- Prohibits apps that facilitate the sale of marijuana regardless of legality (in-app ordering, arranging delivery or pickup, selling THC products).
- **Rule for Terpoir: no ordering, no delivery, no dispensary menus or purchase links**, including via catalog providers. Re-up Spots and price fields are the user's own records.
- No entity requirement; Android ships under a personal developer account. Check current closed-testing requirements for new personal accounts at signup.
- **First submission (maintainer, 2026-10-13):**
  - **Identity:** developer name "Terpoir", public contact `terpoirapp@gmail.com`, website `https://terpoir.app`, privacy policy `https://terpoir.app/privacy` (`site/privacy.html`).
  - **Declarations:** health apps declaration for Sleep Management only (maintainer, 2026-10-14). Medication and Treatment Management was declared at first, but it is in the form's Medical group, which Google expects from organization accounts; Play rejected it for the personal account. An organization account (an LLC plus a D-U-N-S number) stays the fallback if Google later treats Targeted symptoms as medical.
  - **No payments profile yet:** no Connoisseur unlock or tip jar in this release.
  - **No background location** in this release (see "Notify me").
  - **Answers:** every Play Console answer (listing copy, Data safety from the merged manifest and Google's SDK disclosures, IARC, target audience 18+, ads, advertising ID, sign-in details, foreground service) is in `docs/play-listing.md`, checked against the Play Console Help Center (2026-10-13). Store graphics are in `android/release/store/`.
  - **In-app policy text (maintainer, 2026-10-13):** the app shows the privacy policy as text (Privacy Center > "Privacy policy"), not as a link: `site/privacy.html` is copied into the assets at build time, so there is one source and nothing is fetched. Links to terpoir.app in it show as plain text, so the app never sends anyone there; links to other sites (Google's, OpenFreeMap's policies) open in the browser only when tapped.
  - **Play policy text in the app:** a reminder to consult a healthcare professional on Insights and in Targeted symptoms; a disclosure, with Continue, before the camera and microphone permission prompts.
  - **Developer verification:** the package name is proven with `android/scripts/sign-verification-apk.sh`, which signs a code-free APK carrying the Console's snippet with the YubiKey app key.

## Data model

### Core idea: an entry is a session
Each journal entry records one consumption session, not a product review. Product and strain ratings are derived from many sessions, which is what makes personal patterns visible.

### Record types
- **Session (journal entry)**
  - When: date and time, editable after the fact.
  - Product lines: one or more, each with product, delivery method, and dose. Stored as a map keyed by line ID.
  - Context (all optional): activity, intention (sleep, relax, create, relief), mood before, and **solo / with others**. No place or relationship labels.
  - Effects (decided): map of tag → intensity 0–5. Fixed core vocabulary (relaxed, euphoric, focused, creative, sleepy, hungry, anxious, paranoid, dry mouth, couch-lock) plus user-defined custom tags.
  - Timeline: onset, peak, duration.
  - Outcome: mood after, overall rating, would repeat.
  - Stream (see below), photos, optional Spot.
  - Targeted symptoms (optional section, see below).
- **Product:** name, form (flower, pre-roll, vape cart, concentrate, edible, tincture, beverage, topical), brand, strain(s), label data (THC/CBD % or mg per unit, top terpenes), batch/lot, package date, and optional scanned identifiers (barcode / package tag). Where and for how much it was bought lives on Purchase records, not here.
- **Purchase (decided):** date, optional Re-up Spot, and line items (map keyed by line ID), each with product, batch, and price. A product links to every purchase of it, so the same product bought at two shops can be compared. Session product lines may reference the purchase line they came from.
- **Strain / cultivar:** name, lineage, dominant terpenes, indica/sativa/hybrid label (richer detail from 0.4.0: see "Richer strains"). Insights are driven by terpene and cannabinoid profiles, not the label, which is scientifically weak.
- **Brand.**
- **Spot:** see Location and saved spots.

### Delivery methods and doses
- Method-specific units: hits or bowl fraction (flower), puffs (carts), dab size (concentrates), mg (edibles), ml or mg (tinctures).
- Where label data allows, the core also estimates **mg THC** so sessions compare across methods.

### Targeted symptoms (decided)
- Optional section, **off by default**, enabled in the Privacy Center; hidden until enabled.
- Symptoms, never conditions or diagnoses: suggestions (sleep, pain, anxiety, appetite, nausea, focus) plus custom.
- Each targeted symptom rated **👍 helped / 👎 didn't help**, skippable when unsure.
- Off-action asks whether to hide or delete existing symptom data.
- Framed as self-tracking, never advice.
- **Implementation (2026-10-12):**
  - **Core:** built-in symptoms are `TargetedSymptom` (sleep, pain, anxiety, appetite, nausea, focus; custom ones are tags of category `symptom`). Results are helped, did not help, or `not_rated` ("Not sure", the skip). Built-in tag slugs are tested to be unique across every vocabulary. `Journal::delete_symptoms` clears every session's symptoms (the removals sync like edits) and deletes the custom symptom tags.
  - **Android:** the Privacy Center toggle "Targeted symptoms" (new group: Journal; off by default; not in either preset). While on, the session editor shows the section after Effects; while off, saved symptoms are carried unchanged and nothing shows them.
  - **Turning it off by hand** asks "Keep, hidden" or "Delete". Presets and "Restore maximum privacy" only hide, so data is never deleted without asking. Toggles can now carry such a choice (`OffChoice`).

### What I track (decided 2026-10-12)
- **The user decides which data points their journal has.** Someone with no interest in brands, prices or strains can hide them, and the app stops asking. A field nobody fills in is data that never exists.
- **Where:** a "What I track" screen in Settings, not the Privacy Center (maintainer, 2026-10-12). The Privacy Center covers what leaves the app or relaxes a protection; this is the journal's own scope. Targeted symptoms stays in the Privacy Center because it is health-adjacent. Not an onboarding step: everything starts shown.
- **Can be hidden:** brand; strain (and strain type); price and shop, or purchases as a whole; THC/CBD; terpenes; batch and package date; dose; timeline; mood before and after; intention; solo / with others.
- **Always shown (the minimum for a useful journal and for insights):** when, product, method, effects and rating.
- **Turning one off** asks "Keep, hidden" or "Delete", as targeted symptoms does. Hidden data is carried unchanged by every editor (they already keep fields they don't show). Delete clears that field on every record through a core function, and the removals sync like edits.
- **Synced (maintainer, 2026-10-12):** the choices live in one tracking-preferences record, encrypted and synced like any other, so every paired device shows the same fields. Its ID is derived from the master key (HKDF, a fixed purpose label) rather than random, so devices that each create it address the same record and its per-field stamps merge (last writer wins per field). Field names a device doesn't know (from a newer app) are kept.
- **Effects elsewhere:**
  - Insights skip features built on a hidden field (strain type when strain is hidden; shop insights when price or shop is hidden).
  - Scans skip what is hidden: no prices from receipts when price is hidden, no brand or strain from labels when those are hidden.
  - Share cards don't offer hidden fields.
  - A product shows without its brand when brand is hidden ("Product names").
  - The plaintext export still includes data kept hidden, because it exports what is stored.
- **Core (implemented 2026-10-13):**
  - **Data points** (`TrackedField`, slugs in display order): `brand`, `strain` (with strain type), `price_and_shop`, `purchases` (hiding it also hides `price_and_shop`), `potency` (THC and CBD), `terpenes`, `batch` (with package date), `dose`, `timeline`, `mood` (before and after), `intention`, `social`.
  - **The record:** a new record kind, `Tracking` (code 9), with one map entry per hidden data point (`hidden/<slug>`), so choices merge per data point. Its ID is the first 16 bytes of an HKDF subkey of the master key (`tracking-preferences`); master-key rotation moves it to the new ID with its fields and stamps. It is a setting, so the plaintext export leaves it out. An app from before this version defers the record when syncing (an unknown kind), like any record from a newer version.
  - **Delete** (`Journal::delete_data_point`): clears the data point's fields on every record and, for brand, strain and purchases, deletes those records too (crypto-erasure); sessions' purchase references and shops go with purchases. Spots stay, since they are places, not shops. The removals sync like edits.
  - **FFI:** `trackable_data_points`, `Journal::hidden_data_points`, `set_data_point_tracked` and `delete_data_point`.
  - **Insights (in the core):** no terpene features with terpenes hidden, no strain-type features with strains hidden, no dose confounder with dose hidden. Shop insights are empty with prices and shops hidden; within them, brand and strain effects need brands and strains, freshness needs package dates, and product labels drop the brand when brands are hidden.
- **Android (implemented 2026-10-13):**
  - **Where:** the app has no separate Settings screen yet, so "What I track" opens from a sliders button in the journal's header (beside "Lock now"), outside the Privacy Center as decided. One switch per data point; turning one off asks "Keep, hidden" or "Delete" (with "can't be undone"). Prices and shops show as off while purchases are off.
  - **Everywhere else** through one `LocalTracking` value that JournalHome loads from the core and refreshes when a screen closes:
    - the session editor leaves out intention, solo or with others, mood, timeline, dose and the line's shop;
    - the product editor leaves out brand, strains, THC/CBD, terpenes, batch and package date, shops and the purchase list (prices and shops within it), and a label scan fills none of those;
    - receipts record no prices, shop or currency with prices and shops hidden, and with purchases hidden there is no receipt scan, "Quick log?" opens nothing, and "Show me" offers no "log a purchase";
    - the Products tab drops the Strains or Brands list, and product names drop the brand, when those are hidden;
    - share cards don't offer strain, intention, timeline, THC/CBD, terpenes or batch when hidden, a session's method leaves out the dose, and a product's title drops the brand.
  - Editors keep hidden values in their drafts, so saving never clears them.

### Structural rules
- **Per-record keys for every record type**, not just sessions (spots and products are sensitive too). Deleting a session leaves its products; deleting a spot detaches it from sessions.
- **Merge-friendly shapes:** maps (effects, product lines, stream snippets) so each element merges independently under per-field LWW.
- **IDs:** random 128-bit, never time-ordered.
- **Schema version on every record**; migrations live in the Rust core.

### Model implementation (decided, `core/crates/terpoir-model`)
- **Every record is a document:** ID, kind, schema version, and a map of named fields, each carrying its own stamp (HLC time + device ID) for per-field last-writer-wins.
- **Maps are flattened** to one field per entry (`effects/relaxed`, `lines/<id>`, `attachments/<id>`). Values that only make sense together (a dose's amount and unit, coordinates, a price) are a single tuple replaced as a whole. Removing an entry writes a null with a fresh stamp, so removals merge like edits.
- **Edits stamp only what changed.** Fields a record type doesn't know (written by a newer app) are kept untouched, and unknown vocabulary values round-trip unchanged, so an older device never destroys newer data.
- **Fixed-point numbers:** doses and label amounts in thousandths of their unit; prices in minor currency units; coordinates in 1e-7 degrees with an accuracy radius.
- **Serialization:** `postcard` (compact, stable wire format) inside a versioned envelope; the serialized document is what the record key encrypts. Record-kind codes and value variants are append-only.
- **Migrations** live in the core, run on read, must be deterministic and keep stamps (devices that migrate independently agree; a migration never beats a real edit). Documents from a newer schema are reported, not rewritten.
- **Schema upgrades (decided 2026-10-13, maintainer):** an app upgrade or a restore must never lose data because of a schema mismatch. A backup that won't load is treated as worse than a missing feature, so this work comes before the remaining sync backends.
  - **Newer data is kept, not skipped:** records from a newer schema are stored sealed and unchanged, and applied once the app is new enough. An import never quietly drops them, and the user is told when an update is needed.
    - **Implementation (2026-10-13):** a record this version can't read (a record-kind code it doesn't know, a document envelope newer than it parses, or a newer schema) goes into the store's `pending` table exactly as received: ID, kind code, wrapped key, ciphertext. It is never opened, edited or rewritten, and a newer copy of the same ID replaces it (it can't be merged unread). Sync blobs carry unknown kind codes instead of rejecting the whole blob, and exports and sync blobs include pending records, so a restore or another older device keeps them too. Master-key rotation re-wraps them with everything else. Deleting a record removes its pending copy, and a deletion marker keeps it from coming back. On every unlock the app calls `apply_pending_records`, which merges whatever this version can now read like an incoming sync blob and stops keeping it; the rest stays. A blob whose container format is newer is refused as `NewerVersion`, not as invalid data.
    - **Telling the user (2026-10-13):** restoring or importing an export first checks every blob (`check_export_archive`, which changes nothing). If any blob is in a format only a newer app parses, the whole file is refused with "made by a newer version of Terpoir; update, then try again; nothing was changed", so a restore never finishes partway. Records a newer app wrote don't block anything: they are kept, and while any are, the journal timeline says how many entries are waiting for an update. Folder sync leaves a newer-format blob in the folder for after the update and says so on the sync card.
    - **Known limit:** an off-action that clears fields inside records (such as deleting targeted symptoms) can't reach a record it can't read. A pending record from a newer device keeps those fields until it is applied, and a field the clear stamped later still wins the merge.
  - **Derived data is versioned and rebuilt:** indexes, scan aliases and links, and platform caches are rebuilt (and re-linked) on the first unlock after an upgrade that changes them.
    - **Implementation (2026-10-13):** the store keeps, per kind of derived data, the version of the rules that built it (a `derived` table; schema 4). The index (references, local days, tags) has `INDEX_VERSION`, pinned to its rules by a test, so changing what is indexed without bumping it fails CI. On every unlock the core calls `refresh_index`: when the stored version differs (or is missing, as in every database from before this), it rebuilds the whole index from the records in one transaction and records the version. A new database starts current. A record that doesn't open is left unindexed rather than keeping the journal locked, and a failed rebuild is retried at the next unlock with the old index still answering.
    - **Not derived:** the attached-files table (a file's owner is recorded when the file is written, before the record lists it) and the pending records. Nothing is rebuilt from them.
    - **Android alerts:** the notification cache, the fence ID scheme and the purchase-prompt state carry `SpotAlerts.CACHE_VERSION` (a file in the no-backup directory). At process start, an outdated version drops the cache and prompt state, and fence events are ignored until the next unlock re-registers the fences and rebuilds the cache, which every unlock already does. A downgrade is treated the same way.
    - **Scan aliases and links:** learned receipt names aren't built yet. They will be product data (synced), not derived, matched with `normalize_name`; a change to that normalisation must come with its own derived version that re-normalises stored aliases on the first unlock after the upgrade.
  - **Old backups keep working:** checked-in fixtures from every released version (export, sync blob, database) must still open and migrate. CI enforces this.
    - **Implementation (2026-10-14):** `core/crates/terpoir-ffi/tests/fixtures/<version>/` holds a database (with an attached file), an encrypted export, a sync folder's blobs and a capture inbox, written by that release's own code from fixed, published test credentials (`make_compat_fixtures.rs`, run in a checkout of the tag). `tests/compat_fixtures.rs` finds every version directory and, with the current code, opens and migrates the database, restores the export, merges the sync blobs in both orders, merges the inbox note, checks every field of every record, then round-trips each through a new export and new sync blobs. Each release adds its directory (docs/releasing.md); v0.2.0 changed no format, so v0.1.0's cover it.
  - **Older apps refuse newer databases** with a clear message, never a reset.
    - **Implementation (2026-10-14):** the core refuses a database whose `user_version` is newer (`NewerDatabase`, `CoreError::NewerVersion` across the FFI) before changing it; a test opens one twice and checks its version and tables are untouched. On Android, a failure after the unlock prompt leaves the journal locked and shows "saved by a newer version of Terpoir; update the app" (or a general "couldn't be opened") instead of crashing; nothing offers to reset it. Newer exports and sync blobs were already refused before anything changes, and derived indexes from a newer version are rebuilt, not refused.
- **Stream snippets** are separate records referencing their session; photos and voice clips are attachment IDs on the session.
- **Purchases (record kind 8):** `when`, optional `spot`, and `lines/<id>` → (product, batch, price minor units, currency). A price is all-or-nothing, like a dose. Purchases are in the date index by `when`, and a line's product is a reference, so "every purchase of this product" is an index lookup.
  - **Session product lines** may carry a (purchase ID, purchase line ID) pair; both or neither. Older four-item line values still read. Changing a line's product in the editor drops its purchase reference.
  - **Product identifiers** are a map `identifiers/<code>` → kind (`barcode`, `qr`, `package_tag`), so codes scanned on two devices merge. Codes are free text and stay out of the plaintext index. The package date is a day number (days since 1970-01-01).
  - **Spot purchase prompts** are a per-spot flag, stored only when on (default off).
  - Pre-release, so Product's old `bought_at` and `price` fields get no migration; any already written are kept as unknown fields.
- **Vocabularies come from the core.** The FFI's `vocabulary()` lists every built-in slug in display order: core effects, methods, dose units, forms, label units, strain types, intentions, social and symptom results.
  - `dose_units_for_method` gives each method's sensible units, the usual one first. Topicals have none, and an unknown method gets all of them.
  - `default_method_for_form` gives a product form's usual method.
  - Platforms only translate the slugs; an unknown slug from a newer app is shown as is.
- **Android journal UI (`app.terpoir.journal`):**
  - **Screens:** journal list (newest first), session editor, and product, strain and brand editors. They sit behind a bottom bar: Journal, Products (with strains and brands), Privacy.
  - **Data layer:** every read and write goes through `JournalRepository` to the core, off the main thread.
  - **Effects map:** the core effects plus the user's own effect tags. Each selected effect gets a 0–5 intensity.
  - **Editing never drops data:** fields an editor does not show yet (activity, spot, symptoms, attachments, product terpenes, package date and identifiers, a line's purchase) are carried over unchanged.
  - **Lock clears everything:** locking replaces the whole UI, open editors included.
- `Debug` output of documents, values and records is redacted; decrypted text is wiped when a document is dropped.

## Richer strains (decided 2026-10-14, maintainer)

Strains can be described in as much detail as the user wants. Planned for 0.4.0 (detail setting, terpenes, lineage) and 0.5.0 (cannabinoids, grouped insights), after the Privacy Center pass in 0.3.0 and before strain lookups in 0.6.0; see `docs/roadmap.md`, "Release plan".

- **One detail setting for the journal**, synced like "What I track", with three steps:
  - **Simple:** indica / sativa / hybrid only.
  - **Terpenes:** also which terpenes the strain is known for. The picker guides the user to 3 or fewer ("pick the ones that stand out"); more is allowed, never blocked.
  - **Detailed:** also each terpene's rough proportion (dominant, secondary or trace, never exact percentages, which stay on products where a label states them), parent strains (lineage), and further cannabinoids.

  A lower step hides the extra fields; it never deletes them, and stepping up shows them again. Existing journals start at Terpenes, which matches today's strain editor. With terpenes hidden in "What I track", the strain editor shows no terpenes at any step.
- **Proportions:** a strain's terpenes stay a set (which ones), and each may also have a level. Terpenes logged before 0.4.0 have no level ("unrated") until the user sets one; nothing is guessed.
- **Lineage:** parents are links to strain records. A parent can be one the user has logged, or a typed name, which creates a minimal strain record to fill in later. The old free-text lineage stays as a note on the strain; it isn't parsed into links.
- **Cannabinoids (0.5.0):** besides THC and CBD, products can carry label amounts for CBN, CBG, THCa and CBDV, in the same units. A strain can note which of them it is known for. Exact amounts belong to products only.
- **Insights:**
  - 0.4.0: when a product's label lists no terpenes, the terpene insights fall back to its strain's dominant terpene (a terpene rated dominant, or the only one listed), and the insight says the profile came from the strain, not a label. Implemented in the core's `Journal::insights`: the fallback applies only while terpenes and strains are tracked and the detail step is above Simple; a product's first strain (in ID order) with a dominant terpene is used; and each insight reports how many of its sessions got the terpene only from a strain profile (`from_strain_profiles`), which the app shows under the evidence line.
  - 0.5.0: insights and the shop effect can group strains by shared parent or by dominant terpene, not only by strain record ("Shop insights").
- **Compatibility:** the new fields are additive (levels, parents and cannabinoids are new fields beside the existing ones), so an older app keeps reading the strain's name, type and terpene set. A 0.4.0 compatibility fixture is checked in with the release (`docs/releasing.md`, step 7). The fixture generator (`make_compat_fixtures.rs`) now writes a name-only parent strain, levels, a parent link, a cannabinoid and the Detailed step, and `compat_fixtures.rs` expects them from any fixture whose manifest names a `parent_strain` (and empty fields and the Terpenes step from older ones). Parent links are references in the index (`INDEX_VERSION` 2), so "what refers to this strain" includes its children; journals from 0.2.x rebuild the index on the first unlock after upgrading.
- **Core implementation (`terpoir-model`):** `Strain` gains `terpene_levels` (terpene → `TerpeneLevel`: `dominant`, `secondary`, `trace`), `parents` (strain record IDs) and `cannabinoids` (`Cannabinoid`: `cbn`, `cbg`, `thca`, `cbdv`); each is a map of its own fields, so edits merge per entry and an older app keeps them untouched. A level counts only while its terpene is listed (`Strain::level`); `Strain::dominant_terpene` is the one rated dominant, or the only one listed. The detail step lives in the synced tracking record (`strain_detail`); unset reads as `terpenes`, and a step from a newer app as `detailed`, so nothing is hidden by mistake.
- **Android:** the detail step is chosen in "What I track", under Strain detail (shown while strains are tracked). The strain editor follows it: the type always; at Terpenes, chips for every built-in and custom terpene, with a note past three; at Detailed, a level per chosen terpene, parents (picked from the journal or added by name, which creates a name-only strain) and the cannabinoids. The free-text lineage is now "Lineage notes", shown from Terpenes up. Whatever the current step or hidden data points leave out is saved exactly as it was.
- **Lookups (0.6.0):** a strain catalog can fill these fields as suggestions the user confirms ("Catalog lookups").
- **Shares and export:** the new fields follow the same rules as the old ones: hidden fields are never offered, and the plaintext export includes what the journal holds.

## Product names (decided 2026-10-12)
- A product is shown as its brand followed by its own name ("Fire Bros Block Berry Cart") everywhere it appears: the catalog, the timeline, and the product pickers. The stored name stays the product's own, so it needn't repeat the brand. A name that already starts with its brand is shown as it is, never doubled. Display only: the model keeps name and brand as separate fields.

## Rapid product entry (decided)
- **Receipt scan:** one photo fills in several products with prices, the date, and the shop.
  - On-device text recognition only. **Engine (decided):** the open engine, Tesseract (through Tesseract4Android), is the default everywhere on Android; ML Kit is a Play-only opt-in.
    - **Play, default:** not in the base app. The first scan offers to download Tesseract: the code as an on-demand feature module (Play Feature Delivery), the language data as an asset pack (Play Asset Delivery). The Play Store app downloads them, so this needs no INTERNET permission.
    - **Play, opt-in (part of YOLO Mode):** ML Kit text recognition through Google Play services: tiny in the app, and Google downloads and updates the model. Usually the most accurate on photographed receipts. Its card discloses that it is a closed Google component, and that Google's terms let it fetch updates and send usage metrics (operational data, never the scanned text; see "Cloud models").
    - **FOSS:** F-Droid has no on-demand delivery and the app never downloads code itself, so the foss build bundles Tesseract (the from-source build, "OCR native libraries" under Builds and releases) with a compact English model. Larger models and other languages are optional data, once the app can download (see the INTERNET permission).
    - **iOS:** Apple's Vision framework (built in, nothing to download).
    - The scanner keeps the engine behind one interface, so the choice is one switch.
    - **Implementation (2026-10-11):** Tesseract 5 through Tesseract4Android 4.9.0, with the English `tessdata_fast` model; its only published build is on JitPack, which `settings.gradle.kts` restricts to that one group, and its native logging is a recorded exception (see "Logs and crash reports"). Sources and hashes: `android/ocr/README.md`. The scanner's `TextEngine` interface has three implementations, chosen per build in `Distribution`:
      - **foss:** `BundledTextEngine`, with Tesseract and the model in the APK (the model from the `:ocr-model` module's files).
      - **Play, default:** `PlayTesseractEngine`. Tesseract is the on-demand feature module `:ocr` and the model the on-demand asset pack `ocr_model` (`:ocr-model`); the Play base app carries neither. Before a label or receipt scan, the app asks once to download them from Google Play ("a few megabytes", progress shown, the Store's own confirmation when it asks for one); "Not now" closes the scan, and nothing is fetched unasked. The base app loads the module's entry point by name (SplitCompat makes it usable without a restart). A build check verifies the Play bundle: Tesseract only in `:ocr`, the model only in the pack, no logging after R8 in any module.
      - **Play, opt-in:** `GoogleTextEngine`, behind the Privacy Center toggle "Google text recognition" (new group: Scanning; off by default; not offered in foss). On only switches the engine; the scan asks before Play services downloads the model. Off switches back and releases the model (`ModuleInstallClient.releaseModules`), so Play services can remove it. Joining YOLO Mode waits for the privacy presets.
    - **Network and permissions on Play (decided 2026-10-11, changed 2026-10-12):** ML Kit's Google Data Transport dependency requests INTERNET to send usage metrics. Until the online map, the play manifest removed INTERNET, so the metrics were never sent from the app. Every build now has INTERNET for the online map, so while "Google text recognition" is on ML Kit can send its usage metrics (never scanned text) from the app; its card says so (maintainer decision, 2026-10-12). Play services downloads the model in its own process. Three normal permissions (no prompt) are allowed on Play only: FOREGROUND_SERVICE and FOREGROUND_SERVICE_DATA_SYNC, which Play Asset Delivery needs to copy a downloaded pack into the app's storage, and ACCESS_NETWORK_STATE, which Data Transport's upload jobs need to be scheduled. The foss allowlist is unchanged apart from INTERNET. Play Asset Delivery stays on 2.2.x, because 2.3.0 adds WorkManager and its permissions.
    - **ML Kit's form:** the Play opt-in uses the Play services variant of ML Kit text recognition, which the "Google Play libraries" licensing permission covers. The bundled `com.google.mlkit` artifacts, which that permission does not clearly cover, are not used.
    - **`ocrs` (pure-Rust OCR) was considered** and not used: the license of its model weights could not be confirmed.
  - Extracted lines appear as a checklist to confirm, fix, or drop.
  - Header text suggests the shop name, matched on-device against existing spots.
  - **The receipt image is never saved** (receipts can carry the user's name, loyalty numbers, or medical card numbers). Only confirmed fields are kept.
    - **Changed (2026-10-12):** a small copy (at most 1600 pixels on its longest side) stays in memory while the user checks the receipt, so they can tap the lines to keep. It is recycled when the review screen closes and is never written to storage. The capture screen says so. Previously the photo was recycled as soon as it was read.
- **Label scan:** on-device text recognition of the compliance label pre-fills name, brand, THC/CBD, terpenes, batch, and package date. Optionally keep the photo as the product picture (EXIF-stripped, never written to the gallery).
- **Barcodes and QR codes:** decoded on-device; stored locally as identifiers so re-buys are recognized instantly.
  - **Online lookups of a scanned code are opt-in** via the catalog toggles; a package tag is a precise identifier.
  - **Opening a label's lab-results (COA) link is opt-in** with the same disclosure: the lab's website learns the user's IP address and which batch they bought.
- **Duplicate detection:** "Same product, new batch?" keeps history unified.
- **Implementation (decided):**
  - **Parsing lives in the core** (`terpoir-scan`): receipt lines, shop, date and total; label THC/CBD, terpenes, batch, package date and package tags; code classification (barcode, QR, package tag, link). Platforms only capture and recognize.
  - **Duplicate detection and shop matching run in the core** over decrypted records (`Journal::find_products`, `Journal::match_spot`). Matching is by scanned code first, then by normalized name and brand. Codes are never put in the plaintext index.
  - **Android capture:** a CameraX viewfinder in the app. Photos are taken into memory at full quality (capped at 12 megapixels) and read; a label photo is recycled at once, and a receipt's small review copy when its review screen closes. Nothing is written to storage or the gallery. Code scanning decodes live frames with ZXing, so nothing is photographed. The camera permission is asked on first use.
  - **Reading receipts (2026-10-12):** the first version read receipts poorly, because recognizers report a photo as blocks, and on a receipt the names and prices land in different blocks.
    - **Rows in the core:** engines report lines with their boxes, and `terpoir-scan` groups them into printed rows by position (`group_rows`) before parsing, for labels too. The parser reports which rows each item and the shop came from.
    - **Before reading:** the app makes a gray copy with its contrast stretched (1st to 99th percentile of brightness). Tesseract reads a receipt as one column of rows (`PSM_SINGLE_COLUMN`, with local Sauvola thresholding for shadows); labels keep automatic layout.
    - **Tap to tag:** the review screen shows the photo with a box on each row. Tapping a row adds it as an item (read with `parse_receipt_line`, which filters nothing, since the user chose it) or removes it; holding a row makes it the shop.
  - **Receipts never offer personal or payment lines** (cardholder, loyalty, patient or member numbers, totals, tax) on their own; the user can still tap one in. The checklist is saved as one Purchase. The shop becomes an existing spot, a new Re-up Spot with no location, or nothing, as the user chooses.
  - **Built-in terpene vocabulary** (myrcene, limonene, …) in the core, so label terpenes are tags like any other.
  - **Lab-results links** are saved with the product either way. Opening one is behind the Privacy Center toggle "Open lab-results links" (Catalogs, off by default).
  - **Online code lookups are not planned** (maintainer, 2026-10-12; see "Settled 2026-10-12").
- On-device model tiers can improve parsing (messy receipts) but are not required; pattern parsing works without them.

## Scan recognition from the journal (decided 2026-10-12, maintainer)
Scans get better at recognising what the user buys by using what the journal already holds: products, brands, strains, shops, and how receipts have named them before. All of it runs on the device, in the Rust core (`terpoir-scan` for matching, the product record for learned names); the platforms only show the results. Planned as one of the first versioned feature releases.
- **Learned receipt names.** When the user links a receipt line to a product, the product keeps the line's normalised text as an alias (encrypted, synced with the product). The next time a receipt prints that exact text, the line links to that product **automatically**, shown with a "matched" chip that one tap changes or undoes.
- **OCR-tolerant suggestions.** Without an exact alias, the core ranks the user's products, brands and strains against the scanned text: word by word, tolerant of common OCR confusions (`0/O`, `1/l/I`, `5/S`, `8/B`, `rn/m`), boosted for products whose "Where I buy it" includes the receipt's shop and for lines that contain a known brand. These are **suggestions only** ("Looks like Acme Blue Dream?"); nothing below an exact alias links on its own.
- **Labels.** Known brand and strain names found in label text pre-fill Brand and Strain, and narrow the duplicate-product check.
- **Growing over time.** Matched text is broken down into what it identifies (brand, strain, product type or form, size), so later scans recognise those pieces even in products the user hasn't logged before.
- **Later: a strain database.** Recognition can draw on a strain database when one exists, in the way the "Catalog lookups" section allows: a downloadable offline catalog first (no per-scan queries, so nothing about what the user scans leaves the phone); any online lookup is its own opt-in catalog toggle with a disclosure card.
- **Not done: vocabulary fed to the OCR engine.** Tesseract only takes custom words as a file on disk, which would put journal-derived names outside the encrypted store, and ML Kit has no such hook. Correcting after recognition gets most of the benefit without that.
- **Privacy:** no new toggle; nothing leaves the encrypted store. Aliases are part of the product: included in the plaintext export, never on share cards.

## Stream journaling, tagging, and insights

### Stream
- Free-flow text per session (added during or after) plus standalone entries not tied to a session.
- Stored as timestamped snippets with their own IDs, so concurrent additions from two devices both survive.
- Optional voice input through **on-device speech recognition only**. Disclose that system-keyboard dictation may send audio to Google or Apple; the app offers its own on-device mic button instead.

### Tag extraction (user chooses the tier)
1. **Keyword matching** (default when enabled): curated vocabulary of effects, terpenes, flavors, and moods, matched in the Rust core. No model; auditable.
2. **OS on-device model:** Gemini Nano (supported Android devices) or Apple's Foundation Models framework (supported iPhones). Disclose that text passes to an OS service the app doesn't control, on-device.
3. **Bundled open model:** small quantized model running in-app; works on de-Googled devices. 1–2 GB download via app-store asset delivery or a neutral host with a pinned hash (never terpoir.app).
- **Never a cloud model (clarified 2026-10-11):** no model that runs in or from the cloud on the user's queries or content. A component whose only traffic is operational (downloading or updating a model, usage metrics that carry no journal content, such as ML Kit's) is not a cloud model: it may be offered as an opt-in with its disclosure card, for example in YOLO Mode.
- **Human in the loop:** suggestions link to the phrase they came from; the user accepts, edits, or rejects. Only confirmed tags feed statistics. Each tag records its provenance (user-entered vs. model-suggested).
- **Keyword tier implementation (decided, `core/crates/terpoir-tagging`):**
  - **Vocabulary:** the core effects, the built-in terpenes, and new built-in **flavors** (citrus, berry, sweet, earthy, pine, diesel, skunk, spicy, herbal, floral, woody, minty) and **moods** (happy, calm, content, energetic, stressed, irritable, sad, tired). Each built-in tag has a short list of English phrases ("munchies" → hungry, "lemony" → citrus). Built-in slugs are unique across all vocabularies, because a snippet stores a built-in tag by slug alone; a test enforces it. The user's own effect, terpene, flavor and mood tags are matched by name, and win over a built-in synonym.
  - **Matching:** case-insensitive whole words. A phrase's last word may take a common English ending (relax → relaxed, relaxing). The longest phrase wins, and each tag is suggested once. A mention within three words after a negation ("not", "no", "wasn't", "without", …) is skipped. Spans are in UTF-16 units, so the UI can quote the phrase. English only for now.
  - **Always available, no toggle:** keyword matching runs in the core on text already in the app, and nothing leaves it. The OS-model and bundled-model tiers (Phase 5) will be opt-in toggles.
  - **Android:** suggestions appear under a note's text as it changes. Each shows the tag and the quoted phrase, with Add and No. A rejected suggestion stays hidden while the note is open. Tags can also be added by hand (provenance `user`; accepted suggestions are `keyword`). A session's notes are listed in the session editor; "New note" there opens over the editor, so the session's unsaved edits survive. The journal timeline shows a note's first tags.

### Insights engine
- Plain statistics in the Rust core, not the LLM, so conclusions are checkable.
- Opt-in; nothing computed until enabled.
- Evidence thresholds: minimum sessions on each side and a meaningful difference before anything surfaces.
- Every insight shows its evidence, e.g. "myrcene-dominant products averaged 1.8★ vs. 3.9★ (6 vs. 14 sessions)," with tap-through to those sessions.
- Flags likely confounders (dose, method, time of day).
- Wording: "your sessions suggest," never medical claims.
- User can dismiss, mute a category, or mark an insight wrong.

- **Implementation (2026-10-12):**
  - **Core:** the `terpoir-insights` crate compares the average rating of rated sessions with and without each feature. Features are a product's dominant terpene, the method, the form, the strain type, and the time of day (morning, afternoon, evening, night). An insight needs 5 sessions on each side and a 1-star difference (maintainer's thresholds), lists its sessions, and flags likely confounders. Method or time of day is flagged when a value's share differs by 50 points or more between the groups; dose when the average mg dose differs by 50% or more. Strongest first, at most 12.
  - **Feedback** (dismissed, wrong, category muted) is synced and exported (**changed by the maintainer, 2026-10-13**; it was kept on each device only, so a restore or a second device lost it). It lives in one insight-feedback record (record kind 10), encrypted and synced like any other. Like the tracking-preferences record, its ID is derived from the master key, and each insight or category is its own map entry, so feedback given on two devices merges per entry (last writer wins). Master-key rotation moves it to the new ID. The off-action's "forget" writes removals that sync, so feedback is forgotten on every device. States a newer app adds are kept. Feedback that earlier versions kept in this device's metadata moves into the record on the next unlock (entries already in the record win). Insights themselves are never stored: `Journal::insights` computes them when called.
  - **Android:** the Privacy Center toggle "Insights" (Journal group, off by default, in neither preset). While it is on, an Insights button on the journal opens the screen. Each card reads "Your sessions suggest: …", with its evidence, its confounders, and the sessions behind it, plus Dismiss, "Seems wrong" and "Hide … insights". The off-action forgets all feedback.

### Shop insights (decided)
- Sessions grouped by the Re-up Spot where each product was bought: average rating and **hit rate** (share of sessions rated 4★+ or marked "would repeat") per shop.
- **Selection vs. shop effect:** separates "you tend to buy better-suited products here" from "the same product rated higher when bought at this shop." The shop effect surfaces only when the same product was bought at two or more shops.
- **Freshness:** from label package dates, product age at purchase per shop ("products from Shop B were four months older on average").
- Same evidence thresholds and tap-through as all insights.
- **Implementation (2026-10-12):**
  - **Products, then shops (settled 2026-10-12):** a session is about products, and a product is bought at shops, so shop insights go session → product → shop. The exact purchase date matters little; a product gets **"Where I buy it"** (`Product.shops`), a set of Re-up Spots. Saving a receipt with a spot adds that spot to each product it lists; the user can add or remove shops on the product at any time.
  - **A session line's shop** (`ProductLine.shop`, stored in the session's `line_shops` map so apps that predate it keep it untouched) is picked with a "From" chip row, shown only when the product has two or more shops.
  - **Attribution** (`Journal::shop_insights`): a line counts for a shop when the user picked it (From), else when it names a purchase from a spot, else when the product has exactly one known shop (Where I buy it, plus receipt spots). Otherwise it is left out, never guessed.
  - **A shop shows** once it has 5 rated sessions. It shows the average rating, the hit rate (among sessions rated, or marked would-repeat or not), and freshness: the average days from package date to purchase, once 5 are known. Freshness counts only lines that name their purchase (the only ones with a purchase date).
  - **Shop effect, by brand, product and strain:** products often appear at one shop only, while a brand's are usually consistent across shops, so **brand** is the main level; product and strain follow. For each brand, product or strain bought at two or more shops: its rated sessions bought here vs elsewhere, needing 5 on each side and a 1-star difference. A session with it from both sides counts on neither. One only ever bought at one shop is selection, so it is never reported as a shop effect.
  - **Strain is compared by strain record for now.** When strains gain lineage and terpene proportions (Backlog, "Richer strains"), the shop effect can compare by lineage or terpene profile too.
  - **Android:** "Where I buy it" chips on the product editor, the From chips on a session's product line, and a "Shops" section on the Insights screen, behind the same toggle, with each shop's numbers, its shop effects (brand first), and its sessions. The plaintext export's `products.csv` gains a `shops` column.

### Privacy Center entries
- **Tag suggestions** (choose tier). Off-action: discard unconfirmed suggestions; optionally delete the downloaded model.
- **Insights.** Off-action: delete computed insights and cached statistics; journal and confirmed tags remain.
- **Targeted symptoms.** Off-action: hide or delete.
- **Address lookup for spots.** Off-action: remove stored addresses.
- **Follow-up check-ins.** Off-action: cancel pending check-ins.
- **Purchase prompts.** Off-action: remove purchase-prompt geofences; clear pending state.
- **Open lab-results (COA) links.** Off-action: stop opening them; nothing stored to clean up.
- **Notify me at my spots.** Off-action: remove every geofence; delete the notification cache and the quiet period; clear a showing alert.
- **Spot names in alerts / Details in alerts** (notification tiers). Off-action: narrow the notification cache to what the remaining tier shows, or delete it.
- **Google text recognition** (Play only). Off-action: scans go back to the open engine; release ML Kit's model so Play services can remove it.
- **Online map** (with its provider: OpenFreeMap, MapTiler with the user's key, or Google Maps on Play). Off-action: stop every map request; delete MapLibre's stored files; the provider returns to OpenFreeMap. A saved MapTiler key stays until the user removes it.
- **MapTiler key.** Removing it deletes the key and switches the provider back to OpenFreeMap.

## Photos
- In-app capture writes directly into the encrypted sandbox, never the camera roll.
- Imports use system pickers (Android Photo Picker, PHPicker); no broad library permission.
- Strip EXIF and GPS on import; tell users the original remains in their gallery and may be cloud-backed.
- Nothing written back to the gallery unless the user explicitly exports.

## Catalog lookups
- Each catalog source has its own toggle, all off by default, with a card stating who runs it and what it can see.
- Local user-created entries and downloadable offline catalogs always available.
- Disclosure of online lookup leakage: DNS (resolver/ISP), TLS SNI (unless ECH), IP and timing, and query content to the provider.
- Options for less leakage: offline catalog, encrypted DNS (Android Private DNS, iOS profiles), VPN or Tor (Orbot).
- Engineering: search on explicit submit only (no type-ahead), cache results in the encrypted store, app-level API keys (no per-user identifiers), minimal headers.
- Check each provider's terms on caching and offline redistribution.

## OS indexing and integrations
- Default: index nothing, donate no activities.
- Optional toggle with granularity (names only vs full text):
  - Android: dynamic shortcuts and AppSearch on; off-action removes shortcuts and clears search data.
  - iOS: Core Spotlight and activity eligibility on; off-action deletes the searchable index and Siri interaction donations.
- Disclose that indexed content sits outside app encryption and removal depends on the OS honoring deletion.

## Privacy Center
- One screen holding every privacy-relevant toggle, grouped by surface: Location, Notifications, Catalogs, OS Integration, Keyboard and Voice, Photos, Sync and Keys, Device Security.
- Each toggle has a card: what it does, what leaves the app, who can see it, how to undo it.
- Status summary at top ("All protections on" / "3 protections relaxed").
- **Layout (maintainer, 2026-10-14, option B of the readability pass):** the first screen is the status summary with the current preset, the preset buttons, one row per section, "Restore maximum privacy" and the privacy policy. Each section row names its toggles and is marked "Private" or with how many of its toggles are on. A section opens on its own page, with its toggles in one card in the style of option A: each with its switch and what it does; a toggle that is on always shows the rest of its card, labelled "What leaves the phone", "Who can see it" and "To undo", and a private one shows it on request ("What it shares"). Settings that belong with a section come at the end of its page: the map provider under Location, the "New note" shortcut under OS integration, export and recovery (and folder sync while it is on) under Sync and keys, auto-lock under Device security. Back from a section returns to the list.
- "Restore maximum privacy" resets defaults and runs every cleanup action.
- Shown once during onboarding with everything off. Android: the last onboarding step is the Privacy Center in tour mode (an introduction; "Finish setup" completes onboarding).
- Optional encrypted local history of setting changes.
- **Rule:** every privacy toggle is designed as a paired on-action and off-action (cleanup) from the start.
- **Android implementation (`app.terpoir.privacy`):**
  - **The pairing is enforced by type.** A `PrivacyToggle` cannot be built without both actions. It also needs:
    - its section;
    - its card (what it does, what leaves the app, who can see it, how to undo);
    - its most private state. That is usually off, but on for protections such as screen protection.
  - **Checks at runtime:** every action is checked to have taken effect, and toggle IDs must be unique.
  - **Idempotent actions:** "Restore maximum privacy" runs each toggle's action for its private state, including toggles already there, so a cleanup interrupted earlier still finishes.
  - **Disclosure first:** relaxing a protection shows its card in a confirmation dialog first. Returning to the private state is immediate.
  - **Only while unlocked:** the Privacy Center opens only when the journal is unlocked, and locking closes it. Someone holding the phone cannot relax protections without unlocking the journal.
  - **Device test:** an instrumented test runs every registered toggle both ways on the device.

### Presets and YOLO Mode (decided)
- **Granular first:** every privacy-relevant behavior is its own toggle, with its own card and its own off-action. Presets never replace toggles.
- **Presets are sets of toggle states.** Choosing one shows a single combined confirmation that lists every toggle it changes, each with its card, and then sets them. Afterwards each toggle can still be changed on its own; the status summary shows "Custom" once they no longer match a preset.
  - **Most private:** the default. Same as "Restore maximum privacy".
  - **YOLO Mode** (working name; it amuses us): uses the platform's features for the best experience. Examples: ML Kit text recognition, OS on-device models for tagging, OS indexing and shortcuts, rich notifications, address lookup, online catalogs. Leaving it runs every off-action it caused.
  - A middle preset may come later if users ask for one.
  - **YOLO Mode's toggles (decided by the maintainer, 2026-10-12):** Google text recognition (Play only), open lab-results links, "Show me", spot names in alerts, details in alerts, and follow-up check-ins. Screen protection stays on, and "Notify me" stays its own choice because it needs location "all the time". New toggles join YOLO Mode only by a recorded decision.
    - **Added 2026-10-12 (maintainer):** the online map. On Play builds with a Google key, YOLO Mode sets Google Maps as its provider (maintainer, 2026-10-12); elsewhere it uses OpenFreeMap, or the user's own MapTiler key if one is set. Leaving YOLO Mode turns the map off and returns the provider to OpenFreeMap.
- **Android implementation (2026-10-12):** a preset is a full set of toggle states (`PrivacyPreset`): its listed toggles relaxed, every other toggle at its most private state; IDs a build doesn't register are ignored. Applying one runs every toggle's own action, including toggles already in place, so leaving YOLO Mode runs every off-action it caused. The Privacy Center shows "Preset: Most private", "Preset: YOLO Mode" or "Preset: Custom", and a button per preset opens one combined confirmation listing each toggle it turns on or off, with the card of each one it relaxes.
- **Threat model at onboarding:** one optional question ("What are you protecting this journal from?") suggests a preset. Skipping it, or any answer, leaves the defaults at most private until the user confirms a preset.
  - **Android:** asked at the top of the onboarding Privacy Center tour, never stored. "People who might pick up my phone", "Legal trouble where I live" and "Companies and apps tracking me" suggest Most private (already set). "Not much, I just want it to work well" suggests YOLO Mode, with a Review button that opens its confirmation.
- **Hard lines in every mode, YOLO Mode included:**
  - App data stays out of OS backups (Google and iCloud), and keys stay on the device. The encrypted sync and export model already covers backup.
  - No cloud model on journal content (see "Never a cloud model").
  - No analytics, ads, crash-reporting or attribution SDKs.
  - The app never contacts terpoir.app or any project-run endpoint, and there is no server component.

## Problem reports (decided 2026-10-14, maintainer)

While releases are early, errors offer "Send a report", and the app offers it once after a crash. It may be removed once releases are routine; it is one component (`app.terpoir.report`).

- **By email only, from the user's own email app.** The report opens addressed to terpoirapp@gmail.com and filled in; nothing is sent unless the user sends it, and they can read and edit it first. Terpoir sends nothing itself and gains no network access, so "the app never contacts a project endpoint" still holds. No GitHub route (public, needs an account).
- **What a report holds:** the app version and build (Play or foss), the Android version, which problem it was (the message's resource name), the error's type (class names of it and its causes, never their messages), and for file problems the storage app's name. Never journal content, keys, locations, file names, the device model or any identifier.
- **Where it is offered:** failures the user can't fix themselves: export, import, folder sync, restore, unlock and key errors, locked capture, speech recognition, the text-recognition download, and a spot's location fix. Not where the message already says what to do (the wrong file, no screen lock, an update needed, permission denied) or on input validation.
- **Crashes:** an uncaught error writes a short summary (types and code locations: class, method, file and line, at most 40 frames; never messages) to app-private storage outside backups, then Android handles the crash as before. The next launch offers "Send report" or "Dismiss"; either deletes it. Release builds' names are obfuscated; each release's R8 mapping decodes them.
- **Filed as public issues (maintainer, 2026-10-14):** bugs are tracked in GitHub Issues. A report that shows a bug is filed there by the maintainer (`scripts/file-report.sh`) with the reporter's description and the details block, never their name or address; the report itself and the privacy policy say so.
- **Privacy policy:** says reports exist, what they hold, that a sent report reaches the developer with the sender's address, and that a bug may be posted publicly without it.

## Optional components (decided)
- The base app ships only what every user needs. Heavy, optional pieces (text-recognition engines, language data, the bundled tagging model, offline catalogs) are downloads the user chooses, each named in the Privacy Center or the feature that uses it, with its size, and removable.
- **Code** (native libraries, Kotlin/Java): only through the store. Google Play forbids downloading executable code from anywhere else, so on Play it comes as an on-demand feature module. F-Droid has no equivalent, so code the foss build needs is bundled; only data is optional there.
- **Data** (models, language packs, catalogs): Play Asset Delivery on Play. Elsewhere, once the app has the INTERNET permission, from a neutral host with a SHA-256 pinned in the app, never terpoir.app.
- Removing a component deletes its files; turning off the feature that uses it offers to remove it.

## Website and domain (terpoir.app)
- The app does not contact terpoir.app in normal use. The domain hosts the website, docs, release notes, and published build hashes / reproducible-build verification.
- Universal links / associated domains cause Apple and Google to fetch a verification file from the domain; note this in disclosure docs.
- DNS hardening in place (Cloudflare): DNSSEC, CAA restricting issuance (Let's Encrypt plus Cloudflare's automatic Universal SSL CAs), null MX, SPF `-all`, DMARC reject, empty DKIM.
- Website: no analytics or third-party scripts; disclose that Cloudflare terminates TLS and can see visitors.
- **Website implementation (decided, `site/`):** one static page, hosted free on Cloudflare Pages from `site/` on `main`. System fonts only and no external requests; `_headers` sets a CSP that allows nothing beyond inline styles. Cloudflare Web Analytics stays off. Voice follows the in-app tone: candid about AI authorship, trademark and money. No AI-made logo, icon or illustration: the page is type-led, with `ARTIST:` slots for commissioned human artwork.
- **Infrastructure as code (decided 2026-10-12, maintainer):** the zone's DNS, DNSSEC, the website's Cloudflare Pages project and this repository's settings (merge methods, Dependabot, the `protect-main` ruleset) are managed with Terraform in the private repository `nolsen311/Terpoir-infra`. Changes go through a PR there, with a plan reviewed before a manual apply. Dashboard edits show up as drift. Its state sits in the maintainer's own Azure storage. None of it is a server for the app.

## Licensing, trademark, and anti-clone protection
- **License: GPL-3.0-or-later with an additional permission (GPLv3 §7) allowing app store distribution**, provided source stays available under the GPL elsewhere. See `LICENSING.md`.
- **Linking permission for Google Play libraries (decided):** a second §7 permission lets the program be conveyed linked with Google's proprietary Play libraries:
  - what it covers: Play and Play services client libraries and their Google-distributed dependencies, for example the Maven groups `com.google.android.play`, `com.google.android.gms` and `com.android.billingclient`;
  - what it excludes: no other non-free library;
  - why: needed for the `play` flavor's Google-only features (Play Age Signals first, Play Billing later);
  - the `foss` flavor never contains these libraries (`verifyFossDependencies`).

  The existing SPDX identifier covers both permissions rather than every file header changing. Any future contributor's work is covered because contributions are accepted under LICENSING.md's terms.
- **Trademark policy** (`TRADEMARKS.md`): the code is free, the name and logo are not. Modified builds must use a different name and icon.
- **Contributions:** Developer Certificate of Origin sign-off (`git commit -s`), no CLA.
- **DCO enforcement (blocking):** the `dco` job in `.github/workflows/dco.yml` runs on every pull request and is a required status check on `main`.
  - Human-authored commits must carry a `Signed-off-by:` matching the commit author's email.
  - GitHub bots (Dependabot) are exempt.
  - Claude-authored commits are exempt only in pull requests opened by the repository owner; Claude never signs off on anyone's behalf, and the maintainer adds the sign-off to the squash-merge message.
  - Known limit: the squash commit is created at merge time, after checks run, so the maintainer's own sign-off cannot be blocked pre-merge. The `dco-audit` job flags any commit on `main` without a sign-off. (It must not be a required check: it only runs on pushes to `main`.)
- **Verifiable original:** official builds are identified by signing-certificate fingerprints and reproducible-build hashes published on terpoir.app.
- **Ownership (decided):** held personally by the maintainer; no LLC for now. Accepted tradeoff: the maintainer's name appears on public trademark, copyright, and app store records. An LLC can be formed later and the trademark and copyright assigned to it.
- **Trademark plan:** USPTO clearance search (Terpoir, terp* and terroir variants, sound-alikes; weigh the existing "Terpenes with Terroir" mark), then an intent-to-use (Section 1(b)) application for the standard-character mark TERPOIR in Class 9 (downloadable software), using an ID Manual description that is accurate and neutral. Use a PO box / virtual mailbox and dedicated email for the public record. File the Statement of Use with a store-listing specimen after launch. Attorney consult recommended before filing.
- **Copyright (decided): not planned.** The code is expected to be almost entirely AI-generated, and the US Copyright Office protects only human authorship. Reassess at v1.0 only if meaningful human-authored material exists (logo, in-app copy, substantially reworked code). Raise at the attorney consult.
- **Recorded risk:** the GPL is enforced through copyright, so it has limited force over uncopyrightable AI-generated code. It still covers human contributions and states intent. **Anti-clone and revenue protection rest on the trademark and on being the verifiable, signed original.**
- **Trademark is the priority (decided):** clearance search now; attorney consult (goods description, AI-authorship questions); intent-to-use Class 9 filing soon, since the priority date is the filing date and the name is already public in the repo; Statement of Use after the Play launch; use ™ until registered.
- **Repository protection:** a GitHub ruleset on `main` (no deletion, no force push, pull request required with 0 approvals, optional linear history and signed commits). Claude Code works on branches and opens PRs; the maintainer reviews, signs off, and merges. Add required status checks and a DCO check once CI exists.

## Threat model

Assets: journal entries, photos, saved spots, ratings, keys, and metadata (frequency, timing, the app's presence).

| Adversary | Reach | Defense | Residual |
|---|---|---|---|
| Platform vendors | Backups, notifications, geofence daemon, keyboards, summarization | Backup exclusion, local notifications, tiers, hardware-backed keys | OS is trusted computing base |
| User-chosen storage provider | Ciphertext, sizes, timing, version history | Client-side encryption, padding, batching, opaque names, per-entry crypto-erasure, master-key rotation | Metadata only |
| Network observers | DNS, SNI, traffic patterns | TLS, encrypted DNS / VPN / Tor options | Traffic analysis can reveal app use |
| Catalog providers | Query content, IP, timing | Toggles, disclosure, offline catalogs | Per user choice |
| Map providers | IP, areas viewed (OpenFreeMap, MapTiler, Google) | Map off by default, one provider at a time, gateway allowlist, no tile cache, disclosure | Per user choice; Google outside the gateway |
| Casual physical access | Unlocked phone, glances | App lock, discreet icon/name, notification tiers, switcher blur | App presence visible unless disguised |
| Compelled access | Device under legal compulsion | Passcode-only option, app passphrase, quick-lock, deliberate panic wipe (no duress code), user-initiated deletion with crypto-erasure | Forensic extraction of unlocked device |
| Loss or theft | Device at rest | Data protection class complete, StrongBox / Secure Enclave keys | No remote wipe (no server) |
| Other apps on device | Listeners, clipboard, accessibility, screenshots | FLAG_SECURE, clipboard avoidance, private notifications | Accessibility tradeoff (open) |
| Phishing / impersonation | Fake site, email, or clone app asking for recovery key | Domain anti-spoofing (DMARC reject, null MX), trademark policy, published signing fingerprints, "we never ask for your key" messaging | User education |
| Supply chain / update channel | Malicious or coerced update could exfiltrate keys | Minimal deps, open source, reproducible builds, protected signing keys, possible binary transparency | **Root of trust** |

Explicitly out of scope: jailbroken or malware-infected devices; direct coercion of the user to unlock.

## Decided
- Shared core: Rust + UniFFI, Android first.
- Data keys: per-entry, crypto-erasure on delete, periodic master-key rotation, provider-versioning warnings.
- No auto-expiry of entries.
- Sync merge: per-field LWW with HLC + device ID; union for new entries; ID-only deletion markers.
- License: GPL-3.0-or-later with app store additional permission; trademark policy; DCO sign-off.
- Data model: session-centric entries; per-record keys; map-shaped fields for merging.
- Spots: purpose types only, no relational labels, no automatic reverse geocoding.
- Map picker: opt-in online map; keyless OpenFreeMap by default, MapTiler with the user's own key, Google Maps as an active Play-only choice (onboarding and YOLO Mode); no shared key in the default path.
- Stream journaling with on-device tag suggestions (keyword / OS model / bundled model), human-confirmed; never a cloud model.
- Insights: opt-in, statistical, evidence-backed.
- Follow-up check-ins and targeted symptoms (👍/👎), both opt-in.
- Purchases as their own records; rapid entry by on-device receipt, label, and barcode scanning; shop insights; opt-in discreet purchase prompts.
- Effects vocabulary: fixed core list plus custom tags.
- What I track: the user can hide data points they don't care about (brand, price, strain and more) in Settings; choices sync; hide or delete when turned off.
- Ownership: personal (no LLC for now); trademark and copyright registration plans recorded above.
- Keyboards: user's keyboard by default with zero-cost Android protections; opt-in iOS protections; voice-first FAB with on-device recognition only.
- Screen capture: protection on by default (FLAG_SECURE / capture blur); Android accessibility-sensitive marking always on; screen readers unaffected.
- Storage: per-record encryption inside SQLCipher; minimal indexes; no on-disk free-text index; secure delete; separate notification cache for Named/Rich tiers.
- Logs: none in release; redacted types in debug; no crash SDKs or analytics; opt-in reviewed diagnostics.
- Sharing: field-picked share cards with preview, never via the gallery, discreet brand mark on by default; encrypted export by default, opt-in plaintext export.
- Memory and unlock: zeroizing key types; clear view state on background; lock with the device by default, stricter timers opt-in; crypto-bound biometric-or-passcode unlock.
- Quick capture: "New note" app shortcut on by default; Quick Settings tile, widget, lock-screen controls and voice assistants opt-in; all open the capture sheet in the user's default mode and show no content; write-only capture lets them work while the journal is locked.
- Age gating: top-tier store rating; on-device platform age signals; self-attestation without date of birth.
- Device security: passcode-only and app-passphrase options; quick-lock; two-level panic wipe; no duress passcode; neutral discreet icon/name.
- Privacy presets: granular toggles first; "Most private" default; opt-in YOLO Mode for platform features; hard lines (no OS backups, no cloud model on content, no analytics or crash SDKs, no terpoir.app contact, no server) in every mode.
- Optional components: lean base app; code only through the store, data from the store or a pinned neutral host; Tesseract default, ML Kit a Play-only opt-in.
- INTERNET permission: added with the first feature that needs it, behind a single toggle-checking gateway.
- Builds: Play, F-Droid (FOSS flavor), GitHub Releases, App Store; maintainer key on two YubiKeys; reproducible, pinned, signed pipeline.
- Monetization: free core; one-time Connoisseur unlock; tip jar; donations; never paywall privacy or security.
- Store policy: Android ships as an individual; iOS attempted as an individual, Android-only acceptable if Apple requires an entity; no sale facilitation; all-ages store metadata.
- IP: trademark is the priority (file intent-to-use soon); copyright registration not planned given AI authorship; monetization paths unchanged.

## Open decisions
- None at the moment.

## Settled 2026-10-12 (maintainer)
- **Online code lookups are not planned.** The maintainer doubts barcodes are worth as much as reading the label's text, so no catalog provider will be chosen for now. Scanned codes still work locally (re-buys are recognized); label text parsing is where scanning effort goes.
- **First sync backend: a folder through the system file picker.** Encrypted blobs go into any folder the Android file picker offers (a Syncthing folder, or Google Drive, Nextcloud or Dropbox through their apps' document providers). Terpoir needs no INTERNET permission and holds no credentials for it; sync runs while the app is open. WebDAV, S3 and direct device pairing come later.
- **Personal gear (backlog):** an optional detail of a consumption method, never a replacement for it. A product line keeps its method ("smoked") and can name the piece used ("the green bong"); each piece belongs to one method, and only that method's pieces are offered. Storage and sharing are designed when it is scheduled.
- **Insight thresholds:** at least 5 sessions on each side of a comparison and a difference of at least 1.0 star in average rating before an insight surfaces.
- **Map providers:** keyless OpenFreeMap for the basic map, with no shared key; MapTiler only with the user's own key; Google Maps as an active choice on Play, offered early in onboarding and part of YOLO Mode. The project's MapTiler key and its paid-plan follow-up are dropped. Details under "Map picker".
- **What I track:** users can hide data points they have no interest in, in Settings rather than the Privacy Center, with the choices synced across devices. Details under "What I track".

# Terpoir — Design Decisions

Working name: **Terpoir** (terpene + terroir). Domain: **terpoir.app**.
Repository: https://github.com/nolsen311/Terpoir
Status: pre-development, high-level design. Last updated 2026-10-10.

## Purpose
A mobile app for cannabis consumers to journal experiences with strains, products, brands, delivery methods, and anything cannabis-related, and to understand how those affect them personally.

## Guiding principles
- **Privacy first.** Protect user data from platform vendors, storage providers, network observers, and anyone without the user's consent.
- **Consent first.** Every practice that affects the user is an explicit, informed, revocable choice. Defaults are the most private option.
- **Honest disclosure.** When a feature leaks information, say exactly what, to whom, and how to avoid it.
- **Voice and tone.** Playful and warm inside the app: cannabis can be fun, and people are sharing something personal, so laughing is allowed. Neutral wording on discreet surfaces (Discreet-tier notifications, discreet-icon mode). Humor never pressures a privacy choice: any playful nudge is one tap to dismiss with no extra step and no repeat nagging.
- **Recorded exceptions to "most private default":** keyboard choice. The keyboard is the user's own configured tool; the app defaults to it unchanged, keeps only zero-cost protections, and discloses the risks (see Keyboards and voice input).

## Platforms and architecture
- Native apps on both Android and iOS. **Android first, iOS second.**
- **Shared core in Rust**, exposed to both platforms via UniFFI (Kotlin and Swift bindings). The core owns encryption, the data model, sync, and merge logic. Platform code handles only UI and OS integration. One implementation to audit.
  - Rationale: mature, audited crypto libraries; `zeroize` for wiping key material from memory (not reliably possible on the JVM); production-grade CRDT libraries available if ever needed.
  - Android UI: Kotlin + Jetpack Compose. Build via cargo-ndk; iOS via xcframework.
- **No operator-run backend.** The developer never hosts user data and is never a subpoena or compromise target.
- Local-first: the encrypted on-device store is the source of truth.

## Sync and storage
- Bring-your-own storage only: iCloud Drive, Google Drive, WebDAV/Nextcloud, S3-compatible buckets, Syncthing folders, or encrypted export/import files.
- Everything uploaded is a client-side-encrypted blob; providers see only ciphertext.
- Metadata defense on the client: padding, batched writes, opaque file names.
- Device pairing is direct (QR code or local transfer); no third party involved.
- Account sign-in (Google, Apple), if offered at all, is transport only. Keys are never derived from or escrowed with the account provider.

### Merge strategy (decided)
- **Per-field last-writer-wins.** Each field carries a hybrid logical clock (HLC) timestamp plus device ID; on conflict the later write wins, ties broken by device ID. Every device reaches the same result independently.
- **New entries merge as a set union.**
- **Deletions sync as a minimal marker** carrying only the entry ID (no content). Markers are cleared once all paired devices have seen them.
- Concurrent edits to the *same field* lose one side; accepted as rare for a personal journal.
- CRDT text (e.g. for the notes field) deferred; revisit only if same-field conflicts become a real problem. CRDT operation history conflicts with crypto-erasure, so it would need compaction.

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

### Crypto details (decided, `core/crates/terpoir-crypto`)
- **Key hierarchy in code:** a recovery key or app passphrase yields a key-encryption key, which wraps the master key; the platform keystore also wraps the master key. The master key wraps only other keys: one random key per record and the SQLCipher database key. Subkeys (sync container and similar) come from HKDF-SHA-256 over the master key with fixed purpose labels.
- **Domain separation:** every XChaCha20-Poly1305 operation binds a purpose label (record, wrapped master/record/database key, file stream) and its context in the associated data. A wrapped record key is bound to its record ID and will not unwrap for another record.
- **Formats are versioned:** sealed data is `version || 192-bit random nonce || ciphertext || tag`; wrapped keys are 73 bytes.
- **Files** (photos, clips) use the STREAM construction (`aead-stream`, 64 KiB chunks) with a per-file key derived from the owning record key and the file ID, so destroying the record key erases its files.
- **Recovery key:** 256 bits, written as 56 characters of Crockford Base32 in 14 groups of 4 (version byte, key, 16-bit SHA-256 checksum). Input ignores case, spaces and hyphens and reads I/L as 1 and O as 0. Chosen over a word list: no word list to ship or translate, and it fits a QR code and a password manager equally well.
- **App passphrase:** Argon2id, 64 MiB, 3 iterations, 1 lane, 128-bit random salt; parameters are stored with the wrapped key and capped (1 GiB, 16 iterations) so tampered parameters cannot hang unlock.
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

## Notifications
- Local notifications only; nothing via APNs or FCM.
- Privacy tiers, default Discreet:
  - **Discreet:** "You're near a saved spot."
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

## Keyboards and voice input (decided)

### Keyboards
- **Default: the user's normal keyboard experience.** The user knows their own input preferences; the app trusts that choice.
- **Zero-cost protections stay on by default** because they don't change typing: on Android, the no-personalized-learning flag on all fields and opting fields out of autofill. iOS has no equivalent free protection.
- **Opt-in protections (Privacy Center), iOS:** block third-party keyboards app-wide; disable autocorrect and predictions on journal fields. Each with a disclosure card.
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

### Privacy Center entries
- **Keyboard** (iOS options above). Off-action: future typing only; card links to keyboard dictionary reset.
- **New note default:** voice or text.
- **Voice engine:** OS on-device or bundled. Off-action deletes the bundled model.
- **Keep voice clips.** Off-action: delete stored clips (with confirmation) or keep existing ones.

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

## Local storage format (decided)
- **Two layers:**
  1. **Per-record encryption inside:** each record is serialized and encrypted with its own key and stored as a blob. The same ciphertext is what syncs, so sync never re-encrypts and destroying a record key erases it everywhere.
  2. **SQLCipher outside:** the whole SQLite database is encrypted with a database key wrapped by the master key and held in Keystore / Keychain, hiding structure, row counts, and index columns.
- **Minimal on-disk indexes** (inside SQLCipher only): record IDs, record type, references between records, dates, confirmed tag IDs. **Free text is never indexed on disk**; search and insights build working indexes in memory after unlock.
- **Deletion hygiene:** `PRAGMA secure_delete=ON`; write-ahead log checkpointed and truncated regularly; periodic VACUUM.
- **Photos and voice clips:** separate encrypted files using streaming encryption under the owning session's key; erased when that key is destroyed.
- **Primitives (vetted Rust crates only):** XChaCha20-Poly1305 for records, files, and key wrapping; Argon2id for passphrase-derived keys; HKDF for subkeys.
- **Lock-state tradeoff:** strongest OS file protection makes data unreadable while locked. Discreet notifications and check-ins need no journal data. **Named and Rich notifications use a small separate notification cache** (spot names, summary lines) readable after first unlock. The Rich/Named disclosure cards mention it; their off-action deletes it.

## Logs and crash reports (decided)
- **Release builds log nothing.** Logging is compiled out, not silenced:
  - Rust: release max level off; custom panic handler reports only the code location, never panic message contents.
  - Android: R8 strips logging calls.
  - iOS: logging compiled out of release builds.
  - CI lints fail the build on stray debug prints (e.g. `dbg!`, `println!`). Rust (`.github/workflows/rust.yml`): `cargo clippy -D warnings` on all targets, plus `dbg_macro` and `todo` denied everywhere; `print_stdout`, `print_stderr` and `unwrap_used` denied in library and binary code (tests may print and unwrap).
- **Debug builds follow the no-content rule:** sensitive values use wrapper types that print as `<redacted>`.
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

### Full export
- **Encrypted export (default):** whole journal as an archive encrypted under the recovery key; same format as export/import sync.
- **Plaintext export (opt-in):** JSON + CSV + photos for portability, with a strong warning; saved only through the system file picker, noting that cloud-synced folders will upload it.

### Import
- From Terpoir encrypted exports. CSV import from other apps later.

### Future enhancement (approved, post-launch)
- Terpoir-to-Terpoir sharing: a product or strain record travels as a small file or QR code that a friend imports into their own journal. No server.

## Memory hygiene and app unlock (decided)
- **Key material (Rust core):** held in self-zeroing types (`zeroize`, `secrecy`); avoid copies; lock key pages against swap/compression where the platform allows (best effort).
- **Decrypted content:** decrypt on demand, hold only what is on screen, drop promptly. Known limit: Kotlin and Swift strings cannot be reliably wiped, so the UI briefly holds plaintext; view state is cleared whenever the app goes to the background.
- **Auto-lock:** journal content cleared from view immediately on background; after the timeout, the master key is wiped from memory and reopening requires authentication.
  - Timeout options: immediately, 1, 5, or 15 minutes. **Default: 1 minute.**
  - Locking the device locks the app immediately, regardless of timeout.
- **Biometric unlock (crypto-bound):** the master-key wrapping key lives in Android Keystore / iOS Keychain and is usable only after OS-verified authentication, so a patched app cannot skip the prompt.
  - Default: **biometric or device passcode** (strong biometrics on Android; Face ID / Touch ID on iOS), which survives new biometric enrollments.
  - Optional strict mode: key bound to the currently enrolled biometric set; enrolling a new face or fingerprint invalidates it (re-unlock with passcode or recovery key).

## Age gating (decided)
- **Store rating at the top tier:** 18+ on the App Store, Mature on Google Play. Parental controls block installs on supervised devices.
- **Platform age signals, checked on-device:** Apple Declared Age Range API and Google Play Age Signals API (verify current availability). If the signal indicates a minor, show only a "Terpoir is for adults" screen. Read the category only; never store or transmit it. No server involved.
- **Self-attestation on first launch:** "I'm of legal age to use cannabis where I live." One local yes/no; no date of birth. Sole gate where age signals are unavailable (older OS versions, de-Googled devices).
- **Disclosure:** the age category comes from Apple or Google; Terpoir only reads it.
- Context: state app store accountability laws (Texas, enjoined; Utah, Louisiana, California upcoming) require runtime use of these APIs; re-check status before launch.

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
- **F-Droid** (de-Googled users; builds from source and can publish the maintainer-signed APK when builds match byte for byte). Requires a **FOSS build flavor** without Play services: AOSP location fallback, no Gemini Nano tier, no Play age signals, no Play Billing.
- **GitHub Releases:** maintainer-signed APKs (works with update tools such as Obtainium).
- **App Store** (iOS).

### Signing keys
- **Play App Signing:** Google holds the key that signs Play builds. Disclosed; F-Droid and GitHub builds are signed with the maintainer's own key, and both certificate fingerprints are published on terpoir.app and in the README.
- **Maintainer Android signing key on YubiKey:** generated once on an offline machine, imported into **two YubiKeys** (primary and backup, stored separately) via PIV, one encrypted offline backup kept, machine copy wiped. Signing through `apksigner` with Yubico's PKCS#11 library; PIN and touch required. APK signature scheme v3 key rotation available if ever needed.
- **Play upload key:** may also live on a YubiKey (lower stakes; Google can reset it).
- **Apple distribution certificate:** dedicated Mac keychain (Apple re-signs App Store builds; certificates are quickly revocable).
- **Git tags and release manifests:** signed with an SSH `ed25519-sk` key on the YubiKey.
- Signing material never in CI or the repo.
- **iOS caveat (disclosed):** App Store binaries cannot be matched byte for byte to the source because Apple encrypts and re-signs them.

### Build pipeline (GitHub Actions)
- Reproducible Rust and Android builds, with a CI job that rebuilds and compares hashes.
- Lockfiles committed; Gradle dependency verification; `cargo-deny` (licenses, advisories); `cargo-vet` (audits).
  - `cargo-deny` policy (`core/deny.toml`): fail on known vulnerabilities and on yanked, unmaintained, or unsound crates; licenses limited to a GPL-3.0-compatible allow list (Apache-2.0, BSD, CC0, ISC, MIT, MPL-2.0, Unicode-3.0, Zlib), extended only with a stated reason; crates.io as the only source; crash-reporting and telemetry crates banned.
  - `cargo-vet`: every Rust dependency audited, imported from a trusted auditor, or explicitly exempted (`core/supply-chain/`).
- Actions pinned by commit hash; release builds only from protected `main` and signed tags.
- SBOM published with each release.

### Transparency and updates
- Release hashes published on terpoir.app and signed; later, logged to Sigstore's public transparency log.
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
- **2.3.6:** honest age-rating answers (see Age gating).

### Google Play
- Prohibits apps that facilitate the sale of marijuana regardless of legality (in-app ordering, arranging delivery or pickup, selling THC products).
- **Rule for Terpoir: no ordering, no delivery, no dispensary menus or purchase links**, including via catalog providers. Re-up Spots and price fields are the user's own records.
- No entity requirement; Android ships under a personal developer account. Check current closed-testing requirements for new personal accounts at signup.

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
- **Product:** name, form (flower, pre-roll, vape cart, concentrate, edible, tincture, beverage, topical), brand, strain(s), label data (THC/CBD % or mg per unit, top terpenes), batch/lot, optionally where bought and price.
- **Strain / cultivar:** name, lineage, dominant terpenes, indica/sativa/hybrid label. Insights are driven by terpene and cannabinoid profiles, not the label, which is scientifically weak.
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

### Structural rules
- **Per-record keys for every record type**, not just sessions (spots and products are sensitive too). Deleting a session leaves its products; deleting a spot detaches it from sessions.
- **Merge-friendly shapes:** maps (effects, product lines, stream snippets) so each element merges independently under per-field LWW.
- **IDs:** random 128-bit, never time-ordered.
- **Schema version on every record**; migrations live in the Rust core.

## Stream journaling, tagging, and insights

### Stream
- Free-flow text per session (added during or after) plus standalone entries not tied to a session.
- Stored as timestamped snippets with their own IDs, so concurrent additions from two devices both survive.
- Optional voice input through **on-device speech recognition only**. Disclose that system-keyboard dictation may send audio to Google or Apple; the app offers its own on-device mic button instead.

### Tag extraction (user chooses the tier)
1. **Keyword matching** (default when enabled): curated vocabulary of effects, terpenes, flavors, and moods, matched in the Rust core. No model; auditable.
2. **OS on-device model:** Gemini Nano (supported Android devices) or Apple's Foundation Models framework (supported iPhones). Disclose that text passes to an OS service the app doesn't control, on-device.
3. **Bundled open model:** small quantized model running in-app; works on de-Googled devices. 1–2 GB download via app-store asset delivery or a neutral host with a pinned hash (never terpoir.app).
- **Never a cloud model.**
- **Human in the loop:** suggestions link to the phrase they came from; the user accepts, edits, or rejects. Only confirmed tags feed statistics. Each tag records its provenance (user-entered vs. model-suggested).

### Insights engine
- Plain statistics in the Rust core, not the LLM, so conclusions are checkable.
- Opt-in; nothing computed until enabled.
- Evidence thresholds: minimum sessions on each side and a meaningful difference before anything surfaces.
- Every insight shows its evidence, e.g. "myrcene-dominant products averaged 1.8★ vs. 3.9★ (6 vs. 14 sessions)," with tap-through to those sessions.
- Flags likely confounders (dose, method, time of day).
- Wording: "your sessions suggest," never medical claims.
- User can dismiss, mute a category, or mark an insight wrong.

### Privacy Center entries
- **Tag suggestions** (choose tier). Off-action: discard unconfirmed suggestions; optionally delete the downloaded model.
- **Insights.** Off-action: delete computed insights and cached statistics; journal and confirmed tags remain.
- **Targeted symptoms.** Off-action: hide or delete.
- **Address lookup for spots.** Off-action: remove stored addresses.
- **Follow-up check-ins.** Off-action: cancel pending check-ins.

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
- One screen holding every privacy-relevant toggle, grouped by surface: Location, Notifications, Catalogs, OS Integration, Photos, Sync and Keys, Device Security.
- Each toggle has a card: what it does, what leaves the app, who can see it, how to undo it.
- Status summary at top ("All protections on" / "3 protections relaxed").
- "Restore maximum privacy" resets defaults and runs every cleanup action.
- Shown once during onboarding with everything off.
- Optional encrypted local history of setting changes.
- **Rule:** every privacy toggle is designed as a paired on-action and off-action (cleanup) from the start.

## Website and domain (terpoir.app)
- The app does not contact terpoir.app in normal use. The domain hosts the website, docs, release notes, and published build hashes / reproducible-build verification.
- Universal links / associated domains cause Apple and Google to fetch a verification file from the domain; note this in disclosure docs.
- DNS hardening in place (Cloudflare): DNSSEC, CAA restricting issuance (Let's Encrypt plus Cloudflare's automatic Universal SSL CAs), null MX, SPF `-all`, DMARC reject, empty DKIM.
- Website: no analytics or third-party scripts; disclose that Cloudflare terminates TLS and can see visitors.

## Licensing, trademark, and anti-clone protection
- **License: GPL-3.0-or-later with an additional permission (GPLv3 §7) allowing app store distribution**, provided source stays available under the GPL elsewhere. See `LICENSING.md`.
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
- Stream journaling with on-device tag suggestions (keyword / OS model / bundled model), human-confirmed; never a cloud model.
- Insights: opt-in, statistical, evidence-backed.
- Follow-up check-ins and targeted symptoms (👍/👎), both opt-in.
- Effects vocabulary: fixed core list plus custom tags.
- Ownership: personal (no LLC for now); trademark and copyright registration plans recorded above.
- Keyboards: user's keyboard by default with zero-cost Android protections; opt-in iOS protections; voice-first FAB with on-device recognition only.
- Screen capture: protection on by default (FLAG_SECURE / capture blur); Android accessibility-sensitive marking always on; screen readers unaffected.
- Storage: per-record encryption inside SQLCipher; minimal indexes; no on-disk free-text index; secure delete; separate notification cache for Named/Rich tiers.
- Logs: none in release; redacted types in debug; no crash SDKs or analytics; opt-in reviewed diagnostics.
- Sharing: field-picked share cards with preview, never via the gallery, discreet brand mark on by default; encrypted export by default, opt-in plaintext export.
- Memory and unlock: zeroizing key types; clear view state on background; 1-minute default auto-lock; crypto-bound biometric-or-passcode unlock.
- Age gating: top-tier store rating; on-device platform age signals; self-attestation without date of birth.
- Device security: passcode-only and app-passphrase options; quick-lock; two-level panic wipe; no duress passcode; neutral discreet icon/name.
- Builds: Play, F-Droid (FOSS flavor), GitHub Releases, App Store; maintainer key on two YubiKeys; reproducible, pinned, signed pipeline.
- Monetization: free core; one-time Connoisseur unlock; tip jar; donations; never paywall privacy or security.
- Store policy: Android ships as an individual; iOS attempted as an individual, Android-only acceptable if Apple requires an entity; no sale facilitation; all-ages store metadata.
- IP: trademark is the priority (file intent-to-use soon); copyright registration not planned given AI authorship; monetization paths unchanged.

## Open decisions
None at the design level. New questions go here as they arise.

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

## Location and saved spots
- Location is opt-in, per entry, coarse by default; precise only on request.
- Raw coordinates never leave the device except inside encrypted blobs.
- "Near a saved spot" detection runs on-device:
  - Android: GeofencingClient (Play services), with an AOSP LocationManager fallback for de-Googled devices.
  - iOS: Core Location region monitoring (~20 regions; rotate nearest set).
- Disclosed tradeoff: to geofence, the OS location daemon holds spot coordinates in plaintext on-device.
- Two modes: **Notify me** (geofences, "Always" permission, high consent bar) and **Show me** ("While Using" only; check nearby spots on app open; nothing registered with the OS).
- Minimum radius and cooldown so alerts don't become a visit log; dismissed alerts are not persisted.

## Notifications
- Local notifications only; nothing via APNs or FCM.
- Privacy tiers, default Discreet:
  - **Discreet:** "You're near a saved spot."
  - **Named:** includes spot name.
  - **Rich:** ratings, last product, notes.
- Rich opt-in discloses where content lands: lock screen, notification history, notification-listener apps, wearables, OS summarization.
- Middle option: rich content only when unlocked (Android private visibility with redacted public version; iOS hidden-previews placeholder).
- Same tiering to apply to widgets and Live Activities later.

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
- **Verifiable original:** official builds are identified by signing-certificate fingerprints and reproducible-build hashes published on terpoir.app.
- Planned: register "Terpoir" as a trademark (software class) and optionally register the copyright, ideally through a business entity for identity separation.

## Threat model

Assets: journal entries, photos, saved spots, ratings, keys, and metadata (frequency, timing, the app's presence).

| Adversary | Reach | Defense | Residual |
|---|---|---|---|
| Platform vendors | Backups, notifications, geofence daemon, keyboards, summarization | Backup exclusion, local notifications, tiers, hardware-backed keys | OS is trusted computing base |
| User-chosen storage provider | Ciphertext, sizes, timing, version history | Client-side encryption, padding, batching, opaque names, per-entry crypto-erasure, master-key rotation | Metadata only |
| Network observers | DNS, SNI, traffic patterns | TLS, encrypted DNS / VPN / Tor options | Traffic analysis can reveal app use |
| Catalog providers | Query content, IP, timing | Toggles, disclosure, offline catalogs | Per user choice |
| Casual physical access | Unlocked phone, glances | App lock, discreet icon/name, notification tiers, switcher blur | App presence visible unless disguised |
| Compelled access | Device under legal compulsion | Passcode-only option, quick-lock, panic wipe, user-initiated deletion with crypto-erasure | Forensic extraction of unlocked device |
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

## Open decisions
1. Data model (what a journal entry contains: product, strain, brand, delivery method, dose, effects, ratings, notes, photos, spot).
2. Keyboards and input (refuse third-party keyboards on iOS; no personalized learning on Android; likely a Privacy Center toggle).
3. Accessibility vs. screen capture (FLAG_SECURE, recording blur, users who rely on accessibility services).
4. Local storage format (SQLCipher or equivalent, secure-delete settings, encrypted timestamps and identifiers).
5. Logs and crash reports (none in release, or opt-in with review before sending).
6. Sharing and export (metadata stripping, "leaves our protection" warning).
7. Memory hygiene (key zeroing, purge on background, re-auth timeout).
8. Age gating (self-attestation vs. app store requirements).
9. Device security features (passcode-only option, quick-lock, panic wipe, discreet icon or name).
10. Builds and releases (reproducible builds, signing-key protection, binary transparency).
11. Monetization (earning without touching user data).
12. App store policy (verify current Google Play and Apple cannabis-app rules).
13. Trademark clearance and filing for "Terpoir"; whether to form an LLC and move the repo to a GitHub organization.

# Terpoir Roadmap

Working plan for building Terpoir. `docs/design-decisions.md` is the source of
truth for *what* to build; this file is the order to build it in. Claude Code:
start at the first unchecked item, work on a branch, and open a pull request
per item or small group of items. Keep this file updated as items land.

## Maintainer tasks (outside the code)

- [x] Squash-merge PR #1 with a `Signed-off-by:` line in the merge message.
- [x] Settings → Code security: enable Dependabot alerts and Dependabot
      security updates. (Managed in `Terpoir-infra` since its first apply,
      2026-10-13.)
- [x] After the `DCO` workflow has run once, add `dco` as a required status
      check in the `protect-main` ruleset (exempt nothing; bots are handled
      inside the check). (In the ruleset with no bypass actors; managed in
      `Terpoir-infra` since 2026-10-13.)
- [ ] Trademark: USPTO clearance search, attorney consult, intent-to-use
      filing (Class 9).
- [x] Generate the Android signing key offline; import into two YubiKeys;
      offline encrypted backup. (Done by 2026-10-13.)
- [x] Pin the app signing certificate: its SHA-256 in
      `android/release/app-certificate.sha256`, its SHA-1 in Terpoir-infra's
      `maps_release_sha1s` (`docs/releasing.md`, one-time setup).
- [x] Create the Google Play developer account (personal); check current
      closed-testing requirements. (Done; first review passed 2026-10-14.)
- [ ] Recruit closed testers: 12 opted in for 14 days in a row before
      Play production access (3 as of 2026-10-14).
- [x] Play Console: declare background location for "Notify me" (the
      permissions declaration form and a short demonstration video) before
      the first release that includes it. Not needed for the first Play
      release, which leaves background location out (maintainer,
      2026-10-13). The full checklist, disclosure text, form answers and
      video script are in `docs/play-listing.md`, "Location declaration":
  - [x] Code PR (Claude): the Play build requests it, with the in-app
        disclosure screen, privacy policy and Data safety updates
        (2026-10-16).
  - [x] Record the video on an internal-testing build; upload it
        unlisted.
  - [x] Fill in App content → Sensitive app permissions → Location
        permissions, and send it for review with that release
        (done by 2026-10-21).
- [x] Mastodon account for release announcements (2026-10-15;
      `docs/design-decisions.md`, "Release announcements"):
  - [x] Create the account on mastodon.social (for example
        `terpoirapp`); mark it as a bot ("automated account") in its
        profile settings.
  - [x] Preferences → Development → New application, with only the
        `write:statuses` and `write:media` scopes; copy its access token.
  - [x] Add the token as `MASTODON_TOKEN` in the Terpoir repository's
        `announce` environment (Settings → Environments), after
        Terpoir-infra has created it.
  - [x] Copy the token to Keeper, folder `Terpoir` (Terpoir-infra D9).
- [ ] Strain data permission requests (2026-10-15;
      `docs/research/strain-lookups.md`). Fill in your name, send, then
      record each answer in the research doc (all sent by 2026-10-21;
      answers still to record):
  - [x] Kannapedia / Medicinal Genomics, `info@medicinalgenomics.com`:
        saved as a Gmail draft.
  - [x] AllBud, `legal@allbud.com`: saved as a Gmail draft.
  - [x] SeedFinder.eu: no address published; send through its contact
        form. Its text is a Gmail draft with no recipient.
  - [x] TerpHunter: sent through its contact form (2026-10-15). Its
        maintainer replied with personal Pro access through an MCP endpoint
        (2026-10-20); recorded in the research doc.
  - [x] TerpHunter follow-up sent (2026-10-21): bundling, per-user
        sign-in from the app, measured versus inferred values.
  - [ ] TerpHunter: try the connector and send feedback, then record the
        answer.

## Phase 0: Repository and CI foundations

- [x] DCO enforcement workflow (`.github/workflows/dco.yml`).
- [x] Dependabot version updates for GitHub Actions.
- [x] Add `cargo` and `gradle` ecosystems to `.github/dependabot.yml`
      (grouped, 7-day cooldown) once `core/` and `android/` exist.
- [x] CodeQL code scanning (Rust, Kotlin; Swift later), actions pinned by SHA.
- [x] OpenSSF Scorecard workflow.
- [x] Rust CI: `cargo fmt --check`, `cargo clippy -D warnings` with lints
      denying `dbg_macro`, `print_stdout`, `print_stderr`, `todo`,
      `unwrap_used` in release code; `cargo test`.
- [x] `cargo-deny` (licenses compatible with GPL-3.0, advisories, banned
      crates, sources) and `cargo-vet` (audits).
- [x] Add the CI jobs as required status checks in the `protect-main`
      ruleset (maintainer step; managed in Terpoir-infra, applied
      2026-10-14).

## Phase 1: Rust core (`core/`)

Cargo workspace; every crate carries the SPDX header; no logging of content.
Run `cargo vet init` when creating the workspace (`core/supply-chain/`,
importing audits from trusted auditors): the `cargo-vet` CI job fails
without it.

- [x] `terpoir-crypto`: XChaCha20-Poly1305 AEAD, key wrapping, Argon2id,
      HKDF, random 128-bit IDs; key types using `zeroize` and `secrecy`.
      Recovery key encoding (word list or grouped Base32 with checksum).
      Key hierarchy: recovery → master → per-record keys; rotation re-wraps.
- [x] `terpoir-model`: record types (Session, Product, Strain, Brand, Spot,
      stream snippets, tags), schema version on every record, serialization,
      migrations.
- [x] `terpoir-store`: SQLCipher database with per-record encrypted blobs,
      minimal indexes, `secure_delete`, WAL truncation, vacuum; encrypted
      file storage for photos and clips (streaming AEAD); crypto-erasure on
      delete.
- [x] `terpoir-sync`: hybrid logical clocks, per-field LWW with device-ID
      tie-break, set union for new records, ID-only deletion markers, blob
      format with padding and opaque names. Property tests proving
      convergence regardless of merge order.
- [x] `terpoir-ffi`: UniFFI bindings (Kotlin first).
- [x] Reproducible-build check for the core.

## Phase 2: Android shell (`android/`)

- [x] Kotlin + Jetpack Compose app; `play` and `foss` build flavors (FOSS:
      no Play services, AOSP location, no Gemini Nano, no Play age signals,
      no Play Billing).
- [x] Keystore (StrongBox where available) wrapping key; crypto-bound
      BiometricPrompt unlock (biometric or device passcode); auto-lock with
      1-minute default; lock on device lock.
- [x] Backup exclusion; FLAG_SECURE screen protection (default on);
      accessibility-data-sensitive marking; app-switcher blur; no-learning
      and autofill opt-out on text fields.
- [x] Privacy Center skeleton with a toggle registry that requires a paired
      on-action and off-action for every entry.
- [x] Onboarding: age signals + self-attestation, recovery key creation and
      backup confirmation, Privacy Center tour with everything off.
- [x] R8 log stripping; release builds log nothing.

## Phase 3: Journaling features

- [x] Sessions, products, strains, brands; effects map and core vocabulary.
- [x] `terpoir-model`: add the **Purchase** record kind (date, optional
      Re-up Spot, line items with product, batch, price); remove
      `bought_at` and `price` from Product (pre-release, so no migration
      needed); add package date and scanned identifiers to Product; add the
      per-spot purchase-prompt flag to Spot; let session product lines
      reference a purchase line. Update the FFI records and tests.
- [x] Rapid product entry: receipt scan (multi-product, image discarded by
      default), label scan, on-device barcode / QR decoding, duplicate
      detection. Online code lookups and COA links opt-in.
- [x] OCR native libraries built from pinned source for F-Droid
      (`android/ocr-native/`, asynchronous `OCR native libraries`
      workflow, reproducible on two runners).
- [x] Privacy presets: "Most private" and YOLO Mode as sets of toggle
      states with one combined confirmation; "Custom" status; the
      optional threat-model question at onboarding. YOLO Mode includes
      "Google text recognition" on Play.
- [x] OCR engine for the scanner: Tesseract as a Play on-demand feature
      module plus asset pack (no INTERNET permission), bundled in foss;
      ML Kit (through Play services) as a Play-only opt-in in YOLO Mode.
      One engine interface. (Adding the ML Kit toggle to YOLO Mode is part
      of the privacy presets item.)
- [x] When the scanner adds Tesseract4Android: the `terpoir.ocr.aar`
      Gradle property that switches the `foss` flavor to the from-source
      AAR (JitPack repository declared only without it); check the foss
      build with it in the OCR workflow.
- [x] On-device smoke test of the from-source OCR AAR (built and verified,
      never run): the instrumented emulator job, built with the property,
      loads the libraries and recognizes text in a bundled test image.
- [x] Voice-first FAB with on-device speech recognition (never cloud).
- [x] Stream snippets; keyword tag suggestions with human confirmation.
- [x] Spots with purpose types; "Show me" mode first, then "Notify me"
      geofencing; notification tiers and check-ins.
- [x] Map picker for spot locations: an opt-in online map (MapTiler
      through MapLibre, builds with a key only), every request through one
      gateway that refuses all of them while the toggle is off.
- [x] Map providers (decided 2026-10-12).
  - [x] Keyless OpenFreeMap by default in every build; MapTiler with the
        user's own key; the project's MapTiler key dropped.
  - [x] Google Maps as an active Play-only choice (onboarding and YOLO
        Mode).
- [x] "What I track" in Settings: hide data points (brand, price, strain
      and more), synced, with keep-hidden or delete.
  - [x] Core: the synced tracking-preferences record, and deleting a data
        point from every record.
  - [x] Android: the screen, and editors, scans, insights and share cards
        that skip hidden data points.
- [x] Targeted symptoms (opt-in).
- [x] Encrypted export; import; restore on a new phone from an export
      and the recovery key.
- [x] Sharing cards and plaintext export.
- [x] Insights engine (opt-in, evidence thresholds, confounder flags).
- [x] Auto-lock default: lock with the device (no in-app timer); opt-in
      timeouts of immediately, 1, 5 or 15 minutes, with the setting's
      disclosure card.
- [x] Quick capture: "New note" app shortcut (optional voice/text pair),
      opening the capture sheet in the user's default mode; opt-in Quick
      Settings tile and home-screen widget, each a Privacy Center toggle
      with an off-action. No journal content on any surface.
- [x] Write-only capture: capture key pair in the core (private key
      wrapped by the master key, re-keyed on rotation), sealed captures in
      a versioned inbox, merged on unlock with "added while locked" review,
      secure deletion of merged captures. Property tests; FFI; capture
      sheet works while the journal is locked.
- [x] Shop insights: per-shop rating and hit rate, selection vs. shop
      effect, freshness by package date.
- [x] Purchase prompts on Re-up Spots: dwell-then-exit geofence, discreet
      "Quick log?", no visit log; "Show me" banner alternative.
- [x] Schema upgrades without data loss (maintainer, 2026-10-13; ahead of
      the remaining sync backends, because a backup that won't load is
      worse than a missing feature). See "Schema upgrades" in the design
      doc.
  - [x] Keep records from a newer app version: store them sealed and
        unchanged, and apply them automatically after the app updates, so
        an import or restore never drops them.
  - [x] Restore and import say plainly when an export or sync blob needs a
        newer app, before anything is changed, and never finish partially
        in silence.
  - [x] Versioned derived data: indexes (local day, references, tags,
        files), scan aliases and links, and Android caches (alert cache,
        geofences, notification cache) carry a version and are rebuilt on
        the first unlock after an upgrade, with re-linking where the
        rules changed. (Files aren't derived; scan aliases get their
        version when they are built: see the design doc.)
  - [x] Compatibility fixtures: an encrypted export, a sync blob and a
        database from each released version, checked in, and a test that
        every one still opens, migrates and round-trips. A release that
        breaks one fails CI. (v0.1.0's, in
        `core/crates/terpoir-ffi/tests/fixtures/`; v0.2.0 to v0.2.2
        changed no format. v0.4.0's adds the richer strain fields; v0.6.0's,
        written from its tag on 2026-10-16, subtypes and receipt formats;
        v0.6.1's, a pack's "each / in all" reading; v0.7.0's, a strain's
        breeder; v0.8.0's, a saved web lookup site; v0.8.1's, an archived
        product.)
  - [x] Downgrade: an older app opening a newer database refuses (as
        today) with a message to update, never a crash or a reset.
- [ ] Bring-your-own-storage sync backends and direct device pairing.
  - [x] First backend: a folder chosen through the system file picker
        (Syncthing, or cloud drives through their apps); no INTERNET.
    - [x] Shows it works: a probe file at each sync, and a green check
          ("Working") after a sync that worked (maintainer, 2026-10-14).
  - [ ] WebDAV / S3-compatible backends (need INTERNET through the gateway).
  - [ ] Direct device pairing (QR code, local transfer).
- [x] Privacy Center readability pass (maintainer, 2026-10-12): the screen
      is visually dense and hard to scan. Review it and try a few layout
      options for the maintainer to pick from. Next, as 0.3.0
      (maintainer, 2026-10-14).
  - [x] Options shown; the maintainer chose sections you open (B,
        2026-10-14).
  - [x] Built, and shipped in 0.2.2 (in testers' hands since).

## Phase 4: Release

- [x] Development builds: main's debug APKs as a rolling `dev-build`
      pre-release on GitHub Releases, replaced on every push.
- [x] Release pipeline: a `v*` tag builds the unsigned APKs and Play
      bundle into a draft release; the maintainer signs offline with the
      YubiKey (`android/scripts/sign-release.sh`, `docs/releasing.md`).
- [ ] Reproducible Android builds verified in CI; SBOM; signed release
      hashes published on terpoir.app; signing fingerprints in README.
  - [x] Reproducible Android builds verified in CI: the core and the whole
        foss APK (`Reproducible build`, `android-core` and `foss-apk`;
        2026-10-22).
  - [x] SBOMs (CycloneDX) on every release: foss and Play, the Rust core
        per Android target, the OCR libraries' sources (maintainer chose
        the CycloneDX tools, 2026-10-22).
  - [x] Signed release hashes on terpoir.app/releases/, every release
        from 0.1.0, published by `release.sh` through a pull request
        (`publish-hashes.sh`, 2026-10-22).
  - [x] Both signing-certificate fingerprints (the maintainer's key and
        Play App Signing) and the release-signing SSH key
        (`android/release/allowed_signers`) on terpoir.app ("Verify your
        download") and in the README, with the commands to check an APK,
        `SHA256SUMS.sig` and the tags. Until 0.9.0 the hashes stay in each
        release's signed `SHA256SUMS`.
  - [ ] Backup release-signing SSH key: an `ed25519-sk` key on the backup
        YubiKey, added to `allowed_signers`, the site and the README
        (maintainer step; then the backup can sign tags and releases).
- [ ] Google Play, F-Droid, GitHub Releases.
  - [x] GitHub Releases offer only the foss APK; the signed Play bundle
        goes to Google Play from `release.sh` (`play-upload.sh`, internal
        track as a draft by default). Needs the one-time Play setup in
        `docs/releasing.md`.
  - [x] First Play submission prepared: privacy policy at
        terpoir.app/privacy, `docs/play-listing.md` (listing, Data safety,
        IARC, declarations), store graphics, no background location on
        Play, and the developer-verification APK script.
  - [x] First Play review passed (2026-10-14), with the health
        declaration narrowed to Sleep Management.
  - [ ] Production access: a closed test with at least 12 testers opted
        in for 14 days in a row, if the developer account is personal and
        newer than 13 November 2023 (`docs/play-listing.md`, "Release").
  - [x] Play policy gaps found when checking the Help Center
        (`docs/play-listing.md`, "Checked"): the privacy policy as text
        inside the app (Privacy Center, from `site/privacy.html` bundled at
        build time); a reminder to consult a healthcare professional
        (Insights, Targeted symptoms); a disclosure before the camera and
        microphone permission prompts.
  - [ ] F-Droid recipe: builds the OCR AAR with
        `android/ocr-native/build.sh` and sets `terpoir.ocr.aar` for the
        foss build (outline in `android/ocr-native/README.md`); settle
        with F-Droid whether the source tarballs come in as `srclibs`.
    Tabled 2026-10-14; found while starting, both needed before F-Droid
    can publish the maintainer-signed APK (builds must match byte for byte):
    - `release.yml` builds the foss APK with JitPack's prebuilt OCR
      libraries; it must build `android/ocr-native` and pass
      `-Pterpoir.ocr.aar` as F-Droid will.
      - [x] Done (2026-10-22): release builds of both flavors use the
            from-source libraries, checked against `SHA256SUMS` and for
            logging; the reproducible-APK check builds the same way.
    - The Rust core library embeds build-machine paths (`$CARGO_HOME`
      registry sources, OpenSSL's build directory under `core/target`), so
      it differs between GitHub's runners and F-Droid's build server. Remap
      them (`--remap-path-prefix`) and extend the reproducible-build check
      to a different `CARGO_HOME` and to the whole foss APK.
      - [x] Path remapping and the core's reproducible Android build
            (`-Pterpoir.reproducible`, set by release builds), checked from
            two checkouts with different Cargo homes (2026-10-22).
      - [x] The whole foss APK in the reproducible-build check (2026-10-22).
    - Then: a helper script that installs the pinned Rust toolchain,
      targets and `cargo-ndk` (like Mullvad's `android/fdroid-build/`), the
      recipe with `Binaries` and `AllowedAPKSigningKeys`, and
      `fastlane/metadata/android/en-US/` for the listing. Test with
      `fdroid build` before submitting to `fdroiddata`.
      - [x] `android/fdroid/`: `setup.sh` (pinned rustup, toolchain,
            targets, `cargo-ndk`, SDK packages), the recipe with `Binaries`
            and `AllowedAPKSigningKeys`, and the `fastlane` listing with
            changelogs, which `release.yml` requires (2026-10-22). Passes
            `fdroid lint` and `fdroid scanner`.
      - [ ] `fdroid build` of the 0.9.0 tag matches the signed release
            (needs a machine that can download the OCR source tarballs),
            then the `fdroiddata` merge request at 1.0.0.
- [ ] Trademark Statement of Use after launch.
- [ ] Register the FOSS build's signing certificate for Android developer
      verification (Play Console, Android developer verification,
      `app.terpoir`, Add key; proof with
      `android/scripts/sign-verification-apk.sh` if asked). Deferred by the
      maintainer (2026-10-14) until enforcement reaches sideloaded and
      F-Droid installs (global from 2027); the Play build is registered
      through Play App Signing.

## Release plan (maintainer, 2026-10-14)

Which version each remaining item is expected in. Versions are estimates and
move as priorities do; 1.0 is the public launch, gated on Play's tester
requirement rather than on features.

| Version | Contents |
|---|---|
| 0.2.1 | Google Maps on Play (first build with the release Maps key), unlock prompt on open, newer-journal message. Rolling out 2026-10-14. |
| 0.2.2 | The Privacy Center readability pass (planned as 0.3.0), problem reports by email with an offer to report the last crash, exports and folder sync checking every file was saved, the app icon in notifications. |
| 0.3.0 | Privacy Center readability pass: layout options for the maintainer to pick from, then the chosen one. Shipped early, in 0.2.2; no 0.3.0 release. |
| 0.4.0 | Richer strains I: detail setting, terpene levels, lineage links, strain-profile fallback in insights. |
| 0.5.0 | Richer strains II, part 1: CBN, CBG, THCa and CBDV label amounts. Also the session timeline (hold-and-flick) and the session editor redesign, one Settings tab with a theme choice, folder sync's working check, and a fix for a crash when the journal locks while loading. |
| 0.5.1 | Richer strains II, part 2 (moved from 0.5.0, maintainer 2026-10-15): insights and shop effect grouped by lineage or terpene profile. |
| 0.5.2 | Fixes: the Keystore "did not respond" error (one unlock operation at a time), folder sync's card showing at once. The unlock screen in brand green; the Products tab's floating New button and strain-first titles; New entry as the journal's main button, Insights as a lightbulb, swipe-away location cards (maintainer, 2026-10-15). |
| 0.5.3 | Fixes: Google text recognition in Play release builds (R8 removed ML Kit's registrar constructors), the crash turning it off or restoring maximum privacy, a chosen sync folder or export file lost behind the system picker, the OCR download's error code in problem reports, the keyboard as an inset. The rating as faces; swipeable Products lists; Scan receipt and the product editor's Scan as floating buttons; the session timeline in shares; the receipt-scanning core (maintainer, 2026-10-15). |
| 0.6.0 | Simpler receipt scanning and product subtypes, together (maintainer, 2026-10-15): both change the synced format and share the scanning vocabulary, so one compatibility fixture and one update. Shipped: the review rebuilt as cards with the shop first, learned receipt names, size and quantity on purchase lines, shop phone and licence numbers, zoom, join and split; subtypes under each type (built-in and the user's own) and the Transdermal method; timeline insights ("How it usually goes"). |
| 0.6.1 | The rest of simpler receipt scanning, built after 0.6.0 was tagged: two photos for a long receipt ("Add the rest of the receipt"), a pack's amount read as each or in all (stored beside the 0.6.0 size, which 0.6.0 keeps untouched), "Seen on receipts as" with a delete on the product page, and merging two products that share a learned receipt name (done at Save, with Undo; the editor's "Merge with…" can keep either, maintainer 2026-10-16). Also (maintainer, 2026-10-16): the rating centred and starting from the middle face, strains and parents picked by search, the product editor's extra "Brand" heading gone, and "New entry" beside "New note" on the app shortcut, Quick Settings tiles and widget. |
| 0.6.2 | "Notify me" and purchase reminders on Google Play (maintainer, 2026-10-16): the Play build requests location "all the time" behind the in-app prominent disclosure. Released to internal testing first for the background-location declaration's video, and promoted once Google approves the declaration. |
| 0.6.3 | Fixes and settings (maintainer, 2026-10-16 and 17): unlocking from a "Notify me" alert on a locked phone waits for the phone instead of reporting a Keystore error; YOLO Mode widened by Privacy Center group, with "Notify me" kept separate and placed under "Show me"; no Save button on screens that save each change at once. Like 0.6.2, internal testing until Google approves the background-location declaration. |
| 0.6.4 | Tidier location buttons (maintainer, 2026-10-17): the "Show me" card on one row with icon actions ("Session", and "Purchase" at a Re-up Spot), and the spot editor's "Approximate", "Precise" and "Map" side by side at one size. Like 0.6.2, internal testing until Google approves the background-location declaration. |
| 0.7.0 | Scan recognition from the journal, timeline comparisons and the offline strain catalog (maintainer, 2026-10-17): the rest of "Scan recognition from the journal" (OCR-tolerant suggestions ranked by shop and brand, brand and strain detection on labels; matched text broken down over time went to the backlog, maintainer 2026-10-21); "comes on faster than your other edibles"-style comparisons from the session timeline, once there is enough data to make them trustworthy; and the offline catalog from the candidate found, Kushy's MIT strain table (name, type, breeder, crosses; `docs/research/strain-lookups.md`, option 1), bundled and on with no toggle; the provenance check was waived for its factual columns (maintainer, 2026-10-21). |
| 0.8.0 | Strain lookups you drive (maintainer, 2026-10-21 and 22): "Look up on the web" (saved sites opened in the browser, its own toggle) and sharing a strain's page back to fill the strain. Online sources inside the app moved to a later release, numbered once a source agrees (maintainer, 2026-10-22): they wait on the sources' replies and the maintainer's choice. Like 0.6.2 to 0.7.0, internal testing until Google approves the background-location declaration. |
| 0.8.1 | Archiving and fixes (maintainer, 2026-10-22), ahead of the F-Droid groundwork, which stays 0.9.0: archiving products that are done (out of the Products list's main section and the pickers, back on a new purchase); product cards with the strain as the title and the brand in the corner (picked from prototypes); the Products list opening at the top; one Terpoir task when a page is shared back from the browser. Like 0.6.2 to 0.8.0, internal testing until Google approves the background-location declaration. |
| 0.9.0 | F-Droid groundwork, a release of its own (maintainer, 2026-10-17): OCR built from source in `release.yml`, path remapping, reproducible check over the whole foss APK, SBOM, hashes on terpoir.app. Moved from 0.7.0 (maintainer, 2026-10-15), then from 0.8.0 (2026-10-17). |
| 1.0.0 | Public launch: Play production access, F-Droid submission, "Get Terpoir" with real links and fingerprints. |
| 1.x | WebDAV / S3 sync; direct device pairing; scan recognition "growing over time" (backlog); personal gear; lock-screen capture. Order to be set after 1.0. |
| 2.0 | iOS. |

## Feature releases

- [x] YOLO Mode widened (maintainer, 2026-10-16; `docs/design-decisions.md`,
      "Presets and YOLO Mode"): Insights, every Location toggle but "Notify
      me" (which stays separate) with Google Maps on Play, every
      notification toggle, catalogs (and online
      lookups once built), Google text recognition, the tiles and widget,
      screen protection off, auto-lock when the phone locks, everything in
      What I track and the Terpenes strain detail. Keyboard and voice and
      Sync stay as they are. Decide while building how a preset sets the
      two journal settings, and keep the combined confirmation listing
      each toggle's card.
- [x] "Notify me" right under "Show me" in the Privacy Center's Location
      group, so the two read as a pair (maintainer, 2026-10-16).
- [x] Settings save as they change (maintainer, 2026-10-16): What I track,
      the Privacy Center and other settings screens already apply each
      change at once, so a screen timeout or lock loses nothing. Drop the
      Save button that is shown but never enabled on What I track (and any
      other settings screen with one), so it doesn't suggest otherwise Done
      2026-10-17: What I track, Insights, Theme, Share and the
      text-recognition download screen have no Save button.

- [x] Richer strains (`docs/design-decisions.md`, "Richer strains").
  - [x] 0.4.0: journal-wide detail setting (Simple / Terpenes / Detailed),
        the terpene picker guiding to 3 or fewer, terpene levels
        (dominant, secondary, trace; existing terpenes unrated), parent
        strains as links (a typed name creates a minimal strain), insights
        falling back to the strain's dominant terpene, and a 0.4.0
        compatibility fixture.
    - [x] Core data model: terpene levels, parent links, "known for"
          cannabinoids and the synced detail step, through the FFI.
    - [x] Strain editor and detail setting in the app.
    - [x] Insights: the strain's dominant terpene when a label has none.
  - [x] 0.5.0 and 0.5.1: CBN, CBG, THCa and CBDV label amounts on
        products and "known for" cannabinoids on strains (0.5.0); insights
        and the shop effect grouped by shared parent or dominant terpene
        (0.5.1; moved by the maintainer, 2026-10-15).
    - [x] Label amounts for CBN, CBG, THCa and CBDV on products (core,
          plaintext export, product editor), and a fixture that writes one.
          "Known for" cannabinoids on strains shipped in 0.4.0.
    - [x] 0.5.1: insights and the shop effect grouped by shared parent or
          dominant terpene.
- [ ] Strain lookups: the offline catalog in 0.7.0, web lookups in 0.8.0, online sources later (`docs/design-decisions.md`, "Catalog
      lookups"; maintainer, 2026-10-14: Leafly and a seed database).
  - [x] Research, during 0.4.0: for each candidate (Leafly, seed
        databases, open strain datasets), whether it has an API or a
        downloadable dataset, its terms on caching and offline
        redistribution, its licence, and what a lookup reveals. A short
        comparison for the maintainer to choose from:
        `docs/research/strain-lookups.md`. Done with gaps: only
        GitHub-hosted sources could be read, so the providers' terms are
        unverified until their domains are allowed (`docs/follow-ups.md`).
  - [x] The offline source (maintainer, 2026-10-17): the candidate
        found, Kushy's MIT strain table, limited to its factual columns
        (research option 1). Online sources still to pick.
  - [x] An offline catalog, 0.7.0: bundled with the app (about 260 KB,
        maintainer, 2026-10-21) and searched on the device as a strain's
        name is typed, filling its type, breeder and parents as suggestions
        the user confirms; strains gain a breeder field. Kushy's provenance
        check was waived (maintainer, 2026-10-21: factual columns only);
        its cannabinoid figures were left out (no stated unit). Its MIT
        notice ships with the app's other notices
        (`assets/licenses/Kushy-MIT.txt`).
  - [x] "Look up on the web", 0.8.0 (maintainer, 2026-10-21): saved sites
        opened in the browser with the strain's name, its own toggle; the
        app fetches nothing.
  - [x] Sharing back to Terpoir, 0.8.0: selected text or a screenshot of a
        strain's page, read on the phone into suggestions for the last
        strain looked up, another, or a new one (maintainer, 2026-10-21).
  - [ ] Online sources the maintainer picks, after 0.8.0 (moved
        2026-10-22, waiting on the sources' replies), each its own toggle
        (off by default) with a disclosure card; search on submit only.

- [x] Simpler receipt scanning, 0.6.0 and 0.6.1 (maintainer, 2026-10-15): text
      recognition should fill the right parts of a new entry (brand,
      strain, form, size, price, shop, date) with less checking and
      typing. Options in
      `docs/research/receipt-scan-options.md` (scan-and-review cards, tap to
      assign on the photo, label first, learning from corrections).
      Chosen (2026-10-15): A with D behind it, plus shop-first header,
      wrapped-line joining, cut-off detection, abbreviation matching
      without an alias list, and size and quantity on the purchase line
      (`docs/design-decisions.md`, "Simpler review").
      Overlaps "Scan recognition from the journal".
      - [x] Core, first slice: wrapped names joined by layout with each
            item's rows, re-read from the user's grouping; cut-off
            detection; item breakdown from a built-in vocabulary;
            strain, brand and product suggestions by initials, letters
            in order and cut-off names, ranked by the shop; the shop
            matched from the header by name, phone and licence numbers.
      - [x] Learned receipt names (exact line on the product), in the
            core: learned, forgotten, synced and linked automatically;
            the "matched" chip came with the Android cards.
      - [x] Size and quantity on the purchase line, default size on the
            product (synced, with a compatibility fixture).
      - [x] Phone and licence numbers on spots (synced, with a fixture).
      - [x] 0.6.1: a pack with an amount: read the amount as the pack's total when
            amount × pack is implausibly large for the product, and an easy
            "each / in all" and unit change on the card (maintainer,
            2026-10-15).
      - [x] 0.6.1: merging two products that share a learned receipt name,
            keeping the most information from each (maintainer,
            2026-10-15).
      - [x] 0.6.1: a second photo for long receipts, joined on overlapping rows:
            `join_receipt_photos` in the core, and "Add the rest of the
            receipt" in the Android review.
      - [x] Android: review cards with chips, the shop header card, "Join
            with next line" and "Split here", the cut-off warning, and the
            photo's zoom.
      - [x] 0.6.1: "Seen on receipts as" with a delete on the product
            page.

- [x] Product subtypes (maintainer, 2026-10-15; `docs/design-decisions.md`,
      "Product subtypes"). In 0.6.0, with simpler receipt scanning
      (maintainer, 2026-10-15).
  - [x] The maintainer confirms the Transdermal timing (12 hours, points
        at 1, 2, 4, 8 and 12 hours, a check-in after 2 hours).
  - [x] Core: subtypes under each type (built-in list and the user's
        own), a subtype's suggested method, the Transdermal method, uneven
        timeline points; through the FFI; the compatibility fixture maker
        (the v0.6.0 fixture itself, written from the release tag, is in
        `fixtures/v0.6.0/`).
  - [x] Product editor: subtype chips under the type, and adding one's
        own with its usual method.
  - [x] Scanning terms for subtypes.

- [x] Scan recognition from the journal, 0.7.0 (`docs/design-decisions.md`,
      "Scan recognition from the journal"): learned receipt names with
      automatic linking on an exact match; OCR-tolerant suggestions
      ranked by shop and brand; brand and strain detection on labels;
      matched text broken down into brand, strain, product type and size
      over time. Later, an offline strain database (per "Catalog lookups").
      Done in 0.7.0 apart from "growing over time", moved to the backlog
      (maintainer, 2026-10-21).
  - [x] Brand and strain detection on labels: the journal's brands and
        strains named in full fill an empty Brand and Strains, and the
        offline catalog's offers "Add …" chips (2026-10-21).

- [x] Session timeline (maintainer, 2026-10-15; `docs/design-decisions.md`,
      "Session timeline"): how strong it felt over time replaces onset,
      peak and duration.
  - [x] Points per method, the hold-and-flick indicator, the chart with
        guessed points, "You last updated", in the session editor and the
        check-in screen; stored as levels per minute in the core.
  - [x] The timeline in shares.
  - [x] Insights that use the timeline: "How it usually goes" per method,
        product and strain (comes on, peaks, eases off), from set points.
  - [x] Timeline comparisons ("comes on faster than your other edibles"),
        once there is enough data to make them trustworthy (0.7.0).

## Phase 5: Stretch

- [ ] iOS app (Swift, UniFFI Swift bindings); submit as an individual,
      Android-only acceptable if rejected under guideline 5.1.1(ix).
- [ ] OS on-device and bundled LLM tagging tiers; bundled speech model.
- [ ] Terpoir-to-Terpoir record sharing.
- [ ] Sigstore transparency logging for releases.

## Backlog (not yet scheduled)

Ideas the maintainer wants kept, to be designed and scheduled later.

- [ ] Scan recognition "growing over time" (backlogged by the maintainer,
      2026-10-21, from 0.7.0): when a receipt line is linked to a product
      and the rest of the line is already explained, remember which words
      named the brand and which the strain (`STZ` → Stiiizy, `BD` → Blue
      Dream), so later receipts recognise them in products not logged yet.
      New synced data (needs a design entry and a compatibility fixture);
      the hard part is attributing words safely.

- [x] "Secure storage not responding" on unlock: reopened (maintainer,
      2026-10-16) with a report from 0.6.2 (12), Play build, Android 17
      (API 37): `android.security.keystore.UserNotAuthenticatedException`.
      0.5.2 limited unlocking to one Keystore operation at a time
      (`docs/design-decisions.md`, "One open unlock operation"); this looks
      like a different cause. Likely: the master key's wrapping key is
      created with `setUnlockedDeviceRequired(true)`, and Keystore refuses
      to initialise its cipher with `UserNotAuthenticatedException` while
      the phone itself is locked. `MainActivity.unlock` calls
      `vault.decryptCipher()` before the prompt and maps any exception to
      `error_keystore`, so an unlock attempted over the lock screen (the
      app opened from a notification, tile or widget, or the automatic
      prompt firing before the keyguard is gone) shows this error. Fix:
      treat `UserNotAuthenticatedException` at that step as "unlock your
      phone first" and retry once the device is unlocked
      (`KeyguardManager.isDeviceLocked`, `ACTION_USER_PRESENT`), not as a
      Keystore failure; the same for creating and restoring. The
      maintainer opened the app from a "Notify me" alert at one of the
      Play declaration's demo spots, which fits: a notification can open
      the app while the phone is still locked.
      Fixed 2026-10-16 (`docs/design-decisions.md`, "Opened over the
      lock screen").

- [ ] Solventless and solvent-based concentrates as a grouping for
      insights (tabled by the maintainer, 2026-10-15: possibly useful, but
      subtypes may be enough; `docs/design-decisions.md`, "Product
      subtypes").
- [ ] More themes (maintainer, 2026-10-14), each a choice next to
      System, Light and Dark on the Theme page, with its sample controls:
      - a Material 3 theme from the wallpaper (dynamic color, Android 12
        and later), following the phone's light or dark setting;
      - the same wallpaper colors with light and dark reversed: dark when
        the phone is light, light when it is dark.
      To design: how these combine with the light/dark choice, what older
      Android versions show, and keeping every text color at WCAG AA
      (`ThemeContrastTest` can't check colors taken from a wallpaper).
- [x] Play's edge-to-edge warnings (maintainer, 2026-10-15; Play Console
      on 0.4.0, versionCode 5): "Edge-to-edge may not display for all
      users" and "Your app uses deprecated APIs or parameters for
      edge-to-edge". Terpoir's only activity already calls
      `enableEdgeToEdge` and draws under the system bars. To check:
      - the deprecated calls Play found: `enableEdgeToEdge` itself sets the
        bar colors (`Window.setStatusBarColor`/`setNavigationBarColor`,
        deprecated in Android 15) for older versions, and our navigation-bar
        scrim is such a color; whether a newer `androidx.activity` avoids
        them;
      - the activities that come with Google libraries in the Play build
        (`PlayCoreDialogWrapperActivity`, `GoogleApiActivity`,
        `AgeSharingConsentWrapperActivity`), which we don't control;
      - every screen on Android 15 and later (insets under the bars, the
        keyboard, the timeline overlay).
      Then fix what is ours, and note in `docs/play-listing.md` what is a
      library's.
  - [x] Checked (2026-10-15): the deprecated calls are all
        androidx.activity's, none ours, and no release of it avoids them
        yet; the three library activities are listed in
        `docs/play-listing.md`. Every screen sits inside the root's
        `safeDrawing` padding; the one fix was the keyboard (`adjustResize`,
        so it is an inset, not a pan). See "Edge to edge" in the design
        doc. The deprecated-API warning stays until androidx changes.
- [ ] Personal gear for consumption methods (maintainer, 2026-10-12): let
      users name their own pieces (a favorite bong, a favorite pipe, a
      particular vaporizer) and pick one on a session's product line, so
      sessions can be compared piece against piece, as with any other
      insight. Decided: gear is an optional detail of a method, never a
      replacement for it. A line stays "smoked" and can add "the green
      bong"; each piece belongs to a method, and only that method's pieces
      are offered. Still to design: how it is stored (its own record or a
      tag kind), and how it shows in shares (excluded by default) and the
      plaintext export.
- [ ] Prompt frequency for location prompts (maintainer, 2026-10-14): let
      the user choose how often "Notify me" alerts and purchase reminders
      can come, from about hourly to a low-key "every so often" (for
      example: about hourly; a few times a day, today's 4-hour alert
      cooldown; once a day, today's purchase-reminder limit; every few
      days). To design: one setting for both or one each, the choices
      and defaults, and keeping the "no visit log" rule: only the end of
      the quiet period is stored (per spot for reminders, as opaque IDs),
      however long it is.
- [ ] Capture from the device lock screen (maintainer, 2026-10-13):
      investigate letting the quick-capture buttons work on the lock screen
      itself, without unlocking the phone, on the Android versions that
      support it. Write-only capture already seals notes without the
      journal's key, so the question is what the OS allows. To check:
      - the Quick Settings tile from the lock screen, instead of asking to
        unlock the phone first (it calls `unlockAndRun` today);
      - lock-screen widgets, on the devices and versions that offer them;
      - whether apps can offer a lock-screen shortcut, or only the system
        can;
      - showing the capture sheet over the lock screen, with nothing from
        the journal visible.
      For each: the minimum Android version, what a passer-by could see or
      do, and the toggle and disclosure card it needs. The design doc
      already keeps lock-screen capture as its own opt-in.

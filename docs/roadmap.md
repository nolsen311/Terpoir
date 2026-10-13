# Terpoir Roadmap

Working plan for building Terpoir. `docs/design-decisions.md` is the source of
truth for *what* to build; this file is the order to build it in. Claude Code:
start at the first unchecked item, work on a branch, and open a pull request
per item or small group of items. Keep this file updated as items land.

## Maintainer tasks (outside the code)

- [ ] Squash-merge PR #1 with a `Signed-off-by:` line in the merge message.
- [x] Settings → Code security: enable Dependabot alerts and Dependabot
      security updates. (Managed in `Terpoir-infra` since its first apply,
      2026-10-13.)
- [ ] After the `DCO` workflow has run once, add `dco` as a required status
      check in the `protect-main` ruleset (exempt nothing; bots are handled
      inside the check).
- [ ] Trademark: USPTO clearance search, attorney consult, intent-to-use
      filing (Class 9).
- [ ] Generate the Android signing key offline; import into two YubiKeys;
      offline encrypted backup.
- [ ] Create the Google Play developer account (personal); check current
      closed-testing requirements.
- [ ] Play Console: declare background location for "Notify me" (the
      permissions declaration form and a short demonstration video) before
      the first release that includes it.

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
- [ ] Add the CI jobs as required status checks in the `protect-main`
      ruleset (maintainer step).

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
- [ ] Map providers (decided 2026-10-12): keyless OpenFreeMap by default
      in every build; MapTiler with the user's own key; Google Maps as an
      active Play-only choice (onboarding and YOLO Mode). Drops the
      project's MapTiler key.
- [ ] "What I track" in Settings: hide data points (brand, price, strain
      and more), synced, with keep-hidden or delete.
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
- [ ] Write-only capture: capture key pair in the core (private key
      wrapped by the master key, re-keyed on rotation), sealed captures in
      a versioned inbox, merged on unlock with "added while locked" review,
      secure deletion of merged captures. Property tests; FFI; capture
      sheet works while the journal is locked.
- [x] Shop insights: per-shop rating and hit rate, selection vs. shop
      effect, freshness by package date.
- [x] Purchase prompts on Re-up Spots: dwell-then-exit geofence, discreet
      "Quick log?", no visit log; "Show me" banner alternative.
- [ ] Bring-your-own-storage sync backends and direct device pairing.
  - [x] First backend: a folder chosen through the system file picker
        (Syncthing, or cloud drives through their apps); no INTERNET.
  - [ ] WebDAV / S3-compatible backends (need INTERNET through the gateway).
  - [ ] Direct device pairing (QR code, local transfer).
- [ ] Privacy Center readability pass (maintainer, 2026-10-12): the screen
      is visually dense and hard to scan. Review it and try a few layout
      options for the maintainer to pick from. After the main
      functionality above.

## Phase 4: Release

- [x] Development builds: main's debug APKs as a rolling `dev-build`
      pre-release on GitHub Releases, replaced on every push.
- [ ] Reproducible Android builds verified in CI; SBOM; signed release
      hashes published on terpoir.app; signing fingerprints in README.
- [ ] Google Play, F-Droid, GitHub Releases.
  - [ ] F-Droid recipe: builds the OCR AAR with
        `android/ocr-native/build.sh` and sets `terpoir.ocr.aar` for the
        foss build (outline in `android/ocr-native/README.md`); settle
        with F-Droid whether the source tarballs come in as `srclibs`.
- [ ] Trademark Statement of Use after launch.

## Feature releases (after the first public release)

Features planned for the first versioned releases, each its own release.

- [ ] Scan recognition from the journal (`docs/design-decisions.md`,
      "Scan recognition from the journal"): learned receipt names with
      automatic linking on an exact match; OCR-tolerant suggestions
      ranked by shop and brand; brand and strain detection on labels;
      matched text broken down into brand, strain, product type and size
      over time. Later, an offline strain database (per "Catalog lookups").

## Phase 5: Stretch

- [ ] iOS app (Swift, UniFFI Swift bindings); submit as an individual,
      Android-only acceptable if rejected under guideline 5.1.1(ix).
- [ ] OS on-device and bundled LLM tagging tiers; bundled speech model.
- [ ] Terpoir-to-Terpoir record sharing.
- [ ] Sigstore transparency logging for releases.

## Backlog (not yet scheduled)

Ideas the maintainer wants kept, to be designed and scheduled later.

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
- [ ] Richer strains (maintainer, 2026-10-12): two ways to describe a
      strain, the user's choice. Simple: indica / sativa / hybrid, as now.
      Detailed: lineage as links to parent strains, and a terpene profile
      as rough proportions (such as dominant, secondary, trace), never
      exact percentages, which stay on products where a label states them.
      Insights could then use a strain's profile when a product's label
      lists no terpenes, and group related strains,
      and the shop effect (which compares by strain record for now) could
      compare by lineage or terpene profile.

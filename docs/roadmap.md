# Terpoir Roadmap

Working plan for building Terpoir. `docs/design-decisions.md` is the source of
truth for *what* to build; this file is the order to build it in. Claude Code:
start at the first unchecked item, work on a branch, and open a pull request
per item or small group of items. Keep this file updated as items land.

## Maintainer tasks (outside the code)

- [ ] Squash-merge PR #1 with a `Signed-off-by:` line in the merge message.
- [ ] Settings → Code security: enable Dependabot alerts and Dependabot
      security updates.
- [ ] After the `DCO` workflow has run once, add `dco` as a required status
      check in the `protect-main` ruleset (exempt nothing; bots are handled
      inside the check).
- [ ] Trademark: USPTO clearance search, attorney consult, intent-to-use
      filing (Class 9).
- [ ] Generate the Android signing key offline; import into two YubiKeys;
      offline encrypted backup.
- [ ] Create the Google Play developer account (personal); check current
      closed-testing requirements.

## Phase 0: Repository and CI foundations

- [x] DCO enforcement workflow (`.github/workflows/dco.yml`).
- [x] Dependabot version updates for GitHub Actions.
- [ ] Add `cargo` and `gradle` ecosystems to `.github/dependabot.yml`
      (grouped, 7-day cooldown) once `core/` and `android/` exist.
- [x] CodeQL code scanning (Rust, Kotlin; Swift later), actions pinned by SHA.
- [ ] OpenSSF Scorecard workflow.
- [ ] Rust CI: `cargo fmt --check`, `cargo clippy -D warnings` with lints
      denying `dbg_macro`, `print_stdout`, `print_stderr`, `todo`,
      `unwrap_used` in release code; `cargo test`.
- [ ] `cargo-deny` (licenses compatible with GPL-3.0, advisories, banned
      crates, sources) and `cargo-vet` (audits).
- [ ] Add the CI jobs as required status checks in the `protect-main`
      ruleset (maintainer step).

## Phase 1: Rust core (`core/`)

Cargo workspace; every crate carries the SPDX header; no logging of content.

- [ ] `terpoir-crypto`: XChaCha20-Poly1305 AEAD, key wrapping, Argon2id,
      HKDF, random 128-bit IDs; key types using `zeroize` and `secrecy`.
      Recovery key encoding (word list or grouped Base32 with checksum).
      Key hierarchy: recovery → master → per-record keys; rotation re-wraps.
- [ ] `terpoir-model`: record types (Session, Product, Strain, Brand, Spot,
      stream snippets, tags), schema version on every record, serialization,
      migrations.
- [ ] `terpoir-store`: SQLCipher database with per-record encrypted blobs,
      minimal indexes, `secure_delete`, WAL truncation, vacuum; encrypted
      file storage for photos and clips (streaming AEAD); crypto-erasure on
      delete.
- [ ] `terpoir-sync`: hybrid logical clocks, per-field LWW with device-ID
      tie-break, set union for new records, ID-only deletion markers, blob
      format with padding and opaque names. Property tests proving
      convergence regardless of merge order.
- [ ] `terpoir-ffi`: UniFFI bindings (Kotlin first).
- [ ] Reproducible-build check for the core.

## Phase 2: Android shell (`android/`)

- [ ] Kotlin + Jetpack Compose app; `play` and `foss` build flavors (FOSS:
      no Play services, AOSP location, no Gemini Nano, no Play age signals,
      no Play Billing).
- [ ] Keystore (StrongBox where available) wrapping key; crypto-bound
      BiometricPrompt unlock (biometric or device passcode); auto-lock with
      1-minute default; lock on device lock.
- [ ] Backup exclusion; FLAG_SECURE screen protection (default on);
      accessibility-data-sensitive marking; app-switcher blur; no-learning
      and autofill opt-out on text fields.
- [ ] Privacy Center skeleton with a toggle registry that requires a paired
      on-action and off-action for every entry.
- [ ] Onboarding: age signals + self-attestation, recovery key creation and
      backup confirmation, Privacy Center tour with everything off.
- [ ] R8 log stripping; release builds log nothing.

## Phase 3: Journaling features

- [ ] Sessions, products, strains, brands; effects map and core vocabulary.
- [ ] Voice-first FAB with on-device speech recognition (never cloud).
- [ ] Stream snippets; keyword tag suggestions with human confirmation.
- [ ] Spots with purpose types; "Show me" mode first, then "Notify me"
      geofencing; notification tiers and check-ins.
- [ ] Targeted symptoms (opt-in).
- [ ] Sharing cards and encrypted / plaintext export; import.
- [ ] Insights engine (opt-in, evidence thresholds, confounder flags).
- [ ] Bring-your-own-storage sync backends and direct device pairing.

## Phase 4: Release

- [ ] Reproducible Android builds verified in CI; SBOM; signed release
      hashes published on terpoir.app; signing fingerprints in README.
- [ ] Google Play, F-Droid, GitHub Releases.
- [ ] Trademark Statement of Use after launch.

## Phase 5: Stretch

- [ ] iOS app (Swift, UniFFI Swift bindings); submit as an individual,
      Android-only acceptable if rejected under guideline 5.1.1(ix).
- [ ] OS on-device and bundled LLM tagging tiers; bundled speech model.
- [ ] Terpoir-to-Terpoir record sharing.
- [ ] Sigstore transparency logging for releases.

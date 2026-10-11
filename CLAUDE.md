# CLAUDE.md

Standing instructions for Claude Code working in this repository.

## Read first

`docs/design-decisions.md` is the source of truth for architecture, the threat
model, and decisions already made. Do not contradict a recorded decision
without raising it with the maintainer first. When a decision is made in a
session, update that doc in the same change.

`docs/roadmap.md` is the build order. Start at the first unchecked item in
the earliest phase, and tick items off in the same pull request that
completes them.

## Workflow

- `main` is protected: never push to it. Work on a branch and open a pull
  request; the maintainer reviews, signs off, and squash-merges.
- Pin every GitHub Action by full commit SHA with a version comment.

## Architecture

- `core/` (planned): Rust library owning encryption, the data model, sync,
  and merge logic. Exposed to platforms via UniFFI.
- `android/` (planned, first): Kotlin + Jetpack Compose. UI and OS
  integration only.
- `ios/` (planned, second): Swift + SwiftUI. UI and OS integration only.
- No server component. Never add one.

## Non-negotiable rules

- **Crypto, data model, sync, and merge logic belong in the Rust core.**
  Never reimplement them in Kotlin or Swift.
- **Use established crypto primitives from vetted crates.** Never hand-roll
  cryptography. Wipe key material with `zeroize`.
- **Per-entry data keys.** Each entry and its photos get their own random key,
  wrapped by the master key. Deleting an entry destroys its wrapped key.
- **Sync merge:** per-field last-writer-wins using hybrid logical clocks with
  device-ID tie-break; set union for new entries; deletion markers carry only
  the entry ID.
- **No third-party SDKs** for analytics, ads, crash reporting, or attribution.
  Justify every new dependency (network access, telemetry, license).
- **No journal content, keys, coordinates, or identifiers in logs**, in any
  build type. Strip logging from release builds.
- **Every privacy toggle is a pair:** an on-action and an off-action that
  cleans up what the on-action created (indexes, geofences, shortcuts, etc.).
  Register both with the Privacy Center.
- **Defaults are the most private option.** Leaky features are opt-in and come
  with a plain-language disclosure card.
- **Nothing leaves the encrypted store unencrypted** except through an
  explicit, user-initiated, disclosed action (export, share, rich
  notifications, OS indexing).
- **App data is excluded from OS backups**; keys live in Android Keystore
  (StrongBox where available) and the iOS Keychain (this-device-only).
- **The app never contacts terpoir.app** or any project-run endpoint.

## Conventions

- New source files carry the SPDX header from `LICENSING.md`.
- Human contributors sign off commits (`git commit -s`), per `CONTRIBUTING.md`.
  The sign-off is a personal legal certification: never add a
  `Signed-off-by` line on anyone's behalf. The maintainer signs off on
  Claude-authored changes when merging.
- Never commit signing keys, keystores, provisioning profiles, or secrets.

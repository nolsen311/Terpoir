# Terpoir

A private journal for cannabis connoisseurs. Record your experiences with
strains, products, brands, and delivery methods, and learn how they affect
*you*, without handing that record to anyone else.

> **Status:** pre-development. The design is documented in
> [docs/design-decisions.md](docs/design-decisions.md). No releases yet.

## Principles

- **Privacy first.** Your journal is encrypted on your device. Anything that
  leaves it is ciphertext only you can read.
- **Consent first.** Every feature that could reveal something about you is
  off by default, explained in plain language, and reversible.
- **Honest disclosure.** When a feature leaks information, Terpoir tells you
  exactly what, to whom, and how to avoid it.

## How it works

- **No Terpoir servers.** The project never hosts, sees, or can hand over your
  data.
- **Bring your own storage.** Sync through iCloud Drive, Google Drive,
  WebDAV/Nextcloud, S3-compatible storage, Syncthing, or encrypted
  export files. Providers see only encrypted blobs.
- **You hold the keys.** A recovery key you back up yourself. There is no
  escrow, so losing every device and your recovery key means losing your
  journal.
- **Location stays on your phone.** Saved spots and "near a spot" alerts are
  computed on-device and never sent anywhere.
- **One audited core.** Encryption, sync, and the data model live in a shared
  Rust library used by both the Android and iOS apps.

## Platforms

Native Android (Kotlin, Jetpack Compose) first, then native iOS (Swift),
both on a shared Rust core via UniFFI.

## Verifying official builds

Only builds signed with the keys published on https://terpoir.app are
official. Signing-certificate fingerprints and reproducible-build hashes will
be published there and here once releases begin.

**No one from Terpoir will ever ask for your recovery key.**

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). Security issues: see
[SECURITY.md](SECURITY.md).

## License

GPL-3.0-or-later, with an additional permission allowing app store
distribution. See [LICENSING.md](LICENSING.md) and [LICENSE](LICENSE).

"Terpoir" and its logo are not covered by the code license. See
[TRADEMARKS.md](TRADEMARKS.md).

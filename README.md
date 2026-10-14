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

Only builds signed with these certificates are official. They are also
published on https://terpoir.app#verify; a difference between the two is a
reason to stop.

| Builds | Signed by | Certificate SHA-256 |
|---|---|---|
| GitHub Releases (and F-Droid, later) | The maintainer's key | `55:9B:59:33:95:3E:2F:13:0A:23:DB:5A:4F:61:0D:12:83:D4:B6:A7:1E:E6:7F:9C:DE:CE:3E:C9:B6:EF:2C:A9` |
| Google Play | Google (Play App Signing) | `B7:6E:05:CD:31:96:52:AC:6F:1F:C3:FE:08:E7:66:ED:4F:E8:F5:45:DE:E7:5E:C1:19:DC:8A:B2:D1:A1:B3:CB` |

Check an APK with `apksigner verify --print-certs <apk>`.

Each release's `SHA256SUMS` and every release tag are signed with the
maintainer's hardware SSH key (fingerprint
`SHA256:LCGONnBz4qi3yn8ABDBTtb5lAxpVDR8MaXvtPxyQ7Lc`), listed in
[`android/release/allowed_signers`](android/release/allowed_signers):

```bash
ssh-keygen -Y verify -f android/release/allowed_signers \
    -I 64344+nolsen311@users.noreply.github.com -n file \
    -s SHA256SUMS.sig < SHA256SUMS
sha256sum -c SHA256SUMS
git -c gpg.ssh.allowedSignersFile=android/release/allowed_signers verify-tag v0.2.2
```

Reproducible-build hashes will be published on terpoir.app too.

**No one from Terpoir will ever ask for your recovery key.**

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). Security issues: see
[SECURITY.md](SECURITY.md).

## License

GPL-3.0-or-later, with additional permissions allowing app store
distribution and, in the Google Play build only, linking with Google's Play
libraries. See [LICENSING.md](LICENSING.md) and [LICENSE](LICENSE).

"Terpoir" and its logo are not covered by the code license. See
[TRADEMARKS.md](TRADEMARKS.md).

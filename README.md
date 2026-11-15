# Terpoir

A private journal for adults who use cannabis where it's legal. Note what
you tried, how much, and how it felt, then look back and see what works for
*you*, without handing that record to anyone else. Terpoir sells nothing,
recommends nothing, and never tells you to use more.

> **Status:** Android is released (0.10.2): on GitHub Releases and in
> closed testing on Google Play, with F-Droid submission planned for 2027.
> The iOS app is in development, with its first build going to TestFlight.
> See [docs/roadmap.md](docs/roadmap.md).

## What it does

- **Sessions:** when, which products, how much, and the effects you felt,
  with an intensity for each. Optional mood, intention, timeline and
  symptoms.
- **Products, strains and brands,** with label details such as THC and CBD
  and top terpenes; purchases, by hand or from a scanned receipt.
- **Notes,** typed or spoken; speech is turned into text on the phone and
  the audio is never kept. A quick-note tile and widget seal a note until
  you unlock.
- **Scanning:** product labels, receipts and barcodes, recognized on the
  phone.
- **Spots:** saved places with an optional location, address search, and
  optional alerts when you stop at one.
- **Insights** worked out on the phone from your own entries. Patterns, not
  advice.
- **What I track:** turn off anything you don't care about, and Terpoir
  stops asking for it.

## Privacy

- **No account and no Terpoir server.** The project never hosts, sees or
  can hand over your journal, and the app never contacts terpoir.app.
- **Encrypted on your device.** Each entry has its own key, wrapped by a
  master key kept in Android Keystore or the iOS Keychain. The journal
  unlocks with your fingerprint, face or screen lock, and a recovery key
  that only you hold. There's no escrow: losing every device and the
  recovery key means losing the journal.
- **Out of backups.** App data is excluded from OS backups.
- **No ads, analytics, crash reporting or tracking.**
- **Private by default.** Every optional feature that could reveal
  something is a switch in the Privacy Center, off until you turn it on,
  with a plain card saying what leaves the phone, to whom, and how to undo
  it.
- **Location stays on your phone.** Spots and "near a spot" alerts are
  worked out on the device.

## Your data

- **Sync through a folder you choose,** such as a Syncthing folder or a
  cloud drive app's folder. Only encrypted files are written there, and
  whatever syncs that folder moves them: Terpoir uploads nothing itself.
- **Export everything** as one encrypted file that only the recovery key
  opens, or as readable JSON and CSV files in a ZIP, behind a warning.
- Other storage backends (WebDAV/Nextcloud, S3-compatible) and direct
  device pairing are planned.

## Platforms and code

Native apps on one shared Rust core:

| Directory | What |
|---|---|
| [`core/`](core/README.md) | Rust: encryption, the data model, storage, sync and merging, exposed through UniFFI. One implementation to audit. |
| [`android/`](android/README.md) | Kotlin and Jetpack Compose: UI and OS integration only. Two flavours: `foss` (GitHub, F-Droid) and `play` (Google Play, with optional Google features). |
| [`ios/`](ios/README.md) | Swift and SwiftUI: UI and OS integration only. Each platform gets every feature the other has. |
| [`docs/`](docs/) | [Design decisions](docs/design-decisions.md) (the source of truth), the [roadmap](docs/roadmap.md), and [releasing](docs/releasing.md). |

Each directory's README says how to build and test it.

## Verifying official builds

Only builds signed with these certificates are official. They are also
published on https://terpoir.app#verify; a difference between the two is a
reason to stop.

| Builds | Signed by | Certificate SHA-256 |
|---|---|---|
| GitHub Releases (and F-Droid, later) | The maintainer's key | `55:9B:59:33:95:3E:2F:13:0A:23:DB:5A:4F:61:0D:12:83:D4:B6:A7:1E:E6:7F:9C:DE:CE:3E:C9:B6:EF:2C:A9` |
| Google Play | Google (Play App Signing) | `B7:6E:05:CD:31:96:52:AC:6F:1F:C3:FE:08:E7:66:ED:4F:E8:F5:45:DE:E7:5E:C1:19:DC:8A:B2:D1:A1:B3:CB` |

Check an APK with `apksigner verify --print-certs <apk>`.

The foss APK on GitHub Releases is built reproducibly in CI: anyone can
rebuild it from the tag and compare. Each release lists what it is built
from (SBOMs).

Each release's `SHA256SUMS` and every release tag are signed with the
maintainer's hardware SSH key (fingerprint
`SHA256:LCGONnBz4qi3yn8ABDBTtb5lAxpVDR8MaXvtPxyQ7Lc`), listed in
[`android/release/allowed_signers`](android/release/allowed_signers):

```bash
ssh-keygen -Y verify -f android/release/allowed_signers \
    -I 64344+nolsen311@users.noreply.github.com -n file \
    -s SHA256SUMS.sig < SHA256SUMS
sha256sum -c SHA256SUMS
git -c gpg.ssh.allowedSignersFile=android/release/allowed_signers verify-tag v0.10.2
```

Every release's signed `SHA256SUMS` is also on
[terpoir.app/releases](https://terpoir.app/releases/), so a download can be
checked against two places.

**No one from Terpoir will ever ask for your recovery key.**

## Not medical advice

Terpoir is a personal self-tracking tool. It isn't a medical device and
doesn't diagnose, treat, cure or prevent any condition.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). Security issues: see
[SECURITY.md](SECURITY.md).

## License

GPL-3.0-or-later, with additional permissions allowing app store
distribution and, in the Google Play build only, linking with Google's Play
libraries. See [LICENSING.md](LICENSING.md) and [LICENSE](LICENSE).

"Terpoir" and its logo are not covered by the code license. See
[TRADEMARKS.md](TRADEMARKS.md).

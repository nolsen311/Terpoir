<!-- SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception -->
<!-- Copyright (C) 2026 The Terpoir Authors -->

# Compatibility fixtures

One directory per released version, written by that release's own code:
a database, an encrypted export, a sync folder and a capture inbox.
`../compat_fixtures.rs` finds every directory here and checks that this
version still opens, migrates and round-trips each of them
(docs/design-decisions.md, "Schema upgrades": old backups keep working).
A release that breaks one fails CI's `cargo test`.

**Every key and passphrase here is a published test value.** Never use
them for a real journal.

## Test credentials (every version)

| What | Value |
| --- | --- |
| Recovery key (bytes `0x00..=0x1f`) | `0400-20G3-0G2G-C1R8-1450-P30D-1R7H-048J-2CA1-A5GQ-30CH-M6RW-3MF1-Y1YN` |
| Master key | 32 bytes of `0x4d` |
| App passphrase | `terpoir fixture passphrase` |
| MapTiler key (device metadata) | `FixtureMapTilerKey0` |

The database key is random, wrapped by the master key in
`keys/wrapped_database_key.bin`. Each `manifest.json` repeats the
credentials and lists the record IDs the release made.

## Layout of `<version>/`

| Path | What |
| --- | --- |
| `README.md` | Which release and commit made it. |
| `manifest.json` | Credentials, record IDs, which sync blobs are older. |
| `keys/master_wrapped_by_recovery.bin` | The master key wrapped by the recovery key. |
| `keys/wrapped_database_key.bin` | The database key wrapped by the master key. |
| `keys/passphrase_params.bin`, `keys/passphrase_wrapped_master_key.bin` | The master key wrapped under the app passphrase (Argon2id). |
| `database/journal.db` | The SQLCipher database, closed after maintenance (no WAL). |
| `database/files/<id>` | The session's attached file (3000 bytes counting 0..=255), encrypted. |
| `export.terpoir` | The encrypted export (`TERPOIR1` archive) of the final journal. |
| `sync/<32 hex>.terpoir` | Sync folder blobs: one set written before the last edit and deletion, one after (see `manifest.json`). |
| `inbox/capture.pub`, `inbox/<id>.cap` | The capture inbox with one note sealed while locked and not merged yet. |

## The journal

Written by `make_compat_fixtures.rs` with fixed contents and clock:

- brand "Acme", and brand "Gone", deleted (only its ID remains, as a marker);
- strain "Blue Dream" (lineage, terpenes, type); since 0.4.0 also terpene
  levels (myrcene dominant, pinene secondary), parent "Haze" (a name-only
  strain, `parent_strain` in the manifest) and the cannabinoid CBG; since
  0.7.0 also the breeder "DJ Short" (`strain_breeder` in the manifest);
- spot "Corner shop" (purpose, coordinates, radius, purchase prompts on);
  since 0.6.0 also the phone number 5550102030 and the licence number
  C100000123LIC (`receipt_formats` in the manifest);
- product "Gummies" (form, brand, strain, THC/CBD, terpene, batch, package
  date, barcode, shop); since 0.5.0 also a CBN label amount
  (`product_cannabinoids` in the manifest); since 0.6.0 also the subtype
  gummy (`product_subtypes` in the manifest), and a default size of 10
  pieces, 100 mg (since 0.6.1 read as each piece's, `pack_per` in the manifest), and the learned receipt name "ACME GUMMIES 10PK 100MG"
  (`receipt_formats`);
- a purchase of it at the spot (batch, price); since 0.6.0 its line also
  has that size and a quantity of 2 (`receipt_formats`);
- tags "giggly" (effect) and "jaw tension" (symptom); since 0.6.0 also
  "Lozenge", a custom subtype under edible with tincture as its method
  (`subtype_tag` in the manifest);
- a session using the product from that purchase line, with every field
  set: effects (built-in and custom), symptoms (built-in and custom),
  timeline, moods, rating 4 then edited to 5, spot, one attachment; since
  0.5.0 also timeline levels and their update time (`session_timeline` in
  the manifest); since 0.6.0 also a second line, a 10 mg patch with no
  product, by the `transdermal` method;
- a note on the session, and a note captured while locked and merged;
- "What I track" with terpenes hidden (since 0.4.0, also the Detailed
  strain step), and one insight dismissed.
- since 0.8.0, a saved web lookup site, "AllBud" (`lookup_sites` in the
  manifest).
- since 0.8.1, the product archived at the end, after its purchase
  (`product_archived` in the manifest).

## Adding the next release

At each release (docs/releasing.md), from a checkout of the new tag,
never sharing its build directory with another checkout:

```bash
git worktree add /tmp/terpoir-vX.Y.Z vX.Y.Z
cd /tmp/terpoir-vX.Y.Z/core
cp <this repo>/core/crates/terpoir-ffi/tests/fixtures/make_compat_fixtures.rs \
  crates/terpoir-ffi/examples/
CARGO_TARGET_DIR=/tmp/terpoir-vX.Y.Z-target cargo run --locked \
  -p terpoir-ffi --example make_compat_fixtures -- /tmp/fixtures-vX.Y.Z
```

Copy `/tmp/fixtures-vX.Y.Z` here as `vX.Y.Z/` with a README like
`v0.1.0/README.md`, and run `cargo test`. The generator is not built
here; if the API it uses changed, update it so it writes the same
journal, and keep it here for the next release. If a release adds record
kinds or files, extend the generator and the test together; the test's
checks must still pass for every older directory.

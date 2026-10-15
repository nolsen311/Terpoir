<!-- SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception -->
<!-- Copyright (C) 2026 The Terpoir Authors -->

# Fixtures from v0.5.0

Written on 2026-10-15 by `../make_compat_fixtures.rs`, run as an example
in a checkout of tag `v0.5.0` (commit `d726635`) with its own
`CARGO_TARGET_DIR`. The core crates' version is still 0.1.0, which is
what `manifest.json` records. Credentials, layout and contents are in
`../README.md`; record IDs are in `manifest.json`.

This is the first fixture with 0.5.0's fields: a product's label amount
for CBN (`product_cannabinoids`) and a session's timeline levels with
their update time (`session_timeline`). The sizes match v0.4.0's:
sync blobs, and the export built from them, are padded to size buckets
before encryption, and the database grows in whole pages.

| File | Bytes |
| --- | --- |
| `database/journal.db` | 73728 |
| `database/files/ad33…a9` | 3036 |
| `export.terpoir` | 12467 |
| `sync/` (3 blobs) | 4137 + 8233 + 8233 |
| `inbox/` | 33 + 593 |
| `keys/` | 73 × 3 + 29 |

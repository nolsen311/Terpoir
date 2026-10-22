<!-- SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception -->
<!-- Copyright (C) 2026 The Terpoir Authors -->

# Fixtures from v0.8.1

Written on 2026-10-22 by `../make_compat_fixtures.rs`, run as an example
in a checkout of tag `v0.8.1` (commit `5ea881f`) with its own
`CARGO_TARGET_DIR`. The core crates' version is still 0.1.0, which is
what `manifest.json` records. Credentials, layout and contents are in
`../README.md`; record IDs are in `manifest.json`.

This is the first fixture with an archived product (`product_archived` in
the manifest): the Gummies, archived at the end, after its purchase,
stored as the product's `archived` field.

| File | Bytes |
| --- | --- |
| `database/journal.db` | 73728 |
| `database/files/517f…ea20a` | 3036 |
| `export.terpoir` | 12467 |
| `sync/` (3 blobs) | 8233 + 8233 + 4137 |
| `inbox/` | 593 + 33 |
| `keys/` | 73 + 29 + 73 + 73 |

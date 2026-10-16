<!-- SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception -->
<!-- Copyright (C) 2026 The Terpoir Authors -->

# Fixtures from v0.6.1

Written on 2026-10-16 by `../make_compat_fixtures.rs`, run as an example
in a checkout of tag `v0.6.1` (commit `7144c3c`) with its own
`CARGO_TARGET_DIR`. The core crates' version is still 0.1.0, which is
what `manifest.json` records. Credentials, layout and contents are in
`../README.md`; record IDs are in `manifest.json`.

This is the first fixture with a pack's "each / in all" reading
(`pack_per` in the manifest), stored beside the 0.6.0 size:
`line_size_per/<line>` on the purchase and `default_size_per` on the
product.

| File | Bytes |
| --- | --- |
| `database/journal.db` | 73728 |
| `database/files/06ce…c1a70` | 3036 |
| `export.terpoir` | 12467 |
| `sync/` (3 blobs) | 8233 + 4137 + 8233 |
| `inbox/` | 33 + 593 |
| `keys/` | 73 + 73 + 73 + 29 |

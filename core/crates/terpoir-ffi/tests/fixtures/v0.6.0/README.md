<!-- SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception -->
<!-- Copyright (C) 2026 The Terpoir Authors -->

# Fixtures from v0.6.0

Written on 2026-10-16 by `../make_compat_fixtures.rs`, run as an example
in a checkout of tag `v0.6.0` (commit `c38680e`) with its own
`CARGO_TARGET_DIR`. The core crates' version is still 0.1.0, which is
what `manifest.json` records. Credentials, layout and contents are in
`../README.md`; record IDs are in `manifest.json`.

This is the first fixture with the 0.6.0 formats: product subtypes (a
built-in one and the user's own, with its form and method), the
Transdermal method, a purchase line's size and quantity, a product's
usual size and learned receipt names. v0.6.0 had no "each / in all"
reading for a pack (0.6.1), so for this run the generator's `per` field
was left out and the manifest has no `pack_per`. v0.4.1 to v0.5.3
changed no format that v0.4.0's or v0.5.0's fixtures don't cover.

| File | Bytes |
| --- | --- |
| `database/journal.db` | 73728 |
| `database/files/d9d1…a17ad` | 3036 |
| `export.terpoir` | 12467 |
| `sync/` (3 blobs) | 8233 + 4137 + 8233 |
| `inbox/` | 33 + 593 |
| `keys/` | 73 + 73 + 73 + 29 |

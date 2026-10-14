<!-- SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception -->
<!-- Copyright (C) 2026 The Terpoir Authors -->

# Fixtures from v0.4.0

Written on 2026-10-14 by `../make_compat_fixtures.rs`, run as an example
in a checkout of tag `v0.4.0` (commit `a11593b`) with its own
`CARGO_TARGET_DIR`. The core crates' version is still 0.1.0, which is
what `manifest.json` records. Credentials, layout and contents are in
`../README.md`; record IDs are in `manifest.json`.

This is the first fixture with the 0.4.0 strain fields: terpene levels,
a parent strain (`parent_strain`), a cannabinoid and the Detailed strain
step. v0.2.1 and v0.2.2 changed no format, so v0.1.0's fixtures cover
them.

| File | Bytes |
| --- | --- |
| `database/journal.db` | 73728 |
| `database/files/3b4f…db01e` | 3036 |
| `export.terpoir` | 12467 |
| `sync/` (3 blobs) | 4137 + 8233 + 8233 |
| `inbox/` | 33 + 593 |
| `keys/` | 73 × 3 + 29 |

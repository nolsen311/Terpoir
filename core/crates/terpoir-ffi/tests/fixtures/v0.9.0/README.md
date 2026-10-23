<!-- SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception -->
<!-- Copyright (C) 2026 The Terpoir Authors -->

# Fixtures from v0.9.0

Written on 2026-10-23 by `../make_compat_fixtures.rs`, run as an example
in a checkout of tag `v0.9.0` (commit `aef4755`) with its own
`CARGO_TARGET_DIR`. The core crates' version is still 0.1.0, which is
what `manifest.json` records. Credentials, layout and contents are in
`../README.md`; record IDs are in `manifest.json`.

The core is unchanged since v0.8.1 (0.9.0 changed only how releases are
built), so this fixture has the same contents and file sizes as v0.8.1's,
with new random IDs, keys and nonces.

| File | Bytes |
| --- | --- |
| `database/journal.db` | 73728 |
| `database/files/16fd…87ea` | 3036 |
| `export.terpoir` | 12467 |
| `sync/` (3 blobs) | 8233 + 8233 + 4137 |
| `inbox/` | 593 + 33 |
| `keys/` | 73 + 29 + 73 + 73 |

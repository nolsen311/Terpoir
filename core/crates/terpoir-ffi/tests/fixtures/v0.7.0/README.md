<!-- SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception -->
<!-- Copyright (C) 2026 The Terpoir Authors -->

# Fixtures from v0.7.0

Written on 2026-10-21 by `../make_compat_fixtures.rs`, run as an example
in a checkout of tag `v0.7.0` (commit `4fde9c5`) with its own
`CARGO_TARGET_DIR`. The core crates' version is still 0.1.0, which is
what `manifest.json` records. Credentials, layout and contents are in
`../README.md`; record IDs are in `manifest.json`.

This is the first fixture with a strain's breeder (`strain_breeder` in
the manifest): "DJ Short" on Blue Dream, stored as the strain's
`breeder` field.

| File | Bytes |
| --- | --- |
| `database/journal.db` | 73728 |
| `database/files/f317…9fb27` | 3036 |
| `export.terpoir` | 12467 |
| `sync/` (3 blobs) | 8233 + 8233 + 4137 |
| `inbox/` | 593 + 33 |
| `keys/` | 73 + 29 + 73 + 73 |

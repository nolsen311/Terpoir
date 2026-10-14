<!-- SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception -->
<!-- Copyright (C) 2026 The Terpoir Authors -->

# Fixtures from v0.1.0

Written on 2026-10-14 by `../make_compat_fixtures.rs`, run as an example
in a checkout of tag `v0.1.0` (commit `e50a7dd`, core version 0.1.0) with
its own `CARGO_TARGET_DIR`. Credentials, layout and contents are in
`../README.md`; record IDs are in `manifest.json`.

v0.2.0 needs no fixtures of its own: between `v0.1.0` and `v0.2.0` the core
gained only `sync_folder_key_file` and `readable_blobs` (the sync folder's
`terpoir.key` is an export archive with no blobs). The database schema,
the `TERPOIR1` archive, the sync blob and the capture formats are the
same.

| File | Bytes |
| --- | --- |
| `database/journal.db` | 65536 |
| `database/files/7f4f…9b9a` | 3036 |
| `export.terpoir` | 12467 |
| `sync/` (3 blobs) | 4137 + 4137 + 8233 |
| `inbox/` | 33 + 593 |
| `keys/` | 73 × 3 + 29 |

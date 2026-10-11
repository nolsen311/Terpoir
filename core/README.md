<!-- SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception -->
# Terpoir core

The Rust core: encryption, data model, storage, sync and merge logic, exposed
to the apps through UniFFI. See `docs/design-decisions.md` for what it must
do and `docs/roadmap.md` for build order.

| Crate | Purpose |
|---|---|
| `terpoir-crypto` | AEAD, key wrapping, Argon2id, HKDF, record IDs, recovery key, streaming file encryption |

## Checks

Run from this directory; CI runs the same (`.github/workflows/rust.yml`,
`.github/workflows/supply-chain.yml`).

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
cargo deny --locked check
cargo vet --locked
```

The toolchain is pinned in `rust-toolchain.toml`.

## Adding or updating a dependency

State why in the pull request (network access, telemetry, license). Then:

```sh
cargo deny --locked check   # licenses, advisories, bans, sources
cargo vet                   # audit it, or record an exemption with a reason
```

Imported audits come from Mozilla, Google, the Bytecode Alliance, ISRG and
Zcash (`supply-chain/config.toml`).

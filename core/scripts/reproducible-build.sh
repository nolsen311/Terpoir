#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
# Copyright (C) 2026 The Terpoir Authors
# See LICENSING.md for the additional permission under GPLv3 section 7.
#
# Builds the core library reproducibly and prints its SHA-256. Anyone can
# run this on the same commit, from any directory, and get the same hash.
#
# Usage, from anywhere in a checkout:  core/scripts/reproducible-build.sh
#
# How:
# - The committed source (git archive, so local changes and untracked
#   files are excluded) is built at a fixed path with a fixed Cargo home,
#   because compiled-in paths (OpenSSL's ENGINESDIR/MODULESDIR, panic
#   locations in dependencies) would otherwise differ between machines.
# - SOURCE_DATE_EPOCH (the commit time) replaces OpenSSL's build date.
# - The toolchain is pinned in rust-toolchain.toml, dependencies in
#   Cargo.lock; the release profile uses one codegen unit, LTO and no
#   debug info.
#
# Needs: git, rustup, a C compiler, perl, make. Uses /tmp/terpoir-reproducible.

set -eu

work=/tmp/terpoir-reproducible
repo_dir=$(git -C "$(dirname "$0")" rev-parse --show-toplevel)

rm -rf "$work/src" "$work/target"
mkdir -p "$work/src" "$work/cargo"
git -C "$repo_dir" archive HEAD | tar -x -C "$work/src"

# Resolve the pinned toolchain before switching CARGO_HOME (rustup lives in
# the user's own Cargo home).
toolchain=$(sed -n 's/^channel = "\(.*\)"$/\1/p' "$work/src/core/rust-toolchain.toml")
rustup toolchain install "$toolchain" --profile minimal >/dev/null 2>&1
cargo_bin=$(rustup which --toolchain "$toolchain" cargo)
RUSTC=$(rustup which --toolchain "$toolchain" rustc)
export RUSTC

SOURCE_DATE_EPOCH=$(git -C "$repo_dir" log -1 --format=%ct)
export SOURCE_DATE_EPOCH
export CARGO_HOME="$work/cargo"
export CARGO_TARGET_DIR="$work/target"
export CARGO_INCREMENTAL=0
unset RUSTFLAGS CFLAGS CXXFLAGS RUSTUP_TOOLCHAIN

cd "$work/src/core"
"$cargo_bin" build --release --locked -p terpoir-ffi
sha256sum "$CARGO_TARGET_DIR/release/libterpoir_ffi.so" | cut -d' ' -f1

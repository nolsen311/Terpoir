// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

//! Binding generator for `terpoir-ffi`. A separate crate so the generator's
//! dependencies never end up in the app library.
//!
//! ```sh
//! cargo build -p terpoir-ffi
//! cargo run -p uniffi-bindgen -- generate \
//!     --library target/debug/libterpoir_ffi.so --language kotlin --out-dir out/
//! ```

fn main() {
    uniffi::uniffi_bindgen_main();
}

// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

//! Randomness from the operating system's CSPRNG.

use crate::{Error, Result};

/// Fills `dest` with cryptographically secure random bytes from the
/// operating system.
///
/// # Errors
///
/// [`Error::Random`] if the operating system's generator is unavailable.
pub fn fill_random(dest: &mut [u8]) -> Result<()> {
    getrandom::fill(dest).map_err(|_| Error::Random)
}

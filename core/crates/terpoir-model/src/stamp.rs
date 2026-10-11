// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

//! Field stamps: hybrid logical clock time plus the writing device.
//!
//! The clock itself (how stamps are issued and advanced) lives in
//! `terpoir-sync`; this module only defines the data and its ordering.

use core::fmt;
use serde::{Deserialize, Serialize};

use crate::{Error, Result};

/// A hybrid logical clock reading: wall-clock milliseconds since the Unix
/// epoch plus a counter that orders events within the same millisecond.
#[derive(
    Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default, Serialize, Deserialize,
)]
pub struct Hlc {
    /// Milliseconds since the Unix epoch (the larger of local wall time and
    /// any time seen from other devices).
    pub wall_ms: u64,
    /// Tie-breaker within one millisecond.
    pub counter: u32,
}

/// A random 128-bit device identifier, created once per installation.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct DeviceId([u8; 16]);

impl DeviceId {
    /// A new random device ID.
    ///
    /// # Errors
    ///
    /// [`Error::Random`] if the operating system's generator fails.
    pub fn random() -> Result<Self> {
        let mut bytes = [0u8; 16];
        terpoir_crypto::fill_random(&mut bytes).map_err(|_| Error::Random)?;
        Ok(Self(bytes))
    }

    /// Wraps existing bytes.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    /// The raw bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }
}

impl fmt::Debug for DeviceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("DeviceId(<redacted>)")
    }
}

/// When and where a field was last written. Stamps are totally ordered:
/// by clock reading, then by device ID, so every device picks the same
/// winner for concurrent writes.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub struct Stamp {
    /// Clock reading at the write.
    pub hlc: Hlc,
    /// The device that wrote.
    pub device: DeviceId,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stamp(wall_ms: u64, counter: u32, device: u8) -> Stamp {
        Stamp {
            hlc: Hlc { wall_ms, counter },
            device: DeviceId::from_bytes([device; 16]),
        }
    }

    #[test]
    fn ordering_is_time_then_counter_then_device() {
        assert!(stamp(1, 0, 9) < stamp(2, 0, 0));
        assert!(stamp(1, 1, 0) > stamp(1, 0, 9));
        assert!(stamp(1, 1, 2) > stamp(1, 1, 1));
    }

    #[test]
    fn device_debug_is_redacted() {
        assert_eq!(
            format!("{:?}", DeviceId::from_bytes([1; 16])),
            "DeviceId(<redacted>)"
        );
    }
}

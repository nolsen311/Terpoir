// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

//! Record identifiers.

use core::fmt;

use crate::{Error, Result, fill_random};

/// A random 128-bit record identifier.
///
/// IDs are never time-ordered, so they reveal nothing about when a record
/// was created. `Debug` output is redacted (identifiers stay out of logs);
/// use [`RecordId::to_hex`] where the value is actually needed.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RecordId([u8; 16]);

impl RecordId {
    /// Length in bytes.
    pub const LEN: usize = 16;

    /// A new random ID.
    ///
    /// # Errors
    ///
    /// [`Error::Random`] if the operating system's generator fails.
    pub fn random() -> Result<Self> {
        let mut bytes = [0u8; Self::LEN];
        fill_random(&mut bytes)?;
        Ok(Self(bytes))
    }

    /// Wraps existing bytes.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    /// Parses a byte slice of exactly [`RecordId::LEN`] bytes.
    ///
    /// # Errors
    ///
    /// [`Error::Format`] if the length is wrong.
    pub fn from_slice(bytes: &[u8]) -> Result<Self> {
        bytes.try_into().map(Self).map_err(|_| Error::Format)
    }

    /// The raw bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }

    /// Lowercase hexadecimal, 32 characters.
    #[must_use]
    pub fn to_hex(&self) -> String {
        const DIGITS: &[u8; 16] = b"0123456789abcdef";
        let mut out = String::with_capacity(Self::LEN * 2);
        for byte in self.0 {
            out.push(char::from(DIGITS[usize::from(byte >> 4)]));
            out.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
        }
        out
    }

    /// Parses 32 hexadecimal characters (either case).
    ///
    /// # Errors
    ///
    /// [`Error::Format`] on wrong length or a non-hex character.
    pub fn from_hex(hex: &str) -> Result<Self> {
        let hex = hex.as_bytes();
        if hex.len() != Self::LEN * 2 {
            return Err(Error::Format);
        }
        let mut bytes = [0u8; Self::LEN];
        let (pairs, _) = hex.as_chunks::<2>();
        for (byte, pair) in bytes.iter_mut().zip(pairs) {
            *byte = (hex_value(pair[0])? << 4) | hex_value(pair[1])?;
        }
        Ok(Self(bytes))
    }
}

fn hex_value(c: u8) -> Result<u8> {
    match c {
        b'0'..=b'9' => Ok(c - b'0'),
        b'a'..=b'f' => Ok(c - b'a' + 10),
        b'A'..=b'F' => Ok(c - b'A' + 10),
        _ => Err(Error::Format),
    }
}

impl fmt::Debug for RecordId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("RecordId(<redacted>)")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn random_ids_differ() {
        assert_ne!(RecordId::random().unwrap(), RecordId::random().unwrap());
    }

    #[test]
    fn hex_round_trip() {
        let id = RecordId::random().unwrap();
        let hex = id.to_hex();
        assert_eq!(hex.len(), 32);
        assert_eq!(RecordId::from_hex(&hex).unwrap(), id);
        assert_eq!(RecordId::from_hex(&hex.to_uppercase()).unwrap(), id);
    }

    #[test]
    fn hex_rejects_bad_input() {
        assert_eq!(RecordId::from_hex("00"), Err(Error::Format));
        assert_eq!(RecordId::from_hex(&"g".repeat(32)), Err(Error::Format));
    }

    #[test]
    fn slice_length_checked() {
        assert!(RecordId::from_slice(&[0u8; 16]).is_ok());
        assert_eq!(RecordId::from_slice(&[0u8; 15]), Err(Error::Format));
    }

    #[test]
    fn debug_is_redacted() {
        let id = RecordId::from_bytes([0xab; 16]);
        assert_eq!(format!("{id:?}"), "RecordId(<redacted>)");
    }
}

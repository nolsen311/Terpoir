// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

//! Property tests for sealing notes to the capture key pair.

// Test code may unwrap.
#![allow(clippy::unwrap_used)]

use proptest::prelude::*;
use terpoir_crypto::{CAPTURE_SEAL_OVERHEAD, CaptureKey, MasterKey};

proptest! {
    #![proptest_config(ProptestConfig::with_cases(128))]

    #[test]
    fn seal_open_round_trip(
        plaintext in proptest::collection::vec(any::<u8>(), 0..2048),
        aad in proptest::collection::vec(any::<u8>(), 0..64),
    ) {
        let key = CaptureKey::generate().unwrap();
        let sealed = key.public_key().seal(&aad, &plaintext).unwrap();
        prop_assert_eq!(sealed.len(), plaintext.len() + CAPTURE_SEAL_OVERHEAD);
        let opened = key.open(&aad, &sealed).unwrap();
        prop_assert_eq!(opened.as_slice(), plaintext.as_slice());
    }

    #[test]
    fn any_flipped_bit_is_refused(
        plaintext in proptest::collection::vec(any::<u8>(), 0..256),
        position in any::<prop::sample::Index>(),
        bit in 0u8..8,
    ) {
        let key = CaptureKey::generate().unwrap();
        let mut sealed = key.public_key().seal(b"id", &plaintext).unwrap();
        let i = position.index(sealed.len());
        sealed[i] ^= 1 << bit;
        prop_assert!(key.open(b"id", &sealed).is_err());
    }

    #[test]
    fn only_the_wrapping_master_unwraps(_seed in any::<u8>()) {
        let master = MasterKey::generate().unwrap();
        let key = CaptureKey::generate().unwrap();
        let wrapped = master.wrap_capture_key(&key).unwrap();
        let unwrapped = master.unwrap_capture_key(&wrapped, &key.public_key()).unwrap();
        // The unwrapped key opens what was sealed to the original.
        let sealed = key.public_key().seal(b"", b"note").unwrap();
        let opened = unwrapped.open(b"", &sealed).unwrap();
        prop_assert_eq!(opened.as_slice(), b"note");
        let other = MasterKey::generate().unwrap();
        prop_assert!(other.unwrap_capture_key(&wrapped, &key.public_key()).is_err());
    }
}

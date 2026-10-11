// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

//! Cryptographic primitives and the key hierarchy for Terpoir.
//!
//! Everything here is built from vetted RustCrypto crates; nothing is
//! hand-rolled. See `docs/design-decisions.md` ("Keys and recovery", "Local
//! storage format", "Memory hygiene").
//!
//! # Key hierarchy
//!
//! ```text
//! RecoveryKey ──HKDF──▶ KeyEncryptionKey ─┐
//! passphrase ──Argon2id─▶ KeyEncryptionKey ├─wraps─▶ MasterKey ─wraps─▶ RecordKey (one per record)
//! platform keystore (Android/iOS) ────────┘                    └─wraps─▶ DatabaseKey
//! ```
//!
//! - A [`RecordKey`] encrypts one record and the files attached to it.
//!   Destroying its wrapped copy crypto-erases the record.
//! - The [`MasterKey`] only wraps other keys. Rotating it re-wraps record
//!   keys; record ciphertext is untouched.
//! - The [`RecoveryKey`] is the user-held root, written down or stored in a
//!   password manager.
//!
//! # Primitives
//!
//! - XChaCha20-Poly1305 for record encryption and key wrapping (random
//!   192-bit nonces), and in the STREAM construction for files.
//! - Argon2id for passphrase-derived keys.
//! - HKDF-SHA-256 for subkeys.
//! - The operating system's CSPRNG (`getrandom`) for keys, nonces and IDs.
//!
//! All key types zeroize on drop, never implement `Clone`, and print as
//! `<redacted>` in `Debug` output.

#![forbid(unsafe_code)]

mod aead;
mod error;
mod id;
mod kdf;
mod keys;
mod random;
mod recovery;
mod stream;

pub use aead::{SEAL_OVERHEAD, WRAPPED_KEY_LEN, WrappedKey};
pub use error::{Error, RecoveryKeyError, Result};
pub use id::RecordId;
pub use kdf::{PassphraseParams, SALT_LEN};
pub use keys::{DatabaseKey, KEY_LEN, KeyEncryptionKey, MasterKey, RecordKey, SubKey};
pub use random::fill_random;
pub use recovery::RecoveryKey;
pub use stream::{CHUNK_LEN, STREAM_HEADER_LEN, decrypt_stream, encrypt_stream};

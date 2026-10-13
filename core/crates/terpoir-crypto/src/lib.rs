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
//! platform keystore (Android/iOS) ────────┘                    ├─wraps─▶ DatabaseKey
//!                                                              └─wraps─▶ CaptureKey (private half of the capture key pair)
//! ```
//!
//! - A [`RecordKey`] encrypts one record and the files attached to it.
//!   Destroying its wrapped copy crypto-erases the record.
//! - The [`MasterKey`] only wraps other keys. Rotating it re-wraps record
//!   keys; record ciphertext is untouched.
//! - A [`CaptureKey`] opens notes written while the journal is locked:
//!   they are sealed to its [`CapturePublicKey`], which the app keeps
//!   unencrypted so it can seal without the master key.
//! - The [`RecoveryKey`] is the user-held root, written down or stored in a
//!   password manager.
//!
//! # Primitives
//!
//! - XChaCha20-Poly1305 for record encryption and key wrapping (random
//!   192-bit nonces), and in the STREAM construction for files.
//! - HPKE (RFC 9180, base mode: DHKEM(X25519, HKDF-SHA-256), HKDF-SHA-256,
//!   ChaCha20-Poly1305) for notes captured while the journal is locked,
//!   sealed to the capture key pair's public key.
//! - Argon2id for passphrase-derived keys.
//! - HKDF-SHA-256 for subkeys.
//! - The operating system's CSPRNG (`getrandom`) for keys, nonces and IDs.
//!
//! All key types zeroize on drop, never implement `Clone`, and print as
//! `<redacted>` in `Debug` output.

#![forbid(unsafe_code)]

mod aead;
mod capture;
mod error;
mod id;
mod kdf;
mod keys;
mod random;
mod recovery;
mod stream;

pub use aead::{SEAL_OVERHEAD, WRAPPED_KEY_LEN, WrappedKey};
pub use capture::{CAPTURE_PUBLIC_KEY_LEN, CAPTURE_SEAL_OVERHEAD, CapturePublicKey};
pub use error::{Error, RecoveryKeyError, Result};
pub use id::RecordId;
pub use kdf::{PassphraseParams, SALT_LEN};
pub use keys::{CaptureKey, DatabaseKey, KEY_LEN, KeyEncryptionKey, MasterKey, RecordKey, SubKey};
pub use random::fill_random;
pub use recovery::RecoveryKey;
pub use stream::{CHUNK_LEN, STREAM_HEADER_LEN, decrypt_stream, encrypt_stream};

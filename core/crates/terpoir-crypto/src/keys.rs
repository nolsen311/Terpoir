// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

//! Key types. All are 256-bit, heap-allocated, zeroized on drop, not
//! `Clone`, and redacted in `Debug` output.

use core::fmt;
use secrecy::{ExposeSecret, SecretBox};
use zeroize::Zeroizing;

use crate::aead::{self, Domain, WrappedKey};
use crate::kdf::hkdf_sha256;
use crate::{Error, RecordId, Result, fill_random};

/// Length of every key in bytes.
pub const KEY_LEN: usize = 32;

type Secret = SecretBox<[u8; KEY_LEN]>;

fn random_secret() -> Result<Secret> {
    let mut result = Ok(());
    let secret = Secret::init_with_mut(|bytes| result = fill_random(bytes));
    result.map(|()| secret)
}

fn secret_from_slice(bytes: &[u8]) -> Result<Secret> {
    if bytes.len() != KEY_LEN {
        return Err(Error::Format);
    }
    Ok(Secret::init_with_mut(|key| key.copy_from_slice(bytes)))
}

fn secret_from_array(bytes: &[u8; KEY_LEN]) -> Secret {
    Secret::init_with_mut(|key| key.copy_from_slice(bytes))
}

macro_rules! key_type {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        pub struct $name(Secret);

        impl $name {
            #[allow(dead_code, reason = "only some key types are built from derived secrets")]
            pub(crate) fn from_secret(secret: Secret) -> Self {
                Self(secret)
            }

            pub(crate) fn secret(&self) -> &[u8; KEY_LEN] {
                self.0.expose_secret()
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(concat!(stringify!($name), "(<redacted>)"))
            }
        }
    };
}

key_type! {
    /// The journal's master key. It only wraps other keys (record keys and
    /// the database key); it never encrypts content directly.
    ///
    /// On the device it is itself wrapped by a [`KeyEncryptionKey`] from
    /// the recovery key or passphrase, and by the platform keystore.
    MasterKey
}

key_type! {
    /// A per-record data key. Encrypts one record and its attached files.
    RecordKey
}

key_type! {
    /// A key that wraps the [`MasterKey`]: derived from the recovery key or
    /// from a passphrase.
    KeyEncryptionKey
}

key_type! {
    /// The SQLCipher database key, wrapped by the [`MasterKey`].
    DatabaseKey
}

key_type! {
    /// A purpose-specific key derived from another key with HKDF.
    SubKey
}

impl MasterKey {
    /// Generates a new random master key.
    ///
    /// # Errors
    ///
    /// [`Error::Random`] if the operating system's generator fails.
    pub fn generate() -> Result<Self> {
        random_secret().map(Self)
    }

    /// Imports raw key bytes, for example after the platform keystore
    /// unwraps them. The caller should wipe its copy afterwards.
    ///
    /// # Errors
    ///
    /// [`Error::Format`] unless `bytes` is exactly [`KEY_LEN`] long.
    pub fn from_slice(bytes: &[u8]) -> Result<Self> {
        secret_from_slice(bytes).map(Self)
    }

    /// The raw key bytes, for handing to the platform keystore to wrap.
    /// Avoid copying them anywhere else.
    #[must_use]
    pub fn expose_secret(&self) -> &[u8; KEY_LEN] {
        self.secret()
    }

    /// Wraps a record key, binding it to `record`: the wrapped key will
    /// not unwrap for any other record ID.
    ///
    /// # Errors
    ///
    /// [`Error::Random`] or [`Error::Encrypt`].
    pub fn wrap_record_key(&self, key: &RecordKey, record: &RecordId) -> Result<WrappedKey> {
        WrappedKey::wrap(
            self.secret(),
            Domain::WrapRecord,
            record.as_bytes(),
            key.secret(),
        )
    }

    /// Unwraps a record key previously wrapped for `record`.
    ///
    /// # Errors
    ///
    /// [`Error::Decrypt`] for the wrong master key, wrong record, or
    /// tampered input.
    pub fn unwrap_record_key(&self, wrapped: &WrappedKey, record: &RecordId) -> Result<RecordKey> {
        let key = wrapped.unwrap(self.secret(), Domain::WrapRecord, record.as_bytes())?;
        Ok(RecordKey(secret_from_array(&key)))
    }

    /// Re-wraps a record key from this master key to `new_master`, for
    /// master-key rotation. Record ciphertext does not change.
    ///
    /// # Errors
    ///
    /// As [`MasterKey::unwrap_record_key`] and [`MasterKey::wrap_record_key`].
    pub fn rewrap_record_key(
        &self,
        wrapped: &WrappedKey,
        record: &RecordId,
        new_master: &Self,
    ) -> Result<WrappedKey> {
        let key = self.unwrap_record_key(wrapped, record)?;
        new_master.wrap_record_key(&key, record)
    }

    /// Wraps the database key.
    ///
    /// # Errors
    ///
    /// [`Error::Random`] or [`Error::Encrypt`].
    pub fn wrap_database_key(&self, key: &DatabaseKey) -> Result<WrappedKey> {
        WrappedKey::wrap(self.secret(), Domain::WrapDatabase, b"", key.secret())
    }

    /// Unwraps the database key.
    ///
    /// # Errors
    ///
    /// [`Error::Decrypt`] for the wrong master key or tampered input.
    pub fn unwrap_database_key(&self, wrapped: &WrappedKey) -> Result<DatabaseKey> {
        let key = wrapped.unwrap(self.secret(), Domain::WrapDatabase, b"")?;
        Ok(DatabaseKey(secret_from_array(&key)))
    }

    /// Derives a purpose-specific subkey (for example, the sync container
    /// key). `purpose` must be a fixed label, unique per use.
    ///
    /// # Errors
    ///
    /// [`Error::Kdf`] (not expected in practice).
    pub fn derive_subkey(&self, purpose: &str) -> Result<SubKey> {
        hkdf_sha256(self.secret(), &[b"master/", purpose.as_bytes()]).map(SubKey)
    }
}

impl RecordKey {
    /// Generates a new random record key.
    ///
    /// # Errors
    ///
    /// [`Error::Random`] if the operating system's generator fails.
    pub fn generate() -> Result<Self> {
        random_secret().map(Self)
    }

    /// Encrypts a serialized record. `aad` binds the ciphertext to its
    /// context (for example, record ID and type); [`RecordKey::open`] must
    /// pass the same bytes.
    ///
    /// # Errors
    ///
    /// [`Error::Random`] or [`Error::Encrypt`].
    pub fn seal(&self, aad: &[u8], plaintext: &[u8]) -> Result<Vec<u8>> {
        aead::seal(self.secret(), Domain::Record, aad, plaintext)
    }

    /// Decrypts output of [`RecordKey::seal`]. The plaintext is wiped when
    /// dropped.
    ///
    /// # Errors
    ///
    /// [`Error::Decrypt`] for the wrong key, wrong `aad`, or tampered
    /// input; [`Error::Format`] for malformed input.
    pub fn open(&self, aad: &[u8], sealed: &[u8]) -> Result<Zeroizing<Vec<u8>>> {
        aead::open(self.secret(), Domain::Record, aad, sealed)
    }

    /// The key that encrypts the file `file` attached to this record.
    pub(crate) fn file_key(&self, file: &RecordId) -> Result<SubKey> {
        hkdf_sha256(self.secret(), &[b"file/", file.as_bytes()]).map(SubKey)
    }
}

impl KeyEncryptionKey {
    /// Wraps the master key.
    ///
    /// # Errors
    ///
    /// [`Error::Random`] or [`Error::Encrypt`].
    pub fn wrap_master_key(&self, key: &MasterKey) -> Result<WrappedKey> {
        WrappedKey::wrap(self.secret(), Domain::WrapMaster, b"", key.secret())
    }

    /// Unwraps the master key.
    ///
    /// # Errors
    ///
    /// [`Error::Decrypt`] for the wrong recovery key or passphrase, or
    /// tampered input.
    pub fn unwrap_master_key(&self, wrapped: &WrappedKey) -> Result<MasterKey> {
        let key = wrapped.unwrap(self.secret(), Domain::WrapMaster, b"")?;
        Ok(MasterKey(secret_from_array(&key)))
    }
}

impl DatabaseKey {
    /// Generates a new random database key.
    ///
    /// # Errors
    ///
    /// [`Error::Random`] if the operating system's generator fails.
    pub fn generate() -> Result<Self> {
        random_secret().map(Self)
    }

    /// The raw key bytes, for handing to SQLCipher.
    #[must_use]
    pub fn expose_secret(&self) -> &[u8; KEY_LEN] {
        self.secret()
    }
}

impl SubKey {
    /// The raw key bytes.
    #[must_use]
    pub fn expose_secret(&self) -> &[u8; KEY_LEN] {
        self.secret()
    }

    /// Encrypts `plaintext` under this subkey (for example, a sync
    /// container). Domain-separated from record encryption.
    ///
    /// # Errors
    ///
    /// [`Error::Random`] or [`Error::Encrypt`].
    pub fn seal(&self, aad: &[u8], plaintext: &[u8]) -> Result<Vec<u8>> {
        aead::seal(self.secret(), Domain::SubKeySeal, aad, plaintext)
    }

    /// Decrypts output of [`SubKey::seal`]. The plaintext is wiped when
    /// dropped.
    ///
    /// # Errors
    ///
    /// [`Error::Decrypt`] for the wrong key or `aad`, or tampered input.
    pub fn open(&self, aad: &[u8], sealed: &[u8]) -> Result<Zeroizing<Vec<u8>>> {
        aead::open(self.secret(), Domain::SubKeySeal, aad, sealed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id() -> RecordId {
        RecordId::random().unwrap()
    }

    #[test]
    fn record_key_wrap_is_bound_to_record_and_master() {
        let master = MasterKey::generate().unwrap();
        let other_master = MasterKey::generate().unwrap();
        let record_key = RecordKey::generate().unwrap();
        let (a, b) = (id(), id());

        let wrapped = master.wrap_record_key(&record_key, &a).unwrap();
        let unwrapped = master.unwrap_record_key(&wrapped, &a).unwrap();
        assert_eq!(unwrapped.secret(), record_key.secret());

        assert_eq!(
            master.unwrap_record_key(&wrapped, &b).err(),
            Some(Error::Decrypt)
        );
        assert_eq!(
            other_master.unwrap_record_key(&wrapped, &a).err(),
            Some(Error::Decrypt)
        );
    }

    #[test]
    fn rotation_rewraps_without_touching_ciphertext() {
        let old = MasterKey::generate().unwrap();
        let new = MasterKey::generate().unwrap();
        let record = id();
        let key = RecordKey::generate().unwrap();
        let sealed = key.seal(b"ctx", b"journal entry").unwrap();

        let wrapped = old.wrap_record_key(&key, &record).unwrap();
        let rewrapped = old.rewrap_record_key(&wrapped, &record, &new).unwrap();

        assert!(old.unwrap_record_key(&rewrapped, &record).is_err());
        let recovered = new.unwrap_record_key(&rewrapped, &record).unwrap();
        assert_eq!(
            recovered.open(b"ctx", &sealed).unwrap().as_slice(),
            b"journal entry"
        );
    }

    #[test]
    fn database_key_wrap_round_trip() {
        let master = MasterKey::generate().unwrap();
        let db = DatabaseKey::generate().unwrap();
        let wrapped = master.wrap_database_key(&db).unwrap();
        let unwrapped = master.unwrap_database_key(&wrapped).unwrap();
        assert_eq!(unwrapped.expose_secret(), db.expose_secret());
    }

    #[test]
    fn wrapped_keys_are_not_interchangeable_across_purposes() {
        let master = MasterKey::generate().unwrap();
        let record = RecordId::from_bytes([0; 16]);
        let db = DatabaseKey::generate().unwrap();
        let wrapped_db = master.wrap_database_key(&db).unwrap();
        assert_eq!(
            master.unwrap_record_key(&wrapped_db, &record).err(),
            Some(Error::Decrypt)
        );
    }

    #[test]
    fn subkeys_are_deterministic_and_purpose_separated() {
        let master = MasterKey::from_slice(&[1; KEY_LEN]).unwrap();
        let a1 = master.derive_subkey("sync").unwrap();
        let a2 = master.derive_subkey("sync").unwrap();
        let b = master.derive_subkey("names").unwrap();
        assert_eq!(a1.expose_secret(), a2.expose_secret());
        assert_ne!(a1.expose_secret(), b.expose_secret());
        assert_ne!(a1.expose_secret(), master.expose_secret());
    }

    #[test]
    fn subkey_seal_is_separate_from_record_seal() {
        let master = MasterKey::from_slice(&[2; KEY_LEN]).unwrap();
        let sub = master.derive_subkey("sync/container").unwrap();
        let sealed = sub.seal(b"aad", b"container").unwrap();
        assert_eq!(sub.open(b"aad", &sealed).unwrap().as_slice(), b"container");
        assert_eq!(sub.open(b"other", &sealed).err(), Some(Error::Decrypt));
        // Same key bytes used as a record key must not open it.
        let record_key = RecordKey(secret_from_array(sub.expose_secret()));
        assert_eq!(record_key.open(b"aad", &sealed).err(), Some(Error::Decrypt));
    }

    #[test]
    fn from_slice_checks_length() {
        assert_eq!(MasterKey::from_slice(&[0; 31]).err(), Some(Error::Format));
    }

    #[test]
    fn debug_is_redacted() {
        let key = MasterKey::from_slice(&[0x41; KEY_LEN]).unwrap();
        assert_eq!(format!("{key:?}"), "MasterKey(<redacted>)");
        let key = RecordKey::generate().unwrap();
        assert_eq!(format!("{key:?}"), "RecordKey(<redacted>)");
    }
}

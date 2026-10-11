// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

//! Key setup, recovery and passphrase wrapping.

use terpoir_crypto::{DatabaseKey, MasterKey, PassphraseParams, RecoveryKey, WrappedKey};
use zeroize::Zeroizing;

use crate::Result;

/// Everything created when a journal is set up.
#[derive(uniffi::Record)]
pub struct NewKeys {
    /// The recovery key, to show the user once for backup. Never store it.
    pub recovery_key: String,
    /// The raw master key, to wrap with the platform keystore and then
    /// wipe. Never store it unwrapped.
    pub master_key: Vec<u8>,
    /// The master key wrapped by the recovery key. Store it, and include
    /// it with synced data, so a new device can restore from the recovery
    /// key.
    pub master_wrapped_by_recovery: Vec<u8>,
    /// The database key wrapped by the master key. Store it next to the
    /// database.
    pub wrapped_database_key: Vec<u8>,
}

/// Keys after master-key rotation. Replace every stored copy.
#[derive(uniffi::Record)]
pub struct RotatedKeys {
    /// The new raw master key, to wrap with the platform keystore.
    pub master_key: Vec<u8>,
    /// The new master key wrapped by the recovery key.
    pub master_wrapped_by_recovery: Vec<u8>,
    /// The database key wrapped by the new master key.
    pub wrapped_database_key: Vec<u8>,
}

/// The master key wrapped under an app passphrase.
#[derive(uniffi::Record)]
pub struct PassphraseWrapped {
    /// Argon2id parameters and salt (not secret).
    pub params: Vec<u8>,
    /// The wrapped master key.
    pub wrapped_master_key: Vec<u8>,
}

/// Creates the keys for a new journal: a recovery key, a master key and a
/// database key.
///
/// # Errors
///
/// [`crate::CoreError::Internal`] if the system random number generator fails.
#[uniffi::export]
pub fn create_keys() -> Result<NewKeys> {
    let recovery = RecoveryKey::generate()?;
    let master = MasterKey::generate()?;
    let database = DatabaseKey::generate()?;
    Ok(NewKeys {
        recovery_key: recovery.to_display_string().to_string(),
        master_key: master.expose_secret().to_vec(),
        master_wrapped_by_recovery: recovery
            .key_encryption_key()?
            .wrap_master_key(&master)?
            .as_bytes()
            .to_vec(),
        wrapped_database_key: master.wrap_database_key(&database)?.as_bytes().to_vec(),
    })
}

/// Checks a typed recovery key for length, characters and typos, without
/// using it. For live feedback while the user types.
///
/// # Errors
///
/// `RecoveryKeyLength`, `RecoveryKeyCharacter` or `RecoveryKeyChecksum`.
#[uniffi::export]
pub fn check_recovery_key(recovery_key: String) -> Result<()> {
    let text = Zeroizing::new(recovery_key);
    RecoveryKey::parse(&text)?;
    Ok(())
}

/// Recovers the raw master key from the recovery key, on a new device.
///
/// # Errors
///
/// A recovery-key error, or `WrongKey` if it belongs to another journal.
#[uniffi::export]
pub fn master_key_from_recovery(
    recovery_key: String,
    master_wrapped_by_recovery: Vec<u8>,
) -> Result<Vec<u8>> {
    let text = Zeroizing::new(recovery_key);
    let recovery = RecoveryKey::parse(&text)?;
    let wrapped = WrappedKey::from_slice(&master_wrapped_by_recovery)?;
    let master = recovery.key_encryption_key()?.unwrap_master_key(&wrapped)?;
    Ok(master.expose_secret().to_vec())
}

/// Wraps the master key under an app passphrase (Argon2id, recommended
/// parameters). Slow by design: call it off the main thread.
///
/// # Errors
///
/// `InvalidInput` if `master_key` is not a key.
#[uniffi::export]
pub fn wrap_master_key_with_passphrase(
    master_key: Vec<u8>,
    passphrase: String,
) -> Result<PassphraseWrapped> {
    let master_key = Zeroizing::new(master_key);
    let passphrase = Zeroizing::new(passphrase);
    let master = MasterKey::from_slice(&master_key)?;
    let params = PassphraseParams::recommended()?;
    let wrapped = params
        .derive(passphrase.as_bytes())?
        .wrap_master_key(&master)?;
    Ok(PassphraseWrapped {
        params: params.to_bytes(),
        wrapped_master_key: wrapped.as_bytes().to_vec(),
    })
}

/// Unwraps the master key with the app passphrase. Slow by design: call
/// it off the main thread, and apply escalating delays after failures.
///
/// # Errors
///
/// `WrongKey` for a wrong passphrase; `InvalidInput` for malformed input.
#[uniffi::export]
pub fn master_key_from_passphrase(
    passphrase: String,
    wrapped: PassphraseWrapped,
) -> Result<Vec<u8>> {
    let passphrase = Zeroizing::new(passphrase);
    let params = PassphraseParams::from_bytes(&wrapped.params)?;
    let wrapped_key = WrappedKey::from_slice(&wrapped.wrapped_master_key)?;
    let master = params
        .derive(passphrase.as_bytes())?
        .unwrap_master_key(&wrapped_key)?;
    Ok(master.expose_secret().to_vec())
}

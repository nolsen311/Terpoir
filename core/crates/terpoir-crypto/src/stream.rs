// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

//! Streaming encryption for attached files (photos, voice clips).
//!
//! Uses the STREAM construction (`aead-stream`, big-endian 32-bit counter)
//! over XChaCha20-Poly1305, so large files never sit in memory whole and
//! truncation or reordering of chunks is detected.
//!
//! Each file gets its own key, derived with HKDF from the owning record's
//! key and the file's ID; destroying the record key erases its files.
//!
//! Format:
//!
//! ```text
//! header:  version (1) || stream nonce (19)
//! chunks:  ciphertext of up to CHUNK_LEN plaintext bytes || tag (16)
//! ```
//!
//! Every chunk but the last holds exactly [`CHUNK_LEN`] plaintext bytes; the
//! last (possibly empty) carries the STREAM "last block" flag. The header is
//! authenticated as associated data of every chunk.

use aead_stream::{DecryptorBE32, EncryptorBE32};
use chacha20poly1305::aead::Payload;
use chacha20poly1305::{Key, XChaCha20Poly1305};
use std::io::{ErrorKind, Read, Write};
use zeroize::Zeroizing;

use crate::aead::{Domain, VERSION, associated_data};
use crate::{Error, RecordId, RecordKey, Result, fill_random};

/// Plaintext bytes per chunk.
pub const CHUNK_LEN: usize = 64 * 1024;
const TAG_LEN: usize = 16;
const STREAM_NONCE_LEN: usize = 19;

/// Length of the stream header.
pub const STREAM_HEADER_LEN: usize = 1 + STREAM_NONCE_LEN;

/// Encrypts everything from `reader` into `writer`. Returns the number of
/// plaintext bytes encrypted.
///
/// # Errors
///
/// [`Error::Io`] if reading or writing fails; [`Error::Random`] or
/// [`Error::Encrypt`] otherwise.
pub fn encrypt_stream(
    key: &RecordKey,
    file: &RecordId,
    mut reader: impl Read,
    mut writer: impl Write,
) -> Result<u64> {
    let file_key = key.file_key(file)?;
    let mut header = [0u8; STREAM_HEADER_LEN];
    header[0] = VERSION;
    fill_random(&mut header[1..])?;
    let aad = associated_data(Domain::FileStream, &header);

    let nonce = <&[u8; STREAM_NONCE_LEN]>::try_from(&header[1..]).map_err(|_| Error::Format)?;
    let mut encryptor = EncryptorBE32::<XChaCha20Poly1305>::new(
        <&Key>::from(file_key.expose_secret()),
        nonce.into(),
    );
    writer.write_all(&header)?;

    let mut total = 0u64;
    let mut current = Zeroizing::new(vec![0u8; CHUNK_LEN]);
    let mut current_len = read_full(&mut reader, &mut current)?;
    let mut next = Zeroizing::new(vec![0u8; CHUNK_LEN]);
    loop {
        let next_len = if current_len == CHUNK_LEN {
            read_full(&mut reader, &mut next)?
        } else {
            0
        };
        let payload = Payload {
            msg: &current[..current_len],
            aad: &aad,
        };
        total += u64::try_from(current_len).map_err(|_| Error::Encrypt)?;
        if next_len == 0 {
            let chunk = encryptor
                .encrypt_last(payload)
                .map_err(|_| Error::Encrypt)?;
            writer.write_all(&chunk)?;
            break;
        }
        let chunk = encryptor
            .encrypt_next(payload)
            .map_err(|_| Error::Encrypt)?;
        writer.write_all(&chunk)?;
        core::mem::swap(&mut current, &mut next);
        current_len = next_len;
    }
    writer.flush()?;
    Ok(total)
}

/// Decrypts output of [`encrypt_stream`] from `reader` into `writer`.
/// Returns the number of plaintext bytes written.
///
/// Chunks are written as they authenticate. If this returns an error, the
/// output is incomplete or the input was tampered with: the caller must
/// discard everything written.
///
/// # Errors
///
/// [`Error::Format`] for a malformed header or missing data;
/// [`Error::Decrypt`] for the wrong key or file ID, tampering, truncation
/// or reordering; [`Error::Io`] if reading or writing fails.
pub fn decrypt_stream(
    key: &RecordKey,
    file: &RecordId,
    mut reader: impl Read,
    mut writer: impl Write,
) -> Result<u64> {
    let mut header = [0u8; STREAM_HEADER_LEN];
    if read_full(&mut reader, &mut header)? != STREAM_HEADER_LEN || header[0] != VERSION {
        return Err(Error::Format);
    }
    let file_key = key.file_key(file)?;
    let aad = associated_data(Domain::FileStream, &header);
    let nonce = <&[u8; STREAM_NONCE_LEN]>::try_from(&header[1..]).map_err(|_| Error::Format)?;
    let mut decryptor = DecryptorBE32::<XChaCha20Poly1305>::new(
        <&Key>::from(file_key.expose_secret()),
        nonce.into(),
    );

    const SEALED_CHUNK: usize = CHUNK_LEN + TAG_LEN;
    let mut total = 0u64;
    let mut current = vec![0u8; SEALED_CHUNK];
    let mut current_len = read_full(&mut reader, &mut current)?;
    if current_len < TAG_LEN {
        return Err(Error::Format);
    }
    let mut next = vec![0u8; SEALED_CHUNK];
    loop {
        let next_len = if current_len == SEALED_CHUNK {
            read_full(&mut reader, &mut next)?
        } else {
            0
        };
        let payload = Payload {
            msg: &current[..current_len],
            aad: &aad,
        };
        if next_len == 0 {
            let plain = Zeroizing::new(
                decryptor
                    .decrypt_last(payload)
                    .map_err(|_| Error::Decrypt)?,
            );
            writer.write_all(&plain)?;
            total += u64::try_from(plain.len()).map_err(|_| Error::Decrypt)?;
            break;
        }
        if next_len < TAG_LEN {
            return Err(Error::Decrypt);
        }
        let plain = Zeroizing::new(
            decryptor
                .decrypt_next(payload)
                .map_err(|_| Error::Decrypt)?,
        );
        writer.write_all(&plain)?;
        total += u64::try_from(plain.len()).map_err(|_| Error::Decrypt)?;
        core::mem::swap(&mut current, &mut next);
        current_len = next_len;
    }
    writer.flush()?;
    Ok(total)
}

/// Reads until `buf` is full or the reader is exhausted.
fn read_full(reader: &mut impl Read, buf: &mut [u8]) -> Result<usize> {
    let mut filled = 0;
    while filled < buf.len() {
        match reader.read(&mut buf[filled..]) {
            Ok(0) => break,
            Ok(n) => filled += n,
            Err(e) if e.kind() == ErrorKind::Interrupted => {}
            Err(e) => return Err(e.into()),
        }
    }
    Ok(filled)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trip(len: usize) {
        let key = RecordKey::generate().unwrap();
        let file = RecordId::random().unwrap();
        let plain: Vec<u8> = (0..len).map(|i| (i % 251) as u8).collect();
        let mut sealed = Vec::new();
        assert_eq!(
            encrypt_stream(&key, &file, plain.as_slice(), &mut sealed).unwrap(),
            len as u64
        );
        let chunks = len.div_ceil(CHUNK_LEN).max(1);
        assert_eq!(sealed.len(), STREAM_HEADER_LEN + len + chunks * TAG_LEN);

        let mut out = Vec::new();
        assert_eq!(
            decrypt_stream(&key, &file, sealed.as_slice(), &mut out).unwrap(),
            len as u64
        );
        assert_eq!(out, plain);
    }

    #[test]
    fn round_trips_at_chunk_boundaries() {
        for len in [
            0,
            1,
            CHUNK_LEN - 1,
            CHUNK_LEN,
            CHUNK_LEN + 1,
            2 * CHUNK_LEN,
            3 * CHUNK_LEN + 7,
        ] {
            round_trip(len);
        }
    }

    fn sealed(len: usize) -> (RecordKey, RecordId, Vec<u8>) {
        let key = RecordKey::generate().unwrap();
        let file = RecordId::random().unwrap();
        let mut out = Vec::new();
        encrypt_stream(&key, &file, vec![5u8; len].as_slice(), &mut out).unwrap();
        (key, file, out)
    }

    fn decrypt(key: &RecordKey, file: &RecordId, data: &[u8]) -> Result<u64> {
        decrypt_stream(key, file, data, Vec::new())
    }

    #[test]
    fn wrong_key_or_file_fails() {
        let (key, file, data) = sealed(100);
        let other_key = RecordKey::generate().unwrap();
        let other_file = RecordId::random().unwrap();
        assert_eq!(decrypt(&other_key, &file, &data), Err(Error::Decrypt));
        assert_eq!(decrypt(&key, &other_file, &data), Err(Error::Decrypt));
    }

    #[test]
    fn truncation_is_detected() {
        let (key, file, data) = sealed(2 * CHUNK_LEN + 10);
        // Drop the final chunk: the remaining last chunk lacks the flag.
        let cut = STREAM_HEADER_LEN + 2 * (CHUNK_LEN + TAG_LEN);
        assert_eq!(decrypt(&key, &file, &data[..cut]), Err(Error::Decrypt));
        assert_eq!(
            decrypt(&key, &file, &data[..data.len() - 1]),
            Err(Error::Decrypt)
        );
        assert_eq!(
            decrypt(&key, &file, &data[..STREAM_HEADER_LEN]),
            Err(Error::Format)
        );
        assert_eq!(decrypt(&key, &file, &data[..5]), Err(Error::Format));
    }

    #[test]
    fn reordering_and_tampering_are_detected() {
        let (key, file, data) = sealed(2 * CHUNK_LEN + 10);
        let c = CHUNK_LEN + TAG_LEN;
        let h = STREAM_HEADER_LEN;
        let mut swapped = data[..h].to_vec();
        swapped.extend_from_slice(&data[h + c..h + 2 * c]);
        swapped.extend_from_slice(&data[h..h + c]);
        swapped.extend_from_slice(&data[h + 2 * c..]);
        assert_eq!(decrypt(&key, &file, &swapped), Err(Error::Decrypt));

        for at in [1, h, h + c + 3, data.len() - 1] {
            let mut tampered = data.clone();
            tampered[at] ^= 0x80;
            assert_eq!(decrypt(&key, &file, &tampered), Err(Error::Decrypt));
        }
    }

    #[test]
    fn same_file_encrypts_differently_each_time() {
        let key = RecordKey::generate().unwrap();
        let file = RecordId::random().unwrap();
        let (mut a, mut b) = (Vec::new(), Vec::new());
        encrypt_stream(&key, &file, &b"photo"[..], &mut a).unwrap();
        encrypt_stream(&key, &file, &b"photo"[..], &mut b).unwrap();
        assert_ne!(a, b);
    }
}

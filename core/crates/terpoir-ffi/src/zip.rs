// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

//! A ZIP of the plaintext export's files, so both apps save the same single
//! file (maintainer, 2026-11-15). Entries are stored, not compressed: the
//! export is small text, and storing needs no new dependency. Only CRC-32
//! is computed here; it's a checksum the format requires, not cryptography.

use crate::error::{CoreError, Result};
use crate::plaintext::ExportFile;

const LOCAL_HEADER: u32 = 0x0403_4b50;
const CENTRAL_HEADER: u32 = 0x0201_4b50;
const END_OF_CENTRAL: u32 = 0x0605_4b50;
/// ZIP 2.0: the oldest version that every unzip tool reads.
const VERSION: u16 = 20;
/// File names are UTF-8.
const FLAG_UTF8: u16 = 1 << 11;
/// No compression.
const STORED: u16 = 0;

/// The files as one ZIP, each dated `now_ms` (UTC).
///
/// # Errors
///
/// `Internal` if the export is too large for a ZIP without its 64-bit
/// extensions (4 GiB, or 65,535 files), which a journal never is.
pub(crate) fn zip(files: &[ExportFile], now_ms: u64) -> Result<Vec<u8>> {
    let (time, date) = dos_time(now_ms);
    let mut out = Vec::new();
    let mut central = Vec::new();
    for file in files {
        let name = file.name.as_bytes();
        let name_len = u16::try_from(name.len()).map_err(|_| CoreError::Internal)?;
        let size = u32::try_from(file.bytes.len()).map_err(|_| CoreError::Internal)?;
        let offset = u32::try_from(out.len()).map_err(|_| CoreError::Internal)?;
        let crc = crc32(&file.bytes);

        put32(&mut out, LOCAL_HEADER);
        put16(&mut out, VERSION);
        put16(&mut out, FLAG_UTF8);
        put16(&mut out, STORED);
        put16(&mut out, time);
        put16(&mut out, date);
        put32(&mut out, crc);
        put32(&mut out, size);
        put32(&mut out, size);
        put16(&mut out, name_len);
        put16(&mut out, 0);
        out.extend_from_slice(name);
        out.extend_from_slice(&file.bytes);

        put32(&mut central, CENTRAL_HEADER);
        put16(&mut central, VERSION);
        put16(&mut central, VERSION);
        put16(&mut central, FLAG_UTF8);
        put16(&mut central, STORED);
        put16(&mut central, time);
        put16(&mut central, date);
        put32(&mut central, crc);
        put32(&mut central, size);
        put32(&mut central, size);
        put16(&mut central, name_len);
        put16(&mut central, 0); // extra field
        put16(&mut central, 0); // comment
        put16(&mut central, 0); // disk
        put16(&mut central, 0); // internal attributes
        put32(&mut central, 0); // external attributes
        put32(&mut central, offset);
        central.extend_from_slice(name);
    }
    let count = u16::try_from(files.len()).map_err(|_| CoreError::Internal)?;
    let central_size = u32::try_from(central.len()).map_err(|_| CoreError::Internal)?;
    let central_offset = u32::try_from(out.len()).map_err(|_| CoreError::Internal)?;
    out.extend_from_slice(&central);
    put32(&mut out, END_OF_CENTRAL);
    put16(&mut out, 0); // this disk
    put16(&mut out, 0); // disk with the central directory
    put16(&mut out, count);
    put16(&mut out, count);
    put32(&mut out, central_size);
    put32(&mut out, central_offset);
    put16(&mut out, 0); // comment
    Ok(out)
}

fn put16(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn put32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

/// MS-DOS time and date fields for `unix_ms` (UTC). The format starts in
/// 1980 and counts seconds in twos.
fn dos_time(unix_ms: u64) -> (u16, u16) {
    let secs = unix_ms / 1000;
    let days = i64::try_from(secs / 86_400).unwrap_or(i64::MAX / 2);
    let (y, m, d) = crate::plaintext::civil_from_days(days);
    if !(1980..=2107).contains(&y) {
        // 1980-01-01 00:00, the earliest the format has.
        return (0, (1 << 5) | 1);
    }
    let in_day = secs % 86_400;
    let (h, min, s) = (in_day / 3600, in_day % 3600 / 60, in_day % 60);
    // Each value fits its field: hours < 24, minutes and seconds < 60,
    // years since 1980 < 128, months 1-12, days 1-31.
    let time = u16::try_from((h << 11) | (min << 5) | (s / 2)).unwrap_or(0);
    let date = u16::try_from(((y - 1980) << 9) | (m << 5) | d).unwrap_or(0);
    (time, date)
}

/// CRC-32 (IEEE 802.3, as ZIP uses), from a table built at compile time.
fn crc32(bytes: &[u8]) -> u32 {
    const TABLE: [u32; 256] = {
        let mut table = [0u32; 256];
        let mut i = 0;
        while i < 256 {
            #[allow(clippy::cast_possible_truncation)]
            let mut c = i as u32;
            let mut k = 0;
            while k < 8 {
                c = if c & 1 == 1 {
                    0xEDB8_8320 ^ (c >> 1)
                } else {
                    c >> 1
                };
                k += 1;
            }
            table[i] = c;
            i += 1;
        }
        table
    };
    let mut crc = 0xFFFF_FFFFu32;
    for &b in bytes {
        crc = TABLE[usize::from((crc as u8) ^ b)] ^ (crc >> 8);
    }
    !crc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc32_matches_the_standard_check_value() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
        assert_eq!(crc32(b""), 0);
    }

    #[test]
    fn dos_times_count_from_1980() {
        // 2026-10-11 20:40:30 UTC.
        let (time, date) = dos_time(1_791_751_230_000);
        assert_eq!(time, (20 << 11) | (40 << 5) | 15);
        assert_eq!(date, ((2026 - 1980) << 9) | (10 << 5) | 11);
        // Before 1980: the format's first day.
        assert_eq!(dos_time(0), (0, (1 << 5) | 1));
    }

    #[test]
    fn the_zip_lists_every_file_stored() {
        let files = vec![
            ExportFile {
                name: "a.csv".into(),
                bytes: b"x,y\n1,2\n".to_vec(),
            },
            ExportFile {
                name: "terpoir.json".into(),
                bytes: b"{}".to_vec(),
            },
        ];
        let zip = zip(&files, 0).unwrap();
        // The local header of the first file, then its name and bytes.
        assert_eq!(&zip[..4], &LOCAL_HEADER.to_le_bytes());
        assert_eq!(&zip[30..35], b"a.csv");
        assert_eq!(&zip[35..43], b"x,y\n1,2\n");
        // The end record: two entries, and the central directory where it
        // says it is.
        let end = zip.len() - 22;
        assert_eq!(&zip[end..end + 4], &END_OF_CENTRAL.to_le_bytes());
        assert_eq!(u16::from_le_bytes([zip[end + 10], zip[end + 11]]), 2);
        let offset = u32::from_le_bytes(zip[end + 16..end + 20].try_into().unwrap()) as usize;
        assert_eq!(&zip[offset..offset + 4], &CENTRAL_HEADER.to_le_bytes());
    }
}

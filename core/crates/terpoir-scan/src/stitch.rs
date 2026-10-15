// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

//! A long receipt in two photos, joined where their rows overlap.
//!
//! The user photographs the top of the receipt, then the rest, keeping a
//! few printed rows in both photos. [`join_receipt_photos`] finds those
//! rows: the end of the first photo's rows matching the start of the
//! second's, allowing for a row or two cut by either photo's edge and for
//! the recognizer reading the same row slightly differently twice. The
//! second photo's rows below the overlap are then moved into the first
//! photo's coordinates (scaled by the rows' heights, since the two photos
//! are rarely taken from the same distance), so the layout rules that join
//! wrapped item names work across the seam.

use crate::layout::TextRow;

/// Rows of the first photo that may follow the overlap, cut by its bottom
/// edge.
const FIRST_TAIL_SLACK: usize = 2;
/// Rows of the second photo that may come before the overlap, cut by its
/// top edge or above the part both photos show.
const SECOND_HEAD_SLACK: usize = 3;
/// A single overlapping row is trusted only when it is at least this long
/// (letters and digits), so a lone "1" or "TAX" can't join two photos.
const LONE_ROW_MIN: usize = 10;
/// Rows with fewer letters and digits than this can't anchor an overlap.
const SHORT_ROW: usize = 3;

/// Two photos' rows, joined. No `Debug`: the rows are receipt text.
#[derive(Clone, PartialEq, Eq)]
pub struct JoinedRows {
    /// The first photo's rows up to the end of the overlap, then the
    /// second photo's rows below it, in the first photo's coordinates. The
    /// second photo's rows carry no `pieces` (their indices would point
    /// into the other photo's pieces).
    pub rows: Vec<TextRow>,
    /// How many rows both photos show.
    pub overlap: usize,
    /// How many of `rows` came from the first photo.
    pub from_first: usize,
}

/// Joins a long receipt's two photos where their rows overlap, or `None`
/// when no overlap is found (the user can retake the second photo).
#[must_use]
pub fn join_receipt_photos(first: &[TextRow], second: &[TextRow]) -> Option<JoinedRows> {
    let a: Vec<String> = first.iter().map(|r| key(&r.text)).collect();
    let b: Vec<String> = second.iter().map(|r| key(&r.text)).collect();

    // (rows, characters) of the best overlap, and where it starts.
    let mut best: Option<((usize, usize), usize, usize)> = None;
    for j in 0..b.len().min(SECOND_HEAD_SLACK + 1) {
        for i in 0..a.len() {
            let mut rows = 0;
            let mut chars = 0;
            while i + rows < a.len() && j + rows < b.len() && same_row(&a[i + rows], &b[j + rows]) {
                chars += a[i + rows].len();
                rows += 1;
            }
            // The overlap has to reach the end of the first photo, but for
            // rows its bottom edge cut.
            if rows == 0 || i + rows + FIRST_TAIL_SLACK < a.len() {
                continue;
            }
            if rows == 1 && chars < LONE_ROW_MIN {
                continue;
            }
            if best.is_none_or(|(score, ..)| (rows, chars) > score) {
                best = Some(((rows, chars), i, j));
            }
        }
    }
    let ((overlap, _), i, j) = best?;

    let scale = scale(&first[i..i + overlap], &second[j..j + overlap]);
    let (anchor_a, anchor_b) = (&first[i], &second[j]);
    let place = |v: i32, from: i32, to: i32| -> i32 {
        // Both are pixel coordinates within a photo, far from overflowing.
        #[allow(clippy::cast_possible_truncation)]
        let moved = (f64::from(v - from) * scale).round() as i32;
        to + moved
    };
    let from_first = i + overlap;
    let mut rows: Vec<TextRow> = first[..from_first].to_vec();
    rows.extend(second[j + overlap..].iter().map(|r| TextRow {
        text: r.text.clone(),
        left: place(r.left, anchor_b.left, anchor_a.left),
        top: place(r.top, anchor_b.top, anchor_a.top),
        right: place(r.right, anchor_b.left, anchor_a.left),
        bottom: place(r.bottom, anchor_b.top, anchor_a.top),
        pieces: Vec::new(),
    }));
    Some(JoinedRows {
        rows,
        overlap,
        from_first,
    })
}

/// A row's letters and digits, upper case: what two readings of the same
/// printed row share.
fn key(text: &str) -> String {
    text.chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|c| c.to_ascii_uppercase())
        .collect()
}

/// Whether two keys read the same printed row: equal, or a few characters
/// apart (about one in six), as two readings of one row often are.
fn same_row(a: &str, b: &str) -> bool {
    if a.len() < SHORT_ROW || b.len() < SHORT_ROW {
        return false;
    }
    if a == b {
        return true;
    }
    let allowed = (a.len().max(b.len()) / 6).max(1);
    a.len().abs_diff(b.len()) <= allowed && edit_distance(a.as_bytes(), b.as_bytes()) <= allowed
}

fn edit_distance(a: &[u8], b: &[u8]) -> usize {
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut cur = vec![0; b.len() + 1];
    for (x, &ca) in a.iter().enumerate() {
        cur[0] = x + 1;
        for (y, &cb) in b.iter().enumerate() {
            let swap = prev[y] + usize::from(ca != cb);
            cur[y + 1] = swap.min(prev[y + 1] + 1).min(cur[y] + 1);
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[b.len()]
}

/// How much larger the first photo shows the receipt than the second: from
/// the distance between the first and last overlapping rows when there are
/// two or more, else the overlapping row's height; 1 when unclear.
fn scale(first: &[TextRow], second: &[TextRow]) -> f64 {
    if let (Some(a0), Some(a1), Some(b0), Some(b1)) =
        (first.first(), first.last(), second.first(), second.last())
    {
        let (da, db) = (a1.top - a0.top, b1.top - b0.top);
        if da > 0 && db > 0 {
            return sane(f64::from(da) / f64::from(db));
        }
    }
    let mut ratios: Vec<f64> = first
        .iter()
        .zip(second)
        .filter_map(|(a, b)| {
            let (ha, hb) = (a.bottom - a.top, b.bottom - b.top);
            (ha > 0 && hb > 0).then(|| f64::from(ha) / f64::from(hb))
        })
        .collect();
    if ratios.is_empty() {
        return 1.0;
    }
    ratios.sort_by(f64::total_cmp);
    sane(ratios[ratios.len() / 2])
}

/// A scale between two photos of one receipt, or 1 when it is implausible.
fn sane(scale: f64) -> f64 {
    if scale.is_finite() && (0.2..=5.0).contains(&scale) {
        scale
    } else {
        1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(text: &str, top: i32, height: i32) -> TextRow {
        TextRow {
            text: text.into(),
            left: 10,
            top,
            right: 300,
            bottom: top + height,
            pieces: vec![0],
        }
    }

    fn rows(texts: &[&str], top: i32, step: i32) -> Vec<TextRow> {
        texts
            .iter()
            .enumerate()
            .map(|(n, t)| row(t, top + step * i32::try_from(n).unwrap(), step - 4))
            .collect()
    }

    fn texts(joined: &JoinedRows) -> Vec<&str> {
        joined.rows.iter().map(|r| r.text.as_str()).collect()
    }

    #[test]
    fn joins_on_the_rows_both_photos_show() {
        let first = rows(
            &[
                "GREEN LEAF",
                "BLUE DREAM 3.5G 35.00",
                "ACME GUMMIES 10PK 20.00",
                "FIRE OG PR 1G 12.00",
            ],
            100,
            40,
        );
        // Taken closer: rows half again as tall, further apart.
        let second = rows(
            &[
                "ACME GUMMIES 10PK 20.00",
                "FIRE OG PR 1G 12.00",
                "SUBTOTAL 67.00",
                "TOTAL 72.36",
            ],
            50,
            60,
        );
        let joined = join_receipt_photos(&first, &second).unwrap();
        assert_eq!(joined.overlap, 2);
        assert_eq!(joined.from_first, 4);
        assert_eq!(
            texts(&joined),
            [
                "GREEN LEAF",
                "BLUE DREAM 3.5G 35.00",
                "ACME GUMMIES 10PK 20.00",
                "FIRE OG PR 1G 12.00",
                "SUBTOTAL 67.00",
                "TOTAL 72.36",
            ]
        );
        // SUBTOTAL sits one first-photo row below FIRE OG PR (40 px), not 60.
        let fire = &joined.rows[3];
        let subtotal = &joined.rows[4];
        assert!(
            (subtotal.top - fire.top - 40).abs() <= 1,
            "{} {}",
            fire.top,
            subtotal.top
        );
        assert!(subtotal.pieces.is_empty());
        assert_eq!(fire.pieces, [0]);
    }

    #[test]
    fn tolerates_two_readings_and_cut_rows() {
        let first = rows(
            &[
                "GREEN LEAF",
                "BLUE DREAM 3.5G 35.00",
                "ACME GUMMIES 10PK 20.00",
                "FIRE OG PR 1G 12.00",
                "SUBTOT",
            ],
            100,
            40,
        );
        let second = rows(
            &[
                "ME GUM",
                "ACME GUMMIES 1OPK 20.00",
                "FIRE 0G PR 1G 12.00",
                "SUBTOTAL 67.00",
                "TOTAL 72.36",
            ],
            0,
            40,
        );
        let joined = join_receipt_photos(&first, &second).unwrap();
        assert_eq!(joined.overlap, 2);
        assert_eq!(
            texts(&joined),
            [
                "GREEN LEAF",
                "BLUE DREAM 3.5G 35.00",
                "ACME GUMMIES 10PK 20.00",
                "FIRE OG PR 1G 12.00",
                "SUBTOTAL 67.00",
                "TOTAL 72.36",
            ]
        );
    }

    #[test]
    fn no_overlap_is_none() {
        let first = rows(&["GREEN LEAF", "BLUE DREAM 3.5G 35.00"], 0, 40);
        let second = rows(&["SUBTOTAL 67.00", "TOTAL 72.36"], 0, 40);
        assert!(join_receipt_photos(&first, &second).is_none());
    }

    #[test]
    fn a_short_lone_row_does_not_join() {
        let first = rows(&["BLUE DREAM 3.5G 35.00", "TAX"], 0, 40);
        let second = rows(&["TAX", "TOTAL 72.36"], 0, 40);
        assert!(join_receipt_photos(&first, &second).is_none());
        // A long one does.
        let first = rows(&["GREEN LEAF", "BLUE DREAM 3.5G 35.00"], 0, 40);
        let second = rows(&["BLUE DREAM 3.5G 35.00", "TOTAL 72.36"], 0, 40);
        assert_eq!(join_receipt_photos(&first, &second).unwrap().overlap, 1);
    }

    #[test]
    fn the_overlap_must_reach_the_first_photos_end() {
        // The same item early in the first photo isn't the overlap.
        let first = rows(
            &[
                "ACME GUMMIES 10PK 20.00",
                "BLUE DREAM 3.5G 35.00",
                "GREEN CRACK 1G 12.00",
                "OG KUSH 1G 12.00",
                "SOUR D 1G 12.00",
            ],
            0,
            40,
        );
        let second = rows(&["ACME GUMMIES 10PK 20.00", "TOTAL 72.36"], 0, 40);
        assert!(join_receipt_photos(&first, &second).is_none());
    }

    #[test]
    fn edit_distance_counts_changes() {
        assert_eq!(edit_distance(b"FIREOG", b"FIRE0G"), 1);
        assert_eq!(edit_distance(b"ABC", b"ABC"), 0);
        assert_eq!(edit_distance(b"", b"AB"), 2);
    }
}

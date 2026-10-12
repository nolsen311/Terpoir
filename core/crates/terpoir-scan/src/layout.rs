// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

//! Printed rows from recognized text pieces.
//!
//! Text recognizers report a photo as blocks of lines, and on a receipt the
//! item names and their prices usually land in different blocks: read in
//! order, every name comes first and every price after. Grouping the pieces
//! by where they sit on the photo puts each name back beside its price.

/// A piece of recognized text and its box on the photo, in pixels (`y`
/// grows downwards).
#[derive(Clone, PartialEq, Eq)]
pub struct TextPiece {
    /// The recognized text.
    pub text: String,
    /// Left edge.
    pub left: i32,
    /// Top edge.
    pub top: i32,
    /// Right edge.
    pub right: i32,
    /// Bottom edge.
    pub bottom: i32,
}

impl TextPiece {
    fn height(&self) -> i32 {
        (self.bottom - self.top).max(1)
    }

    fn width(&self) -> i32 {
        (self.right - self.left).max(1)
    }

    /// Twice the vertical centre, to stay in integers.
    fn centre2(&self) -> i32 {
        self.top + self.bottom
    }
}

/// One printed row: pieces side by side, left to right.
#[derive(Clone, PartialEq, Eq)]
pub struct TextRow {
    /// The pieces' text, left to right, separated by spaces.
    pub text: String,
    /// The box around the row's pieces.
    pub left: i32,
    /// Top edge.
    pub top: i32,
    /// Right edge.
    pub right: i32,
    /// Bottom edge.
    pub bottom: i32,
    /// Indices of the pieces in the input, left to right.
    pub pieces: Vec<usize>,
}

/// How much of the shorter piece's height two pieces must share to sit on
/// one row, in percent.
const SAME_ROW: i32 = 50;

/// How much of the narrower piece's width two pieces may share and still sit
/// side by side, in percent; more means one is above the other.
const SIDE_BY_SIDE: i32 = 30;

fn overlap(a_start: i32, a_end: i32, b_start: i32, b_end: i32) -> i32 {
    (a_end.min(b_end) - a_start.max(b_start)).max(0)
}

/// Whether `a` and `b` overlap so little horizontally that they sit side
/// by side rather than one above the other.
fn beside(a: &TextPiece, b: &TextPiece) -> bool {
    overlap(a.left, a.right, b.left, b.right) * 100 <= SIDE_BY_SIDE * a.width().min(b.width())
}

/// How much height `piece` shares with `member`, in percent of the shorter
/// one, or `None` if too little for one row.
fn fit(piece: &TextPiece, member: &TextPiece) -> Option<i32> {
    let shared = overlap(piece.top, piece.bottom, member.top, member.bottom);
    let percent = shared * 100 / piece.height().min(member.height());
    (percent >= SAME_ROW).then_some(percent)
}

/// Groups `pieces` into printed rows, top to bottom. Pieces with no text
/// are dropped.
#[must_use]
pub fn group_rows(pieces: &[TextPiece]) -> Vec<TextRow> {
    let mut order: Vec<usize> = (0..pieces.len())
        .filter(|&i| !pieces[i].text.trim().is_empty())
        .collect();
    order.sort_by_key(|&i| (pieces[i].centre2(), pieces[i].left));

    let mut rows: Vec<Vec<usize>> = Vec::new();
    for i in order {
        let piece = &pieces[i];
        // The row this piece fits best, if it fits beside every member.
        let best = rows
            .iter()
            .enumerate()
            .filter(|(_, row)| row.iter().all(|&m| beside(piece, &pieces[m])))
            .filter_map(|(r, row)| {
                row.iter()
                    .filter_map(|&m| fit(piece, &pieces[m]))
                    .max()
                    .map(|score| (score, r))
            })
            .max_by_key(|&(score, r)| (score, std::cmp::Reverse(r)));
        match best {
            Some((_, r)) => rows[r].push(i),
            None => rows.push(vec![i]),
        }
    }

    let mut rows: Vec<TextRow> = rows
        .into_iter()
        .map(|mut row| {
            row.sort_by_key(|&i| pieces[i].left);
            let text = row
                .iter()
                .map(|&i| pieces[i].text.trim())
                .collect::<Vec<_>>()
                .join(" ");
            TextRow {
                text,
                left: row.iter().map(|&i| pieces[i].left).min().unwrap_or(0),
                top: row.iter().map(|&i| pieces[i].top).min().unwrap_or(0),
                right: row.iter().map(|&i| pieces[i].right).max().unwrap_or(0),
                bottom: row.iter().map(|&i| pieces[i].bottom).max().unwrap_or(0),
                pieces: row,
            }
        })
        .collect();
    rows.sort_by_key(|row| (row.top + row.bottom, row.left));
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    fn piece(text: &str, left: i32, top: i32, right: i32, bottom: i32) -> TextPiece {
        TextPiece {
            text: text.to_owned(),
            left,
            top,
            right,
            bottom,
        }
    }

    fn texts(rows: &[TextRow]) -> Vec<&str> {
        rows.iter().map(|r| r.text.as_str()).collect()
    }

    #[test]
    fn puts_prices_back_beside_their_names() {
        // As a recognizer reports a receipt: the names block, then the
        // prices block.
        let pieces = [
            piece("GREEN LEAF", 100, 0, 500, 40),
            piece("Blue Dream 3.5g", 20, 100, 400, 130),
            piece("Mango Gummies", 20, 140, 380, 170),
            piece("Total", 20, 200, 140, 230),
            piece("35.00", 600, 102, 700, 131),
            piece("36.00", 602, 143, 700, 172),
            piece("71.00", 600, 198, 700, 228),
        ];
        let rows = group_rows(&pieces);
        assert_eq!(
            texts(&rows),
            [
                "GREEN LEAF",
                "Blue Dream 3.5g 35.00",
                "Mango Gummies 36.00",
                "Total 71.00",
            ]
        );
        assert_eq!(rows[1].pieces, [1, 4]);
        assert_eq!(
            (rows[1].left, rows[1].top, rows[1].right, rows[1].bottom),
            (20, 100, 700, 131)
        );
    }

    #[test]
    fn keeps_stacked_lines_apart() {
        // Tight line spacing: neighbours overlap a little, but less than
        // half a line.
        let pieces = [
            piece("Live Resin Cart", 20, 100, 400, 130),
            piece("45.00", 600, 120, 700, 150),
            piece("Pre-roll", 20, 125, 300, 155),
        ];
        let rows = group_rows(&pieces);
        assert_eq!(texts(&rows), ["Live Resin Cart", "Pre-roll 45.00"]);
    }

    #[test]
    fn a_piece_above_another_is_not_beside_it() {
        let pieces = [
            piece("Blue Dream", 20, 100, 400, 130),
            piece("Indica", 30, 110, 380, 140),
        ];
        assert_eq!(group_rows(&pieces).len(), 2);
    }

    #[test]
    fn drops_empty_pieces() {
        let pieces = [piece("  ", 0, 0, 10, 10), piece("Shop", 0, 20, 50, 30)];
        let rows = group_rows(&pieces);
        assert_eq!(texts(&rows), ["Shop"]);
        assert_eq!(rows[0].pieces, [1]);
        assert!(group_rows(&[]).is_empty());
    }
}

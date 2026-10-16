// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

//! A pack with an amount (`5PK 0.5G`): is the amount each piece's or the
//! whole pack's? Either happens, by area and product (maintainer,
//! 2026-10-15), so [`amount_per`] reads it as each piece's unless the
//! amount times the pack comes out implausibly large for the product: then
//! the amount is the pack's total. The user can switch it on the card.
//!
//! The most a package plausibly holds in all, by form and unit (more than
//! this, read as each piece's, makes the amount the pack's total):
//!
//! | Form | g | mg | ml |
//! |---|---|---|---|
//! | Flower, pre-roll | 28 g (an ounce) | 1000 mg | 1000 ml |
//! | Concentrate, vape | 10 g | 1000 mg | 1000 ml |
//! | Edible | 500 g | 1000 mg (of THC) | 1000 ml |
//! | Beverage | 500 g | 1000 mg | 6000 ml (a 12-pack of cans) |
//! | Tincture | 500 g | 6000 mg | 300 ml |
//! | Topical | 500 g | 6000 mg | 1000 ml |
//! | No form, or one from a newer app | 28 g | 1000 mg | 6000 ml |
//!
//! A unit from a newer app version decides nothing (`None`).

use terpoir_model::{AmountPer, Form, PackageSize, SizeUnit};

/// Thousandths of a unit in one unit.
const MILLI: i64 = 1_000;

/// The most a package of `form` plausibly holds in all, in thousandths of
/// `unit`; `None` for a unit this version doesn't know.
fn limit_milli(form: Option<&Form>, unit: &SizeUnit) -> Option<i64> {
    let whole = match (form, unit) {
        (Some(Form::Flower | Form::PreRoll), SizeUnit::Gram) => 28,
        (Some(Form::Concentrate | Form::VapeCart), SizeUnit::Gram) => 10,
        (Some(Form::Edible | Form::Beverage | Form::Tincture | Form::Topical), SizeUnit::Gram) => {
            500
        }
        (Some(Form::Tincture | Form::Topical), SizeUnit::Milligram) => 6_000,
        (Some(Form::Beverage) | None | Some(Form::Unknown(_)), SizeUnit::Millilitre) => 6_000,
        (Some(Form::Tincture), SizeUnit::Millilitre) => 300,
        (_, SizeUnit::Gram) => 28,
        (_, SizeUnit::Milligram) => 1_000,
        (_, SizeUnit::Millilitre) => 1_000,
        _ => return None,
    };
    Some(whole * MILLI)
}

/// Whether a package's amount is each piece's or the pack's total, for a
/// product of `form` (the line's, else the linked product's): each piece's,
/// unless the amount times the pack is more than such a package plausibly
/// holds (see the module's table), when it is the pack's total. `None`
/// when the size doesn't have both an amount and a pack, or its unit is
/// from a newer app version.
#[must_use]
pub fn amount_per(size: &PackageSize, form: Option<&Form>) -> Option<AmountPer> {
    let amount = size.amount.as_ref()?;
    let pack = size.pack?;
    let limit = limit_milli(form, &amount.unit)?;
    let total = amount.milli.saturating_mul(i64::from(pack));
    Some(if total > limit {
        AmountPer::Total
    } else {
        AmountPer::Each
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use terpoir_model::SizeAmount;

    fn size(milli: i64, unit: SizeUnit, pack: u32) -> PackageSize {
        PackageSize {
            amount: Some(SizeAmount { milli, unit }),
            pack: Some(pack),
            per: None,
        }
    }

    fn per(milli: i64, unit: SizeUnit, pack: u32, form: Option<Form>) -> Option<AmountPer> {
        amount_per(&size(milli, unit, pack), form.as_ref())
    }

    #[test]
    fn a_plausible_pack_is_each_pieces_amount() {
        // Five half-gram pre-rolls; ten 10 mg gummies; a 2-pack of carts.
        assert_eq!(
            per(500, SizeUnit::Gram, 5, Some(Form::PreRoll)),
            Some(AmountPer::Each)
        );
        assert_eq!(
            per(10_000, SizeUnit::Milligram, 10, Some(Form::Edible)),
            Some(AmountPer::Each)
        );
        assert_eq!(
            per(1_000, SizeUnit::Gram, 2, Some(Form::VapeCart)),
            Some(AmountPer::Each)
        );
        // Exactly the limit is still plausible: 8 eighths make an ounce.
        assert_eq!(
            per(3_500, SizeUnit::Gram, 8, Some(Form::Flower)),
            Some(AmountPer::Each)
        );
        // A 6-pack of 355 ml cans.
        assert_eq!(
            per(355_000, SizeUnit::Millilitre, 6, Some(Form::Beverage)),
            Some(AmountPer::Each)
        );
    }

    #[test]
    fn an_implausibly_large_pack_is_the_packs_total() {
        // 10 × 3.5 g pre-rolls would be 35 g: 3.5 g in all.
        assert_eq!(
            per(3_500, SizeUnit::Gram, 10, Some(Form::PreRoll)),
            Some(AmountPer::Total)
        );
        // 20 × 100 mg gummies would be 2000 mg; 10 × 100 mg is the limit.
        assert_eq!(
            per(100_000, SizeUnit::Milligram, 20, Some(Form::Edible)),
            Some(AmountPer::Total)
        );
        assert_eq!(
            per(100_000, SizeUnit::Milligram, 10, Some(Form::Edible)),
            Some(AmountPer::Each)
        );
        // 5 × 2 g of concentrate would be 10 g, plausible; 5 × 2.5 g is not.
        assert_eq!(
            per(2_000, SizeUnit::Gram, 5, Some(Form::Concentrate)),
            Some(AmountPer::Each)
        );
        assert_eq!(
            per(2_500, SizeUnit::Gram, 5, Some(Form::Concentrate)),
            Some(AmountPer::Total)
        );
    }

    #[test]
    fn without_a_form_the_unit_decides() {
        assert_eq!(per(3_500, SizeUnit::Gram, 10, None), Some(AmountPer::Total));
        assert_eq!(per(500, SizeUnit::Gram, 5, None), Some(AmountPer::Each));
        assert_eq!(
            per(500, SizeUnit::Gram, 5, Some(Form::from_slug("suppository"))),
            Some(AmountPer::Each)
        );
        assert_eq!(
            per(200_000, SizeUnit::Milligram, 10, None),
            Some(AmountPer::Total)
        );
        // A unit from a newer app decides nothing.
        assert_eq!(per(500, SizeUnit::from_slug("oz_future"), 5, None), None);
    }

    #[test]
    fn needs_both_an_amount_and_a_pack() {
        let mut only_amount = size(500, SizeUnit::Gram, 5);
        only_amount.pack = None;
        assert_eq!(amount_per(&only_amount, Some(&Form::PreRoll)), None);
        let only_pack = PackageSize {
            amount: None,
            pack: Some(5),
            per: None,
        };
        assert_eq!(amount_per(&only_pack, Some(&Form::PreRoll)), None);
        // A huge misread can't overflow.
        assert_eq!(
            per(i64::MAX, SizeUnit::Gram, u32::MAX, None),
            Some(AmountPer::Total)
        );
    }
}

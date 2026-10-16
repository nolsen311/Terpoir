// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

//! Purchases: where and for how much products were bought.

use std::collections::BTreeMap;
use terpoir_crypto::RecordId;

use super::{Fields, LocalTime, Reader, Record, RecordKind, owns, slug_enum};
use crate::{Document, Error, Result, Value};

slug_enum! {
    /// The unit of a package size: the units receipts and labels print
    /// (`terpoir-scan` reads the same ones from a receipt line). Ounces and
    /// their fractions are kept in grams.
    SizeUnit {
        /// Grams (flower, concentrates, pre-rolls).
        Gram => "g",
        /// Milligrams (edibles, usually of THC).
        Milligram => "mg",
        /// Millilitres (tinctures, drinks).
        Millilitre => "ml",
    }
}

slug_enum! {
    /// What a package size's amount covers when the package also has a
    /// pack count (`5PK 0.5G`): each piece, or the whole pack. Either
    /// happens, by area and product (maintainer, 2026-10-15).
    AmountPer {
        /// Each piece: five pieces of 0.5 g.
        Each => "each",
        /// The whole pack: 0.5 g in all, across five pieces.
        Total => "total",
    }
}

/// An amount in a package: 3.5 g, 100 mg, 30 ml.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SizeAmount {
    /// The amount in thousandths of `unit` (3.5 g is 3500). Positive.
    pub milli: i64,
    /// The unit.
    pub unit: SizeUnit,
}

/// A package size, as printed on the receipt or label (docs/design-decisions.md,
/// "Simpler review": size and quantity on the purchase line): an amount, a
/// pack count (`5PK`), or both (`5PK 0.5G`). With both, [`per`](Self::per)
/// says whether the amount is each piece's or the whole pack's, once that
/// is decided. At least one of the amount and the pack is set.
///
/// Stored as a tuple of three (the amount, its unit, the pack), as 0.6.0
/// wrote it. What the amount covers (since 0.6.1) is stored beside it, in
/// its own field (`line_size_per/<line>` on a purchase, `default_size_per`
/// on a product), because 0.6.0 refuses a size tuple that isn't three
/// items; this way it reads the size and keeps the reading untouched.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PackageSize {
    /// The amount.
    pub amount: Option<SizeAmount>,
    /// Pieces in one package. At least 1.
    pub pack: Option<u32>,
    /// Whether the amount is each piece's or the pack's total; only with
    /// both an amount and a pack. `None` when undecided (as printed).
    pub per: Option<AmountPer>,
}

impl PackageSize {
    pub(crate) fn to_value(&self) -> Value {
        Value::Tuple(vec![
            self.amount
                .as_ref()
                .map_or(Value::Null, |a| Value::Int(a.milli)),
            self.amount
                .as_ref()
                .map_or(Value::Null, |a| Value::Text(a.unit.slug().to_owned())),
            self.pack.map_or(Value::Null, |p| Value::Int(i64::from(p))),
        ])
    }

    /// What the amount covers, as stored beside the size; `Null` when
    /// undecided.
    pub(crate) fn per_value(&self) -> Value {
        self.per
            .as_ref()
            .map_or(Value::Null, |p| Value::Text(p.slug().to_owned()))
    }

    /// Takes what the amount covers from its stored field: kept only when
    /// the size has both an amount and a pack (an app from before 0.6.1 may
    /// have changed the size and left it). `Err` for a value that isn't
    /// text.
    pub(crate) fn read_per(&mut self, value: &Value) -> core::result::Result<(), ()> {
        self.per = match value {
            Value::Null => None,
            Value::Text(slug) => Some(AmountPer::from_slug(slug)),
            _ => return Err(()),
        }
        .filter(|_| self.amount.is_some() && self.pack.is_some());
        Ok(())
    }

    /// Reads a stored size, with what its amount covers undecided (see
    /// [`read_per`](Self::read_per)); `None` if it is malformed (nothing
    /// set, half an amount, a zero or negative amount or pack).
    pub(crate) fn from_value(value: &Value) -> Option<Self> {
        let Value::Tuple(items) = value else {
            return None;
        };
        let [milli, unit, pack] = items.as_slice() else {
            return None;
        };
        let amount = match (milli, unit) {
            (Value::Null, Value::Null) => None,
            (Value::Int(milli), Value::Text(unit)) if *milli > 0 => Some(SizeAmount {
                milli: *milli,
                unit: SizeUnit::from_slug(unit),
            }),
            _ => return None,
        };
        let pack = match pack {
            Value::Null => None,
            Value::Int(p) => Some(u32::try_from(*p).ok().filter(|p| *p >= 1)?),
            _ => return None,
        };
        let size = Self {
            amount,
            pack,
            per: None,
        };
        size.is_valid().then_some(size)
    }

    /// Whether something is set, every number is positive, and what the
    /// amount covers is set only with both an amount and a pack.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        (self.amount.is_some() || self.pack.is_some())
            && self.amount.as_ref().is_none_or(|a| a.milli > 0)
            && self.pack.is_none_or(|p| p >= 1)
            && (self.per.is_none() || (self.amount.is_some() && self.pack.is_some()))
    }

    /// The amount in the whole package, in thousandths of its unit: the
    /// amount times the pack when it is each piece's, else the amount.
    /// `None` without an amount.
    #[must_use]
    pub fn total_milli(&self) -> Option<i64> {
        let amount = self.amount.as_ref()?;
        Some(match (self.per.as_ref(), self.pack) {
            (Some(AmountPer::Each), Some(pack)) => amount.milli.saturating_mul(i64::from(pack)),
            _ => amount.milli,
        })
    }
}

/// A price the user paid.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Price {
    /// Amount in the currency's minor unit (cents).
    pub minor_units: i64,
    /// ISO 4217 currency code.
    pub currency: String,
}

/// One product bought in a purchase. No `Debug`: the batch is user text.
#[derive(Clone, PartialEq, Eq)]
pub struct PurchaseLine {
    /// The product.
    pub product: Option<RecordId>,
    /// Batch or lot of this unit (it can differ between purchases).
    pub batch: Option<String>,
    /// What it cost.
    pub price: Option<Price>,
    /// The package size bought (since 0.6.0). Stored in the purchase's
    /// `line_sizes` map, and what a pack's amount covers (since 0.6.1) in
    /// its `line_size_per` map, so apps that predate them keep them
    /// untouched.
    pub size: Option<PackageSize>,
    /// How many were bought, at least 1 (since 0.6.0). Stored in the
    /// purchase's `line_quantities` map when more than 1; absent reads as 1.
    pub quantity: u32,
}

impl PurchaseLine {
    fn to_value(&self) -> Value {
        Value::Tuple(vec![
            self.product.map_or(Value::Null, Value::Id),
            self.batch.clone().map_or(Value::Null, Value::Text),
            self.price
                .as_ref()
                .map_or(Value::Null, |p| Value::Int(p.minor_units)),
            self.price
                .as_ref()
                .map_or(Value::Null, |p| Value::Text(p.currency.clone())),
        ])
    }

    fn from_value(value: &Value) -> Option<Self> {
        let Value::Tuple(items) = value else {
            return None;
        };
        let [product, batch, amount, currency] = items.as_slice() else {
            return None;
        };
        let product = match product {
            Value::Null => None,
            Value::Id(id) => Some(*id),
            _ => return None,
        };
        let batch = match batch {
            Value::Null => None,
            Value::Text(text) => Some(text.clone()),
            _ => return None,
        };
        let price = match (amount, currency) {
            (Value::Null, Value::Null) => None,
            (Value::Int(minor_units), Value::Text(currency)) => Some(Price {
                minor_units: *minor_units,
                currency: currency.clone(),
            }),
            _ => return None,
        };
        Some(Self {
            product,
            batch,
            price,
            size: None,
            quantity: 1,
        })
    }
}

/// A purchase: when, optionally where (a Re-up Spot), and what was bought.
#[derive(Clone, PartialEq, Eq)]
pub struct Purchase {
    /// When it was bought.
    pub when: LocalTime,
    /// The Re-up Spot it was bought at, if recorded.
    pub spot: Option<RecordId>,
    /// What was bought, keyed by line ID.
    pub lines: BTreeMap<RecordId, PurchaseLine>,
}

impl Purchase {
    /// A purchase with no lines.
    #[must_use]
    pub const fn new(when: LocalTime) -> Self {
        Self {
            when,
            spot: None,
            lines: BTreeMap::new(),
        }
    }
}

impl Record for Purchase {
    const KIND: RecordKind = RecordKind::Purchase;

    fn to_fields(&self) -> BTreeMap<String, Value> {
        let mut f = Fields::default();
        f.put("when", self.when.to_value())
            .opt("spot", self.spot, Value::Id)
            .map("lines", &self.lines, |id| id.to_hex(), |l| l.to_value())
            .map(
                "line_sizes",
                self.lines
                    .iter()
                    .filter_map(|(id, l)| l.size.as_ref().map(|s| (id, s))),
                |id| id.to_hex(),
                PackageSize::to_value,
            )
            .map(
                "line_size_per",
                self.lines.iter().filter_map(|(id, l)| {
                    l.size.as_ref().filter(|s| s.per.is_some()).map(|s| (id, s))
                }),
                |id| id.to_hex(),
                PackageSize::per_value,
            )
            .map(
                "line_quantities",
                self.lines
                    .iter()
                    .filter(|(_, l)| l.quantity != 1)
                    .map(|(id, l)| (id, l.quantity)),
                |id| id.to_hex(),
                |q| Value::Int(i64::from(q)),
            );
        f.finish()
    }

    fn from_document(doc: &Document) -> Result<Self> {
        let r = Reader::new::<Self>(doc)?;
        let mut lines = BTreeMap::new();
        for (key, value) in r.entries("lines") {
            let id = RecordId::from_hex(key).map_err(|_| Error::InvalidField("lines"))?;
            let line = PurchaseLine::from_value(value).ok_or(Error::InvalidField("lines"))?;
            lines.insert(id, line);
        }
        // A size or quantity for a line that is gone (removed on another
        // device) is ignored.
        for (key, value) in r.entries("line_sizes") {
            let (Ok(id), Some(size)) = (RecordId::from_hex(key), PackageSize::from_value(value))
            else {
                return Err(Error::InvalidField("line_sizes"));
            };
            if let Some(line) = lines.get_mut(&id) {
                line.size = Some(size);
            }
        }
        for (key, value) in r.entries("line_size_per") {
            let id = RecordId::from_hex(key).map_err(|_| Error::InvalidField("line_size_per"))?;
            if !matches!(value, Value::Text(_) | Value::Null) {
                return Err(Error::InvalidField("line_size_per"));
            }
            if let Some(size) = lines.get_mut(&id).and_then(|l| l.size.as_mut()) {
                size.read_per(value)
                    .map_err(|()| Error::InvalidField("line_size_per"))?;
            }
        }
        for (key, value) in r.entries("line_quantities") {
            let quantity = match value {
                Value::Int(q) => u32::try_from(*q).ok().filter(|q| *q >= 1),
                _ => None,
            };
            let (Ok(id), Some(quantity)) = (RecordId::from_hex(key), quantity) else {
                return Err(Error::InvalidField("line_quantities"));
            };
            if let Some(line) = lines.get_mut(&id) {
                line.quantity = quantity;
            }
        }
        Ok(Self {
            when: LocalTime::from_value(r.value("when"), "when")?,
            spot: r.id("spot")?,
            lines,
        })
    }

    fn owns_field(key: &str) -> bool {
        owns(
            key,
            &["when", "spot"],
            &["lines", "line_sizes", "line_size_per", "line_quantities"],
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DeviceId, Hlc, Stamp};

    fn stamp() -> Stamp {
        Stamp {
            hlc: Hlc::default(),
            device: DeviceId::from_bytes([1; 16]),
        }
    }

    fn id(n: u8) -> RecordId {
        RecordId::from_bytes([n; 16])
    }

    #[test]
    fn round_trip() {
        let mut purchase = Purchase::new(LocalTime {
            unix_ms: 1_760_148_000_000,
            utc_offset_min: -420,
        });
        purchase.spot = Some(id(9));
        purchase.lines.insert(
            id(1),
            PurchaseLine {
                product: Some(id(2)),
                batch: Some("B-1042".into()),
                price: Some(Price {
                    minor_units: 3_500,
                    currency: "USD".into(),
                }),
                size: Some(PackageSize {
                    amount: Some(SizeAmount {
                        milli: 3_500,
                        unit: SizeUnit::Gram,
                    }),
                    pack: None,
                    per: None,
                }),
                quantity: 2,
            },
        );
        purchase.lines.insert(
            id(3),
            PurchaseLine {
                product: None,
                batch: None,
                price: None,
                size: Some(PackageSize {
                    amount: Some(SizeAmount {
                        milli: 500,
                        unit: SizeUnit::from_slug("oz_future"),
                    }),
                    pack: Some(5),
                    per: Some(AmountPer::Total),
                }),
                quantity: 1,
            },
        );
        purchase.lines.insert(
            id(4),
            PurchaseLine {
                product: None,
                batch: None,
                price: None,
                size: None,
                quantity: 1,
            },
        );
        let doc = Document::create(&purchase, stamp()).unwrap();
        assert_eq!(doc.kind(), RecordKind::Purchase);
        assert!(Purchase::from_document(&doc).unwrap() == purchase);
    }

    #[test]
    fn rejects_a_half_price() {
        let when = LocalTime {
            unix_ms: 0,
            utc_offset_min: 0,
        };
        let mut doc = Document::create(&Purchase::new(when), stamp()).unwrap();
        let line = Value::Tuple(vec![Value::Null, Value::Null, Value::Int(100), Value::Null]);
        doc.set(format!("lines/{}", id(1).to_hex()), line, stamp());
        assert_eq!(
            Purchase::from_document(&doc).err(),
            Some(Error::InvalidField("lines"))
        );
    }

    #[test]
    fn sizes_and_quantities_sit_beside_the_line_for_older_apps() {
        let when = LocalTime {
            unix_ms: 0,
            utc_offset_min: 0,
        };
        // A line saved before 0.6.0: no size, quantity 1.
        let mut doc = Document::create(&Purchase::new(when), stamp()).unwrap();
        let line = Value::Tuple(vec![Value::Null, Value::Null, Value::Null, Value::Null]);
        doc.set(format!("lines/{}", id(1).to_hex()), line, stamp());
        let old = Purchase::from_document(&doc).unwrap();
        let read = &old.lines[&id(1)];
        assert_eq!((read.size.clone(), read.quantity), (None, 1));
        // The line's own tuple keeps its four items, so an app that
        // predates sizes still reads it; size and quantity are their own
        // map entries, which it leaves untouched.
        let mut new = old.clone();
        let line = new.lines.get_mut(&id(1)).unwrap();
        line.size = Some(PackageSize {
            amount: None,
            pack: Some(10),
            per: None,
        });
        line.quantity = 3;
        doc.update(&new, stamp()).unwrap();
        let key = id(1).to_hex();
        assert!(matches!(doc.value(&format!("lines/{key}")), Value::Tuple(t) if t.len() == 4));
        assert_eq!(
            doc.value(&format!("line_sizes/{key}")),
            &Value::Tuple(vec![Value::Null, Value::Null, Value::Int(10)])
        );
        assert_eq!(doc.value(&format!("line_quantities/{key}")), &Value::Int(3));
        assert!(Purchase::owns_field(&format!("line_sizes/{key}")));
        assert!(Purchase::owns_field(&format!("line_quantities/{key}")));
        assert!(Purchase::from_document(&doc).unwrap() == new);
        // Back to one: the entry is removed.
        new.lines.get_mut(&id(1)).unwrap().quantity = 1;
        doc.update(&new, stamp()).unwrap();
        assert!(doc.value(&format!("line_quantities/{key}")).is_null());
        // A size for a line removed elsewhere is ignored.
        doc.set(
            format!("line_sizes/{}", id(7).to_hex()),
            PackageSize {
                amount: None,
                pack: Some(2),
                per: None,
            }
            .to_value(),
            stamp(),
        );
        assert!(Purchase::from_document(&doc).unwrap() == new);
    }

    #[test]
    fn rejects_malformed_sizes_and_quantities() {
        let when = LocalTime {
            unix_ms: 0,
            utc_offset_min: 0,
        };
        let key = id(1).to_hex();
        for (field, value) in [
            (
                "line_sizes",
                Value::Tuple(vec![Value::Null, Value::Null, Value::Null]),
            ),
            (
                "line_sizes",
                Value::Tuple(vec![Value::Int(100), Value::Null, Value::Null]),
            ),
            (
                "line_sizes",
                Value::Tuple(vec![Value::Int(0), Value::Text("g".into()), Value::Null]),
            ),
            (
                "line_sizes",
                Value::Tuple(vec![Value::Null, Value::Null, Value::Int(0)]),
            ),
            ("line_sizes", Value::Int(3)),
            // A size tuple of four: 0.6.0 refuses it, so it is never written.
            (
                "line_sizes",
                Value::Tuple(vec![
                    Value::Int(500),
                    Value::Text("g".into()),
                    Value::Int(5),
                    Value::Text("each".into()),
                ]),
            ),
            ("line_size_per", Value::Int(1)),
            ("line_quantities", Value::Int(0)),
            ("line_quantities", Value::Text("2".into())),
        ] {
            let mut doc = Document::create(&Purchase::new(when), stamp()).unwrap();
            doc.set(format!("{field}/{key}"), value, stamp());
            assert!(Purchase::from_document(&doc).is_err(), "{field}");
        }
    }

    #[test]
    fn what_a_packs_amount_covers_sits_beside_the_size_for_0_6_0() {
        let when = LocalTime {
            unix_ms: 0,
            utc_offset_min: 0,
        };
        let size = |per| PackageSize {
            amount: Some(SizeAmount {
                milli: 500,
                unit: SizeUnit::Gram,
            }),
            pack: Some(5),
            per,
        };
        let mut purchase = Purchase::new(when);
        let line = |per| PurchaseLine {
            product: None,
            batch: None,
            price: None,
            size: Some(size(per)),
            quantity: 1,
        };
        for per in [
            None,
            Some(AmountPer::Each),
            Some(AmountPer::Total),
            Some(AmountPer::from_slug("per_serving")),
        ] {
            purchase.lines.insert(id(1), line(per));
            let doc = Document::create(&purchase, stamp()).unwrap();
            assert!(Purchase::from_document(&doc).unwrap() == purchase);
            // The size keeps the three items 0.6.0 reads.
            let key = id(1).to_hex();
            assert!(
                matches!(doc.value(&format!("line_sizes/{key}")), Value::Tuple(t) if t.len() == 3)
            );
        }
        let mut doc = Document::create(&purchase, stamp()).unwrap();
        let key = id(1).to_hex();
        assert!(Purchase::owns_field(&format!("line_size_per/{key}")));
        // An app from before 0.6.1 drops the pack and leaves the reading:
        // it no longer applies.
        doc.set(
            format!("line_sizes/{key}"),
            Value::Tuple(vec![Value::Int(500), Value::Text("g".into()), Value::Null]),
            stamp(),
        );
        let read = Purchase::from_document(&doc).unwrap();
        assert_eq!(read.lines[&id(1)].size.as_ref().unwrap().per, None);
        // The whole package's amount.
        assert_eq!(size(Some(AmountPer::Each)).total_milli(), Some(2_500));
        assert_eq!(size(Some(AmountPer::Total)).total_milli(), Some(500));
        assert_eq!(size(None).total_milli(), Some(500));
        // Only with both an amount and a pack.
        let mut pack_only = size(Some(AmountPer::Each));
        pack_only.amount = None;
        assert!(!pack_only.is_valid());
    }
}

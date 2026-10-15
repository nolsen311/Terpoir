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
/// pack count (`5PK`), or both (`5PK 0.5G`, kept as printed, without
/// deciding whether the amount is per piece or in all). At least one is set.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PackageSize {
    /// The amount.
    pub amount: Option<SizeAmount>,
    /// Pieces in one package. At least 1.
    pub pack: Option<u32>,
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

    /// Reads a stored size; `None` if it is malformed (nothing set, half an
    /// amount, a zero or negative amount or pack).
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
        let size = Self { amount, pack };
        size.is_valid().then_some(size)
    }

    /// Whether something is set and every number is positive.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        (self.amount.is_some() || self.pack.is_some())
            && self.amount.as_ref().is_none_or(|a| a.milli > 0)
            && self.pack.is_none_or(|p| p >= 1)
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
    /// `line_sizes` map, so apps that predate it keep it untouched.
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
            &["lines", "line_sizes", "line_quantities"],
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
            ("line_quantities", Value::Int(0)),
            ("line_quantities", Value::Text("2".into())),
        ] {
            let mut doc = Document::create(&Purchase::new(when), stamp()).unwrap();
            doc.set(format!("{field}/{key}"), value, stamp());
            assert!(Purchase::from_document(&doc).is_err(), "{field}");
        }
    }
}

// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

//! Purchases: where and for how much products were bought.

use std::collections::BTreeMap;
use terpoir_crypto::RecordId;

use super::{Fields, LocalTime, Reader, Record, RecordKind, owns};
use crate::{Document, Error, Result, Value};

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
            .map("lines", &self.lines, |id| id.to_hex(), |l| l.to_value());
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
        Ok(Self {
            when: LocalTime::from_value(r.value("when"), "when")?,
            spot: r.id("spot")?,
            lines,
        })
    }

    fn owns_field(key: &str) -> bool {
        owns(key, &["when", "spot"], &["lines"])
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
            },
        );
        purchase.lines.insert(
            id(3),
            PurchaseLine {
                product: None,
                batch: None,
                price: None,
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
}

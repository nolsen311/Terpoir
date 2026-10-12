// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

//! The plaintext export (docs/design-decisions.md, "Full export"): the whole
//! journal as readable files, for portability. Opt-in, behind a strong
//! warning in the app; nothing here is encrypted.
//!
//! - `terpoir.json`: every record with every field, generically from its
//!   stored document, so nothing is left out as the model grows.
//! - `sessions.csv`, `products.csv`, `purchases.csv`: the common views for
//!   spreadsheets, with names instead of record IDs.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use serde_json::{Map, Value as Json, json};
use terpoir_model::{
    Brand, Document, Product, Purchase, Record, RecordId, RecordKind, Session, Spot, Strain, Tag,
    TagRef, Value,
};

use crate::error::Result;

/// One file of a plaintext export. Carries journal content in the clear, so
/// it does not implement `Debug`.
#[derive(uniffi::Record)]
pub struct ExportFile {
    /// The file name, such as `sessions.csv`.
    pub name: String,
    /// The file's bytes (UTF-8 text).
    pub bytes: Vec<u8>,
}

/// The record kinds exported, with their JSON section names.
const KINDS: &[(RecordKind, &str)] = &[
    (RecordKind::Session, "sessions"),
    (RecordKind::Product, "products"),
    (RecordKind::Strain, "strains"),
    (RecordKind::Brand, "brands"),
    (RecordKind::Spot, "spots"),
    (RecordKind::Purchase, "purchases"),
    (RecordKind::Snippet, "notes"),
    (RecordKind::Tag, "tags"),
];

/// Reads documents for the export; the journal supplies it.
pub(crate) trait Source {
    fn ids(&self, kind: RecordKind) -> Result<Vec<RecordId>>;
    fn document(&self, id: &RecordId) -> Result<Option<Document>>;
}

/// Builds every file of the export.
pub(crate) fn export(source: &dyn Source, now_ms: u64) -> Result<Vec<ExportFile>> {
    let mut docs: BTreeMap<RecordKind, Vec<Document>> = BTreeMap::new();
    for (kind, _) in KINDS {
        let mut list = Vec::new();
        for id in source.ids(*kind)? {
            if let Some(doc) = source.document(&id)? {
                list.push(doc);
            }
        }
        docs.insert(*kind, list);
    }
    let names = Names::new(&docs);

    let mut records = Map::new();
    for (kind, section) in KINDS {
        let list = docs.get(kind).map_or(&[][..], Vec::as_slice);
        records.insert(
            (*section).to_owned(),
            Json::Array(list.iter().map(record_json).collect()),
        );
    }
    let json = json!({
        "format": "terpoir-plaintext",
        "version": 1,
        "exported_at_ms": now_ms,
        "records": records,
    });
    let json = serde_json::to_vec_pretty(&json).unwrap_or_default();

    let of = |kind| docs.get(&kind).map_or(&[][..], Vec::as_slice);
    Ok(vec![
        ExportFile {
            name: "terpoir.json".into(),
            bytes: json,
        },
        ExportFile {
            name: "sessions.csv".into(),
            bytes: sessions_csv(of(RecordKind::Session), &names).into_bytes(),
        },
        ExportFile {
            name: "products.csv".into(),
            bytes: products_csv(of(RecordKind::Product), &names).into_bytes(),
        },
        ExportFile {
            name: "purchases.csv".into(),
            bytes: purchases_csv(of(RecordKind::Purchase), &names).into_bytes(),
        },
    ])
}

/// A record as JSON: its ID and schema version, then its fields, with flattened map
/// entries (`effects/relaxed`) nested back (`effects: {relaxed: …}`).
/// Cleared fields are left out.
fn record_json(doc: &Document) -> Json {
    let mut fields = Map::new();
    for (key, field) in doc.fields() {
        if field.value.is_null() {
            continue;
        }
        let value = value_json(&field.value);
        match key.split_once('/') {
            Some((map, entry)) => {
                let slot = fields
                    .entry(map.to_owned())
                    .or_insert_with(|| Json::Object(Map::new()));
                if let Json::Object(entries) = slot {
                    entries.insert(entry.to_owned(), value);
                }
            }
            None => {
                fields.insert(key.clone(), value);
            }
        }
    }
    json!({
        "id": doc.id().to_hex(),
        "schema_version": doc.schema_version(),
        "fields": fields,
    })
}

fn value_json(value: &Value) -> Json {
    match value {
        Value::Null => Json::Null,
        Value::Bool(b) => Json::Bool(*b),
        Value::Int(i) => json!(i),
        Value::Text(t) => Json::String(t.clone()),
        Value::Id(id) => Json::String(id.to_hex()),
        Value::Tuple(items) => Json::Array(items.iter().map(value_json).collect()),
    }
}

/// Display names of the records CSV rows point at.
struct Names {
    records: BTreeMap<RecordId, String>,
}

impl Names {
    fn new(docs: &BTreeMap<RecordKind, Vec<Document>>) -> Self {
        let mut records = BTreeMap::new();
        let of = |kind| docs.get(&kind).map_or(&[][..], Vec::as_slice);
        for doc in of(RecordKind::Brand) {
            if let Ok(b) = Brand::from_document(doc) {
                records.insert(doc.id(), b.name);
            }
        }
        for doc in of(RecordKind::Strain) {
            if let Ok(s) = Strain::from_document(doc) {
                records.insert(doc.id(), s.name);
            }
        }
        for doc in of(RecordKind::Spot) {
            if let Ok(s) = Spot::from_document(doc) {
                records.insert(doc.id(), s.name);
            }
        }
        for doc in of(RecordKind::Tag) {
            if let Ok(t) = Tag::from_document(doc) {
                records.insert(doc.id(), t.name);
            }
        }
        // Products last: their label includes the brand's name.
        for doc in of(RecordKind::Product) {
            if let Ok(p) = Product::from_document(doc) {
                let label = match p.brand.and_then(|b| records.get(&b)) {
                    Some(brand) if !p.name.to_lowercase().starts_with(&brand.to_lowercase()) => {
                        format!("{brand} {}", p.name)
                    }
                    _ => p.name,
                };
                records.insert(doc.id(), label);
            }
        }
        Self { records }
    }

    fn of(&self, id: &RecordId) -> String {
        self.records.get(id).cloned().unwrap_or_default()
    }

    fn tag(&self, tag: &TagRef) -> String {
        match tag {
            TagRef::Builtin(slug) => slug.clone(),
            TagRef::Custom(id) => self.of(id),
        }
    }
}

fn sessions_csv(docs: &[Document], names: &Names) -> String {
    let mut out = Csv::new(&[
        "id",
        "date",
        "time",
        "utc_offset_min",
        "products",
        "methods",
        "doses",
        "intention",
        "social",
        "activity",
        "mood_before",
        "mood_after",
        "rating",
        "would_repeat",
        "effects",
        "onset_min",
        "peak_min",
        "duration_min",
        "spot",
        "symptoms",
    ]);
    for doc in docs {
        let Ok(s) = Session::from_document(doc) else {
            continue;
        };
        let (date, time) = local_date_time(s.when.unix_ms, s.when.utc_offset_min);
        let lines: Vec<_> = s.lines.values().collect();
        out.row(&[
            doc.id().to_hex(),
            date,
            time,
            s.when.utc_offset_min.to_string(),
            join(
                lines
                    .iter()
                    .map(|l| l.product.map(|p| names.of(&p)).unwrap_or_default()),
            ),
            join(lines.iter().map(|l| l.method.slug().to_owned())),
            join(lines.iter().map(|l| {
                l.dose
                    .as_ref()
                    .map(|d| format!("{} {}", milli(d.amount_milli), d.unit.slug()))
                    .unwrap_or_default()
            })),
            opt(s.intention.as_ref().map(|i| i.slug().to_owned())),
            opt(s.social.as_ref().map(|i| i.slug().to_owned())),
            s.activity.clone().unwrap_or_default(),
            opt(s.mood_before),
            opt(s.mood_after),
            opt(s.rating),
            opt(s.would_repeat),
            join(
                s.effects
                    .iter()
                    .map(|(t, v)| format!("{}={v}", names.tag(t))),
            ),
            opt(s.timeline.onset_min),
            opt(s.timeline.peak_min),
            opt(s.timeline.duration_min),
            s.spot.map(|p| names.of(&p)).unwrap_or_default(),
            join(
                s.symptoms
                    .iter()
                    .map(|(t, r)| format!("{}={}", names.tag(t), r.slug())),
            ),
        ]);
    }
    out.finish()
}

fn products_csv(docs: &[Document], names: &Names) -> String {
    let mut out = Csv::new(&[
        "id",
        "product",
        "name",
        "brand",
        "form",
        "strains",
        "thc",
        "thc_unit",
        "cbd",
        "cbd_unit",
        "terpenes_percent",
        "batch",
        "package_date",
        "shops",
    ]);
    for doc in docs {
        let Ok(p) = Product::from_document(doc) else {
            continue;
        };
        out.row(&[
            doc.id().to_hex(),
            names.of(&doc.id()),
            p.name.clone(),
            p.brand.map(|b| names.of(&b)).unwrap_or_default(),
            opt(p.form.as_ref().map(|f| f.slug().to_owned())),
            join(p.strains.iter().map(|s| names.of(s))),
            opt(p.thc.as_ref().map(|a| milli(a.milli))),
            opt(p.thc.as_ref().map(|a| a.unit.slug().to_owned())),
            opt(p.cbd.as_ref().map(|a| milli(a.milli))),
            opt(p.cbd.as_ref().map(|a| a.unit.slug().to_owned())),
            join(
                p.terpenes
                    .iter()
                    .map(|(t, v)| format!("{}={}", names.tag(t), milli(*v))),
            ),
            p.batch.clone().unwrap_or_default(),
            opt(p.package_date.map(iso_date)),
            join(p.shops.iter().map(|s| names.of(s))),
        ]);
    }
    out.finish()
}

fn purchases_csv(docs: &[Document], names: &Names) -> String {
    let mut out = Csv::new(&[
        "purchase_id",
        "date",
        "time",
        "spot",
        "product",
        "batch",
        "price_minor_units",
        "currency",
    ]);
    for doc in docs {
        let Ok(p) = Purchase::from_document(doc) else {
            continue;
        };
        let (date, time) = local_date_time(p.when.unix_ms, p.when.utc_offset_min);
        for line in p.lines.values() {
            out.row(&[
                doc.id().to_hex(),
                date.clone(),
                time.clone(),
                p.spot.map(|s| names.of(&s)).unwrap_or_default(),
                line.product.map(|id| names.of(&id)).unwrap_or_default(),
                line.batch.clone().unwrap_or_default(),
                opt(line.price.as_ref().map(|pr| pr.minor_units)),
                opt(line.price.as_ref().map(|pr| pr.currency.clone())),
            ]);
        }
    }
    out.finish()
}

/// RFC 4180 CSV. Cells that a spreadsheet would run as a formula (starting
/// with `=`, `+`, `-`, `@`, tab or carriage return) get a leading `'`, so
/// journal text can never execute in the user's spreadsheet.
struct Csv {
    out: String,
}

impl Csv {
    fn new(header: &[&str]) -> Self {
        let mut csv = Self { out: String::new() };
        csv.row(&header.iter().map(|h| (*h).to_owned()).collect::<Vec<_>>());
        csv
    }

    fn row(&mut self, cells: &[String]) {
        for (i, cell) in cells.iter().enumerate() {
            if i > 0 {
                self.out.push(',');
            }
            self.out.push_str(&cell_text(cell));
        }
        self.out.push_str("\r\n");
    }

    fn finish(self) -> String {
        self.out
    }
}

fn cell_text(cell: &str) -> String {
    let guarded =
        if cell.starts_with(['=', '+', '-', '@', '\t', '\r']) && cell.parse::<f64>().is_err() {
            format!("'{cell}")
        } else {
            cell.to_owned()
        };
    if guarded.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", guarded.replace('"', "\"\""))
    } else {
        guarded
    }
}

fn join(items: impl Iterator<Item = String>) -> String {
    items.collect::<Vec<_>>().join("; ")
}

fn opt<T: ToString>(value: Option<T>) -> String {
    value.map(|v| v.to_string()).unwrap_or_default()
}

/// Thousandths as a decimal: 2500 is `2.5`, -500 is `-0.5`.
fn milli(value: i64) -> String {
    let sign = if value < 0 { "-" } else { "" };
    let abs = value.unsigned_abs();
    let whole = abs / 1000;
    let frac = abs % 1000;
    if frac == 0 {
        format!("{sign}{whole}")
    } else {
        let mut s = format!("{sign}{whole}.{frac:03}");
        while s.ends_with('0') {
            s.pop();
        }
        s
    }
}

/// The local date (`2026-10-11`) and time (`20:40`) where it was recorded.
fn local_date_time(unix_ms: i64, utc_offset_min: i16) -> (String, String) {
    let local_min = unix_ms.div_euclid(60_000) + i64::from(utc_offset_min);
    let day = local_min.div_euclid(1440);
    let minute = local_min.rem_euclid(1440);
    let mut time = String::new();
    let _ = write!(time, "{:02}:{:02}", minute / 60, minute % 60);
    (iso_date(day), time)
}

/// Days since 1970-01-01 as `YYYY-MM-DD` (proleptic Gregorian; Howard
/// Hinnant's `civil_from_days`).
fn iso_date(days: i64) -> String {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates_and_times_are_local() {
        assert_eq!(iso_date(0), "1970-01-01");
        assert_eq!(iso_date(20_737), "2026-10-11");
        assert_eq!(iso_date(-1), "1969-12-31");
        assert_eq!(iso_date(11_016), "2000-02-29");
        // 2026-10-12 03:40 UTC is the evening before in UTC-7.
        assert_eq!(
            local_date_time(1_760_240_400_000, -420),
            ("2025-10-11".into(), "20:40".into())
        );
    }

    #[test]
    fn decimals_from_thousandths() {
        assert_eq!(milli(2_500), "2.5");
        assert_eq!(milli(22_400), "22.4");
        assert_eq!(milli(5_000), "5");
        assert_eq!(milli(-500), "-0.5");
        assert_eq!(milli(1), "0.001");
    }

    #[test]
    fn csv_cells_are_quoted_and_never_formulas() {
        assert_eq!(cell_text("plain"), "plain");
        assert_eq!(cell_text("a, b"), "\"a, b\"");
        assert_eq!(cell_text("say \"hi\""), "\"say \"\"hi\"\"\"");
        assert_eq!(cell_text("line\nbreak"), "\"line\nbreak\"");
        assert_eq!(cell_text("=HYPERLINK(\"x\")"), "\"'=HYPERLINK(\"\"x\"\")\"");
        assert_eq!(cell_text("@cmd"), "'@cmd");
        assert_eq!(cell_text("-0.5"), "-0.5");
        assert_eq!(cell_text("-rf"), "'-rf");
    }
}

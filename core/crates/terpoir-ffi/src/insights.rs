// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

//! Insights over the journal (see `terpoir-insights`): each session's
//! facts, the comparisons that clear the evidence thresholds, and the
//! user's feedback on them (dismissed, marked wrong, category muted), kept
//! in the encrypted store.

use std::collections::{BTreeMap, BTreeSet};

use terpoir_insights::{SessionFacts, insights, time_of_day};
use terpoir_model::{DoseUnit, Product, RecordId, Session, Strain, TagRef};

use crate::error::Result;

/// A difference in the user's ratings worth showing, with its evidence. The
/// app words it ("your sessions suggest…"); it is never advice.
#[derive(Clone, PartialEq, Eq, uniffi::Record)]
pub struct InsightData {
    /// Stable key for feedback, such as `terpene:myrcene`.
    pub key: String,
    /// `terpene`, `method`, `form`, `strain_type` or `time`.
    pub category: String,
    /// What was compared: a built-in slug (`myrcene`, `vape_cart`,
    /// `evening`), or a custom tag's name when [`Self::custom`].
    pub subject: String,
    /// Whether [`Self::subject`] is a custom tag's name.
    pub custom: bool,
    /// Average rating with it, in thousandths of a star.
    pub with_avg_milli: i64,
    /// Average rating without it.
    pub without_avg_milli: i64,
    /// The sessions with it, for tap-through.
    pub with_sessions: Vec<String>,
    /// How many rated sessions are without it.
    pub without_count: u32,
    /// Likely confounders: `method`, `time` or `dose`.
    pub confounders: Vec<String>,
}

/// What the user said about an insight.
#[derive(Clone, Copy, PartialEq, Eq, Debug, uniffi::Enum)]
pub enum InsightFeedback {
    /// Hide this insight.
    Dismissed,
    /// Hide it, and note that the user thinks it is wrong.
    Wrong,
    /// Hide every insight of this category (`key` is the category).
    Muted,
}

impl InsightFeedback {
    const fn slug(self) -> &'static str {
        match self {
            Self::Dismissed => "dismissed",
            Self::Wrong => "wrong",
            Self::Muted => "muted",
        }
    }
}

/// The store key of the feedback list (`key\tstate` lines).
pub(crate) const META_FEEDBACK: &str = "insight_feedback";

/// What the insights read from the journal.
pub(crate) trait Source {
    fn sessions(&self) -> Result<Vec<(RecordId, Session)>>;
    fn product(&self, id: &RecordId) -> Result<Option<Product>>;
    fn strain(&self, id: &RecordId) -> Result<Option<Strain>>;
    fn tag_name(&self, id: &RecordId) -> Result<Option<String>>;
    fn feedback(&self) -> Result<Vec<u8>>;
}

/// The insights to show: computed, then filtered by the user's feedback.
pub(crate) fn current(source: &dyn Source) -> Result<Vec<InsightData>> {
    let sessions = source.sessions()?;
    let mut products: BTreeMap<RecordId, Option<Product>> = BTreeMap::new();
    let mut strains: BTreeMap<RecordId, Option<Strain>> = BTreeMap::new();
    let mut facts = Vec::with_capacity(sessions.len());
    for (index, (_, session)) in sessions.iter().enumerate() {
        let mut features = BTreeSet::new();
        let mut doses = Vec::new();
        let mut all_mg = !session.lines.is_empty();
        for line in session.lines.values() {
            features.insert(format!("method:{}", line.method.slug()));
            match &line.dose {
                Some(d) if d.unit == DoseUnit::Milligrams => doses.push(d.amount_milli),
                _ => all_mg = false,
            }
            let Some(product_id) = line.product else {
                continue;
            };
            if let std::collections::btree_map::Entry::Vacant(slot) = products.entry(product_id) {
                slot.insert(source.product(&product_id)?);
            }
            let Some(Some(product)) = products.get(&product_id) else {
                continue;
            };
            if let Some(form) = &product.form {
                features.insert(format!("form:{}", form.slug()));
            }
            // The product's dominant terpene (ties: the first in key order).
            if let Some((tag, _)) =
                product
                    .terpenes
                    .iter()
                    .fold(None::<(&TagRef, i64)>, |best, (t, v)| match best {
                        Some((_, b)) if b >= *v => best,
                        _ => Some((t, *v)),
                    })
            {
                features.insert(format!("terpene:{}", tag_key(tag)));
            }
            for strain_id in &product.strains {
                if let std::collections::btree_map::Entry::Vacant(slot) = strains.entry(*strain_id)
                {
                    slot.insert(source.strain(strain_id)?);
                }
                if let Some(Some(strain)) = strains.get(strain_id)
                    && let Some(kind) = &strain.strain_type
                {
                    features.insert(format!("strain_type:{}", kind.slug()));
                }
            }
        }
        let local_min =
            session.when.unix_ms.div_euclid(60_000) + i64::from(session.when.utc_offset_min);
        let hour = u32::try_from(local_min.rem_euclid(1440) / 60).unwrap_or(0);
        features.insert(time_of_day(hour).to_owned());
        facts.push(SessionFacts {
            index,
            rating: session.rating,
            features,
            dose_mg_milli: (all_mg && !doses.is_empty()).then(|| doses.iter().sum()),
        });
    }

    let feedback = parse_feedback(&source.feedback()?);
    let mut out = Vec::new();
    for insight in insights(&facts) {
        let category = insight.category().to_owned();
        if feedback.contains_key(&insight.feature)
            || feedback.get(&category).is_some_and(|s| s == "muted")
        {
            continue;
        }
        let value = insight.feature.split_once(':').map_or("", |(_, v)| v);
        let (subject, custom) = match value.strip_prefix("custom:") {
            Some(hex) => match RecordId::from_hex(hex).ok() {
                Some(id) => (source.tag_name(&id)?.unwrap_or_default(), true),
                None => continue,
            },
            None => (value.to_owned(), false),
        };
        out.push(InsightData {
            key: insight.feature.clone(),
            category,
            subject,
            custom,
            with_avg_milli: insight.with_avg_milli,
            without_avg_milli: insight.without_avg_milli,
            with_sessions: insight
                .with_sessions
                .iter()
                .filter_map(|i| sessions.get(*i).map(|(id, _)| id.to_hex()))
                .collect(),
            without_count: u32::try_from(insight.without_count).unwrap_or(u32::MAX),
            confounders: insight.confounders.clone(),
        });
    }
    Ok(out)
}

fn tag_key(tag: &TagRef) -> String {
    match tag {
        TagRef::Builtin(slug) => slug.clone(),
        TagRef::Custom(id) => format!("custom:{}", id.to_hex()),
    }
}

fn parse_feedback(bytes: &[u8]) -> BTreeMap<String, String> {
    String::from_utf8_lossy(bytes)
        .lines()
        .filter_map(|l| l.split_once('\t'))
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
        .collect()
}

/// The feedback list with `key` set to `feedback`.
pub(crate) fn with_feedback(current: &[u8], key: &str, feedback: InsightFeedback) -> Vec<u8> {
    let mut all = parse_feedback(current);
    // Keys never hold tabs or newlines; refuse to corrupt the list if one does.
    if key.contains(['\t', '\n']) {
        return current.to_vec();
    }
    all.insert(key.to_owned(), feedback.slug().to_owned());
    all.iter()
        .map(|(k, v)| format!("{k}\t{v}\n"))
        .collect::<String>()
        .into_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feedback_round_trips() {
        let list = with_feedback(b"", "terpene:myrcene", InsightFeedback::Dismissed);
        let list = with_feedback(&list, "time", InsightFeedback::Muted);
        let list = with_feedback(&list, "terpene:myrcene", InsightFeedback::Wrong);
        let parsed = parse_feedback(&list);
        assert_eq!(
            parsed.get("terpene:myrcene").map(String::as_str),
            Some("wrong")
        );
        assert_eq!(parsed.get("time").map(String::as_str), Some("muted"));
        assert_eq!(
            with_feedback(&list, "bad\tkey", InsightFeedback::Dismissed),
            list
        );
    }
}

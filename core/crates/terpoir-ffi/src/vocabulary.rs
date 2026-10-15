// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

//! The built-in vocabularies, so platforms show the core's lists rather than
//! keeping their own. Platforms translate each slug for display; an unknown
//! slug (from a newer app version) is shown as is.

use terpoir_model::{
    Cannabinoid, CoreEffect, DoseUnit, Flavor, Form, Intention, LabelUnit, Method, Mood, Social,
    StrainDetail, StrainType, Subtype, SymptomResult, TargetedSymptom, Terpene, TerpeneLevel,
    check_in_minutes_for, timeline_plan_for,
};

use crate::records::TimelinePlanData;

/// Every built-in vocabulary, each as slugs in display order.
#[derive(Clone, PartialEq, Eq, Debug, uniffi::Record)]
pub struct Vocabulary {
    /// The core effects (relaxed, euphoric, …); custom effects are tags.
    pub core_effects: Vec<String>,
    /// Consumption methods.
    pub methods: Vec<String>,
    /// Dose units.
    pub dose_units: Vec<String>,
    /// Product forms.
    pub forms: Vec<String>,
    /// Units for cannabinoid label amounts.
    pub label_units: Vec<String>,
    /// Indica / sativa / hybrid.
    pub strain_types: Vec<String>,
    /// A strain's terpene levels, most first.
    pub terpene_levels: Vec<String>,
    /// Cannabinoids a strain can be known for, besides THC and CBD.
    pub cannabinoids: Vec<String>,
    /// The journal's strain detail steps, least first.
    pub strain_details: Vec<String>,
    /// Session intentions.
    pub intentions: Vec<String>,
    /// Solo or with others.
    pub social: Vec<String>,
    /// Targeted-symptom results.
    pub symptom_results: Vec<String>,
    /// Built-in targeted symptoms (custom symptoms are tags).
    pub symptoms: Vec<String>,
    /// Built-in terpene tags (custom terpenes are tags).
    pub terpenes: Vec<String>,
    /// Built-in flavor tags.
    pub flavors: Vec<String>,
    /// Built-in mood tags.
    pub moods: Vec<String>,
    /// Built-in product subtypes, every form's, in display order (see
    /// `subtypes_for_form` for one form's; custom subtypes are tags). Last,
    /// with a default, so callers that predate it compile.
    #[uniffi(default = [])]
    pub subtypes: Vec<String>,
}

fn owned(slugs: &[&str]) -> Vec<String> {
    slugs.iter().map(|s| (*s).to_owned()).collect()
}

/// The built-in vocabularies.
#[uniffi::export]
#[must_use]
pub fn vocabulary() -> Vocabulary {
    Vocabulary {
        core_effects: CoreEffect::ALL
            .iter()
            .map(|e| e.slug().to_owned())
            .collect(),
        methods: owned(Method::SLUGS),
        dose_units: owned(DoseUnit::SLUGS),
        forms: owned(Form::SLUGS),
        label_units: owned(LabelUnit::SLUGS),
        strain_types: owned(StrainType::SLUGS),
        terpene_levels: owned(TerpeneLevel::SLUGS),
        cannabinoids: owned(Cannabinoid::SLUGS),
        strain_details: owned(StrainDetail::SLUGS),
        intentions: owned(Intention::SLUGS),
        social: owned(Social::SLUGS),
        symptom_results: owned(SymptomResult::SLUGS),
        symptoms: owned(TargetedSymptom::SLUGS),
        terpenes: owned(Terpene::SLUGS),
        flavors: owned(Flavor::SLUGS),
        moods: owned(Mood::SLUGS),
        subtypes: owned(Subtype::SLUGS),
    }
}

/// The built-in subtypes under a product form (slug), in display order.
/// Empty for a form with none (flower, tincture) or from a newer app
/// version.
#[uniffi::export]
#[must_use]
pub fn subtypes_for_form(form: String) -> Vec<String> {
    Subtype::of(&Form::from_slug(&form))
        .map(|s| s.slug().to_owned())
        .collect()
}

/// The form a built-in subtype (slug) is under; `None` for one from a newer
/// app version.
#[uniffi::export]
#[must_use]
pub fn subtype_parent(subtype: String) -> Option<String> {
    Subtype::from_slug(&subtype)
        .parent()
        .map(|f| f.slug().to_owned())
}

/// The default follow-up check-in delay for a session with these methods
/// (slugs), in minutes: the longest of theirs; `None` for no check-in by
/// default (inhaled and topical methods, or none).
#[uniffi::export]
#[must_use]
pub fn check_in_minutes_for_methods(methods: Vec<String>) -> Option<u32> {
    let methods: Vec<Method> = methods.iter().map(|m| Method::from_slug(m)).collect();
    check_in_minutes_for(&methods)
}

/// The dose units that make sense for a method, the usual one first. Empty
/// when doses are not measured (topicals).
#[uniffi::export]
#[must_use]
pub fn dose_units_for_method(method: String) -> Vec<String> {
    Method::from_slug(&method)
        .dose_units()
        .iter()
        .map(|u| u.slug().to_owned())
        .collect()
}

/// The timeline plan for a session with these methods (slugs): the longest
/// method's; smoked's when there are none.
#[uniffi::export]
#[must_use]
pub fn timeline_plan_for_methods(methods: Vec<String>) -> TimelinePlanData {
    let methods: Vec<Method> = methods.iter().map(|m| Method::from_slug(m)).collect();
    let plan = timeline_plan_for(&methods);
    TimelinePlanData {
        minutes: plan.minutes,
        point_minutes: plan.point_minutes(),
        gesture_points: plan.gesture_points,
    }
}

/// The usual method for a product form, as a new session line's default.
/// A product with a subtype takes `Journal::usual_method` instead.
#[uniffi::export]
#[must_use]
pub fn default_method_for_form(form: String) -> Option<String> {
    Form::from_slug(&form)
        .default_method()
        .map(|m| m.slug().to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vocabulary_lists_every_core_effect_in_order() {
        let v = vocabulary();
        assert_eq!(v.core_effects.first().map(String::as_str), Some("relaxed"));
        assert_eq!(v.core_effects.len(), 10);
        assert!(v.forms.contains(&"pre_roll".to_owned()));
        assert!(v.methods.contains(&"vape_cart".to_owned()));
    }

    #[test]
    fn every_listed_method_and_form_resolves() {
        let v = vocabulary();
        for method in &v.methods {
            for unit in dose_units_for_method(method.clone()) {
                assert!(v.dose_units.contains(&unit), "{method}: {unit}");
            }
        }
        for form in &v.forms {
            let method = default_method_for_form(form.clone()).expect(form);
            assert!(v.methods.contains(&method), "{form}: {method}");
        }
        assert!(dose_units_for_method("topical".into()).is_empty());
        assert_eq!(default_method_for_form("unknown_form".into()), None);
        assert_eq!(dose_units_for_method("transdermal".into()), ["mg"]);
    }

    #[test]
    fn every_listed_subtype_is_under_a_listed_form() {
        let v = vocabulary();
        let mut listed = Vec::new();
        for form in &v.forms {
            for subtype in subtypes_for_form(form.clone()) {
                assert_eq!(subtype_parent(subtype.clone()).as_ref(), Some(form));
                listed.push(subtype);
            }
        }
        listed.sort();
        let mut all = v.subtypes.clone();
        all.sort();
        assert_eq!(listed, all);
        assert_eq!(
            subtypes_for_form("topical".into()),
            ["balm".to_owned(), "patch".to_owned()]
        );
        assert!(subtypes_for_form("flower".into()).is_empty());
        assert_eq!(subtype_parent("future".into()), None);
    }
}

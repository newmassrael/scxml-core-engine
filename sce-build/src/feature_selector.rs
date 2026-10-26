// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The predicates a generator writes to select code by the features a build
//! turns on (ADR 0057 §9).
//!
//! An author's condition is monotone ([`crate::presence_condition`]): it may not
//! negate, so that a feature only ever adds. What a generator writes is a
//! different thing. To pick the one alternative of a value that belongs to a
//! variant it writes the variant's *selector*, the exact predicate that holds in
//! that variant's configurations and in no other, and that predicate negates the
//! gates the variant leaves out. A [`Selector`] is therefore the full Boolean
//! language over feature names, spelled for each backend that selects at compile
//! time: Rust's `cfg` predicate and the C preprocessor's `#if` expression.
//!
//! A selector is kept small as it is built: a constant absorbs or annihilates
//! where it meets a combinator, and a combinator of one operand is its operand.

use serde::Serialize;

use crate::feature_model::{Configuration, FeatureModel};
use crate::presence_condition::PresenceCondition;

/// A Boolean predicate over the features of a build.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum Selector {
    /// Holds in every build.
    True,
    /// Holds in no build.
    False,
    /// The feature is on.
    Feature(String),
    All(Vec<Selector>),
    Any(Vec<Selector>),
    Not(Box<Selector>),
}

impl Selector {
    /// The selector of an author's condition: its canonical form (ADR 0057 §6),
    /// each least configuration written by the features nothing else in it
    /// requires.
    pub fn of_condition(condition: &PresenceCondition, model: &FeatureModel) -> Selector {
        let Some(form) = condition.canonical_form(model) else {
            return Selector::True;
        };
        Selector::any(
            form.terms().iter().map(|term| {
                Selector::all(term.iter().map(|name| Selector::Feature(name.to_string())))
            }),
        )
    }

    /// The selector of a configuration among `conditions`: the conjunction of
    /// each condition where `configuration` satisfies it and of its negation
    /// where it does not. Two configurations that every condition answers alike
    /// get one selector, and the selectors of distinct answers exclude each
    /// other, so over any set of configurations they partition the builds.
    pub fn of_variant(
        configuration: &Configuration,
        conditions: &[PresenceCondition],
        model: &FeatureModel,
    ) -> Selector {
        Selector::all(conditions.iter().map(|condition| {
            let selector = Selector::of_condition(condition, model);
            if condition.holds_in(configuration) {
                selector
            } else {
                !selector
            }
        }))
    }

    /// The conjunction of `operands`.
    pub fn all(operands: impl IntoIterator<Item = Selector>) -> Selector {
        let mut kept: Vec<Selector> = Vec::new();
        for operand in operands {
            match operand {
                Selector::True => {}
                Selector::False => return Selector::False,
                Selector::All(inner) => kept.extend(inner),
                other => kept.push(other),
            }
        }
        kept.sort();
        kept.dedup();
        match kept.len() {
            0 => Selector::True,
            1 => kept.pop().expect("one operand"),
            _ => Selector::All(kept),
        }
    }

    /// The disjunction of `operands`.
    pub fn any(operands: impl IntoIterator<Item = Selector>) -> Selector {
        let mut kept: Vec<Selector> = Vec::new();
        for operand in operands {
            match operand {
                Selector::False => {}
                Selector::True => return Selector::True,
                Selector::Any(inner) => kept.extend(inner),
                other => kept.push(other),
            }
        }
        kept.sort();
        kept.dedup();
        match kept.len() {
            0 => Selector::False,
            1 => kept.pop().expect("one operand"),
            _ => Selector::Any(kept),
        }
    }

    pub fn is_true(&self) -> bool {
        *self == Selector::True
    }

    /// Whether the selector holds where exactly the features of `on` are on.
    pub fn holds_with(&self, on: &dyn Fn(&str) -> bool) -> bool {
        match self {
            Selector::True => true,
            Selector::False => false,
            Selector::Feature(name) => on(name),
            Selector::All(operands) => operands.iter().all(|s| s.holds_with(on)),
            Selector::Any(operands) => operands.iter().any(|s| s.holds_with(on)),
            Selector::Not(operand) => !operand.holds_with(on),
        }
    }

    /// The predicate of a Rust `#[cfg(…)]`: `feature = "name"`, `all(…)`,
    /// `any(…)`, `not(…)`; `all()` holds and `any()` does not, which is how the
    /// two constants are written.
    pub fn rust(&self) -> String {
        match self {
            Selector::True => "all()".to_string(),
            Selector::False => "any()".to_string(),
            Selector::Feature(name) => format!("feature = \"{name}\""),
            Selector::All(operands) => format!("all({})", join(operands, Selector::rust, ", ")),
            Selector::Any(operands) => format!("any({})", join(operands, Selector::rust, ", ")),
            Selector::Not(operand) => format!("not({})", operand.rust()),
        }
    }

    /// The expression of a C or C++ `#if`, each feature its macro
    /// [`crate::feature_model::macro_spelling`] tested with `defined`.
    pub fn c(&self) -> String {
        match self {
            Selector::True => "1".to_string(),
            Selector::False => "0".to_string(),
            Selector::Feature(name) => {
                format!("defined({})", crate::feature_model::macro_spelling(name))
            }
            Selector::All(operands) => format!("({})", join(operands, Selector::c, " && ")),
            Selector::Any(operands) => format!("({})", join(operands, Selector::c, " || ")),
            Selector::Not(operand) => format!("!{}", operand.c()),
        }
    }
}

/// The negation, a double negation and a negated constant folded away.
impl std::ops::Not for Selector {
    type Output = Selector;

    fn not(self) -> Selector {
        match self {
            Selector::True => Selector::False,
            Selector::False => Selector::True,
            Selector::Not(inner) => *inner,
            other => Selector::Not(Box::new(other)),
        }
    }
}

fn join(operands: &[Selector], spell: fn(&Selector) -> String, separator: &str) -> String {
    operands
        .iter()
        .map(spell)
        .collect::<Vec<_>>()
        .join(separator)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::feature_model::FeatureDeclaration;
    use crate::presence_condition::parse;

    fn model() -> FeatureModel {
        FeatureModel::new(&[
            FeatureDeclaration {
                name: "unicast".into(),
                requires: vec![],
            },
            FeatureDeclaration {
                name: "accept".into(),
                requires: vec!["unicast".into()],
            },
            FeatureDeclaration {
                name: "auth".into(),
                requires: vec![],
            },
        ])
        .unwrap()
    }

    fn condition(m: &FeatureModel, text: &str) -> PresenceCondition {
        PresenceCondition::resolve(&parse(text).unwrap(), m).unwrap()
    }

    #[test]
    fn a_condition_is_selected_by_its_canonical_form() {
        let m = model();
        assert_eq!(
            Selector::of_condition(&condition(&m, "all(unicast, accept)"), &m).rust(),
            "feature = \"accept\""
        );
        assert_eq!(
            Selector::of_condition(&condition(&m, "any(auth, accept)"), &m).rust(),
            "any(feature = \"accept\", feature = \"auth\")"
        );
        assert_eq!(
            Selector::of_condition(&PresenceCondition::always(), &m),
            Selector::True
        );
    }

    #[test]
    fn the_selectors_of_the_variants_partition_every_configuration() {
        let m = model();
        let conditions = vec![condition(&m, "accept"), condition(&m, "auth")];
        let configurations = m.configurations();
        let selectors: Vec<Selector> = configurations
            .iter()
            .map(|c| Selector::of_variant(c, &conditions, &m))
            .collect();
        for c in &configurations {
            let on = |name: &str| c.contains(m.lookup(name).unwrap());
            let holding: Vec<&Selector> = selectors.iter().filter(|s| s.holds_with(&on)).collect();
            // Every configuration is picked, and only by selectors of variants
            // the conditions answer as they answer it.
            assert!(!holding.is_empty());
            let answers = |x: &Configuration| -> Vec<bool> {
                conditions.iter().map(|p| p.holds_in(x)).collect()
            };
            for (other, selector) in configurations.iter().zip(&selectors) {
                assert_eq!(selector.holds_with(&on), answers(other) == answers(c));
            }
        }
    }

    #[test]
    fn a_selector_is_spelled_for_rust_and_for_c() {
        let s = Selector::all([
            Selector::Feature("session-unicast".into()),
            !Selector::Feature("session-auth".into()),
        ]);
        assert_eq!(
            s.rust(),
            "all(feature = \"session-unicast\", not(feature = \"session-auth\"))"
        );
        assert_eq!(
            s.c(),
            "(defined(SCE_FEATURE_SESSION_UNICAST) && !defined(SCE_FEATURE_SESSION_AUTH))"
        );
        assert_eq!(Selector::True.rust(), "all()");
        assert_eq!(Selector::False.c(), "0");
    }

    #[test]
    fn a_constant_absorbs_or_annihilates_where_it_meets_a_combinator() {
        let a = Selector::Feature("a".into());
        assert_eq!(Selector::all([Selector::True, a.clone()]), a);
        assert_eq!(Selector::all([Selector::False, a.clone()]), Selector::False);
        assert_eq!(Selector::any([Selector::True, a.clone()]), Selector::True);
        assert_eq!(Selector::any([Selector::False, a.clone()]), a);
        assert_eq!(!!a.clone(), a);
        assert_eq!(Selector::all(Vec::new()), Selector::True);
        assert_eq!(Selector::any(Vec::new()), Selector::False);
    }
}

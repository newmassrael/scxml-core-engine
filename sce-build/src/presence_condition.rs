// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The condition under which an element of a document is present (ADR 0057 §1, §5).
//!
//! An author writes a condition in the grammar
//!
//! ```text
//! P ::= NAME | all(P, ...) | any(P, ...)
//! ```
//!
//! read by [`parse`] into a [`PresenceExpr`]. Against a [`FeatureModel`] it becomes a
//! [`PresenceCondition`], the boolean function of configurations it denotes. The
//! grammar has no negation, so every condition is *monotone*: a configuration that
//! satisfies it is contained in every larger configuration that does. That is what
//! keeps an element present when Cargo adds a feature some other crate asked for.
//!
//! # The canonical form
//!
//! A monotone function on the configurations of a model is fixed by the least
//! configurations that satisfy it, and those form an antichain (no one contains
//! another). [`PresenceCondition`] *is* that antichain, so two conditions are equal
//! exactly when they denote the same function, whatever was written. Each least
//! configuration is the closure under *requires* of one prime implicant of the
//! written formula; the form is spelled by writing each configuration by the
//! features of it that no other feature of it requires ([`FeatureModel::generators`]).
//!
//! # Deciding an implication
//!
//! `p ⇒ q` holds in every configuration iff `q` holds in each least configuration
//! of `p`: a configuration satisfying `p` contains one of them, and `q`, being
//! monotone, holds in everything containing a configuration it holds in. The check
//! is therefore exact, needs no enumeration of configurations, and its failure names
//! a least configuration in which `p` holds and `q` does not ([`PresenceCondition::implies`]).

use std::collections::BTreeSet;
use std::fmt;

use crate::feature_model::{is_feature_name, Configuration, FeatureModel};

/// A condition as written, before its names are resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PresenceExpr {
    /// A feature, with the byte offset of its name in the text it was read from.
    Feature {
        name: String,
        offset: usize,
    },
    All(Vec<PresenceExpr>),
    Any(Vec<PresenceExpr>),
}

/// Why a text is not a condition, at a byte offset of the text.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{message} (at byte {offset})")]
pub struct PresenceParseError {
    pub offset: usize,
    pub message: String,
}

/// Read `text` as a condition.
pub fn parse(text: &str) -> Result<PresenceExpr, PresenceParseError> {
    let mut reader = Reader { text, at: 0 };
    reader.skip_space();
    let expr = reader.expr()?;
    reader.skip_space();
    if reader.at != text.len() {
        return Err(reader.error("expected the end of the condition"));
    }
    Ok(expr)
}

struct Reader<'a> {
    text: &'a str,
    at: usize,
}

impl Reader<'_> {
    fn error(&self, message: impl Into<String>) -> PresenceParseError {
        PresenceParseError {
            offset: self.at,
            message: message.into(),
        }
    }

    fn peek(&self) -> Option<char> {
        self.text[self.at..].chars().next()
    }

    fn skip_space(&mut self) {
        while let Some(c) = self.peek() {
            if !c.is_ascii_whitespace() {
                break;
            }
            self.at += c.len_utf8();
        }
    }

    fn word(&mut self) -> &str {
        let start = self.at;
        while let Some(c) = self.peek() {
            if !(c.is_ascii_alphanumeric() || c == '-' || c == '_') {
                break;
            }
            self.at += c.len_utf8();
        }
        &self.text[start..self.at]
    }

    fn expr(&mut self) -> Result<PresenceExpr, PresenceParseError> {
        let start = self.at;
        let word = self.word().to_string();
        if word.is_empty() {
            return Err(self.error("expected a feature name, `all(` or `any(`"));
        }
        self.skip_space();
        if self.peek() != Some('(') {
            if !is_feature_name(&word) {
                return Err(PresenceParseError {
                    offset: start,
                    message: format!(
                        "`{word}` is not a feature name: a feature name is lower-case letters and digits, starting with a letter, in words joined by single hyphens"
                    ),
                });
            }
            return Ok(PresenceExpr::Feature {
                name: word,
                offset: start,
            });
        }
        let combine: fn(Vec<PresenceExpr>) -> PresenceExpr = match word.as_str() {
            "all" => PresenceExpr::All,
            "any" => PresenceExpr::Any,
            "not" => {
                return Err(PresenceParseError {
                    offset: start,
                    message: "a condition cannot negate: Cargo builds the union of the features its dependents ask for, so an element under `not(X)` would vanish because another crate turned `X` on. Write the behaviour for `X` on as a transition gated by `X` placed before the ungated one (ADR 0057 §1)".to_string(),
                })
            }
            other => {
                return Err(PresenceParseError {
                    offset: start,
                    message: format!("`{other}(` is not an operator: a condition combines with `all(` and `any(` only"),
                })
            }
        };
        self.at += 1;
        let mut operands = Vec::new();
        loop {
            self.skip_space();
            operands.push(self.expr()?);
            self.skip_space();
            match self.peek() {
                Some(',') => self.at += 1,
                Some(')') => {
                    self.at += 1;
                    return Ok(combine(operands));
                }
                _ => return Err(self.error("expected `,` or `)`")),
            }
        }
    }
}

/// A name of a condition that the model does not declare.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("`{name}` is not a feature of the model (at byte {offset})")]
pub struct UndeclaredFeature {
    pub name: String,
    pub offset: usize,
}

/// The function of configurations a condition denotes, held as the antichain of the
/// least configurations that satisfy it (see the module documentation).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PresenceCondition {
    least: BTreeSet<Configuration>,
}

impl PresenceCondition {
    /// The condition of an element nothing gates: it holds in every configuration,
    /// the empty one included.
    pub fn always() -> Self {
        Self {
            least: BTreeSet::from([Configuration::empty()]),
        }
    }

    /// The condition `expr` denotes in `model`.
    pub fn resolve(expr: &PresenceExpr, model: &FeatureModel) -> Result<Self, UndeclaredFeature> {
        match expr {
            PresenceExpr::Feature { name, offset } => {
                let feature = model.lookup(name).ok_or_else(|| UndeclaredFeature {
                    name: name.clone(),
                    offset: *offset,
                })?;
                Ok(Self {
                    least: BTreeSet::from([model.closure_of(feature).clone()]),
                })
            }
            PresenceExpr::All(operands) => {
                operands.iter().try_fold(Self::always(), |acc, operand| {
                    Ok(acc.and(&Self::resolve(operand, model)?))
                })
            }
            PresenceExpr::Any(operands) => {
                let mut resolved = operands.iter().map(|operand| Self::resolve(operand, model));
                let first = resolved
                    .next()
                    .expect("the grammar gives `any` an operand")?;
                resolved.try_fold(first, |acc, operand| Ok(acc.or(&operand?)))
            }
        }
    }

    /// Whether the condition holds in every configuration.
    pub fn is_always(&self) -> bool {
        self.least.contains(&Configuration::empty())
    }

    /// Whether the condition holds in `configuration`, which must be closed under
    /// *requires* (a [`Configuration`] a [`FeatureModel`] made).
    pub fn holds_in(&self, configuration: &Configuration) -> bool {
        self.least
            .iter()
            .any(|least| least.is_subset(configuration))
    }

    /// The least configurations that satisfy the condition.
    pub fn least_configurations(&self) -> impl Iterator<Item = &Configuration> {
        self.least.iter()
    }

    /// The conjunction. The union of two configurations is a configuration, so the
    /// least configurations of `p ∧ q` are the least of the pairwise unions.
    pub fn and(&self, other: &Self) -> Self {
        let unions = self
            .least
            .iter()
            .flat_map(|x| other.least.iter().map(move |y| x.union(y)));
        Self::least_of(unions)
    }

    /// The disjunction.
    pub fn or(&self, other: &Self) -> Self {
        Self::least_of(self.least.iter().chain(other.least.iter()).cloned())
    }

    /// Whether `self ⇒ other` holds in every configuration; when it does not, the
    /// least configuration that shows it: `self` holds there and `other` does not.
    /// Of several, the one with the fewest features comes first, then the one that
    /// is first in the model's order, so the witness is a function of the two
    /// conditions.
    pub fn implies(&self, other: &Self) -> Result<(), Configuration> {
        let witness = self
            .least
            .iter()
            .filter(|least| !other.holds_in(least))
            .min_by(|x, y| x.len().cmp(&y.len()).then_with(|| x.cmp(y)));
        match witness {
            Some(witness) => Err(witness.clone()),
            None => Ok(()),
        }
    }

    /// The canonical form, as a disjunction of conjunctions of feature names: each
    /// least configuration written by its generators, the conjunctions in order of
    /// their names. `None` for a condition that always holds, which no backend
    /// writes.
    pub fn canonical_form<'m>(&self, model: &'m FeatureModel) -> Option<CanonicalForm<'m>> {
        if self.is_always() {
            return None;
        }
        let mut terms: Vec<Vec<&'m str>> = self
            .least
            .iter()
            .map(|least| model.generators(least))
            .collect();
        terms.sort();
        Some(CanonicalForm { terms })
    }

    fn least_of(candidates: impl Iterator<Item = Configuration>) -> Self {
        let mut by_size: Vec<Configuration> = candidates.collect();
        by_size.sort_by(|x, y| x.len().cmp(&y.len()).then_with(|| x.cmp(y)));
        by_size.dedup();
        let mut least: Vec<Configuration> = Vec::new();
        for candidate in by_size {
            if !least.iter().any(|kept| kept.is_subset(&candidate)) {
                least.push(candidate);
            }
        }
        Self {
            least: least.into_iter().collect(),
        }
    }
}

/// A condition's canonical form: any of the terms, each all of its features. It is
/// not empty and none of its terms is empty.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalForm<'m> {
    terms: Vec<Vec<&'m str>>,
}

impl<'m> CanonicalForm<'m> {
    pub fn terms(&self) -> &[Vec<&'m str>] {
        &self.terms
    }

    /// Write the form with a backend's spelling of a feature and of the two
    /// combinators. A combinator of one operand is left out, so the form of a
    /// single feature is that feature's spelling.
    pub fn spell(
        &self,
        feature: impl Fn(&str) -> String,
        all: impl Fn(&[String]) -> String,
        any: impl Fn(&[String]) -> String,
    ) -> String {
        let terms: Vec<String> = self
            .terms
            .iter()
            .map(|term| {
                let features: Vec<String> = term.iter().map(|name| feature(name)).collect();
                if features.len() == 1 {
                    features.into_iter().next().expect("one feature")
                } else {
                    all(&features)
                }
            })
            .collect();
        if terms.len() == 1 {
            terms.into_iter().next().expect("one term")
        } else {
            any(&terms)
        }
    }
}

impl fmt::Display for CanonicalForm<'_> {
    /// The form in the authoring grammar.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let written = self.spell(
            str::to_string,
            |operands| format!("all({})", operands.join(", ")),
            |operands| format!("any({})", operands.join(", ")),
        );
        f.write_str(&written)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::feature_model::FeatureDeclaration;

    fn model(entries: &[(&str, &[&str])]) -> FeatureModel {
        let declarations: Vec<FeatureDeclaration> = entries
            .iter()
            .map(|(name, requires)| FeatureDeclaration {
                name: name.to_string(),
                requires: requires.iter().map(|r| r.to_string()).collect(),
            })
            .collect();
        FeatureModel::new(&declarations).expect("a valid model")
    }

    fn condition(model: &FeatureModel, text: &str) -> PresenceCondition {
        PresenceCondition::resolve(&parse(text).expect("parses"), model).expect("resolves")
    }

    fn canonical(model: &FeatureModel, text: &str) -> String {
        condition(model, text)
            .canonical_form(model)
            .map(|form| form.to_string())
            .unwrap_or_else(|| "always".to_string())
    }

    /// Every way of writing the same function is spelled alike.
    #[test]
    fn a_condition_is_spelled_by_what_it_means_not_by_how_it_was_written() {
        let m = model(&[
            ("accept", &["unicast"]),
            ("unicast", &[]),
            ("auth", &[]),
            ("tls", &[]),
        ]);
        assert_eq!(canonical(&m, "accept"), "accept");
        // `accept` requires `unicast`, so asking for both asks for `accept`.
        assert_eq!(canonical(&m, "all(unicast, accept)"), "accept");
        // Absorption: `accept` alone already implies the `all`.
        assert_eq!(canonical(&m, "any(accept, all(accept, auth))"), "accept");
        assert_eq!(canonical(&m, "any(unicast, accept)"), "unicast");
        // Distribution.
        assert_eq!(
            canonical(&m, "all(any(auth, tls), accept)"),
            canonical(&m, "any(all(accept, auth), all(tls, accept))")
        );
        assert_eq!(
            canonical(&m, "all(any(auth, tls), accept)"),
            "any(all(accept, auth), all(accept, tls))"
        );
        assert_eq!(canonical(&m, " all( tls ,auth ) "), "all(auth, tls)");
    }

    /// The canonical form is checked against the meaning of the formula on every
    /// configuration of the model.
    #[test]
    fn a_condition_holds_exactly_where_its_formula_is_true() {
        let m = model(&[("a", &["b"]), ("b", &[]), ("c", &[]), ("d", &["c"])]);
        let formulas = [
            "a",
            "any(a, c)",
            "all(a, d)",
            "any(all(a, c), d)",
            "all(any(a, d), any(b, c))",
            "any(all(b, c), all(a, d), all(a, b, c, d))",
        ];
        fn truth(expr: &PresenceExpr, m: &FeatureModel, c: &Configuration) -> bool {
            match expr {
                PresenceExpr::Feature { name, .. } => c.contains(m.lookup(name).unwrap()),
                PresenceExpr::All(ps) => ps.iter().all(|p| truth(p, m, c)),
                PresenceExpr::Any(ps) => ps.iter().any(|p| truth(p, m, c)),
            }
        }
        for text in formulas {
            let expr = parse(text).unwrap();
            let resolved = PresenceCondition::resolve(&expr, &m).unwrap();
            for c in m.configurations() {
                assert_eq!(
                    resolved.holds_in(&c),
                    truth(&expr, &m, &c),
                    "{text} at {c:?}"
                );
            }
        }
    }

    #[test]
    fn an_implication_is_decided_exactly_and_its_failure_is_the_least_configuration() {
        let m = model(&[("accept", &["unicast"]), ("unicast", &[]), ("auth", &[])]);
        let accept = condition(&m, "accept");
        let unicast = condition(&m, "unicast");
        assert_eq!(accept.implies(&unicast), Ok(()));
        let witness = unicast.implies(&accept).unwrap_err();
        assert_eq!(m.names_of(&witness).collect::<Vec<_>>(), ["unicast"]);
        // Against brute force on every pair of these conditions.
        let all = [
            "accept",
            "unicast",
            "auth",
            "any(accept, auth)",
            "all(unicast, auth)",
        ];
        for p in all {
            for q in all {
                let (p_c, q_c) = (condition(&m, p), condition(&m, q));
                let brute = m
                    .configurations()
                    .into_iter()
                    .all(|c| !p_c.holds_in(&c) || q_c.holds_in(&c));
                assert_eq!(p_c.implies(&q_c).is_ok(), brute, "{p} => {q}");
                if let Err(w) = p_c.implies(&q_c) {
                    assert!(p_c.holds_in(&w) && !q_c.holds_in(&w), "{p} => {q} witness");
                    assert!(m.is_configuration(&w));
                }
            }
        }
    }

    #[test]
    fn an_ungated_element_is_present_everywhere_and_implies_only_what_is_always() {
        let m = model(&[("a", &[])]);
        let always = PresenceCondition::always();
        assert!(always.is_always());
        assert!(always.holds_in(&Configuration::empty()));
        assert!(always.canonical_form(&m).is_none());
        let a = condition(&m, "a");
        assert_eq!(a.implies(&always), Ok(()));
        assert_eq!(always.implies(&a), Err(Configuration::empty()));
        assert_eq!(a.and(&always), a);
        assert_eq!(a.or(&always), always);
    }

    #[test]
    fn a_backend_spells_the_form_with_its_own_operators() {
        let m = model(&[("a", &[]), ("b", &[]), ("c", &[])]);
        let rust = |text: &str| {
            condition(&m, text).canonical_form(&m).unwrap().spell(
                |name| format!("feature = \"{name}\""),
                |ops| format!("all({})", ops.join(", ")),
                |ops| format!("any({})", ops.join(", ")),
            )
        };
        assert_eq!(rust("a"), "feature = \"a\"");
        assert_eq!(
            rust("any(all(b, a), c)"),
            "any(all(feature = \"a\", feature = \"b\"), feature = \"c\")"
        );
    }

    #[test]
    fn a_text_that_is_not_a_condition_is_refused_where_it_goes_wrong() {
        let at = |text: &str| parse(text).unwrap_err().offset;
        assert_eq!(at(""), 0);
        assert_eq!(at("all()"), 4);
        assert_eq!(at("all(a,)"), 6);
        assert_eq!(at("all(a b)"), 6);
        assert_eq!(at("a b"), 2);
        assert_eq!(at("Bad"), 0);
        assert_eq!(at("any(a, xor(b))"), 7);
        let negation = parse("not(a)").unwrap_err();
        assert_eq!(negation.offset, 0);
        assert!(
            negation.message.contains("cannot negate"),
            "{}",
            negation.message
        );
    }

    #[test]
    fn a_name_the_model_does_not_declare_is_refused_at_its_offset() {
        let m = model(&[("a", &[])]);
        let expr = parse("any(a, ghost)").unwrap();
        assert_eq!(
            PresenceCondition::resolve(&expr, &m),
            Err(UndeclaredFeature {
                name: "ghost".into(),
                offset: 7
            })
        );
    }

    #[test]
    fn a_feature_may_be_named_like_an_operator_when_it_is_not_called() {
        let m = model(&[("all", &[]), ("any", &[])]);
        assert_eq!(canonical(&m, "all(all, any)"), "all(all, any)");
    }
}

// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The features of a product line and the configurations they admit (ADR 0057 §1).
//!
//! A feature model is a finite set of features and the *requires* order on them:
//! `a ≤ b` reads "a requires b". A configuration is a set of features closed under
//! *requires*: when it holds `a` and `a ≤ b` it holds `b`. The configurations are
//! closed under union and intersection, so they form a finite distributive lattice
//! with bottom `∅` and top every feature (Birkhoff, "Rings of Sets", 1937).
//!
//! Closure under union is the property the model exists for. Cargo builds one copy
//! of a crate with the union of the features its dependents ask for, so a space of
//! configurations that a union can leave is one no crate can keep. That is why the
//! model holds *requires* and nothing else: an exclusion, an alternative group or an
//! "at least one of" group defines a space that a union leaves.
//!
//! This module knows nothing of XML. The `feature-model` document is read into a
//! list of [`FeatureDeclaration`]s by the forge parser and handed to
//! [`FeatureModel::new`], which is the one place the model's rules are enforced.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

/// A feature of one model, by its position in the model's order.
///
/// The order is derived, never authored: every feature comes after the features it
/// requires, and features that the order does not separate come by name. Two models
/// declared with the same features and edges in any order number them alike.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FeatureId(usize);

impl FeatureId {
    /// The position of the feature in its model's order.
    pub fn index(self) -> usize {
        self.0
    }
}

/// One `<sce:feature>` as written: its name and the names it requires.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeatureDeclaration {
    pub name: String,
    pub requires: Vec<String>,
}

/// Why a list of declarations is not a feature model.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FeatureModelError {
    /// The name is not `[a-z][a-z0-9]*(-[a-z0-9]+)*`.
    #[error("`{name}` is not a feature name: a feature name is lower-case letters and digits, starting with a letter, in words joined by single hyphens")]
    InvalidName { name: String },
    /// Two declarations give the same name.
    #[error("feature `{name}` is declared twice")]
    DuplicateName { name: String },
    /// A `requires` entry names no declared feature.
    #[error("feature `{feature}` requires `{missing}`, which the model does not declare")]
    UndeclaredRequirement { feature: String, missing: String },
    /// A feature requires itself, directly or through others. The features of a
    /// cycle are on and off together, so they are one feature under several names.
    #[error("features {} require each other in a cycle, so they are one feature: declare it once", cycle.join(" -> "))]
    Cycle { cycle: Vec<String> },
}

/// The features of a product line and the *requires* order on them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeatureModel {
    /// Names in the model's order.
    names: Vec<String>,
    /// For each feature, the features it requires directly, as written.
    direct: Vec<BTreeSet<FeatureId>>,
    /// For each feature, itself and every feature it requires, directly or not:
    /// the least configuration that holds it.
    closure: Vec<Configuration>,
    by_name: BTreeMap<String, FeatureId>,
}

/// A set of features. A [`Configuration`] made by a [`FeatureModel`] is closed under
/// its *requires* order; the type itself is the set, so that the terms of a
/// presence condition can be held in it too.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Configuration(BTreeSet<FeatureId>);

impl Configuration {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn contains(&self, feature: FeatureId) -> bool {
        self.0.contains(&feature)
    }

    pub fn is_subset(&self, other: &Configuration) -> bool {
        self.0.is_subset(&other.0)
    }

    pub fn union(&self, other: &Configuration) -> Configuration {
        Configuration(self.0.union(&other.0).copied().collect())
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = FeatureId> + '_ {
        self.0.iter().copied()
    }

    fn insert(&mut self, feature: FeatureId) {
        self.0.insert(feature);
    }
}

/// Why a selection of features given by name is not a configuration of a model.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SelectionError {
    #[error("`{name}` is not a feature of the model")]
    Undeclared { name: String },
    /// The selection holds `feature` but not `missing`, which `feature` requires.
    /// A selection is refused rather than completed, so that a build never runs a
    /// configuration nobody wrote.
    #[error("`{feature}` requires `{missing}`, which the selection leaves out")]
    MissingRequirement { feature: String, missing: String },
}

impl FeatureModel {
    /// Build the model of `declarations`, refusing a list that is not one.
    pub fn new(declarations: &[FeatureDeclaration]) -> Result<Self, FeatureModelError> {
        let mut declared: BTreeMap<&str, &FeatureDeclaration> = BTreeMap::new();
        for declaration in declarations {
            if !is_feature_name(&declaration.name) {
                return Err(FeatureModelError::InvalidName {
                    name: declaration.name.clone(),
                });
            }
            if declared.insert(&declaration.name, declaration).is_some() {
                return Err(FeatureModelError::DuplicateName {
                    name: declaration.name.clone(),
                });
            }
        }
        for declaration in declarations {
            for required in &declaration.requires {
                if !declared.contains_key(required.as_str()) {
                    return Err(FeatureModelError::UndeclaredRequirement {
                        feature: declaration.name.clone(),
                        missing: required.clone(),
                    });
                }
            }
        }
        let order = derived_order(&declared)?;
        let by_name: BTreeMap<String, FeatureId> = order
            .iter()
            .enumerate()
            .map(|(index, name)| (name.to_string(), FeatureId(index)))
            .collect();
        let direct: Vec<BTreeSet<FeatureId>> = order
            .iter()
            .map(|name| {
                declared[name]
                    .requires
                    .iter()
                    .map(|required| by_name[required])
                    .collect()
            })
            .collect();
        // The order puts every requirement first, so each closure is complete by
        // the time a feature that requires it is reached.
        let mut closure: Vec<Configuration> = Vec::with_capacity(order.len());
        for (index, requirements) in direct.iter().enumerate() {
            let mut least = Configuration::empty();
            least.insert(FeatureId(index));
            for required in requirements {
                least = least.union(&closure[required.index()]);
            }
            closure.push(least);
        }
        Ok(Self {
            names: order.into_iter().map(str::to_string).collect(),
            direct,
            closure,
            by_name,
        })
    }

    /// The number of features.
    pub fn len(&self) -> usize {
        self.names.len()
    }

    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }

    /// The features in the model's order.
    pub fn features(&self) -> impl Iterator<Item = FeatureId> + '_ {
        (0..self.names.len()).map(FeatureId)
    }

    pub fn name(&self, feature: FeatureId) -> &str {
        &self.names[feature.index()]
    }

    pub fn lookup(&self, name: &str) -> Option<FeatureId> {
        self.by_name.get(name).copied()
    }

    /// The features `feature` requires directly, as declared.
    pub fn requires(&self, feature: FeatureId) -> impl Iterator<Item = FeatureId> + '_ {
        self.direct[feature.index()].iter().copied()
    }

    /// The least configuration that holds `feature`: the feature and everything it
    /// requires, directly or not.
    pub fn closure_of(&self, feature: FeatureId) -> &Configuration {
        &self.closure[feature.index()]
    }

    /// The least configuration that holds every feature of `features`.
    pub fn close(&self, features: &Configuration) -> Configuration {
        features
            .iter()
            .fold(Configuration::empty(), |least, feature| {
                least.union(self.closure_of(feature))
            })
    }

    /// The configuration with every feature on, which holds every other.
    pub fn top(&self) -> Configuration {
        Configuration(self.features().collect())
    }

    /// Whether `set` is closed under *requires*.
    pub fn is_configuration(&self, set: &Configuration) -> bool {
        set.iter()
            .all(|feature| self.closure_of(feature).is_subset(set))
    }

    /// The configuration of the features named in `names`, refused when a name is
    /// not declared or when the selection leaves out a feature it requires.
    pub fn select<'a, I>(&self, names: I) -> Result<Configuration, SelectionError>
    where
        I: IntoIterator<Item = &'a str>,
    {
        let mut selected = Configuration::empty();
        for name in names {
            let feature = self
                .lookup(name)
                .ok_or_else(|| SelectionError::Undeclared {
                    name: name.to_string(),
                })?;
            selected.insert(feature);
        }
        for feature in selected.iter() {
            if let Some(missing) = self.requires(feature).find(|r| !selected.contains(*r)) {
                return Err(SelectionError::MissingRequirement {
                    feature: self.name(feature).to_string(),
                    missing: self.name(missing).to_string(),
                });
            }
        }
        Ok(selected)
    }

    /// The names of `configuration`'s features in the model's order.
    pub fn names_of<'a>(
        &'a self,
        configuration: &'a Configuration,
    ) -> impl Iterator<Item = &'a str> + 'a {
        configuration.iter().map(|feature| self.name(feature))
    }

    /// The features of `set` that no other feature of `set` requires, by name. A
    /// configuration is the closure of these, and they are how one is written.
    pub fn generators(&self, set: &Configuration) -> Vec<&str> {
        let mut generators: Vec<&str> = set
            .iter()
            .filter(|&feature| {
                !set.iter()
                    .any(|other| other != feature && self.closure_of(other).contains(feature))
            })
            .map(|feature| self.name(feature))
            .collect();
        generators.sort_unstable();
        generators
    }

    /// Every configuration of the model, each once.
    ///
    /// The features are decided in the model's order, so a feature's requirements
    /// are decided before it, and a feature may be taken only when all of them were.
    /// Each closed set is reached by exactly one sequence of decisions, so the walk
    /// costs the number of configurations, not `2^n`.
    pub fn configurations(&self) -> Vec<Configuration> {
        self.configurations_over(&self.top())
    }

    /// Every configuration of the model that holds no feature outside `within`,
    /// which must itself be a configuration, each once.
    ///
    /// These are exactly the sets `c ∩ within` for the configurations `c` of the
    /// whole model, so a condition whose least configurations all lie inside
    /// `within` takes every value over them that it takes over the whole model:
    /// it holds in `c` iff one of its least configurations is inside `c`, that
    /// is inside `c ∩ within`. The walk is [`Self::configurations`]'s over the
    /// features of `within` alone, which costs the configurations of those
    /// features, not of the model.
    pub fn configurations_over(&self, within: &Configuration) -> Vec<Configuration> {
        debug_assert!(
            self.is_configuration(within),
            "the features walked hold what they require"
        );
        let features: Vec<FeatureId> = within.iter().collect();
        let mut found = Vec::new();
        let mut current = Configuration::empty();
        self.extend_over(&features, 0, &mut current, &mut found);
        found
    }

    fn extend_over(
        &self,
        features: &[FeatureId],
        index: usize,
        current: &mut Configuration,
        found: &mut Vec<Configuration>,
    ) {
        let Some(&feature) = features.get(index) else {
            found.push(current.clone());
            return;
        };
        self.extend_over(features, index + 1, current, found);
        if self
            .requires(feature)
            .all(|required| current.contains(required))
        {
            current.insert(feature);
            self.extend_over(features, index + 1, current, found);
            current.0.remove(&feature);
        }
    }
}

impl fmt::Display for FeatureModel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for feature in self.features() {
            let mut required: Vec<&str> = self.requires(feature).map(|r| self.name(r)).collect();
            required.sort_unstable();
            if required.is_empty() {
                writeln!(f, "{}", self.name(feature))?;
            } else {
                writeln!(f, "{} requires {}", self.name(feature), required.join(" "))?;
            }
        }
        Ok(())
    }
}

/// Whether `name` is `[a-z][a-z0-9]*(-[a-z0-9]+)*`.
///
/// The grammar is chosen so that every backend's spelling of a name is a function
/// of the name that loses nothing: Cargo takes it as written, and the C, C++ and
/// Swift macro `SCE_FEATURE_<NAME>` upper-cases it and writes each hyphen as an
/// underscore. With no upper-case letter and no underscore in a name, two names
/// never spell alike.
pub fn is_feature_name(name: &str) -> bool {
    let mut words = name.split('-');
    let Some(first) = words.next() else {
        return false;
    };
    let first_ok = first.chars().next().is_some_and(|c| c.is_ascii_lowercase())
        && first.chars().all(is_name_char);
    first_ok && words.all(|word| !word.is_empty() && word.chars().all(is_name_char))
}

fn is_name_char(c: char) -> bool {
    c.is_ascii_lowercase() || c.is_ascii_digit()
}

/// The C, C++ and Swift spelling of a feature: `SCE_FEATURE_<NAME>`.
pub fn macro_spelling(name: &str) -> String {
    format!(
        "SCE_FEATURE_{}",
        name.to_ascii_uppercase().replace('-', "_")
    )
}

/// The features in an order where each comes after those it requires, the ones
/// the order leaves free coming by name; or the cycle that prevents one.
fn derived_order<'a>(
    declared: &BTreeMap<&'a str, &'a FeatureDeclaration>,
) -> Result<Vec<&'a str>, FeatureModelError> {
    // Kahn's algorithm with the ready set kept sorted, so the order is a function of
    // the model alone.
    let mut pending: BTreeMap<&str, usize> = declared
        .iter()
        .map(|(name, declaration)| {
            let distinct: BTreeSet<&str> =
                declaration.requires.iter().map(String::as_str).collect();
            (*name, distinct.len())
        })
        .collect();
    let mut required_by: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for (name, declaration) in declared {
        let distinct: BTreeSet<&str> = declaration.requires.iter().map(String::as_str).collect();
        for required in distinct {
            required_by.entry(required).or_default().push(name);
        }
    }
    let mut ready: BTreeSet<&str> = pending
        .iter()
        .filter(|(_, count)| **count == 0)
        .map(|(name, _)| *name)
        .collect();
    let mut order = Vec::with_capacity(declared.len());
    while let Some(name) = ready.pop_first() {
        order.push(name);
        for dependent in required_by.get(name).into_iter().flatten() {
            let count = pending
                .get_mut(dependent)
                .expect("every dependent is declared");
            *count -= 1;
            if *count == 0 {
                ready.insert(dependent);
            }
        }
    }
    if order.len() == declared.len() {
        return Ok(order);
    }
    let placed: BTreeSet<&str> = order.into_iter().collect();
    Err(FeatureModelError::Cycle {
        cycle: find_cycle(declared, &placed),
    })
}

/// A cycle among the features `placed` leaves out, written from its least name
/// back to itself. Every feature outside `placed` lies on a cycle or requires one,
/// so following requirements from any of them reaches one.
fn find_cycle(
    declared: &BTreeMap<&str, &FeatureDeclaration>,
    placed: &BTreeSet<&str>,
) -> Vec<String> {
    let start = declared
        .keys()
        .find(|name| !placed.contains(**name))
        .copied()
        .expect("an incomplete order leaves a feature out");
    let mut path: Vec<&str> = vec![start];
    loop {
        let last = *path.last().expect("the path starts non-empty");
        let mut requirements: Vec<&str> = declared[last]
            .requires
            .iter()
            .map(String::as_str)
            .filter(|r| !placed.contains(r))
            .collect();
        requirements.sort_unstable();
        let next = requirements[0];
        if let Some(position) = path.iter().position(|seen| *seen == next) {
            let mut cycle: Vec<&str> = path[position..].to_vec();
            let least = cycle
                .iter()
                .enumerate()
                .min_by_key(|(_, name)| **name)
                .map(|(index, _)| index)
                .expect("a cycle is non-empty");
            cycle.rotate_left(least);
            let first = cycle[0];
            cycle.push(first);
            return cycle.into_iter().map(str::to_string).collect();
        }
        path.push(next);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn declare(entries: &[(&str, &[&str])]) -> Vec<FeatureDeclaration> {
        entries
            .iter()
            .map(|(name, requires)| FeatureDeclaration {
                name: name.to_string(),
                requires: requires.iter().map(|r| r.to_string()).collect(),
            })
            .collect()
    }

    fn model(entries: &[(&str, &[&str])]) -> FeatureModel {
        FeatureModel::new(&declare(entries)).expect("a valid model")
    }

    fn names(model: &FeatureModel, configuration: &Configuration) -> Vec<String> {
        model.names_of(configuration).map(str::to_string).collect()
    }

    #[test]
    fn a_feature_comes_after_what_it_requires_whatever_order_it_is_written_in() {
        let written = model(&[("accept", &["unicast"]), ("unicast", &[]), ("auth", &[])]);
        let order: Vec<&str> = written.features().map(|f| written.name(f)).collect();
        assert_eq!(order, ["auth", "unicast", "accept"]);
        let reordered = model(&[("auth", &[]), ("unicast", &[]), ("accept", &["unicast"])]);
        assert_eq!(written, reordered);
    }

    #[test]
    fn the_closure_of_a_feature_is_everything_it_requires_transitively() {
        let m = model(&[("a", &["b"]), ("b", &["c"]), ("c", &[]), ("d", &[])]);
        let a = m.lookup("a").unwrap();
        assert_eq!(names(&m, m.closure_of(a)), ["c", "b", "a"]);
    }

    #[test]
    fn the_configurations_are_exactly_the_sets_closed_under_requires() {
        let m = model(&[("a", &["b"]), ("b", &[]), ("c", &[])]);
        let found = m.configurations();
        // Brute force over all subsets of three features.
        let all: Vec<FeatureId> = m.features().collect();
        let mut closed = Vec::new();
        for mask in 0u32..(1 << all.len()) {
            let set = Configuration(
                all.iter()
                    .enumerate()
                    .filter(|(i, _)| mask & (1 << i) != 0)
                    .map(|(_, f)| *f)
                    .collect(),
            );
            if m.is_configuration(&set) {
                closed.push(set);
            }
        }
        let mut sorted = found.clone();
        sorted.sort();
        closed.sort();
        assert_eq!(sorted, closed);
        assert_eq!(found.len(), 6, "{{}}, b, c, bc, ab, abc");
        let unique: BTreeSet<&Configuration> = found.iter().collect();
        assert_eq!(unique.len(), found.len(), "each configuration once");
    }

    #[test]
    fn the_union_of_two_configurations_is_a_configuration() {
        let m = model(&[("a", &["b"]), ("b", &[]), ("c", &["b"]), ("d", &[])]);
        let all = m.configurations();
        for x in &all {
            for y in &all {
                assert!(m.is_configuration(&x.union(y)));
            }
        }
    }

    #[test]
    fn a_selection_that_leaves_out_a_requirement_is_refused_not_completed() {
        let m = model(&[("accept", &["unicast"]), ("unicast", &[])]);
        assert_eq!(
            m.select(["accept"]),
            Err(SelectionError::MissingRequirement {
                feature: "accept".into(),
                missing: "unicast".into()
            })
        );
        assert_eq!(
            m.select(["nope"]),
            Err(SelectionError::Undeclared {
                name: "nope".into()
            })
        );
        let both = m.select(["unicast", "accept"]).unwrap();
        assert_eq!(names(&m, &both), ["unicast", "accept"]);
    }

    #[test]
    fn a_model_that_is_not_one_is_refused_with_the_reason() {
        assert_eq!(
            FeatureModel::new(&declare(&[("Bad", &[])])),
            Err(FeatureModelError::InvalidName { name: "Bad".into() })
        );
        assert_eq!(
            FeatureModel::new(&declare(&[("a", &[]), ("a", &[])])),
            Err(FeatureModelError::DuplicateName { name: "a".into() })
        );
        assert_eq!(
            FeatureModel::new(&declare(&[("a", &["z"])])),
            Err(FeatureModelError::UndeclaredRequirement {
                feature: "a".into(),
                missing: "z".into()
            })
        );
        assert_eq!(
            FeatureModel::new(&declare(&[
                ("b", &["c"]),
                ("c", &["a"]),
                ("a", &["b"]),
                ("d", &["a"])
            ])),
            Err(FeatureModelError::Cycle {
                cycle: vec!["a".into(), "b".into(), "c".into(), "a".into()]
            })
        );
        assert_eq!(
            FeatureModel::new(&declare(&[("a", &["a"])])),
            Err(FeatureModelError::Cycle {
                cycle: vec!["a".into(), "a".into()]
            })
        );
    }

    #[test]
    fn a_feature_name_is_lower_case_words_joined_by_single_hyphens() {
        for good in ["a", "session-unicast", "link-raweth", "x1", "a-1b"] {
            assert!(is_feature_name(good), "{good}");
        }
        for bad in ["", "-a", "a-", "a--b", "1a", "A", "a_b", "a.b", "a b", "é"] {
            assert!(!is_feature_name(bad), "{bad}");
        }
    }

    #[test]
    fn two_feature_names_never_share_a_macro_spelling() {
        assert_eq!(
            macro_spelling("session-unicast"),
            "SCE_FEATURE_SESSION_UNICAST"
        );
        // The grammar admits no underscore and no upper-case letter, so the
        // spelling is inverted by lower-casing and writing `_` as `-`.
        for name in ["a", "a-b", "ab", "a1-b2-c3"] {
            let spelled = macro_spelling(name);
            let back = spelled["SCE_FEATURE_".len()..]
                .to_ascii_lowercase()
                .replace('_', "-");
            assert_eq!(back, name);
        }
    }

    #[test]
    fn the_configurations_over_some_features_are_the_whole_models_cut_to_them() {
        let m = model(&[
            ("a", &["b"]),
            ("b", &[]),
            ("c", &[]),
            ("d", &["c"]),
            ("e", &[]),
        ]);
        let within = m.select(["a", "b", "c"]).unwrap();
        let mut over = m.configurations_over(&within);
        over.sort();
        let mut cut: Vec<Configuration> = m
            .configurations()
            .into_iter()
            .map(|c| Configuration(c.0.intersection(&within.0).copied().collect()))
            .collect();
        cut.sort();
        cut.dedup();
        assert_eq!(over, cut);
        assert!(over.iter().all(|c| m.is_configuration(c)));
    }

    #[test]
    fn a_configuration_is_written_by_the_features_nothing_in_it_requires() {
        let m = model(&[("a", &["b"]), ("b", &[]), ("c", &[])]);
        let set = m.select(["a", "b", "c"]).unwrap();
        assert_eq!(m.generators(&set), ["a", "c"]);
        assert!(m.generators(&Configuration::empty()).is_empty());
    }
}

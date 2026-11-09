// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The first check of docs/SCE_TEST_PROFILE.md §9.4: every construct of the
//! suite has exactly one entry, and every exported definition and statement
//! is named by one.
//!
//! - [`places`]: every place the reader of the modules finds is named by
//!   exactly one location of the witness, and every location names a place
//!   that exists.
//! - [`entries`]: every construct a rule of §9.2 translates has, for each
//!   rule [`crate::rules`] requires of it, exactly one entry, and every entry
//!   names a construct of the documents.

use std::collections::{BTreeMap, BTreeSet};

use crate::documents::Documents;
use crate::place::Segment;
use crate::rules::{self, Rule};
use crate::violation::Violation;
use crate::witness::{Construct, Witness};

/// The places the reader found in one module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModulePlaces {
    /// The module, by the name the export gives it.
    pub module: String,
    pub places: Vec<Vec<Segment>>,
}

/// A place of a module, by the module's name.
type Place = (String, Vec<Segment>);

/// The entries whose locations name one place: each by its construct and
/// rule.
type Namers<'w> = Vec<(&'w Construct, Rule)>;

/// The violations of the places of `modules`, the modules that parsed,
/// against the locations of `witness`. A location in a module that did not
/// parse is held to nothing: that module's parse violation stands for it.
pub fn places(modules: &[ModulePlaces], witness: &Witness) -> Vec<Violation> {
    let mut named: BTreeMap<Place, Namers<'_>> = BTreeMap::new();
    for entry in &witness.entries {
        for location in &entry.produced {
            named
                .entry((location.module.clone(), location.path.clone()))
                .or_default()
                .push((&entry.construct, entry.rule));
        }
    }

    let mut violations = Vec::new();
    let mut found: BTreeSet<Place> = BTreeSet::new();
    for read in modules {
        for path in &read.places {
            let place = (read.module.clone(), path.clone());
            if found.contains(&place) {
                violations.push(Violation::PlaceTwice {
                    module: read.module.clone(),
                    path: path.clone(),
                });
                continue;
            }
            match named.get(&place).map_or(0, Vec::len) {
                0 => violations.push(Violation::PlaceUnnamed {
                    module: read.module.clone(),
                    path: path.clone(),
                }),
                1 => {}
                times => violations.push(Violation::PlaceNamedTwice {
                    module: read.module.clone(),
                    path: path.clone(),
                    times,
                }),
            }
            found.insert(place);
        }
    }

    let parsed: BTreeSet<&str> = modules.iter().map(|read| read.module.as_str()).collect();
    let unparsed: BTreeSet<&str> = witness
        .modules
        .iter()
        .map(String::as_str)
        .filter(|module| !parsed.contains(module))
        .collect();
    for (place, namers) in &named {
        if found.contains(place) || unparsed.contains(place.0.as_str()) {
            continue;
        }
        for (construct, rule) in namers {
            violations.push(Violation::LocationDangling {
                module: place.0.clone(),
                path: place.1.clone(),
                construct: (*construct).clone(),
                rule: *rule,
            });
        }
    }
    violations
}

/// The violations of the entries of `witness` against `documents`.
pub fn entries(documents: &Documents, witness: &Witness) -> Vec<Violation> {
    let mut counted: BTreeMap<(Construct, Rule), usize> = BTreeMap::new();
    for entry in &witness.entries {
        *counted
            .entry((entry.construct.clone(), entry.rule))
            .or_default() += 1;
    }

    let mut violations = Vec::new();
    for ((construct, rule), &times) in &counted {
        if times > 1 {
            violations.push(Violation::EntryDuplicate {
                construct: construct.clone(),
                rule: *rule,
                times,
            });
        }
    }
    for (construct, rule) in counted.keys() {
        if !documents.holds(construct) {
            violations.push(Violation::ConstructDangling {
                construct: construct.clone(),
                rule: *rule,
            });
        }
    }
    for document in &documents.documents {
        for node in &document.nodes {
            for rule in rules::required(node.kind, &node.facts) {
                let key = (
                    Construct {
                        document: document.name.clone(),
                        node: node.node.clone(),
                        line: node.line,
                    },
                    rule,
                );
                if !counted.contains_key(&key) {
                    let (construct, rule) = key;
                    violations.push(Violation::ConstructMissing { construct, rule });
                }
            }
        }
    }
    violations
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::documents::tests::Memory;
    use crate::witness;

    const SUITE: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext" version="1.0" sce:kind="test-suite" name="s">
  <sce:module-parameter id="limit" type="uint32" value="5"/>
</scxml>"#;

    /// A witness of the suite alone: its two entries, the integer type of
    /// the types module, the module parameter and the control part.
    fn witness_of(entries: &str) -> Witness {
        witness::read(&format!(
            r#"{{"schema": "sce-ttcn3-witness.v1", "suite": "s", "modules": ["s_Types", "s"],
                "entries": [{entries}]}}"#
        ))
        .expect("a witness")
    }

    const E1: &str = r#"{"construct": {"document": "s.scxml", "node": "scxml", "line": 1},
        "rule": "E1", "produced": [{"module": "s_Types", "path": [{"type": "uint8"}]}]}"#;
    const PARAMETER: &str = r#"{"construct": {"document": "s.scxml",
        "node": "sce:module-parameter 'limit'", "line": 3},
        "rule": "E15", "produced": [{"module": "s", "path": [{"modulepar": "limit"}]}]}"#;
    const CONTROL: &str = r#"{"construct": {"document": "s.scxml", "node": "scxml", "line": 1},
        "rule": "E15", "produced": [{"module": "s", "path": [{"control": "s"}]}]}"#;

    fn modules() -> Vec<ModulePlaces> {
        vec![
            ModulePlaces {
                module: "s_Types".to_string(),
                places: vec![vec![Segment::Type("uint8".to_string())]],
            },
            ModulePlaces {
                module: "s".to_string(),
                places: vec![
                    vec![Segment::Modulepar("limit".to_string())],
                    vec![Segment::Control("s".to_string())],
                ],
            },
        ]
    }

    fn documents() -> Documents {
        Documents::read(&Memory(BTreeMap::from([("s.scxml", SUITE)])), "s.scxml")
            .expect("documents")
    }

    fn codes(violations: &[Violation]) -> Vec<&'static str> {
        violations.iter().map(Violation::code).collect()
    }

    #[test]
    fn a_witness_that_names_each_place_and_construct_once_has_no_violation() {
        let witness = witness_of(&[E1, PARAMETER, CONTROL].join(","));
        assert_eq!(places(&modules(), &witness), Vec::<Violation>::new());
        assert_eq!(entries(&documents(), &witness), Vec::<Violation>::new());
    }

    /// A place no location names; a place two locations name; a location
    /// that names no place.
    #[test]
    fn each_place_is_named_by_exactly_one_location() {
        let witness = witness_of(&[E1, CONTROL].join(","));
        let found = places(&modules(), &witness);
        assert_eq!(codes(&found), ["place-unnamed"]);
        assert_eq!(
            found[0].to_string(),
            r#"place-unnamed: s [{"modulepar":"limit"}]: no location of the witness names it"#
        );

        let twice = CONTROL.replace(
            r#"[{"module": "s", "path": [{"control": "s"}]}]"#,
            r#"[{"module": "s", "path": [{"control": "s"}]}, {"module": "s", "path": [{"control": "s"}]}]"#,
        );
        let witness = witness_of(&[E1, PARAMETER, twice.as_str()].join(","));
        assert_eq!(codes(&places(&modules(), &witness)), ["place-named-twice"]);

        let dangling = PARAMETER.replace(r#"{"modulepar": "limit"}"#, r#"{"modulepar": "limits"}"#);
        let witness = witness_of(&[E1, dangling.as_str(), CONTROL].join(","));
        assert_eq!(
            codes(&places(&modules(), &witness)),
            ["place-unnamed", "location-dangling"]
        );
    }

    /// A module that did not parse holds no location to a place: its parse
    /// violation stands for them.
    #[test]
    fn a_module_that_did_not_parse_holds_no_location() {
        let witness = witness_of(&[E1, PARAMETER, CONTROL].join(","));
        let both = modules();
        assert_eq!(places(&both[..1], &witness), Vec::<Violation>::new());
    }

    /// A construct with no entry of a rule it requires; a construct and rule
    /// with two entries; an entry whose construct the documents do not hold.
    #[test]
    fn each_construct_has_exactly_one_entry_of_each_rule() {
        let witness = witness_of(&[E1, CONTROL].join(","));
        let found = entries(&documents(), &witness);
        assert_eq!(codes(&found), ["construct-missing"]);
        assert_eq!(
            found[0].to_string(),
            "construct-missing: s.scxml sce:module-parameter 'limit' (row 3): E15 translates \
             it, and no entry says so"
        );

        let witness = witness_of(&[E1, PARAMETER, PARAMETER, CONTROL].join(","));
        assert_eq!(codes(&entries(&documents(), &witness)), ["entry-duplicate"]);

        let renamed = PARAMETER.replace("'limit'", "'limits'");
        let witness = witness_of(&[E1, renamed.as_str(), CONTROL].join(","));
        assert_eq!(
            codes(&entries(&documents(), &witness)),
            ["construct-dangling", "construct-missing"]
        );

        let moved = PARAMETER.replace(r#""line": 3"#, r#""line": 4"#);
        let witness = witness_of(&[E1, moved.as_str(), CONTROL].join(","));
        assert_eq!(
            codes(&entries(&documents(), &witness)),
            ["construct-dangling", "construct-missing"]
        );
    }
}

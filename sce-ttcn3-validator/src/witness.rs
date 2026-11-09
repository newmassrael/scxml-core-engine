// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The correspondence witness, `S.witness.json`, read into a model of this
//! crate's own (`schemas/sce-ttcn3-witness.v1.schema.json`;
//! docs/SCE_TEST_PROFILE.md §9.4).
//!
//! A witness is one entry per construct of the suite and rule of §9.2 that
//! translated it, each with every place of the two modules the rule wrote.
//! The model refuses a member the shape does not have, and a witness of
//! another shape by its name, so a witness it reads is one it reads whole.

use std::fmt;

use serde::Deserialize;

use crate::place::Segment;
use crate::rules::Rule;
use crate::InputError;

/// The name of the one shape this crate reads, the witness's `schema`.
pub const SCHEMA: &str = "sce-ttcn3-witness.v1";

/// A witness.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Witness {
    pub schema: String,
    /// The suite's unit name.
    pub suite: String,
    /// The types module, then the behaviour module.
    pub modules: Vec<String>,
    pub entries: Vec<Entry>,
}

/// One construct, the rule that translated it, and every place the rule
/// wrote for it.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    pub construct: Construct,
    pub rule: Rule,
    pub produced: Vec<Location>,
}

/// A construct of a document: the document by its path relative to the
/// suite's directory, the node, and the row of the node where the documents
/// keep one.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Construct {
    pub document: String,
    pub node: String,
    pub line: Option<u32>,
}

impl fmt::Display for Construct {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.document, self.node)?;
        match self.line {
            Some(line) => write!(f, " (row {line})"),
            None => f.write_str(" (no row)"),
        }
    }
}

/// A place of a module, and what it claims.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Location {
    pub module: String,
    pub path: Vec<Segment>,
    #[serde(default)]
    pub claim: Option<Claim>,
}

/// What an `alt` or one of its alternatives claims about the documents.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Claim {
    /// The atomic state an `alt` is in, and the transition each of its
    /// alternatives is, in textual order.
    Alt {
        state: Construct,
        alternatives: Vec<Construct>,
    },
    /// The atomic state an alternative's `alt` is in, the transition it is,
    /// and the states its block exits and enters, each in its order.
    Branch {
        state: Construct,
        transition: Construct,
        exited: Vec<Construct>,
        entered: Vec<Construct>,
    },
}

/// The witness `text` is, of the shape [`SCHEMA`].
pub fn read(text: &str) -> Result<Witness, InputError> {
    let value: serde_json::Value = serde_json::from_str(text)
        .map_err(|error| InputError::Witness(format!("it is no JSON: {error}")))?;
    // The shape's name first, so a witness of another shape is refused by
    // its name rather than by the first member this model does not know.
    match value.get("schema").and_then(serde_json::Value::as_str) {
        Some(SCHEMA) => {}
        Some(other) => {
            return Err(InputError::Witness(format!(
                "its schema is '{other}', and this validator reads '{SCHEMA}'"
            )))
        }
        None => return Err(InputError::Witness("it names no schema".to_string())),
    }
    serde_json::from_value(value)
        .map_err(|error| InputError::Witness(format!("it is no witness of {SCHEMA}: {error}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    const WITNESS: &str = r#"{
      "schema": "sce-ttcn3-witness.v1",
      "suite": "s",
      "modules": ["s_Types", "s"],
      "entries": [
        {
          "construct": {"document": "c.scxml", "node": "state 'a'", "line": 3},
          "rule": "E7",
          "produced": [{
            "module": "s",
            "path": [{"function": "f_c"}, {"label": "a"}, {"statement": 0}],
            "claim": {
              "kind": "alt",
              "state": {"document": "c.scxml", "node": "state 'a'", "line": 3},
              "alternatives": [{"document": "c.scxml", "node": "state 'a' / transition 1", "line": 4}]
            }
          }]
        },
        {
          "construct": {"document": "c.scxml", "node": "scxml", "line": null},
          "rule": "E5",
          "produced": [{"module": "s", "path": [{"function": "f_c"}]}]
        }
      ]
    }"#;

    #[test]
    fn a_witness_is_read_whole() {
        let witness = read(WITNESS).expect("a witness");
        assert_eq!(witness.suite, "s");
        assert_eq!(witness.entries.len(), 2);
        assert_eq!(witness.entries[0].rule, Rule::E7);
        assert!(matches!(
            witness.entries[0].produced[0].claim,
            Some(Claim::Alt { .. })
        ));
        assert_eq!(witness.entries[1].construct.line, None);
        assert_eq!(witness.entries[1].produced[0].claim, None);
        assert_eq!(
            witness.entries[1].construct.to_string(),
            "c.scxml scxml (no row)"
        );
    }

    /// A witness of another shape is refused by its name; a member the
    /// shape does not have is refused.
    #[test]
    fn a_witness_of_another_shape_is_refused() {
        let other = WITNESS.replace("sce-ttcn3-witness.v1", "sce-ttcn3-witness.v2");
        assert!(matches!(read(&other), Err(InputError::Witness(why)) if why.contains("v2")));
        let unknown = WITNESS.replace("\"rule\": \"E5\"", "\"rule\": \"E5\", \"why\": 1");
        assert!(read(&unknown).is_err());
        let rule = WITNESS.replace("\"E5\"", "\"E16\"");
        assert!(read(&rule).is_err());
        assert!(read("[]").is_err());
    }
}

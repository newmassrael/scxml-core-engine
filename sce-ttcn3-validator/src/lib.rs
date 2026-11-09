// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The validator of a test suite's TTCN-3 export (docs/adr/0091 answer 12;
//! docs/SCE_TEST_PROFILE.md §9.4).
//!
//! The equivalence of a suite and its export is held by translation
//! validation of every export: beside the types module `S_Types.ttcn` and
//! the behaviour module `S.ttcn`, the exporter writes the correspondence
//! witness `S.witness.json`, and this crate holds the witness against the
//! suite's documents and the two modules. It is separate from the exporter
//! and shares no code with it: it depends on no crate of SCE, and reads each
//! of its three inputs with a reader of its own.
//!
//! - [`documents`] reads the profile's documents the suite reaches, from its
//!   `test-suite` through the `<sce:import>`s of its test cases,
//!   configurations, components, port types, templates, event schemas, enums
//!   and unions, and its feature model, and names each construct as the
//!   witness does: the document by its path relative to the suite's
//!   directory, the node by its kind and id or its position, the row of the
//!   node.
//! - [`witness`] reads the witness, of the one shape this crate knows.
//! - [`ttcn3`] reads the TTCN-3 the export writes, and nothing beyond it,
//!   into a tree each of whose definitions and statements is a place of the
//!   witness's vocabulary ([`place`]).
//!
//! [`rules`] states, in one table, which rule of §9.2 translates a construct
//! of each kind and when one must. The three checks of §9.4:
//!
//! - [`coverage`]: every place of the two modules is named by exactly one
//!   location of the witness and every location names a place, and every
//!   construct has exactly one entry for each rule that translates it and
//!   every entry names a construct of the documents.
//! - [`form`]: each entry's rule translates its construct's kind, and every
//!   place it names has the TTCN-3 form that rule writes for the construct,
//!   read from the documents ([`types`] reads a type of the documents).
//! - [`order`]: each `alt`'s alternatives are the branches of its atomic
//!   state in the order of P3, and each block is the exits, the content and
//!   the entries of its transition from that state, the sets computed by
//!   [`scxml313`] from the documents and held against the witness's claims.
//!
//! Each finding is a [`Violation`] with a stable code.

pub mod coverage;
pub mod documents;
#[cfg(test)]
mod fixture;
pub mod form;
pub mod order;
pub mod place;
pub mod rules;
pub mod scxml313;
pub mod ttcn3;
pub mod types;
pub mod violation;
pub mod witness;

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

pub use violation::Violation;

use documents::{Directory, Documents};

/// The three texts of an export the validator reads: the two modules, each
/// with or without the header of a generated source, and the witness.
#[derive(Debug, Clone, Copy)]
pub struct Export<'a> {
    /// `S_Types.ttcn`.
    pub types_module: &'a str,
    /// `S.ttcn`.
    pub behaviour_module: &'a str,
    /// `S.witness.json`.
    pub witness: &'a str,
}

/// Why an export is not validated: an input that cannot be read, or that is
/// not one this validator reads. A violation is a finding about an input it
/// did read ([`Violation`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputError {
    /// A file that cannot be read.
    Read { path: String, reason: String },
    /// A document that is no XML.
    Xml { document: String, reason: String },
    /// A document this validator does not read: of no kind of the profile,
    /// of another kind than its import declares, a document of the standard
    /// library, or one whose states it cannot name.
    Document { document: String, reason: String },
    /// A witness that cannot be read, or that is the witness of another
    /// export.
    Witness(String),
}

impl fmt::Display for InputError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InputError::Read { path, reason } => write!(f, "{path} cannot be read: {reason}"),
            InputError::Xml { document, reason } => write!(f, "{document} is no XML: {reason}"),
            InputError::Document { document, reason } => {
                write!(f, "{document} is not read: {reason}")
            }
            InputError::Witness(reason) => write!(f, "the witness is not read: {reason}"),
        }
    }
}

impl std::error::Error for InputError {}

/// The unit name of the document whose file stem is `stem`, which names the
/// suite's modules and its witness: the stem in lower snake case, each `.`
/// and `-` an underscore and an underscore before each upper-case letter that
/// follows a lower-case letter or a digit (docs/SCE_TEST_PROFILE.md §9.5).
pub fn unit_name(stem: &str) -> String {
    if stem.is_empty() {
        return "empty".to_string();
    }
    let chars: Vec<char> = stem
        .chars()
        .map(|c| if c == '.' || c == '-' { '_' } else { c })
        .collect();
    let mut name = String::with_capacity(chars.len() + 4);
    for (position, &c) in chars.iter().enumerate() {
        if position > 0 && c.is_ascii_uppercase() {
            let before = chars[position - 1];
            if before.is_ascii_lowercase() || before.is_ascii_digit() {
                name.push('_');
            }
        }
        name.push(c);
    }
    name.to_lowercase()
}

/// The stem of the file `name` names: its last segment without its
/// extension.
pub(crate) fn stem(name: &str) -> &str {
    let file = name.rsplit('/').next().unwrap_or(name);
    file.rsplit_once('.').map_or(file, |(stem, _)| stem)
}

/// The directory of the suite's document and the document's file name.
fn located(suite: &Path) -> Result<(PathBuf, String), InputError> {
    let file = suite
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| InputError::Read {
            path: suite.display().to_string(),
            reason: "it names no file".to_string(),
        })?;
    let directory = suite.parent().map(Path::to_path_buf).unwrap_or_default();
    Ok((directory, file.to_string()))
}

/// Validate `export`, the export of the suite whose document is `suite`.
pub fn validate(suite: &Path, export: &Export<'_>) -> Result<Vec<Violation>, InputError> {
    let (directory, file) = located(suite)?;
    let documents = Documents::read(&Directory::new(directory), &file)?;
    check(&documents, export)
}

/// Validate the export the directory `export_dir` holds of the suite whose
/// document is `suite`: `S_Types.ttcn`, `S.ttcn` and `S.witness.json`, `S`
/// the suite's unit name.
pub fn validate_directory(suite: &Path, export_dir: &Path) -> Result<Vec<Violation>, InputError> {
    let (directory, file) = located(suite)?;
    let unit = unit_name(stem(&file));
    let read = |name: String| {
        let path = export_dir.join(name);
        fs::read_to_string(&path).map_err(|error| InputError::Read {
            path: path.display().to_string(),
            reason: error.to_string(),
        })
    };
    let types_module = read(format!("{unit}_Types.ttcn"))?;
    let behaviour_module = read(format!("{unit}.ttcn"))?;
    let witness = read(format!("{unit}.witness.json"))?;
    let documents = Documents::read(&Directory::new(directory), &file)?;
    check(
        &documents,
        &Export {
            types_module: &types_module,
            behaviour_module: &behaviour_module,
            witness: &witness,
        },
    )
}

/// The violations of `export` against `documents`, read already. A module
/// that does not parse is one `parse` violation, and its places are held to
/// nothing else.
pub fn check(documents: &Documents, export: &Export<'_>) -> Result<Vec<Violation>, InputError> {
    let witness = witness::read(export.witness)?;
    let suite = unit_name(stem(&documents.suite));
    if witness.suite != suite {
        return Err(InputError::Witness(format!(
            "it is the witness of the suite '{}', and the suite read is '{suite}'",
            witness.suite
        )));
    }
    let types_module = format!("{suite}_Types");
    let modules_named = witness
        .modules
        .iter()
        .map(String::as_str)
        .eq([types_module.as_str(), suite.as_str()]);
    if !modules_named {
        return Err(InputError::Witness(format!(
            "it names the modules {:?}, and the export of '{suite}' writes [\"{types_module}\", \"{suite}\"]",
            witness.modules
        )));
    }

    let mut violations = Vec::new();
    let mut parsed = Vec::new();
    for (expected, text) in [
        (&types_module, export.types_module),
        (&suite, export.behaviour_module),
    ] {
        match ttcn3::parse(text) {
            Ok(module) if module.name == *expected => parsed.push(module),
            Ok(module) => violations.push(Violation::Parse {
                module: expected.clone(),
                line: 1,
                message: format!(
                    "the module is named '{}', where the export of the suite names it '{expected}'",
                    module.name
                ),
            }),
            Err(error) => violations.push(Violation::Parse {
                module: expected.clone(),
                line: error.line,
                message: error.message,
            }),
        }
    }
    let places: Vec<coverage::ModulePlaces> = parsed
        .iter()
        .map(|module| coverage::ModulePlaces {
            module: module.name.clone(),
            places: ttcn3::places(module),
        })
        .collect();
    violations.extend(coverage::places(&places, &witness));
    violations.extend(coverage::entries(documents, &witness));
    let modules: Vec<&ttcn3::ast::Module> = parsed.iter().collect();
    violations.extend(form::check(documents, &modules, &witness));
    // A behaviour module that did not parse holds no order: its parse
    // violation stands for it.
    if let Some(behaviour) = parsed.iter().find(|module| module.name == suite) {
        violations.extend(order::check(documents, behaviour, &witness));
    }
    Ok(violations)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A unit name is the stem in lower snake case, as every unit of a
    /// document is named.
    #[test]
    fn a_unit_name_is_the_stem_in_lower_snake_case() {
        assert_eq!(unit_name("job_suite"), "job_suite");
        assert_eq!(unit_name("test-suite"), "test_suite");
        assert_eq!(unit_name("JobSuite"), "job_suite");
        assert_eq!(unit_name("suite2Fast"), "suite2_fast");
        assert_eq!(unit_name("a.b"), "a_b");
        assert_eq!(unit_name(""), "empty");
    }

    #[test]
    fn a_stem_is_the_file_without_its_extension() {
        assert_eq!(stem("job_suite.scxml"), "job_suite");
        assert_eq!(stem("parts/test-suite.scxml"), "test-suite");
        assert_eq!(stem("plain"), "plain");
    }

    const SUITE: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" sce:kind="test-suite" name="s"/>"#;

    const WITNESS: &str = r#"{"schema": "sce-ttcn3-witness.v1", "suite": "s", "modules": ["s_Types", "s"],
      "entries": [
        {"construct": {"document": "s.scxml", "node": "scxml", "line": 1}, "rule": "E1",
         "produced": [{"module": "s_Types", "path": [{"type": "uint8"}]}]},
        {"construct": {"document": "s.scxml", "node": "scxml", "line": 1}, "rule": "E15",
         "produced": [{"module": "s", "path": [{"control": "s"}]}]}
      ]}"#;

    const TYPES: &str = "// source-hash: 0123\nmodule s_Types {\ntype integer uint8 (0 .. 255);\n}";

    const BEHAVIOUR: &str = "module s {\nimport from s_Types all;\ncontrol {\n}\n}";

    fn suite() -> Documents {
        let files =
            documents::tests::Memory(std::collections::BTreeMap::from([("s.scxml", SUITE)]));
        Documents::read(&files, "s.scxml").expect("documents")
    }

    fn export() -> Export<'static> {
        Export {
            types_module: TYPES,
            behaviour_module: BEHAVIOUR,
            witness: WITNESS,
        }
    }

    #[test]
    fn an_export_its_witness_names_whole_has_no_violation() {
        assert_eq!(check(&suite(), &export()), Ok(Vec::new()));
    }

    /// The witness of another suite, or of the modules in another order, is
    /// no witness of this export.
    #[test]
    fn a_witness_of_another_export_is_not_read() {
        let other = WITNESS.replace(r#""suite": "s""#, r#""suite": "t""#);
        let found = check(
            &suite(),
            &Export {
                witness: &other,
                ..export()
            },
        );
        assert!(matches!(found, Err(InputError::Witness(_))), "{found:?}");
        let swapped = WITNESS.replace(r#"["s_Types", "s"]"#, r#"["s", "s_Types"]"#);
        let found = check(
            &suite(),
            &Export {
                witness: &swapped,
                ..export()
            },
        );
        assert!(matches!(found, Err(InputError::Witness(_))), "{found:?}");
    }

    /// A module that does not parse, or is named otherwise than the export
    /// names it, is one `parse` violation, at its row, and its locations are
    /// held to nothing else.
    #[test]
    fn a_module_that_does_not_parse_is_one_violation() {
        let broken = BEHAVIOUR.replace("control {", "control {\n  while (true) {}");
        let found = check(
            &suite(),
            &Export {
                behaviour_module: &broken,
                ..export()
            },
        )
        .expect("checked");
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(
            matches!(&found[0], Violation::Parse { module, line: 4, .. } if module == "s"),
            "{found:?}"
        );

        let renamed = BEHAVIOUR.replace("module s {", "module t {");
        let found = check(
            &suite(),
            &Export {
                behaviour_module: &renamed,
                ..export()
            },
        )
        .expect("checked");
        let codes: Vec<&str> = found.iter().map(Violation::code).collect();
        assert_eq!(codes, ["parse"]);
    }
}

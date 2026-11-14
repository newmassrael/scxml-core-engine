// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The corpus of the TTCN-3 conformance lane (docs/SCE_TTCN3_LANE.md §12): its
//! suites, where each one's recorded traces and expected logs are, and what its
//! documents say about the runs, read from the documents themselves: the test
//! cases a suite executes, and the record types its mapped ports carry.
//!
//! The documents are read as XML, by the imports they name, and not by the
//! parser: the tests that ask this hold the corpus to what it must hold, and
//! the parser's own tests hold each document to the profile's rules.

#![allow(dead_code)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// The namespace of every `sce:` element and attribute.
const SCE: &str = "http://sce.dev/ext";

/// One suite of the corpus.
pub struct CorpusSuite {
    /// The suite's document, relative to the `sce-build` directory.
    pub document: &'static str,
    /// The directory of its runs, relative to the `sce-build` directory: its
    /// `traces/` and `expected_logs/`, `<test case>.json` each (§10.2, §12).
    pub runs: &'static str,
    /// The world that selects every test case of the suite, the features its
    /// runner is given (`--world`, §4).
    pub world: &'static [&'static str],
}

/// The suites of the corpus, in the order §12.1 names them.
pub const CORPUS: &[CorpusSuite] = &[
    CorpusSuite {
        document: "tests/fixtures/test_profile/runtime/runtime_suite.scxml",
        runs: "tests/fixtures/test_profile/runtime",
        world: &["ptcs", "removal", "guard"],
    },
    CorpusSuite {
        document: "tests/fixtures/test_profile/boundary/boundary_suite.scxml",
        runs: "tests/fixtures/test_profile/runs/boundary_suite",
        world: &[],
    },
    CorpusSuite {
        document: "tests/fixtures/test_profile/job_suite.scxml",
        runs: "tests/fixtures/test_profile/runs/job_suite",
        world: &["job-timeout", "job-retry"],
    },
    CorpusSuite {
        document: "tests/fixtures/ttcn3_export/orders_suite.scxml",
        runs: "tests/fixtures/test_profile/runs/orders_suite",
        world: &[],
    },
    CorpusSuite {
        document: "tests/fixtures/ttcn3_export/nested_suite.scxml",
        runs: "tests/fixtures/test_profile/runs/nested_suite",
        world: &[],
    },
];

/// The `sce-build` directory.
pub fn sce_build() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The directory of the codec documents of the corpus's mapped message types,
/// `<record>_codec.scxml` each (§12).
pub fn codecs_dir() -> PathBuf {
    sce_build().join("tests/fixtures/test_profile/codecs")
}

impl CorpusSuite {
    /// The suite's document.
    pub fn path(&self) -> PathBuf {
        sce_build().join(self.document)
    }

    /// The directory of its traces.
    pub fn traces(&self) -> PathBuf {
        sce_build().join(self.runs).join("traces")
    }

    /// The directory of its expected logs.
    pub fn expected_logs(&self) -> PathBuf {
        sce_build().join(self.runs).join("expected_logs")
    }

    /// The suite's unit name: the stem of its document.
    pub fn name(&self) -> String {
        unit_name(&self.path())
    }
}

/// A unit's name: the stem of its document's file in lower snake case
/// (docs/SCE_TEST_PROFILE.md §12.5), which names a record in a trace and a log.
pub fn unit_name(path: &Path) -> String {
    path.file_stem()
        .unwrap_or_else(|| panic!("{} names a file", path.display()))
        .to_string_lossy()
        .replace('-', "_")
        .to_lowercase()
}

/// The text of the document at `path`.
pub fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{} is read: {e}", path.display()))
}

/// Calls `with` on the root element of the document at `path`.
fn with_root<T>(path: &Path, with: impl FnOnce(roxmltree::Node) -> T) -> T {
    let text = read(path);
    let doc = roxmltree::Document::parse(&text)
        .unwrap_or_else(|e| panic!("{} is XML: {e}", path.display()));
    with(doc.root_element())
}

/// Whether `node` is the `sce:` element `name`.
fn is_sce(node: &roxmltree::Node, name: &str) -> bool {
    node.is_element() && node.tag_name().namespace() == Some(SCE) && node.tag_name().name() == name
}

/// The file the `<sce:import as="alias">` of the document at `path` names,
/// resolved against the document's directory.
fn import_of(path: &Path, root: &roxmltree::Node, alias: &str) -> PathBuf {
    let src = root
        .children()
        .find(|node| is_sce(node, "import") && node.attribute("as") == Some(alias))
        .and_then(|node| node.attribute("src"))
        .unwrap_or_else(|| panic!("{} imports nothing as {alias}", path.display()));
    path.parent()
        .expect("a document is in a directory")
        .join(src)
}

/// The test cases the suite at `suite` executes, in its order: each one's name
/// (the `name` of its document's root, which names its trace) and its document.
pub fn test_cases(suite: &Path) -> Vec<(String, PathBuf)> {
    with_root(suite, |root| {
        root.children()
            .filter(|node| is_sce(node, "execute"))
            .map(|execute| {
                let alias = execute
                    .attribute("test-case")
                    .unwrap_or_else(|| panic!("an execute of {} names its case", suite.display()));
                let case = import_of(suite, &root, alias);
                let name = with_root(&case, |case_root| {
                    case_root
                        .attribute("name")
                        .unwrap_or_else(|| panic!("{} names itself", case.display()))
                        .to_string()
                });
                (name, case)
            })
            .collect()
    })
}

/// The configuration the test case at `case` runs in.
pub fn configuration_of(case: &Path) -> PathBuf {
    with_root(case, |root| {
        let alias = root
            .attribute((SCE, "configuration"))
            .unwrap_or_else(|| panic!("{} names its configuration", case.display()));
        import_of(case, &root, alias)
    })
}

/// The record types the system ports of the configuration at `configuration`
/// carry, in both directions, by name, each with its event schema. Every system
/// port is mapped, since every port takes part in exactly one connect or map
/// (docs/SCE_TEST_PROFILE.md §11.6).
pub fn mapped_types_of(configuration: &Path) -> BTreeMap<String, PathBuf> {
    let port_types: Vec<PathBuf> = with_root(configuration, |root| {
        root.children()
            .filter(|node| is_sce(node, "system-port"))
            .map(|port| {
                let alias = port.attribute("type").unwrap_or_else(|| {
                    panic!(
                        "a system port of {} names its type",
                        configuration.display()
                    )
                });
                import_of(configuration, &root, alias)
            })
            .collect()
    });
    let mut types = BTreeMap::new();
    for port_type in port_types {
        with_root(&port_type, |root| {
            for message in root.children().filter(|node| is_sce(node, "message")) {
                let alias = message.attribute("type").unwrap_or_else(|| {
                    panic!("a message of {} names its type", port_type.display())
                });
                let schema = import_of(&port_type, &root, alias);
                types.insert(unit_name(&schema), schema);
            }
        });
    }
    types
}

/// The record types the mapped ports of every test case of the suite at
/// `suite` carry, by name, each with its event schema.
pub fn mapped_types(suite: &Path) -> BTreeMap<String, PathBuf> {
    test_cases(suite)
        .iter()
        .flat_map(|(_, case)| mapped_types_of(&configuration_of(case)))
        .collect()
}

/// The fields of the record of the event schema at `schema`, in declared order:
/// each one's id and its `sce:type` as written.
pub fn fields(schema: &Path) -> Vec<(String, String)> {
    with_root(schema, |root| {
        root.children()
            .filter(|node| node.is_element() && node.tag_name().name() == "datamodel")
            .flat_map(|datamodel| datamodel.children())
            .filter(|node| node.is_element() && node.tag_name().name() == "data")
            .map(|data| {
                let id = data
                    .attribute("id")
                    .unwrap_or_else(|| panic!("a field of {} has an id", schema.display()));
                let ty = data
                    .attribute((SCE, "type"))
                    .unwrap_or_else(|| panic!("the field {id} of {} has a type", schema.display()));
                (id.to_string(), ty.to_string())
            })
            .collect()
    })
}

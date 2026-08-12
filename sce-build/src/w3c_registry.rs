// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML statechart conformance registry.
//
// Deserializes tests/w3c/conformance/fixtures.json — which upstream tests
// this repository runs, and which harness each one needs — into a typed
// model that `sce-codegen generate-w3c` renders from and
// `sce-codegen list-fixtures` enumerates.
//
// The registry used to be `tests/CMakeLists.txt`, read back with a regex
// over `sce_generate_static_w3c_test(...)` macro calls. Three consumers
// parsed that build script as data (this generator, the visualizer's
// test-list generator, and CMake itself), and a repository that vendors
// SCE without using CMake could not enumerate the fixture set at all —
// nor even resolve the project root, which was probed by looking for
// `tests/CMakeLists.txt` on disk. The forge side had already settled the
// shape this file mirrors: a JSON catalog as the source of truth, with
// `list-fixtures` handing plain text to build systems that have no JSON
// parser (CMake, Gradle, Bash, pytest all consume it that way).
//
// Per-test prose is deliberately NOT here. `resources/<id>/metadata.txt`
// is upstream W3C data with its own description and spec section, and
// the generator reads it directly; duplicating it into this catalog
// would create a second answer to the same question.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde::Deserialize;

/// Repository-relative location of the registry.
///
/// Callers resolve it against the project root rather than hardcoding a
/// path, so a vendored tree that places the repository elsewhere still
/// finds it.
pub const W3C_REGISTRY_RELATIVE_PATH: &str = "tests/w3c/conformance/fixtures.json";

/// The harness a fixture names when the catalog entry omits one.
///
/// The default lives here rather than in each entry so a reader of the
/// catalog never has to know it, and so adding a fixture is one line.
pub const DEFAULT_HARNESS: &str = "simple";

/// One registered upstream test.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct W3cFixture {
    /// Upstream W3C test id, optionally variant-suffixed (`403a`).
    pub id: String,
    /// Key into [`W3cRegistry::harnesses`].
    #[serde(default = "default_harness")]
    pub harness: String,
    /// This repository's curation note, shown by `generate-w3c --list`.
    #[serde(default)]
    pub summary: String,
}

fn default_harness() -> String {
    DEFAULT_HARNESS.to_string()
}

/// The parsed catalog.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct W3cRegistry {
    #[serde(rename = "$schema", default)]
    pub schema: Option<String>,
    pub version: u32,
    #[serde(default)]
    pub description: String,
    /// Every harness a fixture may name, mapped to what the runner does
    /// differently for it. Closed set: a fixture naming a key absent
    /// here is rejected at load.
    pub harnesses: BTreeMap<String, String>,
    pub fixtures: Vec<W3cFixture>,
}

/// Why a registry could not be loaded.
///
/// A plain enum rather than a `ForgeError` variant: this is a repository
/// input read by a batch subcommand, not a document on the diagnostic
/// wire surface, and routing it there would put a build-input mistake
/// into the `--error-format=json` stream consumers gate on.
#[derive(Debug)]
pub enum W3cRegistryError {
    Read {
        path: String,
        source: std::io::Error,
    },
    Parse {
        path: String,
        message: String,
    },
    /// The catalog is structurally valid JSON but says something a
    /// consumer cannot act on.
    Invalid {
        path: String,
        message: String,
    },
}

impl std::fmt::Display for W3cRegistryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            W3cRegistryError::Read { path, source } => {
                write!(f, "cannot read W3C conformance registry {path}: {source}")
            }
            W3cRegistryError::Parse { path, message } => {
                // The registry moved off `tests/CMakeLists.txt`, and
                // that build script is still a real file — so the
                // caller who passes it is the one working from stale
                // instructions, and a bare parse error at column 1
                // leaves them nowhere to go. Naming the catalog is the
                // difference between reporting a failure and pointing
                // at the fix.
                write!(
                    f,
                    "W3C conformance registry {path} is not valid: {message} \
                     (the registry is the JSON catalog at \
                     {W3C_REGISTRY_RELATIVE_PATH} under the project root)"
                )
            }
            W3cRegistryError::Invalid { path, message } => {
                write!(f, "W3C conformance registry {path}: {message}")
            }
        }
    }
}

impl W3cRegistry {
    /// Read and validate the catalog at `path`.
    ///
    /// Validation is hand-rolled rather than delegated to the sibling
    /// JSON Schema for the same reason `conformance.rs` does it: the
    /// schema file is an editor aid that no build step consults, so a
    /// check that only ran there would not run at all. The two are held
    /// together by `w3c_registry_matches_the_schema_file`.
    pub fn load(path: &Path) -> Result<Self, W3cRegistryError> {
        let display = path.display().to_string();
        let text = std::fs::read_to_string(path).map_err(|source| W3cRegistryError::Read {
            path: display.clone(),
            source,
        })?;
        let registry: W3cRegistry =
            serde_json::from_str(&text).map_err(|e| W3cRegistryError::Parse {
                path: display.clone(),
                message: e.to_string(),
            })?;
        registry.validate(&display)?;
        Ok(registry)
    }

    fn validate(&self, path: &str) -> Result<(), W3cRegistryError> {
        let invalid = |message: String| W3cRegistryError::Invalid {
            path: path.to_string(),
            message,
        };
        if self.version != 1 {
            return Err(invalid(format!(
                "declares version {}, and this build reads version 1 only",
                self.version
            )));
        }
        if self.harnesses.is_empty() {
            return Err(invalid(
                "declares no harnesses, so every fixture's `harness` would be unresolvable"
                    .to_string(),
            ));
        }
        if self.fixtures.is_empty() {
            return Err(invalid(
                "registers no fixtures; a run against it would report success having \
                 verified nothing"
                    .to_string(),
            ));
        }
        // The default has to be one of the declared harnesses, or an
        // entry that omits `harness` would name something unresolvable
        // while looking well-formed.
        if !self.harnesses.contains_key(DEFAULT_HARNESS) {
            return Err(invalid(format!(
                "declares no `{DEFAULT_HARNESS}` harness, which is what an entry that \
                 omits `harness` resolves to"
            )));
        }
        let mut seen: BTreeSet<&str> = BTreeSet::new();
        for fixture in &self.fixtures {
            if fixture.id.is_empty() {
                return Err(invalid("a fixture declares an empty id".to_string()));
            }
            if !seen.insert(fixture.id.as_str()) {
                return Err(invalid(format!(
                    "registers id `{}` more than once; the later entry would silently \
                     replace the earlier one",
                    fixture.id
                )));
            }
            if !self.harnesses.contains_key(&fixture.harness) {
                let known: Vec<&str> = self.harnesses.keys().map(String::as_str).collect();
                return Err(invalid(format!(
                    "fixture `{}` names harness `{}`, which is not declared; known \
                     harnesses are {known:?}",
                    fixture.id, fixture.harness
                )));
            }
        }
        Ok(())
    }

    /// Fixtures in catalog order.
    pub fn fixtures(&self) -> &[W3cFixture] {
        &self.fixtures
    }

    /// Ids carrying `harness`, in catalog order. Empty when no fixture
    /// names it — a declared-but-unused harness is legal.
    pub fn ids_with_harness(&self, harness: &str) -> Vec<&str> {
        self.fixtures
            .iter()
            .filter(|f| f.harness == harness)
            .map(|f| f.id.as_str())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo_root() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("sce-build has a parent")
            .to_path_buf()
    }

    fn committed_registry_path() -> std::path::PathBuf {
        repo_root().join(W3C_REGISTRY_RELATIVE_PATH)
    }

    fn load_committed() -> W3cRegistry {
        W3cRegistry::load(&committed_registry_path()).expect("committed registry loads")
    }

    /// Lower bound on the registered set.
    ///
    /// Without it a loader bug that returned an empty catalog would let
    /// every assertion below pass over nothing.
    const MIN_REGISTERED_FIXTURES: usize = 150;

    #[test]
    fn the_committed_registry_loads_and_is_not_trivial() {
        let registry = load_committed();
        assert_eq!(registry.version, 1);
        assert!(
            registry.fixtures().len() >= MIN_REGISTERED_FIXTURES,
            "only {} fixture(s) registered; expected at least \
             {MIN_REGISTERED_FIXTURES}, so a clean load proves something",
            registry.fixtures().len(),
        );
        assert!(
            registry.harnesses.contains_key(DEFAULT_HARNESS),
            "the default harness must be declared",
        );
    }

    /// Every harness the catalog declares is used, and every harness used
    /// is declared.
    ///
    /// The second direction is enforced at load; this pins the first, so
    /// a harness that stops being used cannot sit in the map describing
    /// behaviour no fixture asks for.
    #[test]
    fn declared_harnesses_and_used_harnesses_are_the_same_set() {
        let registry = load_committed();
        let declared: BTreeSet<&str> = registry.harnesses.keys().map(String::as_str).collect();
        let used: BTreeSet<&str> = registry
            .fixtures()
            .iter()
            .map(|f| f.harness.as_str())
            .collect();
        assert_eq!(
            declared, used,
            "declared harnesses and the harnesses fixtures name must agree",
        );
    }

    #[test]
    fn a_duplicate_id_is_refused() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("fixtures.json");
        std::fs::write(
            &path,
            r#"{"version":1,"harnesses":{"simple":"x"},
                "fixtures":[{"id":"144"},{"id":"144"}]}"#,
        )
        .expect("write");
        let err = W3cRegistry::load(&path).expect_err("a repeated id must be refused");
        let msg = err.to_string();
        assert!(
            msg.contains("more than once"),
            "the diagnostic must name the duplication: {msg}",
        );
    }

    #[test]
    fn an_undeclared_harness_is_refused() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("fixtures.json");
        std::fs::write(
            &path,
            r#"{"version":1,"harnesses":{"simple":"x"},
                "fixtures":[{"id":"144","harness":"nosuch"}]}"#,
        )
        .expect("write");
        let err = W3cRegistry::load(&path).expect_err("an unknown harness must be refused");
        let msg = err.to_string();
        assert!(
            msg.contains("nosuch") && msg.contains("simple"),
            "the diagnostic must name the bad key and the known ones: {msg}",
        );
    }

    #[test]
    fn an_empty_catalog_is_refused() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("fixtures.json");
        std::fs::write(
            &path,
            r#"{"version":1,"harnesses":{"simple":"x"},"fixtures":[]}"#,
        )
        .expect("write");
        let err = W3cRegistry::load(&path).expect_err("an empty catalog must be refused");
        assert!(
            err.to_string().contains("verified nothing"),
            "the diagnostic must say why empty is not acceptable: {err}",
        );
    }

    #[test]
    fn an_entry_without_a_harness_takes_the_default() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("fixtures.json");
        std::fs::write(
            &path,
            r#"{"version":1,"harnesses":{"simple":"x"},"fixtures":[{"id":"144"}]}"#,
        )
        .expect("write");
        let registry = W3cRegistry::load(&path).expect("loads");
        assert_eq!(registry.fixtures()[0].harness, DEFAULT_HARNESS);
    }

    /// An unreadable version is refused rather than read as version 1.
    #[test]
    fn a_future_version_is_refused() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("fixtures.json");
        std::fs::write(
            &path,
            r#"{"version":2,"harnesses":{"simple":"x"},"fixtures":[{"id":"144"}]}"#,
        )
        .expect("write");
        let err = W3cRegistry::load(&path).expect_err("a future version must be refused");
        assert!(err.to_string().contains("version 1 only"), "{err}");
    }

    /// The hand-rolled validation and the sibling JSON Schema describe
    /// the same shape.
    ///
    /// The schema is an editor aid no build step consults, so nothing
    /// else stops it from drifting into fiction. Checked by running the
    /// committed catalog through it: if the two disagreed about a
    /// required field or an allowed value, the instance the loader
    /// accepts would fail here.
    #[test]
    fn w3c_registry_matches_the_schema_file() {
        let schema_path = repo_root().join("tests/w3c/conformance/fixtures.schema.json");
        let schema: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&schema_path).expect("read schema"))
                .expect("schema is JSON");
        let validator = jsonschema::JSONSchema::options()
            .with_draft(jsonschema::Draft::Draft7)
            .compile(&schema)
            .expect("schema compiles as draft-07");
        let instance: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(committed_registry_path()).expect("read registry"),
        )
        .expect("registry is JSON");
        let msgs: Vec<String> = match validator.validate(&instance) {
            Ok(()) => Vec::new(),
            Err(errors) => errors.map(|e| e.to_string()).collect(),
        };
        assert!(
            msgs.is_empty(),
            "the committed registry violates its own schema: {msgs:?}",
        );
    }
}

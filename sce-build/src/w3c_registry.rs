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

/// What a curation note must not contain: a statement of which spec
/// section a fixture targets.
///
/// That answer has one home. `resources/<id>/metadata.txt` carries the
/// upstream `specnum`, and `tools/mnemosyne-adoption/gen_verifies_catalog.py`
/// derives it into `docs/spec/scxml/.atomic/verifies-catalog.json`, which
/// the citation gate reads. Summaries restated it by hand, and measured on
/// 2026-09-13 the copy contradicted `specnum` in 74 of 202 entries — not
/// finer-grained, a different section — while nothing noticed, because
/// nothing read the copy as data.
///
/// Matches `W3C 6.2`, `W3C SCXML 3.12.1`, `W3C SCXML C.2` and a
/// `§scxml-` citation; leaves `W3C conformance`, `W3C C++` and `BasicHTTP`
/// alone. A lettered label must carry a dotted part (`C.2`, `B.1`): every
/// appendix label the registry ever held does, and a bare letter would
/// read `W3C C++` as appendix C. The sibling JSON Schema carries the same
/// pattern under `summary.not`, held to this constant by
/// `the_schema_refuses_what_the_loader_refuses`.
pub const SPEC_SECTION_STATEMENT: &str =
    r"\bW3C(\s+SCXML)?\s+([0-9]+(\.[0-9]+)*|[A-H](\.[0-9]+)+)\b|§scxml-";

/// The spec-section statement a curation note carries, if it carries one —
/// a registry summary, or the brief of a fixture's AOT test header.
fn stated_spec_section(note: &str) -> Option<String> {
    let pattern = regex::Regex::new(SPEC_SECTION_STATEMENT)
        .expect("SPEC_SECTION_STATEMENT is a valid regular expression");
    pattern.find(note).map(|m| m.as_str().to_string())
}

/// Repository-relative directory holding one C++ AOT test header,
/// `Test<id>.h`, per registered fixture.
pub const W3C_AOT_HEADER_RELATIVE_DIR: &str = "tests/w3c/aot_tests";

/// The paragraph a C++ doc comment opens with `@brief`, joined into one line.
///
/// Doxygen's own boundary: the paragraph runs until a blank comment line,
/// the next `@` command or the end of the comment. That boundary is what
/// separates the brief — the one sentence saying what a test is — from the
/// body below it, where a header cites the other sections a test touches,
/// and where those citations are expected rather than refused.
pub fn doc_brief(text: &str) -> Option<String> {
    let lines: Vec<&str> = text.lines().collect();
    let at = lines.iter().position(|line| line.contains("@brief"))?;
    let (_, opening) = lines[at].split_once("@brief")?;
    let mut paragraph = vec![opening.split("*/").next().unwrap_or_default().trim()];
    if !lines[at].contains("*/") {
        for line in &lines[at + 1..] {
            let trimmed = line.trim_start();
            if trimmed.starts_with("*/") {
                break;
            }
            let Some(body) = trimmed
                .strip_prefix('*')
                .or_else(|| trimmed.strip_prefix("//"))
            else {
                break;
            };
            let body = body
                .trim_start_matches(['/', '!'])
                .split("*/")
                .next()
                .unwrap_or_default()
                .trim();
            if body.is_empty() || body.starts_with('@') {
                break;
            }
            paragraph.push(body);
            if line.contains("*/") {
                break;
            }
        }
    }
    Some(paragraph.join(" "))
}

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
    /// A registered fixture's AOT test header is missing, or its brief
    /// states the spec section the fixture targets.
    AotHeaders {
        dir: String,
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
            W3cRegistryError::AotHeaders { dir, message } => {
                write!(f, "W3C AOT test headers under {dir}: {message}")
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
            if let Some(stated) = stated_spec_section(&fixture.summary) {
                return Err(invalid(format!(
                    "fixture `{}` has a summary stating `{stated}`. The section a fixture \
                     targets is `specnum` in resources/<id>/metadata.txt, derived into \
                     docs/spec/scxml/.atomic/verifies-catalog.json; a copy in the summary \
                     is a second answer to that question. Drop the section from the summary",
                    fixture.id
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

    /// Refuse every registered fixture whose AOT test header states, in its
    /// brief, the spec section the fixture targets — or that has no header.
    ///
    /// The rule a summary is held to, for the same reason: that section has
    /// one home, `specnum` in `resources/<id>/metadata.txt`. Headers
    /// restated it by hand, and measured on 2026-09-15 the brief of 175 of
    /// 202 headers stated a section while 82 of those contradicted
    /// `specnum`. It went unnoticed the way the summaries' copy did, because
    /// nothing read it as data.
    ///
    /// Only the brief is read ([`doc_brief`]). The body below it cites the
    /// other sections a test touches, which is what those citations are
    /// for, so a check refusing every mention would delete them. Every
    /// offender is named in one error, so one run shows the whole repair.
    pub fn check_aot_header_briefs(&self, header_dir: &Path) -> Result<(), W3cRegistryError> {
        let mut offenders: Vec<String> = Vec::new();
        for fixture in &self.fixtures {
            let header = header_dir.join(format!("Test{}.h", fixture.id));
            match std::fs::read_to_string(&header) {
                Ok(text) => {
                    if let Some(stated) = doc_brief(&text).as_deref().and_then(stated_spec_section)
                    {
                        offenders
                            .push(format!("{}: the brief states `{stated}`", header.display()));
                    }
                }
                Err(e) => offenders.push(format!(
                    "{}: cannot be read ({e}), and every registered fixture needs one",
                    header.display()
                )),
            }
        }
        if offenders.is_empty() {
            return Ok(());
        }
        Err(W3cRegistryError::AotHeaders {
            dir: header_dir.display().to_string(),
            message: format!(
                "{} header(s) refused. The section a fixture targets is `specnum` in \
                 resources/<id>/metadata.txt, derived into \
                 docs/spec/scxml/.atomic/verifies-catalog.json; a brief restating it is a \
                 second answer to that question. Drop the section from the brief and cite \
                 the sections the test touches in the body below it:\n  {}",
                offenders.len(),
                offenders.join("\n  ")
            ),
        })
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

    /// A summary stating a spec section is refused, whichever of the
    /// spellings the committed registry actually carried it in.
    #[test]
    fn a_summary_stating_a_spec_section_is_refused() {
        for summary in [
            "W3C SCXML 3.12.1: executable content executes in document order",
            "invalid target raises error.execution (W3C SCXML 6.2)",
            "BasicHTTP param encoding (W3C C.2 AOT)",
            "executable content execution order (§scxml-D-executeTransitionContent)",
        ] {
            let dir = tempfile::tempdir().expect("tempdir");
            let path = dir.path().join("fixtures.json");
            let body = serde_json::json!({
                "version": 1,
                "harnesses": {"simple": "x"},
                "fixtures": [{"id": "158", "summary": summary}],
            });
            std::fs::write(&path, body.to_string()).expect("write");
            let err = W3cRegistry::load(&path)
                .expect_err("a summary stating a spec section must be refused");
            let msg = err.to_string();
            assert!(
                msg.contains("`158`") && msg.contains("verifies-catalog.json"),
                "the diagnostic must name the fixture and the one place the answer \
                 lives: {msg}",
            );
        }
    }

    /// The control for the case above: a note that mentions W3C, SCXML or a
    /// dotted token without naming a section still loads. Without it the
    /// refusal could fire on every summary, and the case above would pass
    /// for that reason alone.
    #[test]
    fn a_summary_naming_no_section_loads() {
        for summary in [
            "W3C conformance, C++ Interpreter + AOT",
            "W3C C++ harness",
            "BasicHTTP event processor (optional)",
            "SCXML Event I/O Processor location field as send target",
            "_event.sendid field binding in error events; send idlocation attribute",
        ] {
            let dir = tempfile::tempdir().expect("tempdir");
            let path = dir.path().join("fixtures.json");
            let body = serde_json::json!({
                "version": 1,
                "harnesses": {"simple": "x"},
                "fixtures": [{"id": "158", "summary": summary}],
            });
            std::fs::write(&path, body.to_string()).expect("write");
            W3cRegistry::load(&path)
                .unwrap_or_else(|e| panic!("{summary:?} names no spec section and must load: {e}"));
        }
    }

    /// The editor schema refuses the same statement the loader does.
    ///
    /// Two checks, because one is not enough: the pattern strings being
    /// equal says nothing about whether the schema applies it, and a `not`
    /// placed at the wrong depth would compile and refuse nothing.
    #[test]
    fn the_schema_refuses_what_the_loader_refuses() {
        let schema_path = repo_root().join("tests/w3c/conformance/fixtures.schema.json");
        let schema: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&schema_path).expect("read schema"))
                .expect("schema is JSON");
        assert_eq!(
            schema
                .pointer("/properties/fixtures/items/properties/summary/not/pattern")
                .and_then(|p| p.as_str()),
            Some(SPEC_SECTION_STATEMENT),
            "fixtures.schema.json `summary.not.pattern` must equal \
             w3c_registry::SPEC_SECTION_STATEMENT",
        );
        let validator = jsonschema::JSONSchema::options()
            .with_draft(jsonschema::Draft::Draft7)
            .compile(&schema)
            .expect("schema compiles as draft-07");
        let valid = serde_json::json!({
            "version": 1,
            "harnesses": {"simple": "x"},
            "fixtures": [{"id": "158", "harness": "simple",
                          "summary": "executable content executes in document order"}],
        });
        assert!(
            validator.is_valid(&valid),
            "the control instance must be valid, or the refusal below proves nothing",
        );
        let mut stating = valid.clone();
        stating["fixtures"][0]["summary"] =
            serde_json::json!("W3C SCXML 3.12.1: executable content executes in document order");
        assert!(
            !validator.is_valid(&stating),
            "the schema accepted a summary stating a spec section",
        );
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

    /// A one-fixture registry (`158`) and a directory holding its header.
    fn registry_with_header(header: Option<&str>) -> (tempfile::TempDir, W3cRegistry) {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("fixtures.json");
        std::fs::write(
            &path,
            r#"{"version":1,"harnesses":{"simple":"x"},"fixtures":[{"id":"158"}]}"#,
        )
        .expect("write registry");
        if let Some(text) = header {
            std::fs::write(dir.path().join("Test158.h"), text).expect("write header");
        }
        let registry = W3cRegistry::load(&path).expect("loads");
        (dir, registry)
    }

    /// A brief stating a spec section is refused in every shape the
    /// committed headers carried one: leading, closing a sentence, two
    /// sections joined by a slash, lettered, and wrapped onto the brief's
    /// second line.
    #[test]
    fn a_brief_stating_a_spec_section_is_refused() {
        for (header, stated) in [
            (
                "/**\n * @brief W3C SCXML 3.12.1: executable content in document order\n */\n",
                "W3C SCXML 3.12.1",
            ),
            (
                "/**\n * @brief Basic delayed send (W3C SCXML 6.2 AOT)\n */\n",
                "W3C SCXML 6.2",
            ),
            (
                "/**\n * @brief W3C SCXML 3.6/3.4: default initial state\n */\n",
                "W3C SCXML 3.6",
            ),
            (
                "/**\n * @brief W3C SCXML C.2: BasicHTTP content element\n */\n",
                "W3C SCXML C.2",
            ),
            (
                "/**\n * @brief Send idlocation is stored where\n * W3C SCXML 6.2 names it\n */\n",
                "W3C SCXML 6.2",
            ),
            (
                "/// @brief W3C SCXML 5.10: system variables\n",
                "W3C SCXML 5.10",
            ),
        ] {
            let (dir, registry) = registry_with_header(Some(header));
            let err = registry
                .check_aot_header_briefs(dir.path())
                .expect_err("a brief stating a spec section must be refused");
            let msg = err.to_string();
            assert!(
                msg.contains("Test158.h") && msg.contains(stated) && msg.contains("metadata.txt"),
                "the diagnostic must name the header, what its brief states and the one \
                 place the answer lives: {msg}",
            );
        }
    }

    /// The control for the case above: a section cited in the body below
    /// the brief, or after a command that ends the brief, is what a header
    /// is for and passes. Without it the refusal could fire on every
    /// header, and the case above would pass for that reason alone.
    #[test]
    fn a_section_cited_below_the_brief_is_not_refused() {
        for header in [
            "/**\n * @brief W3C conformance test 158 on the W3C C++ harness\n *\n \
             * W3C SCXML 3.12.1: executable content executes in document order.\n \
             * W3C SCXML C.2 is not involved.\n */\n",
            "/**\n * @brief Raise ordering\n * @see W3C SCXML 4.2\n */\n",
            "/** @brief One-line brief */\n// W3C SCXML 6.2: cited outside the brief\n",
        ] {
            let (dir, registry) = registry_with_header(Some(header));
            registry
                .check_aot_header_briefs(dir.path())
                .unwrap_or_else(|e| panic!("{header:?} states no section in its brief: {e}"));
        }
    }

    #[test]
    fn a_registered_fixture_without_a_header_is_refused() {
        let (dir, registry) = registry_with_header(None);
        let err = registry
            .check_aot_header_briefs(dir.path())
            .expect_err("a missing header must be refused, not skipped");
        assert!(err.to_string().contains("Test158.h"), "{err}");
    }

    #[test]
    fn a_comment_with_no_brief_has_no_brief() {
        assert_eq!(
            doc_brief("/**\n * Just a comment\n */\nstruct X {};\n"),
            None
        );
    }

    /// Every committed AOT test header passes the check the build runs.
    ///
    /// The build runs `check-aot-briefs` only where CMake configures the W3C
    /// suite; this holds the committed tree to the same rule on every lane
    /// that runs this crate's tests.
    #[test]
    fn every_committed_aot_header_brief_states_no_section() {
        load_committed()
            .check_aot_header_briefs(&repo_root().join(W3C_AOT_HEADER_RELATIVE_DIR))
            .unwrap_or_else(|e| panic!("{e}"));
    }
}

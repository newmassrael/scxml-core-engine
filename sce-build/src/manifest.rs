// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

//! The stdout manifest — the single JSON line `sce-codegen` writes on
//! success.
//!
//! Lives here rather than beside the CLI for the same reason the
//! diagnostic, forge-AST, and sourcemap producers do: a wire surface's
//! shape and its schema-file lockstep guard belong together, and a
//! guard inside a `[[bin]]` target cannot be reached by the
//! cross-surface registry test. `SCE_WIRE_CONTRACTS.md` lists the
//! surface; `SCE_ERROR_CONTRACT.md` §10 defines the prose contract.

use serde::Serialize;

use crate::script_engine_analyzer::ScriptEngineCauseRecord;

/// Manifest schema version. Bumped only on a breaking shape change,
/// under the same policy that governs the error contract
/// (`SCE_ERROR_CONTRACT.md` §8). Additive field growth does not bump it.
pub const MANIFEST_SCHEMA_VERSION: u32 = 1;

/// Stability status of the manifest wire surface.
///
/// Pinned to the `x-sce-schema-status` header of
/// `schemas/sce-manifest.v1.schema.json` by
/// [`tests::schema_file_declares_status`], so the registry row in
/// `SCE_WIRE_CONTRACTS.md` cannot go stale against the schema file.
pub const MANIFEST_SCHEMA_STATUS: &str = "pre-release";

/// Which subcommand produced a manifest.
///
/// Consumers branch on this before reading subcommand-specific fields,
/// so the set is part of the wire contract rather than an internal
/// label. [`tests::json_schema_kind_enum_matches_rust_source_of_truth`]
/// pins it to the schema file's `kind.enum`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManifestKind {
    /// `sce-codegen generate` — artifacts were written.
    Generate,
    /// `sce-codegen check` — the same verdict, nothing written.
    Check,
    /// `sce-codegen orchestrate` — artifacts were written for a
    /// document set.
    ///
    /// Distinct from [`Self::Generate`] because the two answer the
    /// question about different units: `generate` reports one
    /// document's lowering, `orchestrate` reports a build whose
    /// artifacts come from several documents and whose
    /// `needs_script_engine` is the union over the set. A consumer that
    /// treated them as one kind would have to guess which.
    Orchestrate,
}

impl ManifestKind {
    /// The spelling that goes on the wire.
    pub fn as_str(self) -> &'static str {
        match self {
            ManifestKind::Generate => "generate",
            ManifestKind::Check => "check",
            ManifestKind::Orchestrate => "orchestrate",
        }
    }
}

/// Every manifest kind, in wire order. The source of truth the schema's
/// `kind.enum` is checked against — a new subcommand that emits a
/// manifest lands here and fails the lockstep test until the schema
/// names it too.
pub const ALL_MANIFEST_KINDS: &[ManifestKind] = &[
    ManifestKind::Generate,
    ManifestKind::Check,
    ManifestKind::Orchestrate,
];

/// One written file.
///
/// An object rather than a bare string so the shape can grow additively
/// (size, hash, artifact kind) without a version bump.
#[derive(Serialize)]
pub struct ArtifactEntry {
    pub path: String,
}

/// A W3C-spec rejection that produced stub files instead of generated
/// code. Absence means clean generation.
#[derive(Serialize)]
pub struct RejectedInfo {
    pub spec: &'static str,
    pub name: String,
}

/// Deploy declarations SCE records without acting on.
///
/// An object rather than a flat `static_analyzer` key because the spec
/// has further descriptive build-environment axes, and they belong
/// beside this one rather than each claiming a top-level manifest key.
#[derive(Serialize)]
pub struct DeployInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub static_analyzer: Option<&'static str>,
}

impl DeployInfo {
    /// `None` when nothing was declared, so the manifest omits the key
    /// entirely rather than carrying an empty object.
    pub fn from_facts(facts: Option<&crate::DeployFacts>) -> Option<Self> {
        let analyzer = facts?.static_analyzer?;
        Some(DeployInfo {
            static_analyzer: Some(analyzer.as_str()),
        })
    }
}

/// Whether one backend would generate the document.
///
/// `check` reports this per language because the two refusal axes are
/// not the same question: a `validation/*` refusal says the document is
/// wrong, while a `generate/unsupported-feature` refusal says only that
/// the backend cannot lower a construct the document is entitled to
/// use. A single exit code cannot distinguish them, so the manifest
/// names the axis instead of leaving the consumer to infer it.
#[derive(Serialize)]
pub struct LanguageVerdict {
    pub language: &'static str,
    pub status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

impl LanguageVerdict {
    /// Wire spelling for a backend that would generate the document.
    pub const STATUS_OK: &'static str = "ok";
    /// Wire spelling for a backend that would refuse it.
    pub const STATUS_REJECTED: &'static str = "rejected";

    pub fn ok(language: &'static str) -> Self {
        LanguageVerdict {
            language,
            status: Self::STATUS_OK,
            code: None,
        }
    }

    pub fn rejected(language: &'static str, code: String) -> Self {
        LanguageVerdict {
            language,
            status: Self::STATUS_REJECTED,
            code: Some(code),
        }
    }
}

/// The manifest itself.
///
/// Field order is the wire order. Optional fields are omitted rather
/// than emitted empty, so a manifest that predates a field stays
/// byte-identical once the field exists.
#[derive(Serialize)]
pub struct Manifest<'a> {
    pub v: u32,
    pub kind: &'static str,
    /// Commit of the generator that produced this record, or
    /// `"unknown"` on a build with no git checkout to read. Present so
    /// a build system capturing the manifest attributes its output
    /// without a second invocation and without a hand-maintained
    /// version sidecar.
    pub generator: &'static str,
    pub artifacts: Vec<ArtifactEntry>,
    pub needs_script_engine: bool,
    /// Omitted (not `[]`) on a pure-static machine.
    #[serde(skip_serializing_if = "<[_]>::is_empty")]
    pub script_engine_causes: &'a [ScriptEngineCauseRecord],
    /// Which driving entry point the emitted machine requires of its
    /// host: `true` means `Engine::tick()`, `false` means `step()` is
    /// enough.
    ///
    /// `tick()` is two mechanisms — it drains the delayed-send scheduler
    /// and ticks invoked child sessions — and
    /// `step()` performs neither. A machine that schedules a `<send
    /// delay>` / `<cancel>`, or that drives a session-bearing
    /// `<invoke>`, and is driven by `step()` alone therefore loses
    /// those events with no error and no diagnostic: the symptom is
    /// events that simply never arrive.
    ///
    /// Reported for the same reason as [`Self::needs_script_engine`] —
    /// both answer "what does the host have to supply?", and the
    /// generator settles both while compiling. Always present, so a
    /// consumer reads `false` as an answer rather than as a field it
    /// has to guess the absence of.
    pub needs_event_scheduler: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rejected: Option<RejectedInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deploy: Option<DeployInfo>,
    /// Present only for `kind = "check"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub languages: Option<Vec<LanguageVerdict>>,
}

impl<'a> Manifest<'a> {
    /// Serialise to the single line that goes on stdout.
    ///
    /// Returned rather than printed so the caller owns the stream and
    /// the tests can assert on the bytes without capturing stdout.
    pub fn to_line(&self) -> String {
        serde_json::to_string(self).expect("Manifest serialises; every field is owned or borrowed")
    }
}

/// Wire-format name for a language.
///
/// Re-exported from the codegen matrix so the manifest cannot grow a
/// third spelling table for the same six backends.
pub use crate::forge::codegen_matrix::language_wire_name;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mesh::deploy::StaticAnalyzer;

    const SCHEMA_BYTES: &str = include_str!("../../schemas/sce-manifest.v1.schema.json");

    fn schema() -> serde_json::Value {
        serde_json::from_str(SCHEMA_BYTES).expect("manifest schema is valid JSON")
    }

    /// The producer-side status constant and the schema-file header are
    /// one claim in two places; a flip that touches only one of them is
    /// the drift `SCE_WIRE_CONTRACTS.md` exists to prevent.
    #[test]
    fn schema_file_declares_status() {
        let declared = schema()["x-sce-schema-status"]
            .as_str()
            .expect("schema declares x-sce-schema-status")
            .to_string();
        assert_eq!(
            declared, MANIFEST_SCHEMA_STATUS,
            "schemas/sce-manifest.v1.schema.json x-sce-schema-status disagrees with \
             MANIFEST_SCHEMA_STATUS; SCE_WIRE_CONTRACTS.md requires one commit to move both",
        );
    }

    #[test]
    fn schema_version_matches_producer_constant() {
        let declared = schema()["properties"]["v"]["const"]
            .as_u64()
            .expect("schema pins v.const");
        assert_eq!(
            declared as u32, MANIFEST_SCHEMA_VERSION,
            "schema v.const disagrees with MANIFEST_SCHEMA_VERSION",
        );
    }

    /// A subcommand that starts emitting a manifest adds a
    /// [`ManifestKind`]; without this lockstep the schema keeps
    /// rejecting the new record while every producer-side test passes.
    #[test]
    fn json_schema_kind_enum_matches_rust_source_of_truth() {
        let schema = schema();
        let mut declared: Vec<String> = schema["properties"]["kind"]["enum"]
            .as_array()
            .expect("kind.enum is an array")
            .iter()
            .map(|v| {
                v.as_str()
                    .expect("kind enum member is a string")
                    .to_string()
            })
            .collect();
        declared.sort();
        let mut actual: Vec<String> = ALL_MANIFEST_KINDS
            .iter()
            .map(|k| k.as_str().to_string())
            .collect();
        actual.sort();
        assert_eq!(
            declared, actual,
            "schemas/sce-manifest.v1.schema.json kind.enum drifted from ALL_MANIFEST_KINDS",
        );
    }

    /// The per-language verdict names backends by their wire spelling;
    /// a seventh backend must reach the schema in the same commit.
    #[test]
    fn json_schema_language_enum_matches_rust_source_of_truth() {
        use crate::generator::Language;
        let schema = schema();
        let mut declared: Vec<String> = schema["definitions"]["languageVerdict"]["properties"]
            ["language"]["enum"]
            .as_array()
            .expect("languageVerdict.language.enum is an array")
            .iter()
            .map(|v| v.as_str().expect("language is a string").to_string())
            .collect();
        declared.sort();
        let mut actual: Vec<String> = Language::ALL
            .iter()
            .map(|l| language_wire_name(*l).to_string())
            .collect();
        actual.sort();
        assert_eq!(
            declared, actual,
            "manifest schema languageVerdict.language.enum drifted from Language wire names",
        );
    }

    #[test]
    fn json_schema_static_analyzer_enum_matches_rust_source_of_truth() {
        let schema = schema();
        let mut declared: Vec<String> = schema["definitions"]["deploy"]["properties"]
            ["static_analyzer"]["enum"]
            .as_array()
            .expect("static_analyzer.enum is an array")
            .iter()
            .map(|v| v.as_str().expect("analyzer is a string").to_string())
            .collect();
        declared.sort();
        let mut actual: Vec<String> = [StaticAnalyzer::PcLintPlus, StaticAnalyzer::Coverity]
            .iter()
            .map(|a| a.as_str().to_string())
            .collect();
        actual.sort();
        assert_eq!(
            declared, actual,
            "manifest schema static_analyzer.enum drifted from StaticAnalyzer",
        );
    }

    /// Schema violations for one record, as message strings.
    ///
    /// The instance is declared before the validator so the borrow the
    /// error iterator holds ends before the instance drops.
    fn schema_violations(line: &str) -> Vec<String> {
        let instance: serde_json::Value =
            serde_json::from_str(line).expect("manifest line is JSON");
        let schema_value = schema();
        let validator = jsonschema::JSONSchema::options()
            .with_draft(jsonschema::Draft::Draft7)
            .compile(&schema_value)
            .expect("manifest schema compiles as draft-07");
        // Bound to a local so the `Result`'s temporary — which carries
        // the borrow of `instance` — drops before `instance` does.
        let msgs: Vec<String> = match validator.validate(&instance) {
            Ok(()) => Vec::new(),
            Err(errors) => errors.map(|e| e.to_string()).collect(),
        };
        msgs
    }

    fn assert_valid(line: &str) {
        let msgs = schema_violations(line);
        assert!(
            msgs.is_empty(),
            "manifest instance violates its own schema: {msgs:?}\ninstance: {line}",
        );
    }

    fn assert_invalid(line: &str, why: &str) {
        assert!(
            !schema_violations(line).is_empty(),
            "schema must reject this record ({why}): {line}",
        );
    }

    /// The schema is only a contract if a produced record is checked
    /// against it. Structural key-presence assertions elsewhere cannot
    /// catch a type narrowing, a const mismatch, or a field the schema
    /// forbids.
    #[test]
    fn generate_manifest_instance_validates_against_schema() {
        let m = Manifest {
            v: MANIFEST_SCHEMA_VERSION,
            kind: ManifestKind::Generate.as_str(),
            generator: "deadbeefcafe",
            artifacts: vec![ArtifactEntry {
                path: "out/foo_sm.rs".to_string(),
            }],
            needs_script_engine: false,
            script_engine_causes: &[],
            needs_event_scheduler: false,
            rejected: None,
            deploy: None,
            languages: None,
        };
        assert_valid(&m.to_line());
    }

    #[test]
    fn rejected_generate_manifest_instance_validates_against_schema() {
        let m = Manifest {
            v: MANIFEST_SCHEMA_VERSION,
            kind: ManifestKind::Generate.as_str(),
            generator: "deadbeefcafe",
            artifacts: vec![ArtifactEntry {
                path: "out/foo_sm.h".to_string(),
            }],
            needs_script_engine: false,
            script_engine_causes: &[],
            needs_event_scheduler: false,
            rejected: Some(RejectedInfo {
                spec: "W3C SCXML 5.8",
                name: "untestable_doc".to_string(),
            }),
            deploy: Some(DeployInfo {
                static_analyzer: Some(StaticAnalyzer::Coverity.as_str()),
            }),
            languages: None,
        };
        assert_valid(&m.to_line());
    }

    #[test]
    fn check_manifest_instance_validates_against_schema() {
        let m = Manifest {
            v: MANIFEST_SCHEMA_VERSION,
            kind: ManifestKind::Check.as_str(),
            generator: "deadbeefcafe",
            artifacts: Vec::new(),
            needs_script_engine: false,
            script_engine_causes: &[],
            needs_event_scheduler: false,
            rejected: None,
            deploy: None,
            languages: Some(vec![
                LanguageVerdict::ok("rust"),
                LanguageVerdict::rejected("cpp", "generate/unsupported-feature".to_string()),
            ]),
        };
        assert_valid(&m.to_line());
    }

    /// A malformed record must be rejected, otherwise the positive
    /// cases above prove only that the validator accepts everything.
    #[test]
    fn schema_rejects_an_unknown_manifest_kind() {
        assert_invalid(
            r#"{"v":1,"kind":"generate-w3c","generator":"deadbeefcafe","artifacts":[],"needs_script_engine":false,"needs_event_scheduler":false}"#,
            "kind outside ALL_MANIFEST_KINDS",
        );
    }

    #[test]
    fn schema_rejects_a_missing_required_field() {
        assert_invalid(
            r#"{"v":1,"kind":"generate","generator":"deadbeefcafe","artifacts":[]}"#,
            "needs_script_engine absent",
        );
    }

    #[test]
    fn schema_rejects_an_unknown_language_in_a_check_verdict() {
        assert_invalid(
            r#"{"v":1,"kind":"check","generator":"deadbeefcafe","artifacts":[],"needs_script_engine":false,"needs_event_scheduler":false,"languages":[{"language":"haskell","status":"ok"}]}"#,
            "language outside the backend set",
        );
    }
}

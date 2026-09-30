// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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

use crate::host_processor_analyzer::HostProcessorCauseRecord;
use crate::script_engine_analyzer::ScriptEngineCauseRecord;

/// Manifest schema version. Bumped only on a breaking shape change,
/// under the same policy that governs the error contract
/// (`SCE_ERROR_CONTRACT.md` §8). Additive field growth does not bump it.
pub const MANIFEST_SCHEMA_VERSION: u32 = 1;

/// The language a machine's expressions are actually evaluated in.
///
/// A document declares `datamodel="ecmascript"`, and a consumer reasonably
/// reads that attribute as the answer. It is not — but not for the reason
/// this comment used to give, which was "SCE ships no ECMAScript engine:
/// the expression is parsed at generation time and emitted as **Lua**".
/// Measured 2026-08-27 (`docs/SCE_LUA_TRANSLATION_SEAM.md`): SCE ships two
/// ECMAScript engines and both are defaults — QuickJS on C++
/// (`SCE_SCRIPT_ENGINE=quickjs`) and Rhino on Kotlin
/// (`W3CTestBase.DEFAULT_ENGINE`) — and on those two backends the emitted
/// machine hands the engine the author's ECMAScript rather than Lua.
///
/// What is true is narrower and is the reason this field exists: the answer
/// is the TARGET BACKEND's, not the document's. Four backends receive Lua
/// that `sce-build/src/ecmascript/` lowered at generation time
/// (`docs/SCE_ACCEPTED_SUBSET.md` §B.2, "What `ecmascript` currently
/// means"); two receive source.
///
/// Reported on the manifest because that gap cost a downstream consumer a
/// round in 2026-08: their guards were ECMAScript source, they worked, and
/// nothing on any surface said which language would run them — so the
/// consumer had to probe an expression before daring to write it. The
/// prose has said this for a while; what was missing was a place a
/// program reads.
///
/// ⚠ It IS per-backend, and this constant used to deny that. The doc here
/// read "Not per-backend: the lowering happens in `sce-build`, before any
/// backend renders, so every language's generated machine evaluates the
/// same Lua", and `script_engine_language` was hard-coded to Lua for every
/// target. Measured 2026-08-27 and written up in
/// `docs/SCE_LUA_TRANSLATION_SEAM.md`: four backends do receive lowered Lua
/// (`to_lua_guard` / `to_lua_expr` in their templates), and **C++ and Kotlin
/// do not** — they hand the engine the author's ECMAScript source, which is
/// why each carries a runtime rewriter. So a C++ or Kotlin host obeying the
/// manifest supplied a Lua engine for a machine that speaks ECMAScript to
/// it, and both of those backends default to an ECMAScript engine
/// (`SCE_SCRIPT_ENGINE=quickjs`, `W3CTestBase.DEFAULT_ENGINE="rhino"`).
///
/// Which Lua differs among the four (`docs/SCE_ACCEPTED_SUBSET.md` records
/// the go-lua standard-library divergence), and that is a separate question
/// from which language the source was translated into.
///
/// The mapping lives on [`crate::generator::Language::script_engine_language`]
/// so it has one home, and a test binds it to the templates.
pub const SCRIPT_ENGINE_LANGUAGE_LUA: &str = "lua";

/// The other answer: the engine is handed the author's ECMAScript.
///
/// Spelled here rather than at the two call sites so the wire vocabulary is
/// one list, and so the schema's `enum` can be checked against it.
pub const SCRIPT_ENGINE_LANGUAGE_ECMASCRIPT: &str = "ecmascript";

/// Every value the wire surface admits for `script_engine_language`.
///
/// A test asserts this equals the schema's `enum`, and that every
/// [`crate::generator::Language`] maps into it — so a seventh backend, or a
/// third engine language, cannot reach the wire without both lists moving.
pub const SCRIPT_ENGINE_LANGUAGES: &[&str] = &[
    SCRIPT_ENGINE_LANGUAGE_LUA,
    SCRIPT_ENGINE_LANGUAGE_ECMASCRIPT,
];

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

/// The kind a run read its one document as, and whether the document
/// said so.
///
/// A document with no `sce:kind` is a statechart (SCE_FORGE.md §3.2), and
/// that default is silent: a transform whose author forgot the attribute
/// is read as a statechart, refused or accepted by that kind's rules, and
/// nothing in the verdict names the reading. An author choosing a kind
/// from prose — a person or a model — cannot confirm the product read
/// what was meant from the diagnostics alone, because a statechart that
/// happens to be well-formed produces none. So the reading is published:
/// `name` is the kind the run compiled, `declared` is `false` exactly when
/// it came from the default rather than from the document.
#[derive(Serialize, Debug, Clone, Copy, PartialEq, Eq)]
pub struct DocumentKind {
    /// The `sce:kind` spelling of the kind the run compiled —
    /// [`crate::forge::model::ForgeKind::as_attr`].
    pub name: &'static str,
    /// `false` when the root carries no `sce:kind` and the document was
    /// read as a statechart by default.
    pub declared: bool,
    /// Whether the root states why the document is this kind, in a
    /// `<sce:kind-basis>` ([`crate::forge::kind_basis`]). A manifest is
    /// written only for a document that parsed, so a basis counted here
    /// is one the parser accepted.
    pub basis_recorded: bool,
}

impl DocumentKind {
    /// The reading [`crate::classify_document`] routes on, for `content`.
    ///
    /// `None` when the root cannot be read or names a kind no variant goes
    /// by: such a document is refused before any manifest is written, so
    /// there is no reading to publish.
    pub fn of(content: &str) -> Option<Self> {
        use crate::forge::model::{ForgeKind, SCE_NAMESPACE};
        let doc = roxmltree::Document::parse(content).ok()?;
        let root = doc.root_element();
        let (name, declared) = match crate::forge::parser::detect_kind_from_node(&root) {
            Ok(Some(kind)) => (kind.as_attr(), true),
            Ok(None) => (ForgeKind::Statechart.as_attr(), false),
            Err(_) => return None,
        };
        let basis_recorded = root.children().any(|n| {
            n.is_element()
                && n.tag_name().namespace() == Some(SCE_NAMESPACE)
                && n.tag_name().name() == crate::forge::kind_basis::ELEMENT
        });
        Some(Self {
            name,
            declared,
            basis_recorded,
        })
    }
}

/// The external formatter that shaped a run's emitted artefacts, as the run
/// resolved it.
///
/// A formatter is an input of the bytes like the generator itself, so a run
/// that used one names it beside `generator` — the major is pinned
/// (`formatter::CLANG_FORMAT_MAJOR`), and this records which build of it ran.
/// See docs/SCE_CODEGEN_DETERMINISM.md §9.
#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct FormatterInfo {
    /// The tool, e.g. `clang-format`.
    pub tool: &'static str,
    /// The version it reported, e.g. `19.1.1`.
    pub version: String,
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
    /// Which kind the run read its document as — see [`DocumentKind`].
    /// Present exactly when the run is one document; a document set holds
    /// several kinds and reports none rather than one of them.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_kind: Option<DocumentKind>,
    pub artifacts: Vec<ArtifactEntry>,
    pub needs_script_engine: bool,
    /// Omitted (not `[]`) on a pure-static machine.
    #[serde(skip_serializing_if = "<[_]>::is_empty")]
    pub script_engine_causes: &'a [ScriptEngineCauseRecord],
    /// Which language the engine this machine needs must evaluate, from
    /// [`SCRIPT_ENGINE_LANGUAGES`]. Present exactly when
    /// [`Self::needs_script_engine`] is true; there is no engine to
    /// describe otherwise.
    ///
    /// `needs_script_engine` says a host must supply an engine. This says
    /// what kind, and it is the TARGET BACKEND's answer, not the
    /// document's: see
    /// [`crate::generator::Language::script_engine_language`].
    ///
    /// Absent on a run that targets more than one backend. `check` sweeps
    /// every language by default and the six do not agree, so one value
    /// there would have to be wrong for somebody — the reason this field
    /// was wrong for C++ and Kotlin in the first place. Carrying it per
    /// [`LanguageVerdict`] is the shape that answers for a sweep; until
    /// that lands, a sweep says nothing rather than something false.
    ///
    /// A consumer reading only the flag, and the document's
    /// `datamodel="ecmascript"`, would supply the wrong engine — see
    /// [`SCRIPT_ENGINE_LANGUAGE_LUA`] for why that reading is wrong, what
    /// it cost, and what this field itself got wrong until 2026-08-27.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub script_engine_language: Option<&'static str>,
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
    /// Whether any `<send>` / `<invoke>` in this run names an Event I/O
    /// Processor or invoker type the build has no lowering path for.
    ///
    /// Third member of the family [`Self::needs_script_engine`] and
    /// [`Self::needs_event_scheduler`] belong to — all three answer
    /// "what does the host have to supply?", and the generator settles
    /// all three while compiling.
    ///
    /// `true` does not mean the document is wrong. §scxml-6.2 and
    /// §scxml-6.4.1 define what happens to an unsupported type — the
    /// site raises `error.execution` — so a document naming one is valid
    /// SCXML with defined meaning, and a build deliberately relying on
    /// that refusal is a legitimate reading. What the field reports is
    /// that the refusal will happen, at a state the host may not enter
    /// for hours, with nothing before this line to say so.
    ///
    /// Always present, so a consumer reads `false` as an answer rather
    /// than as a field it has to guess the absence of.
    pub needs_host_processor: bool,
    /// Whether the host must register a Mesh router — the Event I/O
    /// Processor `sce:mesh` and the invoker `sce:mesh-rpc` — for this
    /// machine: it sends to a Mesh peer (`<send target="#peer">`) or requests
    /// one (`<invoke type="sce:mesh-rpc">`), and on this run's target
    /// language the peer is reached through a router the host supplies.
    /// `false` on C++, whose generated `TransportRouter` carries both itself.
    ///
    /// The same kind of answer as [`Self::needs_host_processor`], and
    /// published for the same reason: without it a host learns that the
    /// machine talks to other machines from the first `error.execution`.
    /// Present exactly when the run has one target language; a run that
    /// spans backends has no single answer and reports none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub needs_mesh_router: Option<bool>,
    /// Whether this machine sends to its parent session
    /// (`<send target="#_parent">`), and so can only run as a child.
    ///
    /// The same kind of answer as [`Self::needs_host_processor`]: the
    /// document is valid SCXML, and started on its own every such send
    /// raises `error.communication` (§scxml-6.2.4) at a state the host may
    /// not enter for hours. Whether a parent exists is a fact about the
    /// deployment, so a single-document run publishes the need rather than
    /// judging it. Present exactly when the run is one statechart document;
    /// a document-set run's answer depends on who invokes whom and is not
    /// this union, so it reports none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub needs_parent: Option<bool>,
    /// Which sites made [`Self::needs_parent`] true. Omitted (not `[]`)
    /// when there are none, matching [`Self::host_processor_causes`].
    #[serde(skip_serializing_if = "<[_]>::is_empty")]
    pub parent_sends: &'a [crate::parent_send_analyzer::ParentSend],
    /// Every `sce:unresolved` / `sce:assumed` marker the document this run
    /// read carries — the records `sce-codegen unresolved` prints, in the
    /// same order. On a document-set run, the markers of every member, each
    /// record naming the file it was written in (statecharts first, then
    /// forge documents, each in input order). Omitted (not `[]`) when there
    /// are none.
    ///
    /// ⚠ A set used to publish none, on the reasoning that its members each
    /// have their own. That is true of a parent and a host processor, and
    /// false of a question: one member that leaves a payload open makes the
    /// set leave it open, and a set that came back `accepted` with nothing
    /// to say so is the silence this field exists to end.
    ///
    /// Published because the run succeeds either way: a marker blocks only
    /// `--strict-unresolved`, which is right for a draft still being
    /// written, and meant that a document with an open question came back
    /// exactly like one with none: a draft that left a count
    /// `sce:unresolved` was reported `accepted` by `check --lint` with
    /// nothing in its manifest to say a question was open.
    #[serde(skip_serializing_if = "<[_]>::is_empty")]
    pub unresolved: &'a [crate::unresolved_check::UnresolvedRecord],
    /// What the accepted run still leaves to a person: the open questions,
    /// the values chosen without the specification, event-schemas a
    /// statechart imports without declaring its interface closed, a parent
    /// the machine needs, a processor the host must serve — one sentence
    /// each, from the records above ([`crate::open_matters`]). Omitted (not
    /// `[]`) when there are none. A document-set run reports the questions,
    /// assumed values and open interfaces of its members, and not a parent
    /// or a host processor: those stay one document's answer, since the
    /// union would name a parent none of the members may lack.
    ///
    /// `accepted` means the product found nothing to refuse, and that is
    /// not the same as a finished design. The sentences live in one module
    /// so the acceptance report, the acceptance record and every client
    /// that relays this field say them in the same words.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub open: Vec<crate::open_matters::OpenMatter>,
    /// Which sites made [`Self::needs_host_processor`] true. Omitted
    /// (not `[]`) when there are none, matching
    /// [`Self::script_engine_causes`].
    #[serde(skip_serializing_if = "<[_]>::is_empty")]
    pub host_processor_causes: &'a [HostProcessorCauseRecord],
    /// Event I/O Processor types this build was told the host serves
    /// (`--host-processor`). Omitted when none were declared.
    ///
    /// Reported because the declaration and the registration are two
    /// halves that must agree and are made in two different places — the
    /// build command and the host's startup code. Publishing the
    /// build's half is what lets a consumer check the pair instead of
    /// discovering the mismatch as an `error.execution` at run time.
    ///
    /// A declared type is absent from [`Self::host_processor_causes`]:
    /// that is what declaring it does.
    #[serde(skip_serializing_if = "<[_]>::is_empty")]
    pub host_processor_types: &'a [String],
    /// `<invoke type="...">` values this build was told the host can run
    /// (`--host-invoker`). Omitted when none were declared.
    ///
    /// A separate field from [`Self::host_processor_types`] because they
    /// are separate contracts — a host that delivers events is not
    /// thereby able to run an invoked process — and a consumer checking
    /// its registrations has two lists to check.
    #[serde(skip_serializing_if = "<[_]>::is_empty")]
    pub host_invoker_types: &'a [String],
    /// §scxml-5.3: the typed `<data>` declarations this run emitted no
    /// reader for, and why. Omitted when every typed declaration got one.
    ///
    /// A host reaches a variable through its reader, and a reader that is
    /// not there looks, from the host's side, exactly like one the generator
    /// forgot — or like a name the host misspelled. Published so the absence
    /// has a cause attached: the declarations disagree about the type, the
    /// name cannot be written in an expression, or it cannot be spelled in
    /// some backend without meeting a keyword, a member the generated type
    /// already carries, or another variable's reader. Decided the same for
    /// every backend (`reader_names`), so a `check` sweep and a single
    /// `generate` report the same list.
    #[serde(skip_serializing_if = "<[_]>::is_empty")]
    pub unreadable_variables: &'a [crate::reader_names::UnreadableVariable],
    /// The object a transform keeps its `previous()` values in, by the
    /// names the target backend gave it. Present exactly when the run
    /// generated one; omitted otherwise, and omitted on a run spanning
    /// several backends, whose names differ.
    ///
    /// A transform is pure functions unless an output reads a field
    /// through `previous()`, and then the host must drive it through the
    /// holder — construct one, call `update` once per activation, `reset`
    /// to start again — because only the holder keeps what the next
    /// activation reads. Which of the two a document is follows from its
    /// content, not its kind, so a host cannot tell from `sce:kind`; this
    /// says it, with the names to call. Decided by
    /// [`crate::forge::generator::transform_holder`], the call the
    /// generator itself makes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub holder: Option<crate::forge::generator::TransformHolderSymbols>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rejected: Option<RejectedInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deploy: Option<DeployInfo>,
    /// The formatter that shaped the emitted artefacts, when one did.
    /// Omitted when the run formatted nothing: `--no-format`, or no C++ was
    /// emitted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub formatter: Option<FormatterInfo>,
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

    /// A kind reaches the published reading in the same commit it reaches
    /// the parser, in the spelling the parser accepts.
    #[test]
    fn json_schema_document_kind_enum_matches_rust_source_of_truth() {
        use crate::forge::model::ForgeKind;
        let schema = schema();
        let declared: Vec<&str> = schema["properties"]["document_kind"]["properties"]["name"]
            ["enum"]
            .as_array()
            .expect("document_kind.name.enum is an array")
            .iter()
            .map(|v| v.as_str().expect("document_kind enum member is a string"))
            .collect();
        assert_eq!(
            declared,
            ForgeKind::ALL_ATTR_NAMES,
            "schemas/sce-manifest.v1.schema.json document_kind.name.enum drifted from \
             ForgeKind::ALL_ATTR_NAMES",
        );
    }

    /// The three readings a root can produce: declared forge kind, declared
    /// statechart, and the silent default this field exists to name.
    #[test]
    fn document_kind_names_the_default_as_undeclared() {
        let root = |attr: &str| {
            format!(
                "<scxml xmlns=\"http://www.w3.org/2005/07/scxml\" \
                 xmlns:sce=\"http://sce.dev/ext\" version=\"1.0\"{attr}/>"
            )
        };
        let reading = |name, declared| {
            Some(DocumentKind {
                name,
                declared,
                basis_recorded: false,
            })
        };
        assert_eq!(
            DocumentKind::of(&root(" sce:kind=\"lookup\"")),
            reading("lookup", true)
        );
        assert_eq!(
            DocumentKind::of(&root(" sce:kind=\"statechart\"")),
            reading("statechart", true)
        );
        assert_eq!(DocumentKind::of(&root("")), reading("statechart", false));
        assert_eq!(DocumentKind::of(&root(" sce:kind=\"lookupp\"")), None);
    }

    /// A basis on the root is counted; one of the same local name in
    /// another namespace is not.
    #[test]
    fn document_kind_says_whether_the_kind_was_argued() {
        let with = |child: &str| {
            format!(
                "<scxml xmlns=\"http://www.w3.org/2005/07/scxml\" \
                 xmlns:sce=\"http://sce.dev/ext\" xmlns:x=\"urn:x\" \
                 sce:kind=\"lookup\">{child}</scxml>"
            )
        };
        let recorded =
            |child: &str| DocumentKind::of(&with(child)).map(|reading| reading.basis_recorded);
        assert_eq!(
            recorded("<sce:kind-basis><sce:evidence>t</sce:evidence></sce:kind-basis>"),
            Some(true)
        );
        assert_eq!(recorded("<x:kind-basis/>"), Some(false));
        assert_eq!(recorded(""), Some(false));
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
            document_kind: None,
            artifacts: vec![ArtifactEntry {
                path: "out/foo_sm.rs".to_string(),
            }],
            needs_script_engine: false,
            script_engine_causes: &[],
            script_engine_language: None,
            needs_event_scheduler: false,
            needs_host_processor: false,
            needs_mesh_router: None,
            needs_parent: None,
            parent_sends: &[],
            unresolved: &[],
            open: Vec::new(),
            host_processor_causes: &[],
            host_processor_types: &[],
            host_invoker_types: &[],
            unreadable_variables: &[],
            holder: None,
            rejected: None,
            deploy: None,
            formatter: None,
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
            document_kind: None,
            artifacts: vec![ArtifactEntry {
                path: "out/foo_sm.h".to_string(),
            }],
            needs_script_engine: false,
            script_engine_causes: &[],
            script_engine_language: None,
            needs_event_scheduler: false,
            needs_host_processor: false,
            needs_mesh_router: None,
            needs_parent: None,
            parent_sends: &[],
            unresolved: &[],
            open: Vec::new(),
            host_processor_causes: &[],
            host_processor_types: &[],
            host_invoker_types: &[],
            unreadable_variables: &[],
            holder: None,
            rejected: Some(RejectedInfo {
                spec: "W3C SCXML 5.8",
                name: "untestable_doc".to_string(),
            }),
            deploy: Some(DeployInfo {
                static_analyzer: Some(StaticAnalyzer::Coverity.as_str()),
            }),
            formatter: Some(FormatterInfo {
                tool: "clang-format",
                version: "19.1.1".to_string(),
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
            document_kind: None,
            artifacts: Vec::new(),
            needs_script_engine: false,
            script_engine_causes: &[],
            script_engine_language: None,
            needs_event_scheduler: false,
            needs_host_processor: false,
            needs_mesh_router: None,
            needs_parent: None,
            parent_sends: &[],
            unresolved: &[],
            open: Vec::new(),
            host_processor_causes: &[],
            host_processor_types: &[],
            host_invoker_types: &[],
            unreadable_variables: &[],
            holder: None,
            rejected: None,
            deploy: None,
            formatter: None,
            languages: Some(vec![
                LanguageVerdict::ok("rust"),
                LanguageVerdict::rejected("cpp", "generate/unsupported-feature".to_string()),
            ]),
        };
        assert_valid(&m.to_line());
    }

    /// The populated form, which the empty cases above do not reach:
    /// `host_processor_causes` is skipped when empty, so every test
    /// above validates a record in which the array never appears. This
    /// is the one that puts a record on the wire.
    #[test]
    fn host_processor_causes_validate_against_schema() {
        let causes = vec![
            HostProcessorCauseRecord {
                kind: "send-type",
                processor_type: "x-example-host".to_string(),
                state: Some("sending".to_string()),
                invoke: None,
                location: Some(crate::forge::error::SourceLocation {
                    file: "probe.scxml".to_string(),
                    line: Some(13),
                    col: Some(7),
                }),
            },
            HostProcessorCauseRecord {
                kind: "invoke-type",
                processor_type: "x-example-host".to_string(),
                state: Some("invoking".to_string()),
                invoke: Some("_invoke_0".to_string()),
                location: None,
            },
        ];
        let m = Manifest {
            v: MANIFEST_SCHEMA_VERSION,
            kind: ManifestKind::Generate.as_str(),
            generator: "deadbeefcafe",
            document_kind: None,
            artifacts: vec![ArtifactEntry {
                path: "out/probe_sm.rs".to_string(),
            }],
            needs_script_engine: false,
            script_engine_causes: &[],
            script_engine_language: None,
            needs_event_scheduler: false,
            needs_host_processor: true,
            needs_mesh_router: None,
            needs_parent: None,
            parent_sends: &[],
            unresolved: &[],
            open: Vec::new(),
            host_processor_causes: &causes,
            host_processor_types: &[],
            host_invoker_types: &[],
            unreadable_variables: &[],
            holder: None,
            rejected: None,
            deploy: None,
            formatter: None,
            languages: None,
        };
        let line = m.to_line();
        assert_valid(&line);
        // The record must reach the wire, not merely be schema-legal in
        // principle: an accidental `skip_serializing` would leave the
        // flag true with nothing explaining it, and `assert_valid` alone
        // would still pass.
        assert!(line.contains("\"host_processor_causes\""), "{line}");
        assert!(
            line.contains("\"processor_type\":\"x-example-host\""),
            "{line}"
        );
        assert!(line.contains("\"needs_host_processor\":true"), "{line}");
    }

    /// The one question `needs_script_engine` does not answer: a host
    /// told it must supply an engine still has to know what kind, and
    /// `datamodel="ecmascript"` is the wrong place to read it.
    #[test]
    fn a_machine_needing_an_engine_names_the_language_it_evaluates() {
        let m = Manifest {
            v: MANIFEST_SCHEMA_VERSION,
            kind: ManifestKind::Generate.as_str(),
            generator: "deadbeefcafe",
            document_kind: None,
            artifacts: Vec::new(),
            needs_script_engine: true,
            script_engine_causes: &[],
            script_engine_language: Some(SCRIPT_ENGINE_LANGUAGE_LUA),
            needs_event_scheduler: false,
            needs_host_processor: false,
            needs_mesh_router: None,
            needs_parent: None,
            parent_sends: &[],
            unresolved: &[],
            open: Vec::new(),
            host_processor_causes: &[],
            host_processor_types: &[],
            host_invoker_types: &[],
            unreadable_variables: &[],
            holder: None,
            rejected: None,
            deploy: None,
            formatter: None,
            languages: None,
        };
        let line = m.to_line();
        assert_valid(&line);
        assert!(
            line.contains("\"script_engine_language\":\"lua\""),
            "{line}"
        );
        // The assertion is on the VALUE and not merely on the key's
        // presence, because a key naming the wrong language is worse than
        // no key — which is what this field did for two backends until
        // 2026-08-27. Both spellings are wire vocabulary now, so what is
        // asserted is that the value comes from that list rather than that
        // it is never `ecmascript`: on a C++ or Kotlin target `ecmascript`
        // is the true answer.
        assert!(
            SCRIPT_ENGINE_LANGUAGES.contains(&SCRIPT_ENGINE_LANGUAGE_LUA),
            "the wire vocabulary lost a spelling this record emits"
        );
        assert_ne!(
            SCRIPT_ENGINE_LANGUAGE_LUA,
            SCRIPT_ENGINE_LANGUAGE_ECMASCRIPT
        );
    }

    /// A pure-static machine has no engine to describe, so naming one
    /// would be an answer to a question that was not asked.
    #[test]
    fn a_pure_static_machine_names_no_engine_language() {
        let m = Manifest {
            v: MANIFEST_SCHEMA_VERSION,
            kind: ManifestKind::Generate.as_str(),
            generator: "deadbeefcafe",
            document_kind: None,
            artifacts: Vec::new(),
            needs_script_engine: false,
            script_engine_causes: &[],
            script_engine_language: None,
            needs_event_scheduler: false,
            needs_host_processor: false,
            needs_mesh_router: None,
            needs_parent: None,
            parent_sends: &[],
            unresolved: &[],
            open: Vec::new(),
            host_processor_causes: &[],
            host_processor_types: &[],
            host_invoker_types: &[],
            unreadable_variables: &[],
            holder: None,
            rejected: None,
            deploy: None,
            formatter: None,
            languages: None,
        };
        let line = m.to_line();
        assert_valid(&line);
        assert!(!line.contains("script_engine_language"), "{line}");
    }

    /// The flag is required, in the same family and for the same reason
    /// as its two siblings: a consumer gating on "this build has a path
    /// for every type the document names" must be able to tell `false`
    /// from a generator that does not report the question at all.
    #[test]
    fn schema_requires_the_host_processor_flag() {
        let m = Manifest {
            v: MANIFEST_SCHEMA_VERSION,
            kind: ManifestKind::Generate.as_str(),
            generator: "deadbeefcafe",
            document_kind: None,
            artifacts: Vec::new(),
            needs_script_engine: false,
            script_engine_causes: &[],
            script_engine_language: None,
            needs_event_scheduler: false,
            needs_host_processor: false,
            needs_mesh_router: None,
            needs_parent: None,
            parent_sends: &[],
            unresolved: &[],
            open: Vec::new(),
            host_processor_causes: &[],
            host_processor_types: &[],
            host_invoker_types: &[],
            unreadable_variables: &[],
            holder: None,
            rejected: None,
            deploy: None,
            formatter: None,
            languages: None,
        };
        let line = m.to_line();
        assert_valid(&line);
        let stripped = line.replace(",\"needs_host_processor\":false", "");
        assert_ne!(stripped, line, "the field was not on the wire to remove");
        assert_invalid(&stripped, "needs_host_processor is required");
    }

    /// The build's half of the host-processor contract has to be
    /// readable, or a consumer can only discover a
    /// declared-but-unregistered type by running the machine into the
    /// state that sends.
    #[test]
    fn declared_host_processors_reach_the_wire() {
        let declared = vec!["x-example-host".to_string()];
        let m = Manifest {
            v: MANIFEST_SCHEMA_VERSION,
            kind: ManifestKind::Generate.as_str(),
            generator: "deadbeefcafe",
            document_kind: None,
            artifacts: Vec::new(),
            needs_script_engine: false,
            script_engine_causes: &[],
            script_engine_language: None,
            needs_event_scheduler: false,
            // A declared type is served, so it is NOT a cause — the two
            // fields are the two halves of one answer and a record that
            // showed a type in both would be reporting a refusal the
            // build just arranged not to emit.
            needs_host_processor: false,
            needs_mesh_router: None,
            needs_parent: None,
            parent_sends: &[],
            unresolved: &[],
            open: Vec::new(),
            host_processor_causes: &[],
            host_processor_types: &declared,
            // The invoke half declared beside it, because the two travel
            // together on a real build and a record carrying only one
            // would never exercise both fields on the wire at once.
            host_invoker_types: &declared,
            unreadable_variables: &[],
            holder: None,
            rejected: None,
            deploy: None,
            formatter: None,
            languages: None,
        };
        let line = m.to_line();
        assert_valid(&line);
        assert!(
            line.contains("\"host_processor_types\":[\"x-example-host\"]"),
            "{line}"
        );
        assert!(
            line.contains("\"host_invoker_types\":[\"x-example-host\"]"),
            "{line}"
        );
    }

    /// A transform that keeps state names its holder, and the schema holds
    /// the record to every name a host has to call — a holder missing one
    /// would tell the host to drive an object it cannot reach.
    #[test]
    fn generate_manifest_with_holder_validates_against_schema() {
        let m = Manifest {
            v: MANIFEST_SCHEMA_VERSION,
            kind: ManifestKind::Generate.as_str(),
            generator: "deadbeefcafe",
            document_kind: None,
            artifacts: Vec::new(),
            needs_script_engine: false,
            script_engine_causes: &[],
            script_engine_language: None,
            needs_event_scheduler: false,
            needs_host_processor: false,
            needs_mesh_router: None,
            needs_parent: None,
            parent_sends: &[],
            unresolved: &[],
            open: Vec::new(),
            host_processor_causes: &[],
            host_processor_types: &[],
            host_invoker_types: &[],
            unreadable_variables: &[],
            holder: Some(crate::forge::generator::TransformHolderSymbols {
                holder: "Counter".to_string(),
                outputs: "CounterOutputs".to_string(),
                new: "new".to_string(),
                reset: "reset".to_string(),
                update: "update".to_string(),
                restored: Some("restored".to_string()),
            }),
            rejected: None,
            deploy: None,
            formatter: None,
            languages: None,
        };
        let line = m.to_line();
        assert_valid(&line);
        assert!(
            line.contains("\"holder\":{\"holder\":\"Counter\""),
            "{line}"
        );
        let without_update = line.replace(",\"update\":\"update\"", "");
        assert_ne!(
            without_update, line,
            "the fixture must carry the name it drops"
        );
        assert_invalid(&without_update, "a holder with no `update` is refused");
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

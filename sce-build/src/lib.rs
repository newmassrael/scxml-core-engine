// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// sce-build — SCXML code generator for sce-rust-runtime.
//
// Parses SCXML files and generates Rust code implementing
// `sce_rust_runtime::StatePolicy`. Drop into build.rs to
// eliminate the Python codegen dependency.
//
// Usage:
//   // build.rs
//   fn main() {
//       sce_build::compile_scxml(&["src/traffic_light.scxml"]);
//   }
//
//   // main.rs
//   include!(concat!(env!("OUT_DIR"), "/traffic_light_sm.rs"));

pub mod analyzer;
pub mod cli_error;
pub mod conformance;
pub mod filters;
pub mod forge;
#[cfg(not(target_arch = "wasm32"))]
pub mod formatter;
pub mod generator;
pub mod kotlin;
pub mod lua_transformer;
pub mod mesh;
pub mod model;
pub mod parser;
/// Byte-level mapping from an expanded SCXML document back to its
/// source origins. Consumed by the parser boundary to remap
/// post-expansion diagnostic coordinates (XSD line numbers,
/// roxmltree row/col, semantic validation) to author file/row/col.
/// See [`position_map`] for the shape and lookup semantics.
pub mod position_map;
/// Spec-provenance + requirement-traceability + unresolved-placeholder
/// types — shared by [`model`], [`forge::model`], and
/// [`forge::diagnostic`]. See `nl_to_ir_mapping_roadmap.md` for the
/// shared-type rationale (Items 1, 5, 6 fragment into incompatible
/// representations if each grows its own).
pub mod provenance;
/// NL→IR Mapping Roadmap Item 1: emit per-node `sce:req` NDJSON.
/// Drives the `sce-codegen requirements` CLI subcommand for
/// downstream req-coverage tooling.
pub mod requirements_report;
pub mod script_engine_analyzer;
/// NL→IR Mapping Roadmap Item 3: event-set exhaustiveness
/// validator. Flags compound `<state>` parents whose sibling children
/// disagree on event coverage with no parent fallthrough — the
/// AI-generated SCXML intent-gap pattern. Narrow heuristic (requires
/// a shared event vocabulary across siblings) keeps W3C IRP at zero
/// false positives.
pub mod scxml_exhaustiveness;
/// NL→IR Mapping Roadmap Item 3: guard analysis. Recognises
/// trivially-false `<transition cond>` expressions and shadowed
/// transitions (unconditional siblings making later same-event
/// siblings dead per §scxml-5.10). Stays narrow: language-prefixed
/// conds (`cpp:`, `kotlin:`, `rust:`) are opaque, token-prefix
/// superset shadowing is not flagged.
pub mod scxml_guard_analysis;
/// NL→IR Mapping Roadmap Item 3: Statechart graph reachability
/// validator. BFS from the document `initial` configuration computes
/// the design-time reach set and rejects orphan states / dead
/// transitions before codegen.
pub mod scxml_reachability;
pub mod scxml_semantic;
/// `sce:template` / `sce:use` / `sce:param` preprocessing —
/// parameterised composition adjacent to XInclude. AOT-only;
/// runs immediately after XInclude expansion
/// so templates see a post-XInclude document. See [`template`]
/// for the expansion semantics and error model.
pub mod template;
/// NL→IR Mapping Roadmap Item 5: `<sce:unresolved>` placeholder
/// detection — strict-mode build gate + NDJSON report. Drives
/// `--strict-unresolved` on `generate` and `sce-codegen unresolved`.
pub mod unresolved_check;
pub mod w3c_dist_manifest;
#[cfg(feature = "wasm")]
mod wasm;
/// W3C XInclude preprocessing. Runs between XSD validation and
/// roxmltree's document parse so the AOT code generator consumes
/// the same post-expansion document as the C++ runtime. See
/// [`xinclude`] for the expansion semantics and the deliberate
/// divergence in error handling (runtime warns-and-continues,
/// AOT hard-errors).
pub mod xinclude;

use model::SCXMLModel;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Two-role document label for the forge pipeline.
///
/// The library needs the caller's view of "what this document is called"
/// in two independent roles. Folding them back into one `&str` — as the
/// pre-2026-04-14 API did — forces the caller to pick between a stem
/// (safe for identifiers, loses the `.scxml` suffix in diagnostics) and
/// a basename (clean diagnostics, corrupts generated symbols). Neither
/// is correct for both consumers, so the type carries both explicitly.
///
/// `identifier` flows into [`forge::model`] `name` fields and from there
/// into template symbol generation (Go package, C++ namespace, function
/// name). It must be extension-free or generated code breaks.
///
/// `diagnostic_label` flows into [`forge::error::Located`] and XSD
/// [`forge::xsd_validator::XsdErrors`]`::source_label`, surfacing as the
/// `location.file` of every NDJSON record. Should carry the full
/// basename (with extension) so downstream tooling opens the source
/// without guessing the suffix.
#[derive(Debug, Clone, Copy)]
pub struct DocumentLabel<'a> {
    pub identifier: &'a str,
    pub diagnostic_label: &'a str,
}

impl<'a> DocumentLabel<'a> {
    /// Both roles collapse to one label. For in-memory / WASM callers
    /// with no filesystem path — there is nothing extension-worthy to
    /// distinguish, so identifier and diagnostic label coincide.
    pub fn symmetric(label: &'a str) -> Self {
        Self {
            identifier: label,
            diagnostic_label: label,
        }
    }

    /// Distinct identifier vs diagnostic-label roles. Used by the
    /// synth-invoke inline-`<content>` parser path: `identifier` is
    /// the extension-free synth name (flows into [`SCXMLModel::name`]
    /// → template PascalCase symbols), `diagnostic_label` is the
    /// "as if on disk" filename including `.scxml` (becomes
    /// [`source_location.file`] → SCE-MAP markers + NDJSON
    /// `location.file`) so generated artefacts stay byte-stable
    /// against the pre-refactor disk-write era.
    pub fn asymmetric(identifier: &'a str, diagnostic_label: &'a str) -> Self {
        Self {
            identifier,
            diagnostic_label,
        }
    }
}

/// Result of the parse-phase: the analyzed SCXML model plus the
/// canonical paths of every external file the preprocessor consumed.
///
/// `preprocessor_deps` is collected by [`parser::SCXMLParser::parse_file`]
/// from the XInclude + `sce:template` expanders and surfaces here so
/// every codegen-side entry point can attach the same dep set to its
/// emitted [`generator::GeneratedOutput`] — single source of truth for
/// build-system rerun invalidation. Empty after `parse_string` (no fs
/// access).
pub(crate) struct ParsedSCXML {
    pub model: SCXMLModel,
    pub preprocessor_deps: Vec<std::path::PathBuf>,
}

/// Parse, analyze, and validate an SCXML file for static code generation.
/// SCE Forge inline kinds are extracted during parsing (single XML pass).
///
/// Returns `Located<ForgeError>` so parser and analyzer failures
/// travel through the typed wire contract end-to-end; the public
/// String-returning wrappers (`compile_scxml_to_string`,
/// `compile_from_string`, `compile_scxml_lang`) shim at their own
/// boundary because WASM/JS callers marshal only strings. See the
/// `CompileError` alias below — keeping this signature on the typed
/// side eliminates mid-function stringification.
///
/// The returned [`ParsedSCXML`] carries `preprocessor_deps` so callers
/// can attach the dep set to their codegen output. Discarding the
/// deps (e.g. in the `compile_scxml_to_string` shim that does not
/// have an output struct to populate) is legal but consumers that
/// drive Cargo / depfile sinks MUST forward them — see
/// [`generator::GeneratedOutput::deps`] for the canonical surface.
fn compile_model(scxml_path: &str) -> Result<ParsedSCXML, CompileError> {
    let mut parser = parser::SCXMLParser::new();
    let mut model = parser.parse_file(scxml_path)?;
    analyzer::analyze(&mut model, scxml_path);
    guard_static_generatable(&model, scxml_path)?;
    // NL→IR Mapping Roadmap Item 3 — Statechart graph
    // reachability. Runs after the analyzer finalises the state graph
    // (parallel-region computation, initial-cascade resolution) and
    // after `guard_static_generatable` so the more-fundamental
    // diagnostics — `ScxmlSemanticError::InitialStateUnknown`,
    // `TopLevelScriptUnloaded`, the missing-initial `DynamicFeatures`
    // gate — fire ahead of the per-state orphan walk. Reachability
    // is the structural-quality check that runs only once the basic
    // entry contract is sound; running it earlier would shadow the
    // root-cause diagnostic with a downstream consequence.
    scxml_reachability::validate(&model, scxml_path)?;
    // NL→IR Mapping Roadmap Item 3 — event-set
    // exhaustiveness. Runs after the reachability walk so an unreachable state is
    // reported via the structural code first; the exhaustiveness
    // walker presumes the graph topology is sound and surfaces only
    // the design-time intent-gap pattern.
    scxml_exhaustiveness::validate(&model, scxml_path)?;
    // NL→IR Mapping Roadmap Item 3 — guard analysis. Runs after
    // the exhaustiveness walk so structural intent-gap diagnostics fire ahead
    // of the per-transition guard heuristic.
    scxml_guard_analysis::validate(&model, scxml_path)?;
    // Watching-zenoh RFC §5.O Atomic 0a — IR provenance pre-emit
    // guard. Runs *after* the analyzer (so synthesised IR additions
    // are visible) and *before* `resolve_source_path` populates the
    // template-visible source path (so a `None` cannot leak through
    // to the marker-emitting templates). The walker fires
    // `traceability/scxml-line-range-missing` when a node eligible
    // for SCE-MAP marker emission carries `source_location: None`
    // — codegen-internal invariant, no author repair. See
    // `forge::provenance` for the eligibility scope.
    forge::provenance::validate_emission_provenance(&model, scxml_path)?;
    resolve_source_path(&mut model, scxml_path);
    let preprocessor_deps = parser.preprocessor_deps().to_vec();
    Ok(ParsedSCXML {
        model,
        preprocessor_deps,
    })
}

/// Parse SCXML content string, analyze and validate (no filesystem).
/// SCE Forge inline kinds are extracted during parsing (single XML pass).
fn compile_model_from_string(
    scxml_content: &str,
    scxml_name: &str,
) -> Result<SCXMLModel, CompileError> {
    let mut parser = parser::SCXMLParser::new();
    let mut model = parser.parse_string(scxml_content, scxml_name)?;
    analyzer::analyze(&mut model, "");
    guard_static_generatable(&model, scxml_name)?;
    // NL→IR Mapping Roadmap Item 3 reachability — see `compile_model`
    // for the placement rationale (after the basic static-generation guard
    // so root-cause `ScxmlSemanticError` diagnostics fire ahead of the
    // orphan walk).
    scxml_reachability::validate(&model, scxml_name)?;
    // NL→IR Mapping Roadmap Item 3 event-set exhaustiveness — see
    // `compile_model` for the placement rationale (after the
    // reachability walk so the structural
    // root-cause fires ahead of the heuristic).
    scxml_exhaustiveness::validate(&model, scxml_name)?;
    // NL→IR Mapping Roadmap Item 3 — guard analysis.
    scxml_guard_analysis::validate(&model, scxml_name)?;
    // Watching-zenoh RFC §5.O Atomic 0a — IR provenance pre-emit
    // guard. WASM / parse_string callers share the same invariant
    // as the file-based entry point above; both routes converge on
    // `compile_model*` so the gate has a single placement.
    forge::provenance::validate_emission_provenance(&model, scxml_name)?;
    Ok(model)
}

/// Typed error channel for the two `compile_model*` helpers.
pub type CompileError = forge::error::Located<forge::error::ForgeError>;

/// Promote the `analyzer::can_generate_static` precondition into a
/// `Located<ForgeError>` for the wire layer. §wire-W5 D3 refit:
/// `can_generate_static` itself now returns the correctly-classified
/// `ForgeError` (split between `ValidationDynamicFeatures` for genuine
/// codegen limitations and `ScxmlSemanticError::*` for hard semantic
/// violations); this helper just stamps the source location.
fn guard_static_generatable(model: &SCXMLModel, source_name: &str) -> Result<(), CompileError> {
    use forge::error::Located;
    analyzer::can_generate_static(model).map_err(|err| Located::new(err, source_name, None, None))
}

/// Prepend the spec §6.2.6 `// SCE-GENERATED` header to every file in
/// `output`. The comment prefix is picked per-file based on extension
/// (`.py` → `#`, everything else → `//`).
///
/// Idempotent across re-invocations — files that already lead with the
/// SCE-GENERATED banner get their existing header lines replaced rather
/// than duplicated. Repeat invocations with the same `hashes` /
/// `generated_at_secs` are byte-stable.
///
/// Pairs with `sce-codegen verify <out-dir>`: emit through this helper
/// so the verify command can recompute and compare hashes against the
/// embedded values, fulfilling the spec invariant that every emitted
/// file carries a drift-detectable header.
///
/// Pure helper — does not touch the filesystem. Caller decides whether
/// to write the headered output.
pub fn apply_drift_headers_to_output(
    output: &mut generator::GeneratedOutput,
    hashes: &forge::drift::DriftHashes,
    generated_at_secs: u64,
) {
    use std::path::Path;
    for (filename, content) in &mut output.files {
        let prefix = forge::drift::comment_prefix_for_path(Path::new(filename));
        *content =
            forge::drift::prepend_or_replace_header(content, hashes, generated_at_secs, prefix);
    }
}

/// Resolve SCXML source path to project-relative path.
pub fn resolve_source_path(model: &mut SCXMLModel, scxml_path: &str) {
    if let Ok(abs) = std::fs::canonicalize(scxml_path) {
        if let Ok(cwd) = std::env::current_dir() {
            if let Ok(rel) = abs.strip_prefix(&cwd) {
                model.scxml_source_path = rel.to_string_lossy().to_string();
            } else {
                model.scxml_source_path = abs.to_string_lossy().to_string();
            }
        }
    }
}

/// Watching-zenoh RFC §5.2 Round F-α — resolve every `<sce:driver
/// href="..."/>` reference on the SCXML root against the SCXML file's
/// parent directory (the Q-Round-F-D5 default root). Each successful
/// resolution populates `DriverRef::resolved_path`; the first miss
/// surfaces `mcu/driver-header-not-found` with the `Located`
/// row/column from the parser-stamped source_location so authors land
/// on the exact `<sce:driver>` element.
///
/// Absolute `href` values are passed through `Path::is_absolute` and
/// not joined with the parent. The resolver intentionally does NOT
/// parse the driver header — Q-Round-F-D2 delegates cross-TU symbol
/// resolution to the C compiler. The only contract this helper
/// enforces is filesystem existence, mirroring `XInclude` resolver
/// behaviour for SCXML composition.
///
/// `deploy.yaml`'s `platform.driver_root` override is consumed by
/// [`resolve_driver_refs_with_root`] (deploy-aware entry); this
/// helper is the SCXML-only baseline.
fn resolve_driver_refs(model: &mut SCXMLModel, scxml_path: &str) -> Result<(), CompileError> {
    let parent_default = std::path::Path::new(scxml_path).parent().map_or_else(
        || std::path::PathBuf::from("."),
        std::path::Path::to_path_buf,
    );
    resolve_driver_refs_with_root(model, scxml_path, &parent_default)
}

/// Watching-zenoh RFC §5.2 Round F-α — resolve every `<sce:driver>`
/// against `root` (the override path supplied by `deploy.yaml`'s
/// `platform.driver_root` per Q-Round-F-D5). The compile-model gate
/// uses the SCXML file's parent directory; deploy-aware callers pass
/// the resolved root explicitly so the override beats the default
/// without re-walking from scratch. Absolute `href` values are
/// honoured verbatim (not joined with `root`).
pub fn resolve_driver_refs_with_root(
    model: &mut SCXMLModel,
    diag_label: &str,
    root: &std::path::Path,
) -> Result<(), CompileError> {
    use forge::error::{Located, ValidationError};
    for driver in model.driver_refs.iter_mut() {
        let candidate = std::path::Path::new(&driver.href);
        let resolved = if candidate.is_absolute() {
            candidate.to_path_buf()
        } else {
            root.join(candidate)
        };
        if !resolved.exists() {
            let (line, col) = driver
                .source_location
                .as_ref()
                .map_or((None, None), |loc| (loc.line, loc.col));
            return Err(Located::new(
                ValidationError::McuDriverHeaderNotFound {
                    href: driver.href.clone(),
                    resolved_dir: root.to_string_lossy().to_string(),
                }
                .into(),
                diag_label,
                line,
                col,
            ));
        }
        driver.resolved_path = Some(resolved.to_string_lossy().to_string());
    }
    Ok(())
}

/// Compile SCXML files to Rust source code in `OUT_DIR`.
///
/// Generates `{name}_sm.rs` for each input SCXML file. Intended for
/// use in `build.rs`.
///
/// For every input, emits `cargo::rerun-if-changed=` lines for both
/// the input itself and every preprocessor dependency the parser
/// pulled in (`<xi:include>` targets, `<sce:use template>` fragments).
/// Without the dep lines, Cargo cannot tell that a shared
/// `*.sce-template.xml` participated in the build, so a template
/// edit silently keeps the previous generated `*_sm.rs` until either
/// the host SCXML is touched or `cargo clean` runs — a correctness
/// hazard, not just an ergonomic gap.
///
/// Routes through [`compile_scxml_lang_typed`] (Language::Rust)
/// rather than [`compile_scxml_to_string`] so the dep set survives
/// the parse boundary: the typed entry returns
/// [`generator::GeneratedOutput`] whose `deps` field is the canonical
/// build-system rerun surface.
pub fn compile_scxml(scxml_files: &[&str]) {
    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR not set (must be called from build.rs)");
    let template_dir = find_template_dir();

    for scxml_path in scxml_files {
        let output = compile_scxml_lang_typed(scxml_path, &template_dir, generator::Language::Rust)
            .unwrap_or_else(|e| panic!("Failed to compile {scxml_path}: {e}"));

        let stem = Path::new(scxml_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .expect("Invalid SCXML filename");

        // `compile_scxml_lang_typed` with `Rust` always emits exactly
        // one file (`{stem}_sm.rs`); destructuring the single entry
        // makes the contract explicit. Multi-file backends (C++/C11)
        // route through `compile_scxml_lang_typed_with_section`
        // directly, not this Rust-only build.rs facade.
        let (_filename, code) = output
            .files
            .into_iter()
            .next()
            .expect("compile_scxml_lang_typed(Rust) must emit exactly one file");

        let out_path = Path::new(&out_dir).join(format!("{stem}_sm.rs"));
        std::fs::write(&out_path, &code)
            .unwrap_or_else(|e| panic!("Cannot write {}: {e}", out_path.display()));

        println!("cargo::rerun-if-changed={scxml_path}");
        for dep in &output.deps {
            // `preprocessor_deps` is canonicalised at the parser
            // boundary, so emitting `display()` preserves the
            // absolute path Cargo needs to invalidate against. Cargo
            // de-duplicates rerun-if-changed across invocations
            // internally, so no extra dedup here.
            println!("cargo::rerun-if-changed={}", dep.display());
        }
    }
}

/// Adapt an arbitrary ForgeError-convertible codegen error into the
/// public typed channel. `source_name` tags the file/record label so
/// downstream agents can route repairs; `line`/`col` stay `None`
/// because `GenerateError` is raised by minijinja well after the DOM
/// is discarded — fabricating `(1, 1)` would mislead the repair loop
/// (see the `feedback_correctness_before_features` memory).
fn locate_codegen_error<E: Into<forge::error::ForgeError>>(
    err: E,
    source_name: &str,
) -> CompileError {
    forge::error::Located::new(err.into(), source_name, None, None)
}

/// Typed analogue of [`compile_scxml_to_string`] — consumers that can
/// observe `Located<ForgeError>` should prefer this entry point so they
/// receive structured `code` / `stage` / `fix` data instead of a
/// stringified message.
pub fn compile_scxml_to_string_typed(
    scxml_path: &str,
    template_dir: &Path,
) -> Result<String, CompileError> {
    // String-only entry: deliberately drops `preprocessor_deps` because
    // the return type carries no dep channel. Build-system consumers
    // (`compile_scxml`, `sce-codegen --write-deps`) must route through
    // an entry that returns [`generator::GeneratedOutput`] so the dep
    // set survives to the depfile sink.
    let ParsedSCXML { model, .. } = compile_model(scxml_path)?;
    generator::generate(&model, template_dir, false)
        .map_err(|e| locate_codegen_error(e, scxml_path))
}

/// Compile a single SCXML file to Rust source code string.
///
/// This is the core API — parses SCXML, analyzes model, renders templates.
/// String-returning shim over [`compile_scxml_to_string_typed`]; callers
/// that can consume `CompileError` should use the typed entry point.
pub fn compile_scxml_to_string(scxml_path: &str, template_dir: &Path) -> Result<String, String> {
    compile_scxml_to_string_typed(scxml_path, template_dir).map_err(|e| e.to_string())
}

/// Typed analogue of [`compile_from_string`] — see the typed file-based
/// sibling for routing rationale.
pub fn compile_from_string_typed(
    scxml_content: &str,
    scxml_name: &str,
    templates: &[(&str, &str)],
) -> Result<String, CompileError> {
    let model = compile_model_from_string(scxml_content, scxml_name)?;
    generator::generate_with_templates(&model, templates, false)
        .map_err(|e| locate_codegen_error(e, scxml_name))
}

/// Compile SCXML content string to Rust code (no filesystem access).
///
/// This is the WASM-compatible API. Templates must be provided as (name, content) pairs.
/// String-returning shim over [`compile_from_string_typed`].
pub fn compile_from_string(
    scxml_content: &str,
    scxml_name: &str,
    templates: &[(&str, &str)],
) -> Result<String, String> {
    compile_from_string_typed(scxml_content, scxml_name, templates).map_err(|e| e.to_string())
}

/// Typed analogue of [`compile_from_string_lang`].
pub fn compile_from_string_lang_typed(
    scxml_content: &str,
    scxml_name: &str,
    templates: &[(&str, &str)],
    language: generator::Language,
) -> Result<generator::GeneratedOutput, CompileError> {
    let model = compile_model_from_string(scxml_content, scxml_name)?;

    // `from_string` callers have no filesystem-anchored preprocessor
    // pipeline — `parse_string` leaves `preprocessor_deps` empty by
    // construction (parser.rs docstring). So every branch returns a
    // `GeneratedOutput` whose `deps` defaults to the empty vec
    // populated by `..Default::default()` (or by the C++/C11 helper
    // returns, which already default `deps` via the derive). This
    // is intentional, not a leak — there is no dep channel to
    // forward.
    match language {
        generator::Language::Rust => {
            let code = generator::generate_with_templates(&model, templates, false)
                .map_err(|e| locate_codegen_error(e, scxml_name))?;
            Ok(generator::GeneratedOutput {
                files: vec![(format!("{scxml_name}_sm.rs"), code)],
                ..Default::default()
            })
        }
        generator::Language::Cpp => {
            generator::generate_cpp_with_templates(&model, templates, scxml_name)
                .map_err(|e| locate_codegen_error(e, scxml_name))
        }
        generator::Language::Kotlin => {
            let code = generator::generate_kotlin_with_templates(&model, templates, None)
                .map_err(|e| locate_codegen_error(e, scxml_name))?;
            Ok(generator::GeneratedOutput {
                files: vec![(format!("{scxml_name}Sm.kt"), code)],
                ..Default::default()
            })
        }
        generator::Language::Go => {
            let code = generator::generate_go_with_templates(&model, templates)
                .map_err(|e| locate_codegen_error(e, scxml_name))?;
            Ok(generator::GeneratedOutput {
                files: vec![(format!("{scxml_name}_sm.go"), code)],
                ..Default::default()
            })
        }
        generator::Language::Python => {
            let code = generator::generate_python_with_templates(&model, templates)
                .map_err(|e| locate_codegen_error(e, scxml_name))?;
            Ok(generator::GeneratedOutput {
                files: vec![(format!("{scxml_name}_sm.py"), code)],
                ..Default::default()
            })
        }
        generator::Language::C11 => {
            generator::generate_c11_with_templates(&model, templates, scxml_name)
                .map_err(|e| locate_codegen_error(e, scxml_name))
        }
    }
}

/// Compile SCXML content string for a specific language (WASM-compatible).
/// String-returning shim over [`compile_from_string_lang_typed`].
pub fn compile_from_string_lang(
    scxml_content: &str,
    scxml_name: &str,
    templates: &[(&str, &str)],
    language: generator::Language,
) -> Result<generator::GeneratedOutput, String> {
    compile_from_string_lang_typed(scxml_content, scxml_name, templates, language)
        .map_err(|e| e.to_string())
}

/// Typed analogue of [`compile_scxml_lang`].
pub fn compile_scxml_lang_typed(
    scxml_path: &str,
    template_dir: &Path,
    language: generator::Language,
) -> Result<generator::GeneratedOutput, CompileError> {
    compile_scxml_lang_typed_with_driver_root(scxml_path, template_dir, language, None)
}

/// Watching-zenoh RFC §5.2 Round F-α — deploy-aware variant of
/// [`compile_scxml_lang_typed`] that honours `deploy.yaml`'s
/// `platform.driver_root` override (Q-Round-F-D5). `driver_root: None`
/// means "fall back to the SCXML file's parent directory" so this is
/// a strict superset of the deploy-unaware entry — every existing
/// caller route remains byte-stable when no override is in play.
///
/// The orchestrator ([`compile_scxml_with_imports`]) selects both the
/// driver_root override and the F-α-2 C11 section attribute class per
/// machine and routes through [`compile_scxml_lang_typed_with_section`];
/// this entry is the no-section convenience wrapper that single-file
/// (deploy-unaware) callers keep using.
pub fn compile_scxml_lang_typed_with_driver_root(
    scxml_path: &str,
    template_dir: &Path,
    language: generator::Language,
    driver_root: Option<&Path>,
) -> Result<generator::GeneratedOutput, CompileError> {
    compile_scxml_lang_typed_with_section(scxml_path, template_dir, language, driver_root, None)
}

/// Watching-zenoh RFC §5.2 Round F-α-2 — deploy-aware variant that
/// additionally honours `deploy.yaml`'s `platform.c11_section_attribute.class`
/// for the C11 backend. When `section_class` is `Some("<name>")`, the
/// emitted `*_sm.c` defines `SCE_SM_FN` as
/// `__attribute__((section("<name>")))` and every statechart function
/// definition receives the prefix. When `None`, `SCE_SM_FN` expands to
/// empty so the emitted source stays byte-stable against the F-α
/// baseline (modulo the textual prefix token itself, which is part of
/// the round-trip lock).
///
/// `section_class` is ignored on non-C11 backends; the orchestrator
/// already fires `mcu/section-attribute-on-non-mcu-target` for that
/// case (Q-Round-F-D3) before this entry runs.
pub fn compile_scxml_lang_typed_with_section(
    scxml_path: &str,
    template_dir: &Path,
    language: generator::Language,
    driver_root: Option<&Path>,
    section_class: Option<&str>,
) -> Result<generator::GeneratedOutput, CompileError> {
    let ParsedSCXML {
        mut model,
        preprocessor_deps,
    } = compile_model(scxml_path)?;
    if !model.driver_refs.is_empty() {
        match driver_root {
            Some(root) => resolve_driver_refs_with_root(&mut model, scxml_path, root)?,
            None => resolve_driver_refs(&mut model, scxml_path)?,
        }
    }
    if matches!(language, generator::Language::C11) {
        model.c11_section_attribute_class = section_class.map(str::to_string);
    }

    let input_stem = Path::new(scxml_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown");

    // Run language-specific codegen first, then attach the parse-phase
    // deps to the resulting `GeneratedOutput`. Codegen helpers
    // (`generate*`) do not see the deps — they live one layer above,
    // bound to the parser exit. Attaching here means every backend
    // (single-file Rust/Kotlin/Go/Python + multi-file C++/C11) gets
    // the same dep channel without each `generate_*` learning about
    // build-system metadata it doesn't otherwise touch.
    let mut output = match language {
        generator::Language::Rust => {
            let code = generator::generate(&model, template_dir, false)
                .map_err(|e| locate_codegen_error(e, scxml_path))?;
            generator::GeneratedOutput {
                files: vec![(format!("{input_stem}_sm.rs"), code)],
                deps: Vec::new(),
            }
        }
        generator::Language::Cpp => generator::generate_cpp(&model, template_dir, input_stem)
            .map_err(|e| locate_codegen_error(e, scxml_path))?,
        generator::Language::Kotlin => {
            let code = generator::generate_kotlin(&model, template_dir, None)
                .map_err(|e| locate_codegen_error(e, scxml_path))?;
            generator::GeneratedOutput {
                files: vec![(format!("{input_stem}Sm.kt"), code)],
                deps: Vec::new(),
            }
        }
        generator::Language::Go => {
            let code = generator::generate_go(&model, template_dir)
                .map_err(|e| locate_codegen_error(e, scxml_path))?;
            generator::GeneratedOutput {
                files: vec![(format!("{input_stem}_sm.go"), code)],
                deps: Vec::new(),
            }
        }
        generator::Language::Python => {
            let code = generator::generate_python(&model, template_dir)
                .map_err(|e| locate_codegen_error(e, scxml_path))?;
            generator::GeneratedOutput {
                files: vec![(format!("{input_stem}_sm.py"), code)],
                deps: Vec::new(),
            }
        }
        generator::Language::C11 => generator::generate_c11(&model, template_dir, input_stem)
            .map_err(|e| locate_codegen_error(e, scxml_path))?,
    };
    output.deps = preprocessor_deps;
    Ok(output)
}

/// Compile SCXML file for a specific language (filesystem-based).
/// String-returning shim over [`compile_scxml_lang_typed`].
pub fn compile_scxml_lang(
    scxml_path: &str,
    template_dir: &Path,
    language: generator::Language,
) -> Result<generator::GeneratedOutput, String> {
    compile_scxml_lang_typed(scxml_path, template_dir, language).map_err(|e| e.to_string())
}

/// Locate template directory for a specific language.
pub fn find_template_dir_for(language: generator::Language) -> std::path::PathBuf {
    let subdir = match language {
        generator::Language::Rust => "rust",
        generator::Language::Cpp => "", // C++ templates at root
        generator::Language::Kotlin => "kotlin",
        generator::Language::Go => "go",
        generator::Language::Python => "python",
        // RFC §5.J.1: C11 statechart templates live at
        // `<root>/c/state_machine.{h,c}.jinja2`, but every backend
        // shares `license_header.jinja2` at the root. Returning the
        // root (matching the C++ arm) lets `load_templates` walk both
        // layers in one pass so `{% include 'license_header.jinja2' %}`
        // resolves to the SSoT copy without duplicating it under c/.
        generator::Language::C11 => "",
    };
    let base = find_template_base();
    if subdir.is_empty() {
        base
    } else {
        base.join(subdir)
    }
}

/// Compile a forge (non-statechart) SCXML from already-read content.
///
/// Uses single-parse path: detects kind and parses model in one XML parse.
///
/// The error type is [`Located<ForgeError>`]: location is part of the
/// error contract — every failure ties back to the `label` the caller
/// supplied, so downstream consumers (CLI diagnostics, build scripts,
/// agents) never have to attach file context after the fact.
pub fn compile_forge_from_string(
    content: &str,
    label: DocumentLabel<'_>,
    language: generator::Language,
) -> Result<generator::GeneratedOutput, forge::error::Located<forge::error::ForgeError>> {
    use forge::error::{Located, ValidationError};

    let doc = forge::parser::parse_forge(content, label)?.ok_or_else(|| {
        Located::new(
            ValidationError::WrongPipeline {
                kind: forge::model::ForgeKind::Statechart,
            }
            .into(),
            label.diagnostic_label,
            None,
            None,
        )
    })?;

    // Watching-zenoh RFC §5.O Atomic 0c — forge IR provenance pre-emit
    // guard. Mirrors the SCXML-side `validate_emission_provenance`
    // placement (compile_model* in this same file). The walker fires
    // `traceability/scxml-line-range-missing` when a per-kind body
    // emission would otherwise lose its SCE-MAP marker.
    forge::provenance::validate_forge_emission_provenance(&doc, label.diagnostic_label)?;

    let template_base = find_template_base();

    let output = match language {
        generator::Language::Cpp => forge::generator::generate_cpp(&doc, &template_base),
        generator::Language::Kotlin => forge::generator::generate_kotlin(&doc, &template_base),
        generator::Language::Rust => forge::generator::generate_rust(&doc, &template_base),
        generator::Language::Go => forge::generator::generate_go(&doc, &template_base),
        generator::Language::Python => forge::generator::generate_python(&doc, &template_base),
        generator::Language::C11 => forge::generator::generate_c11(&doc, &template_base),
    }
    .map_err(|e| Located::new(e, label.diagnostic_label, None, None))?;
    Ok(output)
}

/// Forge codegen entry that consumes deploy.yaml machine context for
/// validate-time OS-axis checks — RFC §5.C B6-η.
///
/// Layered on top of [`compile_forge_from_string`]: parses, then runs
/// per-kind deploy-aware validators, then generates. Today the only
/// validator wired here is the §5.C link-class × `platform.os` matrix
/// ([`forge::model::LinkClass::admits_os`]); future forge kinds opt
/// into deploy-aware validation by adding their own arms here without
/// affecting this entry's signature.
///
/// Both `deploy` and `target_machine` are `Option`-typed so the entry
/// stays usable in early-stage development where deploy.yaml is not
/// yet authored. When either is `None`, OR the resolved machine has
/// no `platform` block, the validate-time deploy checks are skipped
/// silently — matching the `validate_or_skip` convention used by
/// `xsd_validator.rs`. Authors who want strict OS-axis checking pass
/// `Some(&deploy)` + `Some("machine_name")` from their CLI/build
/// glue.
///
/// `compile_forge_from_string` remains the deploy-unaware entry; the
/// 6 existing link tests continue to use it and observe no behavioral
/// change from this addition.
pub fn compile_forge_with_deploy(
    content: &str,
    label: DocumentLabel<'_>,
    language: generator::Language,
    deploy: Option<&mesh::deploy::DeployConfig>,
    target_machine: Option<&str>,
) -> Result<generator::GeneratedOutput, forge::error::Located<forge::error::ForgeError>> {
    use forge::error::{Located, ValidationError};

    // watching-zenoh RFC §5.I Atomic B — load target plugin from
    // deploy.yaml `extern_symbols.target_plugin: <path>` (Q-Call-2 (a)
    // path-pointed YAML). Q-Call-6 (a) lock: plugin entries extend
    // the §5.I baseline registry; baseline-shadowing surfaces as
    // `extern/target-plugin-symbol-conflict` (spec line 1852 verbatim).
    // Absent deploy or absent extern_symbols ⇒ baseline-only registry,
    // matching atomic A semantics.
    let plugin_symbols = match deploy {
        Some(cfg) => load_target_plugin_for_compile(cfg, label.diagnostic_label)?,
        None => Vec::new(),
    };

    // C13 deferred-2: stateless_accept extern allowlist (watching-zenoh
    // RFC §5.K line 2466-2469). Single-doc path runs the same
    // allowlist check the orchestrator runs, against the same composed
    // (baseline ∪ plugin_symbols) registry. Failure short-circuits
    // before parse so callers that hit `compile_forge_with_deploy`
    // directly (CLI `sce-codegen generate --deploy=path`, in-process
    // tests) see the deploy-side diagnostic identically to the
    // orchestrator path.
    if let Some(cfg) = deploy {
        mesh::deploy::validate_stateless_accept_externs(cfg, &plugin_symbols).map_err(|e| {
            Located::new(
                forge::error::ForgeError::Mesh(Box::new(mesh::error::MeshError::from(e))),
                label.diagnostic_label,
                None,
                None,
            )
        })?;
    }

    let parsed =
        forge::parser::parse_forge_with_imports_and_plugin(content, label, &plugin_symbols)?
            .ok_or_else(|| {
                Located::new(
                    ValidationError::WrongPipeline {
                        kind: forge::model::ForgeKind::Statechart,
                    }
                    .into(),
                    label.diagnostic_label,
                    None,
                    None,
                )
            })?;
    let extern_decls = parsed.externs.clone();
    let mut doc = parsed.document;

    // RFC variant-default-overlay Atomic A — apply deploy.yaml
    // `variant_defaults:` onto the parsed IR before downstream
    // validators see it. The overlay flips `<sce:arm>` `is_default`
    // flags so the existing γ-3 `CodecVariantNoDefaultArm` validator
    // and the codegen-time arm-selection logic see a single
    // effective source of truth ("overlay if present, SCXML
    // `default=\"true\"` otherwise"). When deploy is `None`, the
    // SCXML's own `is_default` markers carry the choice unchanged —
    // the 107 `compile_forge_with_imports` call sites compile
    // identically.
    if let Some(cfg) = deploy {
        forge::variant_default_overlay::apply_variant_default_overlay(
            &mut doc,
            cfg,
            label.diagnostic_label,
        )?;
    }

    // Watching-zenoh RFC §5.O Atomic 0c — forge IR provenance pre-emit
    // guard. Runs before deploy-aware validators so the wire payload
    // anchors at the same `location.file` an η rejection would.
    forge::provenance::validate_forge_emission_provenance(&doc, label.diagnostic_label)?;

    // η deploy-aware validation. Resolved target_os is the
    // intersection of deploy + target_machine + machine.platform —
    // any missing piece skips silently per Q-η5 (a). When all three
    // are present, the per-kind validator fires on Link documents.
    //
    // `DeployConfig` nests machines under devices (`topology.<device>.
    // machines.<machine>`), so the lookup is two-step: device_for_machine
    // returns the owning `DeviceConfig`, then `.machines.get(name)` lands
    // on the `MachineConfig` whose `platform.os: OsKind` is the η axis.
    if let (Some(cfg), Some(machine_name)) = (deploy, target_machine) {
        let machine = cfg
            .device_for_machine(machine_name)
            .and_then(|d| d.machines.get(machine_name));
        if let Some(machine) = machine {
            if let Some(platform) = machine.platform.as_ref() {
                if let forge::model::ForgeDocument::Link(link) = &doc {
                    if !link.class.admits_os(platform.os) {
                        return Err(Located::new(
                            ValidationError::LinkClassUnsupportedOnTarget {
                                name: link.name.clone(),
                                class: link.class.to_string(),
                                target_os: platform.os.as_str().to_string(),
                                candidates: link
                                    .class
                                    .admitted_os_names()
                                    .into_iter()
                                    .map(|s| s.to_string())
                                    .collect(),
                            }
                            .into(),
                            label.diagnostic_label,
                            None,
                            None,
                        ));
                    }
                }
            }
            // RFC §5.E B7-α buffer-pool placement validation —
            // η-second-consumer pattern. Validates `<sce:section>` body
            // resolves against `machine.memory.sram_regions`. Skips
            // silently when the machine has no `memory` block (Q-η5 (a)
            // precedent). The candidates axis is the resolved machine's
            // declared region names (sorted) — drives `Fix::ReplaceOneOf`
            // so authors can pick a legal section or extend deploy.yaml.
            //
            // RFC §5.E B7-β layered size check — once the section has
            // resolved, verify the storage footprint (`slot_count ×
            // slot_size`) fits the resolved region's `size`. Section
            // resolution is the prerequisite: it makes no sense to
            // emit a size diagnostic against an unresolved section.
            // η-third-consumer pattern (B7-β second extension after
            // B7-α's placement check).
            if let forge::model::ForgeDocument::BufferPool(pool) = &doc {
                if let Some(memory) = machine.memory.as_ref() {
                    if let Some(region) = memory.sram_regions.get(&pool.section) {
                        let bytes_required: u64 =
                            (pool.slot_count as u64) * (pool.slot_size as u64);
                        if bytes_required > region.size {
                            return Err(Located::new(
                                ValidationError::BufferPoolTooLarge {
                                    name: pool.name.clone(),
                                    machine: machine_name.to_string(),
                                    section: pool.section.clone(),
                                    slot_count: pool.slot_count,
                                    slot_size: pool.slot_size,
                                    bytes_required,
                                    region_size: region.size,
                                }
                                .into(),
                                label.diagnostic_label,
                                None,
                                None,
                            ));
                        }
                    } else {
                        let mut candidates: Vec<String> =
                            memory.sram_regions.keys().cloned().collect();
                        candidates.sort();
                        return Err(Located::new(
                            ValidationError::BufferPoolSectionConflict {
                                name: pool.name.clone(),
                                machine: machine_name.to_string(),
                                section: pool.section.clone(),
                                candidates,
                            }
                            .into(),
                            label.diagnostic_label,
                            None,
                            None,
                        ));
                    }
                }

                // ── C5 cache-policy validators (RFC §5.E lines 1543-1545 + 1553) ──
                //
                // Four deploy-aware diagnostics keyed off `platform.has_dcache`
                // / `platform.dcache_line_size` / `platform.has_speculative_prefetch`
                // and the pool's `cache_policy`. Q-η5 (a) silent-skip when
                // the platform block is missing fields (the deploy.yaml-side
                // codes `deploy/has-dcache-missing` + `deploy/dcache-line-size-missing`
                // sit in §5.K / C13 scope — not C5's reach). One exception:
                // `pool/speculative-prefetch-flag-missing` fires even when
                // the field is unset, because the pool's `cache-policy:
                // maintain` makes the field a per-pool requirement, not a
                // schema-shape question.
                if let Some(platform) = machine.platform.as_ref() {
                    let policy_label = match pool.cache_policy {
                        forge::model::CachePolicy::Maintain => Some("maintain"),
                        forge::model::CachePolicy::NonCacheable => Some("non-cacheable"),
                        forge::model::CachePolicy::None => None,
                    };

                    // (1) cache-policy: maintain | non-cacheable on a core
                    //     declared `has_dcache: false` (spec line 1543).
                    if matches!(platform.has_dcache, Some(false)) {
                        if let Some(label_str) = policy_label {
                            return Err(Located::new(
                                ValidationError::BufferPoolCachePolicyUnsupportedOnNoDcacheCore {
                                    name: pool.name.clone(),
                                    machine: machine_name.to_string(),
                                    declared_policy: label_str.to_string(),
                                }
                                .into(),
                                label.diagnostic_label,
                                None,
                                None,
                            ));
                        }
                    }

                    if pool.cache_policy == forge::model::CachePolicy::Maintain {
                        // (2) alignment vs dcache_line_size (spec line 1544)
                        //     and (3) slot_size vs dcache_line_size (spec
                        //     line 1545) — both fire only when the deploy
                        //     declares `dcache_line_size`. Per Q-η5 (a),
                        //     missing field skips silently (it's a §5.K
                        //     completeness rule).
                        if let Some(line_size) = platform.dcache_line_size {
                            if pool.alignment < line_size {
                                return Err(Located::new(
                                    ValidationError::BufferPoolCacheLineAlignment {
                                        name: pool.name.clone(),
                                        machine: machine_name.to_string(),
                                        pool_alignment: pool.alignment,
                                        dcache_line_size: line_size,
                                    }
                                    .into(),
                                    label.diagnostic_label,
                                    None,
                                    None,
                                ));
                            }
                            let remainder = pool.slot_size % line_size;
                            if remainder != 0 {
                                let next_multiple = pool.slot_size + (line_size - remainder);
                                return Err(Located::new(
                                    ValidationError::BufferPoolSlotSizeNotCacheLineMultiple {
                                        name: pool.name.clone(),
                                        machine: machine_name.to_string(),
                                        slot_size: pool.slot_size,
                                        dcache_line_size: line_size,
                                        remainder,
                                        next_multiple,
                                    }
                                    .into(),
                                    label.diagnostic_label,
                                    None,
                                    None,
                                ));
                            }
                        }

                        // (4) `has_speculative_prefetch` REQUIRED when
                        //     `cache-policy: maintain` reaches a machine
                        //     with `has_dcache: true` (spec line 1553).
                        //     The field's value materially changes
                        //     correctness on M7+ cores; silent default
                        //     in either direction would violate
                        //     `feedback_silently_broken_hooks.md`.
                        if matches!(platform.has_dcache, Some(true))
                            && platform.has_speculative_prefetch.is_none()
                        {
                            return Err(Located::new(
                                ValidationError::PoolSpeculativePrefetchFlagMissing {
                                    machine: machine_name.to_string(),
                                    pool_name: pool.name.clone(),
                                }
                                .into(),
                                label.diagnostic_label,
                                None,
                                None,
                            ));
                        }
                    }
                }
            }
        }
    }

    // C2-γ: forge-side anchor for spec §5.D line 912
    // (`worker/scheduler-unsupported`). When a Worker doc compiles
    // against a resolved target machine, the machine MUST list it in
    // `machines.<m>.workers` so the cooperative scheduler can budget
    // a tick slot. Silent-skip when the deploy or target_machine is
    // absent (Q-η5 (a) precedent — deploy-unaware paths cannot enforce
    // slot accounting); the deploy-side sum check
    // (`deploy/scheduler-incompatible-with-worker-count`) catches the
    // counterpart violation at deploy.yaml parse time.
    if let (Some(cfg), Some(machine_name)) = (deploy, target_machine) {
        if let forge::model::ForgeDocument::Worker(worker) = &doc {
            if let Some(machine) = cfg
                .device_for_machine(machine_name)
                .and_then(|d| d.machines.get(machine_name))
            {
                if !machine.workers.contains_key(&worker.name) {
                    return Err(Located::new(
                        ValidationError::WorkerSchedulerUnsupported {
                            worker_name: worker.name.clone(),
                            machine: machine_name.to_string(),
                        }
                        .into(),
                        label.diagnostic_label,
                        None,
                        None,
                    ));
                }
            }
        }
    }

    // C1: forge-side anchor for spec §5.D line 909
    // (`timer/period-below-tick-rate`). When a Timer doc compiles
    // against a resolved cooperative-scheduler machine, the timer's
    // `<sce:period>` MUST be >= `scheduler.tick_period_us` so the
    // dispatcher can hit every deadline. Silent-skip when:
    // - deploy or target_machine is absent (Q-η5 (a)),
    // - the machine has no scheduler block,
    // - `scheduler.kind` is not cooperative (preemptive runtimes
    //   own their own dispatch granularity),
    // - `scheduler.tick_period_us` is absent (the comparison has
    //   no reference point).
    if let (Some(cfg), Some(machine_name)) = (deploy, target_machine) {
        if let forge::model::ForgeDocument::Timer(timer) = &doc {
            if let Some(machine) = cfg
                .device_for_machine(machine_name)
                .and_then(|d| d.machines.get(machine_name))
            {
                if let Some(sched) = machine.scheduler.as_ref() {
                    if matches!(sched.kind, mesh::deploy::SchedulerKind::Cooperative) {
                        if let Some(tick_period_us) = sched.tick_period_us {
                            if timer.period_us < tick_period_us as u64 {
                                return Err(Located::new(
                                    ValidationError::TimerPeriodBelowTickRate {
                                        timer_name: timer.name.clone(),
                                        machine: machine_name.to_string(),
                                        period_us: timer.period_us,
                                        tick_period_us,
                                    }
                                    .into(),
                                    label.diagnostic_label,
                                    None,
                                    None,
                                ));
                            }
                        }
                    }
                }
            }
        }

        // ── C6-γ1 Bounded-collection deploy-time capacity resolution ──
        //
        // RFC §5.L lines 2583-2585: `<sce:capacity source="deploy"
        // key="machines.<m>.limits.<k>"/>` resolves at codegen time to
        // a per-language compile-time constant from the named limit
        // under `machines.<m>.limits:` in deploy.yaml. Fires
        // `collection/capacity-unresolved` when the key references the
        // target machine but the limit is not declared.
        //
        // Silent-skip paths per Q-η5 (a):
        //   - `<sce:capacity const="N"/>` carries no deploy reference;
        //     no validation needed.
        //   - Key does not match the `machines.<m>.limits.<k>` shape
        //     (likely a different deploy schema author convention or
        //     malformed key — α parser accepts opaque strings; β/γ
        //     resolve only the shape spec defines).
        //   - Key's machine segment != target_machine (BC doc designed
        //     for a different machine; deploy resolution runs only on
        //     the host machine's compile).
        if let forge::model::ForgeDocument::BoundedCollection(bc) = &doc {
            if let forge::model::CapacitySource::DeployKey { key } = &bc.capacity {
                if let Some((machine_segment, limit_name)) =
                    parse_bounded_collection_deploy_key(key)
                {
                    if machine_segment == machine_name {
                        if let Some(machine) = cfg
                            .device_for_machine(machine_name)
                            .and_then(|d| d.machines.get(machine_name))
                        {
                            if !machine.limits.contains_key(limit_name) {
                                let mut candidates: Vec<String> =
                                    machine.limits.keys().cloned().collect();
                                candidates.sort();
                                let candidates_list = candidates.join(", ");
                                return Err(Located::new(
                                    ValidationError::CollectionCapacityUnresolved {
                                        collection_name: bc.name.clone(),
                                        key: key.clone(),
                                        machine: machine_name.to_string(),
                                        limit: limit_name.to_string(),
                                        candidates,
                                        candidates_list,
                                    }
                                    .into(),
                                    label.diagnostic_label,
                                    None,
                                    None,
                                ));
                            }
                        }
                    }
                }
            }
        }
    }

    // C5: build the cache_platform options threading from the resolved
    // machine's platform block. Fires only on the deploy-aware path
    // (deploy + target_machine + machine.platform all resolved). When
    // any piece is missing, `cache_platform` stays `None` and the
    // generator falls back to conservative defaults — but the
    // validators above guarantee the field IS resolved whenever a
    // `cache-policy: maintain` pool reaches the codegen layer with
    // `has_dcache: true`.
    let cache_platform = (|| -> Option<CachePlatformInfo> {
        let cfg = deploy?;
        let machine_name = target_machine?;
        let device = cfg.device_for_machine(machine_name)?;
        let machine = device.machines.get(machine_name)?;
        let platform = machine.platform.as_ref()?;
        Some(CachePlatformInfo {
            has_speculative_prefetch: platform.has_speculative_prefetch.unwrap_or(false),
        })
    })();

    // C2-γ: build the worker_placement options threading from the
    // resolved machine's `workers:` block. Fires only on the deploy-
    // aware path with at least one worker declared. Mirrors the
    // cache_platform populator pattern (C5) — `compile_forge_with_imports`
    // never reaches here and worker_placement stays `None`, matching
    // the C2-β codegen-invariant validator's silent-skip on missing
    // placement (Q-η5 (a) precedent).
    let worker_placement = (|| -> Option<Vec<WorkerPlacement>> {
        let cfg = deploy?;
        let machine_name = target_machine?;
        let device = cfg.device_for_machine(machine_name)?;
        let machine = device.machines.get(machine_name)?;
        if machine.workers.is_empty() {
            return None;
        }
        let mut placements: Vec<WorkerPlacement> = machine
            .workers
            .iter()
            .filter_map(|(worker_name, worker_cfg)| {
                worker_cfg.placement.as_ref().map(|p| WorkerPlacement {
                    worker_name: worker_name.clone(),
                    producer_core: p.producer_core,
                    consumer_core: p.consumer_core,
                })
            })
            .collect();
        if placements.is_empty() {
            return None;
        }
        // Deterministic order so downstream codegen-invariant scans
        // are byte-stable across HashMap iteration order.
        placements.sort_by(|a, b| a.worker_name.cmp(&b.worker_name));
        Some(placements)
    })();
    // C6-γ2: bounded-collection capacity resolution. Single-doc path
    // so the map carries at most one entry. CompileConst BCs copy the
    // literal through for uniform render handling (the render layer
    // never needs to read `m.capacity` when this map is populated).
    // DeployKey BCs reuse the γ1 lookup performed above — the validator
    // returned Ok ⇒ the value MUST be present in `machine.limits`. The
    // `index_by_field_sce_type` axis stays `None` because this path
    // lacks the orchestrator's element-type candidate map; BCs going
    // through `compile_forge_with_deploy` with `<sce:index-by>`
    // declared raise `InvalidConfig` at the render layer per Q-γ2
    // upstream-honesty discipline.
    let bounded_collection_resolutions: Option<
        std::collections::HashMap<String, BoundedCollectionResolution>,
    > = (|| -> Option<_> {
        let forge::model::ForgeDocument::BoundedCollection(bc) = &doc else {
            return None;
        };
        let capacity: u32 = match &bc.capacity {
            forge::model::CapacitySource::CompileConst { value } => *value,
            forge::model::CapacitySource::DeployKey { key } => {
                let cfg = deploy?;
                let machine_name = target_machine?;
                let (machine_segment, limit_name) = parse_bounded_collection_deploy_key(key)?;
                if machine_segment != machine_name {
                    return None;
                }
                let machine = cfg
                    .device_for_machine(machine_name)
                    .and_then(|d| d.machines.get(machine_name))?;
                *machine.limits.get(limit_name)?
            }
        };
        let mut map = std::collections::HashMap::new();
        map.insert(
            bc.name.clone(),
            BoundedCollectionResolution {
                capacity,
                index_by_field_sce_type: None,
            },
        );
        Some(map)
    })();

    let options = ForgeCompileOptions {
        cache_platform,
        worker_placement,
        bounded_collection_resolutions,
        ..Default::default()
    };

    // C2-γ: connect the worker_placement populator to its C2-β
    // codegen-invariant consumer. `compile_forge_with_imports` runs
    // this validator on the imports path; `compile_forge_with_deploy`
    // needs to run it equivalently so the deploy.yaml-populated
    // placement reaches the cross-core ordering check. Without this
    // wire, `worker_placement` would be built-but-unconsumed under
    // the deploy-aware path (`feedback_silently_broken_hooks.md`
    // violation).
    validate_worker_inbox_ordering_placement(
        &doc,
        options.worker_placement.as_deref(),
        label.diagnostic_label,
    )?;

    // C2-γ: same rationale for cross-resolution. The `<sce:link-rx>`
    // ref must resolve to a declared kind=link import; under
    // `compile_forge_with_deploy` the validator was previously
    // missing — re-wiring closes the gap.
    validate_worker_cross_refs(&doc, &parsed.imports, label.diagnostic_label)?;

    let template_base = find_template_base();

    // RFC §5.I Atomic C / Q-Call-7 — `<sce:extern>` rejected on
    // non-MCU backends. Mirrors the gate in
    // [`compile_forge_with_imports`]; the deploy-aware path catches
    // the rejection one stage later (after potential plugin loading)
    // because plugin entries also count as `<sce:extern>` declarations
    // through `parsed.externs`. Reuses the existing
    // `codegen/mcu-class-kind-on-non-mcu-language` family per Q-Call-7
    // prose, with `kind = "<sce:extern>"` to disambiguate from kind-
    // axis rejection on the same code.
    if !extern_decls.is_empty()
        && matches!(
            language,
            generator::Language::Kotlin | generator::Language::Go | generator::Language::Python
        )
    {
        return Err(Located::new(
            forge::error::GenerateError::CodegenMcuClassKindOnNonMcuLanguage {
                kind: "<sce:extern>".to_string(),
                language: language_wire_name(language).to_string(),
            }
            .into(),
            label.diagnostic_label,
            None,
            None,
        ));
    }

    let output = match language {
        generator::Language::Cpp => forge::generator::generate_cpp_with_imports_and_externs(
            &doc,
            &template_base,
            &[],
            &extern_decls,
            &options,
        ),
        generator::Language::Kotlin => forge::generator::generate_kotlin(&doc, &template_base),
        generator::Language::Rust => forge::generator::generate_rust_with_imports_and_externs(
            &doc,
            &template_base,
            &[],
            &extern_decls,
            &options,
        ),
        generator::Language::Go => forge::generator::generate_go(&doc, &template_base),
        generator::Language::Python => forge::generator::generate_python(&doc, &template_base),
        generator::Language::C11 => forge::generator::generate_c11_with_imports_and_externs(
            &doc,
            &template_base,
            &[],
            &extern_decls,
            &options,
        ),
    }
    .map_err(|e| Located::new(e, label.diagnostic_label, None, None))?;

    // C5 codegen-invariant guard: `pool/cache-pre-arm-invalidate-missing-on-speculative-core`
    // (spec line 1552). When the resolved platform has
    // `has_speculative_prefetch: true` AND the pool declares
    // `cache-policy: maintain`, the rendered source MUST contain a
    // `sce_dcache_invalidate_by_addr` call. The post-render scan
    // catches a future template edit that drops the pre-arm RX
    // invalidate edge, surfacing the regression as a typed
    // diagnostic rather than silently corrupting RX data on M7+
    // cores. Only meaningful for Rust + C11 (the two backends that
    // emit buffer-pool); Cpp/Go/Kotlin/Python don't produce
    // buffer-pool output and skip silently.
    if matches!(
        language,
        generator::Language::Rust | generator::Language::C11
    ) {
        if let forge::model::ForgeDocument::BufferPool(pool) = &doc {
            if pool.cache_policy == forge::model::CachePolicy::Maintain {
                if let Some(plat) = options.cache_platform.as_ref() {
                    if plat.has_speculative_prefetch {
                        let backend = match language {
                            generator::Language::Rust => "rust",
                            generator::Language::C11 => "c11",
                            _ => unreachable!(),
                        };
                        let primary = output.files.first().map_or("", |(_, src)| src.as_str());
                        if !primary.contains("sce_dcache_invalidate_by_addr") {
                            return Err(Located::new(
                                ValidationError::PoolCachePreArmInvalidateMissingOnSpeculativeCore {
                                    name: pool.name.clone(),
                                    backend: backend.to_string(),
                                }
                                .into(),
                                label.diagnostic_label,
                                None,
                                None,
                            ));
                        }
                    }
                }
            }
        }
    }

    Ok(output)
}

/// Resolve the deploy.yaml `extern_symbols.target_plugin` field into
/// a [`forge::target_plugin::PluginSymbol`] vector for the
/// plugin-aware parser (Atomic B consumer wiring).
///
/// Failure mapping (Atomic B scope = 1 new spec-verbatim code):
///
/// - `BaselineConflict` → [`forge::error::ValidationError::ExternTargetPluginSymbolConflict`]
///   (spec line 1852 verbatim, the new Atomic B code).
/// - `ReadFile` / `Yaml` / `UnknownAbi` → [`forge::error::ForgeError::Io`]
///   (existing `io/filesystem` code). The plugin file lives outside
///   the SCXML pipeline; treating its load failures as I/O on the
///   pipeline's input set keeps atomic B's wire-code surface bounded
///   to the spec-verbatim conflict axis. A future Atomic C may
///   promote these to a dedicated `extern/target-plugin-load`
///   family if UX feedback warrants the split.
fn load_target_plugin_for_compile(
    cfg: &mesh::deploy::DeployConfig,
    diag_label: &str,
) -> Result<Vec<forge::target_plugin::PluginSymbol>, forge::error::Located<forge::error::ForgeError>>
{
    use forge::error::{ForgeError, Located, ValidationError};
    use forge::target_plugin::{parse_target_plugin_yaml, TargetPluginLoadError};

    let plugin_path = match cfg
        .extern_symbols
        .as_ref()
        .and_then(|es| es.target_plugin.as_ref())
    {
        Some(p) => p,
        None => return Ok(Vec::new()),
    };

    match parse_target_plugin_yaml(plugin_path) {
        Ok(symbols) => Ok(symbols),
        Err(TargetPluginLoadError::ReadFile { path, source }) => Err(Located::new(
            ForgeError::Io {
                path: std::path::PathBuf::from(path),
                source,
            },
            diag_label,
            None,
            None,
        )),
        Err(TargetPluginLoadError::Yaml { path, source }) => Err(Located::new(
            ForgeError::Io {
                path: std::path::PathBuf::from(path),
                source: std::io::Error::new(std::io::ErrorKind::InvalidData, source.to_string()),
            },
            diag_label,
            None,
            None,
        )),
        Err(TargetPluginLoadError::UnknownAbi { path, name, abi }) => Err(Located::new(
            ForgeError::Io {
                path: std::path::PathBuf::from(path),
                source: std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!(
                        "plugin symbol `{name}` declares unknown ABI `{abi}`; only `c` and `rust` are accepted (Q-Call-3 closed set)"
                    ),
                ),
            },
            diag_label,
            None,
            None,
        )),
        Err(TargetPluginLoadError::BaselineConflict { path, name }) => Err(Located::new(
            ValidationError::ExternTargetPluginSymbolConflict {
                name,
                plugin_path: path,
            }
            .into(),
            diag_label,
            None,
            None,
        )),
    }
}

/// RFC c7-wildcard W-project: one element-type's field schema, in
/// declaration order. Each entry is `(field_id, SceType, length_field)`,
/// where `length_field` is the codec's explicit `sce:length-field` — the
/// C11 borrowed-byte-view length sibling SSOT — or `None` for a tail/fixed
/// `bytes` field (auto `<field>_len`) or a procedure field.
pub type ElementFieldSchema = Vec<(String, forge::model::SceType, Option<String>)>;

/// Options that steer forge cross-file codegen beyond the plain
/// `(content, name, language, base_dir)` tuple. New language-specific
/// knobs get added as fields here so `compile_forge_with_imports`
/// itself never grows a second parameter.
#[derive(Default, Clone, Debug)]
pub struct ForgeCompileOptions {
    /// Go module path under which the generated package directories live
    /// (e.g. `github.com/newmassrael/sce-forge-runtime/conformance/generated`).
    /// When set, every `<sce:import>` emits `import "{prefix}/{snake}"` in
    /// Go output. Go cross-file imports are only valid inside a Go
    /// module, so `resolve_imports` hard-errors when imports are present
    /// for `Language::Go` but this field is `None` — no silent fallback.
    pub go_module_prefix: Option<String>,
    /// RFC §5.F build-time const-fold iteration budget. When set, caps
    /// the total iteration count across every `<sce:fold>` body in the
    /// document. `None` = use the SSoT default
    /// ([`forge::const_fold::Budget::DEFAULT_MAX_ITERS`] = 1_000_000),
    /// matching the RFC's "default 1M" wording. The CLI surfaces this
    /// as `--const-fold-budget=N` on the `generate` subcommand.
    pub const_fold_budget: Option<u64>,
    /// RFC §5.E C5 cache-maintenance platform info. Populated by
    /// [`compile_forge_with_deploy`] from the resolved
    /// [`mesh::deploy::PlatformConfig`]; left `None` by deploy-unaware
    /// callers (`compile_forge_with_imports`, `sce_codegen` CLI, in-
    /// process test harnesses). When `None`, the buffer-pool template
    /// uses conservative defaults (no pre-arm cache-invalidate edge),
    /// which is correct for non-`maintain` cache policies and for
    /// targets without speculative prefetch. The deploy-aware path's
    /// validators ensure the field is always `Some` when at least one
    /// `cache-policy: maintain` pool exists.
    pub cache_platform: Option<CachePlatformInfo>,
    /// RFC §5.D + §5.I C2-β worker inbox cross-core placement map.
    /// Populated by [`compile_forge_with_deploy`] from the resolved
    /// deploy.yaml `machines.<m>.workers.<w>.placement` block (lands
    /// in C2-γ alongside `MachineSchedulerConfig`); left `None` by
    /// deploy-unaware callers. The codegen-invariant validator
    /// [`validate_worker_inbox_ordering_placement`] silent-skips on
    /// `None` per the Q-η5 (a) precedent — `compile_forge_with_imports`
    /// does not have the cross-core information needed to fire
    /// `worker/inbox-ordering-relaxed-across-cores`. When `Some`, the
    /// validator scans the slice for any entry whose producer/consumer
    /// cores differ and whose worker doc declared `ordering="relaxed"`.
    pub worker_placement: Option<Vec<WorkerPlacement>>,
    /// RFC §5.L C6-γ bounded-collection codegen-time resolutions, keyed
    /// by `BoundedCollectionModel.name`. Populated by the two upstream
    /// pipelines that have the information the BC template needs:
    ///
    /// * [`compile_forge_with_deploy`] populates the entry's `capacity`
    ///   from the resolved `<sce:capacity source="deploy" key=...>`
    ///   value (γ1's `machines.<m>.limits.<k>` lookup). Single-doc path
    ///   so the map has one entry.
    /// * [`compile_scxml_with_imports`] populates `index_by_field_sce_type`
    ///   from the resolved element-type ForgeDocument (codec / procedure)
    ///   for every BC with `<sce:index-by>` declared. Multi-doc path so
    ///   the map carries one entry per BC.
    ///
    /// `None` on deploy-unaware single-file paths
    /// ([`compile_forge_with_imports`], `sce_codegen` CLI). When `None`
    /// AND the BC declares `<sce:capacity source="deploy">` OR
    /// `<sce:index-by>`, the render layer raises a
    /// [`forge::error::GenerateError::InvalidConfig`] naming the BC and
    /// the missing piece — keeping the resolution invariant honest at
    /// the codegen boundary rather than silently emitting placeholders.
    pub bounded_collection_resolutions:
        Option<std::collections::HashMap<String, BoundedCollectionResolution>>,
    /// watching-zenoh RFC §5.C lines 802-833 + §5.M lines 2771-2828
    /// (C10-α) — sorted set of `<sce:link>` doc names whose
    /// orchestrator-resolved (deploy `domain_attrs.trust_class:
    /// session_arming` × machine source SCXML `Accepting.*`
    /// substate-present) pair makes the link a listener. Drives
    /// per-language `render_link_*` sibling emission: when this set
    /// contains the rendered link's name, the template emits BOTH a
    /// Listener half (existing shape) AND a Sibling half (durable
    /// `EstablishedSession` type-name suffix per Q-C10-3 a). The
    /// post-render substring self-check
    /// `link/listener-link-not-paired-with-established-sibling` greps
    /// for the durable suffix and fires on template regression.
    ///
    /// Populated by [`compile_scxml_with_imports`] when `deploy:
    /// Some`; left `None` by deploy-unaware callers
    /// ([`compile_forge_with_imports`], `sce_codegen` CLI on single
    /// forge docs without deploy). When `None`, the codegen path
    /// emits the existing single-instance Listener-only shape — no
    /// sibling synthesized, no self-check — matching pre-C10 behavior
    /// verbatim.
    pub listener_links: Option<std::collections::BTreeSet<String>>,
    /// RFC c7-wildcard W-project: element-type `(field_id, SceType)`
    /// schemas keyed by the element-type's **snake-cased name** (the same
    /// form [`ImportContext::bc_element_snake`] carries —
    /// `to_snake_case(BoundedCollectionModel.element_type)`). Populated by
    /// [`compile_scxml_with_imports`] from `element_type_candidates` so an
    /// algorithm iterating a BC (`<sce:foreach item="entry" in="keys">`)
    /// can type each `entry.<field>` access — `entry.pattern` → `Str`,
    /// `entry.callback_id` → `uint32`. That inference is what the
    /// bounded-string-field → borrowed-`bytes`-view call-site projection
    /// (Q-W-5 (a) lock) keys on. Keyed by element name (not BC name)
    /// because `render_algorithm` recovers the element snake from the BC
    /// import's `bc_element_snake`, whereas the BC's own model name is not
    /// reliably reconstructible from the file-stem-derived import context.
    /// `None` on deploy-unaware single-file paths (`sce_codegen` CLI,
    /// `compile_forge_with_imports`); the projection then does not fire and
    /// the arg falls through verbatim, exactly as in pre-W-project C7.
    pub element_type_field_schemas: Option<std::collections::HashMap<String, ElementFieldSchema>>,
}

/// RFC §5.D + §5.I C2-β cross-core worker placement entry. Populated
/// from deploy.yaml's `machines.<m>.workers.<w>.placement.{producer_core,
/// consumer_core}` block at [`compile_forge_with_deploy`] time and
/// threaded to the inbox-ordering validator via
/// [`ForgeCompileOptions::worker_placement`]. C2-β ships the
/// validator + the wire-format `worker/inbox-ordering-relaxed-across-cores`
/// code; the deploy.yaml schema field + the parser that populates it
/// land in C2-γ alongside `MachineSchedulerConfig`. Until C2-γ ships,
/// the slice is always `None` in production paths — the test suite
/// constructs populated options to exercise the validator end-to-end.
#[derive(Clone, Debug)]
pub struct WorkerPlacement {
    /// Worker doc name (forge `<scxml sce:kind="worker" name="...">`).
    /// Matches `WorkerModel.name` verbatim.
    pub worker_name: String,
    /// Core index hosting the inbox producer (link-rx-driven path).
    /// Zero-based per the deploy.yaml convention.
    pub producer_core: u32,
    /// Core index hosting the inbox consumer (the worker's own SCXML
    /// processing thread).
    pub consumer_core: u32,
}

/// RFC §5.E C5 cache-maintenance codegen-relevant platform invariants.
/// Aggregated from [`mesh::deploy::PlatformConfig`] at
/// [`compile_forge_with_deploy`] time and threaded to the buffer-pool
/// generator via [`ForgeCompileOptions::cache_platform`].
#[derive(Clone, Debug)]
pub struct CachePlatformInfo {
    /// `true` for cores with speculative load / hardware prefetcher
    /// (Cortex-M7+, Cortex-A series). Drives the `free → dma-armed-rx`
    /// pre-arm cache-invalidate edge per spec §5.E lines 1189-1198 +
    /// 1199-1212. Validation in `compile_forge_with_deploy` enforces
    /// the field is set when `has_dcache=true` AND at least one
    /// `cache-policy: maintain` pool exists; missing config raises
    /// `pool/speculative-prefetch-flag-missing` (spec line 1553).
    pub has_speculative_prefetch: bool,
}

/// RFC §5.L C6-γ2 codegen-time resolution bundle for a single
/// bounded-collection document. Both fields are populated upstream
/// — the BC render layer simply reads what it needs and raises
/// `InvalidConfig` when a declared schema feature has no resolution.
///
/// `capacity` covers spec lines 2583-2585: `<sce:capacity
/// source="deploy" key=...>` resolves to a `u32` value from
/// `machines.<m>.limits.<k>`. For `<sce:capacity const="N">` the
/// upstream populator copies the literal so the render layer treats
/// both sources uniformly.
///
/// `index_by_field_sce_type` covers spec line 2615: when
/// `<sce:index-by field="...">` is declared, the orchestrator
/// extracts the abstract [`forge::model::SceType`] of that field
/// from the resolved element-type doc (codec field type / procedure
/// input or internal type). Each backend's render fn converts the
/// abstract type into the language-specific string at codegen time
/// via the existing `rust_type` / `cpp_type` / `kotlin_type` / `c_type` /
/// etc helpers — keeping the IR backend-neutral and the conversion
/// table single-sourced. `None` when no `<sce:index-by>` is set OR
/// when the upstream path cannot resolve it; the render layer
/// rejects the latter as `InvalidConfig` rather than silently
/// dropping the emit.
#[derive(Clone, Debug)]
pub struct BoundedCollectionResolution {
    /// Resolved capacity (spec lines 2571 + 2583-2585). For deploy-key
    /// BCs this is the lookup result; for compile-const BCs this is
    /// the literal `<sce:capacity const="N">` value copied through
    /// for uniform render handling.
    pub capacity: u32,
    /// Abstract field type for the `<sce:index-by field>` axis.
    /// `None` when `<sce:index-by>` is not declared. `Some` populated
    /// by the orchestrator from the resolved element-type doc.
    /// Each backend's render fn converts via its own type-string
    /// helper — γ2 emits `rust_type(...)`, γ3 emits `cpp_type(...)` /
    /// `kotlin_type(...)`, γ4 emits `c_type(...)` / Go / Python.
    pub index_by_field_sce_type: Option<forge::model::SceType>,
}

/// Compile a forge SCXML with cross-file import resolution, validation,
/// and language-specific codegen.
///
/// For each `<sce:import>`, validates the file exists relative to
/// `base_dir` and that its declared `kind` matches the actual kind in
/// the file, then renders the forge document for `language` with the
/// supplied `options`. This is the single entry point for crossfile
/// forge codegen — the CLI, test harness, and in-process build scripts
/// all go through here.
pub fn compile_forge_with_imports(
    content: &str,
    label: DocumentLabel<'_>,
    language: generator::Language,
    base_dir: &Path,
    options: &ForgeCompileOptions,
) -> Result<generator::GeneratedOutput, forge::error::Located<forge::error::ForgeError>> {
    use forge::error::{Located, ValidationError};

    let parsed = forge::parser::parse_forge_with_imports(content, label)?.ok_or_else(|| {
        Located::new(
            ValidationError::WrongPipeline {
                kind: forge::model::ForgeKind::Statechart,
            }
            .into(),
            label.diagnostic_label,
            None,
            None,
        )
    })?;
    compile_forge_from_parsed(&parsed, label, language, base_dir, options)
}

/// Forge codegen entry point for callers who already hold a
/// [`forge::model::ParsedForge`].
///
/// Splits the parse step out of [`compile_forge_with_imports`] so
/// downstream consumers — most importantly the `sce-codegen` CLI when
/// `--emit-ast=<path>` is set — can parse exactly once, observe or
/// emit the parsed IR, and only then trigger codegen. Eliminates the
/// previous double-parse on the AST emit path: parser cost is bounded
/// today but the typed-expression pipeline and XSD validator are
/// growth axes, and re-parsing the same document twice is an
/// architecture mismatch (`ParsedForge` is a cacheable artefact —
/// callers should not have to throw it away just to get codegen).
///
/// Semantics are byte-identical to `compile_forge_with_imports`
/// running on the same input: the wrapper now just funnels through
/// here after parsing. Existing call sites do not change.
pub fn compile_forge_from_parsed(
    parsed: &forge::model::ParsedForge,
    label: DocumentLabel<'_>,
    language: generator::Language,
    base_dir: &Path,
    options: &ForgeCompileOptions,
) -> Result<generator::GeneratedOutput, forge::error::Located<forge::error::ForgeError>> {
    use forge::error::Located;

    // Watching-zenoh RFC §5.O Atomic 0c — forge IR provenance pre-emit
    // guard. Runs before import resolution + cross-doc validators so a
    // missing-provenance regression in any parser site surfaces with
    // its own diagnostic instead of cascading into a downstream
    // import / cross-resolution error.
    forge::provenance::validate_forge_emission_provenance(
        &parsed.document,
        label.diagnostic_label,
    )?;

    let template_base = find_template_base();
    let mut import_ctx = forge::generator::resolve_imports(&parsed.imports, &language, options)
        .map_err(|e| Located::new(e, label.diagnostic_label, None, None))?;

    validate_and_enrich_imports(
        &mut import_ctx,
        &parsed.imports,
        base_dir,
        &language,
        options,
        label.diagnostic_label,
    )?;

    // NL→IR Mapping Roadmap Item 2 — cross-kind typed binding
    // verification. Runs after import enrichment populates the
    // per-import slice (the validator reads its own member surface off
    // the import file contents rather than depending on enrichment
    // data, but the order matters because the cycle detector inside
    // `cross_kind_check::check` is what guarantees the surface
    // re-walk terminates). Today wired only on the Forge→Forge path —
    // see `nl_to_ir_mapping_roadmap.md` Item 2 + the module-level
    // scope comment for why Statechart→Forge stays out of v1.
    forge::cross_kind_check::check(parsed, base_dir, label.diagnostic_label)?;

    // NL→IR Mapping Roadmap Item 4 — physical-quantity unit-mismatch
    // arithmetic verification. Walks expression sites whose typed
    // operands could collide on `sce:quantity=…` annotations and
    // surfaces `validation/cross-kind-type-mismatch` (typed via
    // `ValidationError::QuantityUnitMismatch`, sharing the existing
    // DiagnosticCode slot per the user-confirmed reuse decision) on
    // the first mismatch. Runs after cross-kind check so the unit
    // walker can trust that imported alias references resolve.
    forge::quantity_check::check(parsed, label.diagnostic_label)?;

    // RFC §5.C B6-α' link-side cross-resolution. Runs after enrichment
    // populates `ImportContext::codec_max_bytes` (framer side) and
    // `ImportContext::buffer_pool_slot_size` (pool side); both axes
    // need to be present on the same `import_ctx` slice for the
    // comparison to fire. Cross-resolver is post-enrichment / pre-
    // codegen so a slot-size mismatch surfaces as `link/pool-slot-
    // smaller-than-framer-max` rather than as a silently-truncated
    // emit (or a silently-stage-copying TX path).
    validate_link_pool_framer_resolution(&parsed.document, &import_ctx, label.diagnostic_label)?;

    // RFC §5.D C2-β worker cross-resolution. Per-doc resolution of
    // `<sce:link-rx ref>` against kind=link imports and `<sce:outbox
    // ref>` against kind=statechart imports. Silent-skip on non-
    // Worker docs. Q-C2-3 (a)'s `ForgeWorkerRegistry` lock overturned
    // 2026-05-11 after Gate B preflight surfaced built-but-unconsumed
    // risk — direct parsed.imports check is the η-precedent textbook
    // path.
    validate_worker_cross_refs(&parsed.document, &parsed.imports, label.diagnostic_label)?;

    // RFC §5.I C2-β codegen-invariant for cross-core SPSC ordering.
    // Silent-skip when `options.worker_placement` is `None` (deploy-
    // unaware path); fires when the worker's declared `ordering=
    // "relaxed"` coexists with a placement entry pinning producer +
    // consumer on different cores.
    validate_worker_inbox_ordering_placement(
        &parsed.document,
        options.worker_placement.as_deref(),
        label.diagnostic_label,
    )?;

    // RFC §5.I Atomic C / Q-Call-7 — `<sce:extern>` rejected on non-MCU
    // backends (Kotlin/Go/Python). The wire-format diagnostic reuses
    // the existing `codegen/mcu-class-kind-on-non-mcu-language` family
    // per Q-Call-7 prose ("rejected via codegen/mcu-class-kind-on-non-
    // mcu-language family"); the `kind` field carries the literal
    // string `<sce:extern>` to disambiguate from an MCU-class-kind
    // rejection on the same code (kind-axis rejection puts a kind name
    // there). Atomics A/B build the `parsed.externs` slice;
    // this gate is the consumer that closes the built-but-unconsumed
    // path on non-MCU backends.
    if !parsed.externs.is_empty()
        && matches!(
            language,
            generator::Language::Kotlin | generator::Language::Go | generator::Language::Python
        )
    {
        return Err(Located::new(
            forge::error::GenerateError::CodegenMcuClassKindOnNonMcuLanguage {
                kind: "<sce:extern>".to_string(),
                language: language_wire_name(language).to_string(),
            }
            .into(),
            label.diagnostic_label,
            None,
            None,
        ));
    }

    let output = match language {
        generator::Language::Cpp => forge::generator::generate_cpp_with_imports_and_externs(
            &parsed.document,
            &template_base,
            &import_ctx,
            &parsed.externs,
            options,
        ),
        generator::Language::Kotlin => forge::generator::generate_kotlin_with_imports(
            &parsed.document,
            &template_base,
            &import_ctx,
            options,
        ),
        generator::Language::Rust => forge::generator::generate_rust_with_imports_and_externs(
            &parsed.document,
            &template_base,
            &import_ctx,
            &parsed.externs,
            options,
        ),
        generator::Language::Go => forge::generator::generate_go_with_imports(
            &parsed.document,
            &template_base,
            &import_ctx,
            options,
        ),
        generator::Language::Python => forge::generator::generate_python_with_imports(
            &parsed.document,
            &template_base,
            &import_ctx,
            options,
        ),
        generator::Language::C11 => forge::generator::generate_c11_with_imports_and_externs(
            &parsed.document,
            &template_base,
            &import_ctx,
            &parsed.externs,
            options,
        ),
    }
    .map_err(|e| Located::new(e, label.diagnostic_label, None, None))?;
    Ok(output)
}

/// Multi-doc compile entry point — watching-zenoh RFC §5.D C2 outbox
/// follow-up Atomic A (Q-Outbox-1 (a) lock 2026-05-12).
///
/// Walks every input doc (SCXML statechart + forge artifact files),
/// parses each, builds the build-wide [`forge::cross_doc_registry::
/// SceCrossDocRegistry`] (statechart + worker + link names) plus the
/// [`forge::pool_registry::ForgePoolRegistry`] (buffer-pool names),
/// then runs cross-document validators against the shared registries
/// before emitting code per-doc.
///
/// Distinguishing value vs the single-file entry points
/// ([`compile_scxml_lang_typed`], [`compile_forge_with_imports`]): the
/// orchestrator is the *only* production path that wires
/// [`parser::validate_on_sample_link_references`] into the build.
/// Before this entry point existed, the on-sample validator was
/// reachable only from tests — `<sce:on-sample link="undeclared">`
/// references silently passed every single-file build path (a
/// [`feedback_silently_broken_hooks`](../../.claude/projects/-home-coin-scxml-core-engine/memory/feedback_silently_broken_hooks.md)
/// instance closed by Atomic A's wire-up).
///
/// Output shape: `Vec<(filename_basename, GeneratedOutput)>`. The
/// basename includes the source file extension so callers know which
/// emit path produced each artifact (forge sidecars travel together
/// in their `GeneratedOutput::files` vector; statechart sidecars
/// similarly).
///
/// Empty file lists (both slices empty) are legal and return an empty
/// output vector with no error — `compile_scxml_with_imports(&[], &[],
/// …)` is the no-op case the orchestrator must not crash on.
pub fn compile_scxml_with_imports(
    scxml_files: &[&Path],
    forge_files: &[&Path],
    template_dir: &Path,
    language: generator::Language,
    options: &ForgeCompileOptions,
    deploy: Option<&mesh::deploy::DeployConfig>,
) -> Result<Vec<(String, generator::GeneratedOutput)>, CompileError> {
    use forge::cross_doc_registry::SceCrossDocRegistry;
    use forge::error::{Located, ValidationError};
    use forge::pool_registry::ForgePoolRegistry;

    // Pass 1: parse forge docs, populate cross-doc + pool registries.
    // Worker docs are also captured for the C2 follow-up Atomic B outbox
    // cross-resolution pass (`validate_worker_outbox_references`), which
    // cannot run until pass 2 finishes registering SCXML statechart
    // names (workers may route their outbox to statechart inboxes per
    // Q-Outbox-3 (b)).
    //
    // Bounded-collection docs + codec/procedure docs + aggregated
    // externs are captured for the C6-β cross-doc
    // resolution pass (`validate_bounded_collection_cross_refs`). The
    // `SceCrossDocRegistry` reserves SCXML-cross-reference semantics
    // for Link / Statechart / Worker kinds (those that SCXML
    // documents may reference via `<sce:on-sample>` /
    // `<sce:outbox>`), while codec + procedure participate only in
    // forge→forge cross-references as bounded-collection element
    // types — so the C6-β surface lives on a dedicated map per Gate B
    // user direction 2026-05-13.
    let mut cross_doc = SceCrossDocRegistry::new();
    let mut pool_reg = ForgePoolRegistry::new();
    let mut workers_for_outbox: Vec<(String, forge::model::WorkerModel)> = Vec::new();
    let mut bounded_collections_for_xref: Vec<(String, forge::model::BoundedCollectionModel)> =
        Vec::new();
    let mut element_type_candidates: std::collections::HashMap<
        String,
        forge::model::ForgeDocument,
    > = std::collections::HashMap::new();
    let mut all_externs: Vec<forge::model::ExternDeclaration> = Vec::new();
    // C13-α-1 + C13-α-2 cross-doc validators (`validate_links_cross_doc`,
    // `validate_links_burst_invariants`, `validate_reassembly_cross_doc`)
    // need the parsed forge LinkModel + BufferPoolModel by name. Capture
    // them during pass-1 alongside the worker/BC vectors so the deploy-
    // aware orchestrator pass can build &HashMap views without re-parsing.
    // Mirrors the `workers_for_outbox` + `bounded_collections_for_xref`
    // pattern; consumed by `validate_*_cross_doc` only when `deploy` is
    // `Some` — `None` (deploy-unaware path) silent-skips per Q-η5 (a)
    // precedent (no deploy ⇒ no cross-doc deploy-vs-forge to check).
    let mut link_models_for_xref: Vec<(String, forge::model::LinkModel)> = Vec::new();
    let mut pool_models_for_xref: Vec<(String, forge::model::BufferPoolModel)> = Vec::new();
    // NL→IR Item C1 Path A (Atomic 3) — EventSchemas keyed by file
    // stem (doc name). Populated in pass-1 alongside the other forge-
    // doc captures; consumed in pass-2 by
    // `event_schema_check::resolve_imported_event_schemas` which
    // projects per-statechart event-name views out of the build-wide
    // registry based on each statechart's own
    // `<sce:import kind="event-schema">` declarations.
    //
    // Per-statechart import visibility (DL-7' prerequisite) replaces
    // any legacy "one schema per event globally" rule: two machines
    // on a mesh may legitimately declare different schemas for the
    // same event name (e.g., during a rolling deploy with version
    // skew) — the cross-machine validator
    // (`mesh::deploy::validate_event_schemas_cross_machine`) is the
    // load-bearing rejection signal for any *actual* divergence.
    //
    // Doc-name uniqueness across the build is already enforced by
    // `cross_doc.record_document` above; the map's `insert` is safe
    // (collisions unreachable). Doc-name keying lets the per-
    // statechart resolver follow each import's `src` file stem to
    // the exact schema document the author named.
    let mut event_schemas_by_doc_name: std::collections::BTreeMap<
        String,
        forge::model::EventSchemaModel,
    > = std::collections::BTreeMap::new();
    // NL→IR Item C1 Path A (Atomic 5) — Enum kind documents keyed by
    // file stem (doc name), consumed by
    // `event_schema_check::resolve_imported_enums` to thread each
    // statechart's `<sce:import kind="enum">` set into the
    // receive/send-side literal-width narrowing layer
    // (`enum_underlying_overflow`). Doc-name uniqueness across the
    // build is already enforced by `cross_doc.record_document` above,
    // so the per-stem `insert` here is safe (collisions structurally
    // unreachable). Mirrors the `event_schemas_by_doc_name` capture
    // shape directly so the orchestrator's pass-2 wiring stays
    // symmetric across the two cross-kind import categories.
    let mut enums_by_doc_name: std::collections::BTreeMap<String, forge::model::EnumModel> =
        std::collections::BTreeMap::new();

    for forge_path in forge_files {
        let path_str = forge_path.to_str().unwrap_or("");
        let content = std::fs::read_to_string(forge_path).map_err(|e| {
            Located::new(
                forge::error::GenerateError::InvalidConfig(format!(
                    "compile_scxml_with_imports: cannot read {path_str}: {e}"
                ))
                .into(),
                path_str,
                None,
                None,
            )
        })?;
        let stem = forge_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("forge");
        let basename = forge_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(path_str);
        let label = DocumentLabel {
            identifier: stem,
            diagnostic_label: basename,
        };
        let parsed =
            forge::parser::parse_forge_with_imports(&content, label)?.ok_or_else(|| {
                Located::new(
                    ValidationError::WrongPipeline {
                        kind: forge::model::ForgeKind::Statechart,
                    }
                    .into(),
                    basename,
                    None,
                    None,
                )
            })?;
        let doc_name = parsed.document.name().to_string();
        cross_doc.record_document(&parsed.document).map_err(|existing| {
            Located::new(
                forge::error::GenerateError::InvalidConfig(format!(
                    "compile_scxml_with_imports: doc name '{doc_name}' already registered as kind '{existing_kind}'",
                    existing_kind = existing.as_str()
                ))
                .into(),
                basename,
                None,
                None,
            )
        })?;
        pool_reg.record_document(&parsed.document).map_err(|existing| {
            Located::new(
                forge::error::GenerateError::InvalidConfig(format!(
                    "compile_scxml_with_imports: pool registry rejected '{doc_name}' (existing kind '{existing:?}')"
                ))
                .into(),
                basename,
                None,
                None,
            )
        })?;
        // Aggregate every parsed doc's externs into the
        // build-wide slice consumed by the C6-β multi-writer atomic-
        // import check. The spec contract is "atomic imports must
        // exist somewhere in the build", so the union across all
        // forge docs is the relevant surface; per-doc isolation would
        // force authors to redeclare atomics in every BC doc, which
        // contradicts the §5.I trust-surface design.
        all_externs.extend(parsed.externs.iter().cloned());

        // Capture per-kind for downstream cross-doc validators. The
        // C2 follow-up Atomic B outbox path needs workers; the C6-β
        // path needs bounded-collections (subject docs) + codec /
        // procedure (element-type candidates). Other forge docs (link
        // / algorithm / buffer-pool / timer / transform / condition /
        // lookup / interpolation / filter / observer / validator)
        // silently skip — they have no role in either cross-doc
        // surface.
        match parsed.document {
            forge::model::ForgeDocument::Worker(worker) => {
                workers_for_outbox.push((basename.to_string(), worker));
            }
            forge::model::ForgeDocument::BoundedCollection(bc) => {
                bounded_collections_for_xref.push((basename.to_string(), bc));
            }
            forge::model::ForgeDocument::Link(link) => {
                // C13-α-1 + C13-α-2 cross-doc validators read this
                // back by link name to follow `<sce:rx-pool ref>` to
                // the bound BufferPoolModel. The diag-label
                // (basename) rides along so error sites name the
                // forge link doc that wrote the offending ref.
                link_models_for_xref.push((basename.to_string(), link));
            }
            forge::model::ForgeDocument::BufferPool(pool) => {
                // C13-α-2 reassembly + burst validators look up pool
                // slot_count / slot_size / variant via this capture.
                // `pool_reg` only stores the kind discriminator; full
                // BufferPoolModel field access requires the parallel
                // capture vector.
                pool_models_for_xref.push((basename.to_string(), pool));
            }
            doc @ (forge::model::ForgeDocument::Codec(_)
            | forge::model::ForgeDocument::Procedure(_)) => {
                // Element-type candidate map keyed by doc name; name
                // uniqueness across the build is already enforced by
                // `cross_doc.record_document` above (different kinds
                // sharing a name collide there). `insert` is safe —
                // duplicates are unreachable.
                let key = doc.name().to_string();
                element_type_candidates.insert(key, doc);
            }
            forge::model::ForgeDocument::EventSchema(schema) => {
                // NL→IR Item C1 Path A (Atomic 3, DL-7') — capture
                // EventSchemas by their doc name (file stem) so the
                // per-statechart resolver can follow each
                // `<sce:import src="X.scxml">` to the exact schema
                // document the author named. Doc-name uniqueness was
                // already enforced by `cross_doc.record_document`
                // above, so insertion is safe (collisions
                // structurally unreachable). Per-statechart import
                // visibility means two statecharts may legitimately
                // import different schemas for the same event name;
                // the cross-machine validator surfaces any *actual*
                // divergence as `mesh/event-schema-mismatch` only
                // when an affected cross-machine `<send>` walks.
                event_schemas_by_doc_name.insert(schema.name.clone(), schema);
            }
            forge::model::ForgeDocument::Enum(em) => {
                // NL→IR Item C1 Path A (Atomic 5, DL-5') — capture
                // Enum docs by their doc name (file stem) so the
                // per-statechart resolver can follow each
                // `<sce:import kind="enum">` to the EnumModel and
                // narrow integer-literal comparisons / send-params
                // against the declared `underlying_type`. Same
                // doc-name uniqueness invariant as the EventSchema
                // capture immediately above.
                enums_by_doc_name.insert(em.name.clone(), em);
            }
            _ => {}
        }
    }

    // Pass 2: parse SCXML docs, register statechart names, run cross-ref
    // validators against the shared registries. The on-sample wire-up
    // here is the production-side closure for the pre-Atomic-A silently
    // broken hook (`feedback_silently_broken_hooks.md`).
    let mut scxml_models: Vec<(std::path::PathBuf, SCXMLModel)> = Vec::new();
    for scxml_path in scxml_files {
        let path_str = scxml_path.to_str().unwrap_or("");
        // Pass-2 only consumes the model for cross-doc validators
        // (`validate_on_sample_link_references`, `resolve_listener_links`,
        // per-instance event-queue checks). Preprocessor deps are
        // re-collected in pass-3 by `compile_scxml_lang_typed_with_section`
        // and attached to each per-doc `GeneratedOutput.deps` — so
        // dropping them here is intentional, not a leak.
        let ParsedSCXML { model, .. } = compile_model(path_str)?;
        let basename = scxml_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(path_str);
        if !model.name.is_empty() {
            // SCXML without a `name` attribute is legal at the parser
            // tier; skip registration so outbox refs of the form
            // `<owner>.inbox` resolve only against named docs. The
            // skipped doc still proceeds through cross-ref validation
            // (it cannot be a recipient, but it can be a sender).
            cross_doc
                .record_statechart(model.name.clone())
                .map_err(|existing| {
                    Located::new(
                        forge::error::GenerateError::InvalidConfig(format!(
                            "compile_scxml_with_imports: statechart '{name}' collides with previously-registered kind '{existing_kind}'",
                            name = model.name,
                            existing_kind = existing.as_str()
                        ))
                        .into(),
                        basename,
                        None,
                        None,
                    )
                })?;
        }
        scxml_models.push(((*scxml_path).to_path_buf(), model));
    }
    for (path, model) in &scxml_models {
        let basename = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
        parser::validate_on_sample_link_references(model, &cross_doc, &pool_reg, basename)?;
    }

    // ── NL→IR Item C1 Path A (Atomic 3) EventSchema receive- + send-side typecheck ──
    //
    // Receive-side (DL-5'): walks every parsed statechart's
    // transition `cond` expressions for `_event.data.<field>`
    // member-access patterns and verifies the field against the
    // schema declared for that transition's event.
    //
    // Send-side (DL-4'): walks every `<send event="X">` /
    // `<raise event="X">` (inside transition `actions`, `<onentry>`,
    // `<onexit>`, initial-transition + history-default sequences,
    // and nested `<if>` / `<foreach>` bodies) and verifies each
    // `<param name="F" expr="...">` against the schema's declared
    // field surface.
    //
    // Both passes run after pass-1 capture
    // (`event_schemas_by_doc_name` populated) and after pass-2 SCXML
    // parsing (`scxml_models` populated). The per-statechart
    // resolver walks each SCXML's `<sce:import>` declarations to
    // determine which schemas are in-scope for THIS document — so a
    // statechart that does not declare any event-schema imports keeps
    // the dynamic `_event.data` baseline even when other statecharts
    // in the same build declare schemas. Failure short-circuits
    // codegen pass-3, matching the worker outbox + BC cross-doc
    // pattern.
    for (path, model) in &scxml_models {
        let basename = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
        let per_doc_schemas = forge::event_schema_check::resolve_imported_event_schemas(
            model,
            &event_schemas_by_doc_name,
        );
        // NL→IR Item C1 Path A (Atomic 5, DL-5') — per-statechart enum
        // imports drive the literal-width narrowing layer inside the
        // receive- + send-side validators. A statechart whose schema
        // declares an enum-typed field MUST also declare its own
        // `<sce:import kind="enum" as="<alias>">` for the narrowing to
        // resolve the alias against an `EnumModel`; otherwise the
        // alias is opaque from the statechart's view and the
        // narrowing silent-skips (Atomic 3's conservative-accept
        // default preserves the category-only behavior).
        let per_doc_enums =
            forge::event_schema_check::resolve_imported_enums(model, &enums_by_doc_name);
        forge::event_schema_check::check(model, &per_doc_schemas, &per_doc_enums, basename)?;
        forge::event_schema_check::check_send_side(
            model,
            &per_doc_schemas,
            &per_doc_enums,
            basename,
        )?;
        // §scxml-G-7 — `<sce:action>` Custom Action Element: validate
        // that every native host-dispatch action is a direct <transition>
        // child whose `<sce:arg>`s are typed `_event.data.<field>` references
        // resolving against the triggering event's EventSchema. Engine-free
        // by definition, so a non-conforming construct is rejected here
        // rather than degraded to a runtime script engine.
        forge::native_action::validate(model, &per_doc_schemas, basename)?;
    }

    // ── C2 follow-up Atomic B outbox cross-resolution ──
    //
    // Runs after pass-2 statechart registration so worker→statechart
    // outbox refs resolve symmetrically with worker→worker refs.
    // Captured WorkerModels from pass 1 carry the diag_label
    // (basename) so the diagnostic anchor points at the worker doc
    // that wrote the offending `<sce:outbox>`.
    validate_worker_outbox_references(&workers_for_outbox, &cross_doc)?;

    // ── C6 Atomic β bounded-collection cross-doc resolution ──
    //
    // Runs after pass-1 captures all parsed forge docs (so
    // `element_type_candidates` + `all_externs` are
    // populated) and after worker outbox so cross-doc validators run
    // in spec-section order (§5.D outbox before §5.L bounded-
    // collection). Independent of SCXML statechart registration — the
    // C6-β surface is forge→forge entirely (codec/procedure element
    // types + atomic-purpose `<sce:extern>` declarations), so it
    // does not depend on pass-2's statechart-name population. Failing
    // here short-circuits codegen pass-3, matching the worker outbox
    // pattern.
    validate_bounded_collection_cross_refs(
        &bounded_collections_for_xref,
        &element_type_candidates,
        &all_externs,
    )?;

    // ── C13-α-1 + C13-α-2 deploy-aware cross-doc validators ──
    //
    // Closes the deferred-orchestrator-wiring debt named in
    // `c13_alpha_1_landed.md` + `c13_alpha_2_landed.md`. When `deploy`
    // is `Some`, three validators fire in spec-section walk order:
    //   1. `validate_links_cross_doc` (§5.K Q-C13-5 a) — every forge
    //      `<sce:link name=X>` must have a `deploy.machines.<n>.links.X`
    //      counterpart, and vice versa.
    //   2. `validate_links_burst_invariants` (§5.K lines 2489-2500) —
    //      RX pool drain capacity vs declared `burst_pps` per the
    //      cooperative tick window.
    //   3. `validate_reassembly_cross_doc` (§5.M lines 2946-2995) —
    //      slot_size vs mtu, reassembly fragment count, trust class,
    //      stage-copy WCET.
    //
    // All three silent-skip on `None` deploy per Q-η5 (a) precedent
    // (no deploy ⇒ no deploy-vs-forge axis to check). When deploy
    // is `Some`, the validators consume `&HashMap` views over the
    // pass-1 capture vectors — single source of truth for the
    // 3-way join `deploy.links.<X>` → forge `<sce:link>` →
    // `<sce:rx-pool ref>` → `BufferPoolModel`. Failure short-circuits
    // codegen pass-3, matching the worker outbox + BC cross-doc
    // pattern. Errors route through `ForgeError::Mesh(MeshError)` so
    // the wire payload preserves all rich DeployError diagnostic
    // fields (machine + link_name + pool_name + slot_count +
    // burst_pps + tick_period_us + arrivals_per_tick / drain_per_second);
    // forge-side reassembly validators emit `ValidationError`
    // directly per the existing `#[from]` flow.
    // ── Axis-3 cross-doc role validation ──
    //
    // Runs BEFORE listener-link resolution so partial-claim failures
    // surface as typed `link/...` or `scxml/...` diagnostics rather
    // than silently dropping into the listener-set union. Three typed
    // codes from RFC Q-A7 fire here:
    //   - link/deploy-role-listener-without-scxml-accept-side-role
    //   - scxml/accept-side-role-without-listener-link
    //   - link/role-listener-with-non-session-arming-trust-class
    // Legacy fixtures (no explicit role / session-role declarations)
    // silent-pass per Q-A9 staged migration discipline; the
    // promotion to required-on-every-listener waits until every
    // fixture declares the explicit role pair.
    if let Some(deploy_cfg) = deploy {
        validate_cross_doc_listener_roles(deploy_cfg, &scxml_models).map_err(|e| {
            Located::new(
                forge::error::ForgeError::Validation(e),
                "deploy.yaml",
                None,
                None,
            )
        })?;
    }

    // ── C10-α listener-pair resolution + Axis-3 explicit-role
    //    join ──
    //
    // Computed unconditionally so deploy-aware downstream consumers
    // (the C13 cross-doc validators + the per-doc compile_forge_with_imports
    // codegen pass) see a single source of truth. Defaults to an
    // empty set on `deploy: None` paths — silent-skip per Q-η5 (a):
    // no deploy ⇒ no machine.source × session_arming axis to scan;
    // listener-pair synthesis cannot fire. The join is the
    // explicit-role pair only — the legacy substate-driven walker
    // was deleted from this path; `accepting_substate_present`
    // survives solely as the parser migration-helper's data source.
    let listener_links: std::collections::BTreeSet<String> = match deploy {
        Some(deploy_cfg) => resolve_listener_links(deploy_cfg, &scxml_models),
        None => std::collections::BTreeSet::new(),
    };

    if let Some(deploy_cfg) = deploy {
        let forge_link_models_view: std::collections::HashMap<String, &forge::model::LinkModel> =
            link_models_for_xref
                .iter()
                .map(|(_, link)| (link.name.clone(), link))
                .collect();
        let pool_models_view: std::collections::HashMap<String, &forge::model::BufferPoolModel> =
            pool_models_for_xref
                .iter()
                .map(|(_, pool)| (pool.name.clone(), pool))
                .collect();
        let forge_link_names: Vec<String> = forge_link_models_view
            .keys()
            .cloned()
            .collect::<std::collections::BTreeSet<String>>()
            .into_iter()
            .collect();

        // The diag_label for cross-doc failures points at deploy.yaml —
        // the cross-doc axis names a deploy-side fact, not a per-forge-
        // doc parser error. CLI surfaces the file via the wire `path`
        // field; `Located::new` carries the label.
        const DEPLOY_LABEL: &str = "deploy.yaml";

        mesh::deploy::validate_links_cross_doc(deploy_cfg, &forge_link_names)
            .map_err(mesh::error::MeshError::from)
            .map_err(|e| Located::new(e.into(), DEPLOY_LABEL, None, None))?;

        mesh::deploy::validate_links_burst_invariants(
            deploy_cfg,
            &forge_link_models_view,
            &pool_models_view,
        )
        .map_err(mesh::error::MeshError::from)
        .map_err(|e| Located::new(e.into(), DEPLOY_LABEL, None, None))?;

        mesh::deploy::validate_link_driver_class_consistency(deploy_cfg, &forge_link_models_view)
            .map_err(mesh::error::MeshError::from)
            .map_err(|e| Located::new(e.into(), DEPLOY_LABEL, None, None))?;

        mesh::deploy::validate_reassembly_cross_doc(
            deploy_cfg,
            &forge_link_models_view,
            &pool_models_view,
            &listener_links,
        )
        .map_err(|e| Located::new(e.into(), DEPLOY_LABEL, None, None))?;

        // C13 deferred-2: stateless_accept hmac_extern / rng_extern
        // allowlist (watching-zenoh RFC §5.K line 2466-2469). The
        // sorted union of §5.I baseline + target_plugin-loaded symbols
        // is the closed candidate set. The orchestrator path loads
        // plugin_symbols here (mirroring `compile_forge_with_deploy`'s
        // single-doc loader) so the validator sees the same registry
        // composition the parser would see.
        // load_target_plugin_for_compile returns Located<ForgeError>
        // (the same type as the orchestrator's CompileError alias) —
        // bubble through with ?. Any plugin-load failure surfaces at
        // the deploy.yaml label set above.
        let orchestrator_plugin_symbols = load_target_plugin_for_compile(deploy_cfg, DEPLOY_LABEL)?;
        mesh::deploy::validate_stateless_accept_externs(deploy_cfg, &orchestrator_plugin_symbols)
            .map_err(mesh::error::MeshError::from)
            .map_err(|e| Located::new(e.into(), DEPLOY_LABEL, None, None))?;

        // ── NL→IR Item C1 Path A (Atomic 3, DL-7') cross-machine
        //    EventSchema validation ─────────────────────────────────
        //
        // Per-machine schema visibility (derived from each statechart's
        // `<sce:import kind="event-schema">` declarations) is compared
        // across every cross-machine `<send target="#X">` so divergent
        // schemas surface as `mesh/event-schema-mismatch` instead of
        // silently producing two incompatible wire contracts. Runs in
        // the deploy-aware path because the rejection depends on the
        // mesh topology — single-statechart compilations (no deploy)
        // cannot exhibit a cross-machine mismatch by construction.
        mesh::deploy::validate_event_schemas_cross_machine(
            deploy_cfg,
            &scxml_models,
            &event_schemas_by_doc_name,
        )
        .map_err(mesh::error::MeshError::from)
        .map_err(|e| Located::new(e.into(), DEPLOY_LABEL, None, None))?;

        // ── C10-β link/inbound-event-queue-unsized ──
        //
        // Watching-zenoh RFC §5.N line 3062 verbatim — for every link
        // carrying `<sce:inbound>` events, the build must observe an
        // event-queue capacity binding from one of two sources per
        // Q-C10-β-4 (a): SCXML per-instance `sce:capacity="N"` on the
        // machine's source SCXML doc (preferred), or deploy
        // `scheduler.default_event_queue_capacity` (fallback). The
        // validator walks the deploy/forge link union pair to enumerate
        // (machine, link_name, inbound_count) tuples and fires the
        // diagnostic when both size sources are absent.
        //
        // Sites the validator joins:
        //   1. `forge_link_models_view` (built above) → inbound count
        //      per link name.
        //   2. `deploy.machines.<m>.links` → which machine owns the
        //      link.
        //   3. `scxml_models` (already parsed in pass 2 above) → the
        //      machine's source SCXML's `event_queue_capacity`.
        //   4. `deploy.machines.<m>.scheduler.default_event_queue_capacity`
        //      → per-machine fallback.
        //
        // Silent-skip per Q-C10-β-9 (a) when the link has no inbound
        // events declared OR when no SCXML imports the link (no FSM
        // downstream to size).
        for device in deploy_cfg.topology.values() {
            for (machine_name, machine) in device.machines.iter() {
                for (link_name, _link_cfg) in machine.links.iter() {
                    let Some(forge_link) = forge_link_models_view.get(link_name) else {
                        continue;
                    };
                    let inbound_count = forge_link.inbound.len() as u32;
                    if inbound_count == 0 {
                        continue;
                    }

                    // Per-instance source: machine.source SCXML's
                    // event_queue_capacity wins. Match scxml model by
                    // basename = machine.source.
                    let machine_source = machine.source.as_str();
                    let model = scxml_models.iter().find_map(|(path, model)| {
                        let matches = path
                            .file_name()
                            .and_then(|s| s.to_str())
                            .is_some_and(|n| n == machine_source)
                            || path.to_str().is_some_and(|p| p == machine_source);
                        if matches {
                            Some(model)
                        } else {
                            None
                        }
                    });
                    let has_per_instance = model.and_then(|m| m.event_queue_capacity).is_some();
                    // Per-machine fallback source.
                    let has_per_machine = machine
                        .scheduler
                        .as_ref()
                        .and_then(|s| s.default_event_queue_capacity)
                        .is_some();

                    if !has_per_instance && !has_per_machine {
                        return Err(Located::new(
                            ValidationError::LinkInboundEventQueueUnsized {
                                machine: machine_name.clone(),
                                link_name: link_name.clone(),
                                inbound_event_count: inbound_count,
                            }
                            .into(),
                            DEPLOY_LABEL,
                            None,
                            None,
                        ));
                    }
                }
            }
        }
    }

    // C6-γ2: bounded-collection codegen resolutions for the orchestrator
    // path. CompileConst BCs copy the literal capacity through; DeployKey
    // BCs are skipped here (no deploy access on this entry point — those
    // route through [`compile_forge_with_deploy`]). For BCs with
    // `<sce:index-by>`, extract the Rust type-string of the named field
    // from the resolved element-type ForgeDocument that
    // `validate_bounded_collection_cross_refs` just confirmed. The render
    // layer reads `bounded_collection_resolutions[bc.name]` and surfaces
    // a clear `InvalidConfig` if a needed key is absent.
    let bc_resolutions: std::collections::HashMap<String, BoundedCollectionResolution> =
        bounded_collections_for_xref
            .iter()
            .filter_map(|(_label, bc)| {
                let capacity = match &bc.capacity {
                    forge::model::CapacitySource::CompileConst { value } => *value,
                    // Single-orchestrator path has no deploy; DeployKey
                    // BCs surface their missing resolution at render
                    // time via `InvalidConfig` — keeps this populator
                    // free of guesswork and aligns with the cache_platform
                    // precedent ("populator skips when source is missing").
                    forge::model::CapacitySource::DeployKey { .. } => return None,
                };
                let index_by_field_sce_type = bc.index_by.as_ref().and_then(|field| {
                    let element_doc = element_type_candidates.get(&bc.element_type)?;
                    extract_bounded_collection_index_field_sce_type(element_doc, field)
                });
                Some((
                    bc.name.clone(),
                    BoundedCollectionResolution {
                        capacity,
                        index_by_field_sce_type,
                    },
                ))
            })
            .collect();
    // RFC c7-wildcard W-project: element-type field schemas keyed by the
    // element-type snake name, resolved from the same candidate map. An
    // algorithm iterating a BC types each `entry.<field>` from this so the
    // bounded-string-field → bytes-view projection (Q-W-5) can fire and so
    // a mistyped argument (`entry.callback_id` into a `bytes` param) is
    // caught rather than silently miscompiled. Independent of the
    // `<sce:index-by>` / capacity resolution above (that map is keyed by
    // BC name; this is keyed by element name to match what the BC import
    // context carries — see `ForgeCompileOptions::element_type_field_schemas`).
    let element_type_field_schemas: std::collections::HashMap<String, ElementFieldSchema> =
        bounded_collections_for_xref
            .iter()
            .filter_map(|(_label, bc)| {
                let element_doc = element_type_candidates.get(&bc.element_type)?;
                Some((
                    filters::to_snake_case(bc.element_type.clone()),
                    extract_bounded_collection_element_field_sce_types(element_doc),
                ))
            })
            .collect();
    // C10-α: thread the orchestrator-resolved listener-link set into
    // the per-doc ForgeCompileOptions so each `render_link_*` template
    // can synthesize the Sibling half + the post-render self-check
    // fires on the right links. The deploy-aware path always carries
    // an explicit-empty `Some(empty)` so downstream consumers can
    // distinguish "no listeners declared" from "deploy-unaware
    // compile" (the latter must not synthesize siblings — silent-skip
    // per Q-η5 (a)).
    let listener_links_override: Option<std::collections::BTreeSet<String>> =
        deploy.map(|_| listener_links.clone());
    let needs_override = !bc_resolutions.is_empty()
        || listener_links_override.is_some()
        || !element_type_field_schemas.is_empty();
    let bc_options_override = if !needs_override {
        None
    } else {
        let mut overridden = options.clone();
        if !bc_resolutions.is_empty() {
            overridden.bounded_collection_resolutions = Some(bc_resolutions);
        }
        if let Some(ll) = listener_links_override {
            overridden.listener_links = Some(ll);
        }
        if !element_type_field_schemas.is_empty() {
            overridden.element_type_field_schemas = Some(element_type_field_schemas);
        }
        Some(overridden)
    };

    // Pass 3: codegen. Forge docs route through `compile_forge_with_imports`
    // (which re-parses + runs forge-internal cross-resolution + emits);
    // SCXML docs route through `compile_scxml_lang_typed` (which re-parses
    // + emits). The orchestrator's unique contribution is the registry +
    // validator pass above — codegen itself remains the existing pipeline
    // so single-file callers and the orchestrator share emit paths.
    let mut outputs: Vec<(String, generator::GeneratedOutput)> = Vec::new();

    for forge_path in forge_files {
        let path_str = forge_path.to_str().unwrap_or("");
        let content = std::fs::read_to_string(forge_path).map_err(|e| {
            Located::new(
                forge::error::GenerateError::InvalidConfig(format!(
                    "compile_scxml_with_imports: cannot read {path_str}: {e}"
                ))
                .into(),
                path_str,
                None,
                None,
            )
        })?;
        let stem = forge_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("forge");
        let basename = forge_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(path_str);
        let label = DocumentLabel {
            identifier: stem,
            diagnostic_label: basename,
        };
        let base_dir = forge_path.parent().unwrap_or_else(|| Path::new("."));
        let effective_options = bc_options_override.as_ref().unwrap_or(options);
        let out =
            compile_forge_with_imports(&content, label, language, base_dir, effective_options)?;
        outputs.push((basename.to_string(), out));
    }

    // Watching-zenoh RFC §5.2 Round F-α / F-α-2 — codegen-entry checks
    // that depend on `deploy.yaml`. (i) Non-MCU backend reject of
    // `platform.c11_section_attribute` (Q-Round-F-D3) fires before any
    // SCXML codegen because the section attribute itself has no axis
    // outside C11; the early exit keeps templates from emitting
    // half-applied directives. (ii) `platform.driver_root` override
    // (Q-Round-F-D5) is resolved per-machine and threaded into
    // [`compile_scxml_lang_typed_with_section`] so each statechart
    // resolves `<sce:driver>` against the deploy-specified root.
    // (iii) F-α-2 `c11_section_attribute.class` is captured here and
    // routed through the same entry so the C11 backend's `SCE_SM_FN`
    // macro expands to `__attribute__((section("<class>")))` and every
    // statechart function definition carries the prefix. The first
    // machine carrying either override wins per-orchestrator-run; the
    // single-machine common case (one `deploy.yaml`, one `c11_*` backend
    // target) is the only shape Q-Round-F-D5 commits.
    let mut deploy_driver_root: Option<std::path::PathBuf> = None;
    let mut deploy_section_class: Option<String> = None;
    if let Some(deploy_cfg) = deploy {
        for device in deploy_cfg.topology.values() {
            for machine in device.machines.values() {
                if let Some(platform) = machine.platform.as_ref() {
                    if let Some(section) = platform.c11_section_attribute.as_ref() {
                        forge::codegen_matrix::check_c11_section_attribute(true, language)
                            .map_err(|e| Located::new(e.into(), "deploy.yaml", None, None))?;
                        if deploy_section_class.is_none() {
                            deploy_section_class.clone_from(&section.class);
                        }
                    }
                    if let Some(root) = platform.driver_root.as_deref() {
                        if deploy_driver_root.is_none() {
                            deploy_driver_root = Some(std::path::PathBuf::from(root));
                        }
                    }
                }
            }
        }
    }

    for scxml_path in scxml_files {
        let path_str = scxml_path.to_str().unwrap_or("");
        let basename = scxml_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(path_str);
        let out = compile_scxml_lang_typed_with_section(
            path_str,
            template_dir,
            language,
            deploy_driver_root.as_deref(),
            deploy_section_class.as_deref(),
        )?;
        outputs.push((basename.to_string(), out));
    }

    // ── C10-β per-machine concurrency artifacts ──
    //
    // Watching-zenoh RFC §5.N lines 3041-3055 (Q-C10-β-5/-6 a). Iterate
    // `deploy.machines` and emit per-machine AP `LinkBus` + MCU
    // round-robin scheduler artifacts alongside the per-doc outputs
    // above. The emit fires only on Rust + C11 per Q-C10-β-7 (a) and
    // silent-skips on the deploy-unaware path (single-file CLI) per
    // Q-C10-β-9 (a).
    //
    // Uses [`find_template_base`] rather than the per-language
    // `template_dir` parameter — the latter is the SCXML-side root
    // (`<base>/<lang>` for some backends), while the C10-β per-machine
    // emitters resolve `forge/rust` and `forge/c` directly under the
    // shared template root (matching the existing
    // `compile_forge_with_imports` helper-discovery pattern at lib.rs:1338).
    if let Some(deploy_cfg) = deploy {
        let template_base = find_template_base();
        for device in deploy_cfg.topology.values() {
            for (machine_name, machine) in device.machines.iter() {
                let link_names: Vec<String> = machine
                    .links
                    .keys()
                    .cloned()
                    .collect::<std::collections::BTreeSet<String>>()
                    .into_iter()
                    .collect();
                let tick_period_us = machine.scheduler.as_ref().and_then(|s| s.tick_period_us);
                let per_link_budget_us = machine
                    .scheduler
                    .as_ref()
                    .and_then(|s| s.per_link_budget_us);
                let files = forge::generator::render_machine_concurrency_artifacts(
                    &template_base,
                    language,
                    machine_name,
                    &link_names,
                    tick_period_us,
                    per_link_budget_us,
                )
                .map_err(|e| Located::new(e, "deploy.yaml", None, None))?;
                if !files.is_empty() {
                    // Per-machine C10-β scheduler artifacts are
                    // synthesised from `deploy.yaml`, not the SCXML
                    // preprocessor pipeline — they have no
                    // filesystem-anchored fragment deps to forward.
                    outputs.push((
                        machine_name.clone(),
                        generator::GeneratedOutput {
                            files,
                            ..Default::default()
                        },
                    ));
                }
            }
        }
    }

    Ok(outputs)
}

/// Wire-format name for a [`generator::Language`] — mirrors
/// [`forge::codegen_matrix::language_wire_name`] but accessible from
/// `lib.rs` without crossing the matrix module boundary. Kept private;
/// callers go through this function so the lowercase lock matches the
/// `codegen/mcu-class-kind-on-non-mcu-language` `actual` field's
/// existing case convention.
fn language_wire_name(lang: generator::Language) -> &'static str {
    match lang {
        generator::Language::Cpp => "cpp",
        generator::Language::Kotlin => "kotlin",
        generator::Language::Rust => "rust",
        generator::Language::Go => "go",
        generator::Language::Python => "python",
        generator::Language::C11 => "c11",
    }
}

/// Recursively resolve a codec's full encode-buffer max bytes — RFC §5.B
/// B5-ε. Mirrors the parent generator's `m.max_frame_bytes() + body_max`
/// formula (see `forge::generator` `render_codec` max-bytes computation),
/// but applies it transitively so a parent codec that imports a
/// variant-bearing leaf gets a correctly-sized encode buffer.
///
/// Without this recursion, `validate_and_enrich_imports` populated
/// `ImportContext::codec_max_bytes` from `cm.max_frame_bytes()` alone
/// — the model-level value omits variant arm body / repeat body /
/// tlv-chain body sizing because at parse time the imported codecs
/// aren't enriched yet. For non-variant leaves that's correct (no
/// arm body to add); for a variant-bearing import (B5-ε's first
/// reachable consumer: `codec_zenoh_ext_envelope` carrying a TLV
/// chain of variant-bodied `codec_zenoh_ext_entry` entries) the
/// parent's chain MAX_BYTES would silently truncate ZBuf-encoded
/// entries on C11.
///
/// Cycle detection: tracks visited canonical paths via `visited`. A
/// cycle (A imports B imports A) returns the model-level max for the
/// cycle leaf — same shape as the pre-fix behaviour for that one
/// frame, while every non-cyclic frame gets the correct recursive
/// sum.
fn compute_codec_recursive_max_bytes(
    cm: &forge::model::CodecModel,
    imports: &[forge::model::ForgeImport],
    base_dir: &Path,
    visited: &mut HashSet<PathBuf>,
) -> u32 {
    use forge::model::BitSize;

    // Resolve a single import alias to the imported codec's full
    // recursive max-bytes. Returns `None` on filesystem / parse / kind
    // mismatch / cycle — caller (the variant / repeat / tlv branches
    // below) skips that contribution, falling back to a 0 increment.
    // Skipping is safe because the parent generator's eventual
    // max-bytes pass would also skip on the same conditions.
    fn resolve_import_max(
        alias: &str,
        imports: &[forge::model::ForgeImport],
        base_dir: &Path,
        visited: &mut HashSet<PathBuf>,
    ) -> Option<u32> {
        let imp = imports.iter().find(|i| i.alias == alias)?;
        let imp_path = base_dir.join(&imp.src);
        // Use joined path (not canonical) for cycle detection: matches
        // the way the rest of the enrichment resolves paths and avoids
        // a hard dependency on the file existing at canonicalize time.
        let visit_key = imp_path.clone();
        if !visited.insert(visit_key.clone()) {
            return None;
        }
        let result = (|| -> Option<u32> {
            let content = std::fs::read_to_string(&imp_path).ok()?;
            let stem = imp_path.file_stem()?.to_str()?;
            let basename = imp_path.file_name()?.to_str()?;
            let label = DocumentLabel {
                identifier: stem,
                diagnostic_label: basename,
            };
            let parsed = forge::parser::parse_forge_with_imports(&content, label).ok()??;
            let inner_cm = match parsed.document {
                forge::model::ForgeDocument::Codec(c) => c,
                _ => return None,
            };
            let inner_base = imp_path.parent()?.to_path_buf();
            Some(compute_codec_recursive_max_bytes(
                &inner_cm,
                &parsed.imports,
                &inner_base,
                visited,
            ))
        })();
        visited.remove(&visit_key);
        result
    }

    // Mirrors generator.rs render_codec's mutually-exclusive branches:
    // variant codecs add the worst-case arm body; non-variant codecs
    // add the per-field repeat / tlv-chain body sums. A codec with
    // both is rare in practice and the parent generator itself does
    // not combine them today — staying byte-identical to that formula
    // keeps the helper's output usable as a drop-in for the parent
    // pass on any leaf (sanity invariant: imports get the same value
    // the parent would compute if it rendered the codec directly).
    if let Some(v) = &cm.variant {
        let arm_body_max = v
            .arms
            .iter()
            .chain(v.default_arm.iter())
            .filter_map(|arm| resolve_import_max(&arm.body_alias, imports, base_dir, visited))
            .max()
            .unwrap_or(0);
        cm.max_frame_bytes() + arm_body_max
    } else {
        let repeat_body_max: u32 = cm
            .fields
            .iter()
            .filter(|f| f.is_repeat())
            .filter_map(|f| {
                let alias = f.repeat_body_alias.as_deref()?;
                let body_max = resolve_import_max(alias, imports, base_dir, visited)?;
                let count = forge::limits::resolve_max_count(f.max_count);
                Some(body_max.saturating_mul(count))
            })
            .sum();
        let tlv_chain_body_max: u32 = cm
            .fields
            .iter()
            .filter(|f| f.is_tlv_chain())
            .filter_map(|f| {
                let alias = f.tlv_chain_body_alias.as_deref()?;
                let body_max = resolve_import_max(alias, imports, base_dir, visited)?;
                let max_depth = match &f.bit_size {
                    BitSize::TlvChain { max_depth, .. } => *max_depth,
                    _ => return None,
                };
                Some(body_max.saturating_mul(max_depth))
            })
            .sum();
        cm.max_frame_bytes() + repeat_body_max + tlv_chain_body_max
    }
}

/// Whether a codec's generated Rust struct holds any borrowed (`&'a`)
/// reference and therefore needs a `<'a>` lifetime parameter.
///
/// A codec is borrowed iff it has a scalar `Bytes` / `String` field
/// (decoded as a zero-copy `&'a [u8]` / `&'a str` view per the
/// `SceCursor::peek_slice -> &'a [u8]` borrow contract) OR it embeds /
/// repeats / tlv-chains / variant-dispatches a body codec that is
/// itself transitively borrowed (the body's lifetime infects the
/// parent through `Vec<Body<'a>>` / `Body<'a>` field types).
///
/// Mirrors [`compute_codec_recursive_max_bytes`] in structure: resolves
/// each referenced import alias to its parsed codec model and recurses,
/// with the same `visited`-set cycle guard. A cycle leaf contributes
/// `false` (its own scalar fields are still inspected directly by the
/// frame that opened the cycle). Borrowed-ness is exact, never an
/// over-approximation: an unused lifetime parameter is a hard compile
/// error (E0392), so the inference must not claim `<'a>` for a codec
/// that holds no borrow.
fn codec_is_borrowed_recursive(
    cm: &forge::model::CodecModel,
    imports: &[forge::model::ForgeImport],
    base_dir: &Path,
    visited: &mut HashSet<PathBuf>,
) -> bool {
    // Resolve a single import alias to its body codec's transitive
    // borrowed-ness. `None` (missing / parse error / kind mismatch /
    // cycle) contributes `false` — matches the conservative skip in
    // `compute_codec_recursive_max_bytes::resolve_import_max`.
    fn resolve_import_borrowed(
        alias: &str,
        imports: &[forge::model::ForgeImport],
        base_dir: &Path,
        visited: &mut HashSet<PathBuf>,
    ) -> Option<bool> {
        let imp = imports.iter().find(|i| i.alias == alias)?;
        let imp_path = base_dir.join(&imp.src);
        let visit_key = imp_path.clone();
        if !visited.insert(visit_key.clone()) {
            return None;
        }
        let result = (|| -> Option<bool> {
            let content = std::fs::read_to_string(&imp_path).ok()?;
            let stem = imp_path.file_stem()?.to_str()?;
            let basename = imp_path.file_name()?.to_str()?;
            let label = DocumentLabel {
                identifier: stem,
                diagnostic_label: basename,
            };
            let parsed = forge::parser::parse_forge_with_imports(&content, label).ok()??;
            let inner_cm = match parsed.document {
                forge::model::ForgeDocument::Codec(c) => c,
                _ => return None,
            };
            let inner_base = imp_path.parent()?.to_path_buf();
            Some(codec_is_borrowed_recursive(
                &inner_cm,
                &parsed.imports,
                &inner_base,
                visited,
            ))
        })();
        visited.remove(&visit_key);
        result
    }

    // SSOT predicate (`CodecModel::is_borrowed_with`) over the file-graph
    // resolver: each embed/repeat/tlv/variant body alias is resolved by
    // re-parsing the import and recursing. The scalar-field rule and the
    // body traversal live on the model so codegen-time
    // (`generator.rs::codec_self_borrowed`) and this enrichment-time walk
    // can never diverge.
    cm.is_borrowed_with(|alias| {
        resolve_import_borrowed(alias, imports, base_dir, visited).unwrap_or(false)
    })
}

/// Owned→borrowed projection round: transitive `as_borrowed`-fallibility
/// of a codec — whether its `{Codec}Owned::as_borrowed` must return a
/// `Result` (`try_as_borrowed`). Enrichment-time twin of
/// `codec_is_borrowed_recursive`, sharing `CodecModel::is_as_borrowed_with`'s
/// structural predicate over the file-graph resolver so codegen-time
/// (`generator.rs::codec_self_as_borrowed_fallible`) and this walk cannot
/// diverge.
fn codec_as_borrowed_fallible_recursive(
    cm: &forge::model::CodecModel,
    imports: &[forge::model::ForgeImport],
    base_dir: &Path,
    visited: &mut HashSet<PathBuf>,
) -> bool {
    // A single import alias's *contribution* to the parent's projection
    // fallibility. A non-borrowed body is deep-cloned wholesale by the
    // projection (never fails), so it contributes only when it is itself
    // borrowed AND its own projection is fallible. `None` (missing / parse
    // error / kind mismatch / cycle) contributes `false` — the same
    // conservative skip as the borrowed-ness and max-bytes resolvers.
    fn resolve_import_fallible(
        alias: &str,
        imports: &[forge::model::ForgeImport],
        base_dir: &Path,
        visited: &mut HashSet<PathBuf>,
    ) -> Option<bool> {
        let imp = imports.iter().find(|i| i.alias == alias)?;
        let imp_path = base_dir.join(&imp.src);
        let visit_key = imp_path.clone();
        if !visited.insert(visit_key.clone()) {
            return None;
        }
        let result = (|| -> Option<bool> {
            let content = std::fs::read_to_string(&imp_path).ok()?;
            let stem = imp_path.file_stem()?.to_str()?;
            let basename = imp_path.file_name()?.to_str()?;
            let label = DocumentLabel {
                identifier: stem,
                diagnostic_label: basename,
            };
            let parsed = forge::parser::parse_forge_with_imports(&content, label).ok()??;
            let inner_cm = match parsed.document {
                forge::model::ForgeDocument::Codec(c) => c,
                _ => return None,
            };
            let inner_base = imp_path.parent()?.to_path_buf();
            // Borrowed gate: a non-borrowed body has no `as_borrowed`
            // projection (it is cloned), so it can never contribute
            // fallibility regardless of its internal list fields.
            let mut borrow_visited: HashSet<PathBuf> = HashSet::new();
            borrow_visited.insert(visit_key.clone());
            let borrowed = codec_is_borrowed_recursive(
                &inner_cm,
                &parsed.imports,
                &inner_base,
                &mut borrow_visited,
            );
            if !borrowed {
                return Some(false);
            }
            Some(codec_as_borrowed_fallible_recursive(
                &inner_cm,
                &parsed.imports,
                &inner_base,
                visited,
            ))
        })();
        visited.remove(&visit_key);
        result
    }

    cm.is_as_borrowed_fallible_with(|alias| {
        resolve_import_fallible(alias, imports, base_dir, visited).unwrap_or(false)
    })
}

/// Validate and enrich `<sce:import>` declarations in a single pass.
///
/// For each import, reads the file once and performs:
/// 1. Existence check (`src` file must exist relative to `base_dir`)
/// 2. Kind validation (declared kind must match actual `sce:kind` in file)
/// 3. API enrichment for stateless kinds (discover primary function name
///    and build language-specific qualified call for expression aliasing)
fn validate_and_enrich_imports(
    import_ctx: &mut [forge::generator::ImportContext],
    imports: &[forge::model::ForgeImport],
    base_dir: &Path,
    language: &generator::Language,
    options: &crate::ForgeCompileOptions,
    importing_doc: &str,
) -> Result<(), forge::error::Located<forge::error::ForgeError>> {
    use forge::error::{ImportError, Located};

    for (ctx, imp) in import_ctx.iter_mut().zip(imports.iter()) {
        let src_path = base_dir.join(&imp.src);
        let src_label = src_path.display().to_string();

        // 1. Existence
        if !src_path.exists() {
            return Err(Located::new(
                ImportError::FileNotFound {
                    src: imp.src.clone(),
                    searched: src_label.clone(),
                }
                .into(),
                &src_label,
                None,
                None,
            ));
        }

        // Read once
        let content = std::fs::read_to_string(&src_path).map_err(|e| {
            Located::new(
                forge::error::ForgeError::Import(ImportError::ReadError {
                    src: imp.src.clone(),
                    source: e,
                }),
                &src_label,
                None,
                None,
            )
        })?;

        // 2. Kind validation
        let actual_kind = forge::parser::detect_kind(&content)
            .map_err(|e| Located::new(e, &src_label, None, None))?
            .ok_or_else(|| {
                Located::new(
                    forge::error::ForgeError::Import(ImportError::NotForge {
                        src: imp.src.clone(),
                    }),
                    &src_label,
                    None,
                    None,
                )
            })?;

        if actual_kind != imp.kind {
            // The `sce:kind="..."` attribute that disagrees lives in
            // the importing document, not in the imported file — so
            // the diagnostic's location anchors at `importing_doc` on
            // the `<sce:import>` element's own line (recorded during
            // `parse_imports`). Pointing at `src_label` was a double
            // mislabel: wrong file *and* no line.
            return Err(Located::new(
                ImportError::KindMismatch {
                    src: imp.src.clone(),
                    declared: imp.kind.to_string(),
                    actual: actual_kind.to_string(),
                }
                .into(),
                importing_doc,
                imp.line,
                None,
            ));
        }

        // 3. API enrichment (reuse already-read content). We parse the
        //    imported document once and extract both:
        //      • the qualified function call for stateless kinds (existing
        //        behavior), and
        //      • type information (param/return types for stateless kinds,
        //        member field types for stateful kinds) used by the typed
        //        expression transpiler pipeline in `forge::type_ctx`.
        let stem = src_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");
        let basename = src_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(stem);
        let imported_label = DocumentLabel {
            identifier: stem,
            diagnostic_label: basename,
        };

        if let Some(parsed) = forge::parser::parse_forge_with_imports(&content, imported_label)? {
            let doc = parsed.document;
            // Identity SSOT: recompute the name-derived emission context
            // (`#include` / `use` / `import`, namespace, type/member type)
            // from the imported document's authoritative
            // `ForgeDocument::name()` rather than the provisional file stem
            // `resolve_single_import` keyed on. The two agree for every kind
            // whose model name == file stem; only an `algorithm` carrying a
            // `name=` attribute distinct from its file stem diverges, and
            // there the imported document emits its file / namespace / module
            // from `name()` (see `generate_forge`), so the stem-based
            // provisional values dangle. This recompute lands the cross-doc
            // reference on the symbols the imported document actually emits,
            // uniformly across all six backends (the per-kind C11-only patch
            // that previously corrected only the include is now subsumed —
            // see the cross-doc call SSOT below).
            {
                let id = forge::generator::forge_import_identity(
                    doc.name(),
                    language,
                    ctx.is_stateful,
                    options,
                );
                ctx.include_stmt = id.include_stmt;
                ctx.type_name = id.type_name;
                ctx.namespace = id.namespace;
                ctx.member_type = id.member_type;
            }
            // RFC §5.B variant primitive (B1-β) + B5-ε surface G:
            // codec imports carry their *full recursive* max_frame_bytes
            // forward so the parent codec's variant emit / TLV chain
            // emit / repeat emit can size its encoded buffer to fit the
            // worst-case body even when the imported codec is itself
            // variant-bearing (carries its own arm bodies that
            // model-level `max_frame_bytes()` ignores). Non-codec
            // imports leave it `None`.
            if let forge::model::ForgeDocument::Codec(cm) = &doc {
                let mut visited: HashSet<PathBuf> = HashSet::new();
                visited.insert(src_path.clone());
                let inner_base = src_path
                    .parent()
                    .map_or_else(|| base_dir.to_path_buf(), |p| p.to_path_buf());
                ctx.codec_max_bytes = Some(compute_codec_recursive_max_bytes(
                    cm,
                    &parsed.imports,
                    &inner_base,
                    &mut visited,
                ));
                // Borrowed zero-copy codec round: capture whether the
                // imported codec's generated Rust struct carries a `<'a>`
                // lifetime (holds a `&'a [u8]` / `&'a str` view or embeds
                // a body codec that does). The parent codec uses this to
                // thread `<'a>` into its own struct/decode signature when
                // it embeds / repeats / variant-dispatches this import.
                let mut borrow_visited: HashSet<PathBuf> = HashSet::new();
                borrow_visited.insert(src_path.clone());
                ctx.codec_is_borrowed = codec_is_borrowed_recursive(
                    cm,
                    &parsed.imports,
                    &inner_base,
                    &mut borrow_visited,
                );
                // Owned→borrowed projection round: capture this import's
                // *contribution* to a parent's `as_borrowed` fallibility —
                // true only when it is borrowed (else cloned wholesale,
                // never fails) AND its own projection is fallible (holds a
                // bounded list, or embeds a fallible body). The parent
                // reads this to decide whether to call `_b.as_borrowed()`
                // or `_b.try_as_borrowed()?` on a field of this type, and
                // whether its own projection must return `Result`.
                ctx.codec_as_borrowed_fallible = ctx.codec_is_borrowed && {
                    let mut fallible_visited: HashSet<PathBuf> = HashSet::new();
                    fallible_visited.insert(src_path.clone());
                    codec_as_borrowed_fallible_recursive(
                        cm,
                        &parsed.imports,
                        &inner_base,
                        &mut fallible_visited,
                    )
                };
                // RFC Axis-1 inversion: capture the imported leaf
                // codec's declared `<sce:flag-inputs>` so the parent-
                // local cross-doc validator can confirm every input is
                // bound exactly once via the parent's authored
                // `<sce:flag-bind>` directives. Empty when the leaf
                // declares no flag-inputs.
                ctx.codec_flag_inputs = cm.flag_inputs.clone();
                // RFC §5.B Y3 atomic 2b-ii peek-byte: capture the
                // imported body codec's FIRST `<sce:flags>`-bearing
                // field at byte_offset=0 so the parent variant's
                // peek-byte cross-codec validator can confirm the
                // arm body's own header flag layout matches the
                // peek-byte's declaration. Only the first wire-zero
                // flag-bearing field qualifies — that's the byte the
                // peek would have observed.
                ctx.codec_first_flags = cm
                    .fields
                    .iter()
                    .find(|f| !f.flags.is_empty() && f.byte_offset == 0)
                    .map(|f| (f.id.clone(), f.flags.clone()));
                // RFC variant-default-uniformity Atomic γ-3b-go: the
                // imported codec emits a `NewT()` constructor iff its
                // own β-go gate fires — either any field declares a
                // `<sce:flag value=>` wire-MID, or it carries a
                // `<sce:variant>` whose marked-default arm steers a
                // non-zero Variant body pointer. Mirrors the
                // `has_flag_default or (has_variant and ns.default_arm)`
                // condition in `tools/codegen/templates/forge/go/codec.go.jinja2`
                // so cross-doc inner.NewT() calls stay name-stable.
                let inner_has_flag_default = cm
                    .fields
                    .iter()
                    .any(|f| f.flags.iter().any(|fl| fl.value.is_some()));
                let inner_has_default_arm = cm
                    .variant
                    .as_ref()
                    .is_some_and(|v| v.arms.iter().any(|a| a.is_default));
                ctx.codec_emits_default_ctor = inner_has_flag_default || inner_has_default_arm;
                // RFC B5-ν inversion enrichment: for ANY variant
                // codec import (regardless of legacy tag_scope), surface
                // the arm count and default-arm presence so the new
                // parent-local validator can enforce Q-D-3a (no
                // dispatch + no default arm = decode ambiguity) and
                // Q-D-5a (flag width fits arm count).
                if let Some(v) = cm.variant.as_ref() {
                    ctx.codec_variant_arms_for_inversion = Some(
                        v.arms
                            .iter()
                            .map(|arm| (arm.body_alias.clone(), arm.value, arm.is_default))
                            .collect(),
                    );
                    ctx.codec_variant_has_default_arm = Some(v.arms.iter().any(|a| a.is_default));
                    // RFC B5-ν inversion β shape: tag-less `<sce:variant>`
                    // signals the imported codec uses caller-tag dispatch.
                    // Parents importing this codec MUST supply a `tag: u8`
                    // arg at the leaf's decode call site.
                    ctx.codec_variant_is_caller_tag = v.tag_field.is_none();
                    // RFC B5-ν dispatcher-self-gen Gap 6 (extended to β):
                    // Rust requires explicit `use` for each item from a
                    // module; the dispatcher emits both the `<Stem>` struct
                    // AND the `<Stem>Variant` enum, and the parent's
                    // `b5_nu_derivation_block` match references the Variant
                    // enum. Brace-list emit fires when this parent declares
                    // OR-inversion dispatch on the import — either via the
                    // new `<sce:variant-dispatch>` (`embed_dispatch.is_some()`)
                    // or the legacy `tag="parent.X"` form (carried through
                    // `codec_b5_nu_parent_tag_flag`). Other variant imports
                    // (own-field Local dispatch) do not need the Variant
                    // enum at the parent — keep the bare-struct import to
                    // preserve byte-stable goldens for non-B5-ν consumers.
                    let _ = v; // variant existence already gated this branch
                    let needs_variant_import = imp.embed_dispatch.is_some();
                    if matches!(*language, generator::Language::Rust) && needs_variant_import {
                        // Identity SSOT: the imported module is named from the
                        // codec's `ForgeDocument::name()`, same source the
                        // general recompute above uses — never the file stem.
                        let snake = filters::to_snake_case(doc.name().to_string());
                        ctx.include_stmt = format!(
                            "use super::{snake}::{{{pascal}, {pascal}Variant}};",
                            snake = snake,
                            pascal = ctx.type_name,
                        );
                    }
                }
            }
            // RFC §5.C B6-α' cross-resolution: the link kind's
            // `<sce:rx-pool>` / `<sce:tx-pool>` cross-validator
            // (`validate_link_pool_framer_resolution`) needs the
            // imported pool's slot capacity at resolve time. Captured
            // here under the same enrichment pass that already populated
            // `codec_max_bytes` for the framer side, so the cross-
            // resolver finds both axes on the same `ImportContext`
            // slice without re-walking the imports.
            if let forge::model::ForgeDocument::BufferPool(pm) = &doc {
                ctx.buffer_pool_slot_size = Some(pm.slot_size);
            }
            // RFC §5.A line 311 + §5.L line 2642-2647 (C7-lowering
            // 2026-05-13): bounded-collection imports carry their
            // element-type snake form forward so the algorithm-over-BC
            // iter emit can name the codec's `<element_snake>_t`
            // typedef when the C11 backend stack-copies the element
            // value (dot-access body preservation — see
            // `ImportContext::bc_element_snake` for the cross-backend
            // contract).
            if let forge::model::ForgeDocument::BoundedCollection(bcm) = &doc {
                ctx.bc_element_snake = Some(filters::to_snake_case(bcm.element_type.clone()));
            }
            // NL→IR Item C1 Path A Atomic 2: capture the per-language
            // qualified type name of imported `sce:kind="enum"`
            // documents so downstream renderers can resolve
            // `SceType::Enum(EnumRef { alias })` via
            // `LangCtx::resolved_type` without re-implementing the
            // namespace / separator matrix at every emission site.
            // Mirrors the `ctx.bc_element_snake` enrichment pattern
            // for BoundedCollection imports above.
            if let forge::model::ForgeDocument::Enum(em) = &doc {
                let pascal = filters::to_pascal_case(em.name.clone());
                let snake = filters::to_snake_case(em.name.clone());
                ctx.enum_qualified_type = match language {
                    generator::Language::Cpp => {
                        format!("SCE::Generated::{pascal}::{pascal}")
                    }
                    generator::Language::Rust => format!("{snake}::{pascal}"),
                    // Kotlin: the `import com.sce.generated.<snake>.*`
                    // wildcard from `resolve_single_import` brings the
                    // enum class name into unqualified scope.
                    generator::Language::Kotlin => pascal.clone(),
                    generator::Language::Go => format!("{snake}.{pascal}"),
                    generator::Language::Python => format!("{snake}.{pascal}"),
                    // C11 has no namespace mechanism — the enum's
                    // typedef name carries a `_t` discriminator so
                    // cross-doc references resolve via the typedef
                    // alone (matches the codec / buffer-pool pattern).
                    generator::Language::C11 => format!("{pascal}_t"),
                };
            }
            if !ctx.is_stateful {
                // `ctx.namespace` was recomputed from `doc.name()` above, so
                // the qualifier composes against the symbols the imported
                // document actually emits. `forge_qualified_call` carries the
                // sole kind-specific exception (C11 algorithm → bare symbol);
                // the `#include` is already correct from the identity SSOT
                // recompute, so no per-kind include patch is needed here.
                if let Some(name) = discover_primary_function(&doc, language) {
                    ctx.qualified_call =
                        forge_qualified_call(&doc, &name, &ctx.namespace, language);
                }
                let (params, ret) = discover_stateless_signature(&doc);
                ctx.param_types = params;
                ctx.ret_type = ret;
            } else {
                // Qualify every discovered field key with the import's alias
                // so the typed expression pipeline can look it up via the
                // `"{alias}.{field}"` convention used by `infer_types` when
                // it encounters `Member{Ident(alias), field}` AST nodes.
                // Unqualified bare field names would collide with the
                // enclosing kind's own inputs/internals that happen to share
                // a name, silently producing wrong type inference.
                ctx.member_field_types = discover_stateful_member_fields(&doc)
                    .into_iter()
                    .map(|(field, ty)| (format!("{}.{}", ctx.alias, field), ty))
                    .collect();
                ctx.member_method_sigs = discover_stateful_member_methods(&doc)
                    .into_iter()
                    .map(|(method, params, ret)| (format!("{}.{}", ctx.alias, method), params, ret))
                    .collect();
            }
        }
    }
    Ok(())
}

/// RFC §5.C B6-α' cross-resolution: validate that every `<sce:rx-pool>`
/// / `<sce:tx-pool>` reference on a link kind binds to a buffer-pool
/// whose `<sce:slot-size>` is >= the framer codec's recursive
/// worst-case encoded byte count.
///
/// Runs **after** [`validate_and_enrich_imports`] populates the
/// `ImportContext` slice with the per-import `codec_max_bytes`
/// (codec arm) and `buffer_pool_slot_size` (buffer-pool arm). Skips
/// silently when:
///   * the root document is not a `LinkModel` — other kinds carry no
///     pool/framer pair to cross-check;
///   * the framer alias does not resolve to an enriched codec import
///     (e.g. inline-only framer, partial topology) — the existing
///     `link/framer-missing` parser-stage gate already rejects pure
///     absence; here we tolerate enrichment gaps the way the rest of
///     the cross-file pipeline does;
///   * the link's `<sce:rx-pool>` / `<sce:tx-pool>` alias does not
///     resolve to an enriched buffer-pool import — same tolerance.
///
/// The diagnostic locator anchors at `importing_doc` (the link's own
/// file) — same convention as `validate_and_enrich_imports` for kind
/// mismatches, since the offending element (`<sce:rx-pool>` /
/// `<sce:tx-pool>` ref) lives in the importing document.
fn validate_link_pool_framer_resolution(
    doc: &forge::model::ForgeDocument,
    import_ctx: &[forge::generator::ImportContext],
    importing_doc: &str,
) -> Result<(), forge::error::Located<forge::error::ForgeError>> {
    use forge::error::{Located, ValidationError};
    use forge::model::ForgeDocument;

    let link = match doc {
        ForgeDocument::Link(l) => l,
        _ => return Ok(()),
    };

    // Resolve the framer codec's recursive worst-case bytes via the
    // same `ImportContext` slice the codegen consumes for the variant
    // / repeat / tlv-chain max-bytes folding. `None` here means the
    // framer alias is not a cross-file codec import (or enrichment
    // gave up on it) — the cross-validator has nothing to compare
    // against and falls back to silent-skip.
    let framer_max_bytes = match import_ctx
        .iter()
        .find(|c| c.alias == link.framer)
        .and_then(|c| c.codec_max_bytes)
    {
        Some(n) => n,
        None => return Ok(()),
    };

    // Compare each declared pool side independently. Both can be
    // present, only one, or neither (the `None` arms are normal — a
    // link without zero-copy pools is a valid v1 shape, the framer-
    // only path).
    for (side, pool_ref) in [("rx", &link.rx_pool), ("tx", &link.tx_pool)] {
        let pool_alias = match pool_ref.as_deref() {
            Some(s) => s,
            None => continue,
        };
        let pool_slot_size = match import_ctx
            .iter()
            .find(|c| c.alias == pool_alias)
            .and_then(|c| c.buffer_pool_slot_size)
        {
            Some(n) => n,
            // Pool ref present but not resolvable as an enriched
            // buffer-pool import — same tolerance as the framer
            // branch above.
            None => continue,
        };
        if pool_slot_size < framer_max_bytes {
            return Err(Located::new(
                ValidationError::LinkPoolSlotSmallerThanFramerMax {
                    link_name: link.name.clone(),
                    pool_side: side,
                    pool_alias: pool_alias.to_string(),
                    pool_slot_size,
                    framer_alias: link.framer.clone(),
                    framer_max_bytes,
                }
                .into(),
                importing_doc,
                None,
                None,
            ));
        }
    }
    Ok(())
}

/// RFC §5.D C2-β worker cross-resolution. Mirrors
/// `validate_link_pool_framer_resolution` shape — operates on a single
/// parsed worker doc and resolves its `<sce:link-rx ref>` +
/// `<sce:outbox ref>` against `parsed.imports`.
///
/// Q-C2-3 (a) lock 2026-05-10 originally specified a separate
/// `ForgeWorkerRegistry`; Gate B preflight (2026-05-11) surfaced that
/// the registry would carry zero production population today (worker
/// docs cannot import other workers per C2-α layer 1, and the spec
/// example's outbox ref targets a state machine, not another worker).
/// The textbook narrowing follows the η-precedent of resolving cross-
/// refs directly against `parsed.imports` filtered by kind:
///   - `link-rx ref` → must match a `kind="link"` import alias.
///   - `outbox ref` (when present) → first-dot prefix must match a
///     `kind="statechart"` import alias.
///
/// Both diagnostic axes carry `Fix::ReplaceOneOf` over the sorted alias
/// candidate set of the expected kind. Silent-skip on non-Worker docs
/// matches the dispatch shape of `validate_link_pool_framer_resolution`.
fn validate_worker_cross_refs(
    doc: &forge::model::ForgeDocument,
    imports: &[forge::model::ForgeImport],
    importing_doc: &str,
) -> Result<(), forge::error::Located<forge::error::ForgeError>> {
    use forge::error::{Located, ValidationError};
    use forge::model::{ForgeDocument, ForgeKind};

    let worker = match doc {
        ForgeDocument::Worker(w) => w,
        _ => return Ok(()),
    };

    // ── link-rx ref → kind=link import alias ──
    //
    // Sort the kind=link candidate set so `Fix::ReplaceOneOf` payload
    // bytes stay deterministic (η-precedent: every `Fix` candidate
    // list is sorted to keep the wire id reproducible).
    let mut link_aliases: Vec<String> = imports
        .iter()
        .filter(|i| i.kind == ForgeKind::Link)
        .map(|i| i.alias.clone())
        .collect();
    link_aliases.sort();
    if !link_aliases.iter().any(|a| a == &worker.link_rx) {
        let candidates_list = link_aliases.join(", ");
        return Err(Located::new(
            ValidationError::WorkerLinkRxRefUnknown {
                worker_name: worker.name.clone(),
                ref_name: worker.link_rx.clone(),
                candidates: link_aliases,
                candidates_list,
            }
            .into(),
            importing_doc,
            None,
            None,
        ));
    }

    // ── outbox ref cross-resolution lives in a sibling validator ──
    //
    // [`validate_worker_outbox_references`] operates on the cross-doc
    // `SceCrossDocRegistry` (statechart + worker names across the build)
    // rather than this doc's own imports — outbox refs target peer
    // SCXML documents that the single-file
    // [`compile_forge_with_imports`] path cannot see. The orchestrator
    // [`compile_scxml_with_imports`] is the sole caller; single-file
    // forge compile paths accept any non-empty outbox value (the
    // forge-side parser at `forge/parser.rs` only enforces non-empty;
    // semantic resolution requires the cross-doc registry which
    // single-file paths do not assemble).

    Ok(())
}

/// watching-zenoh RFC §5.D C2 follow-up Atomic B (Q-Outbox-1..9
/// LOCKED 2026-05-12). SCXML-side `<sce:outbox ref="<owner>.<suffix>">`
/// cross-resolution against the build's
/// [`forge::cross_doc_registry::SceCrossDocRegistry`]. Three failure
/// axes map onto three spec-extension diagnostics per Q-Outbox-8 (c)
/// lock:
///
/// * `worker/outbox-target-suffix-invalid` — suffix !=  `inbox` (Q-Outbox-6
///   (a) strict-suffix lock). Includes the missing-dot case where the
///   ref lacks a `.` entirely; suffix surfaces as the empty string.
/// * `worker/outbox-ref-unknown` — owner segment not registered in any
///   parsed forge / SCXML doc.
/// * `worker/outbox-target-wrong-kind` — owner registered but kind is
///   neither statechart nor worker (Q-Outbox-3 (b) recipient kinds).
///
/// Suffix check fires first because it is syntactic (no registry
/// dependency); if it passes, owner resolution runs against the
/// statechart + worker union. The one-error-at-a-time wire policy
/// means an owner-typo on a suffix-typo ref surfaces only after the
/// suffix is repaired — but each error message names the exact axis
/// the author hit, so the repair sequence is bounded.
///
/// Workers without `<sce:outbox>` silent-skip (it is optional per
/// RFC §5.D worker schema). Empty registry + worker-with-outbox is
/// a legitimate caller bug (orchestrator should populate the
/// registry before invoking the validator) — surfaces as
/// `worker/outbox-ref-unknown` with `candidates` = empty Vec, which
/// is the same shape as an authored typo against a single-worker
/// build (no peer to send to).
///
/// Called by [`compile_scxml_with_imports`] after pass-2 statechart
/// registration. The per-doc validator [`validate_worker_cross_refs`]
/// (link-rx axis) runs in pass-3 codegen via
/// [`compile_forge_with_imports`]; the outbox axis cannot live there
/// because the registry it consults is built only by the
/// orchestrator.
fn validate_worker_outbox_references(
    workers: &[(String, forge::model::WorkerModel)],
    registry: &forge::cross_doc_registry::SceCrossDocRegistry,
) -> Result<(), forge::error::Located<forge::error::ForgeError>> {
    use forge::cross_doc_registry::ScxmlDocKind;
    use forge::error::{Located, ValidationError};

    for (diag_label, worker) in workers {
        let Some(outbox_value) = worker.outbox.as_ref() else {
            continue;
        };

        // Decompose `<owner>.<suffix>` per Q-Outbox-6 (a) shape lock.
        // `split_once` on `.` yields the first dot's left/right pair,
        // matching spec line 895's `session_fsm.inbox` form. A
        // missing dot routes to suffix-invalid with empty suffix —
        // the strict-suffix lock rejects bare `<owner>` per Q-Outbox-6
        // recommendation rationale (option (c) "bare owner" rejected
        // as a deprecated-on-arrival form).
        let (owner, suffix) = match outbox_value.split_once('.') {
            Some(pair) => pair,
            None => (outbox_value.as_str(), ""),
        };

        // ── Suffix-invalid axis (Q-Outbox-6 (a) strict-suffix) ──
        if suffix != "inbox" {
            return Err(Located::new(
                ValidationError::WorkerOutboxTargetSuffixInvalid {
                    worker_name: worker.name.clone(),
                    outbox_value: outbox_value.clone(),
                    owner: owner.to_string(),
                    suffix: suffix.to_string(),
                }
                .into(),
                diag_label.clone(),
                None,
                None,
            ));
        }

        // ── Owner resolution axis (Q-Outbox-3 (b) recipient kinds) ──
        //
        // Closed candidate union: every registered statechart +
        // worker, each suffixed with `.inbox` so candidates are
        // drop-in replacements for the entire `ref` attribute. Worker
        // self-reference is legal in spec terms (`outbox ref="<self>.
        // inbox"` would deliver into the worker's own inbox — odd
        // but not forbidden); we don't filter self out of the
        // candidate list because the validator surfaces resolution
        // failures, not stylistic guidance.
        let candidates: Vec<String> = registry
            .names_of_any_kind(&[ScxmlDocKind::Statechart, ScxmlDocKind::Worker])
            .into_iter()
            .map(|name| format!("{name}.inbox"))
            .collect();
        let candidates_list = candidates.join(", ");
        match registry.lookup(owner) {
            Some(ScxmlDocKind::Statechart) | Some(ScxmlDocKind::Worker) => {
                // Canonical case — passes.
            }
            Some(other_kind) => {
                return Err(Located::new(
                    ValidationError::WorkerOutboxTargetWrongKind {
                        worker_name: worker.name.clone(),
                        outbox_value: outbox_value.clone(),
                        owner: owner.to_string(),
                        actual_kind: other_kind.as_str().to_string(),
                        candidates,
                        candidates_list,
                    }
                    .into(),
                    diag_label.clone(),
                    None,
                    None,
                ));
            }
            None => {
                return Err(Located::new(
                    ValidationError::WorkerOutboxRefUnknown {
                        worker_name: worker.name.clone(),
                        outbox_value: outbox_value.clone(),
                        owner: owner.to_string(),
                        candidates,
                        candidates_list,
                    }
                    .into(),
                    diag_label.clone(),
                    None,
                    None,
                ));
            }
        }
    }
    Ok(())
}

/// watching-zenoh RFC §5.L C6 Atomic γ1 — parse a
/// `<sce:capacity source="deploy" key>` body into its
/// `(machine_segment, limit_name)` components, matching the
/// `machines.<machine>.limits.<limit>` shape from spec line 2570 +
/// 2583-2585. Returns `None` when the key has fewer than four
/// dot-separated segments OR doesn't start with `machines.` / contain
/// `.limits.`, signalling the validator to silent-skip per the
/// Q-η5 (a) precedent (malformed keys are tolerated at this layer
/// because α's parser accepts opaque strings; γ1 only resolves the
/// shape the spec defines).
///
/// The `limit_name` may itself contain dots (e.g. `local.subs.v2`);
/// the parse splits only the leading `machines.<m>.limits.` prefix
/// and treats the remainder verbatim as the limit name. The
/// `machine_segment` is the second dot-separated segment, with no
/// inner-dot tolerance (machine names are not dotted per the deploy
/// schema convention).
fn parse_bounded_collection_deploy_key(key: &str) -> Option<(&str, &str)> {
    let after_machines = key.strip_prefix("machines.")?;
    let dot_after_machine = after_machines.find('.')?;
    let machine_segment = &after_machines[..dot_after_machine];
    let after_machine = &after_machines[dot_after_machine + 1..];
    let limit_name = after_machine.strip_prefix("limits.")?;
    if limit_name.is_empty() {
        return None;
    }
    Some((machine_segment, limit_name))
}

/// watching-zenoh RFC §5.L C6 Atomic γ2/γ3 — look up `field` on the
/// resolved element-type [`forge::model::ForgeDocument`] and return
/// the abstract [`forge::model::SceType`] of that field. Each
/// backend's render fn converts the abstract type to its language-
/// specific string at codegen time via the existing `rust_type` /
/// watching-zenoh RFC §5.C line 806 (C10-α) — `Accepting.*` substate
/// presence walk over an `SCXMLModel`. The session-FSM canonical state
/// shape (`docs/session-fsm.md` §2.6, §2.7) names the accept-side
/// states `Accepting`, `Accepting.AwaitingInitSyn`,
/// `Accepting.SentInitAck`, etc.; the spec's dot-glob `Accepting.*`
/// is matched here by an ID prefix walk (Sub-Q-C10-α-3 (a) lock).
///
/// Match rule: a state-id matches when it is exactly `Accepting` OR
/// it begins with `Accepting.` (with the trailing dot). The trailing-
/// dot guard rejects unrelated state IDs that share the `Accepting`
/// stem (e.g. `AcceptingPayment`).
pub fn accepting_substate_present(model: &SCXMLModel) -> bool {
    model
        .states
        .keys()
        .any(|id| id == "Accepting" || id.starts_with("Accepting."))
}

/// Axis-3 inversion — resolve the
/// listener-pair set from the explicit cross-document role
/// declarations on both sides:
///
/// 1. **Deploy-side**: `LinkConfig.role: Some(Listener)`.
/// 2. **SCXML-side**: `SCXMLModel.declared_session_roles` contains
///    [`crate::model::SessionRoleKind::AcceptSide`].
///
/// Both halves must be declared for the link to join `listener_links`.
/// The historic substate-driven join (legacy `accepting_substate_
/// present` walker × `trust_class: session_arming`) was deleted —
/// the predicate function remains alive as the data source for the
/// parser-time migration-helper diagnostic
/// `scxml/accept-side-states-without-role-declaration` per Q-A8 (c).
///
/// The Q-A4 (d) matrix validator (in
/// [`validate_cross_doc_listener_roles`]) independently enforces
/// that any link with `role: listener` also carries
/// `trust_class: session_arming`, so the resolution path here does
/// not need to re-check trust class.
///
/// Returns the sorted `BTreeSet<String>` of listener link names —
/// the cross-doc consumer ([`crate::mesh::deploy::validate_reassembly_cross_doc`]
/// extended signature) and the codegen template populator
/// ([`ForgeCompileOptions::listener_links`]) both read this set.
/// `BTreeSet` keeps the iteration order deterministic so any
/// downstream `key_fragments` quoting derives a stable hash.
pub fn resolve_listener_links(
    deploy_cfg: &mesh::deploy::DeployConfig,
    scxml_models: &[(std::path::PathBuf, SCXMLModel)],
) -> std::collections::BTreeSet<String> {
    use crate::model::SessionRoleKind;
    use mesh::deploy::LinkRole;
    let mut listener_links: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for device in deploy_cfg.topology.values() {
        for (_machine_name, machine) in device.machines.iter() {
            // Resolve `machine.source` against the parsed scxml_models
            // by basename match — deploy.yaml's `source` is a path
            // string that the orchestrator passes verbatim to
            // `scxml_files`, so the two reach for the same filename.
            let machine_source = machine.source.as_str();
            let model = scxml_models.iter().find(|(path, _)| {
                path.file_name()
                    .and_then(|s| s.to_str())
                    .is_some_and(|n| n == machine_source)
                    || path.to_str().is_some_and(|p| p == machine_source)
            });
            let Some((_, model)) = model else {
                // Source not in the build's SCXML set — silent-skip
                // (deploy declares a machine whose SCXML is not part
                // of this compile call; the existing reassembly /
                // burst validators silent-skip on the same join
                // absence per Q-η5 (a)).
                continue;
            };
            let scxml_declares_accept_side = model
                .declared_session_roles
                .contains(&SessionRoleKind::AcceptSide);
            if !scxml_declares_accept_side {
                continue;
            }
            for (link_name, link) in machine.links.iter() {
                if matches!(link.role, Some(LinkRole::Listener)) {
                    listener_links.insert(link_name.clone());
                }
            }
        }
    }
    listener_links
}

/// Axis-3 inversion (Q-A4 + Q-A7)
/// — cross-document validation of explicit listener-role declarations.
/// Runs BEFORE [`resolve_listener_links`] so a partial-claim failure
/// is surfaced as a typed `link/...` or `scxml/...` diagnostic rather
/// than silently dropping into the listener-set union.
///
/// Three checks (each NeutralOrDeterministic non_overlap class):
///
/// 1. **Q-A4 (d) matrix**: deploy declares `role: listener` but
///    `trust_class != session_arming` ⇒
///    `link/role-listener-with-non-session-arming-trust-class`.
/// 2. **Deploy→SCXML partial-claim**: deploy declares `role: listener`
///    but the machine's source SCXML has no `<sce:session-role
///    kind="accept-side"/>` declaration ⇒
///    `link/deploy-role-listener-without-scxml-accept-side-role`.
/// 3. **SCXML→Deploy partial-claim**: SCXML declares `<sce:session-
///    role kind="accept-side"/>` but no deploy link on the matched
///    machine has `role: listener` ⇒
///    `scxml/accept-side-role-without-listener-link`.
///
/// Silent-pass cases (matching Q-A4 row table):
/// - `(role, trust_class) = (Some(Listener), SessionArming)`
/// - `role = Some(Initiator)` (forward-compat, v1 has no consumer)
/// - `role = None` (legacy fixtures pre-migration; partial-claim
///   discipline applies only when explicit declarations are present)
///
/// Per RFC Q-A9 this validator does NOT require `role: listener`
/// on every `session_arming` link — that promotion waits until
/// every fixture migrates to the explicit-role shape. It requires
/// consistency only among the explicit declarations actually present.
pub fn validate_cross_doc_listener_roles(
    deploy_cfg: &mesh::deploy::DeployConfig,
    scxml_models: &[(std::path::PathBuf, SCXMLModel)],
) -> Result<(), Box<crate::forge::error::ValidationError>> {
    use crate::forge::error::ValidationError;
    use crate::model::SessionRoleKind;
    use mesh::deploy::{LinkRole, TrustClass};

    for device in deploy_cfg.topology.values() {
        for (machine_name, machine) in device.machines.iter() {
            // Q-A4 (d) matrix — runs independently of the SCXML
            // model lookup since it's a deploy-internal check.
            for (link_name, link) in machine.links.iter() {
                if !matches!(link.role, Some(LinkRole::Listener)) {
                    continue;
                }
                let trust_class = link.domain_attrs.as_ref().map(|d| d.trust_class);
                match trust_class {
                    Some(TrustClass::SessionArming) => {
                        // Happy path — role/trust pair matches.
                    }
                    Some(other) => {
                        return Err(Box::new(
                            ValidationError::LinkRoleListenerWithNonSessionArmingTrustClass {
                                machine: machine_name.clone(),
                                link_name: link_name.clone(),
                                trust_class: other.as_str().to_string(),
                            },
                        ));
                    }
                    None => {
                        // `role: listener` without any `domain_attrs`
                        // block — still a matrix violation. The
                        // `actual` payload echoes the absent trust
                        // tier as `(absent)` so the failure shape is
                        // distinguishable from a present-but-wrong
                        // trust tier.
                        return Err(Box::new(
                            ValidationError::LinkRoleListenerWithNonSessionArmingTrustClass {
                                machine: machine_name.clone(),
                                link_name: link_name.clone(),
                                trust_class: "(absent)".to_string(),
                            },
                        ));
                    }
                }
            }

            // Cross-doc partial-claim — requires the matching SCXML
            // model to be in this compile call. Silent-skip when the
            // model is absent (mirrors `resolve_listener_links`
            // discipline; the existing cross-doc reassembly + burst
            // validators silent-skip on the same join absence per
            // Q-η5 (a)).
            let machine_source = machine.source.as_str();
            let model = scxml_models.iter().find(|(path, _)| {
                path.file_name()
                    .and_then(|s| s.to_str())
                    .is_some_and(|n| n == machine_source)
                    || path.to_str().is_some_and(|p| p == machine_source)
            });
            let Some((_, model)) = model else {
                continue;
            };

            let scxml_declares_accept_side = model
                .declared_session_roles
                .contains(&SessionRoleKind::AcceptSide);

            // Direction 1: deploy → SCXML.
            for (link_name, link) in machine.links.iter() {
                let deploy_role_is_listener = matches!(link.role, Some(LinkRole::Listener));
                if deploy_role_is_listener && !scxml_declares_accept_side {
                    return Err(Box::new(
                        ValidationError::LinkDeployRoleListenerWithoutScxmlAcceptSideRole {
                            machine: machine_name.clone(),
                            link_name: link_name.clone(),
                        },
                    ));
                }
            }

            // Direction 2: SCXML → deploy.
            if scxml_declares_accept_side {
                let any_listener_on_machine = machine
                    .links
                    .values()
                    .any(|link| matches!(link.role, Some(LinkRole::Listener)));
                if !any_listener_on_machine {
                    return Err(Box::new(
                        ValidationError::ScxmlAcceptSideRoleWithoutListenerLink {
                            machine: machine_name.clone(),
                            scxml_source: machine_source.to_string(),
                        },
                    ));
                }
            }
        }
    }
    Ok(())
}

/// `cpp_type` / `kotlin_type` / etc. helpers.
///
/// Mirrors the field enumeration used by
/// [`validate_bounded_collection_cross_refs`]'s `collection/index-by-
/// field-missing` axis (codec: `fields[].id` → `fields[].sce_type`;
/// procedure: `inputs[].id ⊕ internals[].id` → `*.sce_type`). The
/// cross-resolution validator guarantees the field exists on the
/// element-type before this helper runs from the orchestrator
/// populator, so a `None` return here would imply the model layout
/// changed since validation — unreachable on a healthy build.
/// RFC c7-wildcard W-project: the element-type's full
/// `(field_id, SceType, length_field)` schema, in declaration order.
/// Mirrors [`extract_bounded_collection_index_field_sce_type`] but returns
/// every field rather than one named axis, so an algorithm iterating the
/// BC can type each `entry.<field>` access.
///
/// The third tuple slot is the codec field's **explicit** `length_field`
/// (the `sce:length-field` source), carried so the C11 borrowed-bytes-view
/// projection (Q-W-5) references the *actual* length sibling rather than
/// guessing `<field>_len`. `None` for a procedure field, or for a codec
/// field with no length reference (a tail / fixed `bytes` field, whose C11
/// length sibling the codec emit auto-names `<field>_len` — there the
/// convention is structurally guaranteed, so `None` is a faithful signal
/// to use it). Same element-doc admission as the single-field extractor
/// (codec / procedure only — the element-type candidate map holds no other
/// kind).
fn extract_bounded_collection_element_field_sce_types(
    element_doc: &forge::model::ForgeDocument,
) -> ElementFieldSchema {
    use forge::model::ForgeDocument;
    match element_doc {
        ForgeDocument::Codec(codec) => codec
            .fields
            .iter()
            .map(|f| (f.id.clone(), f.sce_type.clone(), f.length_field.clone()))
            .collect(),
        ForgeDocument::Procedure(proc) => proc
            .inputs
            .iter()
            .chain(proc.internals.iter())
            .map(|f| (f.id.clone(), f.sce_type.clone(), None))
            .collect(),
        _ => Vec::new(),
    }
}

fn extract_bounded_collection_index_field_sce_type(
    element_doc: &forge::model::ForgeDocument,
    field: &str,
) -> Option<forge::model::SceType> {
    use forge::model::ForgeDocument;
    match element_doc {
        ForgeDocument::Codec(codec) => codec
            .fields
            .iter()
            .find(|f| f.id == field)
            .map(|f| f.sce_type.clone()),
        ForgeDocument::Procedure(proc) => proc
            .inputs
            .iter()
            .chain(proc.internals.iter())
            .find(|f| f.id == field)
            .map(|f| f.sce_type.clone()),
        // Element-type candidate map admits only codec / procedure
        // docs (pass-1 in `compile_scxml_with_imports`); other kinds
        // do not appear here. The validator above also rejects them
        // ahead of codegen via `collection/element-type-not-a-kind`.
        _ => None,
    }
}

/// watching-zenoh RFC §5.L C6 Atomic β (Q1 user direction
/// 2026-05-13: separate forge-doc map for codec/procedure
/// element-type candidates; Q2 user direction: build-wide cross-doc
/// scan for atomic intrinsic imports). Three failure axes per spec
/// lines 2566-2567 + 2615 + 2560-2562:
///
/// * `collection/element-type-not-a-kind` — body text of
///   `<sce:element-type>NAME` does not resolve in `element_type_candidates`
///   (the orchestrator-assembled `HashMap<String, ForgeDocument>` of
///   codec + procedure docs only). Closed candidate list rides
///   `Fix::ReplaceOneOf`.
/// * `collection/index-by-field-missing` — `<sce:index-by field=\"X\"/>`
///   names a field absent from the resolved element-type's struct.
///   Field enumeration mirrors [`discover_stateful_member_fields`]'s
///   codec + procedure arms (`CodecModel.fields[].id` for codecs,
///   `ProcedureModel.inputs[].id + .internals[].id` for procedures).
/// * `collection/multi-writer-without-atomics` — `<sce:concurrency>`
///   declared as `multi-writer` while the build's aggregated
///   `externs` slice contains no entry whose registry-
///   resolved purpose starts with `\"atomic-\"`. The C4 atomic A
///   baseline registry tags `atomic-load` / `atomic-store` /
///   `atomic-cas-*` / `atomic-fetch-*` uniformly via the
///   [`forge::intrinsic_registry::Symbol::purpose`] field, so a single
///   prefix scan covers the entire family.
///
/// Element-type resolution runs first (suffix-then-owner mirror of
/// [`validate_worker_outbox_references`]); index-by enumeration only
/// fires when element-type resolves to a candidate. Multi-writer runs
/// independently — it does not depend on element-type resolving
/// because the spec contract is "atomic imports must exist in the
/// build" regardless of whether the element type is well-formed.
///
/// The one-error-at-a-time wire policy means a doc with both an
/// unknown element-type and a missing index-by field surfaces only
/// the element-type failure on the first build cycle. After the
/// element-type is fixed, the next build cycle catches the index-by
/// field. Multi-writer is reported separately when the BC's
/// `concurrency` mode is `MultiWriter` and the atomic-import surface
/// is empty.
///
/// Called by [`compile_scxml_with_imports`] in pass-2 (after pass-1
/// captures the parsed forge docs and aggregates externs) and before
/// pass-3 codegen so a failing build fails early. Empty
/// `bounded_collections` returns Ok with no work — the orchestrator
/// callsite is a no-op for builds without any BC docs.
fn validate_bounded_collection_cross_refs(
    bounded_collections: &[(String, forge::model::BoundedCollectionModel)],
    element_type_candidates: &std::collections::HashMap<String, forge::model::ForgeDocument>,
    externs: &[forge::model::ExternDeclaration],
) -> Result<(), forge::error::Located<forge::error::ForgeError>> {
    use forge::error::{Located, ValidationError};
    use forge::model::{ConcurrencyMode, ForgeDocument};

    // Pre-compute the sorted closed candidate set once (used by both
    // element-type-not-a-kind and the index-by validator that depends
    // on element-type resolution). The map iteration order is
    // unspecified, so we sort on every call — the cost is bounded by
    // the number of codec + procedure docs in the build (typically
    // <50) and keeps the wire payload deterministic.
    let mut element_type_names: Vec<String> = element_type_candidates.keys().cloned().collect();
    element_type_names.sort();

    // Pre-compute the build-wide atomic-import surface answer once —
    // every BC checks against the same global slice. The C4 atomic A
    // baseline registry tags load/store/cas-weak/cas-strong/fetch
    // uniformly via `purpose: "atomic-<op>"`, so a single prefix scan
    // covers the entire family. Symbols absent from the baseline
    // registry (would-be plugin extensions) are silently skipped — a
    // plugin author who wires an atomic-family symbol still benefits
    // from declaring the appropriate purpose tag at registry merge
    // time per C4 atomic B's plugin loader contract.
    let build_has_atomic_import = externs.iter().any(|decl| {
        forge::intrinsic_registry::lookup_symbol(&decl.name)
            .is_some_and(|sym| sym.purpose.starts_with("atomic-"))
    });

    for (diag_label, bc) in bounded_collections {
        // ── Element-type kind resolution axis ──
        match element_type_candidates.get(bc.element_type.as_str()) {
            Some(doc) => {
                // ── Index-by field enumeration axis (runs only when
                //    element-type resolves; otherwise the user must
                //    fix element-type first per one-error-at-a-time
                //    wire policy). ──
                if let Some(field) = bc.index_by.as_ref() {
                    let (element_kind, mut field_candidates) = match doc {
                        ForgeDocument::Codec(m) => {
                            let names: Vec<String> =
                                m.fields.iter().map(|f| f.id.clone()).collect();
                            ("codec".to_string(), names)
                        }
                        ForgeDocument::Procedure(m) => {
                            let names: Vec<String> = m
                                .inputs
                                .iter()
                                .map(|f| f.id.clone())
                                .chain(m.internals.iter().map(|f| f.id.clone()))
                                .collect();
                            ("procedure".to_string(), names)
                        }
                        // Unreachable: `element_type_candidates` is
                        // populated only for codec + procedure docs in
                        // pass-1 of `compile_scxml_with_imports`. Other
                        // ForgeDocument variants never enter the map.
                        _ => continue,
                    };
                    if !field_candidates.iter().any(|f| f == field) {
                        field_candidates.sort();
                        let candidates_list = field_candidates.join(", ");
                        return Err(Located::new(
                            ValidationError::CollectionIndexByFieldMissing {
                                collection_name: bc.name.clone(),
                                field: field.clone(),
                                element_type: bc.element_type.clone(),
                                element_kind,
                                candidates: field_candidates,
                                candidates_list,
                            }
                            .into(),
                            diag_label.clone(),
                            None,
                            None,
                        ));
                    }
                }
            }
            None => {
                let candidates_list = element_type_names.join(", ");
                return Err(Located::new(
                    ValidationError::CollectionElementTypeNotAKind {
                        collection_name: bc.name.clone(),
                        element_type: bc.element_type.clone(),
                        candidates: element_type_names.clone(),
                        candidates_list,
                    }
                    .into(),
                    diag_label.clone(),
                    None,
                    None,
                ));
            }
        }

        // ── Multi-writer atomic-import surface axis ──
        if matches!(bc.concurrency, ConcurrencyMode::MultiWriter) && !build_has_atomic_import {
            return Err(Located::new(
                ValidationError::CollectionMultiWriterWithoutAtomics {
                    collection_name: bc.name.clone(),
                }
                .into(),
                diag_label.clone(),
                None,
                None,
            ));
        }
    }
    Ok(())
}

/// RFC §5.I lines 1755-1756 C2-β — codegen-invariant guard for SPSC
/// inbox ordering vs cross-core placement. Silent-skip when
/// `ForgeCompileOptions.worker_placement` is `None` (Q-η5 (a)
/// precedent: deploy-unaware path doesn't know cross-core
/// information). When present, walks the placement slice for an entry
/// matching the worker's name; fires
/// `worker/inbox-ordering-relaxed-across-cores` when the worker's
/// declared ordering is `relaxed` AND producer_core != consumer_core.
///
/// Placement entries that don't match the worker name silent-skip
/// (the slice may carry entries for sibling workers in a multi-worker
/// build). This matches the C5 deploy-aware validator pattern: the
/// caller assembles a slice; the validator queries by name without
/// requiring the slice to be exhaustive.
fn validate_worker_inbox_ordering_placement(
    doc: &forge::model::ForgeDocument,
    placement: Option<&[WorkerPlacement]>,
    importing_doc: &str,
) -> Result<(), forge::error::Located<forge::error::ForgeError>> {
    use forge::error::{Located, ValidationError};
    use forge::model::{ForgeDocument, InboxOrdering};

    let worker = match doc {
        ForgeDocument::Worker(w) => w,
        _ => return Ok(()),
    };

    let entries = match placement {
        Some(p) => p,
        // Deploy-unaware path — cannot determine cross-core layout,
        // so silent-skip per Q-η5 (a) precedent. C2-γ wires the
        // production populator from deploy.yaml.
        None => return Ok(()),
    };

    if worker.inbox.ordering != InboxOrdering::Relaxed {
        return Ok(());
    }

    for entry in entries {
        if entry.worker_name == worker.name && entry.producer_core != entry.consumer_core {
            return Err(Located::new(
                ValidationError::WorkerInboxOrderingRelaxedAcrossCores {
                    worker_name: worker.name.clone(),
                    producer_core: entry.producer_core,
                    consumer_core: entry.consumer_core,
                }
                .into(),
                importing_doc,
                None,
                None,
            ));
        }
    }

    Ok(())
}

/// Extract parameter and return types for a stateless imported kind.
///
/// * Transform → parameters are `inputs`, return is the first `outputs` entry
///   (transforms with multiple outputs cannot be represented as a single call
///   return — the expression transpiler treats them as opaque in that case).
/// * Condition → parameters are `inputs`, return is `Bool`.
/// * Lookup → parameter is `input`, return is `output`.
/// * Interpolation → parameters are opaque (vector-valued); returns `Float64`.
fn discover_stateless_signature(
    doc: &forge::model::ForgeDocument,
) -> (Vec<forge::model::SceType>, Option<forge::model::SceType>) {
    use forge::model::{ForgeDocument, SceType};
    match doc {
        ForgeDocument::Transform(m) => {
            let params: Vec<SceType> = m.inputs.iter().map(|f| f.sce_type.clone()).collect();
            let ret = if m.outputs.len() == 1 {
                Some(m.outputs[0].sce_type.clone())
            } else {
                None
            };
            (params, ret)
        }
        ForgeDocument::Condition(m) => {
            let params: Vec<SceType> = m.inputs.iter().map(|f| f.sce_type.clone()).collect();
            (params, Some(SceType::Bool))
        }
        ForgeDocument::Lookup(m) => (
            vec![m.input.sce_type.clone()],
            Some(m.output.sce_type.clone()),
        ),
        ForgeDocument::Interpolation(_) => {
            // Interpolation takes a typed input (x, or x+y for 2D) and returns
            // float64. Without opening up the Interpolation model further, we
            // treat parameters as empty (opaque) and return Float64.
            (Vec::new(), Some(SceType::Float64))
        }
        // RFC §5.A Algorithm: a stateless free function whose signature is
        // the declared `<sce:signature>` (params in positional order, an
        // optional return). Capturing it here lets `infer_types` resolve the
        // param/return types of a cross-algorithm dispatch (`eq(a, b)`), which
        // the c7-wildcard W-project byte-view projection consumes: a `bytes`
        // parameter receiving a bounded-string element field is projected to a
        // borrowed view at the call site (Q-W-5 (a) lock). Before this arm the
        // catch-all left `param_types`/`ret_type` empty, so cross-algorithm
        // calls inferred `Unknown` (harmless for C7's verbatim-arg dispatch,
        // insufficient for the type-driven projection).
        ForgeDocument::Algorithm(m) => {
            let params: Vec<SceType> = m
                .signature
                .params
                .iter()
                .map(|p| p.sce_type.clone())
                .collect();
            (params, m.signature.return_type.clone())
        }
        _ => (Vec::new(), None),
    }
}

/// Extract the list of (field_name, type) pairs exposed to user expressions
/// for a stateful imported kind.
///
/// The returned names must match the text a user would write in an expression
/// like `alias_.field_name` or `alias.field_name`, which corresponds to the
/// field IDs in the underlying kind's model.
///
/// * Codec → every field in `CodecModel.fields`.
/// * Validator → `inputs` (validator exposes the validated input value as its
///   primary field on the result; prev-values are internal).
/// * Filter → `output` and `input`.
/// * Observer → `inputs` and any exposed monitor state.
/// * Procedure → `inputs` + `internals` (stateful state machine fields).
/// * Timer → nothing (no user-visible fields in expressions).
fn discover_stateful_member_fields(
    doc: &forge::model::ForgeDocument,
) -> Vec<(String, forge::model::SceType)> {
    use forge::model::ForgeDocument;
    let mut out = Vec::new();
    match doc {
        // Statechart never reaches forge codegen — SCXML pipeline owns it.
        ForgeDocument::Statechart(_) => unreachable!(
            "discover_stateful_member_fields called on Statechart — only forge \
             pipeline reaches this helper (see `classify_document`)"
        ),
        ForgeDocument::Codec(m) => {
            for f in &m.fields {
                out.push((f.id.clone(), f.sce_type.clone()));
            }
        }
        ForgeDocument::Validator(m) => {
            for f in &m.inputs {
                out.push((f.id.clone(), f.sce_type.clone()));
            }
        }
        ForgeDocument::Filter(m) => {
            out.push((m.output.id.clone(), m.output.sce_type.clone()));
            out.push((m.input.id.clone(), m.input.sce_type.clone()));
        }
        ForgeDocument::Observer(m) => {
            for f in &m.inputs {
                out.push((f.id.clone(), f.sce_type.clone()));
            }
        }
        ForgeDocument::Procedure(m) => {
            for f in &m.inputs {
                out.push((f.id.clone(), f.sce_type.clone()));
            }
            for f in &m.internals {
                out.push((f.id.clone(), f.sce_type.clone()));
            }
        }
        ForgeDocument::Timer(_) => {}
        // RFC §5.C: Link is stateful (owns an `impl Link` driver) but
        // exposes no SCXML-expression-visible typed fields — the rx /
        // tx surface is method-only, and B6-α has no consumer that
        // calls them from authored expressions. Empty Vec keeps the
        // exhaustive match honest; a later atomic that exposes
        // method-typed members will add the method discovery to
        // `discover_stateful_member_methods`, not field discovery.
        ForgeDocument::Link(_) => {}
        // RFC §5.E: BufferPool is stateful (owns slot table + freelist)
        // but exposes no SCXML-expression-visible typed fields in B7-α
        // — acquire/release/slot/slot_mut/free_count are method-only.
        // Member discovery defers to the first authored consumer that
        // calls them via `<sce:call alias="..."/>` (analogous to Link's
        // method-only stance).
        ForgeDocument::BufferPool(_) => {}
        // RFC §5.D: Worker owns SPSC inbox state but exposes no
        // SCXML-expression-visible typed fields in C2-α — inbox
        // producer/consumer pair, optional outbox, link-rx binding
        // are all instance state but only addressable through methods
        // emitted at C2-β codegen time. Member discovery defers to
        // the first authored `<sce:call alias="..."/>` consumer.
        ForgeDocument::Worker(_) => {}
        // RFC §5.L: BoundedCollection owns the slot table, occupancy
        // mask, generation counters as instance state but exposes no
        // SCXML-expression-visible typed fields in C6-α — the
        // insert/remove/get/iter/len/capacity API is method-only per
        // spec lines 2609-2619 and lands in C6-γ codegen. Member
        // discovery defers to the first authored `<sce:call>` consumer.
        ForgeDocument::BoundedCollection(_) => {}
        // Stateless kinds handled via stateless_signature path.
        // Algorithm (RFC §5.A) is a stateless free function; no member
        // fields exposed to user expressions.
        ForgeDocument::Transform(_)
        | ForgeDocument::Condition(_)
        | ForgeDocument::Lookup(_)
        | ForgeDocument::Interpolation(_)
        | ForgeDocument::Algorithm(_)
        // NL→IR Item C1 Path A: Enum declares typed variants — no
        // SCXML-expression-visible member fields. Authors reference
        // variants as `<EnumName>.<variant>` (resolved through the
        // cross-kind binding pass), not as alias.field.
        | ForgeDocument::Enum(_)
        // NL→IR Item C1 Path A: EventSchema is parse-time metadata.
        // The payload contract lives in `_event.data.<field>`
        // resolution (handled by `event_schema_check.rs`, keyed by
        // SCXML event name) not as `alias.field` access on the
        // schema's import alias. No member surface visible through
        // the `alias.field` path.
        | ForgeDocument::EventSchema(_) => {}
    }
    out
}

/// Discover member method signatures for a stateful import.
///
/// Returns `(method_name, param_types, return_type)` triples. Only instance
/// methods appear here — static factory methods like `decode(raw)` are
/// type-level calls invoked as `TypeName.decode(...)`, not `alias.decode(...)`,
/// so they are not member methods on the imported alias.
///
/// Match is exhaustive over every `ForgeDocument` variant so that adding a
/// new kind to the model forces a compile error here, ruling out the
/// silently-broken case where a new stateful kind exposes methods that the
/// type inference pipeline does not know about. Stateless kinds (transform,
/// condition, lookup, interpolation) never reach this function via the
/// caller's `is_stateful` gate but are listed for the same exhaustiveness
/// guarantee — they return an empty Vec.
fn discover_stateful_member_methods(
    doc: &forge::model::ForgeDocument,
) -> Vec<(String, Vec<forge::model::SceType>, forge::model::SceType)> {
    use forge::model::{ForgeDocument, SceType};
    match doc {
        // Statechart never reaches forge codegen — SCXML pipeline owns it.
        ForgeDocument::Statechart(_) => unreachable!(
            "discover_stateful_member_methods called on Statechart — only forge \
             pipeline reaches this helper (see `classify_document`)"
        ),
        ForgeDocument::Codec(_) => vec![("encode".to_string(), vec![], SceType::Bytes)],
        ForgeDocument::Filter(m) => vec![(
            "update".to_string(),
            vec![m.input.sce_type.clone()],
            m.output.sce_type.clone(),
        )],
        // Stateful kinds whose method APIs are not yet imported by any
        // conformance fixture. Each returns an empty Vec until the first
        // load-bearing consumer lands; the comment lists the methods that
        // would belong here so the next contributor extends rather than
        // re-discovers them.
        // - Validator: validate(args) → ValidationResult
        // - Procedure: execute(handler, args) → ProcedureRunResult
        // - Observer:  update(args) → ()
        // - Timer:     fire() → ()
        // - Link (RFC §5.C): rx() → Option<RxFrame>, tx(TxFrame) → Result<(), LinkError>
        // - Worker (RFC §5.D): inbox.try_push(T) → bool, inbox.try_pop() → Option<T>
        //   (C2-β codegen emits the producer/consumer split; method names
        //   firm up alongside the template's `Producer<T,N>`/`Consumer<T,N>`
        //   API surface).
        ForgeDocument::Validator(_)
        | ForgeDocument::Procedure(_)
        | ForgeDocument::Observer(_)
        | ForgeDocument::Timer(_)
        | ForgeDocument::Link(_)
        | ForgeDocument::BufferPool(_)
        | ForgeDocument::Worker(_)
        // RFC §5.L BoundedCollection: methods insert/remove/get/
        // find_by_index/iter/len/capacity per spec lines 2609-2619 land
        // in C6-γ codegen. Until the first `<sce:call alias.insert(...)>`
        // consumer surfaces, member method discovery returns empty.
        | ForgeDocument::BoundedCollection(_) => Vec::new(),
        // Stateless kinds: caller filters via `is_stateful` before reaching
        // here. Listed so the match stays exhaustive — adding a new
        // ForgeDocument variant forces a decision at this site.
        // Algorithm (RFC §5.A) is a free function with no instance methods.
        ForgeDocument::Transform(_)
        | ForgeDocument::Condition(_)
        | ForgeDocument::Lookup(_)
        | ForgeDocument::Interpolation(_)
        | ForgeDocument::Algorithm(_)
        // NL→IR Item C1 Path A: Enum exposes no instance methods —
        // typed enum declaration emits a type, not a callable. Same
        // empty stance as stateless kinds.
        | ForgeDocument::Enum(_)
        // NL→IR Item C1 Path A: EventSchema exposes no instance
        // methods — the schema is type-only metadata, addressed via
        // SCXML event names, not method calls on an alias.
        | ForgeDocument::EventSchema(_) => Vec::new(),
    }
}

/// Discover the primary function name generated by a stateless forge document.
fn discover_primary_function(
    doc: &forge::model::ForgeDocument,
    language: &generator::Language,
) -> Option<String> {
    match doc {
        // Statechart never reaches forge codegen — SCXML pipeline owns it.
        forge::model::ForgeDocument::Statechart(_) => unreachable!(
            "discover_primary_function called on Statechart — only forge \
             pipeline reaches this helper (see `classify_document`)"
        ),
        forge::model::ForgeDocument::Transform(m) => {
            let output_id = m.outputs.first()?.id.clone();
            // Symbol-name SSOT: the cross-doc callsite resolves to the first
            // output's bare call-base (C11 returns `compute_<snake(output)>`);
            // build_qualified_call re-prepends `<namespace>_` on C11, landing
            // on the emitted `<m.name>_compute_<output>`.
            Some(forge::generator::forge_transform_symbol(&output_id, *language))
        }
        forge::model::ForgeDocument::Condition(m) => {
            // Symbol-name SSOT: the cross-doc callsite is the bare call-base
            // (C11 returns `check`); build_qualified_call re-prepends the
            // `<namespace>_` module prefix on C11, emitting `<m.name>_check`.
            Some(forge::generator::forge_condition_symbol(&m.name, *language))
        }
        forge::model::ForgeDocument::Lookup(m) => {
            // Symbol-name SSOT: the cross-doc callsite is the bare call-base
            // (C11 returns the verb-less `<snake(output_id)>`);
            // build_qualified_call re-prepends `<namespace>_` on C11, landing
            // on the emitted `<m.name>_<output_id>`.
            Some(forge::generator::forge_lookup_symbol(&m.output.id, *language))
        }
        forge::model::ForgeDocument::Interpolation(m) => {
            // Symbol-name SSOT: the def≠call kind. render_interpolation
            // defines the accessor (forge_interpolation_symbol) as a member
            // of a `<Pascal>` wrapper (struct/object/impl) on Cpp/Kotlin/Rust,
            // a free `Lookup` on Go, a module-level `lookup` on Python, and a
            // flat `<snake>_lookup` on C11. Cross-file consumers need the
            // wrapper qualifier; Go/Python resolve the bare name via the
            // import include_stmt; C11 uses the bare `lookup` base and
            // build_qualified_call re-prepends `<namespace>_` → `<m.name>_lookup`.
            let pascal = filters::to_pascal_case(m.name.clone());
            let base = forge::generator::forge_interpolation_symbol(*language);
            Some(match language {
                generator::Language::Cpp | generator::Language::Rust => {
                    format!("{pascal}::{base}")
                }
                generator::Language::Kotlin => format!("{pascal}.{base}"),
                generator::Language::Go
                | generator::Language::Python
                | generator::Language::C11 => base,
            })
        }
        // Stateful kinds (Codec, Validator, Procedure, Filter, Observer, Timer, Link)
        // use member access, not free function calls. They are handled by the
        // member rename mechanism in procedure and validator render functions.
        // Link (RFC §5.C) is stateful (owns its `impl Link` driver) — it has
        // no callsite-visible primary function name.
        forge::model::ForgeDocument::Codec(_)
        | forge::model::ForgeDocument::Validator(_)
        | forge::model::ForgeDocument::Procedure(_)
        | forge::model::ForgeDocument::Filter(_)
        | forge::model::ForgeDocument::Observer(_)
        | forge::model::ForgeDocument::Timer(_)
        | forge::model::ForgeDocument::Link(_)
        | forge::model::ForgeDocument::BufferPool(_)
        | forge::model::ForgeDocument::Worker(_)
        // RFC §5.L: stateful — uses member access via insert/remove
        // etc. (spec lines 2609-2619). No callsite-visible primary
        // free function name.
        | forge::model::ForgeDocument::BoundedCollection(_)
        // NL→IR Item C1 Path A: Enum emits a type declaration, not
        // a callable. Authors reference variants as `<EnumName>.<v>`,
        // resolved through the cross-kind binding pass; no primary
        // function name belongs at this site.
        | forge::model::ForgeDocument::Enum(_)
        // NL→IR Item C1 Path A: EventSchema is parse-time metadata
        // with no primary function callsite — the schema document
        // does not emit a callable surface (Atomic 4 emits a payload
        // struct, not a free function, and the SCXML event handler
        // is dispatched implicitly by event-name match, not by alias
        // function call).
        | forge::model::ForgeDocument::EventSchema(_) => None,
        // RFC §5.A Algorithm: free function whose name is the
        // SCXML-author-declared `name=` attribute, lowered to each
        // language's idiomatic identifier per RFC §5.J.5. The
        // cross-file consumer of `<sce:call target="algo_name"/>`
        // resolves through this name.
        forge::model::ForgeDocument::Algorithm(m) => {
            // Symbol-name SSOT: the cross-doc callsite resolves to the
            // exact symbol `render_algorithm` defines, so both sites read
            // `forge_algorithm_symbol`. The algorithm kind's definition
            // equals its discovery name (no wrapper qualification, unlike
            // Interpolation), so no extra call-side shaping is applied.
            Some(forge::generator::forge_algorithm_symbol(&m.name, *language))
        }
    }
}

/// Build a language-specific qualified function call from function name + namespace.
///
/// For stateless kinds the callsite replaces the user's alias (e.g.
/// `tempConvert(rawTemp)`) with a qualified path that resolves against the
/// `include_stmt` emitted by `resolve_single_import`. The pairings are:
///
/// * C++: `SCE::Generated::Pascal::funcName` — resolved by the `#include`
///   plus the fully-qualified namespace.
/// * Rust: `snake::func_name` — resolved by `use super::snake;` which brings
///   the imported file's module into scope.
/// * Go: `snake.FuncName` — package-qualified; import path itself is still
///   unresolved (see the `Go` branch of `resolve_single_import`).
/// * Python: `snake.func_name` — resolved by `from . import snake` which
///   exposes the module object as a local binding.
/// * Kotlin: bare `funcName` — the `import com.sce.generated.snake.*`
///   wildcard from `resolve_single_import` pulls every top-level function in
///   the imported package into scope, so a qualifier is unnecessary and
///   would in fact fail because Kotlin has no file-level namespace selector.
fn build_qualified_call(
    func_name: &str,
    namespace: &str,
    language: &generator::Language,
) -> String {
    match language {
        generator::Language::Cpp => format!("{namespace}::{func_name}"),
        generator::Language::Kotlin => func_name.to_string(),
        generator::Language::Rust => format!("{namespace}::{func_name}"),
        generator::Language::Go => format!("{namespace}.{func_name}"),
        generator::Language::Python => format!("{namespace}.{func_name}"),
        // C has no namespace mechanism — convention prefixes the module
        // name onto every exported function so cross-file imports never
        // collide. RFC §5.J.1 standardises on `<module>_<func>` to mirror
        // POSIX / lwIP / FreeRTOS style. Routed through the W1 SSOT
        // `forge_c11_flat` so the definition sites and this callsite flatten
        // identically (`namespace` is already snake — the op is idempotent).
        generator::Language::C11 => forge::generator::forge_c11_flat(namespace, func_name),
    }
}

/// SSOT for the cross-doc stateless call qualifier. Wraps
/// [`build_qualified_call`] with the one kind-specific exception that the
/// generic `<namespace>_<func>` C11 shape cannot express: the `algorithm`
/// kind.
///
/// `render_algorithm`'s C11 arm emits the function under its bare `name=`
/// symbol with **no** module prefix (the module *is* the function), so a
/// cross-doc call must be that bare symbol — `build_qualified_call`'s
/// `<namespace>_<func>` would compose `bytes_equal_bytes_equal` against
/// the defined `bytes_equal` and dangle. Every other C11 stateless kind
/// (lookup / condition) prefixes the module name onto its emitted symbol
/// (`<name>_<output>` / `<name>_check`), so the generic shape is correct
/// for them. Folding the exception here keeps the call-shaping rule in
/// one place instead of a compute-then-patch override at the enrichment
/// site.
fn forge_qualified_call(
    doc: &forge::model::ForgeDocument,
    func_name: &str,
    namespace: &str,
    language: &generator::Language,
) -> String {
    if matches!(doc, forge::model::ForgeDocument::Algorithm(_))
        && matches!(language, generator::Language::C11)
    {
        return func_name.to_string();
    }
    build_qualified_call(func_name, namespace, language)
}

/// Build a forge dependency manifest from a directory of SCXML files.
///
/// Scans `dir` for `.scxml` files, extracts `sce:kind` and `<sce:import>`,
/// and produces a JSON-serializable manifest with topological build order.
///
/// Errors carry the scanned directory as their `location.file` so CLI
/// diagnostics and agents see *where* the manifest build failed even
/// when the failure is a cross-file concern (circular imports, missing
/// files) rather than a single-document parse error.
pub fn build_forge_manifest(
    dir: &std::path::Path,
) -> Result<forge::model::ForgeManifest, forge::error::Located<forge::error::ForgeError>> {
    forge::manifest::build_manifest(dir)
}

/// Result of a mesh transport compilation — generated files plus all
/// build-time warnings collected during the pipeline.
pub struct MeshResult {
    /// Generated transport routing files.
    pub output: generator::GeneratedOutput,
    /// Dynamic target warnings (targetexpr cannot be statically resolved).
    pub dynamic_target_warnings: Vec<mesh::topology::TopologyWarning>,
    /// Informational notices when SCE_MESH.md §9.5 deadline precedence
    /// silently overrides a deploy.yaml binding-level deadline with a
    /// per-invoke `<param name="_mesh_deadline_ms">` value.
    pub deadline_override_notices: Vec<mesh::topology::DeadlineOverrideNotice>,
    /// Auto-symmetry subscription sites injected by the topology analyzer
    /// (SCE_MESH.md §13). Each entry means an `<onexit>` unsubscribe was
    /// synthesized for a qualifying `<onentry>` subscribe.
    pub auto_subscriptions: Vec<mesh::topology::AutoSubscription>,
    /// Lint notices for subscribe sends that did not qualify for
    /// auto-symmetry (nested in conditional, manual unsubscribe present,
    /// duplicate). Non-fatal informational output.
    pub subscription_lint_notices: Vec<mesh::topology::SubscriptionLintNotice>,
    /// SCE_MESH.md §16.4 auto-merge notices — one per R1/R2
    /// constraint the permissive-mode resolver collapsed into a
    /// single partition. Empty in strict mode (strict never
    /// merges — it errors).
    pub distributability_merge_notices: Vec<mesh::distributability::MergeNotice>,
    /// SCE_MESH.md §16.3 R3 snapshot-read notices — advisory only,
    /// identifies sibling regions that read an ancestor data
    /// location another region writes. Build never fails on R3.
    pub distributability_snapshot_notices: Vec<mesh::distributability::SnapshotNotice>,
}

/// Generate mesh transport routing code for an SCXML model.
///
/// This is the single public API for the mesh pipeline — CLI, test harness,
/// and build.rs all go through here. The pipeline is ordered so that each
/// stage's precondition is established by an earlier stage, and so that
/// architectural errors surface before implementation errors:
///
/// Step 1. Parse deploy.yaml (device-shared `transports:` and per-target
///         `bindings:` both validated at this stage; invalid values like
///         `mode: pier` are rejected here, before any topology work).
///
/// Step 1b. Resolve external infrastructure config (SCE_MESH.md §13):
///          load each device's vsomeip.json and resolve name-based binding
///          references into numeric IDs before topology runs. Reserved
///          SOME/IP ID key names in deploy.yaml are hard errors here.
///
/// Step 2. Collect <send> targets from the model (single pass).
///
/// Step 2a. Emit targetexpr warnings (dynamic targets cannot be statically resolved).
///
/// Step 2b. Resolve targets against deploy.yaml bindings.
///
/// Step 2c. Pattern capability validation — architectural: is the bound transport
///          even capable of the requested communication pattern? (e.g. zenoh
///          cannot do request/reply). Runs BEFORE event coverage because a
///          transport mismatch is a deploy.yaml design error.
///
/// Step 2d. Event coverage validation — implementation: does the receiver have
///          a matching <transition> for every sent event?
///
/// Step 3. Transport codegen (template rendering). Device-shared session config
///         is read directly from `DeployConfig` (no extraction/merging step —
///         the schema makes shared config explicit).
///
/// Inject server-response synthetic sends into the model.
///
/// Must be called BEFORE SM code generation (`generate_cpp`, etc.) when
/// `--deploy` is provided and the machine is an RPC server. The SM
/// generator must see the injected `<send>` actions to emit
/// `raiseExternal` calls that trigger the mesh send callback for
/// server response routing via `handleServerResponse`.
///
/// Idempotent: safe to call multiple times (the underlying injection
/// skips raises that already have a following synthetic send).
///
/// Auto-symmetry injection (`inject_auto_subscriptions`) is NOT included
/// here because unsubscribe events may lack Event enum variants in the
/// SM — those sends are transport-level lifecycle actions that only the
/// transport codegen needs to see.
/// SCE_MESH.md §14 rule 12: populate partition-aware codegen context
/// on `model`. Always sets `partition_context_present` based on whether
/// `model.name` (or its `source:`-aliased deploy-yaml name) appears
/// under any `partitions.<name>.machines:` list. When `for_partition`
/// names a concrete partition, additionally populates
/// `partition_parallel_roles` with per-`<parallel>` role assignments
/// (Root / NonRoot / SinglePartition) for that partition's
/// perspective.
///
/// Must run BEFORE C++ SM code generation so the template dispatches
/// between the inline `<parallel>`-final branch and the delegated
/// `mesh/cpp/parallel_final.jinja2` include, and within the delegate
/// between the three per-parallel role branches. Idempotent: the
/// output is a pure function of the deploy.yaml, the machine
/// identifier, and the partition identifier.
///
/// Returns the resolved `partition_context_present` flag so CLI /
/// build.rs callers can log or test the membership decision.
pub fn inject_partition_context_flag(
    model: &mut SCXMLModel,
    deploy_path: &Path,
) -> Result<bool, mesh::error::MeshError> {
    inject_partition_context_for(model, deploy_path, None)
}

/// Partition-aware form of [`inject_partition_context_flag`]. Pass
/// `Some(<partition_name>)` to fill the per-`<parallel>` role map for
/// that partition's codegen build; pass `None` to keep the role map
/// empty (matching the pre-rule-12 scaffolding behaviour).
pub fn inject_partition_context_for(
    model: &mut SCXMLModel,
    deploy_path: &Path,
    for_partition: Option<&str>,
) -> Result<bool, mesh::error::MeshError> {
    let deploy_cfg = mesh::deploy::parse_deploy(deploy_path)?;
    let deploy_dir = deploy_path.parent().unwrap_or(Path::new("."));
    // SCE_MESH.md §16.3: run the distributability analyzer before
    // partition-context injection so wire-21 role assignment sees the
    // post-merge plan instead of the author's (possibly violating)
    // original.
    let resolved = mesh::resolve_deploy_config(deploy_dir, &deploy_cfg)?;
    let resolved_name = if deploy_cfg.device_for_machine(&model.name).is_some() {
        model.name.clone()
    } else {
        deploy_cfg
            .find_machine_name_by_source(&model.name)
            .unwrap_or_else(|| model.name.clone())
    };
    let present = mesh::partitions::is_machine_partition_listed(&deploy_cfg, &resolved_name);
    model.partition_context_present = present;
    model.partition_parallel_roles.clear();
    model.partition_wire21_outbound_routes.clear();
    model.partition_wire21_inbound_sources.clear();
    model.partition_self_name = None;
    model.partition_barrier_timeouts.clear();
    model.scxml_remote_outbound_peers.clear();
    model.scxml_remote_inbound_peers.clear();

    // SCE_MESH.md §16.4 / §16.7 liveness opt-in. Set whenever the
    // machine declares `liveliness:` in deploy.yaml, regardless of
    // partition context — the codegen gate
    // `reject_liveliness_without_handler` is symmetric for row 8
    // (`PEER_PARTITIONED`, any machine) and row 13
    // (`REGION_PARTITIONED`, partitioned machine). Transport
    // emission is keyed on `partition_self_name` (row 13 tokens)
    // or the mesh transport codegen's direct deploy.yaml read
    // (row 8 tokens); this flag only drives the SM-level gate.
    model.machine_liveliness_opt_in = deploy_cfg
        .device_for_machine(&resolved_name)
        .and_then(|d| d.machines.get(&resolved_name))
        .and_then(|m| m.liveliness)
        .is_some();

    if let Some(partition_name) = for_partition {
        if let Some(partitions) = resolved.partitions() {
            let Some(decl) = partitions.get(partition_name) else {
                return Err(mesh::error::MeshError::from(
                    mesh::error::DeployError::PartitionParallelRootNotInMachines {
                        partition: partition_name.to_string(),
                        claimed_machine: resolved_name.clone(),
                        partition_machines: vec![],
                    },
                ));
            };

            model.partition_self_name = Some(partition_name.to_string());

            // Claims made by the selected partition — used to mark
            // `<parallel>` ids where this partition is Root.
            let claimed_roots: std::collections::BTreeSet<&String> = decl
                .hosts_parallel_roots
                .as_ref()
                .map(|v| {
                    v.iter()
                        .filter(|e| e.machine == resolved_name)
                        .map(|e| &e.parallel)
                        .collect()
                })
                .unwrap_or_default();

            // Regions of `resolved_name` hosted by ANY partition —
            // used to detect whether a `<parallel>` is distributed.
            let mut parallel_partitions: std::collections::BTreeMap<
                &String,
                std::collections::BTreeSet<&String>,
            > = std::collections::BTreeMap::new();
            for (part_name, part_decl) in partitions.iter() {
                for r in &part_decl.contains.parallel_regions {
                    if r.machine != resolved_name {
                        continue;
                    }
                    for (parallel_id, regions) in &model.parallel_regions {
                        if regions.iter().any(|reg| reg == &r.region) {
                            parallel_partitions
                                .entry(parallel_id)
                                .or_default()
                                .insert(part_name);
                        }
                    }
                }
            }

            // SCE_MESH.md §16.5 wire-21 routing: build a per-`<parallel>`
            // map of (parallel_id → claimant partition name) by scanning
            // every partition's `hosts_parallel_roots:` entries that
            // reference `resolved_name`. Rule 12's per-`(machine, parallel)`
            // uniqueness invariant guarantees at most one claimant per
            // parallel, so the lookup table is a plain map (not a multi-
            // map). Distributed parallels with no claimant cannot reach
            // here — `validate_parallel_root_designation` rejects them
            // before codegen runs.
            let mut parallel_root_partition: std::collections::BTreeMap<&String, &String> =
                std::collections::BTreeMap::new();
            for (part_name, part_decl) in partitions.iter() {
                if let Some(claims) = &part_decl.hosts_parallel_roots {
                    for entry in claims {
                        if entry.machine == resolved_name {
                            parallel_root_partition.insert(&entry.parallel, part_name);
                        }
                    }
                }
            }

            let mut wire21_inbound: std::collections::BTreeSet<String> =
                std::collections::BTreeSet::new();

            for parallel_id in model.parallel_regions.keys() {
                let hosting = parallel_partitions
                    .get(parallel_id)
                    .cloned()
                    .unwrap_or_default();
                let role = if hosting.len() < 2 {
                    // Regions live in at most one partition — legacy
                    // ParallelCompletionHelper path is correct.
                    model::PartitionRole::SinglePartition
                } else if claimed_roots.contains(parallel_id) {
                    model::PartitionRole::Root
                } else if hosting.contains(&partition_name.to_string()) {
                    model::PartitionRole::NonRoot
                } else {
                    // This partition hosts no region of the
                    // `<parallel>` — treat as SinglePartition (template
                    // won't render the inline branch because
                    // `entry_exit_actions.jinja2` only emits
                    // `<parallel>`-final code for regions hosted here).
                    model::PartitionRole::SinglePartition
                };

                // SCE_MESH.md §16.5: derive the wire-21 routes from the
                // role assignment. NonRoot ⇒ outbound (one entry per
                // hosted parallel, keyed by parallel_id, valued by the
                // root partition name). Root ⇒ inbound (every other
                // partition that hosts at least one region of this
                // parallel becomes a wire-21 source). SinglePartition
                // generates no wire-21 traffic by definition.
                match role {
                    model::PartitionRole::NonRoot => {
                        if let Some(root_part) = parallel_root_partition.get(parallel_id) {
                            model
                                .partition_wire21_outbound_routes
                                .insert(parallel_id.clone(), (*root_part).clone());
                        }
                    }
                    model::PartitionRole::Root => {
                        for src in &hosting {
                            if src.as_str() != partition_name {
                                wire21_inbound.insert((*src).clone());
                            }
                        }
                    }
                    model::PartitionRole::SinglePartition => {}
                }

                model
                    .partition_parallel_roles
                    .insert(parallel_id.clone(), role);

                // SCE_MESH.md §16.5 L3500 barrier-timeout plumbing.
                // Rule 12 pins exactly one claimant per distributed
                // parallel, and `barrier_timeout_ms:` on a non-
                // root-claiming partition is rejected at deploy time
                // (`partition-barrier-timeout-without-root`). So the
                // finite value, if any, lives on the partition we
                // matched as Root above — look it up once and stamp
                // the per-parallel map. SinglePartition parallels use
                // the legacy inline helper and never need a timer;
                // NonRoot partitions hold no tracker. A Root role
                // whose partition set None (W3C normative infinity)
                // leaves the map entry absent, which the jinja2
                // template interprets as "no TimerHooks emitted".
                if role == model::PartitionRole::Root {
                    if let Some(timeout_ms) = decl.barrier_timeout_ms {
                        model
                            .partition_barrier_timeouts
                            .insert(parallel_id.clone(), timeout_ms);
                    }
                }
            }

            model.partition_wire21_inbound_sources = wire21_inbound.into_iter().collect();
        }
    }

    classify_remote_scxml_invokes(model, &deploy_cfg, &resolved_name);
    collect_scxml_remote_peers(model, &deploy_cfg, &resolved_name, deploy_path);
    validate_scxml_invoke_target_exclusivity(model, &deploy_cfg, &resolved_name, deploy_path)
        .map_err(mesh::error::MeshError::from)?;
    validate_scxml_invoke_transport(model, &deploy_cfg, &resolved_name)
        .map_err(mesh::error::MeshError::from)?;

    Ok(present)
}

/// SCE_MESH.md §9.6 L1393 — cross-device scxml-remote invoke transport
/// validator. The classifier
/// ([`classify_remote_scxml_invokes`]) has already marked each
/// `Invoke::Scxml` that crosses a partition with
/// `remote_mesh_target`/`remote_mesh_transport`; this pass layers the
/// device-identity check on top and rejects configurations that would
/// either emit no wire traffic (missing binding) or crash at link time
/// (incapable transport) or silently degrade to `SESSION_F_TRANSPORT_UNAVAILABLE`
/// at runtime because the Session 2 C++ wire-14/20 dispatch has not
/// been wired for the declared transport.
///
/// Same-device cross-partition peers remain accepted without a
/// `bindings` entry — they take the implicit shm channel which is the
/// only wired path today (§9.6.2). Cross-device is defined as "peer's
/// device ≠ parent's device" per §14 rule 7 (each partition is
/// single-device, so cross-device ⇔ the peer machine lives on a
/// different `topology.<device>` entry).
///
/// Scope: outbound-side only. A cross-device misdeclaration always
/// surfaces on the parent's codegen run, so inbound-side re-check is
/// redundant — Session 2's C++ wiring will pull symmetric pair
/// validation when the dispatch lands.
fn validate_scxml_invoke_transport(
    model: &SCXMLModel,
    deploy_cfg: &mesh::deploy::DeployConfig,
    resolved_name: &str,
) -> Result<(), mesh::error::DeployError> {
    // Locate the parent's device name. Absent ⇒ parent not deployed;
    // upstream parser rejected, so staying silent here avoids piling a
    // second error on top. Mirrors the fail-silent convention in
    // `validate_scxml_invoke_target_exclusivity`.
    let Some(parent_device) = device_name_for(deploy_cfg, resolved_name) else {
        return Ok(());
    };

    for peer_binding in &model.scxml_remote_outbound_peers {
        let peer_name = &peer_binding.name;
        let Some(peer_device) = device_name_for(deploy_cfg, peer_name) else {
            // Unknown peer — classifier should not have flagged this,
            // but defend against future classifier changes by skipping.
            continue;
        };
        if peer_device == parent_device {
            // Same-device cross-partition — implicit shm path is the
            // only wired codegen today. No binding required.
            continue;
        }

        // Cross-device. Now discriminate the failure shape.
        let failure: Option<mesh::error::ScxmlInvokeCrossDeviceFailure> =
            match &peer_binding.transport {
                None => Some(mesh::error::ScxmlInvokeCrossDeviceFailure::MissingBinding),
                Some(t) if t == "shm" || t == "local" => Some(
                    mesh::error::ScxmlInvokeCrossDeviceFailure::TransportIncapable {
                        transport: t.clone(),
                    },
                ),
                Some(t) if t == "custom_tcp" => {
                    // SCE_MESH.md §9.6 L1393 Session 2: custom_tcp
                    // scxml-remote is wired. Reject only when the
                    // device-shared server cannot be emitted for lack
                    // of a listen endpoint on either side. Both
                    // parent and peer devices need `listen:` because
                    // scxml-remote invoke is bidirectional — parent
                    // receives wire-15/16/18/20 replies, peer
                    // receives wire-14/17/19 requests.
                    let missing_device = [&parent_device, &peer_device].iter().find_map(|dev| {
                        let has_listen = deploy_cfg
                            .topology
                            .get(*dev)
                            .and_then(|d| d.transports.custom_tcp.as_ref())
                            .and_then(|c| c.listen.as_ref())
                            .is_some();
                        (!has_listen).then(|| (*dev).clone())
                    });
                    missing_device.map(|device| {
                        mesh::error::ScxmlInvokeCrossDeviceFailure::TransportListenMissing {
                            transport: t.clone(),
                            device,
                        }
                    })
                }
                Some(t) if t == "someip" => {
                    // SCE_MESH.md §9.6 L1393 Session 4b: someip
                    // scxml-remote is wired. `vsomeip.json` OEM
                    // boundary validation belongs to §13 topology
                    // configuration (vsomeip_config validator + the
                    // OEM's own deploy pipeline), so we do not
                    // re-check applications[*] here — duplicating
                    // that validation would fracture the §13
                    // ownership boundary. The consolidated SCE app
                    // `<machine>[_<partition>]_sce_app_` (RFC F.X-2)
                    // is created unconditionally at codegen time; deploy-time
                    // failures surface through vsomeip runtime init
                    // returning false (TransportRouter::init → false
                    // propagates to the caller).
                    None
                }
                Some(t) if t == "zenoh" => {
                    // SCE_MESH.md §9.6 L1393 Session 5: zenoh
                    // scxml-remote is wired. The §9.6 endpoint
                    // shares the device-wide `zenoh_session_`
                    // (Zenoh has no §13 OEM boundary equivalent —
                    // SCE-reserved namespace is carved out via the
                    // `sce/scxml_invoke/...` key prefix instead),
                    // so there is no listen / endpoint gate to
                    // validate at this layer. Deploy-time session
                    // failures surface through `zenoh::Session::open`
                    // throwing `zenoh::ZException`, which the
                    // template's `try { ... }` block turns into
                    // `init()` returning false.
                    None
                }
                Some(t) => {
                    // Structurally capable (dds / can) but the C++
                    // wire-14/20 dispatch has not landed yet.
                    // Mirrors `partition-wire21-custom-tcp-
                    // unimplemented`: build-time rejection beats
                    // runtime silent fallback.
                    Some(
                        mesh::error::ScxmlInvokeCrossDeviceFailure::TransportUnwired {
                            transport: t.clone(),
                        },
                    )
                }
            };

        if let Some(failure) = failure {
            return Err(mesh::error::DeployError::ScxmlInvokeCrossDeviceTransport(
                Box::new(mesh::error::ScxmlInvokeCrossDeviceTransportPayload {
                    parent: resolved_name.to_string(),
                    peer: peer_name.clone(),
                    parent_device,
                    peer_device,
                    failure,
                }),
            ));
        }
    }
    Ok(())
}

/// Resolve the `topology.<device>` key that hosts `machine`. Returned
/// as an owned `String` because most callers stash it in an error
/// payload; the lookup is O(devices × machines) which is constant for
/// realistic deployments.
fn device_name_for(deploy_cfg: &mesh::deploy::DeployConfig, machine: &str) -> Option<String> {
    deploy_cfg
        .topology
        .iter()
        .find_map(|(dev_name, dev)| dev.machines.contains_key(machine).then(|| dev_name.clone()))
}

/// SCE_MESH.md §9.6 — codegen-shape exclusivity. Reject any deployment
/// where a single machine M is simultaneously (a) invoked by a sibling
/// through the mesh shape `<invoke type="scxml" src="#M">` and (b)
/// invoked through a local-path shape `<invoke src="<M's source file>">`.
/// The two shapes demand different child SM code generation: the mesh
/// shape is default-constructed by
/// [`ChildSessionAdapter<Engine>`](../../sce/include/mesh/ChildSessionAdapter.h)
/// (§9.6 child session lifecycle) and routes `<send target="#_parent">`
/// through the mesh callback, while the local-path shape threads a
/// `ParentStateMachine` template parameter and a `parent_` pointer
/// through the ctor. Supporting both simultaneously would silently break
/// one caller.
///
/// Activates only when this machine already has at least one inbound
/// mesh peer (`scxml_remote_inbound_peers` non-empty) — otherwise no
/// mesh-shape constraint applies and the author may use the local-path
/// shape freely. The sibling scan reads each other machine's SCXML text
/// and matches any `<invoke>` tag whose `src=` is a non-hash path that
/// canonicalizes to this machine's own SCXML source.
///
/// File-read or canonicalize failures fall through silently — mirroring
/// [`collect_scxml_remote_peers`], which cannot distinguish "file not
/// on disk yet" from "file unreadable" and opts for the safer
/// under-report than to surface an IO error from a mesh-inference path.
fn validate_scxml_invoke_target_exclusivity(
    model: &SCXMLModel,
    deploy_cfg: &mesh::deploy::DeployConfig,
    resolved_name: &str,
    deploy_path: &Path,
) -> Result<(), mesh::error::DeployError> {
    if model.scxml_remote_inbound_peers.is_empty() {
        return Ok(());
    }

    // Locate this machine's SCXML source path. Absent ⇒ upstream parser
    // already rejected the deployment; stay silent to avoid piling a
    // second error on top.
    let Some(own_source) = deploy_cfg
        .topology
        .values()
        .flat_map(|dev| dev.machines.iter())
        .find(|(name, _)| name.as_str() == resolved_name)
        .map(|(_, cfg)| cfg.source.clone())
    else {
        return Ok(());
    };

    let deploy_dir = deploy_path.parent().unwrap_or_else(|| Path::new("."));
    let Ok(own_canonical) = deploy_dir.join(&own_source).canonicalize() else {
        return Ok(());
    };

    // Same regex vocabulary as `collect_scxml_remote_peers`: tag open,
    // src attr, type attr. The local-shape filter here is the negation
    // of the mesh filter (`starts_with('#')` is the mesh shape).
    let invoke_tag_re = regex::Regex::new(r##"<invoke\b[^>]*>"##).expect("valid regex");
    let src_attr_re = regex::Regex::new(r##"\bsrc="([^"]*)""##).expect("valid regex");
    let type_attr_re = regex::Regex::new(r##"\btype="([^"]*)""##).expect("valid regex");

    for device in deploy_cfg.topology.values() {
        for (peer_name, peer_cfg) in &device.machines {
            if peer_name.as_str() == resolved_name {
                continue;
            }
            let peer_scxml_path = deploy_dir.join(&peer_cfg.source);
            let content = match std::fs::read_to_string(&peer_scxml_path) {
                Ok(c) => c,
                Err(_) => continue,
            };
            // Sibling SCXML's own directory — the URI base for any
            // relative `src=` it carries (§scxml-6.4.1).
            let sibling_dir = peer_scxml_path.parent().unwrap_or_else(|| Path::new("."));
            for tag in invoke_tag_re.find_iter(&content) {
                let tag_text = tag.as_str();
                let type_ok = match type_attr_re.captures(tag_text) {
                    Some(c) => {
                        let t = &c[1];
                        t.is_empty() || t == "scxml" || t == "http://www.w3.org/TR/scxml/"
                    }
                    None => true,
                };
                if !type_ok {
                    continue;
                }
                let Some(src_cap) = src_attr_re.captures(tag_text) else {
                    continue;
                };
                let src_raw = &src_cap[1];
                if src_raw.is_empty() || src_raw.starts_with('#') {
                    continue;
                }
                let candidate = sibling_dir.join(src_raw);
                let Ok(candidate_canonical) = candidate.canonicalize() else {
                    continue;
                };
                if candidate_canonical == own_canonical {
                    // Downcast the peer-binding vec to plain names — the
                    // diagnostic only shows the peer names as context
                    // hints ("invoked by foo, bar as a mesh peer") and
                    // does not need the transport detail.
                    let inbound_names: Vec<String> = model
                        .scxml_remote_inbound_peers
                        .iter()
                        .map(|p| p.name.clone())
                        .collect();
                    return Err(mesh::error::DeployError::ScxmlInvokeTargetConflict {
                        machine: resolved_name.to_string(),
                        inbound_peers: inbound_names,
                        local_invoker: peer_name.clone(),
                        local_src: src_raw.to_string(),
                    });
                }
            }
        }
    }
    Ok(())
}

/// SCE_MESH.md §9.6.2 wire-14/20 — build the two peer lists the transport
/// router needs to provision `ScxmlInvokeChannel` pairs:
///
/// * `scxml_remote_outbound_peers` — peer machines this machine invokes
///   (harvested directly from the just-classified `remote_mesh_target` field
///   on each local `Invoke::Scxml`).
/// * `scxml_remote_inbound_peers` — peer machines whose SCXML invokes this
///   machine as `<invoke type="scxml" src="#<this>">`. Requires a scan of
///   every sibling deploy.yaml machine's SCXML source. We use a focused
///   regex over the file text rather than a full reparse: the contract is
///   narrow (find `<invoke type="scxml" src="#<name>">`), the scan runs once
///   per per-partition codegen invocation, and the same source files go
///   through the full parser independently when their own codegen runs, so
///   malformed XML surfaces there with full diagnostics. A scan error
///   (missing file, unreadable) is treated as "no invokes into this
///   machine" — the machine's codegen proceeds without an inbound channel,
///   which is the safe default (transport-absent local raise still fires
///   via the classifier's per-invoke `remote_mesh_target` on the caller
///   side).
fn collect_scxml_remote_peers(
    model: &mut SCXMLModel,
    deploy_cfg: &mesh::deploy::DeployConfig,
    resolved_name: &str,
    deploy_path: &Path,
) {
    // Outbound: walk the just-enriched local invokes. Each peer carries
    // the author-declared transport (or `None` for same-device implicit
    // shm) resolved by `classify_remote_scxml_invokes` from the parent's
    // `bindings["#<peer>"]` entry. Two invokes in the same machine pointing
    // at the same peer cannot disagree on the transport — the deploy.yaml
    // binding is per-(parent, peer) pair, not per-invoke — so deduping on
    // `.name` via `BTreeMap` and keeping the first-observed transport is
    // correct. Sorted by name via `BTreeMap` iteration order.
    let mut outbound: std::collections::BTreeMap<String, Option<String>> =
        std::collections::BTreeMap::new();
    for state in model.states.values() {
        for invoke in &state.invokes {
            if let model::Invoke::Scxml(info) = invoke {
                if let Some(target) = &info.remote_mesh_target {
                    outbound
                        .entry(target.clone())
                        .or_insert_with(|| info.remote_mesh_transport.clone());
                }
            }
        }
    }
    model.scxml_remote_outbound_peers = outbound
        .into_iter()
        .map(|(name, transport)| {
            // SCE_MESH.md §9.6 Session 2: custom_tcp peers need the peer's
            // own `transports.custom_tcp.listen` so the parent's per-peer
            // `CustomTcp::Client` ctor has a connect endpoint. Other
            // transports (including shm/None) carry `None` — the template
            // only consults `connect_endpoint` inside the custom_tcp arm.
            let connect_endpoint = mesh::transport::custom_tcp::resolve_connect_endpoint(
                deploy_cfg,
                transport.as_deref(),
                &name,
            );
            model::ScxmlRemotePeerBinding {
                name,
                transport,
                connect_endpoint,
            }
        })
        .collect();

    // Inbound: scan every sibling machine's SCXML for
    // `<invoke type="scxml" src="#<this>">`. The deploy.yaml `source:` field
    // is relative to the deploy.yaml's own directory, per the existing
    // machine-source resolution convention. Each inbound entry records
    // the peer-side transport for `#<this>` — the peer's
    // `bindings["#<this>"].transport` is the authoritative value because
    // the peer is the parent in the wire-14 sense, and deploy.yaml places
    // the transport on the sender's binding (§9.6 L1393).
    let deploy_dir = deploy_path.parent().unwrap_or_else(|| Path::new("."));
    let self_marker = format!("#{resolved_name}");
    let mut inbound: std::collections::BTreeMap<String, Option<String>> =
        std::collections::BTreeMap::new();

    // Regex matches a whole `<invoke ...>` / `<invoke .../>` open tag as one
    // capture, then the same pattern's `src="#<name>"` capture locates the
    // target machine name. We filter by checking the tag's `type=`
    // attribute: absent, empty, "scxml", or the W3C default URI all pass;
    // anything else (sce:mesh-rpc, custom URIs) is excluded. One regex
    // traversal per file; no full XML parse.
    let _ = self_marker; // reserved for future diagnostics; fast-scan below handles the inclusion check
    let invoke_tag_re = regex::Regex::new(r##"<invoke\b[^>]*>"##).expect("valid regex");
    let src_attr_re =
        regex::Regex::new(r##"\bsrc="#([A-Za-z_][A-Za-z0-9_]*)""##).expect("valid regex");
    let type_attr_re = regex::Regex::new(r##"\btype="([^"]*)""##).expect("valid regex");

    let self_binding_key = format!("#{resolved_name}");

    for device in deploy_cfg.topology.values() {
        for (peer_name, peer_cfg) in &device.machines {
            if peer_name.as_str() == resolved_name {
                continue; // skip self — outbound list already covers it
            }
            let peer_scxml_path = deploy_dir.join(&peer_cfg.source);
            let content = match std::fs::read_to_string(&peer_scxml_path) {
                Ok(c) => c,
                Err(_) => continue, // unreadable — fail-silent (see header)
            };
            for tag in invoke_tag_re.find_iter(&content) {
                let tag_text = tag.as_str();
                let type_ok = match type_attr_re.captures(tag_text) {
                    Some(c) => {
                        let t = &c[1];
                        t.is_empty() || t == "scxml" || t == "http://www.w3.org/TR/scxml/"
                    }
                    None => true, // no type attr — W3C default "scxml"
                };
                if !type_ok {
                    continue;
                }
                if let Some(cap) = src_attr_re.captures(tag_text) {
                    if &cap[1] == resolved_name {
                        // Read the peer-side transport declaration from
                        // the peer machine's `bindings["#<this>"]` entry;
                        // `None` means the peer did not declare an
                        // explicit binding (same-device implicit shm).
                        let peer_transport = peer_cfg
                            .bindings
                            .get(self_binding_key.as_str())
                            .map(|b| b.transport.clone());
                        inbound.entry(peer_name.clone()).or_insert(peer_transport);
                    }
                }
            }
        }
    }

    // SCE_MESH.md §9.6.6 rule 3 — synth-side inbound resolution.
    // The parser rewrites inline `<content>` invokes to
    // `src="#<synth>"` in the **in-memory** model only; the parent's
    // on-disk SCXML still carries inline `<content>` with no `src=`
    // attribute, so the regex scan above cannot observe the rewrite
    // when `resolved_name` *is* the synth. Invert via the reserved
    // `__sce_synth_invoke__` infix: the parent is the prefix, declared
    // as a topology machine by the rule-3 override, and must be on a
    // different partition for the remote-mesh shape to apply (matching
    // the classifier's own cross-partition condition at
    // `classify_remote_scxml_invokes`).
    if let Some((parent_candidate, _)) =
        resolved_name.rsplit_once(crate::mesh::deploy::SYNTH_INVOKE_INFIX)
    {
        if !parent_candidate.is_empty() && deploy_cfg.device_for_machine(parent_candidate).is_some()
        {
            let self_partition = mesh::partitions::partition_for_machine(deploy_cfg, resolved_name);
            let parent_partition =
                mesh::partitions::partition_for_machine(deploy_cfg, parent_candidate);
            if self_partition != parent_partition {
                // Synth peers look up the parent's binding for the synth
                // machine's name — symmetric to the named-peer path above.
                let parent_transport = deploy_cfg
                    .device_for_machine(parent_candidate)
                    .and_then(|d| d.machines.get(parent_candidate))
                    .and_then(|m| m.bindings.get(self_binding_key.as_str()))
                    .map(|b| b.transport.clone());
                inbound
                    .entry(parent_candidate.to_string())
                    .or_insert(parent_transport);
            }
        }
    }

    model.scxml_remote_inbound_peers = inbound
        .into_iter()
        .map(|(name, transport)| {
            // SCE_MESH.md §9.6 Session 2: inbound peer's connect endpoint
            // is the *parent's* listen address — the worker's per-peer
            // `CustomTcp::Client` for `c2p_to_<peer>_` dials into it to
            // publish wire-15/16/18/20. Lookup is symmetric to the
            // outbound side (resolve_connect_endpoint consults the peer's
            // own device config).
            let connect_endpoint = mesh::transport::custom_tcp::resolve_connect_endpoint(
                deploy_cfg,
                transport.as_deref(),
                &name,
            );
            model::ScxmlRemotePeerBinding {
                name,
                transport,
                connect_endpoint,
            }
        })
        .collect();
    // SCE_MESH.md §9.6 — codegen-shape seam. When at least one sibling
    // invokes this machine remotely, the generated SM must be
    // default-constructible for `ChildSessionAdapter<Engine>` to own it.
    // The template swap reads this flag to decide whether to emit the
    // `ParentStateMachine`-templated shape (local-invoke only) or the
    // non-templated shape (`<send target="#_parent">` routes through
    // `performMeshSend`).
    model.is_remote_invoke_target = !model.scxml_remote_inbound_peers.is_empty();
    // Recompute the derived flag now that `is_remote_invoke_target` is
    // final. The analyzer set a provisional value earlier in the pipeline
    // (deploy.yaml was not yet consulted); this override is the
    // authoritative value for deploy-aware builds. Kept in sync with the
    // formula in `analyzer::analyze`.
    model.needs_parent_template = model.has_parent_communication && !model.is_remote_invoke_target;
}

/// SCE_MESH.md §9.6 — mark each `Invoke::Scxml` whose `src` is `#<name>`
/// referencing a **distinct** mesh machine declared in `deploy.yaml` as a
/// remote-mesh invoke. Sets
/// [`model::ScxmlInvokeInfo::remote_mesh_target`]; consumed by C++ codegen
/// to emit the §10.7.1 `SESSION_F_NOT_IMPLEMENTED` raise until the Session
/// F wire runtime (patterns 14-20 per §9.6.2) lands.
///
/// Left `None` for:
/// - Local W3C invokes with `src="file:..."` or `src="<relative>.scxml"`
///   (external-file form; remain on the local child-session path).
/// - Inline `<content>` invokes synthesised by the parser into
///   `src="#<parent>__sce_synth_invoke__<id>"` (§9.6.6 rules 1+2) when
///   the synth machine is not registered in `deploy.yaml` — these flow
///   through the local child-session path, matching W3C semantics for
///   non-mesh builds and for mesh builds whose author did not opt into
///   distribution for the synth machine (§9.6.6 rule 3 default:
///   "same partition as parent").
/// - Self-references (`#<own machine>`) — these would always fail at
///   build time, but classification here is defensive against author typos.
/// - Unknown `#<name>` that is not a deploy-declared machine — the build
///   remains a local W3C invoke and the existing "child SCXML not found"
///   path reports it.
/// - Targets whose `deploy.yaml` partition matches the parent's
///   partition — per §9.6.6 rule 3 the synthesised child defaults to the
///   parent's partition; cross-partition placement is what turns the
///   invoke into a remote-mesh invoke.
fn classify_remote_scxml_invokes(
    model: &mut SCXMLModel,
    deploy_cfg: &mesh::deploy::DeployConfig,
    resolved_name: &str,
) {
    let mut mutated = false;
    let parent_partition = mesh::partitions::partition_for_machine(deploy_cfg, resolved_name);
    // SCE_MESH.md §9.6 L1393 — the parent's own `bindings["#<peer>"]`
    // entry is the single source of truth for "which transport when this
    // machine addresses #peer" for both `<send>` and `<invoke>` axes.
    // Cross-device peers require an entry here; same-device peers take
    // the implicit shm fallback (today's only wired codegen path).
    let parent_bindings = deploy_cfg
        .device_for_machine(resolved_name)
        .and_then(|d| d.machines.get(resolved_name))
        .map(|m| &m.bindings);
    for state in model.states.values_mut() {
        for invoke in state.invokes.iter_mut() {
            if let model::Invoke::Scxml(info) = invoke {
                let Some(target) = info.src.strip_prefix('#') else {
                    continue;
                };
                if target == resolved_name {
                    continue;
                }
                if deploy_cfg.device_for_machine(target).is_none() {
                    continue;
                }
                // §9.6.6 rule 3 — same-partition invokes run in one OS
                // process and therefore take the local child-session
                // path; only cross-partition invokes cross a mesh wire.
                let target_partition = mesh::partitions::partition_for_machine(deploy_cfg, target);
                if target_partition != parent_partition {
                    info.remote_mesh_target = Some(target.to_string());
                    // SCE_MESH.md §9.6 L1393 — record the declared transport
                    // for this peer so `validate_scxml_invoke_transport`
                    // can reject cross-device declarations that name an
                    // incapable or not-yet-wired transport. `None` here
                    // means "author declared no binding for this peer";
                    // the validator separates that case from "declared
                    // but transport is shm/local" for better diagnostics.
                    // `TargetId` implements `Borrow<str>`, so the `HashMap`
                    // lookup keys on the raw `"#<target>"` literal directly.
                    let peer_key = format!("#{target}");
                    info.remote_mesh_transport = parent_bindings
                        .and_then(|b| b.get(peer_key.as_str()))
                        .map(|binding| binding.transport.clone());
                    mutated = true;
                }
            }
        }
    }
    // [`SCXMLModel.invokes`] is a flat template-visible view built by
    // [`SCXMLModel::refresh_invokes_view`] during parsing. Refresh it
    // after classification so C++ class-level templates (`model.invokes
    // | scxml`) see the same `remote_mesh_target` the per-state
    // entry-action templates (`state.invokes | scxml`) do — otherwise
    // the class emits local-child-session machinery (child_* member,
    // pending-invoke queue) against a remote invoke whose `src` still
    // carries the `#<peer>` prefix, producing invalid C++ identifiers
    // like `SCE::Generated::#worker`.
    if mutated {
        model.refresh_invokes_view();
    }
}

pub fn inject_server_model_mutations(
    model: &mut SCXMLModel,
    deploy_path: &Path,
) -> Result<Vec<String>, mesh::error::MeshError> {
    let deploy_cfg = mesh::deploy::parse_deploy(deploy_path)?;

    let effective_machine_name = if deploy_cfg.device_for_machine(&model.name).is_some() {
        model.name.clone()
    } else {
        deploy_cfg
            .find_machine_name_by_source(&model.name)
            .unwrap_or_else(|| model.name.clone())
    };

    let server_config = deploy_cfg
        .device_for_machine(&effective_machine_name)
        .and_then(|d| d.machines.get(&effective_machine_name))
        .and_then(|m| m.server.as_ref());

    if server_config.is_none() {
        return Ok(vec![]);
    }

    let server_pairs = mesh::topology::detect_server_pairs(model);
    let field_access_pairs = mesh::topology::detect_server_field_access_pairs(model);
    // SCE_MESH.md §8.1: spontaneous eventgroup notifications declared in
    // deploy.yaml `server.events` with `event_group:` (SOME/IP) — these
    // raises have no in-SCXML request-pair sibling, so they must be read
    // from the deploy config directly. Omitting them here left the
    // injection set depending on whether some other pair (RPC, field get)
    // happened to share the same event name, which is exactly the shape
    // that accidentally masked the bug on the multi fixture while the
    // dedicated unsubscribe fixture exposed it.
    let eventgroup_events = server_config
        .map(mesh::topology::detect_server_eventgroup_events)
        .unwrap_or_default();
    if server_pairs.is_empty() && field_access_pairs.is_empty() && eventgroup_events.is_empty() {
        return Ok(vec![]);
    }

    let response_events: std::collections::HashSet<String> = server_pairs
        .iter()
        .map(|p| p.response_event.clone())
        .chain(field_access_pairs.iter().map(|p| p.response_event.clone()))
        .chain(eventgroup_events.iter().map(|eg| eg.event.clone()))
        .collect();
    Ok(mesh::topology::inject_server_response_sends(
        model,
        &response_events,
    ))
}

/// Watching-zenoh RFC §5.J.2 + §5.L (Q-RustNoStd-7 (a), C3 Atomic
/// B-γ1): apply the deploy.yaml
/// `machines.<m>.scheduler.default_event_queue_capacity` fallback
/// to a model whose per-instance `<scxml sce:capacity="N">` is
/// absent.
///
/// Resolution rule (spec line 1993):
///   - per-instance attribute wins (`model.event_queue_capacity`
///     already set by parser ⇒ this function is a no-op),
///   - fallback to deploy.yaml `default_event_queue_capacity` when
///     attribute is absent,
///   - both absent ⇒ remains `None` (B-γ2's no_std codegen path
///     will surface the missing-capacity diagnostic when the
///     heapless adoption lands; B-γ1 tolerates None because the
///     std codegen path does not consume the value yet).
///
/// Machine-name resolution mirrors
/// [`inject_server_model_mutations`]: try `model.name` first, then
/// fall back to the deploy.yaml's `source:` reverse-lookup. Silent
/// no-op when neither the machine nor the scheduler block resolves.
pub fn populate_event_queue_capacity_from_deploy(
    model: &mut model::SCXMLModel,
    deploy_path: &Path,
) -> Result<(), mesh::error::MeshError> {
    if model.event_queue_capacity.is_some() {
        return Ok(());
    }
    let deploy_cfg = mesh::deploy::parse_deploy(deploy_path)?;

    let effective_machine_name = if deploy_cfg.device_for_machine(&model.name).is_some() {
        model.name.clone()
    } else {
        deploy_cfg
            .find_machine_name_by_source(&model.name)
            .unwrap_or_else(|| model.name.clone())
    };

    let capacity = deploy_cfg
        .device_for_machine(&effective_machine_name)
        .and_then(|d| d.machines.get(&effective_machine_name))
        .and_then(|m| m.scheduler.as_ref())
        .and_then(|s| s.default_event_queue_capacity);

    if let Some(n) = capacity {
        model.event_queue_capacity = Some(n);
    }
    Ok(())
}

/// Returns `Ok(MeshResult)` with generated files and all warnings,
/// or `Err(MeshError)` on hard failure.
pub fn compile_mesh_transport(
    model: &mut SCXMLModel,
    deploy_path: &Path,
    language: generator::Language,
) -> Result<MeshResult, mesh::error::MeshError> {
    // Stage 1: deploy.yaml parsing (typed session config validated by serde)
    let deploy_cfg = mesh::deploy::parse_deploy(deploy_path)?;

    // Stage 1b: resolve external infrastructure config (vsomeip.json) —
    // produces a typed `ExternalResolution` map consumed by topology. The
    // deploy config itself is treated read-only from here on.
    let deploy_dir = deploy_path.parent().unwrap_or(Path::new("."));

    // SCE_MESH.md §14 rules 1, 2, 11 + §16.3/§16.4 distributability —
    // resolve the partition plan against each partition-listed
    // machine's `<parallel>`/`<invoke>` inventory, then run the
    // R1-R4 analyzer (and §16.4 auto-merge under `distributability:
    // permissive`). Errors wrap `DeployError`; a no-op when
    // `partitions:` is absent. The resolved partition map itself is
    // consumed by `inject_partition_context_for` (which holds the
    // per-partition codegen state and re-runs the resolver on its
    // own entry); the notice vectors below surface to the author
    // through [`MeshResult`].
    let resolved_deploy = mesh::resolve_deploy_config(deploy_dir, &deploy_cfg)?;
    let distributability_merge_notices = resolved_deploy.plan.merge_notices.clone();
    let distributability_snapshot_notices = resolved_deploy.plan.snapshot_notices.clone();
    drop(resolved_deploy);

    let external_resolution = mesh::external::resolve_external_bindings(&deploy_cfg, deploy_dir)?;

    // Stage 1c: auto-symmetry injection (SCE_MESH.md §13). Must run
    // BEFORE collect_send_summary so synthesized unsubscribe sends are
    // visible to pattern detection and event coverage validation.
    let (auto_subscriptions, subscription_lint_notices) =
        mesh::topology::inject_auto_subscriptions(model);

    // Stage 1d: Server role detection (SCE_MESH.md §13 Session E).
    // Detect RPC pairs, inject synthetic sends for response events, and
    // resolve the server binding from deploy.yaml. Must run BEFORE
    // collect_send_summary so injected sends are visible to the pipeline,
    // and AFTER external resolution (stage 1b) because SOME/IP server
    // bindings need vsomeip.json IDs.
    //
    // Machine lookup uses model.name first (file stem), falling back to
    // source filename matching when the deploy.yaml key uses the SCXML
    // name attribute (e.g., deploy.yaml has "motor:" but the file stem
    // is "motor_someip_multi").
    // Machine lookup: try model.name (file stem) first, fall back to
    // source filename when the deploy.yaml key uses the SCXML name
    // attribute (e.g., deploy.yaml "motor:" but file stem "motor_someip_multi").
    let effective_machine_name = if deploy_cfg.device_for_machine(&model.name).is_some() {
        model.name.clone()
    } else {
        deploy_cfg
            .find_machine_name_by_source(&model.name)
            .unwrap_or_else(|| model.name.clone())
    };

    let server_config = deploy_cfg
        .device_for_machine(&effective_machine_name)
        .and_then(|d| d.machines.get(&effective_machine_name))
        .and_then(|m| m.server.as_ref());

    let server_pairs = mesh::topology::detect_server_pairs(model);
    let server_fire_forget_events = mesh::topology::detect_server_fire_forget_events(model);
    let server_field_access_pairs = mesh::topology::detect_server_field_access_pairs(model);
    // Eventgroup events are deploy.yaml-driven (server.events with
    // event_group: binding). Detection requires the server config.
    let server_eventgroup_events = server_config
        .map(mesh::topology::detect_server_eventgroup_events)
        .unwrap_or_default();
    // Build the server response event set once — used for both injection
    // and self-send exemption. Covers RPC responses, FieldAccess notifies,
    // and eventgroup notification events. FireForget is one-way (no raise),
    // so its events are excluded.
    let server_response_events: std::collections::HashSet<String> = server_pairs
        .iter()
        .map(|p| p.response_event.clone())
        .chain(
            server_field_access_pairs
                .iter()
                .map(|p| p.response_event.clone()),
        )
        .chain(server_eventgroup_events.iter().map(|eg| eg.event.clone()))
        .collect();

    let server_binding = if let Some(srv_cfg) = server_config {
        if !server_pairs.is_empty()
            || !server_fire_forget_events.is_empty()
            || !server_field_access_pairs.is_empty()
            || !server_eventgroup_events.is_empty()
        {
            // Inject synthetic <send> alongside each <raise> of a server
            // response/notification so the mesh send callback fires for
            // server response/publish routing.
            mesh::topology::inject_server_response_sends(model, &server_response_events);
            Some(mesh::topology::resolve_server_binding(
                srv_cfg,
                &server_pairs,
                &server_fire_forget_events,
                &server_field_access_pairs,
                &server_eventgroup_events,
                &effective_machine_name,
                &external_resolution,
            )?)
        } else {
            None
        }
    } else {
        None
    };

    // Stage 2: single-pass send action collection
    let mut summary = mesh::topology::collect_send_summary(model);

    // Stage 2a: dynamic target warnings (from summary)
    let dynamic_target_warnings = summary.dynamic_warnings.clone();

    // Stage 2a.1: exempt server response self-sends from topology resolution
    // and event coverage validation. These are injected synthetic sends that
    // target the machine's own SCXML name — they are intercepted by
    // handleServerResponse / publishEventgroupNotify before route_send and
    // never reach a transport binding. Without this exemption, the pipeline
    // would fail with "unresolved target" or "uncovered event".
    //
    // Reuses `server_response_events` built above (single-source set for
    // both injection and exemption).
    if server_binding.is_some() && !server_response_events.is_empty() {
        let scxml_name = if model.scxml_name.is_empty() {
            &model.name
        } else {
            &model.scxml_name
        };
        let self_target_str = format!("#{scxml_name}");
        if let Some(self_tid) = mesh::target::TargetId::new(&self_target_str) {
            summary.targets.remove(&self_tid);
            summary
                .target_events
                .retain(|(t, e)| !(t == &self_tid && server_response_events.contains(e)));
            summary
                .actions
                .retain(|a| !(a.target == self_tid && server_response_events.contains(&a.event)));
        }
    }

    // Machine-lifetime subscriptions from deploy.yaml (SCE_MESH.md §13).
    // Looked up once and consumed by both `build_resolved_targets` (to
    // synthesise the implicit target per source) and the codegen call
    // below (to emit the init-time subscribe blocks). Using
    // `effective_machine_name` — the deploy.yaml key — because it may
    // differ from the SCXML `name` attribute / file stem.
    let device = deploy_cfg.device_for_machine(&effective_machine_name);
    let machine_subscriptions: &[mesh::deploy::SubscriptionConfig] = device
        .and_then(|d| d.machines.get(&effective_machine_name))
        .map_or(&[], |m| m.subscriptions.as_slice());

    // Stage 2b: resolve static targets against deploy.yaml bindings,
    // attach per-event SOME/IP IDs, and validate per-event field presence
    // in a single pipeline — callers cannot observe the half-built state
    // between resolution and attach.
    let resolution = mesh::topology::build_resolved_targets(
        &mesh::topology::TargetContributions {
            send_summary: &summary,
            subscriptions: machine_subscriptions,
        },
        &deploy_cfg,
        &effective_machine_name,
        &external_resolution,
    )?;
    let resolved = resolution.targets;
    let deadline_override_notices = resolution.deadline_overrides;

    // A device-level `transports.custom_tcp.listen:` requires this machine
    // to host a TCP server even when it has no client targets, no server
    // binding, and no machine-lifetime subscriptions — pure-receiver
    // machines on a device that listens still need a transport.h to bind
    // the listen socket and dispatch inbound envelopes (SCE_MESH.md §16.8.3).
    // The same predicate gates `transport_types.insert("custom_tcp")` in
    // codegen.rs; `CustomTcpTransportConfig::hosts_server` is the SSoT.
    let has_custom_tcp_listen = device
        .and_then(|d| d.transports.custom_tcp.as_ref())
        .is_some_and(mesh::deploy::CustomTcpTransportConfig::hosts_server);

    // SCE_MESH.md §16.5 wire-21: a partition may produce no
    // conventional `<send target="#X">` traffic and still need a
    // transport.h that opens inter-partition shm channels for
    // ParallelRegionDone forwarding. Detected purely from the wire-21
    // routing fields populated by `inject_partition_context_for`; an
    // unpartitioned codegen always sees both empty and falls through
    // to the legacy early-return.
    let has_wire21_routing = !model.partition_wire21_outbound_routes.is_empty()
        || !model.partition_wire21_inbound_sources.is_empty();

    // SCE_MESH.md §9.6.2 wire 14/20 — a machine that issues a remote SCXML
    // invoke (outbound peer) or is named as a remote invoke target by any
    // sibling (inbound peer) needs `<machine>_transport.h` to open the
    // ScxmlInvokeChannel pairs, even when no conventional bindings/
    // subscriptions/server/custom_tcp-listen/wire-21-routing exists. Same
    // shape as the wire-21 early-return guard above.
    let has_scxml_remote_wire = !model.scxml_remote_outbound_peers.is_empty()
        || !model.scxml_remote_inbound_peers.is_empty();

    if resolved.is_empty()
        && server_binding.is_none()
        && !has_custom_tcp_listen
        && !has_wire21_routing
        && !has_scxml_remote_wire
    {
        let _ = external_resolution; // no bindings → no resolved IDs to consume
        return Ok(MeshResult {
            output: generator::GeneratedOutput::default(),
            dynamic_target_warnings,
            deadline_override_notices,
            auto_subscriptions,
            subscription_lint_notices,
            distributability_merge_notices,
            distributability_snapshot_notices,
        });
    }

    // Stage 2c: pattern capability validation — architectural check.
    // A transport that lacks a required capability is a design error.
    if !resolved.is_empty() {
        let pattern_violations =
            mesh::topology::validate_pattern_capability(&summary, &deploy_cfg, &model.name);
        if !pattern_violations.is_empty() {
            return Err(mesh::error::TopologyError::PatternCapabilityViolation {
                sender: model.name.clone(),
                violations: pattern_violations,
            }
            .into());
        }

        // Stage 2d: event coverage — implementation check.
        let receiver_models =
            mesh::topology::load_receiver_models(&resolved, &deploy_cfg, deploy_dir, &model.name)?;
        let uncovered = mesh::topology::check_sender_event_coverage(
            &model.name,
            &summary,
            &receiver_models,
            &deploy_cfg,
        );
        if !uncovered.is_empty() {
            return Err(mesh::error::TopologyError::UncoveredEvents {
                sender: model.name.clone(),
                findings: uncovered,
            }
            .into());
        }
    }

    // Stage 3: transport codegen. Device-shared transport configs are read
    // directly from the device's `transports:` block; no merging/validation
    // pass is needed because the schema makes shared config structurally
    // singular (one entry per transport type per device).
    //
    // Use effective_machine_name for device lookup — model.name is the file
    // stem (e.g. "motor_zenoh_multi") which may differ from the deploy.yaml
    // key (e.g. "motor") when the SCXML name attribute is used.
    // `device` + `machine_subscriptions` are hoisted above the Stage 2b
    // call so the same slice feeds synthesis and codegen.
    let zenoh_session = device.and_then(|d| d.transports.zenoh.as_ref());
    let someip_config = device.and_then(|d| d.transports.someip.as_ref());
    let custom_tcp_config = device.and_then(|d| d.transports.custom_tcp.as_ref());
    // SCE_MESH.md §10.6.1: per-machine ordering buffer timings. The
    // `resolved_ordering_timings` helper supplies the absent-section
    // defaults from a single source (deploy::DEFAULT_*), so a
    // pure-receiver machine that never enters this branch and a fully
    // configured one share the same constants.
    let machine_ordering = device
        .and_then(|d| d.machines.get(&effective_machine_name))
        .map_or_else(
            mesh::deploy::OrderingTimings::default_const,
            mesh::deploy::MachineConfig::resolved_ordering_timings,
        );
    // SCE Mesh §16.7 row 8 (PEER_PARTITIONED): opt-in Zenoh liveliness
    // tokens. Absent section on the machine ⇒ `None`, and the template
    // emits zero liveliness code for that machine. `LivelinessConfig`
    // is `Copy`, so we flatten with `copied()` rather than holding a
    // reference across the codegen boundary.
    let machine_liveliness = device
        .and_then(|d| d.machines.get(&effective_machine_name))
        .and_then(|m| m.liveliness);
    // SCE Mesh §10.10: opt-in per-machine outbound buffer. Absent
    // section ⇒ `None`, and the template emits zero buffer code for
    // that machine. `OutboundBufferConfig` is `Copy`, so we flatten
    // with `copied()`-equivalent (`and_then(|m| m.outbound_buffer)`
    // already produces an owned value).
    let machine_outbound_buffer = device
        .and_then(|d| d.machines.get(&effective_machine_name))
        .and_then(|m| m.outbound_buffer);
    let template_base = find_template_base();
    // SCE_MESH.md §16.5 wire-21 partition routes — threaded through to
    // codegen so the per-partition shm channel constants and members
    // can be emitted alongside the conventional per-`<send>` transport
    // wiring. Empty maps are a no-op in the template.
    let partition_self_name = model.partition_self_name.clone();
    let partition_wire21_outbound = model.partition_wire21_outbound_routes.clone();
    let partition_wire21_inbound = model.partition_wire21_inbound_sources.clone();
    let scxml_remote_outbound_peers = model.scxml_remote_outbound_peers.clone();
    let scxml_remote_inbound_peers = model.scxml_remote_inbound_peers.clone();
    // SCE Mesh RFC F.X-1: compute the deploy-wide §9.6 SOMEIP scxml-invoke
    // service ID map once. The validator already ran inside
    // `parse_deploy_str` above, so any overflow / pin / collision error
    // would have surfaced there; reaching this point means the deploy is
    // assignable and the function returns Ok. Codegen reads self's ID and
    // each peer's ID from this map to emit per-target constants instead of
    // `serviceIdForMachine(...)` constexpr calls.
    let someip_invoke_service_ids = mesh::deploy::assign_someip_invoke_service_ids(&deploy_cfg)?;
    // RFC F.X-3: §16.4 region-liveness service IDs live in the disjoint
    // [0x8180, 0x81FF] sub-range of the SCE-reserved space. The deploy
    // validator already ran inside `parse_deploy_str` above, so any
    // overflow / pin / collision error would have surfaced there;
    // reaching this point means the deploy is assignable. Codegen reads
    // self's partition-keyed liveness ID and each sibling partition's ID
    // from this map to emit per-target constants
    // (`SCE_LIVENESS_SERVICE_SELF` and
    // `SCE_LIVENESS_SERVICE_PEER_<sibling_partition>`).
    let someip_liveness_service_ids =
        mesh::deploy::assign_someip_liveness_service_ids(&deploy_cfg)?;
    // RFC F.X-4: §16.7 row 8 SOME/IP machine-level liveness service IDs
    // live in the disjoint [0x8280, 0x82FF] sub-range. The deploy
    // validator already ran inside `parse_deploy_str` above, so any
    // overflow / pin / collision error would have surfaced there;
    // reaching this point means the deploy is assignable. Codegen reads
    // self's machine-keyed liveness ID and each peer machine's ID from
    // this map to emit per-target constants
    // (`SCE_MACHINE_LIVENESS_SERVICE_SELF` and
    // `SCE_MACHINE_LIVENESS_SERVICE_PEER_<peer_machine>`).
    let someip_machine_liveness_service_ids =
        mesh::deploy::assign_someip_machine_liveness_service_ids(&deploy_cfg)?;
    let output = mesh::codegen::generate_mesh(
        mesh::codegen::MeshCodegenInputs {
            machine_name: &model.name,
            targets: &resolved,
            server: server_binding.as_ref(),
            zenoh_session,
            someip_config,
            custom_tcp_config,
            subscriptions: machine_subscriptions,
            machine_ordering,
            machine_liveliness,
            machine_outbound_buffer,
            partition_self_name: partition_self_name.as_deref(),
            partition_wire21_outbound: &partition_wire21_outbound,
            partition_wire21_inbound: &partition_wire21_inbound,
            scxml_remote_outbound_peers: &scxml_remote_outbound_peers,
            scxml_remote_inbound_peers: &scxml_remote_inbound_peers,
            someip_invoke_service_ids: &someip_invoke_service_ids,
            someip_liveness_service_ids: &someip_liveness_service_ids,
            someip_machine_liveness_service_ids: &someip_machine_liveness_service_ids,
            source_location: model.source_location.as_ref(),
            template_base: &template_base,
        },
        language,
    )?;
    Ok(MeshResult {
        output,
        dynamic_target_warnings,
        deadline_override_notices,
        auto_subscriptions,
        subscription_lint_notices,
        distributability_merge_notices,
        distributability_snapshot_notices,
    })
}

/// Validate event coverage across multiple SCXML models in a deployment.
///
/// Multi-model API: takes all models referenced by deploy.yaml and
/// cross-references send events against receiver transitions. Separate
/// from `compile_mesh_transport` because it requires all models in the
/// topology, while codegen operates per-model.
///
/// Returns warnings (not errors) for events that have no matching handler
/// in the target machine.
pub fn validate_mesh_event_coverage(
    models: &[(&str, &SCXMLModel)],
    deploy_path: &Path,
) -> Result<Vec<mesh::topology::EventCoverageWarning>, mesh::error::MeshError> {
    let deploy_cfg = mesh::deploy::parse_deploy(deploy_path)?;
    Ok(mesh::topology::validate_event_coverage(models, &deploy_cfg))
}

/// Which processing pipeline an input document must be routed through.
///
/// The discriminant is the `sce:kind` attribute on the `<scxml>` root:
///
/// | `sce:kind` value                                | [`Pipeline`]      |
/// |-------------------------------------------------|-------------------|
/// | absent                                          | [`Self::Scxml`]   |
/// | `"statechart"`                                  | [`Self::Scxml`]   |
/// | any other XSD-declared value (`transform` etc.) | [`Self::Forge`]   |
/// | any string *not* in the XSD enumeration         | [`Self::Forge`]   |
///
/// The last row is the contract-critical case. An author who wrote
/// `sce:kind="bogus"` intended a forge document; reporting that failure
/// through the SCXML parser would mis-label the diagnostic as a plain
/// XML parse error when the truth is an `sce:kind` violation caught by
/// the Forge XSD. Routing such documents to [`Self::Forge`] lets the
/// forge pipeline emit the honest stage (`xml/schema-validation` when
/// the bundled XSD is reachable, or `validation/unsupported-kind` when
/// the schemas were not vendored) — either of which is strictly more
/// actionable than a generic SCXML-parser error.
///
/// Documents whose XML is too malformed for `roxmltree` to reach the
/// root attribute list fall back to [`Self::Scxml`]: intent is not
/// knowable, and the SCXML parser's own XML-level diagnostic is the
/// least-wrong answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pipeline {
    /// Route through the SCXML parser (classic statechart compilation).
    Scxml,
    /// Route through the Forge pipeline (XSD + kind-specific validator).
    Forge,
}

/// Decide which pipeline should process `content`.
///
/// See [`Pipeline`] for the full routing table and the rationale behind
/// each case. This predicate is the single source of truth for routing
/// — the CLI dispatches on it, and any future embedding API must too.
/// Watching-zenoh RFC §5.J.2 (item C3): reject SCXML constructs
/// that are incompatible with the `sce-rust-runtime` no_std variant.
///
/// Caller invokes this only when `sce-codegen generate -l rust --no-std`
/// is in effect — the gate is `lang == Rust && no_std == true` at the
/// CLI surface. The function reads three model flags already
/// populated by the parser and analyzer:
///
/// - `model.needs_script_engine` (per-state executable content that
///   requires ECMAScript)
/// - `model.has_unresolved_external_script` (parse-time
///   `<script src=...>` the parser could not load)
/// - `model.needs_http_send` (any `<send type="BasicHTTPEventProcessor">`
///   with an http(s) target/targetexpr)
///
/// Single-diagnostic-per-call matches the C2-outbox precedent (one
/// rejection per pass; the next surfaces after the author repairs the
/// first). Axis order is **most-specific first** so the author repair
/// path names the offending construct directly:
///
/// 1. **fs-load** — `<data src="...">` (filesystem helpers gated to
///    `!no_std` in `helpers/datamodel_init.rs`).
/// 2. **invoke** — `<invoke>` (alloc-coupled `Arc`/`Mutex`/`HashMap`
///    in `helpers/invoke_processing.rs`).
/// 3. **script** — any ECMAScript cause the analyzer detects
///    (`needs_script_engine` is the broad catch-all that also flags
///    `<data expr>`, transition guards, send-expr, etc.).
/// 4. **http** — `<send type="BasicHTTPEventProcessor">` (specific
///    rejection on its own when the document is otherwise script-clean).
///
/// `document` is the SCXML basename; `locations` is a single
/// human-readable summary so downstream agents can dispatch on
/// `key_fragments` while authors get a readable message.
///
/// Returns `Ok(())` when no axis fires — note that today this does
/// **not** mean the generated Rust code will compile under `no_std`;
/// the runtime crate's `engine.rs` + remaining `helpers/` modules still
/// use std types. B-γ2c (this commit) closes the helper cfg-gates for
/// the three Q-Port-1/2/3 sites alongside the author-visible
/// `<data src>` and `<invoke>` rejections; the full compile-target gate
/// remains the responsibility of a later atomic.
pub fn validate_no_std_compatibility(
    model: &model::SCXMLModel,
    scxml_path: &std::path::Path,
) -> Result<(), forge::error::ForgeError> {
    use forge::error::GenerateError;

    let document = scxml_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    // C3 Atomic B-γ2c (watching-zenoh RFC §5.J.2 lines 1989-1994): the
    // no_std variant has zero alloc dependency. Filesystem-coupled
    // helpers in `sce-rust-runtime/src/helpers/datamodel_init.rs` are
    // gated to `!no_std`; reject `<data src="...">` up-front so the
    // generated crate never tries to call them.
    //
    // The script-engine analyzer also flags `<data src>` (it triggers
    // `needs_script_engine` via [`crate::script_engine_analyzer::collect_datamodel_causes`]),
    // so checking fs-load *before* script lets the author see the more
    // specific repair ("remove src or inline content") instead of the
    // catch-all script diagnostic.
    let fs_locations: Vec<String> = std::iter::empty::<(Option<String>, &str)>()
        .chain(model.variables.iter().filter_map(|v| {
            if v.src.is_empty() {
                None
            } else {
                Some((None, v.src.as_str()))
            }
        }))
        .chain(model.states.iter().flat_map(|(state_id, state)| {
            state.datamodel.iter().filter_map(move |v| {
                if v.src.is_empty() {
                    None
                } else {
                    Some((Some(state_id.clone()), v.src.as_str()))
                }
            })
        }))
        .map(|(state, src)| match state {
            Some(s) => format!("<data src=\"{}\"> in state '{}'", src, s),
            None => format!("<data src=\"{}\"> at document scope", src),
        })
        .collect();
    if !fs_locations.is_empty() {
        return Err(GenerateError::CodegenNoStdFsLoadNotSupported {
            document,
            locations: fs_locations.join("; "),
        }
        .into());
    }

    // C3 Atomic B-γ2c: invoke processing in
    // `sce-rust-runtime/src/helpers/invoke_processing.rs` is
    // whole-module gated to `!no_std` because `Arc<Mutex<Vec<…>>>` +
    // `HashMap` are alloc-coupled. `model.invokes` is the aggregated
    // flat view refreshed in `parser::refresh_invokes_view` at the end
    // of every parse, so a single non-empty check covers state-nested
    // invokes too. Checked *before* the script axis: child-invoke
    // metadata propagates `child_needs_script_engine = true` whenever
    // the child file is missing or itself script-using (parser.rs:3568
    // `parse_child_metadata`), so the broad script axis would
    // otherwise mask the invoke-specific repair.
    if !model.invokes.is_empty() {
        let n = model.invokes.len();
        let locations = if n == 1 {
            "1 <invoke> element".to_string()
        } else {
            format!("{} <invoke> elements", n)
        };
        return Err(GenerateError::CodegenNoStdInvokeNotSupported {
            document,
            locations,
        }
        .into());
    }

    if model.needs_script_engine || model.has_unresolved_external_script {
        let locations = if !model.global_scripts.is_empty() {
            format!(
                "{} <script> action(s) at document scope",
                model.global_scripts.len()
            )
        } else if model.has_unresolved_external_script {
            "<script src=...> with unresolved external reference".to_string()
        } else {
            "ECMAScript executable content (analyzer-detected)".to_string()
        };
        return Err(GenerateError::CodegenNoStdScriptNotSupported {
            document,
            locations,
        }
        .into());
    }

    if model.needs_http_send {
        return Err(GenerateError::CodegenNoStdHttpNotSupported {
            document,
            locations: "BasicHTTPEventProcessor <send> target/targetexpr (analyzer-detected)"
                .to_string(),
        }
        .into());
    }

    Ok(())
}

pub fn classify_document(content: &str) -> Pipeline {
    use forge::error::{ForgeError, ValidationError};
    match forge::parser::detect_kind(content) {
        // Explicit forge kind with a recognised value.
        Ok(Some(k)) if k != forge::model::ForgeKind::Statechart => Pipeline::Forge,
        // No `sce:kind` attribute, or `sce:kind="statechart"`.
        Ok(_) => Pipeline::Scxml,
        // `sce:kind` attribute is present but the value is outside the
        // XSD enumeration. Route to forge so the schema / validator
        // emits the authoritative stage — see `Pipeline` doc comment.
        // `Validation` carries a `Box<ValidationError>` (see [`ForgeError`]
        // docs); the guard dereferences once before the inner match.
        Err(ForgeError::Validation(boxed))
            if matches!(*boxed, ValidationError::UnsupportedKind(_)) =>
        {
            Pipeline::Forge
        }
        // XML was not parseable; intent unknowable — defer to SCXML.
        Err(_) => Pipeline::Scxml,
    }
}

/// Locate the base template directory (contains rust/, kotlin/, actions/).
pub fn find_template_base() -> std::path::PathBuf {
    if let Ok(dir) = std::env::var("SCE_TEMPLATE_DIR") {
        // If pointing to a language subdir, go up one level
        let p = Path::new(&dir);
        if p.join("state_machine.rs.jinja2").exists() || p.join("state_machine.kt.jinja2").exists()
        {
            return p.parent().unwrap_or(p).to_path_buf();
        }
        return p.to_path_buf();
    }
    // `CARGO_MANIFEST_DIR` is baked at compile time and may point at a
    // stale source tree on install targets. The `.exists()` check is
    // the guard: when the installed binary runs on a machine that does
    // not carry the original source tree, the candidate path is absent
    // and we fall through to the panic below. Silent-wrong output is
    // only possible if the exact source-tree path happens to exist on
    // the install target — an edge case the `SCE_TEMPLATE_DIR` override
    // above remains the authoritative escape hatch for.
    let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let candidate = crate_dir.join("../tools/codegen/templates");
    if candidate.exists() {
        return candidate;
    }
    let candidate = Path::new("tools/codegen/templates");
    if candidate.exists() {
        return candidate.to_path_buf();
    }
    panic!(
        "Cannot find Jinja2 templates. Set SCE_TEMPLATE_DIR to the installed \
         templates directory (e.g. /usr/local/share/sce/codegen/templates)."
    );
}

/// Locate the Rust Jinja2 template directory.
///
/// Delegates to `find_template_dir_for(Language::Rust)` for consistent behavior.
pub fn find_template_dir() -> std::path::PathBuf {
    find_template_dir_for(generator::Language::Rust)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The typed entry point surfaces `ValidationError::DynamicFeatures`
    /// as a structured `Located<ForgeError>` so Rust consumers can
    /// dispatch on the variant instead of parsing the human message.
    /// Pins the post-Task-D invariant that parser+analyzer failures
    /// travel the wire contract end-to-end for both WASM/JS (String
    /// shim) and in-process Rust callers (typed).
    #[test]
    fn typed_entry_exposes_structured_validation_error() {
        use forge::error::ForgeError;
        use scxml_semantic::{InitialStateScope, ScxmlSemanticError};
        // `initial="nope"` names a non-existent state — §wire-W5 D3
        // refit: this is a hard semantic violation, NOT a "dynamic
        // feature". Pre-W5 surface routed through
        // `ValidationError::DynamicFeatures`; W5 splits to
        // `ScxmlSemanticError::InitialStateUnknown` mapping to the
        // existing `validation/invalid-reference` wire code.
        let scxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="nope" name="typed_probe">
    <state id="s1"/>
</scxml>"##;
        let err = compile_from_string_typed(scxml, "typed_probe", &[])
            .expect_err("initial points at undeclared state must reject");
        assert!(
            matches!(
                err.error,
                ForgeError::Scxml(ref boxed)
                    if matches!(
                        **boxed,
                        ScxmlSemanticError::InitialStateUnknown {
                            ref state_id,
                            scope: InitialStateScope::DocumentRoot,
                            ..
                        } if state_id == "nope"
                    ),
            ),
            "expected ScxmlSemanticError::InitialStateUnknown(state_id=\"nope\", scope=DocumentRoot), \
             got: {:?}",
            err.error,
        );
    }

    // ── C11 backend foundation (RFC §5.J.1, M1) ─────────────────
    //
    // M2 (this commit) replaces the M1 InvalidConfig boundary with a
    // working emitter for the minimum vertical slice (test355 — flat,
    // datamodel-less, eventless). The M1 tests pinned that the boundary
    // *rejected* C11; the M2 tests pin that the same boundary now
    // *accepts* C11 and produces a `.h` + `.c` pair, so a future
    // regression that re-routes C11 through `InvalidConfig` (e.g. by
    // grouping it with Python in a fall-through arm) still trips a test.
    //
    // Sister tests for FromStr live in `generator::tests`.

    /// Forge cpp procedure codegen emits the bytes-typed cap-check
    /// guard around an `<assign location="X" expr="_event.data"/>` when
    /// `X` is a `<data sce:type="bytes" sce:max-size="N"/>` slot. This
    /// pins the bounded-bytes cap contract's
    /// cpp half: heap-backed runtime raises `error.execution` through
    /// the shared `run_procedure` loop instead of throwing or letting
    /// the assign silently overflow. Companion tests in
    /// `forge::validate` cover the static (parse-time) cap-consistency
    /// check; this test covers the runtime-emitted guard.
    #[test]
    fn forge_cpp_procedure_emits_bytes_cap_check() {
        let scxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="procedure" initial="req" version="1.0">
  <datamodel>
    <data id="seed" sce:type="bytes" sce:direction="internal" sce:max-size="32"/>
  </datamodel>
  <state id="req">
    <onentry><send sce:service="Probe" sce:response-max-size="32"/></onentry>
    <transition event="ok" target="done">
      <assign location="seed" expr="_event.data"/>
    </transition>
  </state>
  <final id="done"/>
</scxml>"##;
        let label = DocumentLabel {
            identifier: "bytes_probe",
            diagnostic_label: "bytes_probe.scxml",
        };
        let out = compile_forge_from_string(scxml, label, generator::Language::Cpp)
            .expect("forge cpp codegen must succeed for a bytes-bounded probe");
        assert_eq!(out.files.len(), 1);
        let (_, body) = &out.files[0];

        // Event enum must always include ErrorExecution so the cap-check
        // guard has a target — even when no fixture transition matches it.
        assert!(
            body.contains("ErrorExecution"),
            "Event::ErrorExecution must be emitted; full source:\n{body}",
        );

        // executeTransitionActions returns std::optional<Event> so an
        // assign-time raise can be routed back through processTransition.
        assert!(
            body.contains("std::optional<Event> executeTransitionActions"),
            "executeTransitionActions must return std::optional<Event>; full source:\n{body}",
        );

        // The bytes-typed assign must be wrapped with the cap-check
        // pattern: capture into temp, check size against the resolved
        // cap (32 from the explicit annotation), raise ErrorExecution
        // on overflow, otherwise std::move into the slot.
        assert!(
            body.contains("_scope_tmp"),
            "bytes-typed assign must use the temp-then-check shape; full source:\n{body}",
        );
        assert!(
            body.contains("if (_scope_tmp.size() > 32)"),
            "cap-check must use the resolved cap (32 from sce:max-size); full source:\n{body}",
        );
        assert!(
            body.contains("return Event::ErrorExecution"),
            "cap violation path must raise Event::ErrorExecution; full source:\n{body}",
        );
        assert!(
            body.contains("seed_ = std::move(_scope_tmp)"),
            "successful path must std::move into the slot; full source:\n{body}",
        );
    }

    /// Forge Rust procedure codegen mirrors cpp's contract (commit 3a)
    /// for the cap-check raise path: a bytes-typed assign wraps in a
    /// temp+check shape, returns `Some(Event::ErrorExecution)` on
    /// overflow, and the procedure runtime's `execute_transition_actions`
    /// signature returns `Option<Event>`. Companion to
    /// `forge_cpp_procedure_emits_bytes_cap_check`; together they pin
    /// the 1:1 cpp↔Rust lift of the bounded-bytes cap-check contract.
    #[test]
    fn forge_rust_procedure_emits_bytes_cap_check() {
        let scxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="procedure" initial="req" version="1.0">
  <datamodel>
    <data id="seed" sce:type="bytes" sce:direction="internal" sce:max-size="32"/>
  </datamodel>
  <state id="req">
    <onentry><send sce:service="Probe" sce:response-max-size="32"/></onentry>
    <transition event="ok" target="done">
      <assign location="seed" expr="_event.data"/>
    </transition>
  </state>
  <final id="done"/>
</scxml>"##;
        let label = DocumentLabel {
            identifier: "bytes_probe",
            diagnostic_label: "bytes_probe.scxml",
        };
        let out = compile_forge_from_string(scxml, label, generator::Language::Rust)
            .expect("forge rust codegen must succeed for a bytes-bounded probe");
        assert_eq!(out.files.len(), 1);
        let (_, body) = &out.files[0];

        // Always-emitted Event::ErrorExecution variant.
        assert!(
            body.contains("ErrorExecution"),
            "Event::ErrorExecution must be emitted in Rust enum; full source:\n{body}",
        );

        // execute_transition_actions returns Option<Event> (signature
        // mirror of cpp's std::optional<Event>).
        assert!(
            body.contains("fn execute_transition_actions(&mut self, source: State, tr_index: usize) -> Option<Event>"),
            "execute_transition_actions must return Option<Event>; full source:\n{body}",
        );

        // Bytes-typed assign uses the temp-then-check shape.
        assert!(
            body.contains("let _scope_tmp"),
            "bytes-typed assign must use temp shape; full source:\n{body}",
        );
        assert!(
            body.contains("if _scope_tmp.len() > 32"),
            "cap-check must use the resolved cap (32 from sce:max-size); full source:\n{body}",
        );
        assert!(
            body.contains("return Some(Event::ErrorExecution)"),
            "cap violation path must raise Event::ErrorExecution; full source:\n{body}",
        );
        assert!(
            body.contains("self.seed = _scope_tmp"),
            "successful path must move into the slot; full source:\n{body}",
        );
    }

    /// Watching-zenoh RFC §5.C B6-α: link kind happy path. A
    /// well-formed `<sce:kind="link">` document with udp class +
    /// framer ref + minimal events → Rust generator emits a
    /// `<Pascal><L: Link>` wrapper struct that routes RX/TX through
    /// the `sce-link-runtime::Link` trait. Asserts presence of the
    /// load-bearing tokens (struct decl + impl block + trait import +
    /// LINK_CLASS / FRAMER_REF / BACKPRESSURE constants) so codegen
    /// drift fails the build.
    #[test]
    fn link_rust_happy_path_emits_wrapper_struct() {
        let scxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="link" name="udp_scout" version="1.0">
  <sce:link-class>udp</sce:link-class>
  <sce:framer ref="scout_frame_codec"/>
  <sce:backpressure>drop</sce:backpressure>
  <sce:events>
    <sce:inbound event="scout.hello.received" when="decoded.msg_id == 0x02"/>
    <sce:outbound event="scout.query.send" encode="scout_frame_codec"/>
  </sce:events>
</scxml>"##;
        let label = DocumentLabel {
            identifier: "udp_scout",
            diagnostic_label: "udp_scout.scxml",
        };
        let out = compile_forge_from_string(scxml, label, generator::Language::Rust)
            .expect("forge rust codegen must succeed for a well-formed link");
        assert_eq!(out.files.len(), 1, "link emits a single .rs file");
        let (filename, body) = &out.files[0];
        assert_eq!(filename, "udp_scout.rs");

        // `sce_link_runtime` import is the contract anchor — a drift
        // here means the generated code lost its trait surface.
        assert!(
            body.contains("use sce_link_runtime::{Link, LinkError, RxFrame, TxFrame}"),
            "generated link must import the trait surface; full source:\n{body}",
        );

        // Pascal-cased wrapper with `<L: Link>` parameterization.
        assert!(
            body.contains("pub struct UdpScout<L: Link>"),
            "wrapper must be parameterized over an `impl Link`; full source:\n{body}",
        );
        assert!(
            body.contains("impl<L: Link> UdpScout<L>"),
            "wrapper must have an inherent impl block; full source:\n{body}",
        );

        // Author-declared static metadata round-trips into constants.
        assert!(
            body.contains("LINK_CLASS: &'static str = \"udp\""),
            "link-class must round-trip as a const; full source:\n{body}",
        );
        assert!(
            body.contains("FRAMER_REF: &'static str = \"scout_frame_codec\""),
            "framer ref must round-trip as a const; full source:\n{body}",
        );
        assert!(
            body.contains("BACKPRESSURE: &'static str = \"drop\""),
            "backpressure policy must round-trip as a const; full source:\n{body}",
        );

        // RX / TX entry points threading through the trait.
        assert!(
            body.contains("self.driver.rx()"),
            "rx() must delegate to the driver; full source:\n{body}",
        );
        assert!(
            body.contains("self.driver.tx(TxFrame::new(bytes))"),
            "tx() must wrap the slice in TxFrame; full source:\n{body}",
        );
    }

    /// Watching-zenoh RFC §5.C B6-α: link kind reject — missing
    /// `<sce:framer ref>` raises the dedicated `link/framer-missing`
    /// diagnostic at parse time, not at codegen. Pairs with the
    /// happy-path test above to verify the framer requirement is
    /// load-bearing (RFC §5.C "Codegen contract" — RX/TX paths thread
    /// through `framer.decode()` / `framer.encode()`).
    #[test]
    fn link_no_framer_rejects_via_link_framer_missing() {
        use crate::forge::diagnostic::{DiagnosticCode, ToDiagnostics};
        let scxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="link" name="udp_scout" version="1.0">
  <sce:link-class>udp</sce:link-class>
  <sce:backpressure>drop</sce:backpressure>
  <sce:events>
    <sce:inbound event="scout.hello.received"/>
  </sce:events>
</scxml>"##;
        let label = DocumentLabel {
            identifier: "udp_scout",
            diagnostic_label: "udp_scout.scxml",
        };
        let err = compile_forge_from_string(scxml, label, generator::Language::Rust)
            .err()
            .expect("link without <sce:framer ref> must reject");
        let diags = err.to_diagnostics();
        assert_eq!(diags.len(), 1, "single diagnostic for missing framer");
        let d = &diags[0];
        assert!(
            matches!(d.code, DiagnosticCode::LinkFramerMissing),
            "must be DiagnosticCode::LinkFramerMissing; got {:?}",
            d.code,
        );
        assert!(
            d.message.contains("<sce:framer ref="),
            "message must name the missing element; got {}",
            d.message,
        );
    }

    /// Watching-zenoh RFC §5.C B6-γ: `<sce:link-class>` body text
    /// outside the closed enum (RFC §5.C lines 765-771 — `udp` /
    /// `tcp` / `serial` / `websocket` / `raw_eth`) is caught by the
    /// XSD `linkClassType` enumeration in the default pipeline,
    /// surfacing as `xml/schema-validation` to the author. The
    /// dedicated `link/link-class-unknown` parser arm exists as a
    /// schema-skipped fallback (vendored builds without the
    /// `schemas/` directory) — exactly the same dual-path shape as
    /// `validation/unsupported-kind` for `sce:kind="bogus"`. The
    /// wire-format of the parser-arm diagnostic is locked by the
    /// `forge/link-link-class-unknown` golden in
    /// `forge_golden_entries`; this test pins the user-visible
    /// behavior in the standard XSD-validated pipeline.
    #[test]
    fn link_unknown_class_in_default_pipeline_routes_via_xsd() {
        use crate::forge::diagnostic::{DiagnosticCode, ToDiagnostics};
        let scxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="link" name="udp_scout" version="1.0">
  <sce:link-class>udpx</sce:link-class>
  <sce:framer ref="scout_frame_codec"/>
  <sce:backpressure>drop</sce:backpressure>
  <sce:events>
    <sce:inbound event="scout.hello.received"/>
  </sce:events>
</scxml>"##;
        let label = DocumentLabel {
            identifier: "udp_scout",
            diagnostic_label: "udp_scout.scxml",
        };
        let err = compile_forge_from_string(scxml, label, generator::Language::Rust)
            .err()
            .expect("link with unknown <sce:link-class> body must reject");
        let diags = err.to_diagnostics();
        assert!(
            !diags.is_empty(),
            "at least one diagnostic for unknown class"
        );
        let d = &diags[0];
        assert!(
            matches!(d.code, DiagnosticCode::XmlSchemaValidation),
            "default pipeline must surface XmlSchemaValidation \
             (XSD enum); got {:?}. The dedicated LinkLinkClassUnknown \
             arm fires only when XSD is skipped (sce-build vendored \
             without schemas/) — see the wire-format golden \
             `forge/link-link-class-unknown` for the parser-arm shape.",
            d.code,
        );
    }

    /// Watching-zenoh RFC §5.C B6-γ: `<sce:backpressure>` element is
    /// required on every link kind. B6-α tolerated the missing
    /// element by parser-side defaulting to `drop`; γ promotes the
    /// absence to a hard error (`link/backpressure-undeclared`) so
    /// authors must declare `drop` / `block` / `signal-event`
    /// intentionally. The repair is structural element-add, so the
    /// fix surface is None and the message prose enumerates the
    /// three legal bodies.
    #[test]
    fn link_missing_backpressure_rejects_via_link_backpressure_undeclared() {
        use crate::forge::diagnostic::{DiagnosticCode, ToDiagnostics};
        let scxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="link" name="udp_scout" version="1.0">
  <sce:link-class>udp</sce:link-class>
  <sce:framer ref="scout_frame_codec"/>
  <sce:events>
    <sce:inbound event="scout.hello.received"/>
  </sce:events>
</scxml>"##;
        let label = DocumentLabel {
            identifier: "udp_scout",
            diagnostic_label: "udp_scout.scxml",
        };
        let err = compile_forge_from_string(scxml, label, generator::Language::Rust)
            .err()
            .expect("link without <sce:backpressure> must reject");
        let diags = err.to_diagnostics();
        assert_eq!(diags.len(), 1, "single diagnostic for missing backpressure");
        let d = &diags[0];
        assert!(
            matches!(d.code, DiagnosticCode::LinkBackpressureUndeclared),
            "must be DiagnosticCode::LinkBackpressureUndeclared; got {:?}",
            d.code,
        );
        assert!(
            d.message.contains("<sce:backpressure>"),
            "message must name the missing element; got {}",
            d.message,
        );
        assert!(
            d.message.contains("drop|block|signal-event"),
            "message must enumerate the three legal policies; got {}",
            d.message,
        );
    }

    /// Watching-zenoh RFC §5.C B6-η: `<sce:link-class>` must be admitted by
    /// the deploy-resolved `platform.os`. The strict-literal matrix at
    /// [`forge::model::LinkClass::admits_os`] mirrors RFC §5.C lines 765-771
    /// — `serial` admits `bare_metal` only. Compiling an `udp_scout`-style
    /// link with `<sce:link-class>serial</sce:link-class>` against a
    /// `platform: { class: ap, os: linux }` machine raises
    /// `link/class-unsupported-on-target`. The new entry
    /// [`compile_forge_with_deploy`] is the only path that fires this
    /// diagnostic; the deploy-unaware [`compile_forge_from_string`] path
    /// stays silent (Q-η5 (a)) so the 6 existing link tests are unaffected.
    #[test]
    fn link_serial_on_linux_target_rejects_via_link_class_unsupported_on_target() {
        use crate::forge::diagnostic::{DiagnosticCode, ToDiagnostics};
        // Minimal deploy.yaml authoring `linux_node` with platform.os=linux.
        // `DeployConfig` nests machines under devices via `topology.<device>.
        // machines.<machine>` per `mesh::deploy::DeployConfig`'s shape.
        let deploy_yaml = r#"
version: "1.0"
topology:
  ap_device:
    machines:
      linux_node:
        source: serial_console.scxml
        platform:
          class: ap
          os: linux
"#;
        let deploy = mesh::deploy::parse_deploy_str(deploy_yaml).expect("deploy.yaml parses");
        let scxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="link" name="serial_console" version="1.0">
  <sce:link-class>serial</sce:link-class>
  <sce:framer ref="serial_frame_codec"/>
  <sce:backpressure>drop</sce:backpressure>
</scxml>"##;
        let label = DocumentLabel {
            identifier: "serial_console",
            diagnostic_label: "serial_console.scxml",
        };
        let err = compile_forge_with_deploy(
            scxml,
            label,
            generator::Language::Rust,
            Some(&deploy),
            Some("linux_node"),
        )
        .err()
        .expect("serial link on linux target must reject");
        let diags = err.to_diagnostics();
        assert_eq!(
            diags.len(),
            1,
            "single diagnostic for class-unsupported-on-target"
        );
        let d = &diags[0];
        assert!(
            matches!(d.code, DiagnosticCode::LinkClassUnsupportedOnTarget),
            "must be DiagnosticCode::LinkClassUnsupportedOnTarget; got {:?}",
            d.code,
        );
        assert!(
            d.message.contains("`serial`"),
            "message must name the offending class; got {}",
            d.message,
        );
        assert!(
            d.message.contains("`linux`"),
            "message must name the target os; got {}",
            d.message,
        );
        assert!(
            d.message.contains("bare_metal"),
            "message must enumerate the admitted OS axis; got {}",
            d.message,
        );

        // Q-η5 (a) skip-when-no-deploy: the same SCXML compiled via the
        // deploy-unaware entry passes parse + validate (it only fails
        // later at codegen-on-non-MCU which is a different diagnostic).
        // Verifying the η check is layered on top of the existing
        // pipeline rather than added to it.
        let no_deploy_err =
            compile_forge_with_deploy(scxml, label, generator::Language::Rust, None, None);
        // Either succeeds (rust accepts MCU-class kinds on AP code path
        // via the codegen-matrix) or fails for codegen reasons unrelated
        // to η — what matters is η does NOT fire.
        if let Err(e) = no_deploy_err {
            for d in e.to_diagnostics() {
                assert!(
                    !matches!(d.code, DiagnosticCode::LinkClassUnsupportedOnTarget),
                    "η must not fire when deploy is None; got {:?}",
                    d.code,
                );
            }
        }
    }

    /// Watching-zenoh RFC §5.C / §5.J.4: link is the first
    /// `KindClass::McuClass` kind. Authoring against cpp/kotlin/go/
    /// python raises `codegen/mcu-class-kind-on-non-mcu-language` via
    /// the existing A6 gate at `codegen_matrix::check`. Asserting on
    /// each non-MCU backend so a future refactor cannot accidentally
    /// flip a backend into the McuClass match arm.
    #[test]
    fn link_on_non_mcu_languages_rejects_via_codegen_matrix() {
        use crate::forge::diagnostic::{DiagnosticCode, ToDiagnostics};
        let scxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="link" name="udp_scout" version="1.0">
  <sce:link-class>udp</sce:link-class>
  <sce:framer ref="scout_frame_codec"/>
  <sce:backpressure>drop</sce:backpressure>
</scxml>"##;
        for lang in [
            generator::Language::Cpp,
            generator::Language::Kotlin,
            generator::Language::Go,
            generator::Language::Python,
        ] {
            let label = DocumentLabel {
                identifier: "udp_scout",
                diagnostic_label: "udp_scout.scxml",
            };
            let err = compile_forge_from_string(scxml, label, lang)
                .err()
                .unwrap_or_else(|| panic!("link must reject on {lang:?}"));
            let diags = err.to_diagnostics();
            assert_eq!(diags.len(), 1);
            assert!(
                matches!(
                    diags[0].code,
                    DiagnosticCode::CodegenMcuClassKindOnNonMcuLanguage
                ),
                "{lang:?} link emit must raise mcu-class-kind-on-non-mcu-language; got {:?}",
                diags[0].code,
            );
        }
    }

    /// Watching-zenoh RFC §5.C B6-β: link kind c11 happy path. Same
    /// fixture as the rust happy test → C11 generator emits a header
    /// composing a `sce_forge_link_t` driver via the canonical Linux-
    /// kernel separate-vtable shape (`const sce_forge_link_ops_t *ops` +
    /// `void *self`). Asserts presence of the load-bearing tokens
    /// (contract include + wrapper struct + init/rx/tx static-inline
    /// helpers + LINK_CLASS / LINK_FRAMER_REF / LINK_BACKPRESSURE
    /// macros + ops-pointer dispatch) so codegen drift fails the
    /// build. Per Q-β1=(b) the dispatch goes through
    /// `self->driver.ops->rx(self->driver.self, out)` — pattern (a)
    /// inline-vtable would route through `self->driver.rx(...)` with
    /// no `ops` indirection, so this assertion is what locks the
    /// shape.
    #[test]
    fn link_c11_happy_path_emits_struct_of_fnptrs() {
        let scxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="link" name="udp_scout" version="1.0">
  <sce:link-class>udp</sce:link-class>
  <sce:framer ref="scout_frame_codec"/>
  <sce:backpressure>drop</sce:backpressure>
  <sce:events>
    <sce:inbound event="scout.hello.received" when="decoded.msg_id == 0x02"/>
    <sce:outbound event="scout.query.send" encode="scout_frame_codec"/>
  </sce:events>
</scxml>"##;
        let label = DocumentLabel {
            identifier: "udp_scout",
            diagnostic_label: "udp_scout.scxml",
        };
        let out = compile_forge_from_string(scxml, label, generator::Language::C11)
            .expect("forge c11 codegen must succeed for a well-formed link");
        assert_eq!(out.files.len(), 1, "link emits a single .h file");
        let (filename, body) = &out.files[0];
        assert_eq!(filename, "udp_scout.h");

        // Header guard pin (mirrors `algorithm.h` / `codec.h` shape).
        assert!(
            body.contains("#ifndef SCE_FORGE_UDP_SCOUT_H"),
            "header must declare the SCE_FORGE_UDP_SCOUT_H guard; full source:\n{body}",
        );

        // `sce/forge/link.h` is the contract anchor — a drift here
        // means the generated code lost its driver-handle surface.
        assert!(
            body.contains("#include \"sce/forge/link.h\""),
            "generated link must include the contract header; full source:\n{body}",
        );

        // Wrapper struct composes a `sce_forge_link_t` by value.
        assert!(
            body.contains("sce_forge_link_t driver;"),
            "wrapper must compose a sce_forge_link_t driver; full source:\n{body}",
        );
        assert!(
            body.contains("} udp_scout_link_t;"),
            "wrapper struct must be typedef'd as udp_scout_link_t; full source:\n{body}",
        );

        // Init/rx/tx are static-inline helpers (header-only).
        assert!(
            body.contains("static inline void udp_scout_link_init("),
            "init helper must be emitted; full source:\n{body}",
        );
        assert!(
            body.contains("static inline bool udp_scout_link_rx("),
            "rx helper must be emitted; full source:\n{body}",
        );
        assert!(
            body.contains("static inline sce_forge_link_status_t udp_scout_link_tx("),
            "tx helper must return sce_forge_link_status_t; full source:\n{body}",
        );

        // Separate-vtable dispatch (Q-β1=(b)): the indirection goes
        // through `ops` rather than per-instance function pointers.
        // This assertion is the load-bearing pin for the textbook
        // Linux-kernel pattern decision.
        assert!(
            body.contains("self->driver.ops->rx(self->driver.self, out)"),
            "rx dispatch must route through `ops` (Q-β1=(b) separate vtable); full source:\n{body}",
        );
        assert!(
            body.contains("self->driver.ops->tx(self->driver.self, frame)"),
            "tx dispatch must route through `ops` (Q-β1=(b) separate vtable); full source:\n{body}",
        );

        // Author-declared static metadata round-trips into #defines
        // (C11 idiom; rust mirror uses `pub const`).
        assert!(
            body.contains("#define UDP_SCOUT_LINK_CLASS \"udp\""),
            "link-class must round-trip as a #define; full source:\n{body}",
        );
        assert!(
            body.contains("#define UDP_SCOUT_LINK_FRAMER_REF \"scout_frame_codec\""),
            "framer ref must round-trip as a #define; full source:\n{body}",
        );
        assert!(
            body.contains("#define UDP_SCOUT_LINK_BACKPRESSURE \"drop\""),
            "backpressure policy must round-trip as a #define; full source:\n{body}",
        );
    }

    /// Forge Kotlin procedure codegen mirrors cpp/Rust commit 3a/3b
    /// contract: bytes-typed assigns wrap in temp+check, return
    /// Event.ErrorExecution on overflow, executeTransitionActions
    /// signature now returns `Event?` per RFC §8 commit 3c.
    #[test]
    fn forge_kotlin_procedure_emits_bytes_cap_check() {
        let scxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="procedure" initial="req" version="1.0">
  <datamodel>
    <data id="seed" sce:type="bytes" sce:direction="internal" sce:max-size="32"/>
  </datamodel>
  <state id="req">
    <onentry><send sce:service="Probe" sce:response-max-size="32"/></onentry>
    <transition event="ok" target="done">
      <assign location="seed" expr="_event.data"/>
    </transition>
  </state>
  <final id="done"/>
</scxml>"##;
        let label = DocumentLabel {
            identifier: "bytes_probe",
            diagnostic_label: "bytes_probe.scxml",
        };
        let out = compile_forge_from_string(scxml, label, generator::Language::Kotlin)
            .expect("forge kotlin codegen must succeed for a bytes-bounded probe");
        assert_eq!(out.files.len(), 1);
        let (_, body) = &out.files[0];

        assert!(
            body.contains("ErrorExecution"),
            "Event.ErrorExecution must be emitted in Kotlin enum; full source:\n{body}",
        );
        assert!(
            body.contains(
                "override fun executeTransitionActions(source: State, trIndex: Int): Event?"
            ),
            "executeTransitionActions must return Event?; full source:\n{body}",
        );
        assert!(
            body.contains("scopeTmp"),
            "bytes-typed assign must use temp shape; full source:\n{body}",
        );
        assert!(
            body.contains("if (scopeTmp.size > 32)"),
            "cap-check must use the resolved cap (32 from sce:max-size); full source:\n{body}",
        );
        assert!(
            body.contains("return Event.ErrorExecution"),
            "cap violation path must raise Event.ErrorExecution; full source:\n{body}",
        );
    }

    /// Forge Go procedure codegen mirrors cpp/Rust/Kotlin contract:
    /// bytes-typed assigns wrap in scopeTmp+check shape, return
    /// (eventErrorExecution, true) on overflow per RFC §8 commit 3d.
    /// Go uses (raised, ok) tuple instead of Optional<Event> per
    /// idiomatic Go convention.
    #[test]
    fn forge_go_procedure_emits_bytes_cap_check() {
        let scxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="procedure" initial="req" version="1.0">
  <datamodel>
    <data id="seed" sce:type="bytes" sce:direction="internal" sce:max-size="32"/>
  </datamodel>
  <state id="req">
    <onentry><send sce:service="Probe" sce:response-max-size="32"/></onentry>
    <transition event="ok" target="done">
      <assign location="seed" expr="_event.data"/>
    </transition>
  </state>
  <final id="done"/>
</scxml>"##;
        let label = DocumentLabel {
            identifier: "bytes_probe",
            diagnostic_label: "bytes_probe.scxml",
        };
        let out = compile_forge_from_string(scxml, label, generator::Language::Go)
            .expect("forge go codegen must succeed for a bytes-bounded probe");
        assert_eq!(out.files.len(), 1);
        let (_, body) = &out.files[0];

        assert!(
            body.contains("eventErrorExecution"),
            "eventErrorExecution must be emitted in Go const block; full source:\n{body}",
        );
        assert!(
            body.contains("ExecuteTransitionActions(source int, trIndex int) (int, bool)"),
            "ExecuteTransitionActions must return (int, bool); full source:\n{body}",
        );
        assert!(
            body.contains("scopeTmp"),
            "bytes-typed assign must use scopeTmp shape; full source:\n{body}",
        );
        assert!(
            body.contains("if len(scopeTmp) > 32"),
            "cap-check must use the resolved cap (32); full source:\n{body}",
        );
        assert!(
            body.contains("return eventErrorExecution, true"),
            "cap violation path must return (eventErrorExecution, true); full source:\n{body}",
        );
    }

    /// Forge Python procedure codegen mirrors cpp/Rust/Kotlin/Go
    /// contract: bytes-typed assigns wrap in _scope_tmp+check shape,
    /// return Event.ErrorExecution on overflow per RFC §8 commit 3e.
    /// Python uses Optional[int] for the Optional<Event> analogue.
    #[test]
    fn forge_python_procedure_emits_bytes_cap_check() {
        let scxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="procedure" initial="req" version="1.0">
  <datamodel>
    <data id="seed" sce:type="bytes" sce:direction="internal" sce:max-size="32"/>
  </datamodel>
  <state id="req">
    <onentry><send sce:service="Probe" sce:response-max-size="32"/></onentry>
    <transition event="ok" target="done">
      <assign location="seed" expr="_event.data"/>
    </transition>
  </state>
  <final id="done"/>
</scxml>"##;
        let label = DocumentLabel {
            identifier: "bytes_probe",
            diagnostic_label: "bytes_probe.scxml",
        };
        let out = compile_forge_from_string(scxml, label, generator::Language::Python)
            .expect("forge python codegen must succeed for a bytes-bounded probe");
        assert_eq!(out.files.len(), 1);
        let (_, body) = &out.files[0];

        assert!(
            body.contains("ErrorExecution = 1"),
            "Event.ErrorExecution must be in IntEnum; full source:\n{body}",
        );
        assert!(
            body.contains("def _execute_transition_actions"),
            "_execute_transition_actions present; full source:\n{body}",
        );
        assert!(
            body.contains("-> Optional[int]"),
            "_execute_transition_actions must return Optional[int]; full source:\n{body}",
        );
        assert!(
            body.contains("_scope_tmp = "),
            "bytes-typed assign must use _scope_tmp shape; full source:\n{body}",
        );
        assert!(
            body.contains("if len(_scope_tmp) > 32"),
            "cap-check must use the resolved cap (32); full source:\n{body}",
        );
        assert!(
            body.contains("return Event.ErrorExecution"),
            "cap violation path must return Event.ErrorExecution; full source:\n{body}",
        );
    }

    /// Forge C11 procedure L2 codegen completes the 6-backend
    /// contract (cpp/Rust/Kotlin/Go/Python + C11) for the bytes
    /// cap-check raise path. C uses a stack-bounded
    /// sce_forge_bytes_t struct (no heap) and a
    /// (raised, event) tuple analogue from
    /// sce/forge/procedure.h. RFC §8 commit 4a — codegen path
    /// only; conformance harness wiring lands in commit 4b.
    #[test]
    fn forge_c11_procedure_emits_bytes_cap_check() {
        let scxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="procedure" initial="req" version="1.0">
  <datamodel>
    <data id="seed" sce:type="bytes" sce:direction="internal" sce:max-size="32"/>
  </datamodel>
  <state id="req">
    <onentry><send sce:service="Probe" sce:response-max-size="32"/></onentry>
    <transition event="ok" target="done">
      <assign location="seed" expr="_event.data"/>
    </transition>
  </state>
  <final id="done"/>
</scxml>"##;
        let label = DocumentLabel {
            identifier: "bytes_probe",
            diagnostic_label: "bytes_probe.scxml",
        };
        let out = compile_forge_from_string(scxml, label, generator::Language::C11)
            .expect("forge c11 codegen must succeed for a bytes-bounded probe");
        assert_eq!(out.files.len(), 1);
        let (_, body) = &out.files[0];

        // Always-emitted EVENT_ERROR_EXECUTION enum value.
        assert!(
            body.contains("EVENT_ERROR_EXECUTION"),
            "C11 enum must include EVENT_ERROR_EXECUTION; full source:\n{body}",
        );
        // Cap-check raise typedef from the new runtime header.
        assert!(
            body.contains("sce_forge_procedure_raise_t"),
            "execute_transition_actions must return sce_forge_procedure_raise_t; full source:\n{body}",
        );
        // Bytes-typed assign uses the temp+check shape with the
        // resolved cap (32 from sce:max-size).
        assert!(
            body.contains("sce_forge_bytes_t _scope_tmp"),
            "bytes-typed assign must use _scope_tmp shape; full source:\n{body}",
        );
        assert!(
            body.contains("if (_scope_tmp.len > 32)"),
            "cap-check must use the resolved cap (32); full source:\n{body}",
        );
        // Raise path sets _r.raised + _r.event = ErrorExecution.
        assert!(
            body.contains("_r.raised = true"),
            "cap violation path must set _r.raised; full source:\n{body}",
        );
    }

    #[test]
    fn compile_from_string_lang_c11_emits_h_and_c_pair() {
        let scxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
       datamodel="null" name="c11_probe" initial="s">
  <state id="s"/>
</scxml>"##;
        // Resolve the template root the same way `find_template_dir_for(C11)`
        // does so this test runs from any cargo invocation directory.
        let template_dir = find_template_dir_for(generator::Language::C11);
        let templates: Vec<(String, String)> = collect_templates_for_test(&template_dir);
        let template_refs: Vec<(&str, &str)> = templates
            .iter()
            .map(|(n, c)| (n.as_str(), c.as_str()))
            .collect();
        let out = compile_from_string_lang_typed(
            scxml,
            "c11_probe",
            &template_refs,
            generator::Language::C11,
        )
        .expect("C11 statechart codegen must succeed for the minimum fixture");
        // Pair shape: `<stem>_sm.h` + `<stem>_sm.c`, both non-empty.
        let names: Vec<&str> = out.files.iter().map(|(n, _)| n.as_str()).collect();
        assert!(
            names.contains(&"c11_probe_sm.h"),
            "C11 output must include c11_probe_sm.h: {names:?}"
        );
        assert!(
            names.contains(&"c11_probe_sm.c"),
            "C11 output must include c11_probe_sm.c: {names:?}"
        );
        for (name, body) in &out.files {
            assert!(!body.is_empty(), "C11 emitted empty body for {name}");
        }
    }

    /// Walk the template directory recursively and collect every `.jinja2`
    /// into `(relative_name, content)` pairs — the same shape that
    /// `compile_from_string_lang_typed` expects for the WASM-compatible
    /// path. Test-only helper so we exercise the string-template path
    /// without spawning a separate test for the filesystem path.
    fn collect_templates_for_test(base: &std::path::Path) -> Vec<(String, String)> {
        let mut out = Vec::new();
        fn walk(base: &std::path::Path, cur: &std::path::Path, out: &mut Vec<(String, String)>) {
            for entry in std::fs::read_dir(cur).expect("read template dir") {
                let entry = entry.expect("dir entry");
                let path = entry.path();
                if path.is_dir() {
                    walk(base, &path, out);
                } else if path.extension().and_then(|e| e.to_str()) == Some("jinja2") {
                    let rel = path.strip_prefix(base).expect("strip prefix");
                    let name = rel.to_string_lossy().replace('\\', "/");
                    let content = std::fs::read_to_string(&path).expect("read template");
                    out.push((name, content));
                }
            }
        }
        walk(base, base, &mut out);
        out
    }

    #[test]
    fn find_template_dir_for_c11_returns_template_root() {
        let dir = find_template_dir_for(generator::Language::C11);
        // M2 land: `find_template_dir_for(C11)` returns the shared
        // template root (matching the C++ arm) so `load_templates`
        // walks both `<root>/c/state_machine.{h,c}.jinja2` and the
        // root-level shared templates (`license_header.jinja2`,
        // `actions/*.jinja2`) in one pass.
        assert!(
            dir.exists(),
            "C11 template root must exist: {}",
            dir.display()
        );
        let c_subdir = dir.join("c");
        assert!(
            c_subdir.exists(),
            "C11 templates must live under <root>/c/: {}",
            c_subdir.display()
        );
        assert!(
            c_subdir.join("state_machine.h.jinja2").exists(),
            "C11 must ship state_machine.h.jinja2 at <root>/c/: {}",
            c_subdir.display()
        );
        assert!(
            c_subdir.join("state_machine.c.jinja2").exists(),
            "C11 must ship state_machine.c.jinja2 at <root>/c/: {}",
            c_subdir.display()
        );
    }

    /// Regression guard for the bug fixed in this commit: a server whose
    /// only reply path is a spontaneous eventgroup notification (no RPC
    /// pair, no FieldAccess pair) must still trigger
    /// `inject_server_response_sends` so the SM generator sees the
    /// synthetic `<send>` alongside the `<raise>`. Before the fix,
    /// eventgroup-only servers hit the `server_pairs.is_empty() &&
    /// field_access_pairs.is_empty()` early return; the raise emitted
    /// but the mesh send callback never fired.
    ///
    /// Pre-fix behaviour was masked on the multi fixture because a
    /// field.get pair happened to produce the same response event
    /// name as the eventgroup push — SCE Mesh §8.1's pure-push case
    /// (spontaneous `field.notify.X` from a sensor.update trigger)
    /// was therefore never exercised until the dedicated unsubscribe
    /// fixture landed.
    #[test]
    fn inject_server_model_mutations_handles_eventgroup_only_server() {
        use std::fs;
        use std::sync::atomic::{AtomicUsize, Ordering};

        static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);
        let tmp = std::env::temp_dir().join(format!(
            "sce_inject_eventgroup_only_{}_{}",
            std::process::id(),
            TEST_COUNTER.fetch_add(1, Ordering::Relaxed),
        ));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        let deploy = r##"version: "1.0"
topology:
  ecu1:
    transports:
      someip:
        config: vsomeip.json
        application_name: brake_app
    machines:
      brake:
        source: brake.scxml
        bindings:
          "#motor":
            transport: someip
            service: motor_control
            events:
              "event.subscribe.speed":
                event_group: speed_group
      motor:
        source: motor.scxml
        server:
          transport: someip
          service: motor_control
          events:
            "field.notify.vehicle_speed":
              event_group: speed_group
"##;
        let vsomeip = r##"{
  "applications": [{"name": "brake_app"}],
  "services": [{
    "name": "motor_control",
    "service": "0x2000",
    "instance": "0x0001",
    "eventgroups": [{"name": "speed_group", "eventgroup": "0x0002", "events": ["0x8002"]}]
  }]
}
"##;
        // Motor has ONLY a spontaneous raise — no field.get/field.set pair,
        // no RPC pair. This is the exact shape the bug bit on.
        let motor_scxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="null" name="motor" initial="ready">
    <state id="ready">
        <transition event="sensor.update" target="ready">
            <raise event="field.notify.vehicle_speed"/>
        </transition>
    </state>
</scxml>"##;
        let brake_scxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="null" name="brake" initial="idle">
    <state id="idle"/>
</scxml>"##;
        fs::write(tmp.join("deploy.yaml"), deploy).unwrap();
        fs::write(tmp.join("vsomeip.json"), vsomeip).unwrap();
        fs::write(tmp.join("motor.scxml"), motor_scxml).unwrap();
        fs::write(tmp.join("brake.scxml"), brake_scxml).unwrap();

        let mut model = parser::SCXMLParser::new()
            .parse_file(tmp.join("motor.scxml").to_str().unwrap())
            .expect("parse motor");

        let injected =
            inject_server_model_mutations(&mut model, &tmp.join("deploy.yaml")).expect("inject");

        assert!(
            !injected.is_empty(),
            "eventgroup-only server should trigger injection; got empty vec"
        );

        // The motor's single transition now carries a [raise, send] pair
        // instead of the naked raise. Find it and assert the shape.
        let ready_state = model.states.get("ready").expect("ready state exists");
        let transition = ready_state
            .transitions
            .iter()
            .find(|t| t.event == "sensor.update")
            .expect("sensor.update transition exists");
        assert_eq!(
            transition.actions.len(),
            2,
            "transition must have raise + injected send (got {} actions)",
            transition.actions.len()
        );
        assert_eq!(transition.actions[0].action_type, "raise");
        assert_eq!(transition.actions[0].event, "field.notify.vehicle_speed");
        assert_eq!(transition.actions[1].action_type, "send");
        assert_eq!(transition.actions[1].event, "field.notify.vehicle_speed");
        assert_eq!(transition.actions[1].target, "#motor");

        let _ = fs::remove_dir_all(&tmp);
    }

    // ── SCE_MESH.md §9.6 remote-invoke classifier ─────────────────────

    /// The deploy config used by the remote-invoke classifier tests.
    /// Declares `parent` and `worker` on the same device; the classifier
    /// only cares about `device_for_machine()` membership, so a single
    /// device with both machines is enough to pin the behaviour.
    fn two_machine_deploy_cfg() -> mesh::deploy::DeployConfig {
        let yaml = r#"
version: "1.0"
topology:
  ecu1:
    platform: linux-x86_64
    machines:
      parent:
        source: parent.scxml
      worker:
        source: worker.scxml
"#;
        mesh::deploy::parse_deploy_str(yaml).expect("parse deploy yaml")
    }

    fn parse_parent(scxml: &str) -> SCXMLModel {
        parser::SCXMLParser::new()
            .parse_string(scxml, "parent")
            .expect("parse parent scxml")
    }

    /// Baseline: `<invoke type="scxml" src="#worker">` where `worker` is a
    /// distinct declared machine must be flagged as remote so C++ codegen
    /// emits the §10.7.1 `SESSION_F_NOT_IMPLEMENTED` raise.
    #[test]
    fn remote_mesh_invoke_flagged_when_src_references_declared_peer() {
        let scxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s1" name="parent">
  <state id="s1">
    <invoke type="scxml" src="#worker" id="inv0"/>
  </state>
</scxml>"##;
        let mut model = parse_parent(scxml);
        classify_remote_scxml_invokes(&mut model, &two_machine_deploy_cfg(), "parent");
        let state = model.states.get("s1").expect("s1 present");
        let info = match &state.invokes[0] {
            model::Invoke::Scxml(i) => i,
            other => panic!("expected Scxml invoke, got {other:?}"),
        };
        assert_eq!(info.remote_mesh_target.as_deref(), Some("worker"));
    }

    /// WASM-mode inline-content invoke (`parse_string`, no `base_dir`)
    /// keeps `src` empty because the parser cannot write the synth
    /// sibling without a filesystem; the classifier therefore finds
    /// nothing to strip the `#` prefix from and leaves the entry
    /// alone. End-to-end inline synthesis (§9.6.6 rule 2) is covered
    /// by the W3C 191/192/253 ctest suite once a real sibling dir is
    /// in play.
    #[test]
    fn inline_content_invoke_left_unflagged() {
        let scxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s1" name="parent">
  <state id="s1">
    <invoke type="scxml" id="inv0">
      <content>
        <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="c"><state id="c"/></scxml>
      </content>
    </invoke>
  </state>
</scxml>"##;
        let mut model = parse_parent(scxml);
        classify_remote_scxml_invokes(&mut model, &two_machine_deploy_cfg(), "parent");
        let state = model.states.get("s1").expect("s1 present");
        let info = match &state.invokes[0] {
            model::Invoke::Scxml(i) => i,
            other => panic!("expected Scxml invoke, got {other:?}"),
        };
        assert!(
            info.remote_mesh_target.is_none(),
            "WASM-mode inline-content invoke must not be flagged; got {:?}",
            info.remote_mesh_target
        );
    }

    /// SCE_MESH.md §9.6.6 rule 3: a target whose deploy.yaml partition
    /// matches the parent's partition is a local invoke — the mesh
    /// wire would introduce a needless hop. Covers the upgraded
    /// partition-aware classifier; pre-upgrade the classifier would
    /// have flagged this as remote on pure "declared peer" grounds.
    #[test]
    fn same_partition_peer_not_flagged_remote() {
        let deploy = r#"
version: "1.0"
topology:
  ecu1:
    machines:
      parent: { source: parent.scxml }
      worker: { source: worker.scxml }
partitions:
  grouped:
    machines: [parent, worker]
    contains:
      invokes:
        - { machine: parent, invoke: inv0 }
"#;
        let cfg = mesh::deploy::parse_deploy_str(deploy).expect("deploy must parse");
        let scxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s1" name="parent">
  <state id="s1">
    <invoke type="scxml" src="#worker" id="inv0"/>
  </state>
</scxml>"##;
        let mut model = parse_parent(scxml);
        classify_remote_scxml_invokes(&mut model, &cfg, "parent");
        let state = model.states.get("s1").expect("s1 present");
        let info = match &state.invokes[0] {
            model::Invoke::Scxml(i) => i,
            other => panic!("expected Scxml invoke, got {other:?}"),
        };
        assert!(
            info.remote_mesh_target.is_none(),
            "same-partition peer must not be flagged remote; got {:?}",
            info.remote_mesh_target,
        );
    }

    /// §9.6.6 rule 3 cross-partition: distinct partitions ⇒ distinct
    /// OS processes ⇒ mesh hop. Classifier flags as remote so the
    /// C++ codegen takes the §10.7.1 Session F path.
    #[test]
    fn cross_partition_peer_flagged_remote() {
        let deploy = r#"
version: "1.0"
topology:
  ecu1:
    machines:
      parent: { source: parent.scxml }
      worker: { source: worker.scxml }
partitions:
  left:
    machines: [parent]
    contains:
      invokes:
        - { machine: parent, invoke: inv0 }
  right:
    machines: [worker]
    contains:
      invokes:
        - { machine: worker, invoke: dummy }
"#;
        let cfg = mesh::deploy::parse_deploy_str(deploy).expect("deploy must parse");
        let scxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s1" name="parent">
  <state id="s1">
    <invoke type="scxml" src="#worker" id="inv0"/>
  </state>
</scxml>"##;
        let mut model = parse_parent(scxml);
        classify_remote_scxml_invokes(&mut model, &cfg, "parent");
        let state = model.states.get("s1").expect("s1 present");
        let info = match &state.invokes[0] {
            model::Invoke::Scxml(i) => i,
            other => panic!("expected Scxml invoke, got {other:?}"),
        };
        assert_eq!(
            info.remote_mesh_target.as_deref(),
            Some("worker"),
            "cross-partition peer must be flagged remote",
        );
    }

    /// Self-reference (`src="#parent"`) is almost certainly an author
    /// mistake, but classification-wise it is not remote — the machine
    /// cannot `<invoke>` itself as a peer. Leaving it unflagged preserves
    /// the existing local-invoke error surface (unresolved child SCXML).
    #[test]
    fn self_reference_not_flagged_remote() {
        let scxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s1" name="parent">
  <state id="s1">
    <invoke type="scxml" src="#parent" id="inv0"/>
  </state>
</scxml>"##;
        let mut model = parse_parent(scxml);
        classify_remote_scxml_invokes(&mut model, &two_machine_deploy_cfg(), "parent");
        let state = model.states.get("s1").expect("s1 present");
        let info = match &state.invokes[0] {
            model::Invoke::Scxml(i) => i,
            other => panic!("expected Scxml invoke, got {other:?}"),
        };
        assert!(info.remote_mesh_target.is_none());
    }

    /// SCE_MESH.md §9.6 codegen-shape exclusivity. When the same machine
    /// is both a mesh peer (inbound `<invoke src="#<this>">`) and a local
    /// invoke target (inbound `<invoke src="<this>.scxml">`) within the
    /// same deployment, the two child-SM shapes conflict and the build
    /// must reject with `ScxmlInvokeTargetConflict`. Absence of either
    /// shape passes through cleanly.
    #[test]
    fn scxml_invoke_target_conflict_rejected_when_both_shapes_present() {
        use std::fs;
        use std::sync::atomic::{AtomicUsize, Ordering};

        static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);
        let tmp = std::env::temp_dir().join(format!(
            "sce_invoke_target_conflict_{}_{}",
            std::process::id(),
            TEST_COUNTER.fetch_add(1, Ordering::Relaxed),
        ));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        let deploy = r##"version: "1.0"
topology:
  ecu1:
    platform: linux-x86_64
    machines:
      parent_local:
        source: parent_local.scxml
      parent_mesh:
        source: parent_mesh.scxml
      worker:
        source: worker.scxml
"##;
        // parent_local takes the W3C local-file shape: src resolves to
        // worker.scxml via the sibling dir (same tempdir).
        let parent_local_scxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s1" name="parent_local">
  <state id="s1">
    <invoke type="scxml" src="worker.scxml" id="inv0"/>
  </state>
</scxml>"##;
        // parent_mesh takes the mesh shape: src starts with `#`.
        let parent_mesh_scxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s1" name="parent_mesh">
  <state id="s1">
    <invoke type="scxml" src="#worker" id="inv0"/>
  </state>
</scxml>"##;
        let worker_scxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="null" initial="done" name="worker">
  <final id="done"/>
</scxml>"##;

        fs::write(tmp.join("deploy.yaml"), deploy).unwrap();
        fs::write(tmp.join("parent_local.scxml"), parent_local_scxml).unwrap();
        fs::write(tmp.join("parent_mesh.scxml"), parent_mesh_scxml).unwrap();
        fs::write(tmp.join("worker.scxml"), worker_scxml).unwrap();

        let mut model = parser::SCXMLParser::new()
            .parse_file(tmp.join("worker.scxml").to_str().unwrap())
            .expect("parse worker");

        let err = inject_partition_context_for(&mut model, &tmp.join("deploy.yaml"), None)
            .expect_err("must reject when worker is both mesh peer and local invoke target");
        match err {
            mesh::error::MeshError::Deploy(boxed) => match *boxed {
                mesh::error::DeployError::ScxmlInvokeTargetConflict {
                    machine,
                    inbound_peers,
                    local_invoker,
                    local_src,
                } => {
                    assert_eq!(machine, "worker");
                    assert_eq!(inbound_peers, vec!["parent_mesh".to_string()]);
                    assert_eq!(local_invoker, "parent_local");
                    assert_eq!(local_src, "worker.scxml");
                }
                other => panic!("expected ScxmlInvokeTargetConflict, got {other:?}"),
            },
            other => panic!("expected ScxmlInvokeTargetConflict, got {other:?}"),
        }

        let _ = fs::remove_dir_all(&tmp);
    }

    /// SCE_MESH.md §9.6 codegen-shape exclusivity — the absence path.
    /// When only one shape is present (here: mesh-only), the validator
    /// must accept cleanly so the round-trip W3C workers and pure mesh
    /// topologies continue to build. Also pins `is_remote_invoke_target`
    /// population through the same call path so downstream template
    /// authors can rely on the flag's definition.
    #[test]
    fn scxml_invoke_target_mesh_only_accepted_and_flag_set() {
        use std::fs;
        use std::sync::atomic::{AtomicUsize, Ordering};

        static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);
        let tmp = std::env::temp_dir().join(format!(
            "sce_invoke_target_mesh_only_{}_{}",
            std::process::id(),
            TEST_COUNTER.fetch_add(1, Ordering::Relaxed),
        ));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        let deploy = r##"version: "1.0"
topology:
  ecu1:
    platform: linux-x86_64
    machines:
      parent_mesh:
        source: parent_mesh.scxml
      worker:
        source: worker.scxml
"##;
        let parent_mesh_scxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s1" name="parent_mesh">
  <state id="s1">
    <invoke type="scxml" src="#worker" id="inv0"/>
  </state>
</scxml>"##;
        let worker_scxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="null" initial="done" name="worker">
  <final id="done"/>
</scxml>"##;

        fs::write(tmp.join("deploy.yaml"), deploy).unwrap();
        fs::write(tmp.join("parent_mesh.scxml"), parent_mesh_scxml).unwrap();
        fs::write(tmp.join("worker.scxml"), worker_scxml).unwrap();

        let mut model = parser::SCXMLParser::new()
            .parse_file(tmp.join("worker.scxml").to_str().unwrap())
            .expect("parse worker");

        let ctx = inject_partition_context_for(&mut model, &tmp.join("deploy.yaml"), None)
            .expect("mesh-only worker must accept");
        // Partition context not required — single-device deploy.
        assert!(!ctx);
        assert_eq!(
            model.scxml_remote_inbound_peers,
            vec![model::ScxmlRemotePeerBinding::new("parent_mesh")],
        );
        assert!(
            model.is_remote_invoke_target,
            "mesh-peer worker must have is_remote_invoke_target set for the codegen-shape seam"
        );

        let _ = fs::remove_dir_all(&tmp);
    }

    /// `#<name>` that does not match any declared machine — e.g., author
    /// typo or a reference to something that was never registered —
    /// stays unflagged. The local W3C invoke lookup path reports the
    /// unresolved child with its existing diagnostic.
    #[test]
    fn unknown_peer_name_not_flagged_remote() {
        let scxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s1" name="parent">
  <state id="s1">
    <invoke type="scxml" src="#nonexistent" id="inv0"/>
  </state>
</scxml>"##;
        let mut model = parse_parent(scxml);
        classify_remote_scxml_invokes(&mut model, &two_machine_deploy_cfg(), "parent");
        let state = model.states.get("s1").expect("s1 present");
        let info = match &state.invokes[0] {
            model::Invoke::Scxml(i) => i,
            other => panic!("expected Scxml invoke, got {other:?}"),
        };
        assert!(info.remote_mesh_target.is_none());
    }

    /// End-to-end §9.6.6 rule 3 override. Author places the
    /// synthesised child on a different partition than the parent;
    /// deploy validation admits the override-style topology entry and
    /// the classifier flags the synth as remote so C++ codegen emits
    /// the §10.7.1 Session F scaffold. Inline `<content>` is captured
    /// on the invoke as [`ScxmlInvokeInfo::inline_child`] so the
    /// child-side metadata (`child_needs_script_engine` etc.) is
    /// populated from the in-memory submodel rather than a sibling
    /// `.scxml` file on disk.
    #[test]
    fn synth_invoke_override_flagged_remote() {
        use std::fs;
        use std::sync::atomic::{AtomicUsize, Ordering};

        static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);
        let tmp = std::env::temp_dir().join(format!(
            "sce_synth_override_{}_{}",
            std::process::id(),
            TEST_COUNTER.fetch_add(1, Ordering::Relaxed),
        ));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        let deploy = r##"version: "1.0"
topology:
  ecu1:
    platform: linux-x86_64
    machines:
      parent:
        source: parent.scxml
      parent__sce_synth_invoke__inv0:
        source: parent__sce_synth_invoke__inv0.scxml
partitions:
  p_main:
    machines: [parent]
    contains:
      invokes:
        - { machine: parent, invoke: inv0 }
  p_remote:
    machines: [parent__sce_synth_invoke__inv0]
    contains:
      invokes:
        - { machine: parent__sce_synth_invoke__inv0, invoke: unused }
"##;
        let parent_scxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s1" name="parent">
  <state id="s1">
    <invoke type="scxml" id="inv0">
      <content>
        <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="done">
          <final id="done"/>
        </scxml>
      </content>
    </invoke>
  </state>
</scxml>"##;

        fs::write(tmp.join("deploy.yaml"), deploy).unwrap();
        fs::write(tmp.join("parent.scxml"), parent_scxml).unwrap();

        // parse_file captures the inline `<content>` submodel on the
        // invoke (no disk side-effect); `inject_partition_context_for`
        // and the classifier consume the in-memory child without
        // needing a sibling file.
        let mut model = parser::SCXMLParser::new()
            .parse_file(tmp.join("parent.scxml").to_str().unwrap())
            .expect("parse parent with inline content");

        // Validate only the classification axis; partition-coverage
        // rule 1 is out of scope here (the fixture uses `unused`
        // invoke ids on the synth side to satisfy rule 10 non-empty).
        let cfg = mesh::deploy::parse_deploy_str(deploy).expect("deploy must admit override");
        classify_remote_scxml_invokes(&mut model, &cfg, "parent");

        let state = model.states.get("s1").expect("s1 present");
        let info = match &state.invokes[0] {
            model::Invoke::Scxml(i) => i,
            other => panic!("expected Scxml invoke, got {other:?}"),
        };
        assert_eq!(
            info.remote_mesh_target.as_deref(),
            Some("parent__sce_synth_invoke__inv0"),
            "override-partitioned synth must be flagged remote",
        );

        let _ = fs::remove_dir_all(&tmp);
    }

    /// §9.6.6 rule 3 default. Parent and inline-content invoke run in
    /// the same partition (the synth's implicit inheritance). The
    /// classifier must leave the synth local so the generated code
    /// uses the local child-session path, matching W3C test191/192/253
    /// semantics under a partitioned deploy.
    #[test]
    fn synth_invoke_default_inherits_parent_partition_local() {
        use std::fs;
        use std::sync::atomic::{AtomicUsize, Ordering};

        static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);
        let tmp = std::env::temp_dir().join(format!(
            "sce_synth_default_{}_{}",
            std::process::id(),
            TEST_COUNTER.fetch_add(1, Ordering::Relaxed),
        ));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        let deploy = r##"version: "1.0"
topology:
  ecu1:
    machines:
      parent:
        source: parent.scxml
partitions:
  p_main:
    machines: [parent]
    contains:
      invokes:
        - { machine: parent, invoke: inv0 }
"##;
        let parent_scxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s1" name="parent">
  <state id="s1">
    <invoke type="scxml" id="inv0">
      <content>
        <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="done">
          <final id="done"/>
        </scxml>
      </content>
    </invoke>
  </state>
</scxml>"##;

        fs::write(tmp.join("deploy.yaml"), deploy).unwrap();
        fs::write(tmp.join("parent.scxml"), parent_scxml).unwrap();

        let mut model = parser::SCXMLParser::new()
            .parse_file(tmp.join("parent.scxml").to_str().unwrap())
            .expect("parse parent with inline content");

        let cfg = mesh::deploy::parse_deploy_str(deploy).expect("deploy must parse");
        classify_remote_scxml_invokes(&mut model, &cfg, "parent");

        let state = model.states.get("s1").expect("s1 present");
        let info = match &state.invokes[0] {
            model::Invoke::Scxml(i) => i,
            other => panic!("expected Scxml invoke, got {other:?}"),
        };
        assert!(
            info.remote_mesh_target.is_none(),
            "default-inheritance synth must stay local; got {:?}",
            info.remote_mesh_target,
        );

        let _ = fs::remove_dir_all(&tmp);
    }

    /// §9.6.6 rule 3 override — peer-collection layer. The parser keeps
    /// inline `<content>` in-memory on [`ScxmlInvokeInfo::inline_child`]
    /// and rewrites `src` to `#<synth>` for classification — the on-disk
    /// parent.scxml still carries inline content with no `src=`
    /// attribute. The synth-side inbound scan therefore needs the
    /// `__sce_synth_invoke__` infix inversion to recover its parent
    /// as an inbound peer; this test pins that contract by running
    /// `collect_scxml_remote_peers` from **both** sides of the same
    /// override fixture used by `synth_invoke_override_flagged_remote`.
    #[test]
    fn synth_invoke_override_wires_both_peer_sides() {
        use std::fs;
        use std::sync::atomic::{AtomicUsize, Ordering};

        static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);
        let tmp = std::env::temp_dir().join(format!(
            "sce_synth_peers_{}_{}",
            std::process::id(),
            TEST_COUNTER.fetch_add(1, Ordering::Relaxed),
        ));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        let deploy = r##"version: "1.0"
topology:
  ecu1:
    platform: linux-x86_64
    machines:
      parent:
        source: parent.scxml
      parent__sce_synth_invoke__inv0:
        source: parent__sce_synth_invoke__inv0.scxml
partitions:
  p_main:
    machines: [parent]
    contains:
      invokes:
        - { machine: parent, invoke: inv0 }
  p_remote:
    machines: [parent__sce_synth_invoke__inv0]
    contains:
      invokes:
        - { machine: parent__sce_synth_invoke__inv0, invoke: unused }
"##;
        let parent_scxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s1" name="parent">
  <state id="s1">
    <invoke type="scxml" id="inv0">
      <content>
        <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="done">
          <final id="done"/>
        </scxml>
      </content>
    </invoke>
  </state>
</scxml>"##;

        let deploy_path = tmp.join("deploy.yaml");
        fs::write(&deploy_path, deploy).unwrap();
        fs::write(tmp.join("parent.scxml"), parent_scxml).unwrap();

        // Parse parent — inline `<content>` is captured as an in-memory
        // submodel on the invoke (no disk side-effect).
        let mut parent_model = parser::SCXMLParser::new()
            .parse_file(tmp.join("parent.scxml").to_str().unwrap())
            .expect("parse parent with inline content");

        // §9.6.6 contract: inline child is pre-parsed and lives on the
        // invoke, not on disk. Snapshot it now (cloned) for the synth-side
        // peer-collection pass below; mutating `parent_model` afterwards
        // does not touch the cloned submodel.
        let mut synth_model = parent_model
            .states
            .get("s1")
            .and_then(|s| s.invokes.first())
            .and_then(|inv| match inv {
                model::Invoke::Scxml(i) => i.inline_child.as_deref().cloned(),
                _ => None,
            })
            .expect("parser must have captured the inline child in-memory");
        assert!(
            !tmp.join("parent__sce_synth_invoke__inv0.scxml").exists(),
            "parser must not write the synth sibling to disk",
        );

        let cfg = mesh::deploy::parse_deploy_str(deploy).expect("deploy must admit override");

        classify_remote_scxml_invokes(&mut parent_model, &cfg, "parent");
        collect_scxml_remote_peers(&mut parent_model, &cfg, "parent", &deploy_path);

        assert_eq!(
            parent_model.scxml_remote_outbound_peers,
            vec![model::ScxmlRemotePeerBinding::new(
                "parent__sce_synth_invoke__inv0",
            )],
            "parent must list synth as outbound peer",
        );
        assert!(
            parent_model.scxml_remote_inbound_peers.is_empty(),
            "synth does not invoke back into parent; inbound must stay empty (got {:?})",
            parent_model.scxml_remote_inbound_peers,
        );

        // Now the synth side. The in-memory submodel contains only
        // `<final>` so it has no invokes of its own. The inbound scan
        // across siblings would miss the parent (parent.scxml still has
        // inline content, no `src=` attribute), so the §9.6.6 rule-3
        // infix inversion is what produces the correct inbound peer set.

        classify_remote_scxml_invokes(&mut synth_model, &cfg, "parent__sce_synth_invoke__inv0");
        collect_scxml_remote_peers(
            &mut synth_model,
            &cfg,
            "parent__sce_synth_invoke__inv0",
            &deploy_path,
        );

        assert!(
            synth_model.scxml_remote_outbound_peers.is_empty(),
            "synth body has no invokes; outbound must be empty (got {:?})",
            synth_model.scxml_remote_outbound_peers,
        );
        assert_eq!(
            synth_model.scxml_remote_inbound_peers,
            vec![model::ScxmlRemotePeerBinding::new("parent")],
            "synth must recognize its parent as an inbound peer via the \
             __sce_synth_invoke__ infix inversion (rule-3 override)",
        );
        assert!(
            synth_model.is_remote_invoke_target,
            "synth with a cross-partition parent must emit WorkerSessionHost",
        );

        let _ = fs::remove_dir_all(&tmp);
    }

    /// §9.6.6 rule 3 default (same partition) — peer-collection layer.
    /// The synth inherits the parent's partition, classifier keeps the
    /// invoke local, and the peer collection must NOT wire the parent
    /// as a mesh inbound peer either. Otherwise the synth would emit a
    /// WorkerSessionHost for a partner that never publishes wire-14,
    /// corrupting the local child-session path shared with W3C
    /// test191/192/253.
    #[test]
    fn synth_invoke_default_does_not_wire_parent_inbound() {
        use std::fs;
        use std::sync::atomic::{AtomicUsize, Ordering};

        static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);
        let tmp = std::env::temp_dir().join(format!(
            "sce_synth_peers_local_{}_{}",
            std::process::id(),
            TEST_COUNTER.fetch_add(1, Ordering::Relaxed),
        ));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        let deploy = r##"version: "1.0"
topology:
  ecu1:
    machines:
      parent:
        source: parent.scxml
partitions:
  p_main:
    machines: [parent]
    contains:
      invokes:
        - { machine: parent, invoke: inv0 }
"##;
        let parent_scxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s1" name="parent">
  <state id="s1">
    <invoke type="scxml" id="inv0">
      <content>
        <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="done">
          <final id="done"/>
        </scxml>
      </content>
    </invoke>
  </state>
</scxml>"##;

        let deploy_path = tmp.join("deploy.yaml");
        fs::write(&deploy_path, deploy).unwrap();
        fs::write(tmp.join("parent.scxml"), parent_scxml).unwrap();

        let parent_model = parser::SCXMLParser::new()
            .parse_file(tmp.join("parent.scxml").to_str().unwrap())
            .expect("parse parent with inline content");

        // §9.6.6: inline `<content>` is captured in-memory on the invoke;
        // the synth submodel is read off the parent model, not re-parsed
        // from disk.
        let mut synth_model = parent_model
            .states
            .get("s1")
            .and_then(|s| s.invokes.first())
            .and_then(|inv| match inv {
                model::Invoke::Scxml(i) => i.inline_child.as_deref().cloned(),
                _ => None,
            })
            .expect("parser must have captured the inline child in-memory");
        assert!(
            !tmp.join("parent__sce_synth_invoke__inv0.scxml").exists(),
            "parser must not write the synth sibling to disk",
        );

        let cfg = mesh::deploy::parse_deploy_str(deploy).expect("deploy must parse");

        classify_remote_scxml_invokes(&mut synth_model, &cfg, "parent__sce_synth_invoke__inv0");
        collect_scxml_remote_peers(
            &mut synth_model,
            &cfg,
            "parent__sce_synth_invoke__inv0",
            &deploy_path,
        );

        assert!(
            synth_model.scxml_remote_inbound_peers.is_empty(),
            "same-partition synth must NOT wire parent as inbound peer; got {:?}",
            synth_model.scxml_remote_inbound_peers,
        );
        assert!(
            !synth_model.is_remote_invoke_target,
            "same-partition synth must keep is_remote_invoke_target=false \
             to preserve the local child-session shape",
        );

        let _ = fs::remove_dir_all(&tmp);
    }

    /// WASM-style parse path (`parse_string`, no `base_dir`) captures
    /// inline `<invoke><content><scxml>` as a structured in-memory
    /// submodel — the historical disk-side-effect skip on `base_dir ==
    /// None` is gone (no asymmetry between filesystem and WASM hosts).
    /// Guards against regression of the WASM enablement that fell out
    /// of the synth-invoke parser purification refactor.
    #[test]
    fn wasm_parse_string_captures_inline_invoke_in_memory() {
        let parent_xml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s1" name="parent">
  <state id="s1">
    <invoke type="scxml" id="inv0">
      <content>
        <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="done">
          <final id="done"/>
        </scxml>
      </content>
    </invoke>
  </state>
</scxml>"##;

        // `parse_string` is the WASM / in-memory entry point; no
        // `base_dir`, no filesystem access. The pre-refactor parser
        // skipped inline extraction entirely in this mode, leaving
        // `child_name` empty.
        let model = parser::SCXMLParser::new()
            .parse_string(parent_xml, "parent")
            .expect("WASM parse must accept inline-invoke parent");

        let invoke = model
            .states
            .get("s1")
            .and_then(|s| s.invokes.first())
            .and_then(|inv| match inv {
                model::Invoke::Scxml(i) => Some(i),
                _ => None,
            })
            .expect("s1 must carry a static Scxml invoke");

        assert_eq!(
            invoke.common.child_name, "parent__sce_synth_invoke__inv0",
            "SCE Mesh §9.6.6 rule 1 synth name must be set without filesystem access",
        );
        assert_eq!(
            invoke.src, "#parent__sce_synth_invoke__inv0",
            "§9.6.6 rule 2: src rewritten to canonical `#<synth>` mesh peer reference",
        );
        let inline = invoke
            .inline_child
            .as_deref()
            .expect("inline child must be parsed in-memory under WASM-style parse_string");
        assert_eq!(
            inline.name, "parent__sce_synth_invoke__inv0",
            "inline child's SCXMLModel.name carries the synth identifier",
        );
        assert!(
            inline.states.contains_key("done"),
            "inline child must preserve its declared states",
        );
        assert!(
            invoke.inline_child_xml.is_some(),
            "raw XML must ride alongside the parsed model so codegen can re-emit to -o",
        );
    }

    // ── SCE_MESH.md §9.6 L1393 cross-device transport validator ──
    //
    // Shared scaffold for the three rejection modes: two machines on
    // separate devices, parent invokes `#worker`, caller supplies a
    // per-target `bindings:` block shape (or none).
    fn setup_cross_device_deployment(
        tmp_subdir: &str,
        parent_bindings: &str,
    ) -> (std::path::PathBuf, std::path::PathBuf) {
        use std::fs;
        use std::sync::atomic::{AtomicUsize, Ordering};

        static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);
        let tmp = std::env::temp_dir().join(format!(
            "sce_crossdev_{}_{}_{}",
            tmp_subdir,
            std::process::id(),
            TEST_COUNTER.fetch_add(1, Ordering::Relaxed),
        ));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        let deploy = format!(
            r##"version: "1.0"
topology:
  ecu_a:
    machines:
      parent:
        source: parent.scxml
{parent_bindings}
  ecu_b:
    machines:
      worker:
        source: worker.scxml
"##
        );
        let parent_scxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s1" name="parent">
  <state id="s1">
    <invoke type="scxml" src="#worker" id="inv0"/>
  </state>
</scxml>"##;
        let worker_scxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="done" name="worker">
  <final id="done"/>
</scxml>"##;

        let deploy_path = tmp.join("deploy.yaml");
        fs::write(&deploy_path, deploy).unwrap();
        fs::write(tmp.join("parent.scxml"), parent_scxml).unwrap();
        fs::write(tmp.join("worker.scxml"), worker_scxml).unwrap();
        (tmp, deploy_path)
    }

    /// Cross-device peer with no `bindings["#worker"]` entry — the
    /// author declared nothing, so the validator cannot know which
    /// transport to target. Rejected with `MissingBinding`.
    #[test]
    fn cross_device_missing_binding_rejected() {
        use std::fs;
        let (tmp, deploy_path) = setup_cross_device_deployment("missing", "");
        let mut model = parser::SCXMLParser::new()
            .parse_file(tmp.join("parent.scxml").to_str().unwrap())
            .expect("parse parent");
        let err = inject_partition_context_for(&mut model, &deploy_path, None)
            .expect_err("cross-device invoke without binding must reject");
        match err {
            mesh::error::MeshError::Deploy(boxed) => match *boxed {
                mesh::error::DeployError::ScxmlInvokeCrossDeviceTransport(payload) => {
                    let mesh::error::ScxmlInvokeCrossDeviceTransportPayload {
                        parent,
                        peer,
                        parent_device,
                        peer_device,
                        failure,
                    } = *payload;
                    assert!(matches!(
                        failure,
                        mesh::error::ScxmlInvokeCrossDeviceFailure::MissingBinding
                    ));
                    assert_eq!(parent, "parent");
                    assert_eq!(peer, "worker");
                    assert_eq!(parent_device, "ecu_a");
                    assert_eq!(peer_device, "ecu_b");
                }
                other => {
                    panic!("expected ScxmlInvokeCrossDeviceTransport/MissingBinding, got {other:?}")
                }
            },
            other => {
                panic!("expected ScxmlInvokeCrossDeviceTransport/MissingBinding, got {other:?}")
            }
        }
        let _ = fs::remove_dir_all(&tmp);
    }

    /// Cross-device peer with `bindings["#worker"].transport: shm` —
    /// shm segments are pid-namespaced and cannot cross a device boundary.
    /// Rejected with `TransportIncapable`.
    #[test]
    fn cross_device_shm_incapable_rejected() {
        use std::fs;
        let bindings = "        bindings:\n          \"#worker\":\n            transport: shm\n";
        let (tmp, deploy_path) = setup_cross_device_deployment("shm", bindings);
        let mut model = parser::SCXMLParser::new()
            .parse_file(tmp.join("parent.scxml").to_str().unwrap())
            .expect("parse parent");
        let err = inject_partition_context_for(&mut model, &deploy_path, None)
            .expect_err("cross-device invoke over shm must reject");
        match err {
            mesh::error::MeshError::Deploy(boxed) => match *boxed {
                mesh::error::DeployError::ScxmlInvokeCrossDeviceTransport(payload) => {
                    let mesh::error::ScxmlInvokeCrossDeviceTransportPayload {
                        peer, failure, ..
                    } = *payload;
                    match failure {
                        mesh::error::ScxmlInvokeCrossDeviceFailure::TransportIncapable {
                            transport,
                        } => {
                            assert_eq!(peer, "worker");
                            assert_eq!(transport, "shm");
                        }
                        other => panic!(
                            "expected ScxmlInvokeCrossDeviceTransport/TransportIncapable, got {other:?}"
                        ),
                    }
                }
                other => panic!(
                    "expected ScxmlInvokeCrossDeviceTransport/TransportIncapable, got {other:?}"
                ),
            },
            other => {
                panic!("expected ScxmlInvokeCrossDeviceTransport/TransportIncapable, got {other:?}")
            }
        }
        let _ = fs::remove_dir_all(&tmp);
    }

    /// Cross-device peer with `bindings["#worker"].transport: zenoh` —
    /// structurally capable transport but the C++ wire-14/20 dispatch
    /// has not landed for it. Rejected with `TransportUnwired`.
    #[test]
    fn cross_device_capable_transport_unwired_rejected() {
        // dds is structurally capable for §9.6 (RequestReply / FireForget /
        // PubSub / FieldAccess in transport.rs) but the codegen template
        // does not yet emit wire-14/20 dispatch for it (transport.rs
        // `implemented: false`). The classifier rejects with
        // `TransportUnwired` so authors see a build-time error instead of
        // a runtime silent fallback. Mirrors the original assertion's
        // intent — formerly aimed at zenoh, now aimed at the next
        // unwired-but-known transport since Session 5 wired zenoh
        // (§9.6 L1393 Session 5).
        use std::fs;
        let bindings = "        bindings:\n          \"#worker\":\n            transport: dds\n";
        let (tmp, deploy_path) = setup_cross_device_deployment("dds", bindings);
        let mut model = parser::SCXMLParser::new()
            .parse_file(tmp.join("parent.scxml").to_str().unwrap())
            .expect("parse parent");
        let err = inject_partition_context_for(&mut model, &deploy_path, None)
            .expect_err("cross-device invoke over dds must reject — wire-14/20 not wired for dds");
        match err {
            mesh::error::MeshError::Deploy(boxed) => match *boxed {
                mesh::error::DeployError::ScxmlInvokeCrossDeviceTransport(payload) => {
                    let mesh::error::ScxmlInvokeCrossDeviceTransportPayload {
                        peer, failure, ..
                    } = *payload;
                    match failure {
                        mesh::error::ScxmlInvokeCrossDeviceFailure::TransportUnwired {
                            transport,
                        } => {
                            assert_eq!(peer, "worker");
                            assert_eq!(transport, "dds");
                        }
                        other => panic!(
                            "expected ScxmlInvokeCrossDeviceTransport/TransportUnwired, got {other:?}"
                        ),
                    }
                }
                other => panic!(
                    "expected ScxmlInvokeCrossDeviceTransport/TransportUnwired, got {other:?}"
                ),
            },
            other => {
                panic!("expected ScxmlInvokeCrossDeviceTransport/TransportUnwired, got {other:?}")
            }
        }
        let _ = fs::remove_dir_all(&tmp);
    }

    /// Build a cross-device deploy.yaml whose two devices optionally
    /// declare their own `transports.custom_tcp.listen:`. Used by the
    /// custom_tcp acceptance / listen-missing suites — the base
    /// `setup_cross_device_deployment` helper hardcodes the topology
    /// shape and cannot thread device-level transport config through.
    fn setup_custom_tcp_deployment(
        tmp_subdir: &str,
        bindings: &str,
        ecu_a_listen: Option<&str>,
        ecu_b_listen: Option<&str>,
    ) -> (std::path::PathBuf, std::path::PathBuf) {
        use std::fs;
        use std::sync::atomic::{AtomicUsize, Ordering};

        static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);
        let tmp = std::env::temp_dir().join(format!(
            "sce_crossdev_tcp_{}_{}_{}",
            tmp_subdir,
            std::process::id(),
            TEST_COUNTER.fetch_add(1, Ordering::Relaxed),
        ));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        let render_transports = |listen: Option<&str>| -> String {
            match listen {
                Some(ep) => {
                    format!("    transports:\n      custom_tcp:\n        listen: \"{ep}\"\n")
                }
                None => String::new(),
            }
        };
        let ecu_a_transports = render_transports(ecu_a_listen);
        let ecu_b_transports = render_transports(ecu_b_listen);

        let deploy = format!(
            r##"version: "1.0"
topology:
  ecu_a:
{ecu_a_transports}    machines:
      parent:
        source: parent.scxml
{bindings}
  ecu_b:
{ecu_b_transports}    machines:
      worker:
        source: worker.scxml
"##
        );
        let parent_scxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s1" name="parent">
  <state id="s1">
    <invoke type="scxml" src="#worker" id="inv0"/>
  </state>
</scxml>"##;
        let worker_scxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="done" name="worker">
  <final id="done"/>
</scxml>"##;

        let deploy_path = tmp.join("deploy.yaml");
        fs::write(&deploy_path, deploy).unwrap();
        fs::write(tmp.join("parent.scxml"), parent_scxml).unwrap();
        fs::write(tmp.join("worker.scxml"), worker_scxml).unwrap();
        (tmp, deploy_path)
    }

    /// Cross-device peer with `bindings["#worker"].transport: someip` —
    /// SCE_MESH.md §9.6 Session 4b: someip is wired for scxml-invoke
    /// cross-device. Unlike custom_tcp, someip has no listen/endpoint
    /// gate on the SCE side (vsomeip.json OEM boundary owns that per
    /// §13). The classifier must accept the binding without reaching
    /// into topology.transports.someip.
    #[test]
    fn cross_device_someip_accepted() {
        use std::fs;
        let bindings = "        bindings:\n          \"#worker\":\n            transport: someip\n";
        let (tmp, deploy_path) = setup_cross_device_deployment("someip", bindings);
        let mut model = parser::SCXMLParser::new()
            .parse_file(tmp.join("parent.scxml").to_str().unwrap())
            .expect("parse parent");
        inject_partition_context_for(&mut model, &deploy_path, None)
            .expect("cross-device someip scxml-invoke must accept — §9.6 Session 4b wired");
        let _ = fs::remove_dir_all(&tmp);
    }

    /// Cross-device peer with `bindings["#worker"].transport: zenoh` —
    /// SCE_MESH.md §9.6 Session 5: zenoh is wired for scxml-invoke
    /// cross-device. Like someip, zenoh has no listen/endpoint gate
    /// on the SCE side at this layer (deploy-time session failures
    /// surface through `zenoh::Session::open` ZException → init()
    /// returning false). The classifier must accept the binding
    /// without reaching into topology.transports.zenoh.
    #[test]
    fn cross_device_zenoh_accepted() {
        use std::fs;
        let bindings = "        bindings:\n          \"#worker\":\n            transport: zenoh\n";
        let (tmp, deploy_path) = setup_cross_device_deployment("zenoh", bindings);
        let mut model = parser::SCXMLParser::new()
            .parse_file(tmp.join("parent.scxml").to_str().unwrap())
            .expect("parse parent");
        inject_partition_context_for(&mut model, &deploy_path, None)
            .expect("cross-device zenoh scxml-invoke must accept — §9.6 Session 5 wired");
        let _ = fs::remove_dir_all(&tmp);
    }

    /// Cross-device peer with `bindings["#worker"].transport: custom_tcp`
    /// and both devices declaring `transports.custom_tcp.listen:` — the
    /// wired path accepts without error.
    #[test]
    fn cross_device_custom_tcp_accepted() {
        use std::fs;
        let bindings =
            "        bindings:\n          \"#worker\":\n            transport: custom_tcp\n";
        let (tmp, deploy_path) = setup_custom_tcp_deployment(
            "accepted",
            bindings,
            Some("127.0.0.1:19200"),
            Some("127.0.0.1:19201"),
        );
        let mut model = parser::SCXMLParser::new()
            .parse_file(tmp.join("parent.scxml").to_str().unwrap())
            .expect("parse parent");
        inject_partition_context_for(&mut model, &deploy_path, None)
            .expect("cross-device custom_tcp with listen on both devices must accept");
        let _ = fs::remove_dir_all(&tmp);
    }

    /// Cross-device peer with `custom_tcp` but the peer's device
    /// omits `transports.custom_tcp.listen:` — wire-14/17/19 has no
    /// inbound channel on the peer side. Rejected with
    /// `TransportListenMissing` identifying the peer device.
    #[test]
    fn cross_device_custom_tcp_peer_listen_missing_rejected() {
        use std::fs;
        let bindings =
            "        bindings:\n          \"#worker\":\n            transport: custom_tcp\n";
        let (tmp, deploy_path) =
            setup_custom_tcp_deployment("peer_missing", bindings, Some("127.0.0.1:19202"), None);
        let mut model = parser::SCXMLParser::new()
            .parse_file(tmp.join("parent.scxml").to_str().unwrap())
            .expect("parse parent");
        let err = inject_partition_context_for(&mut model, &deploy_path, None)
            .expect_err("custom_tcp without peer listen must reject");
        match err {
            mesh::error::MeshError::Deploy(boxed) => match *boxed {
                mesh::error::DeployError::ScxmlInvokeCrossDeviceTransport(payload) => {
                    let mesh::error::ScxmlInvokeCrossDeviceTransportPayload { failure, .. } =
                        *payload;
                    match failure {
                        mesh::error::ScxmlInvokeCrossDeviceFailure::TransportListenMissing {
                            transport,
                            device,
                        } => {
                            assert_eq!(transport, "custom_tcp");
                            assert_eq!(device, "ecu_b");
                        }
                        other => panic!(
                            "expected ScxmlInvokeCrossDeviceTransport/TransportListenMissing, got {other:?}"
                        ),
                    }
                }
                other => panic!(
                    "expected ScxmlInvokeCrossDeviceTransport/TransportListenMissing, got {other:?}"
                ),
            },
            other => panic!(
                "expected ScxmlInvokeCrossDeviceTransport/TransportListenMissing, got {other:?}"
            ),
        }
        let _ = fs::remove_dir_all(&tmp);
    }

    /// Cross-device peer with `custom_tcp` but the parent's own
    /// device omits `transports.custom_tcp.listen:` — wire-15/16/18/20
    /// replies have no inbound channel on the parent side. Rejected
    /// with `TransportListenMissing` identifying the parent device
    /// (checked first in the `[parent, peer]` walk order).
    #[test]
    fn cross_device_custom_tcp_parent_listen_missing_rejected() {
        use std::fs;
        let bindings =
            "        bindings:\n          \"#worker\":\n            transport: custom_tcp\n";
        let (tmp, deploy_path) =
            setup_custom_tcp_deployment("parent_missing", bindings, None, Some("127.0.0.1:19203"));
        let mut model = parser::SCXMLParser::new()
            .parse_file(tmp.join("parent.scxml").to_str().unwrap())
            .expect("parse parent");
        let err = inject_partition_context_for(&mut model, &deploy_path, None)
            .expect_err("custom_tcp without parent listen must reject");
        match err {
            mesh::error::MeshError::Deploy(boxed) => match *boxed {
                mesh::error::DeployError::ScxmlInvokeCrossDeviceTransport(payload) => {
                    let mesh::error::ScxmlInvokeCrossDeviceTransportPayload { failure, .. } =
                        *payload;
                    match failure {
                        mesh::error::ScxmlInvokeCrossDeviceFailure::TransportListenMissing {
                            transport,
                            device,
                        } => {
                            assert_eq!(transport, "custom_tcp");
                            assert_eq!(device, "ecu_a");
                        }
                        other => panic!(
                            "expected ScxmlInvokeCrossDeviceTransport/TransportListenMissing, got {other:?}"
                        ),
                    }
                }
                other => panic!(
                    "expected ScxmlInvokeCrossDeviceTransport/TransportListenMissing, got {other:?}"
                ),
            },
            other => panic!(
                "expected ScxmlInvokeCrossDeviceTransport/TransportListenMissing, got {other:?}"
            ),
        }
        let _ = fs::remove_dir_all(&tmp);
    }

    // ── §5.E B7-α buffer-pool kind ─────────────────────────────

    /// Watching-zenoh RFC §5.E B7-γ: buffer-pool kind happy path.
    /// A well-formed `<sce:kind="buffer-pool">` document → Rust
    /// generator emits a `<Pascal>` struct owning a `[[u8; SLOT_SIZE];
    /// SLOT_COUNT]` storage table + per-slot `slot_states` array +
    /// phantom-typed `Slot<S>` API per spec §5.E lines 1232-1237 +
    /// SECTION / ALIGNMENT / CACHE_POLICY constants. Asserts the
    /// load-bearing tokens so codegen drift fails the build.
    #[test]
    fn buffer_pool_rust_happy_path_emits_storage_struct() {
        let scxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="buffer-pool" name="rx_pool_sram1" version="1.0">
  <sce:slot-count>8</sce:slot-count>
  <sce:slot-size>256</sce:slot-size>
  <sce:section>sram1</sce:section>
  <sce:alignment>32</sce:alignment>
  <sce:dma-channel>DW0_CH3</sce:dma-channel>
  <sce:cache-policy>maintain</sce:cache-policy>
</scxml>"##;
        let label = DocumentLabel {
            identifier: "rx_pool_sram1",
            diagnostic_label: "rx_pool_sram1.scxml",
        };
        let out = compile_forge_from_string(scxml, label, generator::Language::Rust)
            .expect("forge rust codegen must succeed for a well-formed buffer-pool");
        assert_eq!(out.files.len(), 1, "buffer-pool emits a single .rs file");
        let (filename, body) = &out.files[0];
        assert_eq!(filename, "rx_pool_sram1.rs");
        assert!(
            body.contains("pub const SLOT_COUNT: usize = 8;"),
            "SLOT_COUNT constant must lower the slot-count body; full source:\n{body}",
        );
        assert!(
            body.contains("pub const SLOT_SIZE: usize = 256;"),
            "SLOT_SIZE constant must lower the slot-size body; full source:\n{body}",
        );
        assert!(
            body.contains("pub const SECTION: &'static str = \"sram1\";"),
            "SECTION constant must round-trip the section name; full source:\n{body}",
        );
        assert!(
            body.contains("pub const ALIGNMENT: u32 = 32;"),
            "ALIGNMENT constant must lower the alignment body; full source:\n{body}",
        );
        assert!(
            body.contains("pub const DMA_CHANNEL: &'static str = \"DW0_CH3\";"),
            "DMA_CHANNEL constant must round-trip the channel binding; full source:\n{body}",
        );
        assert!(
            body.contains("pub const CACHE_POLICY: &'static str = \"maintain\";"),
            "CACHE_POLICY constant must round-trip the policy enum; full source:\n{body}",
        );
        // γ: STATE_COUNT / TRANSITION_COUNT mirror
        // forge::buffer_pool_fsm constants (7 + 11 per §5.E lines
        // 1129-1135 / 1141-1156). Drift between the IR table and
        // the emitted constants would mean the generator and
        // template disagree on the FSM contract.
        assert!(
            body.contains("pub const STATE_COUNT: usize = 7;"),
            "STATE_COUNT must mirror forge::buffer_pool_fsm::STATE_COUNT; full source:\n{body}",
        );
        assert!(
            body.contains("pub const TRANSITION_COUNT: usize = 11;"),
            "TRANSITION_COUNT must mirror forge::buffer_pool_fsm::TRANSITION_COUNT; full source:\n{body}",
        );
        assert!(
            body.contains("pub struct RxPoolSram1"),
            "must emit pascal-cased struct; full source:\n{body}",
        );
        // γ: phantom-typed API replaces the α bare-`usize` surface.
        assert!(
            body.contains("pub fn pool_acquire_for_encode(&mut self) -> Option<Slot<CpuMut>>"),
            "must emit phantom-typed acquire surface (spec §5.E line 1233); full source:\n{body}",
        );
        assert!(
            body.contains("pub fn link_arm_rx(&mut self) -> Option<Slot<DmaArmedRx>>"),
            "must emit phantom-typed RX-arm surface (spec §5.E line 1236); full source:\n{body}",
        );
        assert!(
            body.contains("impl Slot<CpuMut>"),
            "must emit Slot<CpuMut> impl block; full source:\n{body}",
        );
        assert!(
            body.contains("pub fn link_arm_tx(self, pool: &mut RxPoolSram1)"),
            "must emit consuming link_arm_tx on Slot<CpuMut>; full source:\n{body}",
        );
        assert!(
            body.contains("pub fn pool_return(self, pool: &mut RxPoolSram1)"),
            "must emit consuming pool_return on Slot<CpuMut> (spec §5.E line 1234); full source:\n{body}",
        );
        assert!(
            body.contains("impl Slot<CpuRef>"),
            "must emit Slot<CpuRef> impl block; full source:\n{body}",
        );
        assert!(
            body.contains("pub enum SlotState"),
            "must emit SlotState enum mirroring C11 tag values; full source:\n{body}",
        );
        assert!(
            body.contains("storage: [[0u8; SLOT_SIZE]; SLOT_COUNT]"),
            "must initialize storage as fixed-size array of slot-sized chunks; full source:\n{body}",
        );
        assert!(
            body.contains("slot_states: [SlotState::Free; SLOT_COUNT]"),
            "γ replaces the α `in_use` bitmap with a per-slot SlotState array; full source:\n{body}",
        );
    }

    /// Watching-zenoh RFC §5.E B7-γ: the emitted Rust buffer-pool
    /// module must compile end-to-end as a real Rust source file —
    /// byte assertions alone do not prove the phantom-typed `Slot<S>`
    /// API is well-formed (per `feedback_byte_goldens_not_compile.md`).
    /// Drives the generator output through `rustc --crate-type lib
    /// --emit=metadata` so that any drift in the trait bounds,
    /// PhantomData variance, or `#[must_use]` annotation surfaces
    /// here rather than at integration time.
    #[test]
    fn buffer_pool_rust_emitted_module_compiles_under_rustc() {
        let scxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="buffer-pool" name="rx_pool_sram1" version="1.0">
  <sce:slot-count>4</sce:slot-count>
  <sce:slot-size>64</sce:slot-size>
  <sce:section>sram1</sce:section>
  <sce:alignment>16</sce:alignment>
  <sce:dma-channel>DW0_CH3</sce:dma-channel>
  <sce:cache-policy>maintain</sce:cache-policy>
</scxml>"##;
        let label = DocumentLabel {
            identifier: "rx_pool_sram1",
            diagnostic_label: "rx_pool_sram1.scxml",
        };
        let out = compile_forge_from_string(scxml, label, generator::Language::Rust)
            .expect("forge rust codegen must succeed for a well-formed buffer-pool");
        let (_, body) = &out.files[0];

        let tmp = std::env::temp_dir().join(format!(
            "sce-build-buffer-pool-gamma-rustc-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).expect("create tmp dir");
        let src = tmp.join("rx_pool_sram1.rs");
        std::fs::write(&src, body).expect("write generated source");

        let status = std::process::Command::new("rustc")
            .arg("--edition=2021")
            .arg("--crate-type=lib")
            .arg("--emit=metadata")
            .arg("-o")
            .arg(tmp.join("rx_pool_sram1.rmeta"))
            .arg(&src)
            .arg("--cap-lints")
            .arg("allow")
            .output()
            .expect("rustc must be on PATH for the build environment");

        let _ = std::fs::remove_dir_all(&tmp);

        assert!(
            status.status.success(),
            "generated Rust source must compile under rustc.\nstdout:\n{}\nstderr:\n{}\n--- source ---\n{}",
            String::from_utf8_lossy(&status.stdout),
            String::from_utf8_lossy(&status.stderr),
            body,
        );
    }

    /// Watching-zenoh RFC §5.E / §5.J.4: buffer-pool is the second
    /// `KindClass::McuClass` kind (after Link). Authoring against
    /// cpp/kotlin/go/python raises `codegen/mcu-class-kind-on-non-mcu-language`
    /// via the existing A6 gate. C11 succeeds since B7-β landed the
    /// c11 template (`__attribute__((section, aligned))` storage table +
    /// sidecar linker fragment); the c11 happy-path emission is
    /// pinned by `buffer_pool_c11_happy_path_emits_storage_struct_and_linker_fragment`.
    #[test]
    fn buffer_pool_on_non_mcu_languages_rejects_via_codegen_matrix() {
        use crate::forge::diagnostic::{DiagnosticCode, ToDiagnostics};
        let scxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="buffer-pool" name="rx_pool_sram1" version="1.0">
  <sce:slot-count>8</sce:slot-count>
  <sce:slot-size>256</sce:slot-size>
  <sce:section>sram1</sce:section>
  <sce:alignment>32</sce:alignment>
  <sce:cache-policy>none</sce:cache-policy>
</scxml>"##;
        for lang in [
            generator::Language::Cpp,
            generator::Language::Kotlin,
            generator::Language::Go,
            generator::Language::Python,
        ] {
            let label = DocumentLabel {
                identifier: "rx_pool_sram1",
                diagnostic_label: "rx_pool_sram1.scxml",
            };
            let err = compile_forge_from_string(scxml, label, lang)
                .err()
                .unwrap_or_else(|| panic!("buffer-pool on {lang:?} must reject"));
            let diags = err.to_diagnostics();
            assert_eq!(
                diags.len(),
                1,
                "single diagnostic for MCU-class-on-non-MCU-language; lang={lang:?}",
            );
            let d = &diags[0];
            assert!(
                matches!(d.code, DiagnosticCode::CodegenMcuClassKindOnNonMcuLanguage),
                "must be CodegenMcuClassKindOnNonMcuLanguage; got {:?} on {lang:?}",
                d.code,
            );
        }
        // C11 takes the `KindClass::McuClass` arm and `template_ships`
        // returns true now that B7-β landed the c11 buffer-pool
        // template + linker fragment sidecar — emission succeeds. The
        // load-bearing tokens are asserted in the c11 happy-path test
        // (`buffer_pool_c11_happy_path_emits_storage_struct_and_linker_fragment`);
        // here we only pin that the dispatch reaches `EmitOutcome::Emit`
        // rather than the previous `TemplateMissing` arm.
        let label = DocumentLabel {
            identifier: "rx_pool_sram1",
            diagnostic_label: "rx_pool_sram1.scxml",
        };
        let out = compile_forge_from_string(scxml, label, generator::Language::C11)
            .expect("buffer-pool on c11 must succeed since B7-β landed");
        assert_eq!(
            out.files.len(),
            2,
            "c11 emits header + linker fragment per B7-β contract",
        );
    }

    /// Watching-zenoh RFC §5.E B7-α: η-second-consumer pattern.
    /// `<sce:section>` body must resolve against the deploy-resolved
    /// machine's `memory.sram_regions` map. Compiling a buffer-pool
    /// with `<sce:section>nonexistent</sce:section>` against a machine
    /// declaring `sram1` + `dtcm` raises `mem/pool-section-conflict`
    /// with `Fix::ReplaceOneOf` candidates listing the declared regions.
    /// The new entry [`compile_forge_with_deploy`] is the only path that
    /// fires this diagnostic (Q-η5 (a) precedent — silent skip when
    /// deploy is unavailable).
    #[test]
    fn buffer_pool_section_not_in_deploy_memory_rejects_via_mem_pool_section_conflict() {
        use crate::forge::diagnostic::{DiagnosticCode, ToDiagnostics};
        let deploy_yaml = r#"
version: "1.0"
topology:
  mcu_device:
    machines:
      mcu_node:
        source: pool_owner.scxml
        platform:
          class: mcu
          os: bare_metal
        memory:
          sram_regions:
            sram1:
              base: 0x08000000
              size: 524288
            dtcm:
              base: 0x20000000
              size: 65536
"#;
        let deploy = mesh::deploy::parse_deploy_str(deploy_yaml)
            .expect("deploy.yaml parses with memory.sram_regions");
        let scxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="buffer-pool" name="rx_pool_phantom" version="1.0">
  <sce:slot-count>4</sce:slot-count>
  <sce:slot-size>128</sce:slot-size>
  <sce:section>nonexistent</sce:section>
  <sce:alignment>32</sce:alignment>
  <sce:cache-policy>none</sce:cache-policy>
</scxml>"##;
        let label = DocumentLabel {
            identifier: "rx_pool_phantom",
            diagnostic_label: "rx_pool_phantom.scxml",
        };
        let err = compile_forge_with_deploy(
            scxml,
            label,
            generator::Language::Rust,
            Some(&deploy),
            Some("mcu_node"),
        )
        .err()
        .expect("section not in deploy memory must reject");
        let diags = err.to_diagnostics();
        assert_eq!(
            diags.len(),
            1,
            "single diagnostic for mem-pool-section-conflict",
        );
        let d = &diags[0];
        assert!(
            matches!(d.code, DiagnosticCode::MemPoolSectionConflict),
            "must be DiagnosticCode::MemPoolSectionConflict; got {:?}",
            d.code,
        );
        assert!(
            d.message.contains("nonexistent"),
            "message must name the offending section; got {}",
            d.message,
        );
        assert!(
            d.message.contains("mcu_node"),
            "message must name the target machine; got {}",
            d.message,
        );
        assert!(
            d.message.contains("dtcm") && d.message.contains("sram1"),
            "message must enumerate the declared regions for repair; got {}",
            d.message,
        );

        // Q-η5 (a) skip-when-no-deploy: same SCXML compiled via the
        // deploy-unaware entry passes parse + validate (rust generates
        // the slot table without section validation). Verifying η is
        // layered on top of the existing pipeline rather than added
        // to it.
        let no_deploy_out =
            compile_forge_with_deploy(scxml, label, generator::Language::Rust, None, None)
                .expect("no-deploy path must skip section validation per Q-η5 (a)");
        assert_eq!(no_deploy_out.files.len(), 1);
    }

    /// Watching-zenoh RFC §5.E B7-α: positive case for η-second-consumer.
    /// A pool with `<sce:section>sram1</sce:section>` against a machine
    /// declaring `sram1` in `memory.sram_regions` passes validation and
    /// produces the same Rust output as the deploy-unaware entry. Pins
    /// the validation-then-codegen ordering so a future refactor cannot
    /// accidentally short-circuit the validate step.
    #[test]
    fn buffer_pool_section_in_deploy_memory_passes() {
        let deploy_yaml = r#"
version: "1.0"
topology:
  mcu_device:
    machines:
      mcu_node:
        source: pool_owner.scxml
        platform:
          class: mcu
          os: bare_metal
        memory:
          sram_regions:
            sram1:
              base: 0x08000000
              size: 524288
"#;
        let deploy = mesh::deploy::parse_deploy_str(deploy_yaml).expect("deploy.yaml parses");
        let scxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="buffer-pool" name="rx_pool_sram1" version="1.0">
  <sce:slot-count>8</sce:slot-count>
  <sce:slot-size>256</sce:slot-size>
  <sce:section>sram1</sce:section>
  <sce:alignment>32</sce:alignment>
  <sce:cache-policy>none</sce:cache-policy>
</scxml>"##;
        let label = DocumentLabel {
            identifier: "rx_pool_sram1",
            diagnostic_label: "rx_pool_sram1.scxml",
        };
        let out = compile_forge_with_deploy(
            scxml,
            label,
            generator::Language::Rust,
            Some(&deploy),
            Some("mcu_node"),
        )
        .expect("pool section in deploy memory map must pass validation");
        assert_eq!(out.files.len(), 1);
        let (_, body) = &out.files[0];
        assert!(
            body.contains("pub const SECTION: &'static str = \"sram1\";"),
            "must round-trip the validated section; full source:\n{body}",
        );
    }

    /// Watching-zenoh RFC §5.E B7-α negative coverage. The XSD at
    /// `schemas/sce-forge-ext.xsd` constrains `<sce:cache-policy>` to
    /// the closed enumeration AND `<sce:slot-count>` / `<sce:slot-size>`
    /// / `<sce:alignment>` to `xs:positiveInteger` — so bogus body text
    /// and zero-valued integers are XSD-pre-empted in the default
    /// pipeline (γ precedent: `LinkLinkClassUnknown` is XSD-pre-empted
    /// likewise; the parser-side check is the schema-skipped fallback
    /// pinned by the wire-format golden, not by a live default-pipeline
    /// test). Required-element absence (`<sce:section>` etc.) is enforced
    /// only by the parser because the XSD does not constrain
    /// foreign-namespace `<scxml>` body composition for B7-α.
    #[test]
    fn buffer_pool_parser_negative_coverage() {
        use crate::forge::diagnostic::{DiagnosticCode, ToDiagnostics};
        // Missing <sce:section> — parser-only check (XSD does not
        // enforce required children of foreign-namespace `<scxml>`).
        let no_section = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="buffer-pool" name="bad_pool" version="1.0">
  <sce:slot-count>4</sce:slot-count>
  <sce:slot-size>128</sce:slot-size>
  <sce:alignment>32</sce:alignment>
  <sce:cache-policy>none</sce:cache-policy>
</scxml>"##;
        let label = DocumentLabel {
            identifier: "bad_pool",
            diagnostic_label: "bad_pool.scxml",
        };
        let err = compile_forge_from_string(no_section, label, generator::Language::Rust)
            .err()
            .expect("buffer-pool without <sce:section> must reject");
        let diags = err.to_diagnostics();
        assert!(
            matches!(diags[0].code, DiagnosticCode::ValidationMissingElement),
            "must be ValidationMissingElement for missing section; got {:?}",
            diags[0].code,
        );

        // Bogus <sce:cache-policy> — XSD-pre-empted via cachePolicyType
        // enumeration. γ precedent (LinkLinkClassUnknown) — wire-format
        // for the parser-side InvalidAttribute fallback is locked by
        // golden, not by a live default-pipeline test.
        let bad_policy = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="buffer-pool" name="bad_pool" version="1.0">
  <sce:slot-count>4</sce:slot-count>
  <sce:slot-size>128</sce:slot-size>
  <sce:section>sram1</sce:section>
  <sce:alignment>32</sce:alignment>
  <sce:cache-policy>maybe</sce:cache-policy>
</scxml>"##;
        let err = compile_forge_from_string(bad_policy, label, generator::Language::Rust)
            .err()
            .expect("buffer-pool with invalid cache-policy must reject");
        let diags = err.to_diagnostics();
        assert!(
            matches!(diags[0].code, DiagnosticCode::XmlSchemaValidation),
            "must be XmlSchemaValidation for bogus cache-policy (XSD-pre-empted); got {:?}",
            diags[0].code,
        );

        // Zero-valued slot-size — XSD-pre-empted via xs:positiveInteger
        // (excludes 0). Parser-side `if value == 0` check is the
        // schema-skipped fallback.
        let zero_slot_size = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="buffer-pool" name="bad_pool" version="1.0">
  <sce:slot-count>4</sce:slot-count>
  <sce:slot-size>0</sce:slot-size>
  <sce:section>sram1</sce:section>
  <sce:alignment>32</sce:alignment>
  <sce:cache-policy>none</sce:cache-policy>
</scxml>"##;
        let err = compile_forge_from_string(zero_slot_size, label, generator::Language::Rust)
            .err()
            .expect("buffer-pool with zero slot-size must reject");
        let diags = err.to_diagnostics();
        assert!(
            matches!(diags[0].code, DiagnosticCode::XmlSchemaValidation),
            "must be XmlSchemaValidation for zero slot-size (XSD-pre-empted); got {:?}",
            diags[0].code,
        );
    }

    // ── §5.E B7-β buffer-pool kind c11 parity + linker fragment ─

    /// Watching-zenoh RFC §5.E B7-β: c11 parity for the rust slot
    /// table landed in B7-α. Compiling a well-formed buffer-pool to
    /// `Language::C11` emits a header that places the storage table
    /// in `__attribute__((section(".sram1_<name>"), aligned(32)))` and
    /// pairs it with a sidecar linker fragment carrying the matching
    /// `SECTIONS{}` block. Asserts the load-bearing tokens on both
    /// files so codegen drift fails the build.
    #[test]
    fn buffer_pool_c11_happy_path_emits_storage_struct_and_linker_fragment() {
        let scxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="buffer-pool" name="rx_pool_sram1" version="1.0">
  <sce:slot-count>8</sce:slot-count>
  <sce:slot-size>256</sce:slot-size>
  <sce:section>sram1</sce:section>
  <sce:alignment>32</sce:alignment>
  <sce:dma-channel>DW0_CH3</sce:dma-channel>
  <sce:cache-policy>maintain</sce:cache-policy>
</scxml>"##;
        let label = DocumentLabel {
            identifier: "rx_pool_sram1",
            diagnostic_label: "rx_pool_sram1.scxml",
        };
        let out = compile_forge_from_string(scxml, label, generator::Language::C11)
            .expect("forge c11 codegen must succeed for a well-formed buffer-pool");
        // β emits a (.h, .ld) pair — header first, linker fragment
        // second — per `render_buffer_pool_linker_fragment` push order
        // in `generate_c11_with_imports`.
        assert_eq!(
            out.files.len(),
            2,
            "buffer-pool c11 emits a (.h, .ld) pair; full file list:\n{:?}",
            out.files.iter().map(|(n, _)| n).collect::<Vec<_>>(),
        );
        let header = out
            .files
            .iter()
            .find(|(n, _)| n == "rx_pool_sram1.h")
            .map(|(_, body)| body.as_str())
            .expect("header file must be `rx_pool_sram1.h`");
        let ld = out
            .files
            .iter()
            .find(|(n, _)| n == "rx_pool_sram1_pool.ld")
            .map(|(_, body)| body.as_str())
            .expect("linker fragment must be `rx_pool_sram1_pool.ld`");

        // Header — round-trip macros + storage attribute.
        assert!(
            header.contains("#define RX_POOL_SRAM1_SLOT_COUNT ((size_t)8)"),
            "SLOT_COUNT macro must lower the slot-count body; full header:\n{header}",
        );
        assert!(
            header.contains("#define RX_POOL_SRAM1_SLOT_SIZE ((size_t)256)"),
            "SLOT_SIZE macro must lower the slot-size body; full header:\n{header}",
        );
        assert!(
            header.contains("#define RX_POOL_SRAM1_SECTION \"sram1\""),
            "SECTION macro must round-trip the section name; full header:\n{header}",
        );
        assert!(
            header.contains("#define RX_POOL_SRAM1_ALIGNMENT (32u)"),
            "ALIGNMENT macro must lower the alignment body; full header:\n{header}",
        );
        assert!(
            header.contains("#define RX_POOL_SRAM1_DMA_CHANNEL \"DW0_CH3\""),
            "DMA_CHANNEL macro must round-trip the channel binding; full header:\n{header}",
        );
        assert!(
            header.contains("#define RX_POOL_SRAM1_CACHE_POLICY \"maintain\""),
            "CACHE_POLICY macro must round-trip the policy enum; full header:\n{header}",
        );
        assert!(
            header.contains("__attribute__((section(\".sram1_rx_pool_sram1\"), aligned(32)))"),
            "storage variable must carry section+aligned attribute; full header:\n{header}",
        );
        assert!(
            header.contains("static uint8_t rx_pool_sram1_storage[8][256];"),
            "storage shape must be [SLOT_COUNT][SLOT_SIZE]; full header:\n{header}",
        );
        // γ: STATE_COUNT / TRANSITION_COUNT macros mirror the
        // forge::buffer_pool_fsm IR module (7 + 11 per §5.E lines
        // 1129-1135 / 1141-1156).
        assert!(
            header.contains("#define RX_POOL_SRAM1_STATE_COUNT ((size_t)7)"),
            "STATE_COUNT macro must mirror forge::buffer_pool_fsm::STATE_COUNT; full header:\n{header}",
        );
        assert!(
            header.contains("#define RX_POOL_SRAM1_TRANSITION_COUNT ((size_t)11)"),
            "TRANSITION_COUNT macro must mirror forge::buffer_pool_fsm::TRANSITION_COUNT; full header:\n{header}",
        );
        // ε: the seven-state FSM enum + tag-checked handle struct
        // (spec §5.E lines 1129-1135 / 1239-1242) now live in
        // `<sce/sample.h>` (sce-c-runtime Tier 1 INTERFACE). The
        // per-pool emit pulls them in transitively so downstream code
        // compiling against just `<pool>.h` still sees the canonical
        // typedefs. The runtime header pins the discriminants via
        // `_Static_assert` against this template's spec anchors —
        // anonymous-tag typedef redeclaration is not portable, hence
        // the move to a single source of truth.
        assert!(
            header.contains("#include <sce/sample.h>"),
            "B7-ε integration: pool header must `#include <sce/sample.h>` so \
             the runtime header's seven-state FSM + tag-checked handle + \
             Layer 1 typestate family reach consumer builds; full header:\n{header}",
        );
        assert!(
            header.contains("static sce_slot_state_t rx_pool_sram1_slot_states["),
            "γ replaces the β `in_use[]` bitmap with a slot_states array; full header:\n{header}",
        );
        // γ: tag-checked author API per spec §5.E lines 1232-1242.
        assert!(
            header.contains("static inline sce_slot_handle_t rx_pool_sram1_pool_acquire_for_encode(void)"),
            "must emit pool_acquire_for_encode tag-checked surface (spec §5.E line 1233); full header:\n{header}",
        );
        assert!(
            header.contains("static inline sce_slot_handle_t rx_pool_sram1_link_arm_rx(void)"),
            "must emit link_arm_rx tag-checked surface (spec §5.E line 1236); full header:\n{header}",
        );
        assert!(
            header.contains(
                "static inline bool rx_pool_sram1_link_arm_tx(sce_slot_handle_t *handle)"
            ),
            "must emit tag-checked link_arm_tx (spec §5.E line 1235); full header:\n{header}",
        );
        assert!(
            header.contains(
                "static inline bool rx_pool_sram1_pool_return(sce_slot_handle_t *handle)"
            ),
            "must emit tag-checked pool_return (spec §5.E line 1234); full header:\n{header}",
        );

        // Linker fragment — RFC §5.E lines 1031-1086 contract.
        assert!(
            ld.contains(".sram1_rx_pool_sram1 (NOLOAD) : ALIGN(32)"),
            "SECTIONS entry must carry explicit ALIGN(N) per §5.E lines 1031-1086; full ld:\n{ld}",
        );
        assert!(
            ld.contains("KEEP(*(.sram1_rx_pool_sram1*))"),
            "KEEP must wildcard-pattern the section; full ld:\n{ld}",
        );
        assert!(
            ld.contains("> SRAM1"),
            "MEMORY region directive must uppercase the section name; full ld:\n{ld}",
        );
        assert!(
            ld.contains(". = ALIGN(32);"),
            "inter-pool sentinel must follow the SECTIONS body per §5.E lines 1059-1064; full ld:\n{ld}",
        );
    }

    /// Watching-zenoh RFC §5.E B7-γ: the emitted C11 buffer-pool
    /// header must compile end-to-end as a real C source — byte
    /// assertions alone do not prove the tag-checked handle API is
    /// well-formed (per `feedback_byte_goldens_not_compile.md`).
    /// Drives the generator output through `gcc -c -std=c11
    /// -Wall -Wextra -Werror` so that any drift in the enum
    /// initializer, struct layout, or `static inline` signatures
    /// surfaces here rather than at integration time. The
    /// `__attribute__((section, aligned))` storage variable is
    /// MCU-firmware-targeted; the host gcc compile validates only
    /// the C-language well-formedness, not the linker placement
    /// (the sidecar `.ld` fragment carries that contract). The host
    /// build is permitted to drop the section attribute via
    /// `-D__attribute__(x)=` so unused-static warnings do not gate
    /// the test.
    #[test]
    fn buffer_pool_c11_emitted_header_compiles_under_gcc() {
        let scxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="buffer-pool" name="rx_pool_sram1" version="1.0">
  <sce:slot-count>4</sce:slot-count>
  <sce:slot-size>64</sce:slot-size>
  <sce:section>sram1</sce:section>
  <sce:alignment>16</sce:alignment>
  <sce:dma-channel>DW0_CH3</sce:dma-channel>
  <sce:cache-policy>maintain</sce:cache-policy>
</scxml>"##;
        let label = DocumentLabel {
            identifier: "rx_pool_sram1",
            diagnostic_label: "rx_pool_sram1.scxml",
        };
        let out = compile_forge_from_string(scxml, label, generator::Language::C11)
            .expect("forge c11 codegen must succeed for a well-formed buffer-pool");
        let header = out
            .files
            .iter()
            .find(|(n, _)| n == "rx_pool_sram1.h")
            .map(|(_, body)| body.as_str())
            .expect("header file must be `rx_pool_sram1.h`");

        let tmp = std::env::temp_dir().join(format!(
            "sce-build-buffer-pool-gamma-cc-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).expect("create tmp dir");
        let header_path = tmp.join("rx_pool_sram1.h");
        std::fs::write(&header_path, header).expect("write generated header");
        // The header is `static`-only; #include it from a tiny
        // translation unit so the compile drives every static
        // function. `__used__` would suppress unused warnings on a
        // hypothetical extension, but for B7-γ we just call each
        // entry point so the storage / state arrays count as used.
        let driver_path = tmp.join("driver.c");
        std::fs::write(
            &driver_path,
            r#"#include "rx_pool_sram1.h"

/* C5: cache-policy: maintain emits sce_dcache_*_by_addr calls in
 * link_arm_tx + link_arm_rx. Provide host-side no-op stubs so the
 * gcc link finds the symbols. On bare-metal targets these come from
 * sce_intrinsics_runtime per spec §5.I lines 1707-1711. */
void sce_dcache_clean_by_addr(const void *start, size_t len) {
    (void)start; (void)len;
}
void sce_dcache_invalidate_by_addr(void *start, size_t len) {
    (void)start; (void)len;
}

int main(void) {
    sce_slot_handle_t h = rx_pool_sram1_pool_acquire_for_encode();
    if (h.state == SCE_SLOT_INVALID) return 1;
    uint8_t *w = rx_pool_sram1_slot_write(&h);
    if (w == NULL) return 2;
    w[0] = 0xAB;
    if (!rx_pool_sram1_pool_return(&h)) return 3;

    sce_slot_handle_t r = rx_pool_sram1_link_arm_rx();
    if (r.state != SCE_SLOT_DMA_ARMED_RX) return 4;
    /* Reading or writing a dma-armed-rx slot must be rejected
     * by the runtime tag check. */
    if (rx_pool_sram1_slot_read(&r) != NULL) return 5;
    if (rx_pool_sram1_slot_write(&r) != NULL) return 6;

    /* link_arm_tx on a non-cpu-mut handle is rejected. */
    if (rx_pool_sram1_link_arm_tx(&r)) return 7;

    if (rx_pool_sram1_free_count() != RX_POOL_SRAM1_SLOT_COUNT - 1) return 8;
    return 0;
}
"#,
        )
        .expect("write driver.c");

        let exec_path = tmp.join("driver");
        // B7-ε integration: the generated pool header pulls in
        // `<sce/sample.h>` from `sce-c-runtime/include/`, so the host
        // gcc compile must see that include path alongside the temp
        // dir holding the generated header.
        let crate_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let runtime_include = crate_dir.join("../sce-c-runtime/include");
        let status = std::process::Command::new("gcc")
            .arg("-std=c11")
            .arg("-Wall")
            .arg("-Wextra")
            // The generated header places a `static` storage
            // variable in a section that does not exist on the host
            // toolchain (`.sram1_rx_pool_sram1`). Stripping the
            // attribute keeps the host gcc compile honest about the
            // C-level API surface; the linker placement contract is
            // pinned by the sidecar `.ld` fragment golden test. The
            // same suppression neutralises the `SCE_CONSUMABLE`
            // whole-struct attribute on `sce_sample_t` from
            // `<sce/sample.h>` — the typestate family is silently-
            // inert on host gcc per §5.E lines 1444-1453.
            .arg("-D__attribute__(x)=")
            .arg("-I")
            .arg(&tmp)
            .arg("-I")
            .arg(&runtime_include)
            .arg(&driver_path)
            .arg("-o")
            .arg(&exec_path)
            .output()
            .expect("gcc must be on PATH for the build environment");

        if !status.status.success() {
            let _ = std::fs::remove_dir_all(&tmp);
            panic!(
                "generated C header must compile under gcc.\nstdout:\n{}\nstderr:\n{}\n--- header ---\n{}",
                String::from_utf8_lossy(&status.stdout),
                String::from_utf8_lossy(&status.stderr),
                header,
            );
        }
        // Run the driver — every tag-check and state transition we
        // assert through it is a runtime check on the emitted code.
        let run = std::process::Command::new(&exec_path)
            .output()
            .expect("driver binary must execute");
        let _ = std::fs::remove_dir_all(&tmp);
        assert!(
            run.status.success(),
            "driver run failed: status={:?}, stderr={}",
            run.status.code(),
            String::from_utf8_lossy(&run.stderr),
        );
    }

    /// Watching-zenoh RFC §5.E B7-β: η-third-consumer extension on
    /// [`compile_forge_with_deploy`]. After section validation passes
    /// (B7-α prerequisite gate), the storage footprint must fit the
    /// resolved region's `size`. A pool declaring `slot_count=32` ×
    /// `slot_size=4096` (= 128 KiB) against a region of 64 KiB raises
    /// `mem/pool-too-large` with the bytes_required / region_size
    /// values reflected in the message.
    #[test]
    fn buffer_pool_storage_exceeds_region_size_rejects_via_mem_pool_too_large() {
        use crate::forge::diagnostic::{DiagnosticCode, ToDiagnostics};
        let deploy_yaml = r#"
version: "1.0"
topology:
  mcu_device:
    machines:
      mcu_node:
        source: pool_owner.scxml
        platform:
          class: mcu
          os: bare_metal
        memory:
          sram_regions:
            sram1:
              base: 0x08000000
              size: 65536
"#;
        let deploy = mesh::deploy::parse_deploy_str(deploy_yaml)
            .expect("deploy.yaml parses with sized sram region");
        let scxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="buffer-pool" name="rx_pool_sram1" version="1.0">
  <sce:slot-count>32</sce:slot-count>
  <sce:slot-size>4096</sce:slot-size>
  <sce:section>sram1</sce:section>
  <sce:alignment>32</sce:alignment>
  <sce:cache-policy>none</sce:cache-policy>
</scxml>"##;
        let label = DocumentLabel {
            identifier: "rx_pool_sram1",
            diagnostic_label: "rx_pool_sram1.scxml",
        };
        let err = compile_forge_with_deploy(
            scxml,
            label,
            generator::Language::Rust,
            Some(&deploy),
            Some("mcu_node"),
        )
        .err()
        .expect("storage footprint exceeding region size must reject");
        let diags = err.to_diagnostics();
        assert_eq!(diags.len(), 1, "single diagnostic for mem-pool-too-large");
        let d = &diags[0];
        assert!(
            matches!(d.code, DiagnosticCode::MemPoolTooLarge),
            "must be DiagnosticCode::MemPoolTooLarge; got {:?}",
            d.code,
        );
        assert!(
            d.message.contains("131072"),
            "message must name the bytes_required figure; got {}",
            d.message,
        );
        assert!(
            d.message.contains("65536"),
            "message must name the region_size figure; got {}",
            d.message,
        );
        assert!(
            d.message.contains("rx_pool_sram1") && d.message.contains("sram1"),
            "message must name pool + section for repair; got {}",
            d.message,
        );
    }

    /// Watching-zenoh RFC §5.E B7-β positive path: a pool whose
    /// storage footprint fits the resolved region size passes
    /// validation under [`compile_forge_with_deploy`] and produces
    /// the same (.h, .ld) pair as the deploy-unaware c11 entry.
    /// 8 × 256 = 2 KiB ≤ 64 KiB → pass.
    #[test]
    fn buffer_pool_storage_fits_region_size_passes() {
        let deploy_yaml = r#"
version: "1.0"
topology:
  mcu_device:
    machines:
      mcu_node:
        source: pool_owner.scxml
        platform:
          class: mcu
          os: bare_metal
        memory:
          sram_regions:
            sram1:
              base: 0x08000000
              size: 65536
"#;
        let deploy = mesh::deploy::parse_deploy_str(deploy_yaml).expect("deploy.yaml parses");
        let scxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="buffer-pool" name="rx_pool_sram1" version="1.0">
  <sce:slot-count>8</sce:slot-count>
  <sce:slot-size>256</sce:slot-size>
  <sce:section>sram1</sce:section>
  <sce:alignment>32</sce:alignment>
  <sce:cache-policy>none</sce:cache-policy>
</scxml>"##;
        let label = DocumentLabel {
            identifier: "rx_pool_sram1",
            diagnostic_label: "rx_pool_sram1.scxml",
        };
        let out = compile_forge_with_deploy(
            scxml,
            label,
            generator::Language::C11,
            Some(&deploy),
            Some("mcu_node"),
        )
        .expect("pool footprint fitting region must pass and emit c11 (.h, .ld) pair");
        assert_eq!(out.files.len(), 2, "c11 emits header + linker fragment");
    }

    /// Watching-zenoh RFC §5.E B7-β codegen-invariant force-fixture.
    /// The `mem/inter-pool-padding-not-emitted` self-check inspects
    /// the rendered linker fragment for the `. = ALIGN(N);` sentinel
    /// (§5.E lines 1059-1064). In normal use the template always
    /// emits the sentinel — the diagnostic exists to catch a future
    /// template edit that drops it. This force-fixture drives the
    /// invariant check directly with a synthesized broken fragment
    /// so the diagnostic has a live consumer per
    /// `feedback_silently_broken_hooks.md`. The convert path is
    /// validated end-to-end (ValidationError → Diagnostic →
    /// DiagnosticCode), keeping the normal-render path
    /// performance-free.
    #[test]
    fn buffer_pool_inter_pool_padding_self_check_force_fixture() {
        use crate::forge::diagnostic::{DiagnosticCode, ToDiagnostics};
        use forge::error::{ForgeError, Located, ValidationError};
        // Synthesize the codegen-invariant violation through the
        // ValidationError → diagnostic conversion pipeline rather
        // than through a live render call. The render path always
        // emits the sentinel by template construction; this fixture
        // forces the diagnostic so the wire format and convert path
        // stay byte-stable.
        let err: ForgeError = ValidationError::BufferPoolInterPoolPaddingNotEmitted {
            name: "rx_pool_sram1".into(),
        }
        .into();
        let located: Located<ForgeError> = Located::new(err, "rx_pool_sram1.scxml", None, None);
        let diags = located.to_diagnostics();
        assert_eq!(diags.len(), 1);
        let d = &diags[0];
        assert!(
            matches!(d.code, DiagnosticCode::MemInterPoolPaddingNotEmitted),
            "must be DiagnosticCode::MemInterPoolPaddingNotEmitted; got {:?}",
            d.code,
        );
        assert!(
            d.message.contains("rx_pool_sram1"),
            "message must name the offending pool; got {}",
            d.message,
        );
        assert!(
            d.message.contains(". = ALIGN(N);"),
            "message must name the missing artifact; got {}",
            d.message,
        );
        assert!(
            d.message.contains("§5.E lines 1059-1064"),
            "message must cite the spec anchor; got {}",
            d.message,
        );
    }

    /// Watching-zenoh RFC §5.E B7-ε codegen-invariant force-fixture.
    /// The `pool/sample-typestate-attributes-disabled` self-check
    /// guards the `#include <sce/sample.h>` directive in
    /// `tools/codegen/templates/forge/c/buffer_pool.h.jinja2`. The
    /// runtime header is the producer of the Layer 1 typestate macro
    /// family (`SCE_CONSUMABLE` / `SCE_CALLABLE_WHEN` /
    /// `SCE_SET_TYPESTATE` / `SCE_PARAM_TYPESTATE` / `SCE_WARN_UNUSED`) +
    /// the `sce_sample_t` borrow type; consumer builds compiling
    /// against just the per-pool header inherit those decls only when
    /// the include is present. In normal use the template emits the
    /// include unconditionally — this fixture drives the
    /// ValidationError → Diagnostic → DiagnosticCode pipeline directly
    /// against a synthesized broken-emit scenario so the diagnostic has
    /// a live consumer per `feedback_silently_broken_hooks.md` and the
    /// wire format / convert path stays byte-stable. Mirrors the β
    /// `mem/inter-pool-padding-not-emitted` codegen self-check shape.
    #[test]
    fn buffer_pool_sample_typestate_pull_through_self_check_force_fixture() {
        use crate::forge::diagnostic::{DiagnosticCode, ToDiagnostics};
        use forge::error::{ForgeError, Located, ValidationError};
        let err: ForgeError = ValidationError::BufferPoolSampleTypestateAttributesDisabled {
            name: "rx_pool_sram1".into(),
        }
        .into();
        let located: Located<ForgeError> = Located::new(err, "rx_pool_sram1.scxml", None, None);
        let diags = located.to_diagnostics();
        assert_eq!(diags.len(), 1);
        let d = &diags[0];
        assert!(
            matches!(
                d.code,
                DiagnosticCode::PoolSampleTypestateAttributesDisabled
            ),
            "must be DiagnosticCode::PoolSampleTypestateAttributesDisabled; got {:?}",
            d.code,
        );
        assert!(
            d.message.contains("rx_pool_sram1"),
            "message must name the offending pool; got {}",
            d.message,
        );
        assert!(
            d.message.contains("#include <sce/sample.h>"),
            "message must name the missing artifact; got {}",
            d.message,
        );
        assert!(
            d.message.contains("§5.E lines 1276-1346"),
            "message must cite the spec anchor; got {}",
            d.message,
        );
    }

    /// Locate a Clang ≥ 9 binary on PATH. Layer 1 typestate analysis
    /// requires Clang's `consumable` / `callable_when` /
    /// `param_typestate` / `set_typestate` / `warn_unused_result`
    /// attribute family which GCC does not implement. Returns `None`
    /// when no clang binary is on PATH so build environments without
    /// Clang skip the typestate-axis tests informatively rather than
    /// failing — the gcc compile-check above already covers the
    /// silently-inert path mandated by spec lines 1444-1453.
    #[cfg(test)]
    fn locate_clang_binary() -> Option<String> {
        // `clang` is the canonical name; distros often install only a
        // versioned binary (`clang-19` etc). Try unversioned first then
        // descend through the version range that supports the
        // consumable family (Clang 3.4+, but only Clang 9+ ships both
        // the warn_unused_result combination + thread-safety analysis
        // we depend on per Q-ε1).
        let candidates: &[&str] = &[
            "clang", "clang-19", "clang-18", "clang-17", "clang-16", "clang-15", "clang-14",
            "clang-13", "clang-12", "clang-11", "clang-10", "clang-9",
        ];
        for name in candidates {
            if std::process::Command::new(name)
                .arg("--version")
                .output()
                .ok()
                .is_some_and(|out| out.status.success())
            {
                return Some((*name).to_string());
            }
        }
        None
    }

    /// Watching-zenoh RFC §5.E B7-ε Q-ε7: Clang `-Wconsumed`
    /// `-Wthread-safety` rejects three Layer 1 typestate misuse
    /// patterns against the runtime header
    /// `sce-c-runtime/include/sce/sample.h`:
    ///
    /// 1. **use-after-take** — `sce_sample_payload` (callable_when
    ///    "unconsumed") on a sample whose typestate has already
    ///    transitioned to "consumed" by a prior `sce_sample_take`.
    /// 2. **double-take** — `sce_sample_take` (param_typestate
    ///    "unconsumed") on a sample whose typestate has already been
    ///    consumed by a prior take.
    /// 3. **warn_unused_result ignored** — `sce_sample_take` carries
    ///    `__attribute__((warn_unused_result))`; discarding the
    ///    `sce_result_t` return surfaces under `-Werror=unused-result`.
    ///
    /// All three drivers are compiled with `-Werror=consumed
    /// -Werror=unused-result` so each diagnostic flips to a hard
    /// failure. The test asserts each compilation FAILS — the
    /// diagnostic firing IS the success signal. Skips informatively
    /// on build environments without Clang.
    #[test]
    fn sample_h_layer1_typestate_clang_rejects_misuse() {
        let Some(clang) = locate_clang_binary() else {
            eprintln!(
                "sample_h_layer1_typestate_clang_rejects_misuse: skipped — \
                 no clang binary on PATH; Layer 1 typestate analysis \
                 requires Clang ≥ 9. Spec lines 1444-1453 document the \
                 silently-inert path on non-Clang toolchains; this test \
                 only enforces Clang's rejection contract.",
            );
            return;
        };

        let crate_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let runtime_include = crate_dir.join("../sce-c-runtime/include");

        // Each entry: (case-name, driver source, substring stderr must
        // contain so a diagnostic regression on the wrong axis is
        // visible). All three drivers share the same prologue + struct
        // bodies so `sce_sample_t` can be instantiated locally without
        // a runtime crate link.
        let prologue = r#"#include <sce/sample.h>
struct sce_keyexpr_t { int dummy; };
struct sce_timestamp_t { uint64_t lo, hi; };
"#;
        let cases: &[(&str, &str, &str)] = &[
            (
                "use-after-take",
                r#"
int main(void) {
    sce_sample_t s = (sce_sample_t){ 0 };
    uint8_t buf[16];
    size_t n = 0;
    (void)sce_sample_take(&s, buf, sizeof buf, &n);
    /* Layer 1 violation: payload accessor is callable_when("unconsumed")
     * but the prior take transitioned the sample to "consumed". */
    const uint8_t *p = sce_sample_payload(&s);
    (void)p;
    return 0;
}
"#,
                "consumed",
            ),
            (
                "double-take",
                r#"
int main(void) {
    sce_sample_t s = (sce_sample_t){ 0 };
    uint8_t buf[16];
    size_t n = 0;
    (void)sce_sample_take(&s, buf, sizeof buf, &n);
    /* Layer 1 violation: param_typestate("unconsumed") but the prior
     * take has already transitioned the sample to "consumed". */
    (void)sce_sample_take(&s, buf, sizeof buf, &n);
    return 0;
}
"#,
                "consumed",
            ),
            (
                "warn-unused-result-ignored",
                r#"
int main(void) {
    sce_sample_t s = (sce_sample_t){ 0 };
    uint8_t buf[16];
    size_t n = 0;
    /* warn_unused_result violation: sce_sample_take returns
     * sce_result_t; discarding it forfeits the OK / ERR signal
     * the spec mandates the caller inspect. */
    sce_sample_take(&s, buf, sizeof buf, &n);
    return 0;
}
"#,
                "unused",
            ),
        ];

        for (case, body, stderr_substr) in cases {
            let tmp = std::env::temp_dir().join(format!(
                "sce-build-eps-clang-reject-{}-{}",
                std::process::id(),
                case,
            ));
            let _ = std::fs::remove_dir_all(&tmp);
            std::fs::create_dir_all(&tmp).expect("create tmp dir");
            let driver = tmp.join("driver.c");
            std::fs::write(&driver, format!("{}{}", prologue, body)).expect("write driver.c");

            let out = std::process::Command::new(&clang)
                .arg("-std=c11")
                .arg("-Wall")
                .arg("-Wextra")
                .arg("-Wconsumed")
                .arg("-Wthread-safety")
                .arg("-Werror=consumed")
                .arg("-Werror=unused-result")
                .arg("-c")
                .arg("-I")
                .arg(&runtime_include)
                .arg(&driver)
                .arg("-o")
                .arg(tmp.join("driver.o"))
                .output()
                .expect("clang must execute");

            let stderr = String::from_utf8_lossy(&out.stderr);
            let _ = std::fs::remove_dir_all(&tmp);

            assert!(
                !out.status.success(),
                "clang must REJECT the {case} misuse pattern under \
                 `-Werror=consumed -Werror=unused-result`; compilation \
                 succeeded which means Layer 1 typestate is silently \
                 inert. stderr:\n{stderr}",
            );
            assert!(
                stderr.to_lowercase().contains(stderr_substr),
                "clang's diagnostic for {case} must mention `{stderr_substr}` \
                 so a future regression on the wrong axis is visible. \
                 stderr:\n{stderr}",
            );
        }
    }

    /// Watching-zenoh RFC §5.E B7-ε Q-ε7: the silently-inert axis.
    /// On non-Clang toolchains the Layer 1 attribute family (per
    /// `<sce/sample.h>`) expands to empty per spec lines 1444-1453;
    /// the emitted pool header + transitively pulled-in sample.h must
    /// still compile under host gcc with strict flags including
    /// `-Werror` so a hypothetical extension that emits a stray
    /// warning surfaces. This test pairs with the Clang-axis reject
    /// test above — Clang catches the typestate violations, gcc
    /// merely confirms the host build is clean despite the
    /// silently-inert macros.
    #[test]
    fn buffer_pool_emitted_header_with_sample_pull_through_compiles_silently_under_gcc() {
        let scxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="buffer-pool" name="rx_pool_sram1" version="1.0">
  <sce:slot-count>4</sce:slot-count>
  <sce:slot-size>64</sce:slot-size>
  <sce:section>sram1</sce:section>
  <sce:alignment>16</sce:alignment>
  <sce:cache-policy>none</sce:cache-policy>
</scxml>"##;
        let label = DocumentLabel {
            identifier: "rx_pool_sram1",
            diagnostic_label: "rx_pool_sram1.scxml",
        };
        let out = compile_forge_from_string(scxml, label, generator::Language::C11)
            .expect("forge c11 codegen must succeed for a well-formed buffer-pool");
        let header = out
            .files
            .iter()
            .find(|(n, _)| n == "rx_pool_sram1.h")
            .map(|(_, body)| body.as_str())
            .expect("header file must be `rx_pool_sram1.h`");

        let tmp =
            std::env::temp_dir().join(format!("sce-build-eps-gcc-silent-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).expect("create tmp dir");
        std::fs::write(tmp.join("rx_pool_sram1.h"), header).expect("write generated header");
        // Minimal driver — just `#include "<pool>.h"` to drive the
        // include chain through `<sce/sample.h>`. The `(void)0`
        // statement keeps `main` well-formed; we are not exercising
        // pool functions here (the γ functional test already does).
        std::fs::write(
            tmp.join("driver.c"),
            r#"#include "rx_pool_sram1.h"
struct sce_keyexpr_t { int dummy; };
struct sce_timestamp_t { uint64_t lo, hi; };
int main(void) { (void)0; return 0; }
"#,
        )
        .expect("write driver.c");

        let crate_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let runtime_include = crate_dir.join("../sce-c-runtime/include");
        let out = std::process::Command::new("gcc")
            .arg("-std=c11")
            .arg("-Wall")
            .arg("-Wextra")
            .arg("-Werror")
            // Strip the host-incompatible section attribute on the
            // pool storage table (`.sram1_rx_pool_sram1` does not exist
            // on the host toolchain). The same suppression neutralises
            // any `__attribute__` payload elsewhere in the include
            // chain — Layer 1 attributes are already empty under gcc
            // per the silently-inert path.
            .arg("-D__attribute__(x)=")
            .arg("-I")
            .arg(&tmp)
            .arg("-I")
            .arg(&runtime_include)
            .arg("-c")
            .arg(tmp.join("driver.c"))
            .arg("-o")
            .arg(tmp.join("driver.o"))
            .output()
            .expect("gcc must be on PATH");
        let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
        let success = out.status.success();
        let _ = std::fs::remove_dir_all(&tmp);

        assert!(
            success,
            "buffer_pool.h emission with #include <sce/sample.h> must \
             compile under host gcc -std=c11 -Wall -Wextra -Werror; the \
             silently-inert path means 0 warnings AND clean exit. \
             stdout:\n{stdout}\nstderr:\n{stderr}",
        );
    }

    /// Drift guard: the `tools/codegen/templates/forge/c/
    /// buffer_pool.h.jinja2` template must contain the
    /// `#include <sce/sample.h>` directive that the force-fixture
    /// above's diagnostic guards against. The presence of the include
    /// in the template is the *real* enforcement — the diagnostic only
    /// fires if it goes missing. This test reads the template directly
    /// so a future edit removing the include surfaces here even before
    /// any pool render runs. Pairs with
    /// `buffer_pool_sample_typestate_pull_through_self_check_force_fixture`
    /// which exercises the diagnostic wire format.
    #[test]
    fn buffer_pool_template_pulls_in_sce_sample_h() {
        let crate_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let template_path =
            crate_dir.join("../tools/codegen/templates/forge/c/buffer_pool.h.jinja2");
        let body = std::fs::read_to_string(&template_path).unwrap_or_else(|e| {
            panic!(
                "template {} must exist for B7-ε integration: {e}",
                template_path.display(),
            )
        });
        assert!(
            body.contains("#include <sce/sample.h>"),
            "buffer_pool.h.jinja2 must `#include <sce/sample.h>` so consumer \
             builds inherit the Layer 1 typestate attribute family + \
             sce_sample_t; missing this is the codegen-invariant violation \
             pool/sample-typestate-attributes-disabled guards against \
             (RFC §5.E lines 1276-1346)",
        );
    }

    // ── §5.C / §5.E B6-side schema co-landing ──────────────────

    /// Watching-zenoh RFC §5.C body + §5.E B7-α schema-only: a link
    /// document with `<sce:rx-pool ref="..."/>` / `<sce:tx-pool ref="..."/>`
    /// children parses successfully and emits the pool refs as
    /// `pub const RX_POOL` / `pub const TX_POOL` on the wrapper struct.
    /// This is the B6-side schema-only co-landing — no cross-resolution
    /// validator yet (`link/pool-slot-smaller-than-framer-max` defers
    /// to a later atomic that wires pool ↔ framer through
    /// `compile_forge_with_imports`).
    #[test]
    fn link_with_rx_pool_tx_pool_emits_constants() {
        let scxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="link" name="udp_scout" version="1.0">
  <sce:link-class>udp</sce:link-class>
  <sce:framer ref="scout_frame_codec"/>
  <sce:backpressure>drop</sce:backpressure>
  <sce:rx-pool ref="rx_pool_sram1"/>
  <sce:tx-pool ref="tx_pool_sram1"/>
</scxml>"##;
        let label = DocumentLabel {
            identifier: "udp_scout",
            diagnostic_label: "udp_scout.scxml",
        };
        let out = compile_forge_from_string(scxml, label, generator::Language::Rust)
            .expect("link with rx-pool/tx-pool must parse and emit");
        assert_eq!(out.files.len(), 1);
        let (_, body) = &out.files[0];
        assert!(
            body.contains("pub const RX_POOL: &'static str = \"rx_pool_sram1\";"),
            "RX_POOL constant must lower the rx-pool ref; full source:\n{body}",
        );
        assert!(
            body.contains("pub const TX_POOL: &'static str = \"tx_pool_sram1\";"),
            "TX_POOL constant must lower the tx-pool ref; full source:\n{body}",
        );

        // Without <sce:rx-pool>, the constant must NOT emit (silent
        // omission, not a default value). This pins the {% if %}
        // guard on the template against drift.
        let no_pools = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="link" name="udp_scout" version="1.0">
  <sce:link-class>udp</sce:link-class>
  <sce:framer ref="scout_frame_codec"/>
  <sce:backpressure>drop</sce:backpressure>
</scxml>"##;
        let out = compile_forge_from_string(no_pools, label, generator::Language::Rust)
            .expect("link without pools must still emit");
        let (_, body) = &out.files[0];
        assert!(
            !body.contains("RX_POOL"),
            "RX_POOL must NOT emit when no <sce:rx-pool> declared; full source:\n{body}",
        );
        assert!(
            !body.contains("TX_POOL"),
            "TX_POOL must NOT emit when no <sce:tx-pool> declared; full source:\n{body}",
        );
    }

    /// watching-zenoh RFC §5.E B7-η' Atomic A1: link-side
    /// `<sce:stage-pool ref="X"/>` lowers to a `STAGE_POOL` const on
    /// the Rust side and a `_LINK_STAGE_POOL` macro on the C11 side,
    /// mirroring the `<sce:rx-pool>` / `<sce:tx-pool>` precedent. The
    /// `{% if has_stage_pool %}` guard means absence emits nothing —
    /// the field is link-declared, not link-required, so borrow-only
    /// callbacks (`<sce:on-sample link="X">` consumers that never
    /// call `Sample::take()`) remain valid without any stage-copy
    /// destination.
    #[test]
    fn link_with_stage_pool_emits_constant() {
        let scxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="link" name="udp_scout" version="1.0">
  <sce:link-class>udp</sce:link-class>
  <sce:framer ref="scout_frame_codec"/>
  <sce:backpressure>drop</sce:backpressure>
  <sce:stage-pool ref="scout_stage_pool"/>
</scxml>"##;
        let label = DocumentLabel {
            identifier: "udp_scout",
            diagnostic_label: "udp_scout.scxml",
        };
        let out = compile_forge_from_string(scxml, label, generator::Language::Rust)
            .expect("link with stage-pool must parse and emit");
        let (_, body) = &out.files[0];
        assert!(
            body.contains("pub const STAGE_POOL: &'static str = \"scout_stage_pool\";"),
            "STAGE_POOL constant must lower the stage-pool ref; full source:\n{body}",
        );

        // C11 backend emits the same field as a `_LINK_STAGE_POOL`
        // preprocessor macro — round-tripping via the
        // `link.h.jinja2` template's `{% if has_stage_pool %}` guard.
        let out_c = compile_forge_from_string(scxml, label, generator::Language::C11)
            .expect("c11 backend must accept the same link");
        let (_, body_c) = &out_c.files[0];
        assert!(
            body_c.contains("#define UDP_SCOUT_LINK_STAGE_POOL \"scout_stage_pool\""),
            "C11 backend must emit the STAGE_POOL macro; full source:\n{body_c}",
        );

        // Absence path: no `<sce:stage-pool>` ⇒ no STAGE_POOL anywhere
        // in the generated output. Pins the `{% if %}` guard against
        // template drift that would inject an empty default.
        let no_stage = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="link" name="udp_scout" version="1.0">
  <sce:link-class>udp</sce:link-class>
  <sce:framer ref="scout_frame_codec"/>
  <sce:backpressure>drop</sce:backpressure>
</scxml>"##;
        let out_none = compile_forge_from_string(no_stage, label, generator::Language::Rust)
            .expect("link without stage-pool must still emit");
        let (_, body_none) = &out_none.files[0];
        assert!(
            !body_none.contains("STAGE_POOL"),
            "STAGE_POOL must NOT emit when no <sce:stage-pool>; full source:\n{body_none}",
        );
        let out_c_none = compile_forge_from_string(no_stage, label, generator::Language::C11)
            .expect("c11 backend without stage-pool must still emit");
        let (_, body_c_none) = &out_c_none.files[0];
        assert!(
            !body_c_none.contains("LINK_STAGE_POOL"),
            "C11 LINK_STAGE_POOL must NOT emit absent <sce:stage-pool>; \
             full source:\n{body_c_none}",
        );
    }

    /// RFC §5.C B6-α' cross-resolution fixtures. Three sibling files in
    /// a tempdir — link.scxml + scout_frame_codec.scxml + a buffer-pool —
    /// drive `compile_forge_with_imports` through enrichment so the
    /// link's `<sce:rx-pool>` / `<sce:tx-pool>` ref can be cross-checked
    /// against the framer codec's `codec_max_bytes`.
    fn write_link_pool_fixture_files(
        dir: &std::path::Path,
        framer_byte_count: u32,
        rx_slot_size: u32,
        tx_slot_size: u32,
    ) -> (std::path::PathBuf, &'static str) {
        // Codec body: a single fixed-byte payload field whose width is
        // controlled by `framer_byte_count`. `max_frame_bytes()` lowers
        // to exactly that width — no variant/repeat/tlv-chain bodies in
        // play, so `ImportContext::codec_max_bytes` matches `framer_byte_count`.
        let codec_scxml = format!(
            r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="scout_frame_codec">
  <datamodel>
    <sce:field id="payload" sce:type="bytes" sce:byte="0" sce:bit-size="tail" sce:max-size="{framer_byte_count}"/>
  </datamodel>
</scxml>"##
        );
        std::fs::write(dir.join("scout_frame_codec.scxml"), codec_scxml).unwrap();

        // Two distinct pools so the rx/tx axes can carry different
        // slot-sizes — driver tests parameterise the side under test.
        let rx_pool_scxml = format!(
            r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="buffer-pool" name="rx_pool_sram1">
  <sce:slot-count>4</sce:slot-count>
  <sce:slot-size>{rx_slot_size}</sce:slot-size>
  <sce:section>sram1</sce:section>
  <sce:alignment>32</sce:alignment>
  <sce:cache-policy>none</sce:cache-policy>
</scxml>"##
        );
        std::fs::write(dir.join("rx_pool_sram1.scxml"), rx_pool_scxml).unwrap();

        let tx_pool_scxml = format!(
            r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="buffer-pool" name="tx_pool_sram1">
  <sce:slot-count>4</sce:slot-count>
  <sce:slot-size>{tx_slot_size}</sce:slot-size>
  <sce:section>sram1</sce:section>
  <sce:alignment>32</sce:alignment>
  <sce:cache-policy>none</sce:cache-policy>
</scxml>"##
        );
        std::fs::write(dir.join("tx_pool_sram1.scxml"), tx_pool_scxml).unwrap();

        let link_path = dir.join("udp_scout.scxml");
        let link_body = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="link" name="udp_scout" version="1.0">
  <sce:import as="scout_frame_codec" kind="codec" src="scout_frame_codec.scxml"/>
  <sce:import as="rx_pool_sram1" kind="buffer-pool" src="rx_pool_sram1.scxml"/>
  <sce:import as="tx_pool_sram1" kind="buffer-pool" src="tx_pool_sram1.scxml"/>
  <sce:link-class>udp</sce:link-class>
  <sce:framer ref="scout_frame_codec"/>
  <sce:backpressure>drop</sce:backpressure>
  <sce:rx-pool ref="rx_pool_sram1"/>
  <sce:tx-pool ref="tx_pool_sram1"/>
</scxml>"##;
        std::fs::write(&link_path, link_body).unwrap();
        (link_path, "udp_scout.scxml")
    }

    #[test]
    fn link_with_undersized_rx_pool_rejects_via_link_pool_slot_smaller_than_framer_max() {
        use crate::forge::error::{ForgeError, ValidationError};
        use tempfile::TempDir;

        let tmp = TempDir::new().expect("tempdir");
        // framer worst-case = 32 bytes, rx slot = 16 bytes (undersized),
        // tx slot = 64 bytes (sufficient). The cross-resolver must
        // surface the rx-axis violation first per the iteration order
        // (rx, tx).
        let (link_path, link_label) = write_link_pool_fixture_files(tmp.path(), 32, 16, 64);
        let content = std::fs::read_to_string(&link_path).unwrap();
        let label = DocumentLabel {
            identifier: "udp_scout",
            diagnostic_label: link_label,
        };
        let result = compile_forge_with_imports(
            &content,
            label,
            generator::Language::Rust,
            tmp.path(),
            &ForgeCompileOptions::default(),
        );
        let err = match result {
            Ok(_) => panic!("undersized rx-pool must be rejected at cross-resolution"),
            Err(e) => e,
        };
        match &err.error {
            ForgeError::Validation(boxed) => match boxed.as_ref() {
                ValidationError::LinkPoolSlotSmallerThanFramerMax {
                    link_name,
                    pool_side,
                    pool_alias,
                    pool_slot_size,
                    framer_alias,
                    framer_max_bytes,
                } => {
                    assert_eq!(link_name, "udp_scout");
                    assert_eq!(*pool_side, "rx");
                    assert_eq!(pool_alias, "rx_pool_sram1");
                    assert_eq!(*pool_slot_size, 16);
                    assert_eq!(framer_alias, "scout_frame_codec");
                    assert_eq!(*framer_max_bytes, 32);
                }
                other => panic!("expected LinkPoolSlotSmallerThanFramerMax, got: {other:?}"),
            },
            other => panic!("expected LinkPoolSlotSmallerThanFramerMax, got: {other:?}"),
        }
    }

    #[test]
    fn link_with_undersized_tx_pool_rejects_via_link_pool_slot_smaller_than_framer_max() {
        use crate::forge::error::{ForgeError, ValidationError};
        use tempfile::TempDir;

        let tmp = TempDir::new().expect("tempdir");
        // framer worst-case = 32 bytes, rx slot = 64 bytes (sufficient),
        // tx slot = 8 bytes (undersized). Cross-resolver must reach the
        // tx axis after rx passes.
        let (link_path, link_label) = write_link_pool_fixture_files(tmp.path(), 32, 64, 8);
        let content = std::fs::read_to_string(&link_path).unwrap();
        let label = DocumentLabel {
            identifier: "udp_scout",
            diagnostic_label: link_label,
        };
        let result = compile_forge_with_imports(
            &content,
            label,
            generator::Language::Rust,
            tmp.path(),
            &ForgeCompileOptions::default(),
        );
        let err = match result {
            Ok(_) => panic!("undersized tx-pool must be rejected at cross-resolution"),
            Err(e) => e,
        };
        match &err.error {
            ForgeError::Validation(boxed) => match boxed.as_ref() {
                ValidationError::LinkPoolSlotSmallerThanFramerMax {
                    pool_side,
                    pool_alias,
                    pool_slot_size,
                    framer_max_bytes,
                    ..
                } => {
                    assert_eq!(*pool_side, "tx");
                    assert_eq!(pool_alias, "tx_pool_sram1");
                    assert_eq!(*pool_slot_size, 8);
                    assert_eq!(*framer_max_bytes, 32);
                }
                other => panic!("expected LinkPoolSlotSmallerThanFramerMax, got: {other:?}"),
            },
            other => panic!("expected LinkPoolSlotSmallerThanFramerMax, got: {other:?}"),
        }
    }

    /// Smoke check that the new B7-ε prereq runtime header
    /// `sce-c-runtime/include/sce/sample.h` is well-formed C11. Drives
    /// gcc under `-std=c11 -Wall -Wextra -Werror` against a tiny
    /// translation unit that `#include`s the header and exercises the
    /// `_Static_assert` invariants + macro expansions. The clang-axis
    /// Layer 1 typestate verification (`-Wconsumed -Wthread-safety`
    /// rejecting use-after-take + double-take + callback-leak) is the
    /// B7-ε atomic's responsibility — gated on §5.E codegen integration
    /// landing the `sce_sample_t` consumer surface. This smoke test is
    /// the runtime-header-side prereq's silent-broken-hook guard:
    /// without it, a typo in the macro family or the `_Static_assert`
    /// list would land unobserved until ε's clang test catches it
    /// sessions later.
    #[test]
    fn sce_c_runtime_sample_h_compiles_under_gcc_c11() {
        let crate_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let include_dir = crate_dir.join("../sce-c-runtime/include");
        assert!(
            include_dir.join("sce/sample.h").exists(),
            "expected sce-c-runtime/include/sce/sample.h to exist; \
             searched at {}",
            include_dir.display(),
        );

        let tmp =
            std::env::temp_dir().join(format!("sce-build-sample-h-smoke-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).expect("create tmp dir");

        // The opaque forward-declared structs (`sce_keyexpr_t` /
        // `sce_timestamp_t`) need a complete definition before we can
        // store *pointers* to them in `sce_sample_t` — pointer-to-
        // incomplete is fine for storage, but inspection (e.g. taking
        // the address of an instance) requires a body. Provide trivial
        // bodies in the driver TU so the header's borrow shape is
        // exercised end-to-end.
        let driver_path = tmp.join("driver.c");
        std::fs::write(
            &driver_path,
            r#"#include <sce/sample.h>

struct sce_keyexpr_t { int dummy; };
struct sce_timestamp_t { uint64_t lo, hi; };

/* Drive the macro family + struct definition + function declarations.
 * No call to sce_sample_take / sce_sample_payload — they are extern
 * decls; this TU compiles to .o without a link. */
static const sce_sub_callback_t cb = (sce_sub_callback_t)0;

int main(void) {
    sce_sample_t s = (sce_sample_t){ 0 };
    /* Touch every field so a typo in sample.h's struct lowering would
     * surface as a missing-member error. */
    (void)s.key_expr;
    (void)s.payload;
    (void)s.payload_len;
    (void)s.timestamp;
    (void)s._slot.state;
    (void)s._slot.idx;
    (void)cb;
    /* SCE_OWNERSHIP_ATTRS_AVAILABLE evaluates to a literal 0 or 1 at
     * preprocess time on every supported toolchain; assert it's
     * one of those two so a malformed detection block surfaces. */
    _Static_assert(SCE_OWNERSHIP_ATTRS_AVAILABLE == 0
                   || SCE_OWNERSHIP_ATTRS_AVAILABLE == 1,
                   "SCE_OWNERSHIP_ATTRS_AVAILABLE must be 0 or 1");
    return 0;
}
"#,
        )
        .expect("write driver.c");

        let exec_path = tmp.join("driver");
        let mut cmd = std::process::Command::new("gcc");
        cmd.arg("-std=c11")
            .arg("-Wall")
            .arg("-Wextra")
            .arg("-Werror")
            // `__has_attribute(consumable)` is false on host gcc; the
            // header's Q-ε4 `#warning` (Clang-detected + attributes
            // unavailable) only fires under Clang, so gcc compiles
            // cleanly. The empty-macro path is the silently-inert
            // Layer 1 surface the spec calls out at lines 1444-1453.
            .arg("-I")
            .arg(&include_dir)
            .arg(&driver_path)
            .arg("-o")
            .arg(&exec_path);
        let status = cmd
            .output()
            .expect("gcc must be on PATH for the build environment");

        if !status.status.success() {
            let _ = std::fs::remove_dir_all(&tmp);
            panic!(
                "sce/sample.h must compile cleanly under gcc -std=c11 \
                 -Wall -Wextra -Werror.\nstdout:\n{}\nstderr:\n{}",
                String::from_utf8_lossy(&status.stdout),
                String::from_utf8_lossy(&status.stderr),
            );
        }
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn link_with_sufficient_pool_slots_passes_cross_resolution() {
        use tempfile::TempDir;

        let tmp = TempDir::new().expect("tempdir");
        // framer worst-case = 32 bytes, both pools >= 32 bytes — the
        // boundary case (slot_size == framer_max_bytes) must pass.
        let (link_path, link_label) = write_link_pool_fixture_files(tmp.path(), 32, 32, 64);
        let content = std::fs::read_to_string(&link_path).unwrap();
        let label = DocumentLabel {
            identifier: "udp_scout",
            diagnostic_label: link_label,
        };
        let out = compile_forge_with_imports(
            &content,
            label,
            generator::Language::Rust,
            tmp.path(),
            &ForgeCompileOptions::default(),
        )
        .expect("sufficient pools must pass cross-resolution and emit");
        // udp_scout.rs is the link's own emit; the codec/pool imports
        // each emit as siblings via cross-file generation.
        let names: Vec<&str> = out.files.iter().map(|(n, _)| n.as_str()).collect();
        assert!(
            names.iter().any(|n| n.contains("udp_scout")),
            "link emit missing from cross-file output: {names:?}",
        );
    }
}

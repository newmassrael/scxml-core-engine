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

pub mod model;
pub mod parser;
pub mod analyzer;
pub mod script_engine_analyzer;
pub mod filters;
pub mod generator;
pub mod kotlin;
pub mod lua_transformer;
pub mod conformance;
pub mod forge;
pub mod mesh;
pub mod cli_error;
pub mod w3c_dist_manifest;
#[cfg(not(target_arch = "wasm32"))]
pub mod formatter;
#[cfg(feature = "wasm")]
mod wasm;

use model::SCXMLModel;
use std::path::Path;

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
        Self { identifier: label, diagnostic_label: label }
    }
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
fn compile_model(scxml_path: &str) -> Result<SCXMLModel, CompileError> {
    let mut parser = parser::SCXMLParser::new();
    let mut model = parser.parse_file(scxml_path)?;
    analyzer::analyze(&mut model, scxml_path);
    guard_static_generatable(&model, scxml_path)?;
    resolve_source_path(&mut model, scxml_path);
    Ok(model)
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
    Ok(model)
}

/// Typed error channel for the two `compile_model*` helpers.
pub type CompileError = forge::error::Located<forge::error::ForgeError>;

/// Promote the `analyzer::can_generate_static` precondition into a
/// typed `ValidationError::DynamicFeatures` diagnostic. The `reason`
/// the analyzer returned flows through verbatim so the wire record
/// names the exact blocker.
fn guard_static_generatable(model: &SCXMLModel, source_name: &str) -> Result<(), CompileError> {
    use forge::error::{Located, ValidationError};
    analyzer::can_generate_static(model).map_err(|reason| {
        Located::new(
            ValidationError::DynamicFeatures {
                name: model.name.clone(),
                reason: reason.to_string(),
            }
            .into(),
            source_name,
            None,
            None,
        )
    })
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

/// Compile SCXML files to Rust source code in `OUT_DIR`.
///
/// Generates `{name}_sm.rs` for each input SCXML file.
/// Intended for use in `build.rs`.
pub fn compile_scxml(scxml_files: &[&str]) {
    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR not set (must be called from build.rs)");
    let template_dir = find_template_dir();

    for scxml_path in scxml_files {
        let code = compile_scxml_to_string(scxml_path, &template_dir)
            .unwrap_or_else(|e| panic!("Failed to compile {scxml_path}: {e}"));

        let stem = Path::new(scxml_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .expect("Invalid SCXML filename");

        let out_path = Path::new(&out_dir).join(format!("{stem}_sm.rs"));
        std::fs::write(&out_path, &code)
            .unwrap_or_else(|e| panic!("Cannot write {}: {e}", out_path.display()));

        println!("cargo::rerun-if-changed={scxml_path}");
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
    let model = compile_model(scxml_path)?;
    generator::generate(&model, template_dir)
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
    generator::generate_with_templates(&model, templates)
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

    match language {
        generator::Language::Rust => {
            let code = generator::generate_with_templates(&model, templates)
                .map_err(|e| locate_codegen_error(e, scxml_name))?;
            Ok(generator::GeneratedOutput {
                files: vec![(format!("{scxml_name}_sm.rs"), code)],
            })
        }
        generator::Language::Cpp => {
            generator::generate_cpp_with_templates(&model, templates, scxml_name)
                .map_err(|e| locate_codegen_error(e, scxml_name))
        }
        generator::Language::Kotlin => {
            let code = generator::generate_kotlin_with_templates(&model, templates)
                .map_err(|e| locate_codegen_error(e, scxml_name))?;
            Ok(generator::GeneratedOutput {
                files: vec![(format!("{scxml_name}Sm.kt"), code)],
            })
        }
        generator::Language::Go => {
            let code = generator::generate_go_with_templates(&model, templates)
                .map_err(|e| locate_codegen_error(e, scxml_name))?;
            Ok(generator::GeneratedOutput {
                files: vec![(format!("{scxml_name}_sm.go"), code)],
            })
        }
        generator::Language::Python => Err(locate_codegen_error(
            // Python statechart codegen is an un-implemented target,
            // not a per-document configuration error — but at the
            // library boundary the caller opted into an unsupported
            // combination, so it lands here with a clear message.
            forge::error::GenerateError::InvalidConfig(
                "Python statechart codegen is not yet supported".into(),
            ),
            scxml_name,
        )),
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
    let model = compile_model(scxml_path)?;

    let input_stem = Path::new(scxml_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown");

    match language {
        generator::Language::Rust => {
            let code = generator::generate(&model, template_dir)
                .map_err(|e| locate_codegen_error(e, scxml_path))?;
            Ok(generator::GeneratedOutput {
                files: vec![(format!("{input_stem}_sm.rs"), code)],
            })
        }
        generator::Language::Cpp => generator::generate_cpp(&model, template_dir, input_stem)
            .map_err(|e| locate_codegen_error(e, scxml_path)),
        generator::Language::Kotlin => {
            let code = generator::generate_kotlin(&model, template_dir)
                .map_err(|e| locate_codegen_error(e, scxml_path))?;
            Ok(generator::GeneratedOutput {
                files: vec![(format!("{input_stem}Sm.kt"), code)],
            })
        }
        generator::Language::Go => {
            let code = generator::generate_go(&model, template_dir)
                .map_err(|e| locate_codegen_error(e, scxml_path))?;
            Ok(generator::GeneratedOutput {
                files: vec![(format!("{input_stem}_sm.go"), code)],
            })
        }
        generator::Language::Python => Err(locate_codegen_error(
            forge::error::GenerateError::InvalidConfig(
                "Python statechart codegen is not yet supported".into(),
            ),
            scxml_path,
        )),
    }
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
        generator::Language::Cpp => "",       // C++ templates at root
        generator::Language::Kotlin => "kotlin",
        generator::Language::Go => "go",
        generator::Language::Python => "python",
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

    let doc = forge::parser::parse_forge(content, label)?
        .ok_or_else(|| Located::new(
            ValidationError::WrongPipeline {
                kind: forge::model::ForgeKind::Statechart,
            }
            .into(),
            label.diagnostic_label,
            None,
            None,
        ))?;

    let template_base = find_template_base();

    let output = match language {
        generator::Language::Cpp => forge::generator::generate_cpp(&doc, &template_base),
        generator::Language::Kotlin => forge::generator::generate_kotlin(&doc, &template_base),
        generator::Language::Rust => forge::generator::generate_rust(&doc, &template_base),
        generator::Language::Go => forge::generator::generate_go(&doc, &template_base),
        generator::Language::Python => forge::generator::generate_python(&doc, &template_base),
    }
    .map_err(|e| Located::new(e, label.diagnostic_label, None, None))?;
    Ok(output)
}

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

    let parsed = forge::parser::parse_forge_with_imports(content, label)?
        .ok_or_else(|| Located::new(
            ValidationError::WrongPipeline {
                kind: forge::model::ForgeKind::Statechart,
            }
            .into(),
            label.diagnostic_label,
            None,
            None,
        ))?;

    let template_base = find_template_base();
    let mut import_ctx = forge::generator::resolve_imports(&parsed.imports, &language, options)
        .map_err(|e| Located::new(e, label.diagnostic_label, None, None))?;

    validate_and_enrich_imports(
        &mut import_ctx,
        &parsed.imports,
        base_dir,
        &language,
        label.diagnostic_label,
    )?;

    let output = match language {
        generator::Language::Cpp => {
            forge::generator::generate_cpp_with_imports(&parsed.document, &template_base, &import_ctx)
        }
        generator::Language::Kotlin => {
            forge::generator::generate_kotlin_with_imports(&parsed.document, &template_base, &import_ctx)
        }
        generator::Language::Rust => {
            forge::generator::generate_rust_with_imports(&parsed.document, &template_base, &import_ctx)
        }
        generator::Language::Go => {
            forge::generator::generate_go_with_imports(&parsed.document, &template_base, &import_ctx)
        }
        generator::Language::Python => {
            forge::generator::generate_python_with_imports(&parsed.document, &template_base, &import_ctx)
        }
    }
    .map_err(|e| Located::new(e, label.diagnostic_label, None, None))?;
    Ok(output)
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

        if let Some(doc) = forge::parser::parse_forge(&content, imported_label)? {
            if !ctx.is_stateful {
                if let Some(name) = discover_primary_function(&doc, language) {
                    ctx.qualified_call = build_qualified_call(&name, &ctx.namespace, language);
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
        ForgeDocument::Lookup(m) => {
            (vec![m.input.sce_type.clone()], Some(m.output.sce_type.clone()))
        }
        ForgeDocument::Interpolation(_) => {
            // Interpolation takes a typed input (x, or x+y for 2D) and returns
            // float64. Without opening up the Interpolation model further, we
            // treat parameters as empty (opaque) and return Float64.
            (Vec::new(), Some(SceType::Float64))
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
        // Stateless kinds handled via stateless_signature path.
        ForgeDocument::Transform(_)
        | ForgeDocument::Condition(_)
        | ForgeDocument::Lookup(_)
        | ForgeDocument::Interpolation(_) => {}
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
/// Currently only Codec kinds expose instance methods (`encode()` → `Bytes`).
/// Future stateful kinds (e.g., Filter::update, Observer::update) can be
/// added here as the pipeline discovers them.
fn discover_stateful_member_methods(
    doc: &forge::model::ForgeDocument,
) -> Vec<(String, Vec<forge::model::SceType>, forge::model::SceType)> {
    use forge::model::{ForgeDocument, SceType};
    match doc {
        ForgeDocument::Codec(_) => vec![("encode".to_string(), vec![], SceType::Bytes)],
        _ => Vec::new(),
    }
}

/// Discover the primary function name generated by a stateless forge document.
fn discover_primary_function(
    doc: &forge::model::ForgeDocument,
    language: &generator::Language,
) -> Option<String> {
    match doc {
        forge::model::ForgeDocument::Transform(m) => {
            let output_id = m.outputs.first()?.id.clone();
            Some(match language {
                generator::Language::Cpp | generator::Language::Kotlin => {
                    format!("compute{}", filters::to_pascal_case(output_id))
                }
                generator::Language::Rust | generator::Language::Python => {
                    format!("compute_{}", filters::to_snake_case(output_id))
                }
                generator::Language::Go => {
                    format!("Compute{}", filters::to_pascal_case(output_id))
                }
            })
        }
        forge::model::ForgeDocument::Condition(m) => {
            Some(match language {
                generator::Language::Cpp | generator::Language::Kotlin => {
                    filters::to_camel_case(m.name.clone())
                }
                generator::Language::Rust | generator::Language::Python => {
                    filters::to_snake_case(m.name.clone())
                }
                generator::Language::Go => {
                    filters::to_pascal_case(m.name.clone())
                }
            })
        }
        forge::model::ForgeDocument::Lookup(m) => {
            Some(match language {
                generator::Language::Cpp | generator::Language::Kotlin => {
                    format!("lookup{}", filters::to_pascal_case(m.output.id.clone()))
                }
                generator::Language::Rust | generator::Language::Python => {
                    format!("lookup_{}", filters::to_snake_case(m.output.id.clone()))
                }
                generator::Language::Go => {
                    format!("Lookup{}", filters::to_pascal_case(m.output.id.clone()))
                }
            })
        }
        forge::model::ForgeDocument::Interpolation(_) => {
            Some(match language {
                generator::Language::Cpp | generator::Language::Kotlin => {
                    "lookup".to_string()
                }
                generator::Language::Rust | generator::Language::Python => {
                    "lookup".to_string()
                }
                generator::Language::Go => {
                    "Lookup".to_string()
                }
            })
        }
        // Stateful kinds (Codec, Validator, Procedure, Filter, Observer, Timer)
        // use member access, not free function calls. They are handled by the
        // member rename mechanism in procedure and validator render functions.
        forge::model::ForgeDocument::Codec(_)
        | forge::model::ForgeDocument::Validator(_)
        | forge::model::ForgeDocument::Procedure(_)
        | forge::model::ForgeDocument::Filter(_)
        | forge::model::ForgeDocument::Observer(_)
        | forge::model::ForgeDocument::Timer(_) => None,
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
    }
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
///   1. Parse deploy.yaml (device-shared `transports:` and per-target
///      `bindings:` both validated at this stage; invalid values like
///      `mode: pier` are rejected here, before any topology work).
///   1b. Resolve external infrastructure config (SCE_MESH.md §13):
///       load each device's vsomeip.json and resolve name-based binding
///       references into numeric IDs before topology runs. Reserved
///       SOME/IP ID key names in deploy.yaml are hard errors here.
///   2. Collect <send> targets from the model (single pass)
///   2a. Emit targetexpr warnings (dynamic targets cannot be statically resolved)
///   2b. Resolve targets against deploy.yaml bindings
///   2c. Pattern capability validation — architectural: is the bound transport
///       even capable of the requested communication pattern? (e.g. zenoh
///       cannot do request/reply). Runs BEFORE event coverage because a
///       transport mismatch is a deploy.yaml design error.
///   2d. Event coverage validation — implementation: does the receiver have
///       a matching <transition> for every sent event?
///   3. Transport codegen (template rendering). Device-shared session config
///      is read directly from `DeployConfig` (no extraction/merging step —
///      the schema makes shared config explicit).
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
                return Err(mesh::error::MeshError::Deploy(
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
                .map(|v| v.iter().filter(|e| e.machine == resolved_name).map(|e| &e.parallel).collect())
                .unwrap_or_default();

            // Regions of `resolved_name` hosted by ANY partition —
            // used to detect whether a `<parallel>` is distributed.
            let mut parallel_partitions: std::collections::BTreeMap<&String, std::collections::BTreeSet<&String>> =
                std::collections::BTreeMap::new();
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

            for (parallel_id, _) in &model.parallel_regions {
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
                            model.partition_wire21_outbound_routes.insert(
                                parallel_id.clone(),
                                (*root_part).clone(),
                            );
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

    Ok(present)
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
    // Outbound: walk the just-enriched local invokes.
    let mut outbound: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for state in model.states.values() {
        for invoke in &state.invokes {
            if let model::Invoke::Scxml(info) = invoke {
                if let Some(target) = &info.remote_mesh_target {
                    outbound.insert(target.clone());
                }
            }
        }
    }
    model.scxml_remote_outbound_peers = outbound.into_iter().collect();

    // Inbound: scan every sibling machine's SCXML for
    // `<invoke type="scxml" src="#<this>">`. The deploy.yaml `source:` field
    // is relative to the deploy.yaml's own directory, per the existing
    // machine-source resolution convention.
    let deploy_dir = deploy_path.parent().unwrap_or_else(|| Path::new("."));
    let self_marker = format!("#{resolved_name}");
    let mut inbound: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();

    // Regex matches a whole `<invoke ...>` / `<invoke .../>` open tag as one
    // capture, then the same pattern's `src="#<name>"` capture locates the
    // target machine name. We filter by checking the tag's `type=`
    // attribute: absent, empty, "scxml", or the W3C default URI all pass;
    // anything else (sce:mesh-rpc, custom URIs) is excluded. One regex
    // traversal per file; no full XML parse.
    let _ = self_marker;  // reserved for future diagnostics; fast-scan below handles the inclusion check
    let invoke_tag_re = regex::Regex::new(r##"<invoke\b[^>]*>"##).expect("valid regex");
    let src_attr_re = regex::Regex::new(r##"\bsrc="#([A-Za-z_][A-Za-z0-9_]*)""##).expect("valid regex");
    let type_attr_re = regex::Regex::new(r##"\btype="([^"]*)""##).expect("valid regex");

    for device in deploy_cfg.topology.values() {
        for (peer_name, peer_cfg) in &device.machines {
            if peer_name.as_str() == resolved_name {
                continue;  // skip self — outbound list already covers it
            }
            let peer_scxml_path = deploy_dir.join(&peer_cfg.source);
            let content = match std::fs::read_to_string(&peer_scxml_path) {
                Ok(c) => c,
                Err(_) => continue,  // unreadable — fail-silent (see header)
            };
            for tag in invoke_tag_re.find_iter(&content) {
                let tag_text = tag.as_str();
                let type_ok = match type_attr_re.captures(tag_text) {
                    Some(c) => {
                        let t = &c[1];
                        t.is_empty() || t == "scxml" || t == "http://www.w3.org/TR/scxml/"
                    }
                    None => true,  // no type attr — W3C default "scxml"
                };
                if !type_ok {
                    continue;
                }
                if let Some(cap) = src_attr_re.captures(tag_text) {
                    if &cap[1] == resolved_name {
                        inbound.insert(peer_name.clone());
                    }
                }
            }
        }
    }
    model.scxml_remote_inbound_peers = inbound.into_iter().collect();
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
///   (parser resolves inline `<content>` into this shape — §9.6.6 mesh
///   synthesis is still Session F scope).
/// - Self-references (`#<own machine>`) — these would always fail at
///   build time, but classification here is defensive against author typos.
/// - Unknown `#<name>` that is not a deploy-declared machine — the build
///   remains a local W3C invoke and the existing "child SCXML not found"
///   path reports it.
fn classify_remote_scxml_invokes(
    model: &mut SCXMLModel,
    deploy_cfg: &mesh::deploy::DeployConfig,
    resolved_name: &str,
) {
    let mut mutated = false;
    for state in model.states.values_mut() {
        for invoke in state.invokes.iter_mut() {
            if let model::Invoke::Scxml(info) = invoke {
                let Some(target) = info.src.strip_prefix('#') else {
                    continue;
                };
                if target == resolved_name {
                    continue;
                }
                if deploy_cfg.device_for_machine(target).is_some() {
                    info.remote_mesh_target = Some(target.to_string());
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
    if server_pairs.is_empty()
        && field_access_pairs.is_empty()
        && eventgroup_events.is_empty()
    {
        return Ok(vec![]);
    }

    let response_events: std::collections::HashSet<String> = server_pairs
        .iter()
        .map(|p| p.response_event.clone())
        .chain(field_access_pairs.iter().map(|p| p.response_event.clone()))
        .chain(eventgroup_events.iter().map(|eg| eg.event.clone()))
        .collect();
    Ok(mesh::topology::inject_server_response_sends(model, &response_events))
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

    let external_resolution =
        mesh::external::resolve_external_bindings(&deploy_cfg, deploy_dir)?;

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
    let server_fire_forget_events =
        mesh::topology::detect_server_fire_forget_events(model);
    let server_field_access_pairs =
        mesh::topology::detect_server_field_access_pairs(model);
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
        .chain(
            server_eventgroup_events
                .iter()
                .map(|eg| eg.event.clone()),
        )
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
            summary.target_events.retain(|(t, e)| {
                !(t == &self_tid && server_response_events.contains(e))
            });
            summary.actions.retain(|a| {
                !(a.target == self_tid && server_response_events.contains(&a.event))
            });
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
        .map(|m| m.subscriptions.as_slice())
        .unwrap_or(&[]);

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

    if resolved.is_empty() && server_binding.is_none() && !has_custom_tcp_listen && !has_wire21_routing && !has_scxml_remote_wire {
        let _ = external_resolution; // no bindings → no resolved IDs to consume
        return Ok(MeshResult {
            output: generator::GeneratedOutput { files: vec![] },
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
        let uncovered =
            mesh::topology::check_sender_event_coverage(&model.name, &summary, &receiver_models, &deploy_cfg);
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
        .map(mesh::deploy::MachineConfig::resolved_ordering_timings)
        .unwrap_or_else(mesh::deploy::OrderingTimings::default_const);
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
    let output = mesh::codegen::generate_mesh(
        &model.name,
        &resolved,
        server_binding.as_ref(),
        zenoh_session,
        someip_config,
        custom_tcp_config,
        machine_subscriptions,
        machine_ordering,
        machine_liveliness,
        machine_outbound_buffer,
        partition_self_name.as_deref(),
        &partition_wire21_outbound,
        &partition_wire21_inbound,
        &scxml_remote_outbound_peers,
        &scxml_remote_inbound_peers,
        language,
        &template_base,
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
        Err(ForgeError::Validation(ValidationError::UnsupportedKind(_))) => Pipeline::Forge,
        // XML was not parseable; intent unknowable — defer to SCXML.
        Err(_) => Pipeline::Scxml,
    }
}

/// Locate the base template directory (contains rust/, kotlin/, actions/).
pub fn find_template_base() -> std::path::PathBuf {
    if let Ok(dir) = std::env::var("SCE_TEMPLATE_DIR") {
        // If pointing to a language subdir, go up one level
        let p = Path::new(&dir);
        if p.join("state_machine.rs.jinja2").exists()
            || p.join("state_machine.kt.jinja2").exists()
        {
            return p.parent().unwrap_or(p).to_path_buf();
        }
        return p.to_path_buf();
    }
    // Release builds refuse silent fallback: `CARGO_MANIFEST_DIR` is
    // baked at compile time and may point at a stale source tree on
    // install targets, producing silently-wrong output. Dev/test
    // builds keep the fallback so `cargo test` and in-repo `cargo run`
    // work without explicit setup.
    #[cfg(debug_assertions)]
    {
        let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let candidate = crate_dir.join("../tools/codegen/templates");
        if candidate.exists() {
            return candidate;
        }
        let candidate = Path::new("tools/codegen/templates");
        if candidate.exists() {
            return candidate.to_path_buf();
        }
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
        use forge::error::{ForgeError, ValidationError};
        // `initial="nope"` names a non-existent state — analyzer
        // rejects with DynamicFeatures carrying the specific blocker.
        let scxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="nope" name="typed_probe">
    <state id="s1"/>
</scxml>"##;
        let err = compile_from_string_typed(scxml, "typed_probe", &[])
            .expect_err("initial points at undeclared state must reject");
        assert!(
            matches!(
                err.error,
                ForgeError::Validation(ValidationError::DynamicFeatures { ref name, .. })
                    if name == "typed_probe",
            ),
            "expected ValidationError::DynamicFeatures(name=\"typed_probe\"), got: {:?}",
            err.error,
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

        let injected = inject_server_model_mutations(&mut model, &tmp.join("deploy.yaml"))
            .expect("inject");

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

    /// Local inline-content invoke resolves to a relative file path
    /// (`parent_child0.scxml` per `extract_inline_child_name`), never a
    /// `#<name>` form. Classifier must leave it alone so W3C test191/192/
    /// 253 keep going through the local W3C invoke path.
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
            "local inline-content invoke must not be flagged; got {:?}",
            info.remote_mesh_target
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
}

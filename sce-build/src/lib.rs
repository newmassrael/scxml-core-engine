// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
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

pub mod diagnostics;
pub mod model;
pub mod parser;
pub mod analyzer;
pub mod filters;
pub mod generator;
pub mod kotlin;
pub mod lua_transformer;
pub mod conformance;
pub mod forge;
pub mod mesh;
#[cfg(not(target_arch = "wasm32"))]
pub mod formatter;
#[cfg(feature = "wasm")]
mod wasm;

use model::SCXMLModel;
use std::path::Path;

/// Parse, analyze, and validate an SCXML file for static code generation.
/// SCE Forge inline kinds are extracted during parsing (single XML pass).
fn compile_model(scxml_path: &str) -> Result<SCXMLModel, String> {
    let mut parser = parser::SCXMLParser::new();
    let mut model = parser.parse_file(scxml_path)?;
    analyzer::analyze(&mut model, scxml_path);
    if !analyzer::can_generate_static(&model) {
        return Err(format!(
            "Cannot generate static code for '{}' (dynamic features not supported)",
            model.name
        ));
    }
    resolve_source_path(&mut model, scxml_path);
    Ok(model)
}

/// Parse SCXML content string, analyze and validate (no filesystem).
/// SCE Forge inline kinds are extracted during parsing (single XML pass).
fn compile_model_from_string(scxml_content: &str, scxml_name: &str) -> Result<SCXMLModel, String> {
    let mut parser = parser::SCXMLParser::new();
    let mut model = parser.parse_string(scxml_content, scxml_name)?;
    analyzer::analyze(&mut model, "");
    if !analyzer::can_generate_static(&model) {
        return Err(format!(
            "Cannot generate static code for '{}' (dynamic features not supported)",
            model.name
        ));
    }
    Ok(model)
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

/// Compile a single SCXML file to Rust source code string.
///
/// This is the core API — parses SCXML, analyzes model, renders templates.
pub fn compile_scxml_to_string(scxml_path: &str, template_dir: &Path) -> Result<String, String> {
    let model = compile_model(scxml_path)?;
    generator::generate(&model, template_dir).map_err(|e| e.to_string())
}

/// Compile SCXML content string to Rust code (no filesystem access).
///
/// This is the WASM-compatible API. Templates must be provided as (name, content) pairs.
pub fn compile_from_string(
    scxml_content: &str,
    scxml_name: &str,
    templates: &[(&str, &str)],
) -> Result<String, String> {
    let model = compile_model_from_string(scxml_content, scxml_name)?;
    generator::generate_with_templates(&model, templates).map_err(|e| e.to_string())
}

/// Compile SCXML content string for a specific language (WASM-compatible).
pub fn compile_from_string_lang(
    scxml_content: &str,
    scxml_name: &str,
    templates: &[(&str, &str)],
    language: generator::Language,
) -> Result<generator::GeneratedOutput, String> {
    let model = compile_model_from_string(scxml_content, scxml_name)?;

    match language {
        generator::Language::Rust => {
            let code = generator::generate_with_templates(&model, templates)
                .map_err(|e| e.to_string())?;
            Ok(generator::GeneratedOutput {
                files: vec![(format!("{scxml_name}_sm.rs"), code)],
            })
        }
        generator::Language::Cpp => {
            generator::generate_cpp_with_templates(&model, templates, scxml_name)
                .map_err(|e| e.to_string())
        }
        generator::Language::Kotlin => {
            let code = generator::generate_kotlin_with_templates(&model, templates)
                .map_err(|e| e.to_string())?;
            Ok(generator::GeneratedOutput {
                files: vec![(format!("{scxml_name}Sm.kt"), code)],
            })
        }
        generator::Language::Go => {
            let code = generator::generate_go_with_templates(&model, templates)
                .map_err(|e| e.to_string())?;
            Ok(generator::GeneratedOutput {
                files: vec![(format!("{scxml_name}_sm.go"), code)],
            })
        }
        generator::Language::Python => Err(
            "Python statechart codegen is not yet supported".to_string(),
        ),
    }
}

/// Compile SCXML file for a specific language (filesystem-based).
pub fn compile_scxml_lang(
    scxml_path: &str,
    template_dir: &Path,
    language: generator::Language,
) -> Result<generator::GeneratedOutput, String> {
    let model = compile_model(scxml_path)?;

    let input_stem = Path::new(scxml_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown");

    match language {
        generator::Language::Rust => {
            let code = generator::generate(&model, template_dir)
                .map_err(|e| e.to_string())?;
            Ok(generator::GeneratedOutput {
                files: vec![(format!("{input_stem}_sm.rs"), code)],
            })
        }
        generator::Language::Cpp => generator::generate_cpp(&model, template_dir, input_stem)
            .map_err(|e| e.to_string()),
        generator::Language::Kotlin => {
            let code = generator::generate_kotlin(&model, template_dir)
                .map_err(|e| e.to_string())?;
            Ok(generator::GeneratedOutput {
                files: vec![(format!("{input_stem}Sm.kt"), code)],
            })
        }
        generator::Language::Go => {
            let code = generator::generate_go(&model, template_dir)
                .map_err(|e| e.to_string())?;
            Ok(generator::GeneratedOutput {
                files: vec![(format!("{input_stem}_sm.go"), code)],
            })
        }
        generator::Language::Python => Err(
            "Python statechart codegen is not yet supported".to_string(),
        ),
    }
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
/// error contract — every failure ties back to the `name` the caller
/// supplied, so downstream consumers (CLI diagnostics, build scripts,
/// agents) never have to attach file context after the fact.
pub fn compile_forge_from_string(
    content: &str,
    name: &str,
    language: generator::Language,
) -> Result<generator::GeneratedOutput, forge::error::Located<forge::error::ForgeError>> {
    use forge::error::{Located, ValidationError};

    let doc = forge::parser::parse_forge(content, name)?
        .ok_or_else(|| Located::new(
            ValidationError::WrongPipeline {
                kind: forge::model::ForgeKind::Statechart,
            }
            .into(),
            name,
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
    .map_err(|e| Located::new(e, name, None, None))?;
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
    name: &str,
    language: generator::Language,
    base_dir: &Path,
    options: &ForgeCompileOptions,
) -> Result<generator::GeneratedOutput, forge::error::Located<forge::error::ForgeError>> {
    use forge::error::{Located, ValidationError};

    let parsed = forge::parser::parse_forge_with_imports(content, name)?
        .ok_or_else(|| Located::new(
            ValidationError::WrongPipeline {
                kind: forge::model::ForgeKind::Statechart,
            }
            .into(),
            name,
            None,
            None,
        ))?;

    let template_base = find_template_base();
    let mut import_ctx = forge::generator::resolve_imports(&parsed.imports, &language, options)
        .map_err(|e| Located::new(e, name, None, None))?;

    validate_and_enrich_imports(&mut import_ctx, &parsed.imports, base_dir, &language)?;

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
    .map_err(|e| Located::new(e, name, None, None))?;
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
            return Err(Located::new(
                ImportError::KindMismatch {
                    src: imp.src.clone(),
                    declared: imp.kind.to_string(),
                    actual: actual_kind.to_string(),
                }
                .into(),
                &src_label,
                None,
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

        if let Some(doc) = forge::parser::parse_forge(&content, stem)? {
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
        .map_err(|e| forge::error::Located::new(e, dir.display().to_string(), None, None))
}

/// Result of a mesh transport compilation — generated files plus all
/// build-time warnings collected during the pipeline.
pub struct MeshResult {
    /// Generated transport routing files.
    pub output: generator::GeneratedOutput,
    /// Dynamic target warnings (targetexpr cannot be statically resolved).
    pub dynamic_target_warnings: Vec<mesh::topology::TopologyWarning>,
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
/// Returns `Ok(MeshResult)` with generated files and all warnings,
/// or `Err(MeshError)` on hard failure.
pub fn compile_mesh_transport(
    model: &SCXMLModel,
    deploy_path: &Path,
    language: generator::Language,
) -> Result<MeshResult, mesh::error::MeshError> {
    // Stage 1: deploy.yaml parsing (typed session config validated by serde)
    let deploy_cfg = mesh::deploy::parse_deploy(deploy_path)?;

    // Stage 1b: resolve external infrastructure config (vsomeip.json) —
    // produces a typed `ExternalResolution` map consumed by topology. The
    // deploy config itself is treated read-only from here on.
    let deploy_dir = deploy_path.parent().unwrap_or(Path::new("."));
    let external_resolution =
        mesh::external::resolve_external_bindings(&deploy_cfg, deploy_dir)?;

    // Stage 2: single-pass send action collection
    let summary = mesh::topology::collect_send_summary(model);

    // Stage 2a: dynamic target warnings (from summary)
    let dynamic_target_warnings = summary.dynamic_warnings.clone();

    // Stage 2b: resolve static targets against deploy.yaml bindings,
    // attach per-event SOME/IP IDs, and validate per-event field presence
    // in a single pipeline — callers cannot observe the half-built state
    // between resolution and attach.
    let resolved = mesh::topology::build_resolved_targets(
        &summary,
        &deploy_cfg,
        &model.name,
        &external_resolution,
    )?;

    if resolved.is_empty() {
        let _ = external_resolution; // no bindings → no resolved IDs to consume
        return Ok(MeshResult {
            output: generator::GeneratedOutput { files: vec![] },
            dynamic_target_warnings,
        });
    }

    // Stage 2c: pattern capability validation — architectural check.
    // A transport that lacks a required capability is a design error.
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

    // Stage 3: transport codegen. Device-shared transport configs are read
    // directly from the device's `transports:` block; no merging/validation
    // pass is needed because the schema makes shared config structurally
    // singular (one entry per transport type per device).
    let device = deploy_cfg.device_for_machine(&model.name);
    let zenoh_session = device.and_then(|d| d.transports.zenoh.as_ref());
    let someip_config = device.and_then(|d| d.transports.someip.as_ref());
    let template_base = find_template_base();
    let output = mesh::codegen::generate_mesh(
        &model.name,
        &resolved,
        zenoh_session,
        someip_config,
        language,
        &template_base,
    )?;
    Ok(MeshResult {
        output,
        dynamic_target_warnings,
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
/// through the SCXML parser would mis-label the `stage` field as
/// `cli/scxml-parse` when the truth is an `sce:kind` violation caught by
/// the Forge XSD. Routing such documents to [`Self::Forge`] lets the
/// forge pipeline emit the honest stage (`xml/schema-validation` when
/// the bundled XSD is reachable, or `validation/unsupported-kind` when
/// the schemas were not vendored) — either of which is strictly more
/// actionable than `cli/scxml-parse`.
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
    let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let candidate = crate_dir.join("../tools/codegen/templates");
    if candidate.exists() {
        return candidate;
    }
    let candidate = Path::new("tools/codegen/templates");
    if candidate.exists() {
        return candidate.to_path_buf();
    }
    panic!("Cannot find Jinja2 templates. Set SCE_TEMPLATE_DIR or run from project root.");
}

/// Locate the Rust Jinja2 template directory.
///
/// Delegates to `find_template_dir_for(Language::Rust)` for consistent behavior.
pub fn find_template_dir() -> std::path::PathBuf {
    find_template_dir_for(generator::Language::Rust)
}

// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
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
pub mod filters;
pub mod generator;
pub mod kotlin;
pub mod lua_transformer;
pub mod forge;
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
    generator::generate(&model, template_dir)
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
    generator::generate_with_templates(&model, templates)
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
            let code = generator::generate_with_templates(&model, templates)?;
            Ok(generator::GeneratedOutput {
                files: vec![(format!("{scxml_name}_sm.rs"), code)],
            })
        }
        generator::Language::Cpp => {
            generator::generate_cpp_with_templates(&model, templates, scxml_name)
        }
        generator::Language::Kotlin => {
            let code = generator::generate_kotlin_with_templates(&model, templates)?;
            Ok(generator::GeneratedOutput {
                files: vec![(format!("{scxml_name}Sm.kt"), code)],
            })
        }
        generator::Language::Go => {
            let code = generator::generate_go_with_templates(&model, templates)?;
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
            let code = generator::generate(&model, template_dir)?;
            Ok(generator::GeneratedOutput {
                files: vec![(format!("{input_stem}_sm.rs"), code)],
            })
        }
        generator::Language::Cpp => generator::generate_cpp(&model, template_dir, input_stem),
        generator::Language::Kotlin => {
            let code = generator::generate_kotlin(&model, template_dir)?;
            Ok(generator::GeneratedOutput {
                files: vec![(format!("{input_stem}Sm.kt"), code)],
            })
        }
        generator::Language::Go => {
            let code = generator::generate_go(&model, template_dir)?;
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
/// Phase 1 supports C++ only.
pub fn compile_forge_from_string(
    content: &str,
    name: &str,
    language: generator::Language,
) -> Result<generator::GeneratedOutput, String> {
    let doc = forge::parser::parse_forge(content, name)?
        .ok_or("Not a forge document (statechart or no sce:kind)")?;

    let template_base = find_template_base();

    match language {
        generator::Language::Cpp => forge::generator::generate_cpp(&doc, &template_base),
        generator::Language::Kotlin => forge::generator::generate_kotlin(&doc, &template_base),
        generator::Language::Rust => forge::generator::generate_rust(&doc, &template_base),
        generator::Language::Go => forge::generator::generate_go(&doc, &template_base),
        generator::Language::Python => forge::generator::generate_python(&doc, &template_base),
    }
}

/// Compile a forge SCXML with cross-file import resolution.
///
/// Uses `parse_forge_with_imports` to extract `<sce:import>` declarations,
/// resolves them to per-language import contexts, and passes them to templates.
/// Import validation (file existence, kind matching) is skipped — use
/// `compile_forge_with_imports_validated` for full validation.
pub fn compile_forge_with_imports(
    content: &str,
    name: &str,
    language: generator::Language,
) -> Result<generator::GeneratedOutput, String> {
    compile_forge_with_imports_impl(content, name, language, None)
}

/// Compile a forge SCXML with cross-file import validation.
///
/// When `base_dir` is provided, validates each `<sce:import>`:
/// 1. The referenced `src` file exists relative to `base_dir`
/// 2. The file is a valid forge document
/// 3. The declared `kind` matches the actual kind in the file
pub fn compile_forge_with_imports_validated(
    content: &str,
    name: &str,
    language: generator::Language,
    base_dir: &Path,
) -> Result<generator::GeneratedOutput, String> {
    compile_forge_with_imports_impl(content, name, language, Some(base_dir))
}

fn compile_forge_with_imports_impl(
    content: &str,
    name: &str,
    language: generator::Language,
    base_dir: Option<&Path>,
) -> Result<generator::GeneratedOutput, String> {
    let parsed = forge::parser::parse_forge_with_imports(content, name)?
        .ok_or("Not a forge document (statechart or no sce:kind)")?;

    let template_base = find_template_base();
    let mut import_ctx = forge::generator::resolve_imports(&parsed.imports, &language);

    // Validate and enrich imports in a single pass (one file read per import)
    if let Some(dir) = base_dir {
        validate_and_enrich_imports(&mut import_ctx, &parsed.imports, dir, &language)?;
    }

    match language {
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
) -> Result<(), String> {
    for (ctx, imp) in import_ctx.iter_mut().zip(imports.iter()) {
        let src_path = base_dir.join(&imp.src);

        // 1. Existence
        if !src_path.exists() {
            return Err(format!(
                "<sce:import src=\"{}\">: file not found (searched: {})",
                imp.src,
                src_path.display()
            ));
        }

        // Read once
        let content = std::fs::read_to_string(&src_path)
            .map_err(|e| format!("<sce:import src=\"{}\">: cannot read: {e}", imp.src))?;

        // 2. Kind validation
        let actual_kind = forge::parser::detect_kind(&content)?
            .ok_or_else(|| {
                format!(
                    "<sce:import src=\"{}\">: not a forge document (no sce:kind)",
                    imp.src
                )
            })?;

        if actual_kind != imp.kind {
            return Err(format!(
                "<sce:import src=\"{}\" kind=\"{}\">: actual kind is '{}' (mismatch)",
                imp.src, imp.kind, actual_kind
            ));
        }

        // 3. Stateless API enrichment (reuse already-read content)
        if !ctx.is_stateful {
            let stem = src_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown");

            if let Some(doc) = forge::parser::parse_forge(&content, stem)? {
                if let Some(name) = discover_primary_function(&doc, language) {
                    ctx.qualified_call = build_qualified_call(&name, &ctx.namespace, language);
                }
            }
        }
    }
    Ok(())
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
        // Stateful kinds (Codec, Validator, Procedure) use member access,
        // not free function calls. They are handled by the member rename
        // mechanism in procedure_l2 and validator render functions.
        forge::model::ForgeDocument::Codec(_)
        | forge::model::ForgeDocument::Validator(_)
        | forge::model::ForgeDocument::Procedure(_) => None,
    }
}

/// Build a language-specific qualified function call from function name + namespace.
fn build_qualified_call(
    func_name: &str,
    namespace: &str,
    language: &generator::Language,
) -> String {
    match language {
        generator::Language::Cpp => format!("{namespace}::{func_name}"),
        generator::Language::Kotlin => func_name.to_string(), // Same package
        generator::Language::Rust => format!("{namespace}::{func_name}"),
        generator::Language::Go => format!("{namespace}.{func_name}"),
        generator::Language::Python => func_name.to_string(), // Direct import
    }
}

/// Build a forge dependency manifest from a directory of SCXML files.
///
/// Scans `dir` for `.scxml` files, extracts `sce:kind` and `<sce:import>`,
/// and produces a JSON-serializable manifest with topological build order.
pub fn build_forge_manifest(dir: &std::path::Path) -> Result<forge::model::ForgeManifest, String> {
    forge::manifest::build_manifest(dir)
}

/// Detect if an SCXML file uses a non-statechart `sce:kind`.
pub fn is_forge_document(content: &str) -> bool {
    forge::parser::detect_kind(content)
        .ok()
        .flatten()
        .map_or(false, |k| k != forge::model::ForgeKind::Statechart)
}

/// Locate the base template directory (contains rust/, kotlin/, actions/).
fn find_template_base() -> std::path::PathBuf {
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

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
    }
}

/// Locate template directory for a specific language.
pub fn find_template_dir_for(language: generator::Language) -> std::path::PathBuf {
    let subdir = match language {
        generator::Language::Rust => "rust",
        generator::Language::Cpp => "",       // C++ templates at root
        generator::Language::Kotlin => "kotlin",
        generator::Language::Go => "go",
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
        _ => Err(format!(
            "Forge codegen for {:?} is planned for Phase 2",
            language
        )),
    }
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

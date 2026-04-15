// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// Multi-language code generator — renders minijinja templates from SCXMLModel.
// Supports Rust, C++, and Kotlin code generation.

use crate::filters;
use crate::forge::error::GenerateError;
use crate::model::SCXMLModel;
use minijinja::Environment;
use std::path::Path;

/// Create a minijinja Environment with Python Jinja2 compatibility enabled.
pub(crate) fn new_env<'a>() -> Environment<'a> {
    let mut env = Environment::new();
    env.set_trim_blocks(true);
    env.set_lstrip_blocks(true);
    // Python Jinja2 compatibility:
    // 1. dict.items(), str.strip(), str.startswith(), etc.
    env.set_unknown_method_callback(minijinja_contrib::pycompat::unknown_method_callback);
    // 2. Undefined attributes propagate as undefined (Chainable) instead of
    //    silently becoming "" (Lenient). This catches template typos while still
    //    allowing optional attribute chains like `model.foo.bar` to work.
    env.set_undefined_behavior(minijinja::UndefinedBehavior::Chainable);
    env
}

/// Target language for code generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    Rust,
    Cpp,
    Kotlin,
    Go,
    Python,
}

impl std::str::FromStr for Language {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "rust" => Ok(Language::Rust),
            "cpp" | "c++" => Ok(Language::Cpp),
            "kotlin" | "kt" => Ok(Language::Kotlin),
            "go" | "golang" => Ok(Language::Go),
            "python" | "py" => Ok(Language::Python),
            _ => Err(format!("Unknown language: {s}")),
        }
    }
}

/// Generated output — may contain multiple files (e.g., C++ .h + .inl).
pub struct GeneratedOutput {
    pub files: Vec<(String, String)>, // (filename, content)
}

/// License configuration matching Python license_config.py
fn license_config() -> serde_json::Value {
    serde_json::json!({
        "project": {
            "name": "SCE (SCXML Core Engine)",
            "copyright_year": "2025-2026",
            "copyright_holder": "newmassrael"
        },
        "urls": {
            "license_main": "https://github.com/newmassrael/scxml-core-engine/blob/main/LICENSE"
        },
        "generated_code_header": {
            "copyright_holder": "[Author of input SCXML file]"
        }
    })
}

pub(crate) fn render_error(e: minijinja::Error) -> GenerateError {
    use std::error::Error;
    let mut msg = format!("Template render error: {e}");
    let mut source: Option<&dyn Error> = e.source();
    while let Some(cause) = source {
        msg.push_str(&format!("\n  caused by: {cause}"));
        source = cause.source();
    }
    if let Some(detail) = e.detail() {
        msg.push_str(&format!("\n  detail: {detail}"));
    }
    GenerateError::TemplateRender(msg)
}

// ── Mesh-rpc backend gate ────────────────────────────────────────
//
// SCE_MESH.md §9.5 (`<invoke type="sce:mesh-rpc">`) only has codegen
// emission in the C++ mesh templates today. Other backends parse the
// invoke happily (the parser is language-agnostic) but produce no
// transport routing for it — the state's onentry would silently
// ignore the invoke at runtime, which is exactly the "fail clearly"
// inversion CLAUDE.md forbids. This helper turns the silent skip
// into an explicit codegen-time refusal so an operator who picks
// the wrong `--lang` sees the gap immediately, not at runtime.
fn reject_mesh_rpc_in_unsupported_lang(
    model: &SCXMLModel,
    language: &'static str,
) -> Result<(), GenerateError> {
    if !model.has_mesh_rpc_invoke() {
        return Ok(());
    }
    Err(GenerateError::UnsupportedFeature(format!(
        "<invoke type=\"sce:mesh-rpc\"> in '{}' has no {} codegen path \
         (mesh transport emission is currently C++-only). \
         Either generate this machine for `--lang cpp` or remove the \
         mesh-rpc invokes from the SCXML.",
        model.name, language
    )))
}

// ── Rust generator ───────────────────────────────────────────────

/// Generate Rust code from an analyzed SCXMLModel (filesystem-based).
pub fn generate(model: &SCXMLModel, template_dir: &Path) -> Result<String, GenerateError> {
    reject_mesh_rpc_in_unsupported_lang(model, "Rust")?;
    let mut env = new_env();
    load_templates(&mut env, template_dir)?;
    filters::register_filters(&mut env);
    render_rust(&env, model)
}

/// Generate Rust code using pre-loaded template strings (WASM-compatible).
pub fn generate_with_templates(
    model: &SCXMLModel,
    templates: &[(&str, &str)],
) -> Result<String, GenerateError> {
    reject_mesh_rpc_in_unsupported_lang(model, "Rust")?;
    let mut env = new_env();
    load_template_strings(&mut env, templates)?;
    filters::register_filters(&mut env);
    render_rust(&env, model)
}

fn render_rust(env: &Environment, model: &SCXMLModel) -> Result<String, GenerateError> {
    let machine_name = filters::to_pascal_case(model.name.clone());

    // SCE Forge: render inline kind declarations as Rust code fragments.
    let (inline_kind_types, inline_kind_fns) = if !model.inline_kinds.is_empty() {
        let code = crate::forge::generator::render_inline_kinds(
            &model.inline_kinds,
            Language::Rust,
            &machine_name,
        )
        .map_err(|e| GenerateError::TemplateRender(e.to_string()))?;
        (code.type_defs, code.member_fns)
    } else {
        (String::new(), String::new())
    };

    let tmpl = env
        .get_template("state_machine.rs.jinja2")
        .map_err(|e| GenerateError::TemplateLoad(format!("Template load error: {e}")))?;
    let ctx = minijinja::context! {
        model => minijinja::Value::from_serialize(model),
        machine_name => machine_name,
        license_config => minijinja::Value::from_serialize(&license_config()),
        inline_kind_types => &inline_kind_types,
        inline_kind_fns => &inline_kind_fns,
    };
    tmpl.render(ctx).map_err(render_error)
}

// ── C++ generator ────────────────────────────────────────────────

/// Generate C++ code from an analyzed SCXMLModel (filesystem-based).
pub fn generate_cpp(
    model: &SCXMLModel,
    template_dir: &Path,
    input_stem: &str,
) -> Result<GeneratedOutput, GenerateError> {
    let mut env = new_env();
    load_templates(&mut env, template_dir)?;
    filters::register_cpp_filters(&mut env);
    render_cpp(&env, model, input_stem)
}

/// Generate C++ code using pre-loaded template strings (WASM-compatible).
pub fn generate_cpp_with_templates(
    model: &SCXMLModel,
    templates: &[(&str, &str)],
    input_stem: &str,
) -> Result<GeneratedOutput, GenerateError> {
    let mut env = new_env();
    load_template_strings(&mut env, templates)?;
    filters::register_cpp_filters(&mut env);
    render_cpp(&env, model, input_stem)
}

fn render_cpp(
    env: &Environment,
    model: &SCXMLModel,
    input_stem: &str,
) -> Result<GeneratedOutput, GenerateError> {
    let inl_filename = format!("{input_stem}_sm.inl");
    // W3C SCXML 5.3: base_path is the directory containing the SCXML file,
    // used by DataModelInitHelper for resolving file: URIs in data src attributes.
    // Python codegen uses Path(output_dir).name; we use scxml_base_path which is
    // the parent directory of the SCXML file (set by analyzer::compute_scxml_base_path).
    let base_path = model.scxml_base_path.clone();

    // SCE Forge: render inline kind declarations as C++ code fragment.
    let inline_kind_code = if !model.inline_kinds.is_empty() {
        let machine_name = filters::to_pascal_case(model.name.clone());
        crate::forge::generator::render_inline_kinds(
            &model.inline_kinds,
            Language::Cpp,
            &machine_name,
        )
        .map_err(|e| GenerateError::TemplateRender(e.to_string()))?
        .member_fns
    } else {
        String::new()
    };

    let header_tmpl = env
        .get_template("state_machine.jinja2")
        .map_err(|e| GenerateError::TemplateLoad(format!("Template load error: {e}")))?;
    let inl_tmpl = env
        .get_template("state_machine_inl.jinja2")
        .map_err(|e| GenerateError::TemplateLoad(format!("Template load error: {e}")))?;

    let model_val = minijinja::Value::from_serialize(model);
    let license_val = minijinja::Value::from_serialize(&license_config());

    let header_ctx = minijinja::context! {
        model => &model_val,
        base_path => &base_path,
        license_config => &license_val,
        inl_filename => &inl_filename,
        inline_kind_code => &inline_kind_code,
    };
    let inl_ctx = minijinja::context! {
        model => &model_val,
        base_path => &base_path,
        license_config => &license_val,
    };

    let header_code = header_tmpl.render(header_ctx).map_err(render_error)?;
    let header_code = postprocess_cpp_header(&header_code);
    let inl_code = inl_tmpl.render(inl_ctx).map_err(render_error)?;
    let inl_code = postprocess_cpp_inl(&inl_code);

    Ok(GeneratedOutput {
        files: vec![
            (format!("{input_stem}_sm.h"), header_code),
            (inl_filename, inl_code),
        ],
    })
}

// ── Kotlin generator ─────────────────────────────────────────────

/// Generate Kotlin code from an analyzed SCXMLModel (filesystem-based).
pub fn generate_kotlin(model: &SCXMLModel, template_dir: &Path) -> Result<String, GenerateError> {
    reject_mesh_rpc_in_unsupported_lang(model, "Kotlin")?;
    let mut env = new_env();
    load_templates(&mut env, template_dir)?;
    filters::register_kotlin_filters(&mut env);
    register_kotlin_dynamic_filters(&mut env, model);
    render_kotlin(&env, model)
}

/// Generate Kotlin code using pre-loaded template strings (WASM-compatible).
pub fn generate_kotlin_with_templates(
    model: &SCXMLModel,
    templates: &[(&str, &str)],
) -> Result<String, GenerateError> {
    reject_mesh_rpc_in_unsupported_lang(model, "Kotlin")?;
    let mut env = new_env();
    load_template_strings(&mut env, templates)?;
    filters::register_kotlin_filters(&mut env);
    register_kotlin_dynamic_filters(&mut env, model);
    render_kotlin(&env, model)
}

/// Register model-dependent Kotlin filters (event refs, parallel checks).
fn register_kotlin_dynamic_filters(env: &mut Environment, model: &SCXMLModel) {
    use crate::kotlin;

    let kotlin_events: std::collections::BTreeSet<String> = model
        .events
        .iter()
        .filter(|e| e.as_str() != "Wildcard")
        .cloned()
        .collect();
    let event_tree = kotlin::build_event_tree(&kotlin_events);
    let branch_events = kotlin::collect_branch_events(&event_tree, "");

    let branch_events_clone = branch_events.clone();
    env.add_filter(
        "to_event_ref",
        move |name: String| -> String { kotlin::to_event_ref(&name, &branch_events_clone) },
    );

    let parallel_regions = model.parallel_regions.clone();
    let states_for_check = model.states.clone();
    env.add_filter(
        "to_parallel_complete_check",
        move |parallel_id: String| -> String {
            kotlin::to_parallel_complete_check(&parallel_id, &parallel_regions, &states_for_check)
        },
    );
}

fn render_kotlin(env: &Environment, model: &SCXMLModel) -> Result<String, GenerateError> {
    use crate::{analyzer, kotlin};

    let machine_name = filters::to_pascal_case(model.name.clone());

    // Shared analysis (language-agnostic, from analyzer)
    let ancestor_chains = analyzer::compute_ancestor_chains(model);
    let parent_map = analyzer::compute_parent_map(model);
    let leaf_map = analyzer::compute_leaf_map(model);
    let parallel_descendants = analyzer::compute_parallel_descendants(model);
    let initial_entry_root = analyzer::compute_initial_entry_root(model);

    // Kotlin-specific analysis (serde_json output for template rendering)
    let effective_transitions = kotlin::compute_effective_transitions(model, &ancestor_chains);
    let (ancestors_with_event_transitions, ancestors_with_null_transitions) =
        kotlin::compute_ancestors_with_transitions(model, &ancestor_chains);
    let deep_initial_entries = kotlin::compute_deep_initial_entries(model);
    let invoke_entries = kotlin::compute_invoke_entries(model);

    // Event tree for sealed interface hierarchy
    let kotlin_events: std::collections::BTreeSet<String> = model
        .events
        .iter()
        .filter(|e| e.as_str() != "Wildcard")
        .cloned()
        .collect();
    let event_tree = kotlin::build_event_tree(&kotlin_events);
    let leaf_events = kotlin::collect_leaf_events(&event_tree, "");

    // Pre-render event tree as Kotlin sealed interfaces
    let event_members =
        kotlin::render_event_tree(&event_tree, &format!("{machine_name}Event"), "    ");

    // SCE Forge: render inline kind declarations as Kotlin code fragment.
    let inline_kind_code = if !model.inline_kinds.is_empty() {
        crate::forge::generator::render_inline_kinds(
            &model.inline_kinds,
            Language::Kotlin,
            &machine_name,
        )
        .map_err(|e| GenerateError::TemplateRender(e.to_string()))?
        .member_fns
    } else {
        String::new()
    };

    let tmpl = env
        .get_template("state_machine.kt.jinja2")
        .map_err(|e| GenerateError::TemplateLoad(format!("Template load error: {e}")))?;

    let ctx = minijinja::context! {
        model => minijinja::Value::from_serialize(model),
        machine_name => machine_name,
        event_tree => minijinja::Value::from_serialize(&event_tree),
        event_members => event_members,
        leaf_events => minijinja::Value::from_serialize(&leaf_events),
        license_config => minijinja::Value::from_serialize(&license_config()),
        kotlin_default => minijinja::Value::from_object(KotlinDefaultFn),
        initial_entry_root => initial_entry_root,
        ancestor_chains => minijinja::Value::from_serialize(&ancestor_chains),
        effective_transitions => minijinja::Value::from_serialize(&effective_transitions),
        parent_map => minijinja::Value::from_serialize(&parent_map),
        leaf_map => minijinja::Value::from_serialize(&leaf_map),
        parallel_descendants => minijinja::Value::from_serialize(&parallel_descendants),
        deep_initial_entries => minijinja::Value::from_serialize(&deep_initial_entries),
        invoke_entries => minijinja::Value::from_serialize(&invoke_entries),
        ancestors_with_event_transitions => minijinja::Value::from_serialize(&ancestors_with_event_transitions),
        ancestors_with_null_transitions => minijinja::Value::from_serialize(&ancestors_with_null_transitions),
        inline_kind_code => &inline_kind_code,
    };

    let output = tmpl.render(ctx).map_err(render_error)?;
    // Template leaves class body open; we close it here (implicit contract with state_machine.kt.jinja2)
    Ok(output.trim_end().to_string() + "\n}\n")
}

/// kotlin_default callable from templates.
#[derive(Debug)]
struct KotlinDefaultFn;

impl std::fmt::Display for KotlinDefaultFn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<kotlin_default>")
    }
}

impl minijinja::value::Object for KotlinDefaultFn {
    fn call(
        self: &std::sync::Arc<Self>,
        _state: &minijinja::State,
        args: &[minijinja::Value],
    ) -> Result<minijinja::Value, minijinja::Error> {
        if let Some(var) = args.first() {
            let var_type = var
                .get_attr("type")
                .ok()
                .and_then(|v| if v.is_undefined() { None } else { Some(v.to_string()) })
                .unwrap_or_default();
            let expr = var
                .get_attr("expr")
                .ok()
                .and_then(|v| if v.is_undefined() { None } else { Some(v.to_string()) })
                .unwrap_or_default();

            let default = if expr.is_empty() {
                // No expr: return type-based default
                match var_type.as_str() {
                    "int" => "0".to_string(),
                    "string" => "\"\"".to_string(),
                    "bool" => "false".to_string(),
                    _ => "null".to_string(),
                }
            } else {
                // Has expr: use the expression value
                match var_type.as_str() {
                    "int" => expr.clone(),
                    "bool" => {
                        if expr == "true" || expr == "false" {
                            expr.clone()
                        } else {
                            "false".to_string()
                        }
                    }
                    "string" => {
                        if expr.starts_with('"') && expr.ends_with('"') {
                            // Already double-quoted: escape $ for Kotlin string interpolation
                            let inner = &expr[1..expr.len() - 1];
                            format!("\"{}\"", inner.replace('$', "\\$"))
                        } else if expr.starts_with('\'') && expr.ends_with('\'') {
                            // Single-quoted: convert to double-quoted Kotlin string
                            let inner = &expr[1..expr.len() - 1];
                            format!("\"{}\"", inner.replace('$', "\\$"))
                        } else {
                            expr.clone()
                        }
                    }
                    _ => "null".to_string(),
                }
            };
            Ok(minijinja::Value::from(default))
        } else {
            Ok(minijinja::Value::from("null"))
        }
    }
}

// ── Go generator ────────────────────────────────────────────────

/// Generate Go code from an analyzed SCXMLModel (filesystem-based).
pub fn generate_go(model: &SCXMLModel, template_dir: &Path) -> Result<String, GenerateError> {
    reject_mesh_rpc_in_unsupported_lang(model, "Go")?;
    let mut env = new_env();
    load_templates(&mut env, template_dir)?;
    filters::register_go_filters(&mut env);
    render_go(&env, model)
}

/// Generate Go code using pre-loaded template strings (WASM-compatible).
pub fn generate_go_with_templates(
    model: &SCXMLModel,
    templates: &[(&str, &str)],
) -> Result<String, GenerateError> {
    reject_mesh_rpc_in_unsupported_lang(model, "Go")?;
    let mut env = new_env();
    load_template_strings(&mut env, templates)?;
    filters::register_go_filters(&mut env);
    render_go(&env, model)
}

fn render_go(env: &Environment, model: &SCXMLModel) -> Result<String, GenerateError> {
    let machine_name = filters::to_pascal_case(model.name.clone());

    // SCE Forge: render inline kind declarations as Go code fragments.
    let (inline_kind_types, inline_kind_fns) = if !model.inline_kinds.is_empty() {
        let code = crate::forge::generator::render_inline_kinds(
            &model.inline_kinds,
            Language::Go,
            &machine_name,
        )
        .map_err(|e| GenerateError::TemplateRender(e.to_string()))?;
        (code.type_defs, code.member_fns)
    } else {
        (String::new(), String::new())
    };

    let tmpl = env
        .get_template("state_machine.go.jinja2")
        .map_err(|e| GenerateError::TemplateLoad(format!("Template load error: {e}")))?;
    let ctx = minijinja::context! {
        model => minijinja::Value::from_serialize(model),
        machine_name => machine_name,
        license_config => minijinja::Value::from_serialize(&license_config()),
        inline_kind_types => &inline_kind_types,
        inline_kind_fns => &inline_kind_fns,
    };
    tmpl.render(ctx).map_err(render_error)
}

// ── Template loading helpers ─────────────────────────────────────

/// Load templates from pre-loaded string pairs (WASM-compatible).
fn load_template_strings(
    env: &mut Environment<'_>,
    templates: &[(&str, &str)],
) -> Result<(), GenerateError> {
    for (name, content) in templates {
        env.add_template_owned(name.to_string(), content.to_string())
            .map_err(|e| GenerateError::TemplateLoad(format!("Template parse error in {name}: {e}")))?;
    }
    Ok(())
}

/// Recursively load all .jinja2 templates from a directory.
pub fn load_templates(env: &mut Environment<'_>, dir: &Path) -> Result<(), GenerateError> {
    if !dir.exists() {
        return Err(GenerateError::TemplateLoad(format!(
            "Template directory not found: {}",
            dir.display()
        )));
    }
    load_templates_recursive(env, dir, dir)
}

fn load_templates_recursive(
    env: &mut Environment<'_>,
    base_dir: &Path,
    current_dir: &Path,
) -> Result<(), GenerateError> {
    let entries = std::fs::read_dir(current_dir)
        .map_err(|e| GenerateError::TemplateLoad(format!("Cannot read {}: {e}", current_dir.display())))?;

    for entry in entries {
        let entry = entry.map_err(|e| GenerateError::TemplateLoad(format!("Dir entry error: {e}")))?;
        let path = entry.path();
        if path.is_dir() {
            load_templates_recursive(env, base_dir, &path)?;
        } else if path.extension().and_then(|e| e.to_str()) == Some("jinja2") {
            let rel = path
                .strip_prefix(base_dir)
                .map_err(|e| GenerateError::TemplateLoad(format!("Path error: {e}")))?;
            let template_name = rel.to_string_lossy().replace('\\', "/");
            let content = std::fs::read_to_string(&path)
                .map_err(|e| GenerateError::TemplateLoad(format!("Cannot read template {}: {e}", path.display())))?;
            env.add_template_owned(template_name, content)
                .map_err(|e| GenerateError::TemplateLoad(format!("Template parse error in {}: {e}", path.display())))?;
        }
    }
    Ok(())
}

// ── C++ post-processing ────────────────────────────────────────
//
// Responsibility: structural corrections that templates cannot express
// (dedent, include sort, blank-line collapse, orphaned-line re-indent).
//
// Style-level formatting (pointer alignment, line wrapping, macro alignment,
// brace insertion) is delegated to clang-format via the CMake build system.
// This keeps a clean separation: codegen → structure → style.

/// Post-process generated C++ header (.h) to match clang-format style.
fn postprocess_cpp_header(code: &str) -> String {
    let lines: Vec<&str> = code.lines().collect();
    let mut out = Vec::with_capacity(lines.len());

    // Sort include blocks and fix preprocessor indentation.
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];

        // Sort contiguous #include blocks alphabetically.
        if line.trim_start().starts_with("#include") {
            let mut include_block = vec![line.to_string()];
            i += 1;
            while i < lines.len() && lines[i].trim_start().starts_with("#include") {
                include_block.push(lines[i].to_string());
                i += 1;
            }
            include_block.sort();
            for inc in include_block {
                out.push(inc);
            }
            continue;
        }

        // Strip indent inside #ifndef preprocessor guards.
        if line.starts_with("    #define ") || line.starts_with("    #define\t") {
            out.push(line.trim_start().to_string());
            i += 1;
            continue;
        }
        if line == "    // Debug logging disabled in release builds" {
            out.push(line.trim_start().to_string());
            i += 1;
            continue;
        }

        // Namespace closing: `} //` → `}  //`
        if line.starts_with("} // namespace") {
            out.push(line.replacen("} // ", "}  // ", 1));
            i += 1;
            continue;
        }

        out.push(line.to_string());
        i += 1;
    }

    // Collapse consecutive blank lines, remove trailing blanks.
    collapse_blank_lines(&out)
}

/// Collapse consecutive blank lines to single blank line, trim trailing.
fn collapse_blank_lines(lines: &[String]) -> String {
    let mut result = String::new();
    let mut prev_blank = false;
    for line in lines {
        let is_blank = line.trim().is_empty();
        if is_blank && prev_blank {
            continue;
        }
        prev_blank = is_blank;
        result.push_str(line);
        result.push('\n');
    }
    let trimmed = result.trim_end();
    if trimmed.is_empty() {
        return String::new();
    }
    format!("{trimmed}\n")
}

/// Post-process generated C++ .inl file to match the project clang-format style.
///
/// The .inl file is `#include`d inside a struct body, so templates produce code
/// with a 4-space base indent. Additionally, action templates (raise, script, etc.)
/// emit code at column 0 regardless of their nesting context (Jinja2 limitation).
///
/// This function:
/// 1. Strips the 4-space base indent from all lines (the template's struct-level indent).
/// 2. Re-indents orphaned lines (lines at 0 indent inside a nested block) by
///    tracking brace depth — a lightweight structural re-indenter.
/// 3. Collapses consecutive blank lines to one.
fn postprocess_cpp_inl(code: &str) -> String {
    // The .inl template uses 4-space indent for all top-level code.
    const BASE_INDENT: usize = 4;

    let mut lines: Vec<String> = Vec::new();
    let mut brace_depth: i32 = 0;

    for raw_line in code.lines() {
        let is_blank = raw_line.trim().is_empty();
        if is_blank {
            lines.push(String::new());
            continue;
        }

        // Strip the base indent (4 spaces) from lines that have it.
        let line = if raw_line.len() >= BASE_INDENT
            && raw_line[..BASE_INDENT].chars().all(|c| c == ' ')
        {
            raw_line[BASE_INDENT..].to_string()
        } else {
            // Line has less than BASE_INDENT leading spaces (e.g., orphaned action code at col 0).
            // Will be re-indented below based on brace depth.
            raw_line.to_string()
        };

        let trimmed = line.trim().to_string();
        if trimmed.is_empty() {
            lines.push(String::new());
            continue;
        }

        // Count braces to track nesting depth, excluding braces in string
        // literals and comments.
        let opens: i32 = count_braces(&trimmed, '{');
        let closes: i32 = count_braces(&trimmed, '}');

        // A line starting with '}' reduces depth BEFORE indentation.
        let effective_depth = if trimmed.starts_with('}') {
            (brace_depth - closes + opens).max(0)
        } else {
            brace_depth
        };

        // The line's own indent (after base stripping).
        let line_indent = line.len() - trimmed.len();

        // If this line is at 0 indent but should be deeper (orphaned action code),
        // re-indent it to match the current brace depth.
        let output_line = if line_indent == 0 && effective_depth > 0 {
            let indent_str: String =
                std::iter::repeat(' ').take(effective_depth as usize * 4).collect();
            format!("{indent_str}{trimmed}")
        } else {
            line
        };

        lines.push(output_line);

        // Update brace depth for the NEXT line.
        if trimmed.starts_with('}') {
            brace_depth = (brace_depth - closes + opens).max(0);
        } else {
            brace_depth = (brace_depth + opens - closes).max(0);
        }
    }

    collapse_blank_lines(&lines)
}

/// Count occurrences of a brace character, skipping string literals and comments.
fn count_braces(line: &str, brace: char) -> i32 {
    let mut count = 0i32;
    let mut in_string = false;
    let mut in_char = false;
    let mut prev = '\0';
    let bytes = line.as_bytes();

    for (i, &b) in bytes.iter().enumerate() {
        let c = b as char;
        if in_string {
            if c == '"' && prev != '\\' {
                in_string = false;
            }
        } else if in_char {
            if c == '\'' && prev != '\\' {
                in_char = false;
            }
        } else {
            // Check for line comment
            if c == '/' && i + 1 < bytes.len() && bytes[i + 1] == b'/' {
                break; // Rest of line is comment
            }
            if c == '"' {
                in_string = true;
            } else if c == '\'' {
                in_char = true;
            } else if c == brace {
                count += 1;
            }
        }
        prev = c;
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::SCXMLParser;

    /// Document with a single `<invoke type="sce:mesh-rpc">` site —
    /// triggers `model.has_mesh_rpc_invoke()` and exercises the
    /// rejection path on backends without mesh codegen.
    const MESH_RPC_SCXML: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
       datamodel="null" name="brake" initial="idle">
  <state id="idle">
    <invoke type="sce:mesh-rpc" src="#motor">
      <param name="_mesh_event" expr="'service.request.compute_force'"/>
    </invoke>
  </state>
</scxml>"##;

    fn parse(content: &str) -> SCXMLModel {
        let mut p = SCXMLParser::new();
        p.parse_string(content, "brake").expect("parse")
    }

    /// SCE_MESH.md §9.5 mesh-rpc invokes only have a C++ codegen path
    /// today. `generate` (Rust) MUST refuse — silent skipping would
    /// hand the operator a state machine where an `<invoke>` quietly
    /// does nothing at runtime.
    #[test]
    fn rust_generate_rejects_mesh_rpc_invoke() {
        let model = parse(MESH_RPC_SCXML);
        let templates: &[(&str, &str)] = &[];
        let err = generate_with_templates(&model, templates).unwrap_err();
        match err {
            GenerateError::UnsupportedFeature(msg) => {
                assert!(msg.contains("sce:mesh-rpc"), "msg names the feature: {msg}");
                assert!(msg.contains("Rust"), "msg names the language: {msg}");
                assert!(msg.contains("brake"), "msg names the machine: {msg}");
            }
            other => panic!("expected UnsupportedFeature, got {other:?}"),
        }
    }

    #[test]
    fn kotlin_generate_rejects_mesh_rpc_invoke() {
        let model = parse(MESH_RPC_SCXML);
        let templates: &[(&str, &str)] = &[];
        let err = generate_kotlin_with_templates(&model, templates).unwrap_err();
        assert!(matches!(err, GenerateError::UnsupportedFeature(_)));
    }

    #[test]
    fn go_generate_rejects_mesh_rpc_invoke() {
        let model = parse(MESH_RPC_SCXML);
        let templates: &[(&str, &str)] = &[];
        let err = generate_go_with_templates(&model, templates).unwrap_err();
        assert!(matches!(err, GenerateError::UnsupportedFeature(_)));
    }

    /// Models without mesh-rpc invokes must NOT be rejected — the gate
    /// is feature-specific, not a blanket non-C++ block.
    #[test]
    fn rust_generate_accepts_plain_scxml() {
        let plain = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
       datamodel="null" name="plain" initial="s">
  <state id="s"><transition event="e" target="s2"/></state>
  <state id="s2"/>
</scxml>"##;
        let model = parse(plain);
        // We only care that the gate doesn't reject; the actual
        // template render needs the full template set, which this
        // unit test deliberately omits to keep it focused. The gate
        // runs FIRST, so its early-return on Ok(()) lets the call
        // proceed to the (failing-without-templates) render path —
        // any error here other than UnsupportedFeature proves the
        // gate is not the blocker.
        let templates: &[(&str, &str)] = &[];
        match generate_with_templates(&model, templates) {
            Err(GenerateError::UnsupportedFeature(_)) => {
                panic!("plain SCXML must not trip the mesh-rpc gate")
            }
            // Anything else (Ok / template error / etc.) means the
            // gate let the model through, which is the contract.
            _ => {}
        }
    }
}

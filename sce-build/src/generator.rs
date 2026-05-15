// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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
///
/// `C11` is reserved for the embedded MCU backend per RFC §5.J.1
/// (watching-zenoh consumer, Phase A5). Enum membership lets every
/// existing dispatch site flag the unimplemented case explicitly
/// rather than silently routing C11 through a more permissive arm.
/// Per-kind `C11` emitters land in M2+ (lookup vertical slice first).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    Rust,
    Cpp,
    Kotlin,
    Go,
    Python,
    C11,
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
            "c11" | "c" => Ok(Language::C11),
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

// ── §16.5 L3500 barrier-timeout observability gate ────────────────
//
// A deploy.yaml `barrier_timeout_ms:` on a Root partition is a signal
// that the author wants to **observe** the `error.communication`
// (reason `PARALLEL_BARRIER_TIMEOUT`) raise when regions fail to
// converge in time. If the author's SCXML carries no transition for
// `error.communication`, the raised event falls into the default
// microstep path and is silently discarded — the knob is set but has
// no observable consequence, the `feedback_silently_broken_hooks`
// anti-pattern verbatim. Refuse at codegen instead so the gap
// surfaces with the SCXML in hand rather than as a post-deploy
// observation that "the timeout does nothing".
//
// The check is local to the machine currently being codegen'd. A
// distributed `<parallel>` whose Root lives in a different machine
// has `partition_barrier_timeouts` empty here (only Root-owning
// machines carry an entry); NonRoot machines never reach this gate.
fn reject_barrier_timeout_without_handler(
    model: &SCXMLModel,
) -> Result<(), GenerateError> {
    if model.partition_barrier_timeouts.is_empty() {
        return Ok(());
    }
    if model.events.contains("error.communication") {
        return Ok(());
    }
    let parallels: Vec<String> = model
        .partition_barrier_timeouts
        .keys()
        .cloned()
        .collect();
    Err(GenerateError::UnsupportedFeature(format!(
        "machine '{}' declares `barrier_timeout_ms:` on a Root partition \
         for <parallel id=\"{}\"> but the SCXML has no transition for \
         event `error.communication`. SCE_MESH.md §16.5 L3500 raises \
         `error.communication` (reason PARALLEL_BARRIER_TIMEOUT) when \
         the barrier elapses — without a transition the raise is \
         silently discarded and the timeout has no observable effect. \
         Add a `<transition event=\"error.communication\">` handler \
         (optionally guarded on `_event.data.reason == \
         'PARALLEL_BARRIER_TIMEOUT'`) or drop `barrier_timeout_ms:` \
         from the partition declaration.",
        model.name,
        parallels.join(", ")
    )))
}

// ── §16.4 / §16.7 liveness observability gate ────────────────────
//
// Symmetric to `reject_barrier_timeout_without_handler` for the
// liveness raise paths. `deploy.yaml`'s `liveliness:` block drives
// two §16.7 rows that both surface as `error.communication`:
//   - row 8 `PEER_PARTITIONED` — fires on DROP of a machine-level
//     `sce/live/<machine>` token, i.e. on every machine that
//     declares `liveliness:` regardless of partitioning.
//   - row 13 `REGION_PARTITIONED` — fires on DROP of a partition
//     token `sce/live/<machine>/<partition>` — partitioned machines
//     only.
// Without a matching `<transition event="error.communication">`
// either raise sinks into the default microstep path and is
// silently discarded, which is exactly the
// `feedback_silently_broken_hooks` anti-pattern. A single gate
// covers both rows because the model flag is set whenever the
// machine declares `liveliness:` — there is no `liveliness:` shape
// that produces row 13 without also authorizing row 8.
fn reject_liveliness_without_handler(
    model: &SCXMLModel,
) -> Result<(), GenerateError> {
    if !model.machine_liveliness_opt_in {
        return Ok(());
    }
    if model.events.contains("error.communication") {
        return Ok(());
    }
    Err(GenerateError::UnsupportedFeature(format!(
        "machine '{}' declares `liveliness:` but the SCXML has no \
         transition for event `error.communication`. SCE_MESH.md \
         §16.4 / §16.7 rows 8 and 13 raise `error.communication` \
         (reason PEER_PARTITIONED or REGION_PARTITIONED) when a \
         peer's Zenoh liveliness token drops — without a transition \
         the raise is silently discarded and the signal has no \
         observable effect. Add a `<transition \
         event=\"error.communication\">` handler (optionally guarded \
         on `_event.data.reason`) or drop `liveliness:` from the \
         machine declaration.",
        model.name
    )))
}

// ── Rust generator ───────────────────────────────────────────────

/// Generate Rust code from an analyzed SCXMLModel (filesystem-based).
///
/// `no_std` toggles the watching-zenoh RFC §5.J.2 (C3 Atomic B-γ2b)
/// codegen mode: emits `#![no_std]` at the crate root and switches
/// `parent_external_queue` + microstep `HashSet` to heapless variants.
/// Default `false` keeps std-coupled output for the existing 200+ AOT
/// W3C fixtures byte-identical. The B-β (`818de8eb`) CLI flag
/// `--no-std` threads through `cmd_generate` to this parameter.
pub fn generate(model: &SCXMLModel, template_dir: &Path, no_std: bool) -> Result<String, GenerateError> {
    reject_mesh_rpc_in_unsupported_lang(model, "Rust")?;
    let mut env = new_env();
    load_templates(&mut env, template_dir)?;
    filters::register_filters(&mut env);
    render_rust(&env, model, no_std)
}

/// Generate Rust code using pre-loaded template strings (WASM-compatible).
///
/// See [`generate`] for `no_std` semantics.
pub fn generate_with_templates(
    model: &SCXMLModel,
    templates: &[(&str, &str)],
    no_std: bool,
) -> Result<String, GenerateError> {
    reject_mesh_rpc_in_unsupported_lang(model, "Rust")?;
    let mut env = new_env();
    load_template_strings(&mut env, templates)?;
    filters::register_filters(&mut env);
    render_rust(&env, model, no_std)
}

fn render_rust(env: &Environment, model: &SCXMLModel, no_std: bool) -> Result<String, GenerateError> {
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
        no_std => no_std,
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
    reject_barrier_timeout_without_handler(model)?;
    reject_liveliness_without_handler(model)?;
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

// ── C11 generator ────────────────────────────────────────────────
//
// RFC §5.J.1 — watching-zenoh consumer (MCU AOT backend). Mirrors the
// C++ pair-render shape (`generate_cpp` above) but emits a `.h` + `.c`
// translation unit instead of `.h` + `.inl` because C11 has no in-class
// definitions to hide behind a textual include.
//
// Mesh patterns are out-of-scope for C11 per the RFC — we still call
// the mesh-shape rejectors so an SCXML carrying mesh markings fails
// loud here rather than producing a half-rendered translation unit.

/// Generate C11 code from an analyzed SCXMLModel (filesystem-based).
pub fn generate_c11(
    model: &SCXMLModel,
    template_dir: &Path,
    input_stem: &str,
) -> Result<GeneratedOutput, GenerateError> {
    let mut env = new_env();
    load_templates(&mut env, template_dir)?;
    filters::register_c11_filters(&mut env);
    render_c11(&env, model, input_stem)
}

/// Generate C11 code using pre-loaded template strings (WASM-compatible).
pub fn generate_c11_with_templates(
    model: &SCXMLModel,
    templates: &[(&str, &str)],
    input_stem: &str,
) -> Result<GeneratedOutput, GenerateError> {
    let mut env = new_env();
    load_template_strings(&mut env, templates)?;
    filters::register_c11_filters(&mut env);
    render_c11(&env, model, input_stem)
}

fn render_c11(
    env: &Environment,
    model: &SCXMLModel,
    input_stem: &str,
) -> Result<GeneratedOutput, GenerateError> {
    reject_mesh_rpc_in_unsupported_lang(model, "C11")?;
    reject_barrier_timeout_without_handler(model)?;
    reject_liveliness_without_handler(model)?;
    let base_path = model.scxml_base_path.clone();

    // SCE Forge: render inline kind declarations as C11 code fragment.
    // Mirrors cpp/Kotlin's single-block emit (no top-level type_defs split
    // because C11 has no nested types — enum typedefs and `static inline`
    // functions both flow into member_fns and inject after the policy
    // typedef in state_machine.h.jinja2).
    let inline_kind_code = if !model.inline_kinds.is_empty() {
        let machine_name = filters::to_pascal_case(model.name.clone());
        crate::forge::generator::render_inline_kinds(
            &model.inline_kinds,
            Language::C11,
            &machine_name,
        )
        .map_err(|e| GenerateError::TemplateRender(e.to_string()))?
        .member_fns
    } else {
        String::new()
    };

    let header_tmpl = env
        .get_template("c/state_machine.h.jinja2")
        .map_err(|e| GenerateError::TemplateLoad(format!("Template load error: {e}")))?;
    let source_tmpl = env
        .get_template("c/state_machine.c.jinja2")
        .map_err(|e| GenerateError::TemplateLoad(format!("Template load error: {e}")))?;

    let model_val = minijinja::Value::from_serialize(model);
    let license_val = minijinja::Value::from_serialize(&license_config());

    let header_ctx = minijinja::context! {
        model => &model_val,
        base_path => &base_path,
        license_config => &license_val,
        inline_kind_code => &inline_kind_code,
    };
    let source_ctx = minijinja::context! {
        model => &model_val,
        base_path => &base_path,
        license_config => &license_val,
    };

    let header_code = header_tmpl.render(header_ctx).map_err(render_error)?;
    let source_code = source_tmpl.render(source_ctx).map_err(render_error)?;

    Ok(GeneratedOutput {
        files: vec![
            (format!("{input_stem}_sm.h"), header_code),
            (format!("{input_stem}_sm.c"), source_code),
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

// ── Python generator ────────────────────────────────────────────
//
// Python AOT (Atomic β): atomic + compound states, basic + eventless
// transitions, onentry/onexit, transition guards/actions, `<data>` early-
// binding datamodel with `<assign>` updates, and `<raise>` for internal
// events. Parallel / history / invoke land in Atomic γ. Generated `*_sm.py`
// modules depend on `sce-python-runtime` (pure-Python W3C SCXML engine) —
// analogous to how the Go backend depends on `sce-go-runtime` and Kotlin on
// its runtime package. The pybind11 channel under `sce-python/` is a
// separate (interpreter-mode) integration and is not used here.

/// Generate Python code from an analyzed SCXMLModel (filesystem-based).
pub fn generate_python(
    model: &SCXMLModel,
    template_dir: &Path,
) -> Result<String, GenerateError> {
    reject_mesh_rpc_in_unsupported_lang(model, "Python")?;
    reject_python_unsupported_features(model)?;
    let mut env = new_env();
    load_templates(&mut env, template_dir)?;
    filters::register_python_filters(&mut env);
    render_python(&env, model)
}

/// Generate Python code using pre-loaded template strings (WASM-compatible).
pub fn generate_python_with_templates(
    model: &SCXMLModel,
    templates: &[(&str, &str)],
) -> Result<String, GenerateError> {
    reject_mesh_rpc_in_unsupported_lang(model, "Python")?;
    reject_python_unsupported_features(model)?;
    let mut env = new_env();
    load_template_strings(&mut env, templates)?;
    filters::register_python_filters(&mut env);
    render_python(&env, model)
}

/// Atomic γ-2 surface — explicitly reject features the Python codegen
/// does not yet implement. Failing loudly here keeps generated `*_sm.py`
/// honest: every accepted document produces a working module instead of
/// a silently degraded one. γ progressively widens the surface and
/// removes the corresponding rejects (γ-1 lifted `<parallel>`; γ-2
/// lifted `<history>`; γ-3 lifts the remaining executable content;
/// γ-4 lifts `<invoke>`).
fn reject_python_unsupported_features(model: &SCXMLModel) -> Result<(), GenerateError> {
    // γ-4a accepts `<invoke type="scxml">`. Hybrid (`srcexpr`/`contentexpr`)
    // invokes still defer to γ-4b because they need runtime expression
    // evaluation; mesh-rpc invokes are permanently rejected per the
    // C++-first mesh policy (`mesh_cpp_first_policy.md`).
    for inv in &model.invokes {
        match inv {
            crate::model::Invoke::Scxml(_) => {}
            crate::model::Invoke::Hybrid(_) => {
                return Err(GenerateError::InvalidConfig(
                    "Python codegen does not yet support hybrid <invoke srcexpr>/<invoke contentexpr>; \
                     deferred to Atomic γ-4b".into(),
                ));
            }
            crate::model::Invoke::MeshRpc(_) => {
                return Err(GenerateError::InvalidConfig(
                    "Python codegen rejects <invoke type=\"sce:mesh-rpc\">: \
                     mesh runtime is C++ alone (mesh_cpp_first_policy)".into(),
                ));
            }
        }
    }
    // β datamodel storage uses dict-keyed `self._ns[name]`. User-written
    // `<script>` / `cond` / `expr` text references the same names as bare
    // Python identifiers, which is parsed by `eval` directly. A `<data>`
    // id that happens to be a Python keyword (`class`, `lambda`, ...)
    // would silently break every user expression that referenced it (and
    // mangling the storage key alone wouldn't help — the user's own
    // expression text would still contain the bare keyword). We reject
    // up front so the SCXML author hits a clear error instead of a
    // SyntaxError at runtime [[feedback-silently-broken-hooks]]. γ may
    // lift this once an expression-rewriter pass lands.
    for var in &model.variables {
        if PYTHON_KEYWORDS.contains(&var.id.as_str()) {
            return Err(GenerateError::InvalidConfig(format!(
                "Python codegen rejects <data id=\"{}\">: name collides with a Python \
                 keyword and would break user expressions referencing it",
                var.id
            )));
        }
    }
    // W3C SCXML 3.13 — `<transition event="*">` matches every external
    // event except the eventless NULL sentinel; the codegen lowers it to
    // `if event != Event.NULL` in `process_transition.py.jinja2`. Prefix
    // (`event="foo.*"`) and multi-event (`event="foo bar"`) descriptors
    // lower through the `_event_name_matches` helper in the same
    // template (W3C 3.13 token-prefix match); no reject is needed.
    // γ-4 send extensions lift the eventexpr / delayexpr / payload /
    // idlocation rejects: dynamic expressions evaluate against the
    // datamodel at action time, payload marshalling runs through
    // `_eval_send_payload`, and `idlocation` is written back by
    // `_resolve_send_id`. External-target (`target="..."`) and
    // `targetexpr` / non-default `send_type` remain reject-walled —
    // those routes need the cross-session router or HTTP transport
    // (BasicHTTPEventProcessor) that γ-4a/γ-4b lift.
    fn check_actions(
        actions: &[crate::model::Action],
        context: &str,
    ) -> Result<(), GenerateError> {
        const SUPPORTED_ACTIONS: &[&str] = &[
            "script", "assign", "raise", "log", "if", "foreach", "send", "cancel",
        ];
        for action in actions {
            if !SUPPORTED_ACTIONS.contains(&action.action_type.as_str()) {
                return Err(GenerateError::InvalidConfig(format!(
                    "Python codegen does not yet support <{}> in {}; deferred to Atomic γ",
                    action.action_type, context
                )));
            }
            if action.action_type == "send" {
                // γ-3b kept only the simple in-machine send form; γ-4
                // send extensions accept dynamic exprs + payload +
                // idlocation; γ-4a additionally accepts `target="#_parent"`
                // (child-to-parent) and `target="#_<invoke_id>"` (parent-
                // to-child via `Invoke.forward_event`); γ-4b's first
                // lift accepts `target="!…"` (the SCXML test-suite's
                // deliberate-invalid sentinel — W3C 6.2: dispatch
                // failure raises `error.execution`). HTTP transports
                // and other absolute URLs stay deferred to the HTTP
                // half of γ-4b.
                if !action.target.is_empty()
                    && !action.target.starts_with("#_")
                    && !action.target.starts_with('!')
                {
                    return Err(GenerateError::InvalidConfig(format!(
                        "Python codegen `<send target=\"{}\">` in {} is deferred \
                         to Atomic γ-4b (HTTP / external transport)",
                        action.target, context
                    )));
                }
                if !action.targetexpr.is_empty() {
                    return Err(GenerateError::InvalidConfig(format!(
                        "Python codegen `<send targetexpr>` in {} is deferred \
                         to Atomic γ-4b (HTTP / external transport target resolution)",
                        context
                    )));
                }
                if !action.send_type.is_empty()
                    && action.send_type != "http://www.w3.org/TR/scxml/#SCXMLEventProcessor"
                {
                    return Err(GenerateError::InvalidConfig(format!(
                        "Python codegen `<send type=\"{}\">` in {} is deferred \
                         to Atomic γ-4b (BasicHTTPEventProcessor)",
                        action.send_type, context
                    )));
                }
                // γ-4 send extensions accept `<send eventexpr>`,
                // `<send delayexpr>`, `<send idlocation>`, and
                // `<param>` / `<content>` / namelist payload
                // marshalling — those rejects are intentionally absent.
                if action.event.is_empty() && action.eventexpr.is_empty() {
                    return Err(GenerateError::InvalidConfig(format!(
                        "Python codegen `<send>` in {} requires `event` or `eventexpr`",
                        context
                    )));
                }
            }
            // Walk into <if>/<foreach> bodies so a nested unsupported
            // action (e.g. <send> inside an <if>) is rejected at the
            // same fail-loud surface as a top-level <send>.
            if action.action_type == "if" {
                check_actions(&action.then_actions, context)?;
                for branch in &action.elseif_branches {
                    check_actions(&branch.actions, context)?;
                }
                check_actions(&action.else_actions, context)?;
            } else if action.action_type == "foreach" {
                check_actions(&action.actions, context)?;
            }
        }
        Ok(())
    }
    for (state_id, state) in &model.states {
        for block in &state.on_entry_blocks {
            check_actions(block, &format!("onentry of `{state_id}`"))?;
        }
        for block in &state.on_exit_blocks {
            check_actions(block, &format!("onexit of `{state_id}`"))?;
        }
        for transition in &state.transitions {
            check_actions(
                &transition.actions,
                &format!("transition from `{state_id}`"),
            )?;
        }
    }
    Ok(())
}

/// Python soft + hard keywords as of 3.12. `match`/`case` are soft
/// keywords but conflict with the same syntactic positions; SCXML
/// system variables (`_event`, `_sessionid`) are reserved by W3C
/// SCXML 5.10 and are caught by the parser long before this point.
const PYTHON_KEYWORDS: &[&str] = &[
    "False", "None", "True", "and", "as", "assert", "async", "await", "break", "class",
    "continue", "def", "del", "elif", "else", "except", "finally", "for", "from", "global",
    "if", "import", "in", "is", "lambda", "nonlocal", "not", "or", "pass", "raise", "return",
    "try", "while", "with", "yield", "match", "case",
];

fn render_python(env: &Environment, model: &SCXMLModel) -> Result<String, GenerateError> {
    let machine_name = filters::to_pascal_case(model.name.clone());

    let tmpl = env
        .get_template("state_machine.py.jinja2")
        .map_err(|e| GenerateError::TemplateLoad(format!("Template load error: {e}")))?;
    let ctx = minijinja::context! {
        model => minijinja::Value::from_serialize(model),
        machine_name => machine_name,
        license_config => minijinja::Value::from_serialize(&license_config()),
    };
    tmpl.render(ctx).map_err(render_error)
}

// ── Go generator ────────────────────────────────────────────────

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
///
/// Watching-zenoh RFC §5.O Atomic 0b — also loads the workspace-
/// shared `_macros/` directory (one level up from the per-backend
/// template root) so cross-backend shared macros like
/// `_macros/sce_map_marker.jinja2` are visible to every language
/// that calls `find_template_dir_for`. Cpp / C11 already pass the
/// template root (their `subdir = ""`); Rust / Kotlin / Go / Python
/// pass a per-language subdir, so without the shared-macro loader
/// they would lose access to the cross-backend macro family. The
/// shared load skips silently when `_macros/` is absent (vendored
/// builds without the macro tree).
pub fn load_templates(env: &mut Environment<'_>, dir: &Path) -> Result<(), GenerateError> {
    if !dir.exists() {
        return Err(GenerateError::TemplateLoad(format!(
            "Template directory not found: {}",
            dir.display()
        )));
    }
    load_templates_recursive(env, dir, dir)?;
    // Sibling `_macros/` lives at `<workspace>/tools/codegen/templates/_macros/`.
    // For per-backend roots like `rust/`, `_macros/` is one level up at
    // (`<workspace>/tools/codegen/templates/_macros/`); for per-kind
    // forge backends like `forge/rust/`, it is two levels up. Walk up
    // the parent chain until either `_macros/` shows up or the chain
    // terminates, so adding a third-level template tree later does
    // not regress the inheritance.
    let mut current = dir;
    while let Some(parent) = current.parent() {
        let shared_macros = parent.join("_macros");
        if shared_macros.is_dir() && shared_macros != dir.join("_macros") {
            // base_dir = parent so the loaded template names start
            // with `_macros/...` — matches the path callers use in
            // `{% import "_macros/sce_map_marker.jinja2" as sce_map %}`.
            load_templates_recursive(env, parent, &shared_macros)?;
            break;
        }
        current = parent;
    }
    Ok(())
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

    // ── Language enum / FromStr drift guards ────────────────────
    //
    // RFC §5.J.1 (watching-zenoh consumer, M1 foundation): the C11 enum
    // variant was added without a working emitter. These tests pin the
    // boundary contract so future edits cannot silently drop "c11"/"c"
    // recognition (which would silently route C11 callers to
    // UnknownLanguage at the CLI) and so the M2+ vertical slice has a
    // signal when adding a real C11 generator path.

    #[test]
    fn language_fromstr_accepts_c11_and_c_aliases() {
        use std::str::FromStr;
        assert_eq!(Language::from_str("c11").unwrap(), Language::C11);
        assert_eq!(Language::from_str("c").unwrap(), Language::C11);
    }

    #[test]
    fn language_fromstr_rejects_unknown_strings() {
        use std::str::FromStr;
        assert!(Language::from_str("c99").is_err());
        assert!(Language::from_str("C11").is_err()); // case-sensitive, matches prior precedent
        assert!(Language::from_str("").is_err());
    }

    #[test]
    fn language_c11_distinct_from_other_variants() {
        // Distinct enum membership is the reason to add the variant
        // before the emitter exists — matches gain a pinned arm so M2+
        // implementation changes are visible in diff review.
        assert_ne!(Language::C11, Language::Cpp);
        assert_ne!(Language::C11, Language::Rust);
        assert_ne!(Language::C11, Language::Kotlin);
        assert_ne!(Language::C11, Language::Go);
        assert_ne!(Language::C11, Language::Python);
    }

    /// SCE_MESH.md §9.5 mesh-rpc invokes only have a C++ codegen path
    /// today. `generate` (Rust) MUST refuse — silent skipping would
    /// hand the operator a state machine where an `<invoke>` quietly
    /// does nothing at runtime.
    #[test]
    fn rust_generate_rejects_mesh_rpc_invoke() {
        let model = parse(MESH_RPC_SCXML);
        let templates: &[(&str, &str)] = &[];
        let err = generate_with_templates(&model, templates, false).unwrap_err();
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

    #[test]
    fn c11_generate_rejects_mesh_rpc_invoke() {
        let model = parse(MESH_RPC_SCXML);
        let templates: &[(&str, &str)] = &[];
        match generate_c11_with_templates(&model, templates, "fixture") {
            Ok(_) => panic!("expected UnsupportedFeature, got Ok"),
            Err(GenerateError::UnsupportedFeature(msg)) => {
                assert!(msg.contains("sce:mesh-rpc"), "msg names the feature: {msg}");
                assert!(msg.contains("C11"), "msg names the language: {msg}");
            }
            Err(other) => panic!("expected UnsupportedFeature, got {other:?}"),
        }
    }

    /// SCE_MESH.md §14 rule 12 / §16.5 shape assertion. With the
    /// semantic payload landed, the C++ codegen output diverges by
    /// role — Root emits a `ParallelCompletionTracker` member and
    /// `onParallelRegionDone` dispatch method; NonRoot emits a
    /// `sendParallelRegionDone` method with a wire-21 envelope
    /// constructor; SinglePartition (empty role map or absent
    /// partition_context) preserves the legacy
    /// `ParallelCompletionHelper` path.
    ///
    /// The P0 byte-identical carve-out has retired — `partition_context_present`
    /// alone (role map empty) still reproduces the pre-mesh output
    /// because `parallel_final.jinja2` falls through to the
    /// SinglePartition branch, but toggling a Role per-`<parallel>`
    /// in `partition_parallel_roles` intentionally perturbs the
    /// generated SM.
    const PARALLEL_FINAL_SCXML: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
       datamodel="null" name="pf_fixture" initial="root">
  <parallel id="root">
    <state id="left" initial="left_run">
      <state id="left_run">
        <transition event="finish_left" target="left_done"/>
      </state>
      <final id="left_done"/>
    </state>
    <state id="right" initial="right_run">
      <state id="right_run">
        <transition event="finish_right" target="right_done"/>
      </state>
      <final id="right_done"/>
    </state>
  </parallel>
</scxml>"##;

    fn render_with_role(role: Option<crate::model::PartitionRole>) -> String {
        let mut model = parse(PARALLEL_FINAL_SCXML);
        crate::analyzer::analyze(&mut model, "pf_fixture.scxml");
        if let Some(role) = role {
            model.partition_context_present = true;
            model
                .partition_parallel_roles
                .insert("root".to_string(), role);
        }
        let template_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("workspace root")
            .join("tools")
            .join("codegen")
            .join("templates");
        let out = generate_cpp(&model, &template_dir, "pf_fixture").expect("render");
        out.files
            .into_iter()
            .map(|(_, body)| body)
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Variant of [`PARALLEL_FINAL_SCXML`] that adds an
    /// `error.communication` handler so the §16.5 L3500 barrier-
    /// timeout runtime can emit without tripping
    /// [`reject_barrier_timeout_without_handler`]. The transition
    /// target (`timeout_failed`) is a dedicated final state so the
    /// raise path is authoring-observable in E2E tests too.
    const PARALLEL_FINAL_WITH_TIMEOUT_HANDLER_SCXML: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
       datamodel="null" name="pf_fixture" initial="root">
  <parallel id="root">
    <state id="left" initial="left_run">
      <state id="left_run">
        <transition event="finish_left" target="left_done"/>
      </state>
      <final id="left_done"/>
    </state>
    <state id="right" initial="right_run">
      <state id="right_run">
        <transition event="finish_right" target="right_done"/>
      </state>
      <final id="right_done"/>
    </state>
    <transition event="error.communication" target="timeout_failed"/>
  </parallel>
  <final id="timeout_failed"/>
</scxml>"##;

    fn render_root_with_barrier_timeout(timeout_ms: Option<u32>) -> String {
        let mut model = parse(PARALLEL_FINAL_WITH_TIMEOUT_HANDLER_SCXML);
        crate::analyzer::analyze(&mut model, "pf_fixture.scxml");
        model.partition_context_present = true;
        model
            .partition_parallel_roles
            .insert("root".to_string(), crate::model::PartitionRole::Root);
        if let Some(ms) = timeout_ms {
            model
                .partition_barrier_timeouts
                .insert("root".to_string(), ms);
        }
        let template_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("workspace root")
            .join("tools")
            .join("codegen")
            .join("templates");
        let out = generate_cpp(&model, &template_dir, "pf_fixture").expect("render");
        out.files
            .into_iter()
            .map(|(_, body)| body)
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn partition_role_root_emits_tracker_and_handlers() {
        let body = render_with_role(Some(crate::model::PartitionRole::Root));
        assert!(
            body.contains("tracker_root_"),
            "Root role must emit ParallelCompletionTracker member `tracker_root_`; body was:\n{body}"
        );
        assert!(
            body.contains("onParallelRegionDone"),
            "Root role must emit the `onParallelRegionDone` wire-21 receiver method"
        );
        // §16.5 wire-21 typed envelope — receiver dispatches on the typed
        // `env.parallel_id` / `env.region_id` (CBOR keys 16/17) without
        // string concat/split. The earlier `subject = "pid/rid"` +
        // `subject.find('/')` + `substr` path is retired.
        assert!(
            body.contains("env.parallel_id.has_value()") && body.contains("env.region_id.has_value()"),
            "Root wire-21 receiver must gate on both typed optional fields `parallel_id` and `region_id`; body was:\n{body}"
        );
        assert!(
            !body.contains("env.subject.has_value()"),
            "Root wire-21 receiver must not read `env.subject` — migrated to typed fields (§16.5)"
        );
        assert!(
            !body.contains("subject.find('/')") && !body.contains("subject.substr"),
            "Root wire-21 receiver must not string-parse a `subject` field — dispatch is on typed fields"
        );
        // §16.5 dispatch path: `parallel_final.jinja2` calls
        // `engine.triggerParallelRegionLocalComplete(parallel_id, region_id)`
        // (a base StaticExecutionEngine method); the SM ctor's
        // `setParallelRegionLocalCompleteCallback` closure routes that
        // through `tracker_<pid>_.onLocalRegionComplete(region_id)`.
        // Asserting both call sites pins the full dispatch path.
        assert!(
            body.contains("triggerParallelRegionLocalComplete(\"root\", \"left\")"),
            "Root region-final branch must dispatch via `engine.triggerParallelRegionLocalComplete`; body was:\n{body}"
        );
        assert!(
            body.contains("setParallelRegionLocalCompleteCallback"),
            "Root SM ctor must install the local-complete callback on the base via `setParallelRegionLocalCompleteCallback`"
        );
        assert!(
            body.contains("tracker_root_.onLocalRegionComplete(region_id)"),
            "Root SM ctor closure must terminate the dispatch in `tracker_root_.onLocalRegionComplete(region_id)`"
        );
        assert!(
            !body.contains("sendParallelRegionDone"),
            "Root role must NOT emit the non-root sender hook — the region is local"
        );
    }

    #[test]
    fn partition_role_non_root_emits_wire21_sender_only() {
        let body = render_with_role(Some(crate::model::PartitionRole::NonRoot));
        assert!(
            body.contains("sendParallelRegionDone"),
            "NonRoot role must emit the `sendParallelRegionDone` wire-21 sender method"
        );
        assert!(
            body.contains("PatternKind::ParallelRegionDone"),
            "NonRoot sender body must construct the wire-21 envelope"
        );
        // §16.5 wire-21 typed envelope — sender assigns BOTH typed fields
        // (`env.parallel_id`, `env.region_id`) on every outbound, replacing
        // the earlier `env.subject = parallel_id + "/" + region_id` concat.
        assert!(
            body.contains("env.parallel_id = parallel_id"),
            "NonRoot sender must set typed `env.parallel_id` on the wire-21 envelope; body was:\n{body}"
        );
        assert!(
            body.contains("env.region_id = region_id"),
            "NonRoot sender must set typed `env.region_id` on the wire-21 envelope"
        );
        assert!(
            !body.contains("env.subject = parallel_id"),
            "NonRoot sender must not populate `env.subject` for wire-21 — migrated to typed fields (§16.5)"
        );
        // §16.5 dispatch path mirrors the Root assertions: the
        // `parallel_final.jinja2` body calls
        // `engine.triggerParallelRegionRemoteSend(parallel_id, region_id, donedata)`,
        // and the SM ctor's `setParallelRegionRemoteSendCallback`
        // closure terminates the dispatch in `sendParallelRegionDone`.
        assert!(
            body.contains("triggerParallelRegionRemoteSend"),
            "NonRoot region-final branch must dispatch via `engine.triggerParallelRegionRemoteSend`; body was:\n{body}"
        );
        assert!(
            body.contains("setParallelRegionRemoteSendCallback"),
            "NonRoot SM ctor must install the remote-send callback on the base via `setParallelRegionRemoteSendCallback`"
        );
        assert!(
            !body.contains("tracker_root_"),
            "NonRoot role must NOT emit a tracker — aggregation is the root's job"
        );
        assert!(
            !body.contains("onParallelRegionDone"),
            "NonRoot role must NOT emit the receiver hook — envelopes land on the root"
        );
        assert!(
            !body.contains("ParallelCompletionHelper::areAllRegionsInFinal"),
            "NonRoot must not fall back to single-partition legacy completion check"
        );
    }

    #[test]
    fn partition_context_absent_falls_back_to_single_partition() {
        // `partition_context_present=false` → template's outer `{% if %}`
        // does not include the delegate; single-partition AOT path is
        // byte-identical to pre-mesh legacy.
        let body = render_with_role(None);
        assert!(
            body.contains("ParallelCompletionHelper::areAllRegionsInFinal"),
            "Default path must emit the legacy single-partition completion check"
        );
        assert!(
            !body.contains("tracker_root_"),
            "Default path must not emit mesh tracker members"
        );
        assert!(
            !body.contains("sendParallelRegionDone"),
            "Default path must not emit mesh sender hooks"
        );
    }

    #[test]
    fn partition_role_single_partition_preserves_legacy_path() {
        // A partitioned machine whose `<parallel>` lives entirely in one
        // partition (SinglePartition role) still uses the legacy helper.
        let body = render_with_role(Some(crate::model::PartitionRole::SinglePartition));
        assert!(
            body.contains("ParallelCompletionHelper::areAllRegionsInFinal"),
            "SinglePartition role must use the legacy completion helper"
        );
        assert!(
            !body.contains("tracker_root_"),
            "SinglePartition role must not emit mesh tracker members"
        );
    }

    // ── §16.5 L3500 barrier-timeout shape + observability gate ───

    #[test]
    fn partition_barrier_timeout_absent_emits_no_timer_machinery() {
        // Root role without `partition_barrier_timeouts` ⇒ W3C
        // normative infinity ⇒ no TimerHooks, no scheduler call,
        // no `PARALLEL_BARRIER_TIMEOUT` string.
        let body = render_root_with_barrier_timeout(None);
        assert!(
            body.contains("tracker_root_"),
            "Root role must still emit the tracker member"
        );
        assert!(
            !body.contains("TimerHooks"),
            "infinity (no barrier_timeout_ms) must not emit `TimerHooks`; body was:\n{body}"
        );
        assert!(
            !body.contains("PARALLEL_BARRIER_TIMEOUT"),
            "infinity must not emit the §16.7 row 6 reason string"
        );
        assert!(
            !body.contains("__sce_barrier_timeout_"),
            "infinity must not emit the deterministic timer send-id constant"
        );
    }

    #[test]
    fn partition_barrier_timeout_present_emits_timer_hooks_and_raise() {
        // Root role with a finite `barrier_timeout_ms` ⇒ TimerHooks
        // block populated, arm/cancel call through `scheduleEvent` /
        // `cancelEvent` with the deterministic send-id, payload shaped
        // by `CommunicationError::toJsonBytes`.
        let body = render_root_with_barrier_timeout(Some(3500));
        assert!(
            body.contains("ParallelCompletionTracker::TimerHooks"),
            "finite barrier_timeout_ms must emit the TimerHooks aggregate; body was:\n{body}"
        );
        assert!(
            body.contains("PARALLEL_BARRIER_TIMEOUT"),
            "finite barrier_timeout_ms must pin the §16.7 row 6 reason string"
        );
        assert!(
            body.contains("__sce_barrier_timeout_root"),
            "arm/cancel must route through a deterministic per-parallel send-id"
        );
        assert!(
            body.contains("CommunicationError"),
            "JSON payload must be shaped via CommunicationError::toJsonBytes"
        );
        assert!(
            body.contains("PolicyType::Event::Error_communication"),
            "timer-fire event must be the W3C-bridged Event::Error_communication"
        );
        assert!(
            body.contains("Error_communication") && body.contains("scheduleEvent"),
            "arm callback must call the base engine's scheduleEvent with the error event"
        );
        assert!(
            body.contains("3500"),
            "timeout_ms must be baked in verbatim from deploy.yaml"
        );
    }

    #[test]
    fn partition_barrier_timeout_without_error_handler_rejects() {
        // Same fixture but WITHOUT the `error.communication` transition
        // — codegen must refuse so the silent-broken observability
        // gap (`feedback_silently_broken_hooks`) is closed at build
        // time instead of as a mysterious no-op at runtime.
        let mut model = parse(PARALLEL_FINAL_SCXML);
        crate::analyzer::analyze(&mut model, "pf_fixture.scxml");
        model.partition_context_present = true;
        model
            .partition_parallel_roles
            .insert("root".to_string(), crate::model::PartitionRole::Root);
        model
            .partition_barrier_timeouts
            .insert("root".to_string(), 1000);
        let template_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("workspace root")
            .join("tools")
            .join("codegen")
            .join("templates");
        let res = generate_cpp(&model, &template_dir, "pf_fixture");
        let err = match res {
            Ok(_) => panic!("barrier_timeout_ms without error.communication handler must reject"),
            Err(e) => e,
        };
        match err {
            GenerateError::UnsupportedFeature(msg) => {
                assert!(msg.contains("barrier_timeout_ms"), "msg cites the knob: {msg}");
                assert!(msg.contains("error.communication"), "msg names the missing handler: {msg}");
                assert!(msg.contains("PARALLEL_BARRIER_TIMEOUT"), "msg names §16.7 row 6 reason: {msg}");
                // Machine name is whatever the test parser assigned via
                // `parse_string(..., "brake")`; the assertion only cares
                // that SOME machine identifier is surfaced.
                assert!(msg.contains("machine"), "msg names the compiled machine: {msg}");
                assert!(msg.contains("root"), "msg names the parallel id: {msg}");
            }
            other => panic!("expected UnsupportedFeature, got {other:?}"),
        }
    }

    #[test]
    fn machine_liveliness_without_error_handler_rejects() {
        // Symmetric to `partition_barrier_timeout_without_error_handler_rejects`
        // for the §16.4 / §16.7 liveness raise paths. `machine_liveliness_opt_in=true`
        // with no `<transition event="error.communication">` in the SCXML
        // must be refused at codegen — the `feedback_silently_broken_hooks`
        // gate covers both row 8 (`PEER_PARTITIONED`, non-partitioned) and
        // row 13 (`REGION_PARTITIONED`, partitioned) because both rows
        // surface through `error.communication` and share the same
        // silent-broken failure mode. Partitioned and non-partitioned
        // fixtures both probed so the gate is never dead for either axis.
        for &(context_present, label) in
            &[(true, "partitioned"), (false, "non-partitioned")]
        {
            let mut model = parse(PARALLEL_FINAL_SCXML);
            crate::analyzer::analyze(&mut model, "pf_fixture.scxml");
            model.partition_context_present = context_present;
            if context_present {
                model
                    .partition_parallel_roles
                    .insert("root".to_string(), crate::model::PartitionRole::Root);
            }
            model.machine_liveliness_opt_in = true;
            let template_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .expect("workspace root")
                .join("tools")
                .join("codegen")
                .join("templates");
            let res = generate_cpp(&model, &template_dir, "pf_fixture");
            let err = match res {
                Ok(_) => panic!(
                    "machine_liveliness_opt_in without error.communication handler must reject \
                     ({label})"
                ),
                Err(e) => e,
            };
            match err {
                GenerateError::UnsupportedFeature(msg) => {
                    assert!(msg.contains("liveliness"), "{label}: msg cites the knob: {msg}");
                    assert!(
                        msg.contains("error.communication"),
                        "{label}: msg names the missing handler: {msg}"
                    );
                    // Gate speaks for both rows; pin both reason codes so
                    // a future narrowing (e.g. re-splitting the gate) has
                    // to update the test intentionally rather than drift.
                    assert!(
                        msg.contains("PEER_PARTITIONED"),
                        "{label}: msg names §16.7 row 8 reason: {msg}"
                    );
                    assert!(
                        msg.contains("REGION_PARTITIONED"),
                        "{label}: msg names §16.7 row 13 reason: {msg}"
                    );
                    // Machine name is whatever `<scxml name=...>` carries
                    // in PARALLEL_FINAL_SCXML — the assertion only pins
                    // that SOME machine identifier is surfaced, not the
                    // exact string.
                    assert!(
                        msg.contains("machine"),
                        "{label}: msg names the compiled machine: {msg}"
                    );
                }
                other => panic!("{label}: expected UnsupportedFeature, got {other:?}"),
            }
        }
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
        match generate_with_templates(&model, templates, false) {
            Err(GenerateError::UnsupportedFeature(_)) => {
                panic!("plain SCXML must not trip the mesh-rpc gate")
            }
            // Anything else (Ok / template error / etc.) means the
            // gate let the model through, which is the contract.
            _ => {}
        }
    }
}

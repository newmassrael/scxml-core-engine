// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// WASM entry points for browser-based SCXML code generation.
// Embeds all Jinja2 templates (Rust, C++, Kotlin) at compile time via include_str!.

use wasm_bindgen::prelude::*;

// Template lists are hardcoded because include_str! requires compile-time string literals.
// When adding new templates, update both the filesystem templates and this list.
//
// SYNC VERIFICATION — run from sce-build/ to detect missing templates:
//   for lang in rust kotlin; do
//     diff <(find ../tools/codegen/templates/$lang -name '*.jinja2' -printf '%P\n' | sort) \
//          <(grep -oP '"[^"]+\.jinja2"' src/wasm.rs | tr -d '"' | grep -v '^state_machine\.' | \
//            grep -v '^actions/' | sort) && echo "$lang: OK" || echo "$lang: MISMATCH"
//   done
// For C++ (templates at root level):
//   diff <(find ../tools/codegen/templates -maxdepth 1 -name '*.jinja2' -printf '%P\n' | sort; \
//          find ../tools/codegen/templates/actions -maxdepth 1 -name '*.jinja2' -printf 'actions/%P\n' | sort) \
//        <(grep -oP '"[^"]+\.jinja2"' src/wasm.rs | tr -d '"' | grep -v '\.rs\.' | grep -v '\.kt\.' | sort)

// ── Rust templates ──────────────────────────────────────────────

fn rust_templates() -> &'static [(&'static str, &'static str)] {
    &[
        ("state_machine.rs.jinja2", include_str!("../../tools/codegen/templates/rust/state_machine.rs.jinja2")),
        ("process_transition.rs.jinja2", include_str!("../../tools/codegen/templates/rust/process_transition.rs.jinja2")),
        ("entry_exit_actions.rs.jinja2", include_str!("../../tools/codegen/templates/rust/entry_exit_actions.rs.jinja2")),
        ("invoke_methods.rs.jinja2", include_str!("../../tools/codegen/templates/rust/invoke_methods.rs.jinja2")),
        ("scriptengine_helpers.rs.jinja2", include_str!("../../tools/codegen/templates/rust/scriptengine_helpers.rs.jinja2")),
        ("utility_methods.rs.jinja2", include_str!("../../tools/codegen/templates/rust/utility_methods.rs.jinja2")),
        ("license_header.rs.jinja2", include_str!("../../tools/codegen/templates/rust/license_header.rs.jinja2")),
        ("datamodel_macros.rs.jinja2", include_str!("../../tools/codegen/templates/rust/datamodel_macros.rs.jinja2")),
        ("conflict_resolution.rs.jinja2", include_str!("../../tools/codegen/templates/rust/conflict_resolution.rs.jinja2")),
        ("actions/assign.rs.jinja2", include_str!("../../tools/codegen/templates/rust/actions/assign.rs.jinja2")),
        ("actions/cancel.rs.jinja2", include_str!("../../tools/codegen/templates/rust/actions/cancel.rs.jinja2")),
        ("actions/foreach.rs.jinja2", include_str!("../../tools/codegen/templates/rust/actions/foreach.rs.jinja2")),
        ("actions/if.rs.jinja2", include_str!("../../tools/codegen/templates/rust/actions/if.rs.jinja2")),
        ("actions/log.rs.jinja2", include_str!("../../tools/codegen/templates/rust/actions/log.rs.jinja2")),
        ("actions/raise.rs.jinja2", include_str!("../../tools/codegen/templates/rust/actions/raise.rs.jinja2")),
        ("actions/script.rs.jinja2", include_str!("../../tools/codegen/templates/rust/actions/script.rs.jinja2")),
        ("actions/send.rs.jinja2", include_str!("../../tools/codegen/templates/rust/actions/send.rs.jinja2")),
    ]
}

// ── C++ templates ───────────────────────────────────────────────

fn cpp_templates() -> &'static [(&'static str, &'static str)] {
    &[
        ("state_machine.jinja2", include_str!("../../tools/codegen/templates/state_machine.jinja2")),
        ("state_machine_inl.jinja2", include_str!("../../tools/codegen/templates/state_machine_inl.jinja2")),
        ("process_transition.jinja2", include_str!("../../tools/codegen/templates/process_transition.jinja2")),
        ("entry_exit_actions.jinja2", include_str!("../../tools/codegen/templates/entry_exit_actions.jinja2")),
        ("invoke_methods.jinja2", include_str!("../../tools/codegen/templates/invoke_methods.jinja2")),
        ("scriptengine_helpers.jinja2", include_str!("../../tools/codegen/templates/scriptengine_helpers.jinja2")),
        ("utility_methods.jinja2", include_str!("../../tools/codegen/templates/utility_methods.jinja2")),
        ("license_header.jinja2", include_str!("../../tools/codegen/templates/license_header.jinja2")),
        ("datamodel_macros.jinja2", include_str!("../../tools/codegen/templates/datamodel_macros.jinja2")),
        ("conflict_resolution.jinja2", include_str!("../../tools/codegen/templates/conflict_resolution.jinja2")),
        ("actions/assign.jinja2", include_str!("../../tools/codegen/templates/actions/assign.jinja2")),
        ("actions/cancel.jinja2", include_str!("../../tools/codegen/templates/actions/cancel.jinja2")),
        ("actions/else.jinja2", include_str!("../../tools/codegen/templates/actions/else.jinja2")),
        ("actions/elseif.jinja2", include_str!("../../tools/codegen/templates/actions/elseif.jinja2")),
        ("actions/foreach.jinja2", include_str!("../../tools/codegen/templates/actions/foreach.jinja2")),
        ("actions/if.jinja2", include_str!("../../tools/codegen/templates/actions/if.jinja2")),
        ("actions/log.jinja2", include_str!("../../tools/codegen/templates/actions/log.jinja2")),
        ("actions/raise.jinja2", include_str!("../../tools/codegen/templates/actions/raise.jinja2")),
        ("actions/script.jinja2", include_str!("../../tools/codegen/templates/actions/script.jinja2")),
        ("actions/send.jinja2", include_str!("../../tools/codegen/templates/actions/send.jinja2")),
    ]
}

// ── Kotlin templates ────────────────────────────────────────────

fn kotlin_templates() -> &'static [(&'static str, &'static str)] {
    &[
        ("state_machine.kt.jinja2", include_str!("../../tools/codegen/templates/kotlin/state_machine.kt.jinja2")),
        ("process_event.kt.jinja2", include_str!("../../tools/codegen/templates/kotlin/process_event.kt.jinja2")),
        ("entry_exit_actions.kt.jinja2", include_str!("../../tools/codegen/templates/kotlin/entry_exit_actions.kt.jinja2")),
        ("scriptengine_helpers.kt.jinja2", include_str!("../../tools/codegen/templates/kotlin/scriptengine_helpers.kt.jinja2")),
        ("events.kt.jinja2", include_str!("../../tools/codegen/templates/kotlin/events.kt.jinja2")),
        ("states.kt.jinja2", include_str!("../../tools/codegen/templates/kotlin/states.kt.jinja2")),
        ("transition_actions.kt.jinja2", include_str!("../../tools/codegen/templates/kotlin/transition_actions.kt.jinja2")),
        ("actions/_render.kt.jinja2", include_str!("../../tools/codegen/templates/kotlin/actions/_render.kt.jinja2")),
        ("actions/_http_send_body.kt.jinja2", include_str!("../../tools/codegen/templates/kotlin/actions/_http_send_body.kt.jinja2")),
        ("actions/assign.kt.jinja2", include_str!("../../tools/codegen/templates/kotlin/actions/assign.kt.jinja2")),
        ("actions/cancel.kt.jinja2", include_str!("../../tools/codegen/templates/kotlin/actions/cancel.kt.jinja2")),
        ("actions/foreach.kt.jinja2", include_str!("../../tools/codegen/templates/kotlin/actions/foreach.kt.jinja2")),
        ("actions/if.kt.jinja2", include_str!("../../tools/codegen/templates/kotlin/actions/if.kt.jinja2")),
        ("actions/log.kt.jinja2", include_str!("../../tools/codegen/templates/kotlin/actions/log.kt.jinja2")),
        ("actions/raise.kt.jinja2", include_str!("../../tools/codegen/templates/kotlin/actions/raise.kt.jinja2")),
        ("actions/script.kt.jinja2", include_str!("../../tools/codegen/templates/kotlin/actions/script.kt.jinja2")),
        ("actions/send.kt.jinja2", include_str!("../../tools/codegen/templates/kotlin/actions/send.kt.jinja2")),
    ]
}

/// Select embedded templates for a language.
fn templates_for(lang: &str) -> Result<&'static [(&'static str, &'static str)], JsValue> {
    match lang {
        "rust" => Ok(rust_templates()),
        "cpp" | "c++" => Ok(cpp_templates()),
        "kotlin" | "kt" => Ok(kotlin_templates()),
        _ => Err(JsValue::from_str(&format!("Unsupported language: {lang}"))),
    }
}

/// Compile SCXML to generated code for any supported language.
///
/// Returns a JSON string: `[["filename", "code"], ...]`
/// All templates are embedded in the WASM binary — no network requests needed.
#[wasm_bindgen]
pub fn compile_scxml_lang(
    scxml_content: &str,
    scxml_name: &str,
    language: &str,
) -> Result<String, JsValue> {
    let templates = templates_for(language)?;
    let lang = language
        .parse::<crate::generator::Language>()
        .map_err(|e| JsValue::from_str(&e))?;

    let output = crate::compile_from_string_lang(scxml_content, scxml_name, &templates, lang)
        .map_err(|e| JsValue::from_str(&e))?;

    // Serialize as JSON array of [filename, code] pairs
    let files: Vec<(&str, &str)> = output.files.iter().map(|(f, c)| (f.as_str(), c.as_str())).collect();
    serde_json::to_string(&files).map_err(|e| JsValue::from_str(&format!("JSON error: {e}")))
}

/// Extract the state machine name from SCXML content.
#[wasm_bindgen]
pub fn get_machine_name(scxml_content: &str) -> Result<String, JsValue> {
    let doc = roxmltree::Document::parse(scxml_content)
        .map_err(|e| JsValue::from_str(&format!("XML parse error: {e}")))?;
    let root = doc.root_element();
    Ok(root.attribute("name").unwrap_or("untitled").to_string())
}

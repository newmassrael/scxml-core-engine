// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

//! Browser entry points for SCXML code generation.
//!
//! This crate owns no template knowledge and no generation logic. It
//! used to be a module inside `sce-build` carrying a hardcoded
//! `include_str!` list of every template, which is how the browser came
//! to generate from a different template set than the native binary:
//! the synth round that introduced the shared `_macros/` family (5-O)
//! added it to the tree and nothing added it to
//! the list, and `include_str!` cannot report a template that was never
//! named. The list now comes from `sce_build::template_registry`, which
//! `sce-build/build.rs` derives from the template tree itself.

use wasm_bindgen::prelude::*;

use sce_build::generator::Language;
use sce_build::template_registry;

/// Languages this build can generate, as the identifiers
/// [`compile_scxml_lang`] accepts.
///
/// Exported so a caller's language menu is a projection of what the
/// generator actually supports rather than a second list to keep in
/// step — the visualizer offered a Go button against a dispatcher that
/// rejected Go for exactly as long as the two were maintained apart.
#[wasm_bindgen]
pub fn supported_languages() -> Vec<String> {
    template_registry::SUPPORTED_LANGUAGES
        .iter()
        .map(|language| language.canonical_name().to_string())
        .collect()
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
    let lang = language
        .parse::<Language>()
        .map_err(|e| JsValue::from_str(&e))?;
    let templates = template_registry::embedded_templates_for(lang);

    let output = sce_build::compile_from_string_lang(scxml_content, scxml_name, &templates, lang)
        .map_err(|e| JsValue::from_str(&e))?;

    // Serialize as JSON array of [filename, code] pairs
    let files: Vec<(&str, &str)> = output
        .files
        .iter()
        .map(|(f, c)| (f.as_str(), c.as_str()))
        .collect();
    serde_json::to_string(&files).map_err(|e| JsValue::from_str(&format!("JSON error: {e}")))
}

/// What the diagram must be told to draw the annotation family — NL→IR
/// closure ledger row G2.
///
/// Returns the JSON of [`sce_build::annotation_overlay::AnnotationOverlay`]:
/// every node with the requirement ids it claims (empty where nothing
/// claims it), and per requirement the dependency closure its evidence
/// rests on.
///
/// ⭐ Exported for the reason this crate exists at all. Its header records
/// the browser once generating from a different template set than the
/// native binary, because the list lived here instead of in the library;
/// the same shape is waiting here in a worse form. A requirement's
/// evidence is NOT the nodes carrying its id — measured over the real
/// pair, 21 of 65 dependency pairs lie on nodes that carry none — so a
/// browser that picked nodes by `sce:req` would draw a different answer
/// from the acceptance report's, and a rendered diagram is not bytes a
/// test can diff. The closure therefore crosses this boundary as DATA,
/// derived by the report's own function, and this function owns no part
/// of deriving it.
#[wasm_bindgen]
pub fn annotation_overlay(scxml_content: &str, scxml_name: &str) -> Result<String, JsValue> {
    let mut parser = sce_build::parser::SCXMLParser::new();
    let model = parser
        .parse_string(scxml_content, scxml_name)
        .map_err(|e| JsValue::from_str(&format!("{e}")))?;
    let overlay = sce_build::annotation_overlay::overlay(&model);
    serde_json::to_string(&overlay).map_err(|e| JsValue::from_str(&format!("JSON error: {e}")))
}

/// The statechart structure the visualizer draws, from this crate's
/// model rather than the C++ engine's parser — so the GUI and every other
/// surface a reviewer reads show one interpretation. The contract is the
/// C++ builder's; see [`sce_build::gui_structure`].
#[wasm_bindgen]
pub fn gui_structure(scxml_content: &str, scxml_name: &str) -> Result<String, JsValue> {
    let mut parser = sce_build::parser::SCXMLParser::new();
    let model = parser
        .parse_string(scxml_content, scxml_name)
        .map_err(|e| JsValue::from_str(&format!("{e}")))?;
    serde_json::to_string(&sce_build::gui_structure::gui_structure(&model))
        .map_err(|e| JsValue::from_str(&format!("JSON error: {e}")))
}

/// Extract the state machine name from SCXML content.
#[wasm_bindgen]
pub fn get_machine_name(scxml_content: &str) -> Result<String, JsValue> {
    let doc = roxmltree::Document::parse(scxml_content)
        .map_err(|e| JsValue::from_str(&format!("XML parse error: {e}")))?;
    let root = doc.root_element();
    Ok(root.attribute("name").unwrap_or("untitled").to_string())
}

/// What the Interpreter runs for `scxml_content`: the document itself or, for
/// a `datamodel="sce-static"` one, the ECMAScript document that means the
/// same ([`sce_build::forge::static_js::lower_source`]).
///
/// The Interpreter refuses a `sce-static` document because its script engine
/// holds no types, so the visualizer lowers before it loads. The structure it
/// draws still comes from [`gui_structure`] on the original text: lowering
/// replaces expressions and nothing else, so the state ids and transition
/// positions the live highlight joins on are the same in both. A document
/// under any other data model is not parsed by this crate's reader and is
/// returned as it is.
///
/// What the lowering cannot spell, and a document that reaches beside itself
/// (an import, a hybrid `<invoke>`), is refused with the product's own
/// message; nothing falls back to running it unlowered.
fn lowered_for_interpreter(scxml_content: &str, scxml_name: &str) -> Result<String, String> {
    let doc =
        roxmltree::Document::parse(scxml_content).map_err(|e| format!("XML parse error: {e}"))?;
    if doc.root_element().attribute("datamodel") != Some("sce-static") {
        return Ok(scxml_content.to_string());
    }
    sce_build::forge::static_js::lower_source(scxml_content, scxml_name)
        .map_err(|e| format!("{e:?}"))
}

/// The document the Interpreter runs for `scxml_content`; see
/// [`lowered_for_interpreter`].
#[wasm_bindgen]
pub fn lower_for_interpreter(scxml_content: &str, scxml_name: &str) -> Result<String, JsValue> {
    lowered_for_interpreter(scxml_content, scxml_name).map_err(|e| JsValue::from_str(&e))
}

#[cfg(test)]
mod tests {
    use super::lowered_for_interpreter;

    const COUNTER: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" datamodel="sce-static" initial="a" name="counter">
  <datamodel><data id="n" sce:type="uint8" expr="0"/></datamodel>
  <state id="a"><transition event="go" target="a"><assign location="n" expr="n + 1"/></transition></state>
</scxml>"#;

    #[test]
    fn a_static_document_is_run_as_the_ecmascript_that_means_the_same() {
        let lowered = lowered_for_interpreter(COUNTER, "counter").expect("lowers");
        assert!(lowered.contains(r#"datamodel="ecmascript""#), "{lowered}");
        assert!(lowered.contains("SceStatic.U8.add(n, 1)"), "{lowered}");
        assert!(!lowered.contains(r#"datamodel="sce-static""#), "{lowered}");
    }

    #[test]
    fn a_document_under_another_data_model_is_returned_as_it_is() {
        let document = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="ecmascript"><final id="f"/></scxml>"#;
        assert_eq!(
            lowered_for_interpreter(document, "plain").unwrap(),
            document
        );
    }

    #[test]
    fn a_document_it_cannot_lower_is_refused_by_name_not_passed_through() {
        let document = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" datamodel="sce-static" initial="s">
  <datamodel><data id="n" sce:type="uint32" expr="0"/></datamodel>
  <state id="s"><transition event="go" target="d"/></state>
  <final id="d"><donedata><content><result code="7">finished</result></content></donedata></final>
</scxml>"#;
        let refusal = lowered_for_interpreter(document, "refused").expect_err("refused");
        assert!(refusal.contains("holding an element"), "{refusal}");
    }
}

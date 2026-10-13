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

use std::collections::BTreeMap;

use sce_build::forge::static_js::LoweredDocument;
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

/// The namespace the extension elements and attributes are written in.
const SCE_NAMESPACE: &str = "http://sce.dev/ext";

/// The documents `scxml_content` names beside itself, by the path it writes: what
/// an `<sce:import src>` imports and what `sce:candidates` lists. The built-in
/// standard library (`sce:std/...`) is not beside anything and is left out.
///
/// Read from the elements and attributes by namespace and not by the prefix an
/// author chose, and on any document of the kind, a statechart or an algorithm,
/// so a caller that holds no file system can fetch what the lowering will ask for
/// before it asks, a document at a time, down to the ones those name.
fn documents_named_by_content(scxml_content: &str) -> Result<Vec<String>, String> {
    let doc =
        roxmltree::Document::parse(scxml_content).map_err(|e| format!("XML parse error: {e}"))?;
    let mut named = Vec::new();
    for node in doc.descendants().filter(|n| n.is_element()) {
        let tag = node.tag_name();
        if tag.namespace() == Some(SCE_NAMESPACE) && tag.name() == "import" {
            if let Some(src) = node.attribute("src") {
                named.push(src.to_string());
            }
        }
        if let Some(list) = node.attribute((SCE_NAMESPACE, "candidates")) {
            named.extend(list.split_whitespace().map(str::to_string));
        }
    }
    let mut seen = std::collections::BTreeSet::new();
    named.retain(|name| !name.starts_with("sce:std/") && seen.insert(name.clone()));
    Ok(named)
}

/// The documents `scxml_content` names beside itself, as a JSON array of paths;
/// see [`documents_named_by_content`].
#[wasm_bindgen]
pub fn documents_named_by(scxml_content: &str) -> Result<String, JsValue> {
    let named = documents_named_by_content(scxml_content).map_err(|e| JsValue::from_str(&e))?;
    serde_json::to_string(&named).map_err(|e| JsValue::from_str(&format!("JSON error: {e}")))
}

/// What the Interpreter runs for `scxml_content`: the document itself or, for
/// a `datamodel="sce-static"` one, the ECMAScript document that means the
/// same ([`sce_build::forge::static_js::lower_documents`]), with every
/// document its hybrid `<invoke>`s may start. The first is the document asked
/// for, under `<scxml_name>.scxml`; the rest stand beside it under the names the
/// lowered document starts them by.
///
/// The Interpreter refuses a `sce-static` document because its script engine
/// holds no types, so the visualizer lowers before it loads. The structure it
/// draws still comes from [`gui_structure`] on the original text: lowering
/// replaces expressions and nothing else, so the state ids and transition
/// positions the live highlight joins on are the same in both. A document
/// under any other data model is not parsed by this crate's reader and is
/// returned as it is.
///
/// `siblings` are the documents that stand beside it, by the name it reaches
/// them under: an imported algorithm or schema, an include, a candidate. The
/// built-in standard library (`sce:std/...`) needs none. A name that is not here
/// is a file that is not there.
///
/// What the lowering cannot spell is refused with the product's own message;
/// nothing falls back to running it unlowered.
fn lowered_for_interpreter(
    scxml_content: &str,
    scxml_name: &str,
    siblings: &BTreeMap<String, String>,
) -> Result<Vec<LoweredDocument>, String> {
    let file_name = format!("{scxml_name}.scxml");
    let doc =
        roxmltree::Document::parse(scxml_content).map_err(|e| format!("XML parse error: {e}"))?;
    if doc.root_element().attribute("datamodel") != Some("sce-static") {
        return Ok(vec![LoweredDocument {
            file_name,
            text: scxml_content.to_string(),
        }]);
    }
    let mut held = siblings.clone();
    held.insert(file_name.clone(), scxml_content.to_string());
    sce_build::forge::static_js::lower_documents(&file_name, &held).map_err(|e| format!("{e:?}"))
}

/// The documents the Interpreter runs for `scxml_content`, as JSON:
/// `[{"file_name": ..., "text": ...}, ...]`, the document asked for first.
/// `siblings_json` is an object of the documents beside it, by name. See
/// [`lowered_for_interpreter`].
#[wasm_bindgen]
pub fn lower_for_interpreter(
    scxml_content: &str,
    scxml_name: &str,
    siblings_json: &str,
) -> Result<String, JsValue> {
    let siblings: BTreeMap<String, String> = serde_json::from_str(siblings_json).map_err(|e| {
        JsValue::from_str(&format!(
            "the sibling documents are not an object of text: {e}"
        ))
    })?;
    let lowered = lowered_for_interpreter(scxml_content, scxml_name, &siblings)
        .map_err(|e| JsValue::from_str(&e))?;
    let documents: Vec<_> = lowered
        .into_iter()
        .map(|d| serde_json::json!({ "file_name": d.file_name, "text": d.text }))
        .collect();
    serde_json::to_string(&documents).map_err(|e| JsValue::from_str(&format!("JSON error: {e}")))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use sce_build::forge::static_js::LoweredDocument;

    fn lowered_for_interpreter(content: &str, name: &str) -> Result<String, String> {
        let mut set = lowered_with_siblings(content, name, &BTreeMap::new())?;
        assert_eq!(set.len(), 1, "one document and nothing it starts");
        Ok(set.remove(0).text)
    }

    fn lowered_with_siblings(
        content: &str,
        name: &str,
        siblings: &BTreeMap<String, String>,
    ) -> Result<Vec<LoweredDocument>, String> {
        super::lowered_for_interpreter(content, name, siblings)
    }

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

    const HYBRID: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" datamodel="sce-static" initial="s">
  <datamodel><data id="pick" sce:type="string" sce:capacity="32" expr="'a.scxml'"/></datamodel>
  <state id="s"><invoke type="scxml" id="c" srcexpr="pick" sce:candidates="a.scxml"/></state>
</scxml>"#;

    const CANDIDATE: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" datamodel="sce-static" initial="w">
  <datamodel><data id="t" sce:type="uint8" expr="0"/></datamodel>
  <state id="w"/>
</scxml>"#;

    #[test]
    fn a_hybrid_invoke_brings_the_candidates_it_may_start_beside_the_document() {
        let siblings = BTreeMap::from([("a.scxml".to_string(), CANDIDATE.to_string())]);
        let set = lowered_with_siblings(HYBRID, "parent", &siblings).expect("lowers");
        let names: Vec<_> = set.iter().map(|d| d.file_name.as_str()).collect();
        assert_eq!(names, ["parent.scxml", "a.scxml"]);
        assert!(set
            .iter()
            .all(|d| d.text.contains(r#"datamodel="ecmascript""#)));
    }

    #[test]
    fn a_candidate_that_is_not_among_the_siblings_is_a_file_that_is_not_there() {
        let refusal = lowered_with_siblings(HYBRID, "parent", &BTreeMap::new())
            .expect_err("a.scxml is not here");
        assert!(refusal.contains("a.scxml"), "{refusal}");
    }

    #[test]
    fn the_documents_a_document_names_are_its_imports_and_candidates() {
        let document = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:x="http://sce.dev/ext" version="1.0" datamodel="sce-static" initial="s">
  <x:import kind="event-schema" src="schema_day.scxml" as="Day"/>
  <x:import kind="algorithm" src="sce:std/time/days_in_month.scxml" as="Days"/>
  <x:import kind="algorithm" src="sub/algo.scxml" as="Algo"/>
  <state id="s"><invoke type="scxml" id="c" srcexpr="pick" x:candidates="a.scxml b.scxml"/></state>
</scxml>"#;
        assert_eq!(
            super::documents_named_by_content(document).unwrap(),
            ["schema_day.scxml", "sub/algo.scxml", "a.scxml", "b.scxml"]
        );
        // A document that names nothing beside itself asks for nothing.
        assert!(super::documents_named_by_content(COUNTER)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn the_standard_library_needs_no_sibling() {
        let document = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" datamodel="sce-static" initial="s">
  <sce:import kind="algorithm" src="sce:std/time/days_in_month.scxml" as="Days"/>
  <datamodel><data id="n" sce:type="uint8" expr="0"/></datamodel>
  <state id="s"><transition event="go" target="s"><assign location="n" expr="Days(2024, 2)"/></transition></state>
</scxml>"#;
        let lowered = lowered_for_interpreter(document, "days").expect("lowers");
        assert!(lowered.contains("SceStatic.algorithms."), "{lowered}");
    }
}

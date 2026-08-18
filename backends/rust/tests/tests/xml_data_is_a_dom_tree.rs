// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML B.2: a `<data>` element's XML content is a DOM structure a
// document can walk — Rust AOT path.
//
// The appendix obliges the Processor to create "the corresponding DOM
// structure". Measured 2026-08-18, every backend created an object carrying
// three methods — `getElementsByTagName`, `getAttribute` and a non-standard
// `getTagName`, which are the two names the W3C IRP suite reads plus one — so
// `doc.tagName`, `doc.firstChild` and `doc.childNodes.length` answered nil on
// all seven channels with 204/204 W3C fixtures green.
//
// What this fixture adds to `backends/rust/lua/tests/dom_read_surface.rs`,
// which measures the same surface against the same shared table, is the SEAM:
// the `<data>` initializer the code generator emits, and the guards it lowered.
// The binding being right does not say a document reaches it.
//
// Fixture: integration_resources/xml_data_is_a_dom_tree/xml_data_is_a_dom_tree.scxml
// (canonical, shared with the C++ / C11 / Go / Kotlin / Python channels).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_xml_data_is_a_dom_tree.sh

use sce_rust_tests::integration::xml_data_is_a_dom_tree::{
    XmlDataIsADomTreePolicy as Policy, XmlDataIsADomTreeState as State,
};

#[test]
fn a_data_elements_xml_is_a_dom_tree_the_document_can_walk() {
    // The fixture reads the DOM in its guards, so this is an
    // ECMAScript-datamodel machine.
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let mut engine = sce_rust_runtime::Engine::new(Policy::new(script_engine));
    engine.initialize();

    // Every transition is eventless, so the verdict is reached in the first
    // macrostep and no event is needed to ask the question.
    engine.step();

    let active = engine.get_active_states();
    assert!(
        !active.contains(&State::NotADocument),
        "the variable did not hold a document: `doc.nodeType === 9`, \
         `doc.nodeName === '#document'`, `doc.documentElement.tagName === 'books'` \
         or `doc.hasAttribute('count')` did not hold (active: {active:?})"
    );
    assert!(
        !active.contains(&State::WrongTree),
        "the document element's children are not the two `<book>` elements in \
         document order — the whitespace between them may have become nodes, or \
         a sibling/parent link is missing (active: {active:?})"
    );
    assert!(
        !active.contains(&State::NoText),
        "character data did not report itself as a text node, or `textContent` \
         did not read the text below the element (active: {active:?})"
    );
    assert!(
        active.contains(&State::Settled),
        "the machine reached none of its four verdicts, so the guards did not \
         evaluate at all (active: {active:?})"
    );
}

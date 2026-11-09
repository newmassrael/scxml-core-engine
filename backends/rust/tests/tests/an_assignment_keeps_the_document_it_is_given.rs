// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.4 and B.2: `<assign location="v" expr="_event.data"/>` leaves
// `v` holding the DOM a `<send>`'s XML `<content>` arrived as, and so does a
// member-path location — Rust AOT.
//
// The template runs `<location> = <expr>` in the Lua engine for every
// location, so the value never leaves the engine; the C++ engines once read a
// bare-name location's value out and wrote it back, which lost the document.
// Each location that loses it lands in its own failure state.
//
// Fixture: integration_resources/an_assignment_keeps_the_document_it_is_given/an_assignment_keeps_the_document_it_is_given.scxml
// (canonical, shared with every channel).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_an_assignment_keeps_the_document_it_is_given.sh

use std::time::Duration;

use sce_rust_tests::integration::an_assignment_keeps_the_document_it_is_given::{
    AnAssignmentKeepsTheDocumentItIsGivenPolicy, AnAssignmentKeepsTheDocumentItIsGivenState,
};

#[test]
fn both_locations_hold_the_document_the_event_carried() {
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let policy = AnAssignmentKeepsTheDocumentItIsGivenPolicy::new(script_engine);
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine
        .initialize()
        .expect("the machine's script session is established");

    let completed = engine.run_until_completion(Duration::from_secs(2), Duration::from_millis(10));
    assert!(
        completed,
        "an_assignment_keeps_the_document_it_is_given timed out before reaching a final \
         state: its own `doc` event never arrived"
    );

    match engine.terminal_state() {
        Some(AnAssignmentKeepsTheDocumentItIsGivenState::Kept) => {}
        Some(AnAssignmentKeepsTheDocumentItIsGivenState::NameLost) => panic!(
            "`<assign location=\"v\" expr=\"_event.data\"/>` did not leave the document in \
             `v`: the value was read out of the data model and written back"
        ),
        Some(AnAssignmentKeepsTheDocumentItIsGivenState::MemberLost) => panic!(
            "`<assign location=\"holder.doc\" expr=\"_event.data\"/>` did not leave the \
             document in `holder.doc`"
        ),
        other => panic!(
            "an_assignment_keeps_the_document_it_is_given ended in {other:?}, which is not \
             a verdict state"
        ),
    }
}

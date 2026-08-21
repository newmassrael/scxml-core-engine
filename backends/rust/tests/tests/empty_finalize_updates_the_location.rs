// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.5.2 — what an EMPTY `<finalize>` does, and what an absent one
// does not. Rust AOT.
//
// The clause makes the empty element mean something on its own: with no
// executable content the Processor "MUST update the data model each time an
// event is received from the child process ... for each item in the
// 'namelist' attribute and each such `<param>` element ... as if by
// `<assign>` with any return value that has a name that matches", and then:
// "Note that the automatic update does not take place if the `<finalize>`
// element is absent as opposed to empty."
//
// Nothing in this repository asked it. The corpus holds two `<finalize>`
// documents (W3C 233/234) and zero empty ones, and measured 2026-08-22 the
// automatic update had no implementation either — every engine gates the
// finalize step on the content being non-empty, and the AOT model carried
// `finalize_content: String` with no way to tell an empty element from a
// missing one. The clause was unrepresentable, not merely unimplemented.
//
// Fixture: integration_resources/empty_finalize_updates_the_location/empty_finalize_updates_the_location.scxml
// (canonical, shared with the other channels).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_empty_finalize_updates_the_location.sh

use std::time::Duration;

use sce_rust_tests::integration::empty_finalize_updates_the_location::{
    EmptyFinalizeUpdatesTheLocationPolicy, EmptyFinalizeUpdatesTheLocationState,
};

#[test]
fn an_empty_finalize_updates_the_location_and_an_absent_one_does_not() {
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let policy = EmptyFinalizeUpdatesTheLocationPolicy::new(script_engine);
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();

    let completed = engine.run_until_completion(Duration::from_secs(15), Duration::from_millis(10));
    assert!(
        completed,
        "empty_finalize_updates_the_location timed out before reaching a final \
         state — even the delayed `timeoutEmpty` / `timeoutAbsent` that judge a \
         silent child never fired, so the machine is not being ticked"
    );

    match engine.get_current_state() {
        EmptyFinalizeUpdatesTheLocationState::Pass => {}
        EmptyFinalizeUpdatesTheLocationState::FailNotUpdated => panic!(
            "the empty `<finalize/>` left `tally` at its old value — W3C SCXML \
             6.5.2 makes an empty element mean the automatic update: for each \
             `namelist` item the Processor updates the location as if by \
             `<assign>` with the matching return value. Treating it as an absent \
             element is the defect the clause's own note names"
        ),
        EmptyFinalizeUpdatesTheLocationState::FailUpdatedWithoutFinalize => panic!(
            "`guard` moved with no `<finalize>` element at all — the note is a \
             prohibition: \"the automatic update does not take place if the \
             <finalize> element is absent as opposed to empty\". Wiring the \
             update to the `namelist` rather than to the empty element is what \
             this state names"
        ),
        EmptyFinalizeUpdatesTheLocationState::FailUnmatchedNameWrote => panic!(
            "an event carrying no matching name still wrote `keeper` — W3C SCXML \
             6.5.2 says \"with ANY return value that has a name that matches\", so \
             an unconditional write blanks the parent's data model on every \
             unrelated answer the child sends"
        ),
        EmptyFinalizeUpdatesTheLocationState::FailUnmatchedChildSilent => panic!(
            "the third child never answered, so the guarded-write half was never \
             exercised"
        ),
        EmptyFinalizeUpdatesTheLocationState::FailEmptyChildSilent => panic!(
            "the first child never answered, so the empty-`<finalize>` half was \
             never exercised — a different failure from getting its verdict wrong"
        ),
        EmptyFinalizeUpdatesTheLocationState::FailAbsentChildSilent => panic!(
            "the second child never answered, so the absent-`<finalize>` half was \
             never exercised"
        ),
        other => panic!(
            "empty_finalize_updates_the_location settled in {other:?}, which is \
             not a verdict state"
        ),
    }
}

// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.7: only a top-level `<final>` ends the session — Rust AOT path.
//
// Appendix D `enterStates` sets `running = false` for a `<final>` only when
// `isSCXMLElement(s.parent)`; otherwise it queues `done.state.<parent>` and
// the machine carries on. `is_final_state` is therefore the structural
// question — "is this state a `<final>` element" — while
// `Engine::is_in_final_state` answers "has this session ended", and only the
// latter may gate completion, the completion callback, and the
// `done.invoke.<id>` a parent emits for this machine.
//
// The fixture rests in the nested final rather than passing through it: a
// machine that continues within the same macrostep is only ever sampled at
// the end, where a right and a wrong predicate agree.
//
// Fixture: integration_resources/nested_final_not_terminal/nested_final_not_terminal.scxml
// (canonical, shared with the C++ / C11 / Go / Kotlin / Python channels).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_nested_final_not_terminal.sh

use std::time::Duration;

use sce_rust_tests::integration::nested_final_not_terminal::{
    NestedFinalNotTerminalEvent, NestedFinalNotTerminalPolicy, NestedFinalNotTerminalState,
};

#[test]
fn a_nested_final_does_not_end_the_session() {
    let policy = NestedFinalNotTerminalPolicy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();

    assert_eq!(
        engine.get_current_state(),
        NestedFinalNotTerminalState::PhaseDone,
        "the fixture is supposed to come to rest in the nested `<final>`; it did not, \
         so nothing below is testing what it claims"
    );
    assert!(
        !engine.is_in_final_state(),
        "the engine reported completion while resting in `phaseDone`, a `<final>` \
         nested inside `phase`. W3C SCXML Appendix D `enterStates` ends the session \
         only when the final's parent is the `<scxml>` element — a nested one \
         finishes its compound state and queues `done.state.phase`, leaving the \
         machine live. Completion must test the parent, not just `is_final_state`"
    );

    engine.raise_external(NestedFinalNotTerminalEvent::Resume, "", "");
    let completed = engine.run_until_completion(Duration::from_secs(2), Duration::from_millis(10));

    assert!(
        completed,
        "the machine did not complete after `resume` (parked in {:?})",
        engine.get_current_state()
    );
    assert_eq!(
        engine.get_current_state(),
        NestedFinalNotTerminalState::Pass,
        "`resume` did not carry the machine out of the nested final to the top-level one"
    );
}

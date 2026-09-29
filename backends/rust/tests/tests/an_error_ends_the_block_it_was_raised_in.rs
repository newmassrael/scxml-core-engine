// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 4.9: an error ends the block it was raised in — whichever element
// raised it — and no other block — Rust AOT path.
//
// Fixture: integration_resources/an_error_ends_the_block_it_was_raised_in/an_error_ends_the_block_it_was_raised_in.scxml
// (canonical, shared with the C++ / C11 / Go / Kotlin / Python channels).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_an_error_ends_the_block_it_was_raised_in.sh

use sce_rust_tests::integration::an_error_ends_the_block_it_was_raised_in::{
    AnErrorEndsTheBlockItWasRaisedInEvent as Event,
    AnErrorEndsTheBlockItWasRaisedInPolicy as Policy,
    AnErrorEndsTheBlockItWasRaisedInState as State,
};

/// The control for `a_root_start_of_a_machine_that_needs_a_parent_is_refused`:
/// this document never sends to `#_parent`, so a root-start policy lets it
/// start, and the checked start runs it as `initialize` would.
#[test]
fn a_root_start_of_a_machine_that_needs_no_parent_runs() {
    use sce_rust_runtime::Engine;
    assert_eq!(Engine::<Policy>::root_start_refusal(), None);
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let mut e = Engine::new(Policy::new(script_engine));
    assert_eq!(e.initialize_as_root(), Ok(()));
    assert!(
        e.is_running(),
        "a root start that is not refused must start the machine"
    );
}

#[test]
fn each_error_ends_only_its_own_block() {
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let mut e = sce_rust_runtime::Engine::new(Policy::new(script_engine));
    e.initialize();
    for event in [Event::T, Event::Finish] {
        e.raise_external(event, "", "");
        e.step();
    }

    assert_eq!(
        e.terminal_state(),
        Some(State::Done),
        "`finish` must carry the run to `done`"
    );
    let p = e.policy();
    let observed = [
        ("errors", p.errors(), 10),
        ("afterAssign", p.after_assign(), 0),
        ("afterScript", p.after_script(), 0),
        ("afterLog", p.after_log(), 0),
        ("afterCancel", p.after_cancel(), 0),
        ("afterIfInner", p.after_if_inner(), 0),
        ("afterIf", p.after_if(), 0),
        ("afterSingle", p.after_single(), 0),
        ("afterTrans", p.after_trans(), 0),
        ("initRan", p.init_ran(), 1),
        ("pairs", p.pairs(), 4),
        ("sum", p.sum(), 90),
        ("ifThen", p.if_then(), 0),
        ("ifElse", p.if_else(), 1),
        ("afterIfCond", p.after_if_cond(), 0),
        ("elseifThen", p.elseif_then(), 0),
        ("elseifElse", p.elseif_else(), 1),
        ("afterElseifCond", p.after_elseif_cond(), 0),
        ("afterNestedIf", p.after_nested_if(), 0),
        ("afterOuterIf", p.after_outer_if(), 0),
    ];
    for (name, got, want) in observed {
        assert_eq!(got, Some(want), "{name} = {got:?}, want {want}");
    }
}

// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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
        ("errors", p.errors(), 7),
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
    ];
    for (name, got, want) in observed {
        assert_eq!(got, Some(want), "{name} = {got:?}, want {want}");
    }
}

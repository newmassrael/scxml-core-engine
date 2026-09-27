// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 4.9 + 4.6: an error raised inside a <foreach> body ends the
// block that contains the <foreach>, and it is the only error raised —
// Rust AOT path.
//
// Fixture: integration_resources/an_error_inside_a_foreach_ends_its_block/an_error_inside_a_foreach_ends_its_block.scxml
// (canonical, shared with the C++ / C11 / Go / Kotlin / Python channels).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_an_error_inside_a_foreach_ends_its_block.sh

use sce_rust_tests::integration::an_error_inside_a_foreach_ends_its_block::{
    AnErrorInsideAForeachEndsItsBlockEvent as Event,
    AnErrorInsideAForeachEndsItsBlockPolicy as Policy,
    AnErrorInsideAForeachEndsItsBlockState as State,
};

#[test]
fn the_error_ends_the_block_and_is_the_only_one() {
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let mut e = sce_rust_runtime::Engine::new(Policy::new(script_engine));
    e.initialize();
    for event in [Event::Go, Event::T, Event::Finish] {
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
        ("iters1", p.iters1(), 1),
        ("after1", p.after1(), 0),
        ("iters2", p.iters2(), 1),
        ("after2", p.after2(), 0),
        ("iters3", p.iters3(), 1),
        ("after3", p.after3(), 0),
        ("errors", p.errors(), 3),
        ("sent", p.sent(), 2),
    ];
    for (name, got, want) in observed {
        assert_eq!(got, Some(want), "{name} = {got:?}, want {want}");
    }
}

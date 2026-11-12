// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Accepted Subset §2.15, "Child sessions", and §scxml-6.4 (docs/adr/0098)
// — Rust AOT path: a `datamodel="sce-static"` child declares its variables
// with the values its invoke handed, each in place of its own initial value,
// so a variable declared after a handed one reads the handed value.
//
// `worker` is handed `start = 7` and declares `copied = start` after it; it ends
// the moment `copied` is 7. A child that wrote the handed value over `start`
// after declaring both holds `copied = 0`, never ends, and leaves the parent in
// `working`.
//
// Fixture: tests/integration/a_static_child_declares_its_data_with_a_handed_value.scxml.
// It is outside `integration_resources/` for the reason
// `tools/codegen/trees/rust-integration.toml` states.
//
// Regeneration (after fixture or template edit):
//   scripts/materialize.sh rust-integration

use sce_rust_runtime::{Engine, SceClock};
use sce_rust_tests::integration::a_static_child_declares_its_data_with_a_handed_value::{
    AStaticChildDeclaresItsDataWithAHandedValuePolicy as Policy,
    AStaticChildDeclaresItsDataWithAHandedValueState as State,
};

#[test]
fn a_variable_declared_after_a_handed_one_reads_the_handed_value() {
    let mut engine = Engine::new(Policy::new());
    engine.set_clock(SceClock::Manual(0));
    let Ok(()) = engine.initialize();
    // Let the child start, run and end, and its parent take `done.invoke`.
    for _ in 0..5 {
        engine.tick();
    }
    assert_eq!(
        engine.terminal_state(),
        Some(State::Passed),
        "`worker` declares `copied = start` after `start`, which its invoke handed \
         7, so it ends; the parent stands in {:?}",
        engine.get_active_states()
    );
}

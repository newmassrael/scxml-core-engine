// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Accepted Subset §2.15, "Child sessions": a string an `<invoke
// type="scxml">` hands its child is held to the bound the child declared for
// that variable, in UTF-8 bytes, as an `<assign>` to it would be. A value past
// it is the evaluation that failed (W3C SCXML 5.7.1): `error.execution` is
// raised, that one value is left out, and the child still starts, holding the
// one its `<data>` gave it.
//
// `static_invoke_string.scxml` invokes three children whose `title` holds four
// bytes: `fits` is handed 'wxyz' and ends on it; `over` is handed eight bytes
// and `wide` is handed two characters of five bytes, both past the bound, so
// each starts with the 'ab' its `<data>` gave it, which it ends on. A child that
// took a value past its bound would hold it and never end. The Kotlin, Go,
// Python, C++ and C halves state the same two numbers.

use sce_rust_runtime::{Engine, SceClock};
use sce_rust_tests::integration::static_datamodel::static_invoke_string_sm::StaticInvokeStringPolicy;

/// Let the children run and report to their parent.
fn settle(engine: &mut Engine<StaticInvokeStringPolicy>) {
    for _ in 0..5 {
        engine.tick();
    }
}

#[test]
fn a_string_past_the_childs_bound_is_left_out_and_reported() {
    let mut engine = Engine::new(StaticInvokeStringPolicy::new());
    engine.set_clock(SceClock::Manual(0));
    engine.initialize();
    settle(&mut engine);
    // 1 (`fits`) + 10 (`over`) + 100 (`wide`): all three ended, so none held a
    // value past its bound but the one that fits.
    assert_eq!(engine.policy().completed(), 111);
    // The two values past the bound were reported, once each.
    assert_eq!(engine.policy().errors(), 2);
}

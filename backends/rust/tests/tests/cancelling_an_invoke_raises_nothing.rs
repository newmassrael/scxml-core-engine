// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.4: cancelling an invocation raises no event in the invoking
// session — Rust AOT path.
//
// Measured 2026-09-26, this channel raised an internal `cancel.invoke` each
// time a still-running static child was cancelled, in every document with an
// scxml `<invoke>` — the analyzer listed the event as a platform event there —
// and any transition matching it (`cancel.invoke`, `cancel.*`, `*`) took it.
//
// Fixture: integration_resources/cancelling_an_invoke_raises_nothing/cancelling_an_invoke_raises_nothing.scxml
// (canonical, shared with the C++ / C11 / Go / Kotlin / Python channels).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_cancelling_an_invoke_raises_nothing.sh

use sce_rust_tests::integration::cancelling_an_invoke_raises_nothing::{
    CancellingAnInvokeRaisesNothingEvent as Event, CancellingAnInvokeRaisesNothingPolicy as Policy,
    CancellingAnInvokeRaisesNothingState as State,
};

#[test]
fn leaving_the_invoking_state_raises_no_cancel_event() {
    // The handlers record with `<assign>`, so the policy takes an engine.
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let mut e = sce_rust_runtime::Engine::new(Policy::new(script_engine));
    e.initialize();
    assert!(
        e.get_active_states().contains(&State::P),
        "the run has to start in `p`, with its child invoked"
    );

    for event in [Event::Leave, Event::Finish] {
        e.raise_external(event, "", "");
        e.step();
    }

    assert_eq!(
        e.terminal_state(),
        Some(State::Done),
        "`finish` must carry the run to `done`"
    );
    assert_eq!(
        e.policy().spurious(),
        Some(0),
        "leaving `p` cancelled its child, and a `cancel.invoke` event reached the invoking session; \
         W3C SCXML 6.4 defines no such event"
    );
}

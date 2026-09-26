// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.3 / Appendix D enterStates, late binding: a state's <data> is
// bound on that state's FIRST entry and never again — Rust AOT path.
//
// `s` is entered, its `v` changed to 5, `s` left and entered again; its
// <onentry> records the `v` it sees each time. Measured 2026-09-26, this
// channel bound late data on every entry, so the second entry saw 1 again.
//
// Fixture: integration_resources/late_data_binds_on_first_entry/late_data_binds_on_first_entry.scxml
// (canonical, shared with the C++ / C11 / Go / Kotlin / Python channels).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_late_data_binds_on_first_entry.sh

use sce_rust_tests::integration::late_data_binds_on_first_entry::{
    LateDataBindsOnFirstEntryEvent as Event, LateDataBindsOnFirstEntryPolicy as Policy,
    LateDataBindsOnFirstEntryState as State,
};

#[test]
fn a_state_binds_its_data_only_on_its_first_entry() {
    // The handlers record with `<assign>`, so the policy takes an engine.
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let mut e = sce_rust_runtime::Engine::new(Policy::new(script_engine));
    e.initialize();
    assert!(
        e.get_active_states().contains(&State::Idle),
        "the run has to start in `idle`"
    );

    // Enter `s`, change its `v`, leave it, enter it again — one step each.
    for event in [Event::Go, Event::Bump, Event::Back, Event::Go] {
        e.raise_external(event, "", "");
        e.step();
    }

    let p = e.policy();
    let seen = format!(
        "entries={:?} seen={:?} contentSeen={:?} (wanted Some(2) / Some(15) / Some(7))",
        p.entries(),
        p.seen(),
        p.content_seen()
    );
    assert!(
        e.get_active_states().contains(&State::S),
        "the second `go` has to leave the machine in `s`. {seen}"
    );
    assert_eq!(
        p.entries(),
        Some(2),
        "both entries of `s` must have run. {seen}"
    );
    assert_eq!(
        p.seen(),
        Some(15),
        "`s` saw v=1 on its first entry and must see the 5 it was changed to on its second: 11 is a \
         processor that binds late data on every entry, and no value at all one that never binds it. {seen}"
    );
    assert_eq!(
        p.content_seen(),
        Some(7),
        "`c` is bound from inline content, not an expr, and must be bound too. {seen}"
    );
}

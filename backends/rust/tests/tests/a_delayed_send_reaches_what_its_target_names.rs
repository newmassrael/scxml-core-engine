// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.2 + 6.4 + C.1: a delay postpones a <send>, it does not change
// where the send goes — Rust AOT path.
//
// Fixture: integration_resources/a_delayed_send_reaches_what_its_target_names/a_delayed_send_reaches_what_its_target_names.scxml
// (canonical, shared with the C++ / C11 / Go / Kotlin / Python channels).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_a_delayed_send_reaches_what_its_target_names.sh

use std::time::Duration;

use sce_rust_tests::integration::a_delayed_send_reaches_what_its_target_names::{
    ADelayedSendReachesWhatItsTargetNamesPolicy as Policy,
    ADelayedSendReachesWhatItsTargetNamesState as State,
};

#[test]
fn a_delayed_send_reaches_what_its_target_names() {
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let mut e = sce_rust_runtime::Engine::new(Policy::new(script_engine));
    e.initialize();
    let completed = e.run_until_completion(Duration::from_secs(3), Duration::from_millis(5));

    assert!(
        completed,
        "the machine never completed (parked in {:?})",
        e.get_current_state()
    );
    assert_the_routes(&e);
}

/// A host that ticks late — every 150ms of its own time, past the 100ms the
/// delayed `lost` waits — gets the same answers. That tick brings `lost` due
/// before it ticks the children, so `gone` must already have taken the `stop`
/// sent to it at once (W3C SCXML 6.4), and `lost` then finds `gone` ended and
/// is reported (C.1). It also holds the children to their parent's host-owned
/// time: before they followed it, the child's delayed `late` never came due.
#[test]
fn a_delayed_send_reaches_what_its_target_names_on_a_late_host() {
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let mut e = sce_rust_runtime::Engine::new(Policy::new(script_engine));
    e.set_clock(sce_rust_runtime::SceClock::Manual(0));
    e.initialize();
    for _ in 0..20 {
        if e.is_in_final_state() {
            break;
        }
        e.advance_time_ms(150);
    }

    assert!(
        e.is_in_final_state(),
        "the machine never completed (parked in {:?})",
        e.get_current_state()
    );
    assert_the_routes(&e);
}

fn assert_the_routes(e: &sce_rust_runtime::Engine<Policy>) {
    assert_eq!(
        e.terminal_state(),
        Some(State::Done),
        "the run must end in `done`"
    );
    let p = e.policy();
    let observed = [
        ("order", p.order(), 31),
        ("innerInternal", p.inner_internal(), 1),
        ("lateOk", p.late_ok(), 1),
        ("lateCount", p.late_count(), 1),
        ("pongOk", p.pong_ok(), 1),
        ("commErrors", p.comm_errors(), 2),
        ("lostArrived", p.lost_arrived(), 0),
        ("afterStranger", p.after_stranger(), 0),
    ];
    let wrong: Vec<_> = observed
        .iter()
        .filter(|(_, got, want)| *got != Some(*want))
        .collect();
    assert!(wrong.is_empty(), "observed (name, got, want): {wrong:?}");
}

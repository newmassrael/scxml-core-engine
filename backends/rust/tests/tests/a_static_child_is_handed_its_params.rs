// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Accepted Subset §2.15, "Child sessions": an `<invoke type="scxml">` hands
// its child the values its `<param>`s and `namelist` name (W3C SCXML 6.4.1),
// each to the child's variable of the same name, of that variable's own type.
//
// `static_invoke_params.scxml` invokes `worker`, which ends the moment it holds
// `start = 7` (from a `<param>` reading `base`) and `enabled = true` (from the
// `namelist`); handed less, it would wait and the parent would stay in
// `working`. `base` is 4 when `working` is entered and the entry action adds 3,
// so 7 arrives only if the value is read when the invoke executes, at the end of
// the macrostep. `control`, the same child handed nothing, keeps its declared
// defaults and never ends.
//
// Kotlin's half is `AStaticChildIsHandedItsParamsTest.kt`.

use sce_rust_runtime::saved_state::SavedState;
use sce_rust_runtime::{Engine, SceClock};
use sce_rust_tests::integration::static_datamodel::static_invoke_params_sm::{
    StaticInvokeParamsPersist, StaticInvokeParamsPolicy, StaticInvokeParamsState,
};

/// The wall-clock moment the machine is saved at.
const SAVED_AT_MS: u64 = 1_700_000_000_000;

fn started() -> Engine<StaticInvokeParamsPolicy> {
    let mut engine = Engine::new(StaticInvokeParamsPolicy::new());
    engine.set_clock(SceClock::Manual(0));
    engine.initialize();
    engine
}

/// Let the child run and report to its parent.
fn settle(engine: &mut Engine<StaticInvokeParamsPolicy>) {
    for _ in 0..5 {
        engine.tick();
    }
}

fn in_state(engine: &Engine<StaticInvokeParamsPolicy>, state: StaticInvokeParamsState) -> bool {
    engine.get_active_states().contains(&state)
}

#[test]
fn a_child_is_handed_the_values_its_invoke_names() {
    let mut engine = started();
    settle(&mut engine);
    // `worker` ended, so it held both values: the parent left `working` and
    // counted it.
    assert!(
        in_state(&engine, StaticInvokeParamsState::Plain),
        "the child was handed `start` and `enabled`, so it ended"
    );
    assert_eq!(engine.policy().completed(), 1);
}

#[test]
fn a_restored_child_is_handed_the_values_the_restored_machine_holds() {
    let mut engine = started();
    settle(&mut engine);
    // `bump` makes `base` 8 while `watcher` runs with the 7 it was handed once,
    // when it started, so it does not end.
    engine.raise_external_by_name("bump", "");
    settle(&mut engine);
    assert_eq!(engine.policy().completed(), 1);

    // A restored child is started again from its beginning, and a start
    // evaluates its arguments: the new `watcher` is handed 8, and ends.
    let saved = engine.save_at(SAVED_AT_MS).expect("saves");
    let mut restored = Engine::<StaticInvokeParamsPolicy>::restore_with(
        StaticInvokeParamsPolicy::new(),
        &SavedState::from_json(&saved.to_json()).expect("reads"),
        SceClock::Manual(0),
        SAVED_AT_MS,
    )
    .expect("restores");
    settle(&mut restored);
    assert_eq!(restored.policy().completed(), 11);
}

#[test]
fn a_child_handed_nothing_keeps_the_values_its_data_gave_it() {
    let mut engine = started();
    settle(&mut engine);
    settle(&mut engine);
    // `control` is the same child handed nothing: it still waits for 7 and
    // true, so `done.invoke.control` never counted.
    assert!(in_state(&engine, StaticInvokeParamsState::Plain));
    assert_eq!(engine.policy().completed(), 1);
}

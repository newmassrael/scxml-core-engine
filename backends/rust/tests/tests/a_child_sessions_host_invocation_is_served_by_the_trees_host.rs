// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Accepted Subset §2.15, "Running host invocations" (docs/adr/0045, section
// 3): the invokers a host registers on the machine it built serve every session
// of that machine's tree. Each request names the session that made it, and the
// host ends an invocation of any session through the machine it holds, by that
// session's id and the request's token.
//
// Fixture: sce-build/tests/fixtures/static_datamodel/static_invoke_tree.scxml,
// whose in-place child `worker` asks its host for the time
// (`sce-host-clock-now`, id `clock`) as it enters `second`, tells its parent
// `stamped` when the host answers, and keeps the id its grandchild is started
// under in `helper_id` (`idlocation`).

use std::sync::{Arc, Mutex};

use sce_rust_runtime::json::Value;
use sce_rust_runtime::saved_state::{SavedInvokes, SavedState};
use sce_rust_runtime::{Engine, HostInvokeEvent, HostInvokeRequest, SceClock};
use sce_rust_tests::integration::static_datamodel::static_invoke_tree_sm::{
    StaticInvokeTreePersist, StaticInvokeTreePolicy,
};

/// The session id the shared format-2 fixtures give their root.
const ROOT: &str = "fixture-root";

/// The session `worker` is: the root's id and the id its first start is
/// started under (`<state>.<start>.<invoke>`).
const WORKER: &str = "fixture-root.working.1.worker";

/// The standard host type the child asks.
const CLOCK: &str = "sce-host-clock-now";

/// A completion of the clock the child's result record reads.
const NOW: &str = r#"{"error":"","utc_ms":1700000000000,"monotonic_ms":5}"#;

/// The wall-clock moment the tree is saved at.
const SAVED_AT_MS: u64 = 1_700_000_000_000;

/// Every start the host was handed, in order.
type Starts = Arc<Mutex<Vec<HostInvokeRequest>>>;

/// Register the clock on `engine`, the root, recording each start.
fn serve(engine: &mut Engine<StaticInvokeTreePolicy>) -> Starts {
    let starts = Starts::default();
    let seen = Arc::clone(&starts);
    engine.register_invoker(CLOCK, move |event| {
        if let HostInvokeEvent::Start(request) = event {
            seen.lock().unwrap().push(request);
        }
        None
    });
    starts
}

/// A root built under `ROOT`, served, and initialized.
fn started() -> (Engine<StaticInvokeTreePolicy>, Starts) {
    let mut engine =
        Engine::new(StaticInvokeTreePolicy::new_with_session_id(ROOT).expect("an Nmtoken"));
    engine.set_clock(SceClock::Manual(0));
    let starts = serve(&mut engine);
    engine.initialize();
    (engine, starts)
}

/// `go`, which the root forwards to the child in one tick and the child takes,
/// entering `second` and starting the clock, in the next.
fn into_second(engine: &mut Engine<StaticInvokeTreePolicy>) {
    engine.raise_external_by_name("go", "");
    engine.tick();
    engine.tick();
}

fn only_start(starts: &Starts) -> HostInvokeRequest {
    let starts = starts.lock().unwrap();
    assert_eq!(starts.len(), 1, "one start: {starts:?}");
    starts[0].clone()
}

#[test]
fn a_childs_invocation_reaches_the_roots_invoker_and_is_ended_through_the_root() {
    let (mut engine, starts) = started();
    into_second(&mut engine);
    let request = only_start(&starts);
    assert_eq!(request.processor_type, CLOCK);
    assert_eq!(request.invoke_id, "clock");
    assert_eq!(
        request.session_id, WORKER,
        "the request names the child session that made it"
    );
    assert!(!request.restarted);

    engine
        .complete_host_invoke_in(WORKER, CLOCK, "clock", request.token, NOW)
        .unwrap_or_else(|refusal| panic!("the child is running that start: {refusal}"));
    engine.tick();
    assert_eq!(
        engine.policy().stamps(),
        1,
        "the child took done.invoke.clock and told its parent"
    );
}

#[test]
fn an_ending_for_a_session_the_tree_does_not_hold_or_a_start_it_does_not_run_is_refused() {
    let (mut engine, starts) = started();
    into_second(&mut engine);
    let request = only_start(&starts);
    for nowhere in [
        "fixture-root.nowhere",
        "fixture-root.working.1.worker.nowhere",
        "elsewhere",
    ] {
        let refusal = engine
            .complete_host_invoke_in(nowhere, CLOCK, "clock", request.token, NOW)
            .expect_err("no such session");
        assert!(
            refusal.reason().contains("no session"),
            "{nowhere}: {refusal}"
        );
    }
    // The root runs no clock of its own.
    let refusal = engine
        .complete_host_invoke_in(ROOT, CLOCK, "clock", request.token, NOW)
        .expect_err("the root runs no such start");
    assert!(refusal.reason().contains("is not running"), "{refusal}");
    // A start ended once is ended: the second ending is stale.
    engine
        .complete_host_invoke_in(WORKER, CLOCK, "clock", request.token, NOW)
        .expect("the first ending");
    let refusal = engine
        .fail_host_invoke_in(WORKER, CLOCK, "clock", request.token, "late")
        .expect_err("the start has ended");
    assert!(refusal.reason().contains("is not running"), "{refusal}");
    engine.tick();
    assert_eq!(
        engine.policy().stamps(),
        1,
        "the refused endings raised nothing"
    );
}

#[test]
fn a_resumed_childs_invocation_starts_again_at_its_first_drive() {
    let (mut engine, starts) = started();
    into_second(&mut engine);
    let first = only_start(&starts);
    let text = engine
        .save_at(SAVED_AT_MS)
        .unwrap_or_else(|refusal| panic!("saves with the clock running: {refusal}"))
        .to_json();

    let mut restored = Engine::<StaticInvokeTreePolicy>::restore_with(
        StaticInvokeTreePolicy::new(),
        &SavedState::from_json(&text).expect("reads"),
        SceClock::Manual(0),
        SAVED_AT_MS,
    )
    .unwrap_or_else(|refusal| panic!("restores: {refusal}"));
    // The host registers its invokers on the machine the restore handed it,
    // after the restore built the child: nothing has started yet.
    let again = serve(&mut restored);
    assert!(again.lock().unwrap().is_empty(), "a restore calls no host");

    // The parent's first drive of the child is the child's first macrostep.
    restored.tick();
    let request = only_start(&again);
    assert!(request.restarted, "started again from the saved request");
    assert_eq!(request.session_id, WORKER);
    assert_eq!(request.invoke_id, "clock");
    assert_ne!(request.token, first.token, "a new token");

    restored
        .complete_host_invoke_in(WORKER, CLOCK, "clock", request.token, NOW)
        .unwrap_or_else(|refusal| panic!("the restarted start: {refusal}"));
    restored.tick();
    assert_eq!(restored.policy().stamps(), 1);
}

#[test]
fn an_invoke_idlocation_keeps_the_id_the_grandchild_is_started_under() {
    // docs/adr/0045: the grandchild is started under `second.1.<its invoke id>`,
    // the `invokeid` a saved tree holds for it, and `helper_id` holds the same.
    let (mut engine, _starts) = started();
    into_second(&mut engine);
    let saved = engine.save_at(SAVED_AT_MS).expect("saves");
    let SavedInvokes::Resumed(children) = &saved.invokes else {
        panic!("a format-2 tree");
    };
    let worker = &children[0].session;
    let SavedInvokes::Resumed(grandchildren) = &worker.invokes else {
        panic!("a format-2 child");
    };
    assert_eq!(grandchildren.len(), 1, "the hybrid candidate runs");
    let started_under = &grandchildren[0].invoke_id;
    assert!(
        started_under.starts_with("second.1."),
        "the first start of the invoke of `second`: {started_under}"
    );
    let held = worker
        .variables
        .iter()
        .find(|(id, _)| id == "helper_id")
        .map(|(_, value)| value.clone());
    assert_eq!(held, Some(Value::Text(started_under.clone())));
}

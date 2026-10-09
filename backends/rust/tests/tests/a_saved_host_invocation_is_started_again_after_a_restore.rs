// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Accepted Subset §2.15, "Saving and restoring": an `<invoke>` the HOST runs
// is part of what a machine is doing (§scxml-6.4.1), and the process that ran it
// is gone with the process that saved it. A saved state holds each running
// host-run invocation with the request it was started with and the deadline it
// had left, and a restore starts it again from that request — never evaluating
// the element a second time — with `restarted` set, a new token and what was
// left of the deadline.
//
// `statechart_static_host_invoke.scxml` hands a job to the host from `working`.
// `done.invoke.h` and `error.invoke.h` each leave it and record a digit in
// `seen`, so the number says which arrived.
//
// Driven entirely on `SceClock::Manual`: no case sleeps, and none can be decided
// by how loaded the build machine is. The shared instance
// `saved/statechart_static_host_invoke_running.json` is the text the Kotlin suite
// writes and restores too.

use std::sync::{Arc, Mutex};

use sce_rust_runtime::saved_state::{SavedState, StateRefusal};
use sce_rust_runtime::{
    Engine, HostInvokeCancel, HostInvokeEvent, HostInvokeRequest, HostInvokeResponse, SceClock,
};
use sce_rust_tests::integration::host_processor::{
    StatechartStaticHostInvokePersist as Persist, StatechartStaticHostInvokePolicy as Policy,
    StatechartStaticHostInvokeState as State,
};

const SHARED_RUNNING: &str = include_str!(
    "../../../../sce-build/tests/fixtures/host_processor/saved/statechart_static_host_invoke_running.json"
);

/// The wall-clock moment the shared instance was saved at.
const SAVED_AT_MS: u64 = 1_700_000_000_000;

/// What the host was told, in order.
#[derive(Debug, Clone)]
enum Told {
    Start(HostInvokeRequest),
    Cancel(HostInvokeCancel),
}

/// Whether two requests say the same thing: everything but the token the engine
/// assigns and the mark a restart carries.
fn same_request(a: &HostInvokeRequest, b: &HostInvokeRequest) -> bool {
    a.processor_type == b.processor_type
        && a.invoke_id == b.invoke_id
        && a.src == b.src
        && a.params == b.params
        && a.event_data == b.event_data
        && a.content == b.content
}

type Log = Arc<Mutex<Vec<Told>>>;

fn register(engine: &mut Engine<Policy>, log: &Log) {
    let log = Arc::clone(log);
    engine.register_invoker("x-sce-host", move |event| -> Option<HostInvokeResponse> {
        log.lock().unwrap().push(match event {
            HostInvokeEvent::Start(request) => Told::Start(request),
            HostInvokeEvent::Cancel(cancel) => Told::Cancel(cancel),
        });
        None
    });
}

fn told(log: &Log) -> Vec<Told> {
    log.lock().unwrap().clone()
}

fn starts(log: &Log) -> Vec<HostInvokeRequest> {
    told(log)
        .into_iter()
        .filter_map(|t| match t {
            Told::Start(request) => Some(request),
            Told::Cancel(_) => None,
        })
        .collect()
}

/// A machine standing in `working`, its invocation started with token 0.
fn working() -> (Engine<Policy>, Log) {
    let log = Log::default();
    let mut engine = Engine::new(Policy::new());
    engine.set_clock(SceClock::Manual(0));
    register(&mut engine, &log);
    engine.initialize();
    engine.raise_external_by_name("start", "");
    engine.tick();
    (engine, log)
}

fn try_restore(text: &str, elapsed_ms: u64) -> Result<(Engine<Policy>, Log), StateRefusal> {
    let engine = Engine::<Policy>::restore_with(
        Policy::new(),
        &SavedState::from_json(text)?,
        SceClock::Manual(0),
        SAVED_AT_MS + elapsed_ms,
    )?;
    Ok((engine, Log::default()))
}

/// `text` restored `elapsed_ms` of wall-clock time after it was saved, its host
/// invoker registered the way a Rust host does it: on the engine the restore
/// returned.
fn restored(text: &str, elapsed_ms: u64) -> (Engine<Policy>, Log) {
    let (mut engine, log) = try_restore(text, elapsed_ms).expect("restores");
    register(&mut engine, &log);
    (engine, log)
}

fn standing_in(engine: &Engine<Policy>) -> Vec<State> {
    engine.get_active_states().into_iter().collect()
}

#[test]
fn a_machine_is_saved_with_the_request_its_invocation_was_started_with() {
    let (engine, log) = working();
    let first = &starts(&log)[0];
    assert!(
        !first.restarted,
        "the document's own start is not a restart"
    );
    assert_eq!(first.token, 0);

    let saved = engine.save_at(SAVED_AT_MS).expect("saves");
    let [entry] = saved.host_invokes.as_slice() else {
        panic!("one running invocation: {:?}", saved.host_invokes)
    };
    assert_eq!(entry.processor_type, "x-sce-host");
    assert_eq!(entry.invoke_id, "h");
    assert_eq!(entry.src, first.src);
    assert_eq!(entry.content, first.content);
    assert_eq!(entry.data, first.event_data);
    // The deadline is the invocation's own, not a send waiting to be delivered.
    assert_eq!(entry.due, Some(SAVED_AT_MS + 5000));
    assert!(saved.pending.is_empty(), "{:?}", saved.pending);
    // One start has been made, so the next token is 1.
    assert_eq!(saved.host_invoke_token, 1);

    // The text the Kotlin suite writes for the same machine, byte for byte.
    assert_eq!(saved.to_json(), SHARED_RUNNING.trim());
}

#[test]
fn a_restore_asks_nothing_of_the_host_until_the_machine_is_driven() {
    let (mut engine, log) = restored(SHARED_RUNNING, 1000);
    assert!(told(&log).is_empty(), "the restore itself calls nobody");
    assert_eq!(standing_in(&engine), [State::Working]);

    // The first macrostep starts it, where an invocation the document entered
    // would have started.
    engine.tick();
    assert_eq!(starts(&log).len(), 1);
}

#[test]
fn a_restored_invocation_is_started_again_from_the_request_it_was_saved_with() {
    let (original, original_log) = working();
    let first = starts(&original_log)[0].clone();
    drop(original);

    let (mut engine, log) = restored(SHARED_RUNNING, 1000);
    engine.tick();
    let all = starts(&log);
    assert_eq!(all.len(), 1, "one start: {all:?}");
    let again = &all[0];

    assert!(again.restarted, "the host is told this is a resumption");
    assert_eq!(again.token, 1, "the next token the saved state carried on");
    assert!(
        same_request(again, &first),
        "the same request, not one evaluated again:\n{again:?}\n{first:?}"
    );
}

#[test]
fn a_restored_invocation_keeps_the_deadline_it_had_left() {
    // Saved with 5000 ms to run, restored 1000 ms later: 4000 ms are left.
    let (mut engine, log) = restored(SHARED_RUNNING, 1000);
    engine.tick();
    engine.advance_time_ms(3999);
    assert_eq!(engine.policy().seen(), 0, "not due yet");
    assert_eq!(standing_in(&engine), [State::Working]);

    engine.advance_time_ms(1);
    assert_eq!(engine.policy().seen(), 2, "error.invoke.h, at 4000 ms");
    assert_eq!(standing_in(&engine), [State::Failed]);
    // The host is told to stop what it was given, with the token it was given.
    assert!(
        matches!(told(&log).last(), Some(Told::Cancel(c)) if c.token == 1 && c.invoke_id == "h"),
        "{:?}",
        told(&log)
    );
}

#[test]
fn a_deadline_that_passed_while_the_machine_was_away_ends_the_invocation_unstarted() {
    // 6 s after the save, and the deadline was at 5 s.
    let (mut engine, log) = restored(SHARED_RUNNING, 6000);
    engine.tick();
    assert!(
        told(&log).is_empty(),
        "the host is not asked to begin what it would be told to stop: {:?}",
        told(&log)
    );
    assert_eq!(
        engine.policy().seen(),
        2,
        "the document hears error.invoke.h"
    );
    assert_eq!(standing_in(&engine), [State::Failed]);
}

/// A host drives a machine by asking when it next needs a tick and sleeping that long
/// (`Engine::time_until_next_scheduled_ms`). A restored invocation owes the machine a tick
/// before anything else: it is started in the first macrostep, and the deadline it keeps
/// is armed there. A machine that answered "nothing" would be left asleep for good, holding
/// an invocation no one had started.
#[test]
fn a_restored_machine_owing_a_start_says_it_needs_a_tick_now() {
    for elapsed_ms in [1000, 6000] {
        let (mut engine, _log) = restored(SHARED_RUNNING, elapsed_ms);
        assert_eq!(
            engine.time_until_next_scheduled_ms(),
            Some(0),
            "{elapsed_ms} ms after the save, before any tick"
        );
        engine.tick();
    }
}

#[test]
fn what_the_first_run_handed_a_host_cannot_answer_for_the_second() {
    // Token 0 belonged to the process that saved the machine. A late reply
    // carrying it is a different process's, and is ignored (§scxml-6.4).
    let (mut engine, log) = restored(SHARED_RUNNING, 1000);
    engine.tick();
    let token = starts(&log)[0].token;
    assert!(!engine.complete_host_invoke("x-sce-host", "h", 0, "late"));
    engine.tick();
    assert_eq!(engine.policy().seen(), 0);
    assert_eq!(standing_in(&engine), [State::Working]);

    assert!(engine.complete_host_invoke("x-sce-host", "h", token, "done"));
    engine.tick();
    assert_eq!(engine.policy().seen(), 1);
    assert_eq!(standing_in(&engine), [State::After]);
}

#[test]
fn a_completion_still_queued_when_the_machine_was_saved_is_accepted_after_the_restore() {
    let (mut engine, log) = working();
    // The host answers; the completion is on the external queue, stamped with the
    // start's token, and the machine has not been driven through it.
    assert!(engine.complete_host_invoke("x-sce-host", "h", 0, "done"));
    let saved = engine.save_at(SAVED_AT_MS).expect("saves");
    assert!(
        saved.host_invokes.is_empty(),
        "the invocation ended: {:?}",
        saved.host_invokes
    );
    let [queued] = saved.external.as_slice() else {
        panic!("one queued event: {:?}", saved.external)
    };
    assert_eq!(queued.name, "done.invoke.h");
    assert_eq!(queued.host_invoke_token, Some(0));
    assert_eq!(told(&log).len(), 1, "started once, never cancelled");

    let (mut again, log) = restored(&saved.to_json(), 0);
    again.tick();
    assert_eq!(again.policy().seen(), 1, "taken once, not refused");
    assert_eq!(standing_in(&again), [State::After]);
    assert!(
        told(&log).is_empty(),
        "an ended invocation is not started again"
    );
    assert_eq!(again.refused_host_invoke_completions(), 0);
}

#[test]
fn a_completion_the_engine_was_about_to_refuse_is_still_refused_after_the_restore() {
    let (mut engine, _) = working();
    // Raised through the ordinary door, so it carries no token: the engine
    // refuses it when it is dequeued, because it may be a cancelled run's reply.
    engine.raise_external_by_name("done.invoke.h", "");
    let saved = engine.save_at(SAVED_AT_MS).expect("saves");
    assert_eq!(saved.external[0].host_invoke_token, None);

    let (mut again, _) = restored(&saved.to_json(), 0);
    again.tick();
    assert_eq!(again.refused_host_invoke_completions(), 1);
    assert_eq!(again.policy().seen(), 0);
    assert_eq!(standing_in(&again), [State::Working]);
}

#[test]
fn an_invocation_cancelled_before_it_was_started_again_is_never_started() {
    let (mut engine, log) = restored(SHARED_RUNNING, 1000);
    // Nothing was started, so there is nothing to tell the host to stop.
    assert!(!engine.cancel_host_invoke("x-sce-host", "h"));
    engine.tick();
    assert!(told(&log).is_empty(), "{:?}", told(&log));
}

#[test]
fn a_machine_restored_and_saved_again_before_it_was_driven_keeps_its_invocation() {
    let (engine, _) = restored(SHARED_RUNNING, 1000);
    let again = engine.save_at(SAVED_AT_MS + 1000).expect("saves");
    // The same request, due at the same wall-clock moment, and the same counter.
    assert_eq!(again.to_json(), SHARED_RUNNING.trim());
}

#[test]
fn a_restart_nobody_can_run_is_the_error_a_start_nobody_can_run_is() {
    let (mut engine, _) = try_restore(SHARED_RUNNING, 1000).expect("restores");
    // No invoker registered by the first macrostep.
    engine.tick();
    assert_eq!(engine.unhandled_error_events(), 1, "error.execution");
}

#[test]
fn a_saved_state_naming_a_host_invocation_the_document_cannot_have_run_is_refused() {
    let swap = |from: &str, to: &str| SHARED_RUNNING.replace(from, to);
    for (what, text, expected) in [
        (
            "an id the document does not invoke",
            swap(r#""id":"h""#, r#""id":"stranger""#),
            "which the document does not have a host invoker run",
        ),
        (
            "a type the document does not hand that id to",
            swap(r#""type":"x-sce-host""#, r#""type":"x-other""#),
            "which the document does not have a host invoker run",
        ),
        (
            "an invocation whose state the configuration is not in",
            swap(
                r#""configuration":["working"]"#,
                r#""configuration":["idle"]"#,
            )
            .replace(r#""current":"working""#, r#""current":"idle""#),
            "whose state the saved configuration does not stand in",
        ),
        (
            "an invocation named twice",
            {
                let start =
                    SHARED_RUNNING.find(r#""hostinvokes":["#).unwrap() + r#""hostinvokes":["#.len();
                let end = SHARED_RUNNING[start..]
                    .find("],\"hostinvoketoken\"")
                    .unwrap()
                    + start;
                let one = &SHARED_RUNNING[start..end];
                SHARED_RUNNING.replace(one, &format!("{one},{one}"))
            },
            "which an earlier entry already names",
        ),
    ] {
        let refusal = try_restore(&text, 0)
            .err()
            .unwrap_or_else(|| panic!("{what} restored"));
        assert!(refusal.reason().contains(expected), "{what}: {refusal}");
    }
}

#[test]
fn the_host_invoke_fields_are_required_and_must_be_what_they_say() {
    let swap = |from: &str, to: &str| SHARED_RUNNING.replace(from, to);
    for (what, text, expected) in [
        (
            "no hostinvokes",
            swap(r#""hostinvokes":["#, r#""hostinvokes_":["#),
            "has no 'hostinvokes'",
        ),
        (
            "hostinvokes that is not an array",
            {
                let start = SHARED_RUNNING.find(r#""hostinvokes":["#).unwrap();
                let end = SHARED_RUNNING.find(r#","hostinvoketoken""#).unwrap();
                format!(
                    "{}\"hostinvokes\":\"h\"{}",
                    &SHARED_RUNNING[..start],
                    &SHARED_RUNNING[end..]
                )
            },
            "'hostinvokes' is not an array",
        ),
        (
            "no hostinvoketoken",
            swap(r#""hostinvoketoken":"1","#, ""),
            "has no 'hostinvoketoken'",
        ),
        (
            "a token that is a number",
            swap(r#""hostinvoketoken":"1""#, r#""hostinvoketoken":1"#),
            "'hostinvoketoken' is not a text",
        ),
        (
            "a token past a signed 64-bit count",
            swap(
                r#""hostinvoketoken":"1""#,
                r#""hostinvoketoken":"9223372036854775808""#,
            ),
            "is not a whole number a token can be",
        ),
        (
            "a deadline that is not a moment",
            swap(r#""due":"1700000005000""#, r#""due":"soon""#),
            "is not a whole number of milliseconds",
        ),
        (
            "a deadline that is a number",
            swap(r#""due":"1700000005000""#, r#""due":1700000005000"#),
            "'hostinvokes[0].due' is neither a text nor null",
        ),
    ] {
        let refusal = SavedState::from_json(&text).expect_err(what);
        assert!(refusal.reason().contains(expected), "{what}: {refusal}");
    }
}

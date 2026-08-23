// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.2.4 + 6.3 — a `<send delay>` addressed to a HOST-served Event
// I/O Processor waits, and can be cancelled while it waits. Rust AOT path.
//
// §scxml-6.2.4 puts the wait before the dispatch and says nothing about which
// processor the send named; §scxml-6.2.5 makes that set open. Put together, a
// host-served send with a delay is an ordinary delayed send that happens to be
// delivered by the host. It was not: every backend chose the host branch ahead
// of the delay branch in one `elif` chain per language, so the act was
// performed at the instant the block ran and `delay` was discarded — while the
// manifest went on answering `needs_event_scheduler: true`, telling the host to
// drive with `tick` for a wait the engine had already thrown away.
//
// Driven entirely on `SceClock::Manual`. No case here sleeps, and none can be
// decided by how loaded the build machine is: the host sets what time it is and
// the engine answers with the configuration that time implies. That matters
// more than usual on this axis, because a wall-clock version of the first case
// would pass on a slow machine for the wrong reason — the handler running
// "early" is only observable against a clock the test controls.
//
// Fixture: sce-build/tests/fixtures/host_processor/statechart_delayed_host_send.scxml
// (canonical, shared with the C++ / C11 / Go / Kotlin / Python channels).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_host_processor.sh

use std::sync::{Arc, Mutex};

use sce_rust_runtime::{Engine, HostSendRequest, HostSendResponse, SceClock, StatePolicy};
use sce_rust_tests::integration::host_processor::{
    StatechartDelayedHostSendPolicy as Policy, StatechartDelayedHostSendState as State,
};

/// The type the fixture was compiled for; `scripts/regen_host_processor.sh`
/// passes this same string to `--host-processor`.
const DECLARED_TYPE: &str = "x-sce-host";

/// What the handler saw, in call order: the engine's own reading of "now" at
/// the moment it was asked to perform the act.
///
/// The engine's clock rather than the test's bookkeeping, because that is the
/// number the contract is about — a handler called at 0 ms for a `delay="200ms"`
/// send is the defect, and any other witness (a counter, a wall-clock stamp)
/// only says it happened, not when the engine thought it was.
type CallLog = Arc<Mutex<Vec<u64>>>;

/// The fixture is only meaningful on a scheduler-driven machine. A `const`
/// block rather than a `#[test]`, matching `late_tick_honours_cancel.rs`: a
/// regression here should fail the build rather than wait for a run.
///
/// This is also the assertion that the manifest's `needs_event_scheduler` is
/// not the whole promise — it was already `true` while every delay in this
/// document was being discarded, which is why the cases below measure the
/// dispatch instant instead of trusting the flag.
const _: () = {
    assert!(<Policy as StatePolicy>::NEEDS_EVENT_SCHEDULER);
};

/// A machine on host-owned time, with a handler that answers `turn.done` and
/// records when it was asked.
///
/// The handler needs the engine's reading of "now" and cannot borrow the engine
/// to ask for it, so the clock is mirrored into the closure through the same
/// `Arc<Mutex<..>>` the log uses. The mirror is written only by the test, right
/// before each `advance_time_ms`, so it holds exactly the value the engine is
/// about to judge deadlines against.
fn armed() -> (Engine<Policy>, CallLog, Arc<Mutex<u64>>) {
    let log: CallLog = Arc::new(Mutex::new(Vec::new()));
    let now = Arc::new(Mutex::new(0_u64));

    let recorder = Arc::clone(&log);
    let clock_mirror = Arc::clone(&now);

    let mut engine = Engine::new(Policy::new());
    engine.set_clock(SceClock::Manual(0));
    engine.register_event_processor(DECLARED_TYPE, move |_req: HostSendRequest| {
        recorder
            .lock()
            .expect("handler log")
            .push(*clock_mirror.lock().expect("clock mirror"));
        vec![HostSendResponse {
            event_name: "turn.done".to_string(),
            event_data: String::new(),
        }]
    });
    engine.initialize();
    (engine, log, now)
}

/// Move host-owned time to `to_ms` and let the engine run what that made due.
fn advance_to(engine: &mut Engine<Policy>, now: &Arc<Mutex<u64>>, to_ms: u64) {
    let from = *now.lock().expect("clock mirror");
    assert!(to_ms >= from, "time does not run backwards in these cases");
    *now.lock().expect("clock mirror") = to_ms;
    engine.advance_time_ms(to_ms - from);
}

fn calls(log: &CallLog) -> Vec<u64> {
    log.lock().expect("handler log").clone()
}

/// The axis. `waiting` arms a host-served send for 200 ms and an ordinary one
/// for 100 ms; the ordinary one must arrive first, which is only true if the
/// host-served one waited.
///
/// The `tooEarly` final state is what the document reaches when it did not: the
/// handler's reply is on the queue before the machine has been anywhere, so
/// `turn.done` wins the race its own `delay` was supposed to lose.
#[test]
fn a_host_served_send_waits_for_its_delay() {
    let (mut engine, log, now) = armed();

    // Nothing is due at 0 ms. This is the whole defect in one assertion: with
    // the host branch chosen ahead of the delay branch, `initialize()` has
    // already performed the act by the time this line runs.
    assert_eq!(
        calls(&log),
        Vec::<u64>::new(),
        "the handler was asked to perform a `delay=\"200ms\"` send at {} ms. \
         §scxml-6.2.4 makes the delay the wait the document asked for, and \
         §scxml-6.2.5 does not exempt a host-served processor from it",
        engine.now_ms(),
    );
    assert_eq!(engine.get_current_state(), State::Waiting);

    // 100 ms: the ordinary `probe` is due, the host-served send is not.
    advance_to(&mut engine, &now, 100);
    assert_eq!(
        engine.get_current_state(),
        State::Armed,
        "the 100 ms `probe` did not arrive first; the machine is in {:?}",
        engine.get_current_state()
    );
    assert_eq!(
        calls(&log),
        Vec::<u64>::new(),
        "the host-served send was dispatched before its 200 ms deadline",
    );

    // 200 ms: now it is due, and the handler's reply moves the machine on.
    advance_to(&mut engine, &now, 200);
    assert_eq!(
        calls(&log),
        vec![200],
        "the host-served send did not fire at its 200 ms deadline",
    );
    assert_eq!(
        engine.get_current_state(),
        State::Cancelling,
        "the handler's `turn.done` did not reach the document",
    );
}

/// §scxml-6.3: a `<cancel>` drops a delayed send that has not been dispatched.
/// A host-served one is not exempt, and the witness is host-side: the handler
/// must never be asked to perform the cancelled act at all.
///
/// This is the half that says which queue the deferred send is in. An engine
/// that honoured the delay by any private means — a side list, a timer thread —
/// would pass the case above and fail here, because `<cancel sendid>` reaches
/// the scheduler and nothing else.
#[test]
fn a_cancel_drops_a_pending_host_served_send() {
    let (mut engine, log, now) = armed();

    advance_to(&mut engine, &now, 100); // probe      -> armed
    advance_to(&mut engine, &now, 200); // turn.done  -> cancelling (arms h2 for 400)
    advance_to(&mut engine, &now, 300); // settle     -> cancelPending (cancels h2)
    assert_eq!(
        engine.get_current_state(),
        State::CancelPending,
        "the second round did not reach the state that runs `<cancel sendid=\"h2\">`",
    );

    // 400 ms: h2's deadline. It was cancelled at 300, so nothing may happen.
    advance_to(&mut engine, &now, 400);
    assert_eq!(
        calls(&log),
        vec![200],
        "the handler was asked to perform `h2` at 400 ms after `<cancel sendid=\"h2\">` \
         ran at 300 ms. A host-served act that a document cancelled must not reach \
         the host: the side effect is the point of the act, and the document has no \
         way to take it back",
    );
    assert_ne!(
        engine.get_current_state(),
        State::CancelLost,
        "`turn.done` arrived for the cancelled send",
    );

    // 500 ms: `finish`. The verdict is itself scheduled, so a channel whose
    // tick loop stopped working fails here rather than passing by not moving.
    advance_to(&mut engine, &now, 500);
    assert_eq!(
        engine.get_current_state(),
        State::Pass,
        "the machine did not reach `pass`; it is in {:?}",
        engine.get_current_state()
    );
}

/// The engine must be able to say when the deferred host send comes due, or a
/// host driving on `time_until_next_scheduled_ms` sleeps straight past it.
///
/// A deferred act kept anywhere the deadline query cannot see would leave this
/// answering `None` at 0 ms — "nothing is owed" — while an act was owed at 200.
#[test]
fn the_engine_says_when_the_deferred_host_send_is_due() {
    let (mut engine, _log, now) = armed();

    let due = engine
        .time_until_next_scheduled_ms()
        .expect("two delayed sends are armed at 0 ms, so a deadline is owed");
    assert_eq!(
        due, 100,
        "the nearer of the two armed sends is the 100 ms `probe`; the engine answered \
         {due} ms",
    );

    advance_to(&mut engine, &now, 100);
    let due = engine
        .time_until_next_scheduled_ms()
        .expect("the host-served send is still pending at 100 ms, so a deadline is owed");
    assert_eq!(
        due, 100,
        "at 100 ms the host-served send is 100 ms out; the engine answered {due} ms. A \
         host sleeping on this answer must land on the deferred act, not past it",
    );
}

/// A deferred act whose handler was never registered is still an act nobody
/// performed, and §scxml-6.2 reports that as `error.execution` — at the moment
/// it was to be performed, not at the moment it was armed.
///
/// The immediate path raises this at the send site. The deferred path cannot:
/// the send site has already returned by the time the deadline arrives, so the
/// engine owes the report. Without this case a wiring mistake on a delayed send
/// is perfect silence — the document waits for a reply that no longer has
/// anyone to come from.
#[test]
fn a_deferred_send_with_no_handler_reports_it_when_it_comes_due() {
    let mut engine = Engine::new(Policy::new());
    engine.set_clock(SceClock::Manual(0));
    engine.initialize();

    // At 100 ms the machine is in `armed`, whose `error.execution` transition
    // is the witness. Nothing has reported anything yet: the send was armed,
    // not performed, so there is nothing to report.
    engine.advance_time_ms(100);
    assert_eq!(
        engine.get_current_state(),
        State::Armed,
        "the report arrived before the send was due; `error.execution` must be \
         raised when the act was to be performed, not when it was armed",
    );

    // 200 ms: the deadline. Nobody is registered, so nobody performs it, and
    // §scxml-6.2 says so.
    engine.advance_time_ms(100);
    assert_ne!(
        engine.get_current_state(),
        State::Cancelling,
        "nothing was registered to perform the act, yet `turn.done` arrived — an \
         unwired host processor read as a served one",
    );
    assert_eq!(
        engine.get_current_state(),
        State::Unserved,
        "the deadline passed with no handler registered and nothing was reported. \
         The send site that raises this for an immediate send returned when the \
         send was armed, so whatever holds the deferred act owes the report — \
         without it a wiring mistake on a delayed send is perfect silence and the \
         document waits for a reply that has nobody to come from",
    );
}

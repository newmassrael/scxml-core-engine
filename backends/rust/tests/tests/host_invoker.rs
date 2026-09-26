// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.4.1 — Rust compile+run gate for an `<invoke type>` the HOST
// runs.
//
// The `<send>` half of this axis is `host_processor.rs`. This is the other
// one, and it is not the same shape: an invoke has a LIFETIME. It starts
// when its state is entered and the macrostep has settled, it is cancelled
// when that state exits, and the document may be waiting on
// `done.invoke.<id>`. A gate that only proved "the handler was called"
// would leave the teardown untested, and a host told to start work and
// never told to stop is worse than one never asked.
//
// The committed SM under `src/integration/host_processor/` is generated
// from `sce-build/tests/fixtures/host_processor/statechart_host_invoker.scxml`
// WITH the declaration (regen: `scripts/regen_host_processor.sh`). One
// binary, driven three ways, so what these measure is the registration and
// not the build.
//
// What this gate does NOT claim, because SCE does not route it: parent-to-
// child `<send target="#_invokeid">`, `autoforward`, and `<finalize>`.
// Those are mechanics between two SCXML sessions; a host invoker takes its
// input from the `Start` request and answers by raising events. Named here
// so the absence is a statement rather than an oversight.

use std::sync::{Arc, Mutex};

use sce_rust_runtime::{Engine, HostInvokeEvent, HostInvokeResponse, IScriptEngine};
use sce_rust_tests::integration::host_processor::{
    StatechartHostInvokerEvent as Event, StatechartHostInvokerPolicy as Policy,
};

/// The type the fixture was compiled for; `scripts/regen_host_processor.sh`
/// passes the same string to the invoker flag.
const DECLARED_TYPE: &str = "x-sce-host";

fn started() -> (Engine<Policy>, Arc<dyn IScriptEngine>) {
    let script_engine: Arc<dyn IScriptEngine> = Arc::new(sce_rust_lua::LuaEngine::new());
    let engine = Engine::new(Policy::new(Arc::clone(&script_engine)));
    (engine, script_engine)
}

fn counter(engine: &Engine<Policy>, script_engine: &Arc<dyn IScriptEngine>, name: &str) -> i64 {
    sce_rust_runtime::helpers::datamodel_read::read_int(
        &**script_engine,
        engine.policy().session_id.as_deref(),
        name,
    )
    .unwrap_or_else(|| panic!("the fixture declares `{name}` in its datamodel"))
}

/// A recording invoker. Returns `done_data` on `Start` so the completion
/// path is exercised too.
fn recording_invoker(
    log: &Arc<Mutex<Vec<String>>>,
) -> impl FnMut(HostInvokeEvent) -> Option<HostInvokeResponse> + Send + 'static {
    let log = Arc::clone(log);
    move |ev: HostInvokeEvent| match ev {
        HostInvokeEvent::Start(req) => {
            log.lock().expect("invoker log").push(format!(
                "START id={} type={} src={} within={:?}",
                req.invoke_id,
                req.processor_type,
                req.src,
                req.params.get("within"),
            ));
            Some(HostInvokeResponse {
                done_data: Some("ok".to_string()),
            })
        }
        HostInvokeEvent::Cancel(c) => {
            log.lock()
                .expect("invoker log")
                .push(format!("CANCEL id={}", c.invoke_id));
            None
        }
    }
}

/// Every start the running invoker saw: `(invoke_id, token)`, in order.
type Starts = Arc<Mutex<Vec<(String, u64)>>>;

/// An invoker whose work outlives the call: it answers nothing on `Start`,
/// so each invocation stays running until the test completes or cancels
/// it. Records the log lines [`recording_invoker`] does, and the token of
/// every start for a later [`Engine::complete_host_invoke`].
fn running_invoker(
    log: &Arc<Mutex<Vec<String>>>,
    starts: &Starts,
) -> impl FnMut(HostInvokeEvent) -> Option<HostInvokeResponse> + Send + 'static {
    let log = Arc::clone(log);
    let starts = Arc::clone(starts);
    move |ev: HostInvokeEvent| {
        match ev {
            HostInvokeEvent::Start(req) => {
                log.lock()
                    .expect("invoker log")
                    .push(format!("START id={}", req.invoke_id));
                starts
                    .lock()
                    .expect("invoker starts")
                    .push((req.invoke_id, req.token));
            }
            HostInvokeEvent::Cancel(c) => {
                log.lock()
                    .expect("invoker log")
                    .push(format!("CANCEL id={}", c.invoke_id));
            }
        }
        None
    }
}

/// The token of the latest start of `invoke_id`.
fn token_of(starts: &Starts, invoke_id: &str) -> u64 {
    starts
        .lock()
        .expect("invoker starts")
        .iter()
        .rev()
        .find(|(id, _)| id == invoke_id)
        .map(|(_, token)| *token)
        .unwrap_or_else(|| panic!("`{invoke_id}` never started"))
}

#[test]
fn a_registered_invoker_is_started_with_what_the_document_wrote() {
    let log: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let (mut engine, script_engine) = started();
    engine.register_invoker(DECLARED_TYPE, recording_invoker(&log));
    engine.initialize();
    engine.step();

    // The fixture counts a completion only when its `_event.invokeid` names
    // the invocation (§scxml-5.10.1), so each counter is that assertion too.
    assert_eq!(
        counter(&engine, &script_engine, "started"),
        1,
        "done.invoke.probe never reached the document, or arrived without its invokeid",
    );
    assert_eq!(
        counter(&engine, &script_engine, "started2"),
        1,
        "done.invoke.probe2 never reached the document, or arrived without its invokeid",
    );
    assert_eq!(counter(&engine, &script_engine, "refused"), 0);
    // The false-positive guard: ordinary entry content must still run.
    assert_eq!(
        counter(&engine, &script_engine, "entered"),
        1,
        "the entry chain stopped running",
    );

    let seen = log.lock().expect("invoker log");
    assert_eq!(seen.len(), 2, "invoker calls: {seen:?}");
    // `src` and `<param>` are how §scxml-6.4.1 lets the document say WHAT
    // to invoke and with what. A request carrying neither would let a
    // document name an invocation it cannot describe. Each invocation is
    // started as itself: `probe2` begins with `probe`, and a dispatch that
    // matched by substring started `probe` twice.
    assert_eq!(
        seen[0],
        format!("START id=probe type={DECLARED_TYPE} src=pane://turn within=Some([\"2500\"])"),
        "the start request lost part of what the document wrote",
    );
    assert_eq!(
        seen[1],
        format!("START id=probe2 type={DECLARED_TYPE} src=pane://other within=None"),
        "the second invocation was not started as itself",
    );
}

/// §scxml-6.4.1: `srcexpr`, `namelist`, `<param expr>` and `<content expr>`
/// are read from the data model when the invocation starts. The request used
/// to carry the literal params alone, so a document that computed what to
/// invoke handed the host an empty description.
#[test]
fn what_the_request_says_is_evaluated_when_the_invocation_starts() {
    let starts: Arc<Mutex<Vec<sce_rust_runtime::HostInvokeRequest>>> =
        Arc::new(Mutex::new(Vec::new()));
    let (mut engine, script_engine) = started();
    let sink = Arc::clone(&starts);
    engine.register_invoker(DECLARED_TYPE, move |ev: HostInvokeEvent| {
        if let HostInvokeEvent::Start(req) = ev {
            sink.lock().expect("invoker log").push(req);
        }
        None
    });
    engine.initialize();
    engine.step();
    // `probe` / `probe2`, which the case above already reads.
    starts.lock().expect("invoker log").clear();
    engine.process_event(Event::Evaluate);

    let seen = starts.lock().expect("invoker log");
    let ids: Vec<&str> = seen.iter().map(|r| r.invoke_id.as_str()).collect();
    // `req3`'s srcexpr cannot be evaluated, so it is never started.
    assert_eq!(ids, ["req", "req2"], "started: {ids:?}");
    assert_eq!(seen[0].src, "pane://dyn", "srcexpr was not evaluated");
    // A repeated name keeps both values in document order; the `<param>`
    // that failed is absent (§scxml-5.7.1) while the invocation still started.
    let expected: std::collections::HashMap<String, Vec<String>> = [
        ("n".to_string(), vec!["7".to_string()]),
        ("twice".to_string(), vec!["a".to_string(), "8".to_string()]),
    ]
    .into_iter()
    .collect();
    assert_eq!(seen[0].params, expected, "params");
    assert_eq!(seen[1].content, "body:7", "content");
    // One error.execution for the dropped `<param>`, one for `req3`.
    assert_eq!(
        counter(&engine, &script_engine, "dropped"),
        2,
        "a failed evaluation was not reported",
    );
}

/// The invocation ends with the state that started it. Without this the
/// host is told to begin work and never told to stop — which no
/// configuration assertion can detect, because the machine looks correct
/// either way.
#[test]
fn leaving_the_state_cancels_the_invocation() {
    let log: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let starts: Starts = Arc::default();
    let (mut engine, script_engine) = started();
    // Still running when the state exits — a completed invocation has
    // nothing left to cancel (see the case after this one).
    engine.register_invoker(DECLARED_TYPE, running_invoker(&log, &starts));
    engine.initialize();
    engine.step();
    engine.process_event(Event::Leave);

    assert_eq!(
        counter(&engine, &script_engine, "ended"),
        1,
        "the machine never left the invoking state",
    );
    // Both invocations end with the state, each told once.
    let seen = log.lock().expect("invoker log");
    for id in ["probe", "probe2"] {
        let cancel = format!("CANCEL id={id}");
        assert_eq!(
            seen.iter().filter(|e| **e == cancel).count(),
            1,
            "cancel for {id} did not reach the invoker exactly once: {seen:?}",
        );
    }
}

/// A cancel is delivered once, and only for an invocation that started.
///
/// The engine, not the emitted code, owns that judgement: the exit chain
/// calls `cancel_host_invoke` unconditionally, so if the engine did not
/// track what started, a state that exits before its macrostep settles
/// would have the host tearing down work it never began.
///
/// Asserted at the engine surface rather than through the fixture on
/// purpose. Driving the machine cannot produce the "never started" case
/// here — every host call that advances it runs a macrostep, and the
/// pending invoke executes at the end of that macrostep, so by the time
/// any exit is reachable the invocation has started. A first attempt
/// tried it through the fixture and measured the opposite of what it
/// claimed.
#[test]
fn cancel_is_not_delivered_for_an_invocation_that_never_started() {
    let log: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let starts: Starts = Arc::default();
    let (mut engine, _script_engine) = started();
    engine.register_invoker(DECLARED_TYPE, running_invoker(&log, &starts));

    // Nothing has started, so there is nothing to cancel.
    assert!(
        !engine.cancel_host_invoke(DECLARED_TYPE, "probe"),
        "a cancel was reported for an invocation that never started",
    );
    assert!(
        log.lock().expect("invoker log").is_empty(),
        "the invoker was called for an invocation that never started",
    );

    // Now let one start, cancel it, and cancel again: the second call has
    // nothing left to do. A registry that answered twice would have the
    // host tear down the same work twice.
    engine.initialize();
    engine.step();
    assert!(engine.cancel_host_invoke(DECLARED_TYPE, "probe"));
    assert!(
        !engine.cancel_host_invoke(DECLARED_TYPE, "probe"),
        "the same invocation was cancelled twice",
    );
    let seen = log.lock().expect("invoker log");
    assert_eq!(
        seen.iter().filter(|e| e.starts_with("CANCEL")).count(),
        1,
        "cancel reached the invoker more than once: {seen:?}",
    );
}

/// §scxml-6.4: `done.invoke` says the invoked process is over, so leaving
/// the state afterwards has nothing to stop. Both invocations here complete
/// synchronously; neither is cancelled.
#[test]
fn a_completed_invocation_is_not_cancelled() {
    let log: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let (mut engine, script_engine) = started();
    engine.register_invoker(DECLARED_TYPE, recording_invoker(&log));
    engine.initialize();
    engine.step();
    engine.process_event(Event::Leave);

    assert_eq!(counter(&engine, &script_engine, "started"), 1);
    assert_eq!(counter(&engine, &script_engine, "ended"), 1);
    let seen = log.lock().expect("invoker log");
    assert!(
        !seen.iter().any(|e| e.starts_with("CANCEL")),
        "a completed invocation was cancelled: {seen:?}",
    );
}

/// A host that finishes later reports it with the start's token, and the
/// completion is taken once: a second report of the same run finds nothing,
/// and the state's exit then cancels only the invocation still running.
#[test]
fn a_late_completion_is_accepted_exactly_once() {
    let log: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let starts: Starts = Arc::default();
    let (mut engine, script_engine) = started();
    engine.register_invoker(DECLARED_TYPE, running_invoker(&log, &starts));
    engine.initialize();
    engine.step();
    assert_eq!(counter(&engine, &script_engine, "started"), 0);

    let token = token_of(&starts, "probe");
    assert!(
        engine.complete_host_invoke(DECLARED_TYPE, "probe", token, "ok"),
        "a running invocation's completion was refused",
    );
    engine.step();
    // The fixture counts it only when `_event.invokeid` names the
    // invocation (§scxml-5.10.1), so this is that assertion too.
    assert_eq!(counter(&engine, &script_engine, "started"), 1);

    assert!(
        !engine.complete_host_invoke(DECLARED_TYPE, "probe", token, "again"),
        "the same run completed twice",
    );
    engine.step();
    assert_eq!(counter(&engine, &script_engine, "started"), 1);

    engine.process_event(Event::Leave);
    let seen = log.lock().expect("invoker log");
    assert_eq!(
        seen.iter()
            .filter(|e| e.starts_with("CANCEL"))
            .cloned()
            .collect::<Vec<_>>(),
        ["CANCEL id=probe2"],
        "only the invocation still running is cancelled",
    );
}

/// What [`timed`] hands a case: the machine, its script engine, the
/// invoker's log and every start's token.
type Timed = (
    Engine<Policy>,
    Arc<dyn IScriptEngine>,
    Arc<Mutex<Vec<String>>>,
    Starts,
);

/// A machine on host-owned time, with an invoker that answers nothing and
/// records, per start, the token and whether the request still carried the
/// deadline param.
fn timed() -> Timed {
    let log: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let starts: Starts = Arc::default();
    let (mut engine, script_engine) = started();
    engine.set_clock(sce_rust_runtime::SceClock::Manual(0));
    let (log_in, starts_in) = (Arc::clone(&log), Arc::clone(&starts));
    engine.register_invoker(DECLARED_TYPE, move |ev: HostInvokeEvent| {
        match ev {
            HostInvokeEvent::Start(req) => {
                log_in.lock().expect("log").push(format!(
                    "START id={} deadline-param={}",
                    req.invoke_id,
                    req.params.contains_key("_sce_deadline_ms"),
                ));
                starts_in
                    .lock()
                    .expect("starts")
                    .push((req.invoke_id, req.token));
            }
            HostInvokeEvent::Cancel(c) => {
                log_in
                    .lock()
                    .expect("log")
                    .push(format!("CANCEL id={}", c.invoke_id));
            }
        }
        None
    });
    engine.initialize();
    engine.step();
    engine.process_event(Event::Time);
    (engine, script_engine, log, starts)
}

/// A deadline that passes while the invocation is still running ends it:
/// the host is told to stop, the document receives `error.invoke.slow` with
/// `_event.data` "deadline", and a reply the host sends afterwards is refused.
/// The param is the engine's — the host never sees it.
#[test]
fn a_deadline_that_passes_ends_the_invocation() {
    let (mut engine, script_engine, log, starts) = timed();
    assert!(
        log.lock()
            .expect("log")
            .contains(&"START id=slow deadline-param=false".to_string()),
        "the host was handed the deadline param, or `slow` never started: {:?}",
        log.lock().expect("log"),
    );

    // A `<cancel>` of the empty send id must not reach the deadline.
    engine.process_event(Event::Forget);
    engine.advance_time_ms(49);
    assert_eq!(
        counter(&engine, &script_engine, "expired"),
        0,
        "expired early"
    );

    engine.advance_time_ms(1);
    assert_eq!(counter(&engine, &script_engine, "expired"), 1);
    assert!(
        log.lock()
            .expect("log")
            .contains(&"CANCEL id=slow".to_string()),
        "the host was not told to stop: {:?}",
        log.lock().expect("log"),
    );

    let token = token_of(&starts, "slow");
    assert!(
        !engine.complete_host_invoke(DECLARED_TYPE, "slow", token, "late"),
        "a reply after the deadline was accepted",
    );
    engine.step();
    assert_eq!(counter(&engine, &script_engine, "finished"), 0);
}

/// The discriminator: a completion before the deadline is the outcome, and
/// the deadline that comes due afterwards does nothing — no cancel, no
/// `error.invoke`, and nothing left for the host to tick toward.
#[test]
fn a_completion_before_the_deadline_disarms_it() {
    let (mut engine, script_engine, log, starts) = timed();
    let token = token_of(&starts, "slow");
    assert!(engine.complete_host_invoke(DECLARED_TYPE, "slow", token, "ok"));
    engine.step();
    assert_eq!(counter(&engine, &script_engine, "finished"), 1);
    assert_eq!(
        engine.time_until_next_scheduled_ms(),
        None,
        "the disarmed deadline is still keeping the host ticking",
    );

    engine.advance_time_ms(100);
    assert_eq!(counter(&engine, &script_engine, "expired"), 0);
    assert!(
        !log.lock()
            .expect("log")
            .contains(&"CANCEL id=slow".to_string()),
        "a completed invocation was cancelled by its deadline",
    );
}

/// §scxml-6.4.1: a deadline that is not a whole number of milliseconds is an
/// argument that cannot be evaluated — `error.execution`, and the host is
/// never asked to start the invocation.
#[test]
fn a_deadline_that_is_not_milliseconds_starts_nothing() {
    let (engine, script_engine, log, _starts) = timed();
    assert_eq!(counter(&engine, &script_engine, "misdated"), 1);
    assert!(
        !log.lock()
            .expect("log")
            .iter()
            .any(|e| e.starts_with("START id=undated")),
        "an invocation with an unreadable deadline was started: {:?}",
        log.lock().expect("log"),
    );
}

/// Every runtime reads a deadline's text by one grammar, held to one table.
/// A number parser would not do: the languages' parsers disagree about hex
/// floats, digit separators and whitespace, and a deadline one backend
/// honours and another refuses makes a document depend on where it was
/// compiled.
#[test]
fn a_deadline_is_read_by_the_shared_table() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../sce-build/tests/fixtures/host_processor/host_invoke_deadline_values.json");
    let text =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let table: serde_json::Value = serde_json::from_str(&text).expect("the table is JSON");

    let accepted = table["accepted"].as_array().expect("`accepted` is a list");
    let refused = table["refused"].as_array().expect("`refused` is a list");
    // A floor: an empty table would pass every assertion below.
    assert!(
        !accepted.is_empty() && !refused.is_empty(),
        "the table is empty"
    );

    for pair in accepted {
        let written = pair[0].as_str().expect("written is a string");
        let ms: u64 = pair[1]
            .as_str()
            .expect("milliseconds are a string")
            .parse()
            .expect("milliseconds are a number");
        assert_eq!(
            sce_rust_runtime::parse_host_invoke_deadline_ms(written),
            Some(ms),
            "{written:?}",
        );
    }
    for written in refused {
        let written = written.as_str().expect("a refused value is a string");
        assert_eq!(
            sce_rust_runtime::parse_host_invoke_deadline_ms(written),
            None,
            "{written:?} was accepted",
        );
    }
}

/// §scxml-6.4: once the state has exited, what the cancelled process sends
/// is ignored. The host's reply arrives after the cancel and is refused.
#[test]
fn a_completion_after_the_cancel_is_refused() {
    let log: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let starts: Starts = Arc::default();
    let (mut engine, script_engine) = started();
    engine.register_invoker(DECLARED_TYPE, running_invoker(&log, &starts));
    engine.initialize();
    engine.step();
    let token = token_of(&starts, "probe");
    engine.process_event(Event::Leave);

    assert!(
        !engine.complete_host_invoke(DECLARED_TYPE, "probe", token, "late"),
        "a cancelled run's completion was accepted",
    );
    engine.step();
    assert_eq!(counter(&engine, &script_engine, "started"), 0);
}

/// Re-entering the state starts the same `<invoke>` again under the same
/// id. The first run's late reply carries the first start's token and is
/// refused; the second run's is accepted.
#[test]
fn a_restarted_invoke_refuses_the_first_runs_reply() {
    let log: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let starts: Starts = Arc::default();
    let (mut engine, script_engine) = started();
    engine.register_invoker(DECLARED_TYPE, running_invoker(&log, &starts));
    engine.initialize();
    engine.step();
    let first = token_of(&starts, "probe");
    engine.process_event(Event::Leave);
    engine.process_event(Event::Again);
    let second = token_of(&starts, "probe");
    assert_ne!(first, second, "a restart reused the first start's token");

    assert!(
        !engine.complete_host_invoke(DECLARED_TYPE, "probe", first, "stale"),
        "the first run's reply was taken for the second run's",
    );
    engine.step();
    assert_eq!(counter(&engine, &script_engine, "started"), 0);
    assert!(engine.complete_host_invoke(DECLARED_TYPE, "probe", second, "ok"));
    engine.step();
    assert_eq!(counter(&engine, &script_engine, "started"), 1);
}

/// A host-run invocation's `done.invoke` raised through the ordinary
/// external-event API skipped the running check, so the engine refuses it
/// and counts the refusal. The metadata names the invocation, so without
/// the refusal the fixture's guarded transition would take it.
#[test]
fn a_done_invoke_raised_the_old_way_is_refused_and_counted() {
    let log: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let starts: Starts = Arc::default();
    let (mut engine, script_engine) = started();
    engine.register_invoker(DECLARED_TYPE, running_invoker(&log, &starts));
    engine.initialize();
    engine.step();

    let metadata = sce_rust_runtime::event::EventMetadata {
        invoke_id: "probe".into(),
        ..Default::default()
    };
    engine.raise_external_by_name_with_meta("done.invoke.probe", &metadata);
    engine.step();

    assert_eq!(
        counter(&engine, &script_engine, "started"),
        0,
        "a completion that skipped the running check reached the document",
    );
    assert_eq!(engine.refused_host_invoke_completions(), 1);
}

/// §scxml-6.4.1: an invoke with no id and an `idlocation` gets a generated
/// `stateid.platformid` id, written to the location, handed to the host, and
/// carried as `_event.invokeid` on the completion. The document names no
/// specific `done.invoke.<id>`, so the completion arrives as the generic
/// `done.invoke` — which used to be dropped, because only the specific name
/// was looked up — and `matched` counts it only when its invokeid is what the
/// document stored.
#[test]
fn an_idlocation_holds_the_id_the_host_is_handed() {
    let log: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let (mut engine, script_engine) = started();
    engine.register_invoker(DECLARED_TYPE, recording_invoker(&log));
    engine.initialize();
    engine.step();
    engine.process_event(Event::Leave);

    let seen = log.lock().expect("invoker log");
    assert!(
        seen.iter()
            .any(|e| e.starts_with("START id=done._invoke_0 ")),
        "the host was not handed the generated id: {seen:?}",
    );
    assert_eq!(
        counter(&engine, &script_engine, "matched"),
        1,
        "the completion did not arrive, or its invokeid is not what idlocation holds",
    );
}

/// §scxml-6.2.4 / §scxml-6.4.1: an `idlocation` is a location expression,
/// so the id is written the way `<assign>` writes (§scxml-5.4): `slot.id`
/// and `slot.sid` are member paths, and land. `n.nope.deeper` cannot take a
/// value, so each element raises error.execution and is abandoned
/// (§scxml-5.9.2) — the host is never asked to start that invoke, and that
/// message is never sent.
#[test]
fn an_idlocation_is_assigned_like_a_location() {
    let log: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let (mut engine, script_engine) = started();
    engine.register_invoker(DECLARED_TYPE, recording_invoker(&log));
    engine.initialize();
    engine.step();
    engine.process_event(Event::Locate);
    engine.step();

    let seen = log.lock().expect("invoker log").clone();
    assert!(
        seen.iter()
            .any(|e| e.starts_with("START id=locating._invoke_1 ")),
        "the member-path invoke was not started: {seen:?}",
    );
    assert!(
        !seen.iter().any(|e| e.contains("locating._invoke_2")),
        "an invoke whose idlocation could not take the id was started: {seen:?}",
    );

    let slot: serde_json::Value = serde_json::from_str(
        &engine
            .policy()
            .slot()
            .expect("the fixture declares `slot` as an object"),
    )
    .expect("`slot` reads back as JSON");
    assert_eq!(slot["id"], "locating._invoke_1", "slot.id: {slot}");
    assert!(
        slot["sid"].as_str().is_some_and(|s| !s.is_empty()),
        "slot.sid did not receive the send id: {slot}",
    );

    assert_eq!(counter(&engine, &script_engine, "slotted"), 1);
    assert_eq!(counter(&engine, &script_engine, "pinged"), 1);
    assert_eq!(
        counter(&engine, &script_engine, "leaked"),
        0,
        "a send whose idlocation could not take the id was still sent",
    );
    assert_eq!(counter(&engine, &script_engine, "lost"), 2);
}

/// The generic `done.invoke` is a host completion too when its invokeid
/// names a host-run invoke, so raised around `complete_host_invoke` it is
/// refused like the specific name is.
#[test]
fn a_generic_done_invoke_raised_the_old_way_is_refused() {
    let log: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let starts: Starts = Arc::default();
    let (mut engine, script_engine) = started();
    engine.register_invoker(DECLARED_TYPE, running_invoker(&log, &starts));
    engine.initialize();
    engine.step();
    engine.process_event(Event::Leave);

    let metadata = sce_rust_runtime::event::EventMetadata {
        invoke_id: "done._invoke_0".into(),
        ..Default::default()
    };
    engine.raise_external_by_name_with_meta("done.invoke", &metadata);
    engine.step();

    assert_eq!(
        counter(&engine, &script_engine, "matched"),
        0,
        "a completion that skipped the running check reached the document",
    );
    assert_eq!(engine.refused_host_invoke_completions(), 1);
}

/// The other half. The build declared the type, so codegen emitted a start
/// — but nothing was registered, so no process was run. Same event as an
/// unsupported type, because from the document's side it is the same fact.
#[test]
fn a_declared_type_with_no_invoker_still_raises_error_execution() {
    let (mut engine, script_engine) = started();
    engine.initialize();
    engine.step();

    // One error.execution per invocation nobody ran.
    assert_eq!(
        counter(&engine, &script_engine, "refused"),
        2,
        "an unregistered invoker was silently treated as started",
    );
    assert_eq!(counter(&engine, &script_engine, "started"), 0);
}

/// An invoker registered for another type does not run this one, and the
/// send-side registry is not consulted either: `register_event_processor`
/// and `register_invoker` are two contracts, and one satisfying the other
/// would let a host promise a lifecycle it never implemented.
#[test]
fn neither_another_type_nor_a_send_processor_serves_this_invoke() {
    let (mut engine, script_engine) = started();
    engine.register_invoker("x-some-other-host", |_ev: HostInvokeEvent| None);
    engine.register_event_processor(DECLARED_TYPE, |_req| Vec::new());
    engine.initialize();
    engine.step();

    assert_eq!(
        counter(&engine, &script_engine, "started"),
        0,
        "an invoke was served by the wrong registration",
    );
    assert_eq!(counter(&engine, &script_engine, "refused"), 2);
}

/// A host that has nothing to report yet returns `None`, and SCE must not
/// synthesise a completion it did not report — §scxml-6.4 fires
/// `done.invoke` when the invoked process terminates, and an invocation
/// still running has not.
#[test]
fn an_invoker_that_reports_no_completion_fires_no_done_invoke() {
    let (mut engine, script_engine) = started();
    engine.register_invoker(DECLARED_TYPE, |ev: HostInvokeEvent| match ev {
        HostInvokeEvent::Start(_) => Some(HostInvokeResponse { done_data: None }),
        HostInvokeEvent::Cancel(_) => None,
    });
    engine.initialize();
    engine.step();

    assert_eq!(
        counter(&engine, &script_engine, "started"),
        0,
        "a completion was invented for an invocation still running",
    );
    assert_eq!(
        counter(&engine, &script_engine, "refused"),
        0,
        "an invocation that started was reported as refused",
    );
}

/// The query the host uses to check its own wiring.
#[test]
fn the_registry_reports_which_invokers_it_holds() {
    let (mut engine, _script_engine) = started();
    assert!(!engine.has_invoker(DECLARED_TYPE));
    engine.register_invoker(DECLARED_TYPE, |_ev: HostInvokeEvent| None);
    assert!(engine.has_invoker(DECLARED_TYPE));
    // The two registries are separate: registering an invoker does not
    // make the same type a send processor.
    assert!(!engine.has_event_processor(DECLARED_TYPE));
}

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

#[test]
fn a_registered_invoker_is_started_with_what_the_document_wrote() {
    let log: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let (mut engine, script_engine) = started();
    engine.register_invoker(DECLARED_TYPE, recording_invoker(&log));
    engine.initialize();
    engine.step();

    assert_eq!(
        counter(&engine, &script_engine, "started"),
        1,
        "done.invoke never reached the document",
    );
    assert_eq!(counter(&engine, &script_engine, "refused"), 0);
    // The false-positive guard: ordinary entry content must still run.
    assert_eq!(
        counter(&engine, &script_engine, "entered"),
        1,
        "the entry chain stopped running",
    );

    let seen = log.lock().expect("invoker log");
    assert_eq!(seen.len(), 1, "invoker calls: {seen:?}");
    // `src` and `<param>` are how §scxml-6.4.1 lets the document say WHAT
    // to invoke and with what. A request carrying neither would let a
    // document name an invocation it cannot describe.
    assert_eq!(
        seen[0],
        format!("START id=probe type={DECLARED_TYPE} src=pane://turn within=Some([\"2500\"])"),
        "the start request lost part of what the document wrote",
    );
}

/// The invocation ends with the state that started it. Without this the
/// host is told to begin work and never told to stop — which no
/// configuration assertion can detect, because the machine looks correct
/// either way.
#[test]
fn leaving_the_state_cancels_the_invocation() {
    let log: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let (mut engine, script_engine) = started();
    engine.register_invoker(DECLARED_TYPE, recording_invoker(&log));
    engine.initialize();
    engine.step();
    engine.process_event(Event::Leave);

    assert_eq!(
        counter(&engine, &script_engine, "ended"),
        1,
        "the machine never left the invoking state",
    );
    let seen = log.lock().expect("invoker log");
    assert_eq!(
        seen.last().map(String::as_str),
        Some("CANCEL id=probe"),
        "no cancel reached the invoker: {seen:?}",
    );
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
    let (mut engine, _script_engine) = started();
    engine.register_invoker(DECLARED_TYPE, recording_invoker(&log));

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

/// The other half. The build declared the type, so codegen emitted a start
/// — but nothing was registered, so no process was run. Same event as an
/// unsupported type, because from the document's side it is the same fact.
#[test]
fn a_declared_type_with_no_invoker_still_raises_error_execution() {
    let (mut engine, script_engine) = started();
    engine.initialize();
    engine.step();

    assert_eq!(
        counter(&engine, &script_engine, "refused"),
        1,
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
    engine.register_event_processor(DECLARED_TYPE, |_req| None);
    engine.initialize();
    engine.step();

    assert_eq!(
        counter(&engine, &script_engine, "started"),
        0,
        "an invoke was served by the wrong registration",
    );
    assert_eq!(counter(&engine, &script_engine, "refused"), 1);
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

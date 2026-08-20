// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.2.5 — Rust compile+run gate for a `<send type>` the HOST
// serves.
//
// §6.2.5 makes the Event I/O Processor identifier extensible, so the set is
// open by design. SCE implemented two of them and refused everything else
// with `error.execution`, and nothing let a platform widen the set: a
// consumer could name a processor and be refused, but not name one and be
// served. Reported from downstream on 2026-08-20 by a Rust consumer that
// wanted its own acts to become document vocabulary and found the engine
// accepted the document, then failed at run time with nothing before that
// saying it would.
//
// The committed SM under `src/integration/host_processor/` is generated from
// `sce-build/tests/fixtures/host_processor/statechart_host_processor.scxml`
// WITH the declaration (regen: `scripts/regen_host_processor.sh`). Because
// the tree is part of the crate it is really type-checked; these tests drive
// that one binary twice, so what they measure is the registration and not
// the build.
//
// The pair is the whole contract:
//
//   * a registered handler receives the send and its reply arrives as an
//     event — the feature working;
//   * the same machine with nothing registered raises `error.execution` —
//     a wiring mistake staying visible instead of reading as success.
//
// Both are needed. A gate holding only the first would pass on an engine
// that dispatched to nothing and called it delivered, which is exactly the
// silence being repaid.

use std::sync::{Arc, Mutex};

use sce_rust_runtime::{Engine, HostSendRequest, HostSendResponse, IScriptEngine};
use sce_rust_tests::integration::host_processor::StatechartHostProcessorPolicy as Policy;

/// The type the fixture was compiled for. `scripts/regen_host_processor.sh`
/// passes this same string to `--host-processor`; a test that registered a
/// different one would measure nothing and pass, so the two spellings are
/// asserted to be one by the `refused` counter below rather than trusted.
const DECLARED_TYPE: &str = "x-sce-host";

fn started() -> (Engine<Policy>, Arc<dyn IScriptEngine>) {
    let script_engine: Arc<dyn IScriptEngine> = Arc::new(sce_rust_lua::LuaEngine::new());
    let engine = Engine::new(Policy::new(Arc::clone(&script_engine)));
    (engine, script_engine)
}

/// The fixture's `<assign>`s are the only witness: every outcome here leaves
/// the machine in the same single state, so the configuration cannot tell
/// them apart.
fn counter(engine: &Engine<Policy>, script_engine: &Arc<dyn IScriptEngine>, name: &str) -> i64 {
    sce_rust_runtime::helpers::datamodel_read::read_int(
        &**script_engine,
        engine.policy().session_id.as_deref(),
        name,
    )
    .unwrap_or_else(|| panic!("the fixture declares `{name}` in its datamodel"))
}

#[test]
fn a_registered_handler_receives_the_send_and_its_reply_arrives() {
    let seen: Arc<Mutex<Vec<HostSendRequest>>> = Arc::new(Mutex::new(Vec::new()));
    let recorder = Arc::clone(&seen);

    let (mut engine, script_engine) = started();
    engine.register_event_processor(DECLARED_TYPE, move |req: HostSendRequest| {
        recorder.lock().expect("handler log").push(req);
        // The request/reply shape: the reply becomes an event the document
        // was already waiting for, which is what lets a state DECLARE an
        // act instead of a host-side table performing it.
        Some(HostSendResponse {
            event_name: "turn.done".to_string(),
            event_data: String::new(),
        })
    });
    engine.initialize();
    engine.step();

    assert_eq!(
        counter(&engine, &script_engine, "served"),
        1,
        "the handler's reply never reached the document",
    );
    assert_eq!(
        counter(&engine, &script_engine, "refused"),
        0,
        "a served send also raised error.execution",
    );
    // The false-positive guard: an ordinary `<send>` in the same block must
    // still deliver. Without it a change that broke every send while leaving
    // the host branch intact would read as a pass.
    assert_eq!(
        counter(&engine, &script_engine, "plain"),
        1,
        "an ordinary <send> in the same block stopped delivering",
    );

    let requests = seen.lock().expect("handler log");
    assert_eq!(
        requests.len(),
        1,
        "the handler ran {} times",
        requests.len()
    );
    let req = &requests[0];
    assert_eq!(req.processor_type, DECLARED_TYPE);
    assert_eq!(req.event_name, "watch.turn");
    // The payload the author wrote has to survive the crossing, or the
    // document can name an act but not parameterise it — which is most of
    // the reason to move an act into the document at all.
    assert_eq!(
        req.params.get("within").map(Vec::as_slice),
        Some(["2500".to_string()].as_slice()),
        "the <param> did not reach the handler: {:?}",
        req.params,
    );
    // §scxml-6.2.4: correlating a reply, or honouring a `<cancel>`, needs
    // the send id — auto-generated here because the fixture declares none.
    assert!(!req.send_id.is_empty(), "the request carried no send id");
}

/// A handler may perform work and have nothing to say. That is not an error,
/// and must not be reported as one — otherwise every fire-and-forget act
/// costs the document a spurious `error.execution`.
#[test]
fn a_handler_that_answers_nothing_is_not_an_error() {
    let (mut engine, script_engine) = started();
    engine.register_event_processor(DECLARED_TYPE, |_req: HostSendRequest| None);
    engine.initialize();
    engine.step();

    assert_eq!(
        counter(&engine, &script_engine, "refused"),
        0,
        "a silent handler was reported as an unsupported processor",
    );
    assert_eq!(
        counter(&engine, &script_engine, "served"),
        0,
        "no reply was sent, so no reply event should have arrived",
    );
}

/// The other half. The build declared the type, so codegen emitted a
/// dispatch — but nothing was registered, so nobody performed the act. From
/// the document's side that is indistinguishable from a processor the
/// platform does not implement, and it gets the same event.
///
/// This is the test that keeps the repair honest: without it the feature
/// could dispatch into an empty registry and the document would proceed as
/// though its act had been carried out.
#[test]
fn a_declared_type_with_no_handler_still_raises_error_execution() {
    let (mut engine, script_engine) = started();
    engine.initialize();
    engine.step();

    assert_eq!(
        counter(&engine, &script_engine, "refused"),
        1,
        "an unregistered processor was silently treated as served",
    );
    assert_eq!(counter(&engine, &script_engine, "served"), 0);
}

/// Registering some other type does not serve this one. The registry is
/// keyed, and a lookup that fell back to "any handler" would deliver a
/// document's acts to a processor it never named.
#[test]
fn a_handler_registered_for_another_type_does_not_serve_this_one() {
    let (mut engine, script_engine) = started();
    engine.register_event_processor("x-some-other-host", |_req: HostSendRequest| {
        Some(HostSendResponse {
            event_name: "turn.done".to_string(),
            event_data: String::new(),
        })
    });
    engine.initialize();
    engine.step();

    assert_eq!(
        counter(&engine, &script_engine, "served"),
        0,
        "a handler for a different type answered this send",
    );
    assert_eq!(counter(&engine, &script_engine, "refused"), 1);
}

/// The query the generated send site uses to tell "ran and said nothing"
/// from "was never wired up". Both return `None` from the dispatch, and only
/// the second is an error, so the distinction cannot come from the return
/// value alone.
#[test]
fn the_registry_reports_what_it_holds() {
    let (mut engine, _script_engine) = started();
    assert!(!engine.has_event_processor(DECLARED_TYPE));
    engine.register_event_processor(DECLARED_TYPE, |_req: HostSendRequest| None);
    assert!(engine.has_event_processor(DECLARED_TYPE));
    assert!(!engine.has_event_processor("x-never-registered"));
}

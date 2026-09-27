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
        vec![HostSendResponse {
            event_name: "turn.done".to_string(),
            event_data: String::new(),
        }]
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

    // W3C SCXML 5.7.1: the second send's unreadable <param> raised, and the
    // message still went — which the second request below confirms.
    assert_eq!(
        counter(&engine, &script_engine, "paramErrors"),
        1,
        "the <param> that cannot be read was not reported",
    );

    let requests = seen.lock().expect("handler log");
    assert_eq!(
        requests.len(),
        2,
        "the handler ran {} times; the served send and the one from `pairs` each run it once",
        requests.len()
    );
    let req = &requests[0];
    // W3C SCXML 5.10: the event data is what a local delivery of this send
    // would carry in `_event.data`, computed by the engine, not rebuilt by
    // the host from `params`. One key, so its spelling is exact.
    assert_eq!(req.event_data, r#"{"within":"2500"}"#);
    let pairs = &requests[1];
    assert_eq!(pairs.event_name, "watch.pairs");
    // The namelist pair reaches the host as a param, the literal one too,
    // and the unreadable one is absent rather than carried empty.
    assert_eq!(
        pairs.params.get("mode").map(Vec::as_slice),
        Some(["pairs".to_string()].as_slice())
    );
    assert_eq!(
        pairs.params.get("kept").map(Vec::as_slice),
        Some(["here".to_string()].as_slice())
    );
    assert!(
        !pairs.params.contains_key("broken"),
        "a <param> that could not be read reached the host: {:?}",
        pairs.params
    );
    // Parsed rather than compared as text: with more than one key, member
    // order is each backend's own and a receiver reads the value, not the
    // spelling.
    let data: serde_json::Value =
        serde_json::from_str(&pairs.event_data).expect("the event data is JSON");
    assert_eq!(data, serde_json::json!({"kept": "here", "mode": "pairs"}));
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
    engine.register_event_processor(DECLARED_TYPE, |_req: HostSendRequest| Vec::new());
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
        vec![HostSendResponse {
            event_name: "turn.done".to_string(),
            event_data: String::new(),
        }]
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
    engine.register_event_processor(DECLARED_TYPE, |_req: HostSendRequest| Vec::new());
    assert!(engine.has_event_processor(DECLARED_TYPE));
    assert!(!engine.has_event_processor("x-never-registered"));
}

/// sce-build/tests/fixtures/host_processor/reserved_type_cases.json: the
/// table the build's declaration check and every runtime's registration
/// read. Each case is asked of the registration itself, not only of the
/// predicate, so a registration that stopped consulting it fails here.
#[test]
fn registration_refuses_the_reserved_types_the_shared_table_names() {
    let table: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../sce-build/tests/fixtures/host_processor/reserved_type_cases.json"
    ))
    .expect("the table is JSON");
    let cases = table["cases"].as_array().expect("the table has cases");
    assert!(cases.len() >= 10, "the table lost cases: {}", cases.len());
    for case in cases {
        let name = case["type"]
            .as_str()
            .expect("a type is a string")
            .to_string();
        let reserved = case["reserved"].as_bool().expect("reserved is a bool");
        assert_eq!(
            sce_rust_runtime::is_reserved_type(&name),
            reserved,
            "{name:?}"
        );
        for invoker in [false, true] {
            let registered = std::panic::catch_unwind(|| {
                let (mut engine, _script_engine) = started();
                if invoker {
                    engine.register_invoker(&name, |_event| None);
                } else {
                    engine.register_event_processor(&name, |_req: HostSendRequest| Vec::new());
                }
            });
            assert_eq!(
                registered.is_err(),
                reserved,
                "{name:?} (invoker: {invoker}) was {} by registration",
                if registered.is_err() {
                    "refused"
                } else {
                    "accepted"
                },
            );
        }
    }
}

/// The Mesh router's own door serves the type the general one refuses.
#[test]
fn a_mesh_router_is_registered_through_its_own_door() {
    let (mut engine, _script_engine) = started();
    assert!(!engine.has_event_processor(sce_rust_runtime::MESH_PROCESSOR_TYPE));
    engine.register_mesh_router(|_req: HostSendRequest| Vec::new());
    assert!(engine.has_event_processor(sce_rust_runtime::MESH_PROCESSOR_TYPE));
}

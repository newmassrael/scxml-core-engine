// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Accepted Subset §2.15, "Saving and restoring": a delayed `<send>` still
// waiting is part of a machine's saved state, and a host-served one — an act
// the HOST performs when it comes due (W3C SCXML 6.2.5) — is saved with every
// field the document wrote, so the handler a restored machine is given sees the
// request it would have seen had the process never died.
//
// The generated `save` / `restore` exist only for a `datamodel="sce-static"`
// document, and a host-served send needs a processor declaration the shared
// static fixtures do not carry. What this measures is the engine-level half,
// which does not depend on the data model: `saved_state::save` and
// `saved_state::enter` over the machine `statechart_delayed_host_send.scxml`
// generates, with a richer request armed on top of what the document arms
// itself, so every field of a request has to cross.
//
// Driven entirely on `SceClock::Manual`, as `delayed_host_send.rs` is: no case
// sleeps, and none can be decided by how loaded the build machine is.
//
// Also here, because a generated machine cannot reach it: a delayed send the
// saved state cannot carry — one routed to a parent — REFUSES the save instead
// of being left out. A document that makes one is generated without the save
// API, so the refusal is for a machine built by hand.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use sce_rust_runtime::saved_state::{self, SavedAct, SavedHostSend, SavedState};
use sce_rust_runtime::{
    Engine, HostSendRequest, HostSendResponse, SceClock, ScheduledRoute, StatePolicy,
};
use sce_rust_tests::integration::host_processor::{
    StatechartDelayedHostSendPolicy as Policy, StatechartDelayedHostSendState as State,
};

const DECLARED_TYPE: &str = "x-sce-host";

/// Any text: the engine-level `save` records the shape it is told, and
/// `check_shape` (which a generated `restore` calls) is not under test here.
const SHAPE: &str = "shape";

/// The wall-clock moment the machine is saved at.
const SAVED_AT_MS: u64 = 1_700_000_000_000;

/// What the document arms on entering `waiting`: a `probe` due at 100 ms and the
/// host-served `watch.turn` due at 200 ms. The request armed on top is due
/// between them and the document's later sends, at 250 ms.
const RICH_DUE_MS: u64 = 250;

/// A request that uses every field a handler can read.
fn rich_request() -> HostSendRequest {
    HostSendRequest {
        processor_type: DECLARED_TYPE.to_string(),
        event_name: "audit".to_string(),
        target: "https://example.test/hook".to_string(),
        content: "the body".to_string(),
        // A repeated name keeps every value in document order.
        params: HashMap::from([
            (
                "kind".to_string(),
                vec!["first".to_string(), "second".to_string()],
            ),
            ("count".to_string(), vec!["3".to_string()]),
        ]),
        send_id: "rich".to_string(),
        event_data: r#"{"kind":["first","second"],"count":["3"]}"#.to_string(),
        invoke_id: "inv".to_string(),
    }
}

/// A machine on host-owned time standing at `waiting`, with `rich_request`
/// armed on top of what the document armed.
fn armed() -> Engine<Policy> {
    let mut engine = Engine::new(Policy::new());
    engine.set_clock(SceClock::Manual(0));
    engine.initialize();
    engine.schedule_host_send(rich_request(), Duration::from_millis(RICH_DUE_MS), "rich");
    engine
}

/// What a restored machine's handler was asked, in call order: the engine's
/// reading of "now" at the moment, and the request.
type CallLog = Arc<Mutex<Vec<(u64, HostSendRequest)>>>;

/// `saved` restored `elapsed_ms` of wall-clock time after it was saved, on a
/// clock of the host's own starting at 0, with a handler that answers the
/// document's `watch.turn` with `turn.done` and records every call.
fn restored(saved: &SavedState, elapsed_ms: u64) -> (Engine<Policy>, CallLog, Arc<Mutex<u64>>) {
    let log: CallLog = Arc::new(Mutex::new(Vec::new()));
    let now = Arc::new(Mutex::new(0_u64));
    let (recorder, mirror) = (Arc::clone(&log), Arc::clone(&now));

    let mut engine = saved_state::enter(
        Policy::new(),
        saved,
        SceClock::Manual(0),
        SAVED_AT_MS + elapsed_ms,
    )
    .unwrap_or_else(|refusal| panic!("the restore was refused: {refusal}"));
    engine.register_event_processor(DECLARED_TYPE, move |request: HostSendRequest| {
        let at = *mirror.lock().expect("clock mirror");
        let answers = request.event_name == "watch.turn";
        recorder.lock().expect("handler log").push((at, request));
        if answers {
            vec![HostSendResponse {
                event_name: "turn.done".to_string(),
                event_data: String::new(),
            }]
        } else {
            Vec::new()
        }
    });
    (engine, log, now)
}

fn advance_to(engine: &mut Engine<Policy>, now: &Arc<Mutex<u64>>, to_ms: u64) {
    let from = *now.lock().expect("clock mirror");
    assert!(to_ms >= from, "time does not run backwards in these cases");
    *now.lock().expect("clock mirror") = to_ms;
    engine.advance_time_ms(to_ms - from);
}

fn saved_through_json(engine: &Engine<Policy>) -> SavedState {
    let saved = saved_state::save(engine, SHAPE, Vec::new(), Vec::new(), SAVED_AT_MS)
        .expect("a running machine saves");
    SavedState::from_json(&saved.to_json()).expect("a saved state reads back")
}

fn calls(log: &CallLog) -> Vec<(u64, String)> {
    log.lock()
        .expect("handler log")
        .iter()
        .map(|(at, request)| (*at, request.event_name.clone()))
        .collect()
}

#[test]
fn a_waiting_host_send_is_saved_with_every_field_of_its_request() {
    let saved = saved_through_json(&armed());

    // Earliest first: the document's own `probe` (an event for this session's
    // external queue) and `watch.turn` (a host-served send), then the request
    // armed on top.
    let due: Vec<u64> = saved.pending.iter().map(|send| send.due).collect();
    assert_eq!(
        due,
        vec![
            SAVED_AT_MS + 100,
            SAVED_AT_MS + 200,
            SAVED_AT_MS + RICH_DUE_MS
        ],
        "{:?}",
        saved.pending
    );
    assert!(
        matches!(&saved.pending[0].act, SavedAct::Raise { event, .. } if event == "probe"),
        "{:?}",
        saved.pending[0]
    );
    assert!(
        matches!(&saved.pending[1].act, SavedAct::Host(host) if host.event == "watch.turn"),
        "{:?}",
        saved.pending[1]
    );
    assert_eq!(
        saved.pending[2].act,
        SavedAct::Host(SavedHostSend {
            processor_type: DECLARED_TYPE.to_string(),
            event: "audit".to_string(),
            target: "https://example.test/hook".to_string(),
            content: "the body".to_string(),
            // By name, whatever order the engine held them in.
            params: vec![
                ("count".to_string(), vec!["3".to_string()]),
                (
                    "kind".to_string(),
                    vec!["first".to_string(), "second".to_string()]
                ),
            ],
            send_id: "rich".to_string(),
            data: r#"{"kind":["first","second"],"count":["3"]}"#.to_string(),
            invoke_id: "inv".to_string(),
        })
    );
}

#[test]
fn a_restored_host_send_is_performed_when_its_moment_comes_with_the_request_it_had() {
    let saved = saved_through_json(&armed());
    // Back 50 ms after the save: `probe` has 50 ms left, `watch.turn` 150 and
    // the request 200.
    let (mut engine, log, now) = restored(&saved, 50);
    assert_eq!(engine.get_current_state(), State::Waiting);

    advance_to(&mut engine, &now, 49);
    assert_eq!(
        engine.get_current_state(),
        State::Waiting,
        "`probe` is not due"
    );
    advance_to(&mut engine, &now, 50);
    assert_eq!(
        engine.get_current_state(),
        State::Armed,
        "`probe`, at 50 ms"
    );
    advance_to(&mut engine, &now, 149);
    assert_eq!(calls(&log), Vec::new(), "no host-served send is due yet");

    advance_to(&mut engine, &now, 150);
    assert_eq!(
        calls(&log),
        vec![(150, "watch.turn".to_string())],
        "the document's own host-served send"
    );
    assert_eq!(
        engine.get_current_state(),
        State::Cancelling,
        "the handler's `turn.done` reached the document"
    );

    advance_to(&mut engine, &now, 199);
    assert_eq!(calls(&log).len(), 1);
    advance_to(&mut engine, &now, 200);
    let log = log.lock().expect("handler log");
    assert_eq!(log.len(), 2, "the armed request, at 200 ms");
    let (at, request) = &log[1];
    assert_eq!(*at, 200);
    let expected = rich_request();
    assert_eq!(request.processor_type, expected.processor_type);
    assert_eq!(request.event_name, expected.event_name);
    assert_eq!(request.target, expected.target);
    assert_eq!(request.content, expected.content);
    assert_eq!(request.params, expected.params);
    assert_eq!(request.send_id, expected.send_id);
    assert_eq!(request.event_data, expected.event_data);
    assert_eq!(request.invoke_id, expected.invoke_id);
}

#[test]
fn a_host_send_already_due_when_the_machine_comes_back_is_performed_in_its_order() {
    let saved = saved_through_json(&armed());
    // A minute away: every wait ran out while the process was dead.
    let (mut engine, log, now) = restored(&saved, 60_000);
    assert_eq!(calls(&log), Vec::new(), "nothing is performed by restoring");

    advance_to(&mut engine, &now, 0);
    assert_eq!(
        calls(&log),
        vec![(0, "watch.turn".to_string()), (0, "audit".to_string())],
        "`probe` first, then the document's host-served send, then the armed \
         request: the order the saved machine would have delivered them in"
    );
}

#[test]
fn a_machine_restored_from_a_text_saves_that_text_again() {
    let saved = saved_through_json(&armed());
    let (engine, _log, _now) = restored(&saved, 0);
    let again = saved_state::save(&engine, SHAPE, Vec::new(), Vec::new(), SAVED_AT_MS)
        .expect("saves again");
    assert_eq!(again.to_json(), saved.to_json());
}

/// What a machine built by hand can hold that a generated one cannot: a delayed
/// send routed to a session the saved state does not carry. Leaving it out would
/// restore a machine that never delivers it and say nothing, so the save is
/// refused and says which send.
#[test]
fn a_delayed_send_to_the_parent_refuses_the_save() {
    let mut engine = armed();
    engine.schedule_routed_event(
        None,
        Duration::from_millis(300),
        "to_parent",
        "",
        "",
        ScheduledRoute::Parent {
            event_name: "report".to_string(),
        },
    );
    let refusal = match saved_state::save(&engine, SHAPE, Vec::new(), Vec::new(), SAVED_AT_MS) {
        Ok(saved) => panic!("the save was expected to be refused: {saved:?}"),
        Err(refusal) => refusal,
    };
    assert!(refusal.reason().contains("to_parent"), "{refusal}");
    assert!(
        refusal.reason().contains("does not carry"),
        "the refusal says why: {refusal}"
    );
}

/// A restore that cannot resolve an event this document does not name refuses
/// before it arms anything.
#[test]
fn a_waiting_send_naming_an_event_the_document_lacks_is_refused() {
    let mut saved = saved_through_json(&armed());
    saved.pending[0].act = SavedAct::Raise {
        event: "warp".to_string(),
        data: String::new(),
        send_id: String::new(),
        origin: String::new(),
    };
    let refusal = match saved_state::enter(Policy::new(), &saved, SceClock::Manual(0), SAVED_AT_MS)
    {
        Ok(_) => panic!("the restore was expected to be refused"),
        Err(refusal) => refusal,
    };
    assert!(refusal.reason().contains("pending[0]"), "{refusal}");
    assert!(refusal.reason().contains("warp"), "{refusal}");
    // `StatePolicy` is what makes `get_event_from_name` the document's lookup.
    assert!(<Policy as StatePolicy>::get_event_from_name("warp").is_none());
}

// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Accepted Subset §2.15, "Saving and restoring": a delayed `<send>` the HOST
// serves is a wait a saved machine carries, with the request it will make when
// the wait ends — measured through the machine a generator wrote.
//
// `a_saved_delayed_host_send_is_performed_after_a_restore.rs` holds the same fact
// at the engine's own save and restore, on a machine built by hand, because no
// shared static fixture used to declare a host processor. This one drives
// `statechart_static_delayed_host_send.scxml`, a `datamodel="sce-static"`
// document, generated with the declaration: the send is armed by the code the
// generator emitted, with its two typed `<param>`s read from the machine's
// fields, and saved and restored by the `Persist` the generator emitted. The
// Kotlin twin is `AGeneratedMachinesWaitingHostSendSurvivesARestoreTest`; the
// text both write and read is `saved/statechart_static_delayed_host_send_waiting.json`.
//
// The run `start`, `bump` changes `job` after the send was armed, so the request
// the wait carries (`job` 7) is told from one evaluated again when the wait ends
// (`job` 8): a restore hands the host the request the document made, and the
// machine itself carries on with the `job` it had.
//
// Driven entirely on `SceClock::Manual`: no case sleeps, and none can be decided
// by how loaded the build machine is.

use std::sync::{Arc, Mutex};

use sce_rust_runtime::saved_state::{SavedAct, SavedState};
use sce_rust_runtime::{Engine, HostSendRequest, SceClock};
use sce_rust_tests::integration::host_processor::{
    StatechartStaticDelayedHostSendPersist as Persist,
    StatechartStaticDelayedHostSendPolicy as Policy, StatechartStaticDelayedHostSendState as State,
};

const DECLARED_TYPE: &str = "x-sce-host";

const SHARED_WAITING: &str = include_str!(
    "../../../../sce-build/tests/fixtures/host_processor/saved/statechart_static_delayed_host_send_waiting.json"
);

/// The wall-clock moment the shared instance was saved at.
const SAVED_AT_MS: u64 = 1_700_000_000_000;

/// How long the document's send waits, and how much of that has gone by when the
/// machine is saved.
const WAIT_MS: u64 = 500;
const WAITED_MS: u64 = 100;

type Calls = Arc<Mutex<Vec<(u64, HostSendRequest)>>>;

/// What a restored machine's handler is asked: the engine's reading of "now" at
/// the moment, and the request.
struct Host {
    calls: Calls,
    now: Arc<Mutex<u64>>,
}

impl Host {
    fn new() -> Self {
        Host {
            calls: Arc::default(),
            now: Arc::default(),
        }
    }

    fn register(&self, engine: &mut Engine<Policy>) {
        let (calls, now) = (Arc::clone(&self.calls), Arc::clone(&self.now));
        engine.register_event_processor(DECLARED_TYPE, move |request: HostSendRequest| {
            let at = *now.lock().expect("clock mirror");
            calls.lock().expect("handler log").push((at, request));
            Vec::new()
        });
    }

    fn advance_to(&self, engine: &mut Engine<Policy>, to_ms: u64) {
        let from = *self.now.lock().expect("clock mirror");
        assert!(to_ms >= from, "time does not run backwards in these cases");
        *self.now.lock().expect("clock mirror") = to_ms;
        engine.advance_time_ms(to_ms - from);
    }

    fn asked(&self) -> Vec<(u64, HostSendRequest)> {
        self.calls.lock().expect("handler log").clone()
    }
}

fn drive(engine: &mut Engine<Policy>, event: &str) {
    engine.raise_external_by_name(event, "");
    engine.tick();
}

/// A machine that armed its send with `job` 7, saw `bump`, and has waited
/// `WAITED_MS` of the `WAIT_MS`.
fn waiting() -> Engine<Policy> {
    let mut engine = Engine::new(Policy::new());
    engine.set_clock(SceClock::Manual(0));
    engine.register_event_processor(DECLARED_TYPE, |_| Vec::new());
    engine.initialize();
    drive(&mut engine, "start");
    drive(&mut engine, "bump");
    engine.advance_time_ms(WAITED_MS);
    assert_eq!(engine.get_current_state(), State::Waiting);
    engine
}

fn restored(text: &str, elapsed_ms: u64) -> (Engine<Policy>, Host) {
    let host = Host::new();
    let mut engine = Engine::<Policy>::restore_with(
        Policy::new(),
        &SavedState::from_json(text).expect("the saved text reads"),
        SceClock::Manual(0),
        SAVED_AT_MS + elapsed_ms,
    )
    .unwrap_or_else(|refusal| panic!("the restore was refused: {refusal}"));
    host.register(&mut engine);
    (engine, host)
}

/// The `job` of a request, from the text the wire carries it as.
fn job_of(request: &HostSendRequest) -> Option<&str> {
    request
        .params
        .get("job")
        .and_then(|values| values.first())
        .map(String::as_str)
}

#[test]
fn a_waiting_send_is_saved_with_the_request_it_was_armed_with() {
    let saved = waiting().save_at(SAVED_AT_MS).expect("saves");

    let [entry] = saved.pending.as_slice() else {
        panic!("one send is waiting: {:?}", saved.pending)
    };
    assert_eq!(entry.due, SAVED_AT_MS + WAIT_MS - WAITED_MS);
    let SavedAct::Host(host) = &entry.act else {
        panic!("a host-served send: {entry:?}")
    };
    assert_eq!(host.processor_type, DECLARED_TYPE);
    assert_eq!(host.event, "audit");
    assert_eq!(host.target, "job://report");
    assert_eq!(host.send_id, "a");
    // Armed with `job` 7: `bump` ran after, and the request does not follow it.
    assert_eq!(
        host.params,
        vec![
            ("job".to_string(), vec!["7".to_string()]),
            ("label".to_string(), vec!["report".to_string()]),
        ]
    );

    // The text the Kotlin suite writes for the same machine, byte for byte. It
    // also says the machine itself carries the `job` it has now (8), apart from
    // the 7 the request was armed with.
    assert_eq!(saved.to_json(), SHARED_WAITING.trim());
}

#[test]
fn a_restored_send_is_made_when_its_moment_comes_with_the_job_it_was_armed_with() {
    // Back 50 ms after the save: the send has 350 ms left of the 400 it had.
    let (mut engine, host) = restored(SHARED_WAITING, 50);
    assert_eq!(engine.get_current_state(), State::Waiting);

    host.advance_to(&mut engine, 349);
    assert!(
        host.asked().is_empty(),
        "nothing is due yet: {:?}",
        host.asked()
    );
    host.advance_to(&mut engine, 350);

    let asked = host.asked();
    let [(at, request)] = asked.as_slice() else {
        panic!("the send is made once: {asked:?}")
    };
    assert_eq!(*at, 350, "at the moment it came due");
    assert_eq!(request.processor_type, DECLARED_TYPE);
    assert_eq!(request.event_name, "audit");
    assert_eq!(request.target, "job://report");
    assert_eq!(request.send_id, "a");
    assert_eq!(
        job_of(request),
        Some("7"),
        "the request the document made, not one evaluated again with the `job` the machine holds now"
    );
    assert_eq!(request.event_data, r#"{"job":7,"label":"report"}"#);
}

#[test]
fn a_send_already_due_when_the_machine_comes_back_is_made_when_it_is_driven() {
    // A minute away: the wait ran out while the process was dead.
    let (mut engine, host) = restored(SHARED_WAITING, 60_000);
    assert!(host.asked().is_empty(), "restoring makes no send");

    host.advance_to(&mut engine, 0);
    assert_eq!(host.asked().len(), 1, "{:?}", host.asked());
    assert_eq!(host.asked()[0].0, 0);
}

#[test]
fn a_restored_send_can_still_be_cancelled_by_its_id() {
    let (mut engine, host) = restored(SHARED_WAITING, 0);
    drive(&mut engine, "cancel");

    host.advance_to(&mut engine, 10_000);
    assert!(
        host.asked().is_empty(),
        "the cancel found the send the saved state carried: {:?}",
        host.asked()
    );
}

#[test]
fn a_restored_machine_carries_on_with_the_job_it_had() {
    // `job` is 8 in the saved state; a `bump` after the restore makes it 9, and a
    // new wait armed from `idle` reads the machine's own field. The restored wait
    // is over (at 400 ms) before the second is armed: a send under an id still
    // waiting is replaced by one backend and left beside it by another, and this
    // case is not about that.
    let (mut engine, host) = restored(SHARED_WAITING, 0);
    host.advance_to(&mut engine, 400);
    drive(&mut engine, "stop");
    drive(&mut engine, "bump");
    drive(&mut engine, "start");
    host.advance_to(&mut engine, 10_000);

    let asked = host.asked();
    let jobs: Vec<Option<&str>> = asked.iter().map(|(_, request)| job_of(request)).collect();
    assert_eq!(
        jobs,
        vec![Some("7"), Some("9")],
        "the restored wait first, then the one armed with the job the restored field held: {asked:?}"
    );
}

#[test]
fn a_saved_state_round_trips_through_its_own_text() {
    let (engine, _) = restored(SHARED_WAITING, 0);
    assert_eq!(
        engine.save_at(SAVED_AT_MS).expect("saves").to_json(),
        SHARED_WAITING.trim()
    );
}

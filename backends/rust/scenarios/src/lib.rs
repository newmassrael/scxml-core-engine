// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! Replay of an engine-neutral scenario against the Rust engine
//! (`datamodel="sce-static"`, docs/SCE_ACCEPTED_SUBSET.md §2.15).
//!
//! A scenario (`sce-build/tests/fixtures/static_datamodel/scenarios/*.json`) is
//! data: a list of steps, each an external event with its payload and what the
//! machine must hold after it runs to quiescence — its current state, the set of
//! its active states (`configuration`), any of its variables, what it asked of
//! its host (`host_calls`), what each child session was given as its host and
//! asked of it (`child_hosts`) and what it sent its Mesh peers (`sent`). What the
//! machine holds is read from its saved state, the text
//! every backend saves byte for byte, so one scenario judges every backend by the
//! same answer and needs no per-type glue.

pub mod child_mesh;
pub mod hosts;
pub mod integration;
pub mod machines;
#[cfg(target_arch = "wasm32")]
pub mod wasm;

use sce_rust_runtime::saved_state::SavedState;
use sce_rust_runtime::{Engine, StatePolicy};
use serde_json::Value;

/// How many rounds a host gives a machine that needs `tick` after an event
/// before it reads the machine back. The engine reports no point at which it has
/// settled, so a host runs rounds: a child session takes an event the parent
/// forwarded in one round and its end reaches the parent in the next, which is
/// two for each level of child. Five is what the other backends' suites give a
/// machine, and a round that finds nothing to do changes nothing.
const SETTLE_TICKS: usize = 5;

/// Give the machine the rounds it needs before it is read back. The generator
/// says which call a machine needs: `step` drains the queues and nothing else,
/// and a machine with a delayed send or a child session is driven by `tick`,
/// which also runs those.
fn settle<P: StatePolicy>(engine: &mut Engine<P>) {
    if P::NEEDS_EVENT_SCHEDULER {
        for _ in 0..SETTLE_TICKS {
            engine.tick();
        }
    } else {
        engine.step();
    }
}

/// Replay `scenario` against `engine`, reading the machine back with
/// `save` after every step. An event name no event of the machine matches
/// fails the replay: the engine drops one silently, which would otherwise
/// pass a misspelt step without running it — unless the step says it expects
/// the drop (`"dropped": true`), which is then what is held. A name matches an
/// event of the machine as the engine delivers it
/// ([`StatePolicy::resolve_event_by_name`], §scxml-3.12.1): the document need
/// not write it.
///
/// A mismatch panics with the step and what differed: a test fails on it, and
/// on a target where a panic stops the module the message is what says why.
pub fn replay<P: StatePolicy>(
    engine: Engine<P>,
    save: impl Fn(&Engine<P>) -> SavedState,
    scenario: &str,
) {
    replay_recording(engine, save, Hosts::none(), scenario);
}

/// [`replay`] for a machine whose `<sce:action>`s go to a host that records them.
///
/// `host_calls` answers what the machine asked of its host since the last time
/// it was asked, oldest first, each as the scenario writes it
/// (`{"action": "showAttempts", "args": [0, false]}`, the arguments in the order
/// the document gives them). A step's `expect.host_calls` holds the calls made in
/// THAT step, the machine's start included in the first, and a step that states
/// none is not judged on them: they are not carried into the next step.
pub fn replay_with_host<P: StatePolicy>(
    engine: Engine<P>,
    save: impl Fn(&Engine<P>) -> SavedState,
    host_calls: impl FnMut(&Engine<P>) -> Vec<Value>,
    scenario: &str,
) {
    replay_recording(engine, save, Hosts::none().host_calls(host_calls), scenario);
}

/// [`replay`] for a machine whose host is read through [`Hosts`]: its own, its
/// children's, or both.
pub fn replay_with_hosts<P: StatePolicy>(
    engine: Engine<P>,
    save: impl Fn(&Engine<P>) -> SavedState,
    hosts: Hosts<'_, P>,
    scenario: &str,
) {
    replay_recording(engine, save, hosts, scenario);
}

/// What a driver offers to read of the hosts a machine acts through, and how it
/// builds the machine again from a saved state. A step that states what a host
/// was asked needs the driver to have a reader for it, and one that restores the
/// machine needs it to restore; a driver with none FAILS that step: a step that
/// judged nothing would pass.
pub struct Hosts<'a, P: StatePolicy> {
    host_calls: Option<HostCallsReader<'a, P>>,
    child_hosts: Option<ChildHostsReader<'a>>,
    restore: Option<Restorer<'a, P>>,
}

/// Reads what the machine asked of its own host since it was last asked.
type HostCallsReader<'a, P> = Box<dyn FnMut(&Engine<P>) -> Vec<Value> + 'a>;

/// Reads the child hosts the machine's host was asked for since it was last asked.
type ChildHostsReader<'a> = Box<dyn FnMut() -> Vec<Value> + 'a>;

/// Builds the machine again from a saved state, as its driver built it first.
type Restorer<'a, P> = Box<dyn FnMut(&SavedState) -> Engine<P> + 'a>;

impl<'a, P: StatePolicy> Hosts<'a, P> {
    /// A machine whose hosts record nothing.
    pub fn none() -> Self {
        Self {
            host_calls: None,
            child_hosts: None,
            restore: None,
        }
    }

    /// The machine a saved state restores (docs/adr/0045): built as the driver
    /// built it first, its host included, and restored from the state. What a
    /// step that `restore`s takes in place of the machine it saved.
    pub fn restorer(mut self, restore: impl FnMut(&SavedState) -> Engine<P> + 'a) -> Self {
        self.restore = Some(Box::new(restore));
        self
    }

    /// What the machine asked of its own host since it was last asked, oldest
    /// first, each as a scenario's `host_calls` writes it.
    pub fn host_calls(mut self, read: impl FnMut(&Engine<P>) -> Vec<Value> + 'a) -> Self {
        self.host_calls = Some(Box::new(read));
        self
    }

    /// The child hosts the machine's host was asked for since it was last asked,
    /// and what the children asked of them, as a scenario's `child_hosts` writes
    /// them ([`hosts::ChildHosts::take`]). A child's host is owned by the child, so
    /// the reader holds a handle of its own to what they recorded.
    pub fn child_hosts(mut self, read: impl FnMut() -> Vec<Value> + 'a) -> Self {
        self.child_hosts = Some(Box::new(read));
        self
    }
}

/// What the machine and its children sent their Mesh peers, oldest first.
type Sent = std::sync::Arc<std::sync::Mutex<Vec<Value>>>;

/// The registrations a replay makes on the machine it drives, as a host makes
/// them on a machine it starts: on the first machine, and again on one a step
/// restored, which a host is handed without them. A machine that is not
/// `routed` is of a host that attached no link (docs/adr/0046, section 4): it is
/// given no router and no factory for its children, so every send to a peer is
/// refused where it is made.
fn register<P: StatePolicy>(
    engine: &mut Engine<P>,
    sent: &Sent,
    children: &child_mesh::Children,
    routed: bool,
) {
    if !routed {
        return;
    }
    // What the machine sends a Mesh peer (`<send target="#name">`) reaches its host
    // through the router the host registers, and a machine with no router has the
    // send refused as error.execution. A replay is every machine's host, so it
    // registers one that only writes the send down: the peer is not there, and what
    // a scenario holds a step to is that the machine asked for the send.
    let log = std::sync::Arc::clone(sent);
    engine.register_mesh_router(move |request| {
        let mut send = serde_json::Map::new();
        send.insert("target".into(), Value::String(request.target));
        send.insert("event".into(), Value::String(request.event_name));
        if !request.event_data.is_empty() {
            send.insert(
                "data".into(),
                serde_json::from_str(&request.event_data).expect("the data of a send is JSON"),
            );
        }
        log.lock()
            .expect("the list of what was sent is never left mid-update")
            .push(Value::Object(send));
        Vec::new()
    });
    // A child session the machine starts is given a router of its own by the
    // factory its host registers (SCE_MESH.md, Child Sessions in Mesh), never
    // the machine's: what a child sends is written into the same list, told by
    // the invoke that started it and the start, and a step's `invoke` hands its
    // event to the child through it.
    engine.register_child_mesh_routers(children.factory());
}

fn replay_recording<P: StatePolicy>(
    mut engine: Engine<P>,
    save: impl Fn(&Engine<P>) -> SavedState,
    mut hosts: Hosts<'_, P>,
    scenario: &str,
) {
    let scenario: Value = serde_json::from_str(scenario).expect("a scenario is JSON");
    let steps = scenario["steps"].as_array().expect("steps");
    assert!(!steps.is_empty(), "a scenario with no steps judges nothing");
    // A scenario that says `"router": false` is of a machine whose host attached no
    // link (docs/adr/0046, section 4).
    let routed = scenario.get("router").and_then(Value::as_bool) != Some(false);
    let sent: Sent = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let children = child_mesh::Children::new(std::sync::Arc::clone(&sent));
    register(&mut engine, &sent, &children, routed);
    engine.initialize();
    // A machine that starts a child session in its first state has that child
    // running, and maybe ended, before any event arrives; it is read as the
    // events leave it, after the rounds that take.
    if P::NEEDS_EVENT_SCHEDULER {
        settle(&mut engine);
    }
    for (n, step) in steps.iter().enumerate() {
        // A step that restores (docs/adr/0045): the machine is saved, discarded and
        // restored from the text before the step's event or time, and the step
        // drives the restored one, made the registrations a host makes.
        if step.get("restore").and_then(Value::as_bool) == Some(true) {
            // What a host recorded is the host's own, and a restored machine is
            // built with a host of its own, which records from nothing: a reader
            // of the first host's record would read the wrong one, so a machine
            // read through its hosts is not restored here, and says so.
            assert!(
                hosts.host_calls.is_none() && hosts.child_hosts.is_none(),
                "step {n}: a machine whose hosts' records a step reads is not restored by this driver"
            );
            let restore = hosts.restore.as_mut().unwrap_or_else(|| {
                panic!("step {n}: this driver cannot restore its machine, so it cannot replay a step that restores")
            });
            let text = save(&engine).to_json();
            let saved = SavedState::from_json(&text)
                .unwrap_or_else(|refusal| panic!("step {n}: the saved text reads back: {refusal}"));
            engine = restore(&saved);
            // The factory is asked for the resumed children's routers as it is
            // registered: those are the sessions that were running, not starts.
            children.resuming(|| register(&mut engine, &sent, &children, routed));
        }
        // A step that moves the machine's time on, for a scenario of a delayed
        // send: the engine is handed a manual clock by its caller, so the wait
        // is the one the step names and not the one the test happened to take.
        if let Some(ms) = step.get("advance_ms").and_then(Value::as_u64) {
            engine.advance_time_ms(ms);
        }
        if let Some(event) = step.get("event").and_then(Value::as_str) {
            let data = step.get("data").map(Value::to_string).unwrap_or_default();
            match step.get("invoke").and_then(Value::as_str) {
                // A peer's event for the running child of that invoke, handed to
                // the router the child was given; its parent applies it when it
                // next drives the child. The child's events are not the
                // machine's, so the machine is not asked whether it matches.
                Some(path) => assert!(
                    children.deliver(path, event, &data),
                    "step {n}: no child of `{path}` is running to be sent `{event}`"
                ),
                None => {
                    let dropped = step.get("dropped").and_then(Value::as_bool) == Some(true);
                    assert_eq!(
                        P::resolve_event_by_name(event).is_none(),
                        dropped,
                        "step {n}: the machine's events {} `{event}`",
                        if dropped { "match" } else { "do not match" }
                    );
                    engine.raise_external_by_name(event, &data);
                }
            }
            settle(&mut engine);
        }
        let expect = &step["expect"];
        let note = step.get("note").and_then(Value::as_str).unwrap_or("");
        // What the machine asked of its host in this step. Read on every step so
        // that a step which states nothing does not hand its calls to the next.
        let asked = hosts.host_calls.as_mut().map(|read| read(&engine));
        match (expect.get("host_calls"), asked) {
            (Some(want), Some(got)) => assert_eq!(
                &Value::Array(got),
                want,
                "step {n} ({note}): what the machine asked of its host"
            ),
            (Some(_), None) => panic!(
                "step {n} ({note}): this driver records no host calls, so it cannot hold a step to them"
            ),
            (None, _) => {}
        }
        // The child hosts the machine's host was asked for in this step, and what
        // each child asked of its host, read and held the same way.
        let children = hosts.child_hosts.as_mut().map(|read| read());
        match (expect.get("child_hosts"), children) {
            (Some(want), Some(got)) => assert_eq!(
                &Value::Array(got),
                want,
                "step {n} ({note}): the child hosts the machine's host was asked for, and what their children asked of them"
            ),
            (Some(_), None) => panic!(
                "step {n} ({note}): this driver records no child hosts, so it cannot hold a step to them"
            ),
            (None, _) => {}
        }
        // What the machine sent its peers in this step, taken on every step for the
        // same reason.
        let sends = std::mem::take(
            &mut *sent
                .lock()
                .expect("the list of what was sent is never left mid-update"),
        );
        if let Some(want) = expect.get("sent") {
            assert_eq!(
                &Value::Array(sends),
                want,
                "step {n} ({note}): what the machine sent its peers"
            );
        }
        // A machine that ended in a top-level <final> has no saved state to
        // read — the save refuses one — so what a scenario can say of it is
        // that it ended, and that is the whole of the step.
        if expect.get("ended").and_then(Value::as_bool) == Some(true) {
            assert!(
                expect.get("state").is_none() && expect.get("variables").is_none(),
                "step {n} ({note}): an ended machine has no state or variables to read"
            );
            assert!(
                engine.is_in_final_state(),
                "step {n} ({note}): the machine ended in a top-level <final>"
            );
            // The data its <donedata> left for the invoking parent, as the
            // JSON the done event carries; compared as a value, so the order
            // the pairs were written in is not part of the answer.
            if let Some(want) = expect.get("donedata") {
                let got: Value = serde_json::from_str(engine.donedata_at_final())
                    .expect("the data of a done event is JSON");
                assert_eq!(
                    &got, want,
                    "step {n} ({note}): the data the final's done event carries"
                );
            }
            continue;
        }
        let saved: Value =
            serde_json::from_str(&save(&engine).to_json()).expect("a saved state is JSON");
        if let Some(state) = expect.get("state") {
            assert_eq!(
                &saved["current"], state,
                "step {n} ({note}): the current state"
            );
        }
        // Every active state, a compound or a parallel one with the atomic ones
        // below it. A set: the order the machine lists them in is not part of the
        // answer.
        if let Some(want) = expect.get("configuration") {
            assert_eq!(
                id_set(&saved["configuration"]),
                id_set(want),
                "step {n} ({note}): the active states"
            );
        }
        if let Some(variables) = expect.get("variables").and_then(Value::as_object) {
            for (name, want) in variables {
                let got = &saved["variables"][name];
                assert!(
                    holds(got, want),
                    "step {n} ({note}): variable `{name}` is {got}, not {want}"
                );
            }
        }
    }
}

/// The state ids of a JSON array, as a set.
fn id_set(ids: &Value) -> std::collections::BTreeSet<&str> {
    ids.as_array()
        .expect("a configuration is an array of state ids")
        .iter()
        .map(|id| id.as_str().expect("a state id is text"))
        .collect()
}

/// Whether a saved value is the scenario's. The saved state writes a 64-bit
/// integer as its decimal text (a JSON number past 2^53 is not exact
/// everywhere), so an expected number matches that text of the same
/// integer; everything else must be equal as written.
fn holds(got: &Value, want: &Value) -> bool {
    match (got, want) {
        (Value::String(text), Value::Number(n)) => {
            (n.is_i64() || n.is_u64()) && *text == n.to_string()
        }
        _ => got == want,
    }
}

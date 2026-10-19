// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! What a machine asked of its host, as a scenario writes it.
//!
//! A generated machine with `<sce:action>`s takes a host that performs them, and
//! the generator writes one that only records them (`Recording<Machine>Actions`).
//! One function per machine here turns what that host recorded into the entries
//! of a scenario's `host_calls` — `{"action": "showAttempts", "args": [0, false]}`,
//! the name the document gives the action and its arguments in the document's
//! order — and hands back only what is new since it was last asked, which is how
//! a step is judged on the calls of that step alone.
//!
//! A child session is given a host of its own by its parent's host, and the
//! child owns it: nothing outside the machine can read what the child asked of it.
//! [`ChildHosts`] is the book every such host writes into, and the one thing a
//! scenario's `child_hosts` is read from. Each machine here wires the factory its
//! generated host takes (a closure, named in the machine by the invoke and, for a
//! candidate, the document it starts) to the host of that child.

use crate::integration::static_datamodel::static_child_host__sce_synth_invoke__worker_sm::StaticChildHostSceSynthInvokeWorkerActions;
use crate::integration::static_datamodel::static_child_host_hybrid_sm::RecordingStaticChildHostHybridActions;
use crate::integration::static_datamodel::static_child_host_sm::RecordingStaticChildHostActions;
use crate::integration::static_datamodel::static_host_call_sm::{
    RecordingStaticHostCallActions, StaticHostCallActionsCall, StaticHostCallPolicy,
};
use crate::integration::static_datamodel::static_host_duty_sm::{
    RecordingStaticHostDutyActions, StaticHostDutyActionsCall, StaticHostDutyPolicy,
};
use crate::integration::static_datamodel::static_hosted_first_sm::StaticHostedFirstActions;
use crate::integration::static_datamodel::static_hosted_second_sm::StaticHostedSecondActions;
use sce_rust_runtime::Engine;
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};

/// The calls `static_host_call` made of its host since `seen` of them were read.
pub fn static_host_call(
    engine: &Engine<StaticHostCallPolicy<RecordingStaticHostCallActions>>,
    seen: &mut usize,
) -> Vec<Value> {
    let calls = engine.policy().actions().calls();
    let fresh = calls[*seen..]
        .iter()
        .map(|call| match call {
            StaticHostCallActionsCall::ShowAttempts { count, exhausted } => {
                json!({"action": "showAttempts", "args": [count, exhausted]})
            }
        })
        .collect();
    *seen = calls.len();
    fresh
}

/// The calls `static_host_duty` made of its host since `seen` of them were read.
pub fn static_host_duty(
    engine: &Engine<StaticHostDutyPolicy<RecordingStaticHostDutyActions>>,
    seen: &mut usize,
) -> Vec<Value> {
    let calls = engine.policy().actions().calls();
    let fresh = calls[*seen..]
        .iter()
        .map(|call| match call {
            StaticHostDutyActionsCall::StartKeepalive => {
                json!({"action": "start_keepalive", "args": []})
            }
            StaticHostDutyActionsCall::NoteEngaged => {
                json!({"action": "note_engaged", "args": []})
            }
            StaticHostDutyActionsCall::StopKeepalive => {
                json!({"action": "stop_keepalive", "args": []})
            }
        })
        .collect();
    *seen = calls.len();
    fresh
}

/// One child host the machine's host was asked for.
struct Child {
    invoke: &'static str,
    start: u64,
    candidate: Option<&'static str>,
    calls: Vec<Value>,
    /// How many of `calls` an earlier step already took.
    taken: usize,
    /// Whether a step already stated that this host was asked for.
    announced: bool,
}

/// The child hosts a machine's host was asked for, in the order asked, and what
/// each child asked of its host. Cloning gives another handle to the same book: a
/// host is moved into the machine, and the scenario reads the book from outside.
#[derive(Clone, Default)]
pub struct ChildHosts {
    book: Arc<Mutex<Vec<Child>>>,
}

impl ChildHosts {
    /// The machine's host was asked for the host of a child its `invoke` started,
    /// and answers it: `candidate` is the stem of the document when the invoke
    /// names one of several. What the child asks of the host is written with
    /// [`ChildLog::call`].
    pub fn open(&self, invoke: &'static str, candidate: Option<&'static str>) -> ChildLog {
        let mut book = self.book.lock().expect("the book is never left mid-update");
        let start = 1 + book.iter().filter(|child| child.invoke == invoke).count() as u64;
        book.push(Child {
            invoke,
            start,
            candidate,
            calls: Vec::new(),
            taken: 0,
            announced: false,
        });
        ChildLog {
            book: Arc::clone(&self.book),
            index: book.len() - 1,
        }
    }

    /// The children asked for or called since this was last called, oldest asked
    /// first, each as a scenario's `child_hosts` writes it: `{"invoke": …,
    /// "start": …, "candidate": …, "host_calls": […]}`, `candidate` left out for a
    /// child the invoke names inline, and `host_calls` the calls made in this
    /// step alone. A child that was asked for earlier and called nothing since has
    /// no entry.
    pub fn take(&self) -> Vec<Value> {
        let mut book = self.book.lock().expect("the book is never left mid-update");
        let mut fresh = Vec::new();
        for child in book.iter_mut() {
            if child.announced && child.taken == child.calls.len() {
                continue;
            }
            let mut entry = serde_json::Map::new();
            entry.insert("invoke".into(), json!(child.invoke));
            entry.insert("start".into(), json!(child.start));
            if let Some(candidate) = child.candidate {
                entry.insert("candidate".into(), json!(candidate));
            }
            entry.insert(
                "host_calls".into(),
                Value::Array(child.calls[child.taken..].to_vec()),
            );
            child.taken = child.calls.len();
            child.announced = true;
            fresh.push(Value::Object(entry));
        }
        fresh
    }
}

/// The handle one child host writes what its child asked of it with.
#[derive(Clone)]
pub struct ChildLog {
    book: Arc<Mutex<Vec<Child>>>,
    index: usize,
}

impl ChildLog {
    /// The child asked its host for `action`, with `args` in the order the
    /// document gives them.
    pub fn call(&self, action: &str, args: Vec<Value>) {
        self.book.lock().expect("the book is never left mid-update")[self.index]
            .calls
            .push(json!({"action": action, "args": args}));
    }
}

/// The host a `worker` child of `static_child_host` acts through.
pub struct WorkerHost(ChildLog);

impl StaticChildHostSceSynthInvokeWorkerActions for WorkerHost {
    fn started(&mut self) {
        self.0.call("started", vec![]);
    }

    fn finished(&mut self, steps: u32) {
        self.0.call("finished", vec![json!(steps)]);
    }
}

/// The host `static_child_host` is given: it answers a [`WorkerHost`] for each
/// start of `worker`, and writes it into `children`.
pub fn static_child_host(children: &ChildHosts) -> RecordingStaticChildHostActions<WorkerHost> {
    let children = children.clone();
    RecordingStaticChildHostActions::new(move || WorkerHost(children.open("worker", None)))
}

/// The host of the first candidate of `static_child_host_hybrid`.
pub struct FirstHost(ChildLog);

impl StaticHostedFirstActions for FirstHost {
    fn first_ran(&mut self) {
        self.0.call("first_ran", vec![]);
    }
}

/// The host of the second candidate of `static_child_host_hybrid`.
pub struct SecondHost(ChildLog);

impl StaticHostedSecondActions for SecondHost {
    fn second_ran(&mut self) {
        self.0.call("second_ran", vec![]);
    }
}

/// The host `static_child_host_hybrid` is given: it answers the host of the
/// candidate it is asked about and of no other, and writes it into `children`
/// under the stem of that candidate's document.
pub fn static_child_host_hybrid(
    children: &ChildHosts,
) -> RecordingStaticChildHostHybridActions<FirstHost, SecondHost> {
    let (for_first, for_second) = (children.clone(), children.clone());
    RecordingStaticChildHostHybridActions::new(
        move || FirstHost(for_first.open("work", Some("static_hosted_first"))),
        move || SecondHost(for_second.open("work", Some("static_hosted_second"))),
    )
}

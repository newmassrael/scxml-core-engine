// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The trace port: what a run tells a test of each step Appendix D takes, so
//! the run can be judged a behaviour of the document's model (docs/adr/0115,
//! D4 and D8).
//!
//! A step is the start of the interpreter, a microstep on eventless
//! transitions, the taking of an internal or an external event (with the
//! microstep it selects, or none), or the exit of the interpreter: the steps
//! `docs/formal/AppendixD.tla` names `Start`, `Eventless`, `Internal`,
//! `External` and `Stop`. Each is told once it is over, with what its record
//! `told` holds there: the transitions it took by their rows, the states it
//! exited in the order it exited them, the states it entered in the order it
//! entered them, and the configuration it left. The macrosteps are the runs of
//! steps from a start or an external event to the next one.
//!
//! What the port is told is written in the one run trace format every lane
//! writes ([`RunTrace::to_json`], `tests/forge/conformance/run_trace.schema.json`,
//! read by `sce-build`'s `run_trace`), and `sce-codegen formal traces` judges
//! the file against the model it names.
//!
//! The port is empty unless a test sets one
//! ([`EngineState::set_trace_port`](crate::EngineState::set_trace_port)), and
//! the whole of it is the `trace-port` feature, which no default asks for: a
//! build without it has no field, no call and no code for it, and with it and
//! no port set each step asks one `Option`. It reads and is never asked
//! anything, so a machine with a port set takes the steps one without takes.
//!
//! An event from outside, and a delayed send that comes due, are steps of the
//! environment and not of the interpreter (AppendixD's `Arrive` and `Due`):
//! the engine cannot tell where an event on its external queue came from, and
//! the external step that takes it names it.

use crate::json::push_escaped;

/// The version of the run trace format [`RunTrace::to_json`] writes, the
/// `TRACE_FORMAT_VERSION` its reader in `sce-build` checks.
pub const TRACE_FORMAT_VERSION: u64 = 1;

/// The lane this runtime is, as a run trace names it.
pub const LANE: &str = "rust";

/// Which step of Appendix D a [`TraceStep`] is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StepKind {
    /// AppendixD's `Start`: the entry into the initial configuration.
    Start,
    /// AppendixD's `Eventless`: a microstep on eventless transitions.
    Eventless,
    /// AppendixD's `Internal`: an event taken off the internal queue, and the
    /// microstep it selects, if it selects one.
    Internal,
    /// AppendixD's `External`: an event taken off the external queue, and the
    /// microstep it selects, if it selects one.
    External,
    /// AppendixD's `Stop`: every state still active exited.
    Stop,
}

impl StepKind {
    /// The step's name in the run trace format.
    pub const fn name(self) -> &'static str {
        match self {
            StepKind::Start => "start",
            StepKind::Eventless => "eventless",
            StepKind::Internal => "internal",
            StepKind::External => "external",
            StepKind::Stop => "stop",
        }
    }
}

/// A transition a step took, by the pair its row of the emitted transition
/// table is named by (docs/adr/0089): the state it is written in and its
/// position among that state's own transitions in document order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Fired {
    /// The state the transition is written in, by its id.
    pub source: &'static str,
    /// Its position among `source`'s own transitions, from 0.
    pub index: u32,
}

/// One step a run took, as the port is told it.
///
/// Each kind says what the run trace format asks of it and no more: a start
/// what it entered and the configuration; a microstep, eventless or on an
/// event, what it took, exited and entered and the configuration; an event
/// step the event too; a stop what it exited and the top-level final state the
/// run ended in. What a kind does not say is empty.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceStep {
    /// Which step.
    pub kind: StepKind,
    /// The event an internal or external step took, by its name.
    pub event: Option<&'static str>,
    /// The transitions the step's microstep took, in the order they were
    /// selected.
    pub fired: Vec<Fired>,
    /// The states exited, in the order they were exited.
    pub exited: Vec<&'static str>,
    /// The states entered, in the order they were entered.
    pub entered: Vec<&'static str>,
    /// The configuration the step left, in document order; empty for a stop.
    pub configuration: Vec<&'static str>,
    /// The top-level `<final>` the run ended in, of a stop that ends one.
    pub outcome: Option<&'static str>,
}

impl TraceStep {
    fn opened(kind: StepKind, event: Option<&'static str>) -> TraceStep {
        TraceStep {
            kind,
            event,
            fired: Vec::new(),
            exited: Vec::new(),
            entered: Vec::new(),
            configuration: Vec::new(),
            outcome: None,
        }
    }

    /// The step as one object of the run trace format: the fields its kind
    /// says, in the order the format lists them.
    pub fn to_json(&self) -> String {
        let mut out = String::new();
        out.push_str("{\"kind\": ");
        push_text(&mut out, self.kind.name());
        match self.kind {
            StepKind::Start => {
                push_names(&mut out, "entered", &self.entered);
                push_names(&mut out, "configuration", &self.configuration);
            }
            StepKind::Eventless | StepKind::Internal | StepKind::External => {
                if self.kind != StepKind::Eventless {
                    out.push_str(", \"event\": ");
                    push_text(&mut out, self.event.unwrap_or_default());
                }
                out.push_str(", \"fired\": [");
                for (at, fired) in self.fired.iter().enumerate() {
                    if at > 0 {
                        out.push_str(", ");
                    }
                    out.push_str("{\"source\": ");
                    push_text(&mut out, fired.source);
                    out.push_str(", \"index\": ");
                    out.push_str(&fired.index.to_string());
                    out.push('}');
                }
                out.push(']');
                push_names(&mut out, "exited", &self.exited);
                push_names(&mut out, "entered", &self.entered);
                push_names(&mut out, "configuration", &self.configuration);
            }
            StepKind::Stop => {
                push_names(&mut out, "exited", &self.exited);
                out.push_str(", \"outcome\": ");
                match self.outcome {
                    Some(outcome) => push_text(&mut out, outcome),
                    None => out.push_str("null"),
                }
            }
        }
        out.push('}');
        out
    }
}

fn push_text(out: &mut String, text: &str) {
    out.push('"');
    push_escaped(out, text);
    out.push('"');
}

fn push_names(out: &mut String, field: &str, names: &[&'static str]) {
    out.push_str(", ");
    push_text(out, field);
    out.push_str(": [");
    for (at, name) in names.iter().enumerate() {
        if at > 0 {
            out.push_str(", ");
        }
        push_text(out, name);
    }
    out.push(']');
}

/// The steps of one run, kept as the port is told them, and written in the
/// run trace format.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RunTrace {
    steps: Vec<TraceStep>,
}

impl RunTrace {
    /// A trace of no step yet.
    pub fn new() -> RunTrace {
        RunTrace::default()
    }

    /// Keep `step`, after the steps kept before it.
    pub fn push(&mut self, step: &TraceStep) {
        self.steps.push(step.clone());
    }

    /// The steps kept, in the order the run took them.
    pub fn steps(&self) -> &[TraceStep] {
        &self.steps
    }

    /// The run in the run trace format: its version, its lane and its steps,
    /// one step to a line.
    pub fn to_json(&self) -> String {
        let mut out = format!(
            "{{\n  \"version\": {TRACE_FORMAT_VERSION},\n  \"lane\": \"{LANE}\",\n  \"steps\": ["
        );
        for (at, step) in self.steps.iter().enumerate() {
            out.push_str(if at == 0 { "\n    " } else { ",\n    " });
            out.push_str(&step.to_json());
        }
        out.push_str(if self.steps.is_empty() {
            "]\n}\n"
        } else {
            "\n  ]\n}\n"
        });
        out
    }
}

/// What the engine holds for a port a test set: any `FnMut + Send` closure.
pub(crate) type Port = Box<dyn FnMut(&TraceStep) + Send>;

/// The port a test set, if any, and the step being told.
///
/// A step is opened where Appendix D begins it and closed where it ends; the
/// states exited and entered between are added to it as the engine exits and
/// enters each. With no port set, nothing is opened, so nothing is kept.
pub(crate) struct Recorder {
    port: Option<Port>,
    step: Option<TraceStep>,
}

impl Recorder {
    /// No port, no step: what every engine starts with.
    pub(crate) const fn new() -> Recorder {
        Recorder {
            port: None,
            step: None,
        }
    }

    /// Tell `port` each step from now on.
    pub(crate) fn set(&mut self, port: Port) {
        self.port = Some(port);
        self.step = None;
    }

    /// Tell no port anything.
    pub(crate) fn clear(&mut self) {
        self.port = None;
        self.step = None;
    }

    /// Whether a step is being told.
    pub(crate) fn is_open(&self) -> bool {
        self.step.is_some()
    }

    /// Begin a step of `kind`, which took `event` when it is an event step.
    pub(crate) fn begin(&mut self, kind: StepKind, event: Option<&'static str>) {
        if self.port.is_some() {
            self.step = Some(TraceStep::opened(kind, event));
        }
    }

    /// The step's microstep takes the transition written `index`-th in
    /// `source`.
    pub(crate) fn fired(&mut self, source: &'static str, index: u32) {
        if let Some(step) = self.step.as_mut() {
            step.fired.push(Fired { source, index });
        }
    }

    /// The step exits `state`.
    pub(crate) fn exited(&mut self, state: &'static str) {
        if let Some(step) = self.step.as_mut() {
            step.exited.push(state);
        }
    }

    /// The step enters `state`.
    pub(crate) fn entered(&mut self, state: &'static str) {
        if let Some(step) = self.step.as_mut() {
            step.entered.push(state);
        }
    }

    /// The step is over: tell the port it, with the configuration it left,
    /// or, for a stop, the top-level `<final>` the run ended in.
    pub(crate) fn end(&mut self, configuration: Vec<&'static str>, outcome: Option<&'static str>) {
        let (Some(port), Some(mut step)) = (self.port.as_mut(), self.step.take()) else {
            return;
        };
        if step.kind == StepKind::Stop {
            step.outcome = outcome;
        } else {
            step.configuration = configuration;
        }
        port(&step);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    fn step(kind: StepKind) -> TraceStep {
        TraceStep {
            kind,
            event: Some("go.on"),
            fired: vec![Fired {
                source: "s0",
                index: 2,
            }],
            exited: vec!["s01", "s0"],
            entered: vec!["s1"],
            configuration: vec!["s1"],
            outcome: Some("pass"),
        }
    }

    #[test]
    fn each_kind_writes_the_fields_the_format_asks_of_it() {
        assert_eq!(
            step(StepKind::Start).to_json(),
            r#"{"kind": "start", "entered": ["s1"], "configuration": ["s1"]}"#
        );
        assert_eq!(
            step(StepKind::Eventless).to_json(),
            r#"{"kind": "eventless", "fired": [{"source": "s0", "index": 2}], "exited": ["s01", "s0"], "entered": ["s1"], "configuration": ["s1"]}"#
        );
        assert_eq!(
            step(StepKind::Internal).to_json(),
            r#"{"kind": "internal", "event": "go.on", "fired": [{"source": "s0", "index": 2}], "exited": ["s01", "s0"], "entered": ["s1"], "configuration": ["s1"]}"#
        );
        assert_eq!(
            step(StepKind::External).to_json(),
            r#"{"kind": "external", "event": "go.on", "fired": [{"source": "s0", "index": 2}], "exited": ["s01", "s0"], "entered": ["s1"], "configuration": ["s1"]}"#
        );
        assert_eq!(
            step(StepKind::Stop).to_json(),
            r#"{"kind": "stop", "exited": ["s01", "s0"], "outcome": "pass"}"#
        );
        let unended = TraceStep {
            outcome: None,
            ..step(StepKind::Stop)
        };
        assert_eq!(
            unended.to_json(),
            r#"{"kind": "stop", "exited": ["s01", "s0"], "outcome": null}"#
        );
    }

    #[test]
    fn a_trace_is_written_with_its_version_and_lane_one_step_to_a_line() {
        let mut trace = RunTrace::new();
        assert_eq!(
            trace.to_json(),
            "{\n  \"version\": 1,\n  \"lane\": \"rust\",\n  \"steps\": []\n}\n"
        );
        trace.push(&step(StepKind::Start));
        trace.push(&step(StepKind::Stop));
        assert_eq!(
            trace.to_json(),
            "{\n  \"version\": 1,\n  \"lane\": \"rust\",\n  \"steps\": [\n    \
             {\"kind\": \"start\", \"entered\": [\"s1\"], \"configuration\": [\"s1\"]},\n    \
             {\"kind\": \"stop\", \"exited\": [\"s01\", \"s0\"], \"outcome\": \"pass\"}\n  ]\n}\n"
        );
        assert_eq!(trace.steps().len(), 2);
    }

    #[test]
    fn a_name_is_written_as_a_json_string() {
        let quoted = TraceStep {
            entered: vec!["a\"b\\c"],
            configuration: Vec::new(),
            ..step(StepKind::Start)
        };
        assert_eq!(
            quoted.to_json(),
            r#"{"kind": "start", "entered": ["a\"b\\c"], "configuration": []}"#
        );
    }

    /// A recorder tells its port each step once the step ends, with what was
    /// added between, and the configuration only of a step that leaves one.
    #[test]
    fn a_recorder_tells_its_port_each_step_once_it_ends() {
        let told: Arc<Mutex<Vec<TraceStep>>> = Arc::default();
        let sink = Arc::clone(&told);
        let mut recorder = Recorder::new();
        recorder.set(Box::new(move |step: &TraceStep| {
            sink.lock().expect("not poisoned").push(step.clone());
        }));

        recorder.begin(StepKind::Internal, Some("e"));
        assert!(recorder.is_open());
        recorder.fired("s0", 1);
        recorder.exited("s0");
        recorder.entered("s1");
        assert!(
            told.lock().expect("not poisoned").is_empty(),
            "told only once it ends"
        );
        recorder.end(vec!["s1"], Some("ignored"));
        assert!(!recorder.is_open());

        recorder.begin(StepKind::Stop, None);
        recorder.exited("s1");
        recorder.end(vec!["ignored"], Some("pass"));

        let told = told.lock().expect("not poisoned").clone();
        assert_eq!(
            told,
            vec![
                TraceStep {
                    kind: StepKind::Internal,
                    event: Some("e"),
                    fired: vec![Fired {
                        source: "s0",
                        index: 1
                    }],
                    exited: vec!["s0"],
                    entered: vec!["s1"],
                    configuration: vec!["s1"],
                    outcome: None,
                },
                TraceStep {
                    kind: StepKind::Stop,
                    event: None,
                    fired: Vec::new(),
                    exited: vec!["s1"],
                    entered: Vec::new(),
                    configuration: Vec::new(),
                    outcome: Some("pass"),
                },
            ]
        );
    }

    /// With no port set nothing is opened, and what the engine adds is kept
    /// nowhere; a cleared port is told nothing more.
    #[test]
    fn a_recorder_with_no_port_keeps_nothing() {
        let mut recorder = Recorder::new();
        recorder.begin(StepKind::Start, None);
        assert!(!recorder.is_open());
        recorder.entered("s0");
        recorder.end(vec!["s0"], None);

        let told: Arc<Mutex<usize>> = Arc::default();
        let sink = Arc::clone(&told);
        recorder.set(Box::new(move |_: &TraceStep| {
            *sink.lock().expect("not poisoned") += 1;
        }));
        recorder.begin(StepKind::Start, None);
        recorder.clear();
        assert!(!recorder.is_open());
        recorder.end(vec!["s0"], None);
        assert_eq!(*told.lock().expect("not poisoned"), 0);
    }
}

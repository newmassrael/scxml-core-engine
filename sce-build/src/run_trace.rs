// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The run trace: the one JSON format in which every lane writes a run of a
//! machine as the steps of Appendix D it took (docs/adr/0115, D4 and D8), so one
//! judge, `sce-codegen formal traces`, can decide whether the run is a
//! behaviour of the document's model.
//!
//! A run is a sequence of steps, each one of the steps `docs/formal/AppendixD.tla`
//! names an action of the interpreter:
//!
//! | Step | AppendixD | What it says |
//! |---|---|---|
//! | `start` | `Start` | `entered`, `configuration` |
//! | `eventless` | `Eventless` | `fired`, `exited`, `entered`, `configuration` |
//! | `internal` | `Internal` | `event`, `fired`, `exited`, `entered`, `configuration` |
//! | `external` | `External` | `event`, `fired`, `exited`, `entered`, `configuration` |
//! | `stop` | `Stop` | `exited`, `outcome` |
//!
//! `fired` is the transitions the step's microstep took, in the order they were
//! selected, each by its source state and its position among that state's own
//! transitions in document order (the row of the emitted transition table,
//! docs/adr/0089); `exited` and `entered` are the states in the order the step
//! exited and entered them; `configuration` is the states active once it was
//! over, a set; `event` is the name of the event an event step took off its
//! queue, the one `_event.name` holds while it is taken: the whole name it was
//! raised or sent under, never a descriptor that matched it, the event of the
//! document a runtime resolved it to, nor the wildcard; `outcome` is the
//! top-level `<final>` a stop ended the run in, or `null`. They are
//! AppendixD's record `told` and the variables
//! `configuration`, `currentEvent` and `outcome` after the step. An event step
//! whose event selects nothing says so with no transition, no state exited and
//! none entered. A macrostep is the steps from a `start` or an `external` step
//! to the next one.
//!
//! Every step says every field its kind has, and no other (D4): a trace that
//! leaves something unsaid may be accepted though the run was wrong, so the
//! reader refuses it rather than let the judge guess. What the format does not
//! say is the environment's: an event from outside and a delayed send that comes
//! due (AppendixD's `Arrive` and `Due`). A runtime cannot tell where an event on
//! its external queue came from; the `external` step that takes one names it,
//! and the judge lets the model's environment have put it there.
//!
//! A run the model made, rather than a lane, is a scenario (docs/adr/0115, D8's
//! converse and D15): the run TLC gives of a violation or a witness, which every
//! lane replays. It says, beside its steps, what its environment did, which a
//! lane must do to replay it: its `inputs`, each event that came from outside
//! (`arrive`, AppendixD's `Arrive`) or a delayed send that came due (`due`,
//! `Due`), with the count of steps taken before it came; and, for a run that
//! ends in a cycle the model takes again and again, the place of the step the
//! `cycle` starts at, which a lane replays once. A lane's trace port writes
//! neither: what came to it is the harness's, and is not the run's to say.
//!
//! Nothing here knows a language or a model: each lane's trace port writes the
//! form ([`RunTrace::to_json`] is the writer of reference), described by
//! `tests/forge/conformance/run_trace.schema.json`, and
//! `sce-codegen formal traces` reads it ([`RunTrace::from_json`]) and judges it.

use serde::{Deserialize, Deserializer, Serialize, Serializer};

pub use crate::history_common::HistoryError as TraceError;

/// The version of the JSON form this module reads and writes. A reader that
/// meets another refuses the file instead of guessing at it.
pub const TRACE_FORMAT_VERSION: u64 = 1;

/// A transition a step took, by the row of the emitted transition table that
/// names it (docs/adr/0089).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Fired {
    /// The id of the state the transition is written in.
    pub source: String,
    /// Its position among `source`'s own transitions in document order, from 0.
    pub index: u32,
}

/// What a microstep did, as a step on eventless transitions or on an event says
/// it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Microstep {
    /// The transitions taken, in the order they were selected.
    pub fired: Vec<Fired>,
    /// The states exited, in the order they were exited.
    pub exited: Vec<String>,
    /// The states entered, in the order they were entered.
    pub entered: Vec<String>,
    /// The states active once the step was over.
    pub configuration: Vec<String>,
}

/// One step of a run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Step {
    /// The entry into the initial configuration.
    Start {
        /// The states entered, in the order they were entered.
        entered: Vec<String>,
        /// The states active once the entry was over.
        configuration: Vec<String>,
    },
    /// A microstep on eventless transitions.
    Eventless(Microstep),
    /// An event taken off the internal queue and what it selected.
    Internal {
        /// The event's name, as `_event.name` holds it.
        event: String,
        /// The microstep it selected, which takes nothing when it selected
        /// nothing.
        microstep: Microstep,
    },
    /// An event taken off the external queue and what it selected.
    External {
        /// The event's name, as `_event.name` holds it.
        event: String,
        /// The microstep it selected, which takes nothing when it selected
        /// nothing.
        microstep: Microstep,
    },
    /// The exit of the interpreter.
    Stop {
        /// The states exited, in the order they were exited.
        exited: Vec<String>,
        /// The top-level `<final>` the run ended in, or none.
        outcome: Option<String>,
    },
}

impl Step {
    /// The step's kind, as the form names it.
    pub fn kind(&self) -> &'static str {
        match self {
            Step::Start { .. } => "start",
            Step::Eventless(_) => "eventless",
            Step::Internal { .. } => "internal",
            Step::External { .. } => "external",
            Step::Stop { .. } => "stop",
        }
    }
}

/// The fields each kind of step says, in the order the form lists them.
pub fn fields_of(kind: &str) -> Option<&'static [&'static str]> {
    match kind {
        "start" => Some(&["entered", "configuration"]),
        "eventless" => Some(&["fired", "exited", "entered", "configuration"]),
        "internal" | "external" => Some(&["event", "fired", "exited", "entered", "configuration"]),
        "stop" => Some(&["exited", "outcome"]),
        _ => None,
    }
}

/// Every kind of step the form has.
pub const KINDS: &[&str] = &["start", "eventless", "internal", "external", "stop"];

/// Where an event that came from the environment came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputKind {
    /// It came from outside (AppendixD's `Arrive`).
    Arrive,
    /// A delayed send the machine made came due (AppendixD's `Due`).
    Due,
}

impl InputKind {
    /// The kind as the form names it.
    pub fn as_str(self) -> &'static str {
        match self {
            InputKind::Arrive => "arrive",
            InputKind::Due => "due",
        }
    }
}

/// One event the environment put on the external queue during a scenario.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Input {
    pub kind: InputKind,
    /// How many steps of the run were over when it came: from 1, since an
    /// event comes once the interpreter has started.
    pub after: usize,
    /// Its name, as `_event.name` holds it once it is taken.
    pub event: String,
}

/// One run of one machine by one lane, or by the model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunTrace {
    /// The lane that ran it: `rust`, `c11`, `cpp`, ..., or `model` for a
    /// scenario.
    pub lane: String,
    /// Its steps, in the order the run took them.
    pub steps: Vec<Step>,
    /// What came from the environment, in the order it came; none in a run a
    /// lane's trace port wrote.
    pub inputs: Vec<Input>,
    /// The place of the step a cycle the run takes again and again starts at:
    /// the steps from there to the end, and the inputs that came after the
    /// step before it, are the cycle, replayed once. `None` for a run that
    /// does not end in one.
    pub cycle: Option<usize>,
}

// ─────────────────────────────── The JSON form ───────────────────────────────

/// The document as it sits in a file. Unknown fields are refused, so a field a
/// writer adds that this reader would drop is an error and not a silent loss of
/// what the writer meant.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TraceDocument {
    version: u64,
    lane: String,
    steps: Vec<StepWire>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    inputs: Vec<InputWire>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    cycle: Option<usize>,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum InputKindWire {
    Arrive,
    Due,
}

/// One input of a scenario.
#[derive(Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
struct InputWire {
    kind: InputKindWire,
    after: usize,
    event: String,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum KindWire {
    Start,
    Eventless,
    Internal,
    External,
    Stop,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
struct FiredWire {
    source: String,
    index: u32,
}

/// One step. Which fields it carries is its kind's; `outcome` is `null` when a
/// stop ended no run in a top-level final state, which is not its absence.
#[derive(Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
struct StepWire {
    kind: KindWire,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    event: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    fired: Option<Vec<FiredWire>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    exited: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    entered: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    configuration: Option<Vec<String>>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    outcome: Option<Option<String>>,
}

/// A field that is present, even as `null`: `Some` of what it holds.
fn present<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(deserializer).map(Some)
}

impl RunTrace {
    /// Read a run from its JSON form.
    ///
    /// Refuses a document that is not JSON, names another version, has a field
    /// this form does not, or a step that does not say every field its kind has
    /// or says one its kind has not; a name that is empty, an event name with an
    /// empty token, a configuration that names a state twice, and a run of no
    /// step, which shows nothing. Of a scenario's inputs it refuses one that
    /// comes before the first step is over or after the last, one out of the
    /// order of the steps, and an event name with an empty token; and a cycle
    /// that starts at no step.
    pub fn from_json(text: &str) -> Result<RunTrace, TraceError> {
        let document: TraceDocument =
            serde_json::from_str(text).map_err(|e| TraceError(format!("not a run trace: {e}")))?;
        Self::from_document(document)
    }

    /// The run a document of the form says, held to every rule
    /// [`RunTrace::from_json`] names.
    fn from_document(document: TraceDocument) -> Result<RunTrace, TraceError> {
        if document.version != TRACE_FORMAT_VERSION {
            return Err(TraceError(format!(
                "version {} is not the version this reader reads ({TRACE_FORMAT_VERSION})",
                document.version
            )));
        }
        if document.lane.is_empty() {
            return Err(TraceError("a run trace names its lane".to_string()));
        }
        if document.steps.is_empty() {
            return Err(TraceError(
                "a run trace of no step shows nothing".to_string(),
            ));
        }
        let steps = document
            .steps
            .into_iter()
            .enumerate()
            .map(|(at, step)| step.observed(at))
            .collect::<Result<Vec<Step>, TraceError>>()?;
        let inputs = inputs_read(document.inputs, steps.len())?;
        if let Some(cycle) = document.cycle.filter(|cycle| *cycle >= steps.len()) {
            return Err(TraceError(format!(
                "`cycle` starts at step {cycle}, and the run has {} steps",
                steps.len()
            )));
        }
        Ok(RunTrace {
            lane: document.lane,
            steps,
            inputs,
            cycle: document.cycle,
        })
    }

    /// Write the run in its JSON form, as [`RunTrace::from_json`] reads it.
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(&self.document()).expect("a run trace serialises")
    }

    /// The run as a document of the form.
    fn document(&self) -> TraceDocument {
        TraceDocument {
            version: TRACE_FORMAT_VERSION,
            lane: self.lane.clone(),
            steps: self.steps.iter().map(StepWire::of).collect(),
            inputs: self
                .inputs
                .iter()
                .map(|input| InputWire {
                    kind: match input.kind {
                        InputKind::Arrive => InputKindWire::Arrive,
                        InputKind::Due => InputKindWire::Due,
                    },
                    after: input.after,
                    event: input.event.clone(),
                })
                .collect(),
            cycle: self.cycle,
        }
    }
}

/// A run is written inside another record, a judgement of properties that
/// holds the scenario of a violation (docs/adr/0115, D15), in its own form.
impl Serialize for RunTrace {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.document().serialize(serializer)
    }
}

/// A run read from inside another record is held to every rule its own file
/// is ([`RunTrace::from_json`]).
impl<'de> Deserialize<'de> for RunTrace {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let document = TraceDocument::deserialize(deserializer)?;
        RunTrace::from_document(document).map_err(serde::de::Error::custom)
    }
}

/// The inputs of a scenario of `steps` steps, each held to the run: it comes
/// once a step is over and before the run is, in the order of the steps.
fn inputs_read(inputs: Vec<InputWire>, steps: usize) -> Result<Vec<Input>, TraceError> {
    let mut read = Vec::with_capacity(inputs.len());
    let mut last = 0usize;
    for (at, input) in inputs.into_iter().enumerate() {
        let refuse = |reason: String| TraceError(format!("inputs[{at}]: {reason}"));
        if input.after == 0 || input.after > steps {
            return Err(refuse(format!(
                "comes after step {}, and an event comes after one of the run's {steps} \
                 steps is over",
                input.after
            )));
        }
        if input.after < last {
            return Err(refuse(format!(
                "comes after step {}, before the input ahead of it, after step {last}",
                input.after
            )));
        }
        last = input.after;
        if input.event.is_empty() || input.event.split('.').any(str::is_empty) {
            return Err(refuse(format!(
                "`event` {:?} is no event name: its tokens are separated by `.`, none empty",
                input.event
            )));
        }
        read.push(Input {
            kind: match input.kind {
                InputKindWire::Arrive => InputKind::Arrive,
                InputKindWire::Due => InputKind::Due,
            },
            after: input.after,
            event: input.event,
        });
    }
    Ok(read)
}

impl StepWire {
    fn of(step: &Step) -> StepWire {
        let empty = StepWire {
            kind: KindWire::Start,
            event: None,
            fired: None,
            exited: None,
            entered: None,
            configuration: None,
            outcome: None,
        };
        let microstep = |kind: KindWire, event: Option<&String>, said: &Microstep| StepWire {
            kind,
            event: event.cloned(),
            fired: Some(
                said.fired
                    .iter()
                    .map(|fired| FiredWire {
                        source: fired.source.clone(),
                        index: fired.index,
                    })
                    .collect(),
            ),
            exited: Some(said.exited.clone()),
            entered: Some(said.entered.clone()),
            configuration: Some(said.configuration.clone()),
            outcome: None,
        };
        match step {
            Step::Start {
                entered,
                configuration,
            } => StepWire {
                entered: Some(entered.clone()),
                configuration: Some(configuration.clone()),
                ..empty
            },
            Step::Eventless(said) => microstep(KindWire::Eventless, None, said),
            Step::Internal {
                event,
                microstep: said,
            } => microstep(KindWire::Internal, Some(event), said),
            Step::External {
                event,
                microstep: said,
            } => microstep(KindWire::External, Some(event), said),
            Step::Stop { exited, outcome } => StepWire {
                kind: KindWire::Stop,
                exited: Some(exited.clone()),
                outcome: Some(outcome.clone()),
                ..empty
            },
        }
    }

    fn kind_name(&self) -> &'static str {
        match self.kind {
            KindWire::Start => "start",
            KindWire::Eventless => "eventless",
            KindWire::Internal => "internal",
            KindWire::External => "external",
            KindWire::Stop => "stop",
        }
    }

    /// The fields this step carries, in the order the form lists them.
    fn carried(&self) -> Vec<&'static str> {
        let mut carried = Vec::new();
        if self.event.is_some() {
            carried.push("event");
        }
        if self.fired.is_some() {
            carried.push("fired");
        }
        if self.exited.is_some() {
            carried.push("exited");
        }
        if self.entered.is_some() {
            carried.push("entered");
        }
        if self.configuration.is_some() {
            carried.push("configuration");
        }
        if self.outcome.is_some() {
            carried.push("outcome");
        }
        carried
    }

    fn observed(self, at: usize) -> Result<Step, TraceError> {
        let kind = self.kind_name();
        let refuse = |reason: String| TraceError(format!("steps[{at}] ({kind}): {reason}"));
        let says = fields_of(kind).expect("every kind of the wire is a kind of the form");
        let carried = self.carried();
        if let Some(missing) = says.iter().find(|field| !carried.contains(*field)) {
            return Err(refuse(format!(
                "`{missing}` is unsaid, and a trace that leaves it unsaid may be accepted \
                 though the run was wrong"
            )));
        }
        if let Some(extra) = carried.iter().find(|field| !says.contains(*field)) {
            return Err(refuse(format!("`{extra}` is no field of this kind")));
        }
        let names = |field: &str, listed: Option<Vec<String>>| -> Result<Vec<String>, TraceError> {
            let listed = listed.unwrap_or_default();
            if listed.iter().any(String::is_empty) {
                return Err(refuse(format!("`{field}` names a state by an empty id")));
            }
            Ok(listed)
        };
        let configuration = |listed: Option<Vec<String>>| -> Result<Vec<String>, TraceError> {
            let listed = names("configuration", listed)?;
            let mut sorted = listed.clone();
            sorted.sort();
            if let Some(twice) = sorted.windows(2).find(|pair| pair[0] == pair[1]) {
                return Err(refuse(format!(
                    "`configuration` is a set and names `{}` twice",
                    twice[0]
                )));
            }
            Ok(listed)
        };
        let event = |name: Option<String>| -> Result<String, TraceError> {
            let name = name.unwrap_or_default();
            if name.is_empty() || name.split('.').any(str::is_empty) {
                return Err(refuse(format!(
                    "`event` {name:?} is no event name: its tokens are separated by `.`, none empty"
                )));
            }
            Ok(name)
        };
        let fired = |fired: Option<Vec<FiredWire>>| -> Result<Vec<Fired>, TraceError> {
            let fired = fired.unwrap_or_default();
            if fired.iter().any(|row| row.source.is_empty()) {
                return Err(refuse(
                    "`fired` names a transition of a state with an empty id".to_string(),
                ));
            }
            Ok(fired
                .into_iter()
                .map(|row| Fired {
                    source: row.source,
                    index: row.index,
                })
                .collect())
        };
        Ok(match self.kind {
            KindWire::Start => Step::Start {
                entered: names("entered", self.entered)?,
                configuration: configuration(self.configuration)?,
            },
            KindWire::Eventless | KindWire::Internal | KindWire::External => {
                let said = Microstep {
                    fired: fired(self.fired)?,
                    exited: names("exited", self.exited)?,
                    entered: names("entered", self.entered)?,
                    configuration: configuration(self.configuration)?,
                };
                match self.kind {
                    KindWire::Internal => Step::Internal {
                        event: event(self.event)?,
                        microstep: said,
                    },
                    KindWire::External => Step::External {
                        event: event(self.event)?,
                        microstep: said,
                    },
                    _ => Step::Eventless(said),
                }
            }
            KindWire::Stop => {
                let outcome = self.outcome.flatten();
                if outcome.as_deref() == Some("") {
                    return Err(refuse("`outcome` names a state by an empty id".to_string()));
                }
                Step::Stop {
                    exited: names("exited", self.exited)?,
                    outcome,
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn trace(steps: &str) -> String {
        format!(r#"{{"version": 1, "lane": "rust", "steps": [{steps}]}}"#)
    }

    fn refused(text: &str) -> String {
        RunTrace::from_json(text)
            .expect_err("the reader refuses this run")
            .to_string()
    }

    const START: &str = r#"{"kind": "start", "entered": ["s0"], "configuration": ["s0"]}"#;

    #[test]
    fn each_kind_reads_with_what_it_says() {
        let read = RunTrace::from_json(&trace(&format!(
            "{START}, \
             {{\"kind\": \"eventless\", \"fired\": [{{\"source\": \"s0\", \"index\": 1}}], \
               \"exited\": [\"s0\"], \"entered\": [\"s1\"], \"configuration\": [\"s1\"]}}, \
             {{\"kind\": \"internal\", \"event\": \"a.b\", \"fired\": [], \"exited\": [], \
               \"entered\": [], \"configuration\": [\"s1\"]}}, \
             {{\"kind\": \"external\", \"event\": \"go\", \"fired\": [], \"exited\": [], \
               \"entered\": [], \"configuration\": [\"s1\"]}}, \
             {{\"kind\": \"stop\", \"exited\": [\"s1\"], \"outcome\": null}}"
        )))
        .expect("every kind reads");
        let kinds: Vec<&str> = read.steps.iter().map(Step::kind).collect();
        assert_eq!(kinds, KINDS);
        assert_eq!(
            read.steps[1],
            Step::Eventless(Microstep {
                fired: vec![Fired {
                    source: "s0".to_string(),
                    index: 1
                }],
                exited: vec!["s0".to_string()],
                entered: vec!["s1".to_string()],
                configuration: vec!["s1".to_string()],
            })
        );
        assert_eq!(
            read.steps[4],
            Step::Stop {
                exited: vec!["s1".to_string()],
                outcome: None
            }
        );
        let again = RunTrace::from_json(&read.to_json()).expect("what is written reads");
        assert_eq!(again, read, "a written run reads back the same");
    }

    #[test]
    fn a_step_says_every_field_its_kind_has_and_no_other() {
        assert!(refused(&trace(r#"{"kind": "start", "entered": ["s0"]}"#))
            .contains("steps[0] (start): `configuration` is unsaid"));
        assert!(refused(&trace(&format!(
            r#"{START}, {{"kind": "stop", "exited": ["s0"]}}"#
        )))
        .contains("steps[1] (stop): `outcome` is unsaid"));
        assert!(refused(&trace(&format!(
            r#"{START}, {{"kind": "internal", "fired": [], "exited": [], "entered": [], "configuration": []}}"#
        )))
        .contains("steps[1] (internal): `event` is unsaid"));
        assert!(refused(&trace(
            r#"{"kind": "start", "event": "e", "entered": ["s0"], "configuration": ["s0"]}"#
        ))
        .contains("steps[0] (start): `event` is no field of this kind"));
        assert!(refused(&trace(&format!(
            r#"{START}, {{"kind": "eventless", "event": "e", "fired": [], "exited": [], "entered": [], "configuration": []}}"#
        )))
        .contains("steps[1] (eventless): `event` is no field of this kind"));
    }

    /// Turns red if a scenario's inputs or its cycle are dropped on the way
    /// through the form, or one that cannot be the run's is read.
    #[test]
    fn a_scenario_says_its_inputs_and_its_cycle() {
        let microstep = r#""fired": [], "exited": [], "entered": [], "configuration": ["s0"]"#;
        let scenario = |inputs: &str, cycle: &str| {
            format!(
                r#"{{"version": 1, "lane": "model", "steps": [{START}, {{"kind": "external", "event": "go", {microstep}}}], "inputs": [{inputs}]{cycle}}}"#
            )
        };
        let read = RunTrace::from_json(&scenario(
            r#"{"kind": "arrive", "after": 1, "event": "go"}, {"kind": "due", "after": 1, "event": "late.one"}"#,
            r#", "cycle": 1"#,
        ))
        .expect("a scenario reads");
        assert_eq!(
            read.inputs,
            vec![
                Input {
                    kind: InputKind::Arrive,
                    after: 1,
                    event: "go".to_string(),
                },
                Input {
                    kind: InputKind::Due,
                    after: 1,
                    event: "late.one".to_string(),
                },
            ]
        );
        assert_eq!(read.cycle, Some(1));
        assert_eq!(
            RunTrace::from_json(&read.to_json()).expect("what is written reads"),
            read
        );
        for (inputs, cycle, refusal) in [
            (
                r#"{"kind": "arrive", "after": 0, "event": "go"}"#,
                "",
                "inputs[0]: comes after step 0",
            ),
            (
                r#"{"kind": "arrive", "after": 3, "event": "go"}"#,
                "",
                "inputs[0]: comes after step 3",
            ),
            (
                r#"{"kind": "arrive", "after": 2, "event": "go"}, {"kind": "arrive", "after": 1, "event": "go"}"#,
                "",
                "inputs[1]: comes after step 1, before the input ahead of it",
            ),
            (
                r#"{"kind": "due", "after": 1, "event": "a..b"}"#,
                "",
                "is no event name",
            ),
            (
                r#"{"kind": "arrive", "after": 1, "event": "go"}"#,
                r#", "cycle": 2"#,
                "`cycle` starts at step 2, and the run has 2 steps",
            ),
        ] {
            let found = refused(&scenario(inputs, cycle));
            assert!(found.contains(refusal), "{found}");
        }
        let lane_run = RunTrace::from_json(&trace(START)).expect("a lane's run reads");
        assert!(lane_run.inputs.is_empty() && lane_run.cycle.is_none());
        assert!(
            !lane_run.to_json().contains("inputs") && !lane_run.to_json().contains("cycle"),
            "a run with no input and no cycle is written as a lane's run is"
        );
    }

    #[test]
    fn the_reader_refuses_what_is_not_the_form() {
        assert!(refused("not json").contains("not a run trace"));
        assert!(
            refused(r#"{"version": 2, "lane": "rust", "steps": []}"#).contains(&format!(
                "is not the version this reader reads ({TRACE_FORMAT_VERSION})"
            ))
        );
        assert!(refused(&trace("")).contains("of no step shows nothing"));
        assert!(refused(r#"{"version": 1, "lane": "", "steps": []}"#).contains("names its lane"));
        assert!(refused(&format!(
            r#"{{"version": 1, "lane": "rust", "steps": [{START}], "note": 1}}"#
        ))
        .contains("not a run trace"));
        assert!(refused(&trace(r#"{"kind": "pause"}"#)).contains("not a run trace"));
        assert!(refused(&trace(
            r#"{"kind": "start", "entered": ["s0"], "configuration": ["s0", "s0"]}"#
        ))
        .contains("names `s0` twice"));
        assert!(refused(&trace(
            r#"{"kind": "start", "entered": [""], "configuration": []}"#
        ))
        .contains("an empty id"));
        assert!(refused(&trace(&format!(
            r#"{START}, {{"kind": "internal", "event": "a..b", "fired": [], "exited": [], "entered": [], "configuration": []}}"#
        )))
        .contains("is no event name"));
        assert!(refused(&trace(&format!(
            r#"{START}, {{"kind": "eventless", "fired": [{{"source": "s0", "index": -1}}], "exited": [], "entered": [], "configuration": []}}"#
        )))
        .contains("not a run trace"));
    }
}

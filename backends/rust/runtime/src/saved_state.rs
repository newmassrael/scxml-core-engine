// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A `datamodel="sce-static"` machine's whole state, saved at a macrostep
//! boundary and restored into a new process (SCE Accepted Subset §2.15, E17).
//!
//! A host whose process can be killed — a phone app, an ECU that reboots —
//! saves the machine and gives it back later. What it saves is everything a
//! macrostep boundary holds that the document cannot recompute: where the
//! machine is (its configuration and current leaf), every variable, the
//! machine's own included — not only the ones a snapshot publishes — and the
//! external queue, in order. Only the internal queue is empty at a macrostep
//! boundary; an event a host raised and has not yet driven the machine
//! through is part of the state.
//!
//! The format is one JSON document (`sce-saved-state`, version [`crate::saved_state::FORMAT`]),
//! the same on every backend, so what one backend saved another can read. A
//! variable is keyed by its document id and written as its `sce:type` says:
//! a number, except a 64-bit integer, written as a text so no reader that
//! holds numbers as doubles loses its low bits; a record as an object of its
//! fields; a list as an array.
//!
//! A saved state is bound to the SHAPE of the document it was saved from — a
//! digest the generator computes of every state with its kind and parent and
//! every variable with its type — not to the document's text. A later build
//! of the same document whose guards, actions or comments changed restores
//! it; one that renamed, re-typed or moved something a saved state names
//! refuses it, since the same id need not mean the same thing there. An app
//! update that touched only a guard does not lose its users' state.
//!
//! The steps every machine takes alike live here
//! ([`crate::saved_state::save`], [`crate::saved_state::check_shape`],
//! [`crate::saved_state::enter`]); a generated machine adds only its own
//! variables between them.
//!
//! ⚠ `std` only: a `no_std` machine has no JSON and nothing to save it to.

#![cfg(not(feature = "no_std"))]

use core::fmt;

use crate::helpers::configuration::ConfigurationRejection;
use crate::json::{self, Value};
use crate::{Engine, EventQueueLike, EventType, EventWithMetadata, StatePolicy};

/// The format version this runtime writes and reads.
pub const FORMAT: u32 = 1;

/// The stability status of the saved-state wire surface, held in lockstep
/// with `x-sce-schema-status` in `schemas/sce-saved-state.v1.schema.json`
/// (`SCE_WIRE_CONTRACTS.md`). A flip to `"stable"` changes both in one commit.
pub const SCHEMA_STATUS: &str = "pre-release";

/// Why a saved state cannot be restored, or a machine cannot be saved.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StateRefusal(String);

impl StateRefusal {
    /// Refuse, saying why.
    pub fn new(reason: impl Into<String>) -> Self {
        Self(reason.into())
    }

    /// Why.
    pub fn reason(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for StateRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A machine's saved state.
#[derive(Clone, Debug, PartialEq)]
pub struct SavedState {
    /// The shape of the document the machine was generated from (module
    /// docs).
    pub shape: String,
    /// The active configuration, as state ids.
    pub configuration: Vec<String>,
    /// The current leaf, as a state id — which of a `<parallel>`'s regions
    /// the machine last stood in, which the configuration alone cannot say.
    pub current: String,
    /// Every variable, by document id, in declaration order.
    pub variables: Vec<(String, Value)>,
    /// What each `<history>` recorded when its parent was last exited
    /// (§scxml-3.10), as state ids in document order, keyed by the history's id
    /// and ordered by it. A history that has recorded nothing is absent: a
    /// resumed machine takes its default transition, as the saved one would
    /// have.
    pub history: Vec<(String, Vec<String>)>,
    /// The external queue, front first: events a host raised and has not yet
    /// driven the machine through. Only the internal queue is empty at a
    /// macrostep boundary, so a state that left these out would lose them.
    pub external: Vec<SavedEvent>,
}

/// One event of a saved external queue: its name and the `_event` fields a
/// document can read (§scxml-5.10.1). A typed payload is not saved — it is
/// lifted again from `data` when the event is delivered, the one path every
/// other delivery takes.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SavedEvent {
    /// The event's name, as the document spells it.
    pub name: String,
    /// `_event.data`, as the wire carries it.
    pub data: String,
    /// `_event.type`: `internal`, `external` or `platform`.
    pub event_type: String,
    /// `_event.sendid`.
    pub send_id: String,
    /// `_event.origin`.
    pub origin: String,
    /// `_event.origintype`.
    pub origin_type: String,
    /// `_event.invokeid`.
    pub invoke_id: String,
}

impl SavedEvent {
    const FIELDS: [&'static str; 7] = [
        "name",
        "data",
        "type",
        "sendid",
        "origin",
        "origintype",
        "invokeid",
    ];

    fn to_value(&self) -> Value {
        let values = [
            &self.name,
            &self.data,
            &self.event_type,
            &self.send_id,
            &self.origin,
            &self.origin_type,
            &self.invoke_id,
        ];
        Value::Object(
            Self::FIELDS
                .iter()
                .zip(values)
                .map(|(k, v)| ((*k).to_string(), Value::Text(v.clone())))
                .collect(),
        )
    }

    fn from_value(value: &Value, what: &str) -> Result<Self, StateRefusal> {
        let text = |key: &str| match value.member(key) {
            Some(Value::Text(s)) => Ok(s.clone()),
            Some(_) => Err(StateRefusal::new(format!("'{what}.{key}' is not a text"))),
            None => Err(StateRefusal::new(format!("'{what}' has no '{key}'"))),
        };
        Ok(Self {
            name: text("name")?,
            data: text("data")?,
            event_type: text("type")?,
            send_id: text("sendid")?,
            origin: text("origin")?,
            origin_type: text("origintype")?,
            invoke_id: text("invokeid")?,
        })
    }
}

impl SavedState {
    /// The variable `id`, or a refusal naming it.
    pub fn variable(&self, id: &str) -> Result<&Value, StateRefusal> {
        self.variables
            .iter()
            .find(|(name, _)| name == id)
            .map(|(_, v)| v)
            .ok_or_else(|| StateRefusal::new(format!("the saved state has no variable '{id}'")))
    }

    /// The state as `sce-saved-state` JSON.
    pub fn to_json(&self) -> String {
        json::write(&Value::Object(vec![
            ("format".to_string(), Value::Number(FORMAT.to_string())),
            ("shape".to_string(), Value::Text(self.shape.clone())),
            (
                "configuration".to_string(),
                Value::Array(
                    self.configuration
                        .iter()
                        .cloned()
                        .map(Value::Text)
                        .collect(),
                ),
            ),
            ("current".to_string(), Value::Text(self.current.clone())),
            (
                "variables".to_string(),
                Value::Object(self.variables.clone()),
            ),
            (
                "history".to_string(),
                Value::Object(
                    self.history
                        .iter()
                        .map(|(id, states)| {
                            (
                                id.clone(),
                                Value::Array(states.iter().cloned().map(Value::Text).collect()),
                            )
                        })
                        .collect(),
                ),
            ),
            (
                "external".to_string(),
                Value::Array(self.external.iter().map(SavedEvent::to_value).collect()),
            ),
        ]))
    }

    /// Read `sce-saved-state` JSON. The shape is not judged here — the
    /// machine that restores it knows its own ([`check_shape`]).
    pub fn from_json(text: &str) -> Result<Self, StateRefusal> {
        let value =
            json::parse(text).map_err(|e| StateRefusal::new(format!("the saved state is {e}")))?;
        let field = |name: &str| {
            value
                .member(name)
                .ok_or_else(|| StateRefusal::new(format!("the saved state has no '{name}'")))
        };
        match field("format")? {
            Value::Number(n) if n == &FORMAT.to_string() => {}
            other => {
                return Err(StateRefusal::new(format!(
                    "the saved state is format {}, and this runtime reads format {FORMAT}",
                    json::write(other)
                )))
            }
        }
        let text_of = |v: &Value, what: &str| match v {
            Value::Text(s) => Ok(s.clone()),
            _ => Err(StateRefusal::new(format!("'{what}' is not a text"))),
        };
        let configuration = match field("configuration")? {
            Value::Array(items) => items
                .iter()
                .map(|v| text_of(v, "configuration"))
                .collect::<Result<Vec<_>, _>>()?,
            _ => return Err(StateRefusal::new("'configuration' is not an array")),
        };
        let variables = match field("variables")? {
            Value::Object(members) => members.clone(),
            _ => return Err(StateRefusal::new("'variables' is not an object")),
        };
        let history = match field("history")? {
            Value::Object(members) => members
                .iter()
                .map(|(id, states)| match states {
                    Value::Array(items) => items
                        .iter()
                        .map(|v| text_of(v, &format!("history.{id}")))
                        .collect::<Result<Vec<_>, _>>()
                        .map(|states| (id.clone(), states)),
                    _ => Err(StateRefusal::new(format!("'history.{id}' is not an array"))),
                })
                .collect::<Result<Vec<_>, _>>()?,
            _ => return Err(StateRefusal::new("'history' is not an object")),
        };
        let external = match field("external")? {
            Value::Array(items) => items
                .iter()
                .enumerate()
                .map(|(i, v)| SavedEvent::from_value(v, &format!("external[{i}]")))
                .collect::<Result<Vec<_>, _>>()?,
            _ => return Err(StateRefusal::new("'external' is not an array")),
        };
        Ok(Self {
            shape: text_of(field("shape")?, "shape")?,
            configuration,
            current: text_of(field("current")?, "current")?,
            variables,
            history,
            external,
        })
    }
}

/// A value a saved state holds: written as its `sce:type` says, and read back
/// only if it is one — a value of another kind, or one its width cannot hold,
/// is refused rather than converted.
pub trait SavedValue: Sized {
    /// The value as saved-state JSON.
    fn to_saved(&self) -> Value;
    /// The value read back from saved-state JSON; `what` names it in a
    /// refusal.
    fn from_saved(value: &Value, what: &str) -> Result<Self, StateRefusal>;
}

fn number<'v>(value: &'v Value, what: &str) -> Result<&'v str, StateRefusal> {
    match value {
        Value::Number(n) => Ok(n),
        _ => Err(StateRefusal::new(format!("'{what}' is not a number"))),
    }
}

macro_rules! saved_narrow_int {
    ($($t:ty),*) => {$(
        impl SavedValue for $t {
            fn to_saved(&self) -> Value {
                Value::Number(self.to_string())
            }
            fn from_saved(value: &Value, what: &str) -> Result<Self, StateRefusal> {
                let n = number(value, what)?;
                n.parse().map_err(|_| {
                    StateRefusal::new(format!(
                        "'{what}' ({n}) is not a whole number its type can hold"
                    ))
                })
            }
        }
    )*};
}
saved_narrow_int!(u8, u16, u32, i8, i16, i32);

// A 64-bit integer is written as a text: a reader that holds JSON numbers as
// doubles would lose its low bits, and a saved state outlives the backend
// that wrote it.
macro_rules! saved_wide_int {
    ($($t:ty),*) => {$(
        impl SavedValue for $t {
            fn to_saved(&self) -> Value {
                Value::Text(self.to_string())
            }
            fn from_saved(value: &Value, what: &str) -> Result<Self, StateRefusal> {
                match value {
                    Value::Text(n) => n.parse().map_err(|_| {
                        StateRefusal::new(format!(
                            "'{what}' ({n}) is not a whole number its type can hold"
                        ))
                    }),
                    _ => Err(StateRefusal::new(format!(
                        "'{what}' is not a 64-bit whole number written as a text"
                    ))),
                }
            }
        }
    )*};
}
saved_wide_int!(u64, i64);

// JSON has no spelling for a value that is not finite, so it is written as
// the text every backend reads it back from.
macro_rules! saved_real {
    ($($t:ty),*) => {$(
        impl SavedValue for $t {
            fn to_saved(&self) -> Value {
                if self.is_nan() {
                    Value::Text("NaN".to_string())
                } else if self.is_infinite() {
                    Value::Text(if *self > 0.0 { "Infinity" } else { "-Infinity" }.to_string())
                } else {
                    Value::Number(format!("{:?}", self))
                }
            }
            fn from_saved(value: &Value, what: &str) -> Result<Self, StateRefusal> {
                match value {
                    Value::Text(t) if t == "NaN" => Ok(<$t>::NAN),
                    Value::Text(t) if t == "Infinity" => Ok(<$t>::INFINITY),
                    Value::Text(t) if t == "-Infinity" => Ok(<$t>::NEG_INFINITY),
                    _ => {
                        let n = number(value, what)?;
                        n.parse().map_err(|_| {
                            StateRefusal::new(format!("'{what}' ({n}) is not a number"))
                        })
                    }
                }
            }
        }
    )*};
}
saved_real!(f32, f64);

impl SavedValue for bool {
    fn to_saved(&self) -> Value {
        Value::Bool(*self)
    }
    fn from_saved(value: &Value, what: &str) -> Result<Self, StateRefusal> {
        match value {
            Value::Bool(b) => Ok(*b),
            _ => Err(StateRefusal::new(format!("'{what}' is not a truth value"))),
        }
    }
}

impl SavedValue for String {
    fn to_saved(&self) -> Value {
        Value::Text(self.clone())
    }
    fn from_saved(value: &Value, what: &str) -> Result<Self, StateRefusal> {
        match value {
            Value::Text(s) => Ok(s.clone()),
            _ => Err(StateRefusal::new(format!("'{what}' is not a text"))),
        }
    }
}

impl<T: SavedValue> SavedValue for Vec<T> {
    fn to_saved(&self) -> Value {
        Value::Array(self.iter().map(SavedValue::to_saved).collect())
    }
    fn from_saved(value: &Value, what: &str) -> Result<Self, StateRefusal> {
        match value {
            Value::Array(items) => items
                .iter()
                .enumerate()
                .map(|(i, item)| T::from_saved(item, &format!("{what}[{i}]")))
                .collect(),
            _ => Err(StateRefusal::new(format!("'{what}' is not an array"))),
        }
    }
}

/// The field `name` of a saved record `value`, read as `T` — what a
/// generated record's [`SavedValue`] reads each field with.
pub fn record_field<T: SavedValue>(
    value: &Value,
    record: &str,
    name: &str,
) -> Result<T, StateRefusal> {
    let field = value
        .member(name)
        .ok_or_else(|| StateRefusal::new(format!("'{record}' has no field '{name}'")))?;
    T::from_saved(field, &format!("{record}.{name}"))
}

/// A saved list or byte string `value`, read back only if it holds no more
/// than the `capacity` the machine bounds it by — a machine never holds more,
/// and a restored one must not be the first to.
pub fn bounded<T: SavedValue>(
    value: &Value,
    what: &str,
    capacity: usize,
) -> Result<Vec<T>, StateRefusal> {
    let items = Vec::<T>::from_saved(value, what)?;
    if items.len() > capacity {
        return Err(StateRefusal::new(format!(
            "'{what}' holds {} elements, past the {capacity} it is bounded by",
            items.len()
        )));
    }
    Ok(items)
}

/// Save `engine`, a machine of the document whose shape is `shape`, with the
/// `variables` its generated code read from its fields. The configuration is
/// written in document order.
///
/// Refused for a machine that is not running — never started, or ended at a
/// top-level `<final>`, where there is nothing left to resume — and for one
/// whose last macrostep stopped at the microstep ceiling: its configuration
/// is not a settled one, and a restore would resume it as if it were.
pub fn save<P: StatePolicy>(
    engine: &Engine<P>,
    shape: &str,
    variables: Vec<(String, Value)>,
    history: Vec<(String, Vec<String>)>,
) -> Result<SavedState, StateRefusal> {
    if !engine.is_running() {
        return Err(StateRefusal::new(
            "the machine is not running: it was never started, or it has ended",
        ));
    }
    if engine.last_macrostep_truncated() {
        return Err(StateRefusal::new(
            "the machine's last macrostep stopped at the microstep ceiling, so it does \
             not stand at a settled configuration",
        ));
    }
    // Document order, so the same machine saves the same text on every
    // backend.
    let mut active = engine.get_active_states();
    active.sort_by_key(|s| P::get_document_order(*s));
    Ok(SavedState {
        shape: shape.to_string(),
        configuration: active
            .iter()
            .map(|s| P::get_state_name(*s).to_string())
            .collect(),
        current: P::get_state_name(engine.get_current_state()).to_string(),
        variables,
        history,
        external: engine
            .external_queue
            .queued()
            .map(|queued| SavedEvent {
                name: P::get_event_name(queued.event).to_string(),
                data: queued.metadata.data.clone(),
                event_type: queued.metadata.event_type.as_str().to_string(),
                send_id: queued.metadata.send_id.clone(),
                origin: queued.metadata.origin.clone(),
                origin_type: queued.metadata.origin_type.clone(),
                invoke_id: queued.metadata.invoke_id.clone(),
            })
            .collect(),
    })
}

/// A `<history>` of the document, as a generated machine declares it: the id
/// a saved state keys it by, the history itself, and whether it is deep.
#[derive(Clone, Copy, Debug)]
pub struct HistoryDecl<H> {
    /// The `<history>`'s id in the document.
    pub id: &'static str,
    /// The history, as the policy names it.
    pub history: H,
    /// Whether it records the atomic states below its parent (§scxml-3.10)
    /// and not only the parent's children.
    pub deep: bool,
}

/// What each of `declared` has recorded in `policy`, as a saved state holds it:
/// state ids in document order — the order one machine writes is every
/// backend's — keyed by the history's id and ordered by it. A history that has
/// recorded nothing is left out.
pub fn save_history<P: StatePolicy>(
    policy: &P,
    declared: &[HistoryDecl<P::History>],
) -> Vec<(String, Vec<String>)> {
    let mut saved: Vec<(String, Vec<String>)> = declared
        .iter()
        .filter_map(|decl| {
            let mut states = policy.history_value(decl.history)?.to_vec();
            states.sort_by_key(|s| P::get_document_order(*s));
            Some((
                decl.id.to_string(),
                states
                    .iter()
                    .map(|s| P::get_state_name(*s).to_string())
                    .collect(),
            ))
        })
        .collect();
    saved.sort_by(|a, b| a.0.cmp(&b.0));
    saved
}

/// What a saved state records for each `<history>` it names, read as the
/// document's own: the history, and the states it recorded.
pub type RestoredHistory<P> = Vec<(<P as StatePolicy>::History, Vec<<P as StatePolicy>::State>)>;

/// What `saved` records for each of `declared`, read as the states of this
/// document, or the refusal that says which value is not one.
///
/// Every value is judged before any is applied, so a state with one bad
/// history restores none of them. A history the document does not declare, one
/// named twice, and a value its history could not have recorded — a state
/// outside its parent, a compound state with two active children
/// ([`validate_history`](crate::helpers::configuration::validate_history)) —
/// are refused: a resumed machine entered through a history must land in a
/// configuration its document could have been in.
pub fn restore_history<P: StatePolicy>(
    saved: &SavedState,
    declared: &[HistoryDecl<P::History>],
) -> Result<RestoredHistory<P>, StateRefusal> {
    use crate::helpers::configuration::validate_history;

    let mut restored: RestoredHistory<P> = Vec::new();
    for (i, (id, names)) in saved.history.iter().enumerate() {
        if saved.history[..i].iter().any(|(earlier, _)| earlier == id) {
            return Err(StateRefusal::new(format!(
                "the saved state records the history '{id}' twice"
            )));
        }
        let decl = declared.iter().find(|d| d.id == id).ok_or_else(|| {
            StateRefusal::new(format!(
                "the saved state records the history '{id}', which the document does not declare"
            ))
        })?;
        let states = names
            .iter()
            .map(|name| {
                P::get_state_from_name(name).ok_or_else(|| {
                    StateRefusal::new(format!(
                        "the history '{id}' records '{name}', which the document does not name"
                    ))
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        validate_history::<P>(P::get_history_parent(decl.history), decl.deep, &states).map_err(
            |rejection| {
                StateRefusal::new(format!(
                    "the history '{id}' records {}",
                    describe_history::<P>(&rejection)
                ))
            },
        )?;
        restored.push((decl.history, states));
    }
    Ok(restored)
}

/// A history refusal in the document's own vocabulary.
fn describe_history<P: StatePolicy>(
    rejection: &crate::helpers::configuration::HistoryRejection<P::State>,
) -> String {
    use crate::helpers::configuration::HistoryRejection;
    let name = |s: P::State| P::get_state_name(s);
    match *rejection {
        HistoryRejection::Empty => "no state, and a history records at least one".to_string(),
        HistoryRejection::Duplicate { state } => format!("'{}' twice", name(state)),
        HistoryRejection::NotBelow { state, parent } => format!(
            "'{}', which is not below its parent '{}'",
            name(state),
            name(parent)
        ),
        HistoryRejection::NotAChild { state, parent } => format!(
            "'{}', which is not a child of '{}' (a shallow history records the children)",
            name(state),
            name(parent)
        ),
        HistoryRejection::NotAtomic { state } => format!(
            "'{}', which has children (a deep history records atomic states)",
            name(state)
        ),
        HistoryRejection::Arity(ref configuration) => {
            format!(
                "states of no configuration: {}",
                describe::<P>(configuration)
            )
        }
    }
}

/// Refuse `saved` unless it was saved from a document of this `shape` —
/// judged before any variable is read, so a state saved from another document
/// is refused for that and not for whichever variable it happens to lack.
pub fn check_shape(saved: &SavedState, shape: &str) -> Result<(), StateRefusal> {
    if saved.shape == shape {
        Ok(())
    } else {
        Err(StateRefusal::new(format!(
            "the saved state is of a document of shape {}, and this machine's is {shape}",
            saved.shape
        )))
    }
}

/// A machine of `policy`, whose variables already hold what `saved` holds,
/// standing at the configuration `saved` names — the machine a host drives in
/// place of one it would [`initialize`](Engine::initialize).
///
/// No `<onentry>` runs and no `<data>` is evaluated ([`Engine::enter_at`]): the
/// run being resumed already did both, and the host calls it caused cannot be
/// made twice. A configuration that is not one of this document is refused,
/// and nothing is entered.
pub fn enter<P: StatePolicy>(policy: P, saved: &SavedState) -> Result<Engine<P>, StateRefusal> {
    let state = |id: &str| {
        P::get_state_from_name(id)
            .ok_or_else(|| StateRefusal::new(format!("the document has no state '{id}'")))
    };
    let configuration = saved
        .configuration
        .iter()
        .map(|id| state(id))
        .collect::<Result<crate::helpers::hierarchy::StateChain<P::State>, _>>()?;
    let current = state(&saved.current)?;
    // Every queued event is read before anything is entered, so one this
    // document cannot name refuses the restore rather than half of the queue.
    let external = saved
        .external
        .iter()
        .enumerate()
        .map(|(i, e)| {
            let event = P::get_event_from_name(&e.name).ok_or_else(|| {
                StateRefusal::new(format!(
                    "external[{i}] is '{}', which the document does not name",
                    e.name
                ))
            })?;
            let event_type = EventType::from_name(&e.event_type).ok_or_else(|| {
                StateRefusal::new(format!(
                    "external[{i}] has the type '{}', which is not an event type",
                    e.event_type
                ))
            })?;
            let mut queued = EventWithMetadata::<P::Event, P::Payload>::new(event);
            queued.metadata.data = e.data.clone();
            queued.metadata.event_type = event_type;
            queued.metadata.send_id = e.send_id.clone();
            queued.metadata.origin = e.origin.clone();
            queued.metadata.origin_type = e.origin_type.clone();
            queued.metadata.invoke_id = e.invoke_id.clone();
            Ok(queued)
        })
        .collect::<Result<Vec<_>, StateRefusal>>()?;
    let mut engine = Engine::new(policy);
    engine
        .enter_at(&configuration, current)
        .map_err(|rejection| StateRefusal::new(describe::<P>(&rejection)))?;
    for queued in external {
        engine.raise_external_with_meta(queued);
    }
    Ok(engine)
}

/// A configuration refusal in the document's own vocabulary, which is the
/// one the saved state names its states in.
fn describe<P: StatePolicy>(rejection: &ConfigurationRejection<P::State>) -> String {
    let name = |s: P::State| P::get_state_name(s);
    match *rejection {
        ConfigurationRejection::Empty => "the saved configuration is empty".to_string(),
        ConfigurationRejection::Duplicate { state } => {
            format!("the saved configuration names '{}' twice", name(state))
        }
        ConfigurationRejection::AncestorMissing { state, parent } => format!(
            "the saved configuration holds '{}' without its parent '{}'",
            name(state),
            name(parent)
        ),
        ConfigurationRejection::RootCount { found } => {
            format!("the saved configuration has {found} roots, and one is a configuration")
        }
        ConfigurationRejection::CompoundChildCount { parent, found } => format!(
            "the saved configuration holds {found} children of '{}', and a compound state \
             has one active",
            name(parent)
        ),
        ConfigurationRejection::ParallelRegionMissing { parallel, region } => format!(
            "the saved configuration holds '{}' without its region '{}'",
            name(parallel),
            name(region)
        ),
        ConfigurationRejection::ParallelChildCount {
            parallel,
            found,
            regions,
        } => format!(
            "the saved configuration holds {found} children of '{}', which has {regions} \
             regions",
            name(parallel)
        ),
        ConfigurationRejection::AtomicHasChildren { state } => format!(
            "the saved configuration gives the atomic state '{}' children",
            name(state)
        ),
        ConfigurationRejection::CurrentNotActive { current } => format!(
            "the saved current state '{}' is not in the saved configuration",
            name(current)
        ),
        ConfigurationRejection::CurrentNotAtomic { current } => {
            format!("the saved current state '{}' is not atomic", name(current))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_saved_state_is_read_back_as_it_was_written() {
        let state = SavedState {
            shape: "abc".to_string(),
            configuration: vec!["counting".to_string()],
            current: "counting".to_string(),
            variables: vec![
                ("count".to_string(), 5u32.to_saved()),
                ("big".to_string(), u64::MAX.to_saved()),
                ("picked".to_string(), vec![3u8, 1].to_saved()),
            ],
            history: vec![
                ("mode".to_string(), vec!["slow".to_string()]),
                ("zone".to_string(), vec!["a".to_string(), "b".to_string()]),
            ],
            external: vec![SavedEvent {
                name: "tick".to_string(),
                data: "{\"n\":1}".to_string(),
                event_type: "external".to_string(),
                ..SavedEvent::default()
            }],
        };
        let back = SavedState::from_json(&state.to_json()).expect("reads");
        assert_eq!(back, state);
        assert_eq!(
            u64::from_saved(back.variable("big").unwrap(), "big"),
            Ok(u64::MAX)
        );
    }

    #[test]
    fn a_value_its_type_cannot_hold_is_refused() {
        assert!(u8::from_saved(&Value::Number("256".to_string()), "level").is_err());
        assert!(u32::from_saved(&Value::Text("5".to_string()), "count").is_err());
        assert!(u64::from_saved(&Value::Number("5".to_string()), "big").is_err());
    }

    #[test]
    fn a_value_that_is_not_finite_is_written_as_a_text_json_can_hold() {
        for x in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let saved = x.to_saved();
            assert!(matches!(saved, Value::Text(_)), "{saved:?}");
            let back = f64::from_saved(&saved, "x").expect("reads back");
            assert!(back == x || (back.is_nan() && x.is_nan()), "{back}");
        }
        assert_eq!(1.5f32.to_saved(), Value::Number("1.5".to_string()));
    }

    #[test]
    fn schema_file_declares_status() {
        let schema = json::parse(include_str!(
            "../../../../schemas/sce-saved-state.v1.schema.json"
        ))
        .expect("the schema is JSON");
        assert_eq!(
            schema.member("x-sce-schema-status"),
            Some(&Value::Text(SCHEMA_STATUS.to_string())),
            "SCHEMA_STATUS and the schema header move together (SCE_WIRE_CONTRACTS.md)"
        );
    }

    #[test]
    fn another_format_is_refused() {
        let text = r#"{"format":2,"shape":"d","configuration":[],"current":"s","variables":{},"history":{},"external":[]}"#;
        assert!(SavedState::from_json(text).is_err());
    }

    /// Every field is always present: a state saved with no history says so
    /// with `{}`, and one that does not say is not this format.
    #[test]
    fn a_saved_state_with_no_history_field_is_refused() {
        let text = r#"{"format":1,"shape":"d","configuration":[],"current":"s","variables":{},"external":[]}"#;
        let refusal = SavedState::from_json(text).expect_err("no history");
        assert!(refusal.reason().contains("'history'"), "{refusal}");
    }

    #[test]
    fn a_history_that_is_not_a_list_of_state_ids_is_refused() {
        let wrong = r#"{"format":1,"shape":"d","configuration":[],"current":"s","variables":{},"history":{"h":"a"},"external":[]}"#;
        let refusal = SavedState::from_json(wrong).expect_err("not an array");
        assert!(refusal.reason().contains("history.h"), "{refusal}");
    }
}

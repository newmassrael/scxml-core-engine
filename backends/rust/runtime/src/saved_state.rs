// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A `datamodel="sce-static"` machine's whole state, saved at a macrostep
//! boundary and restored into a new process (SCE Accepted Subset §2.15, E17).
//!
//! A host whose process can be killed — a phone app, an ECU that reboots —
//! saves the machine and gives it back later. What it saves is everything a
//! macrostep boundary holds that the document cannot recompute: where the
//! machine is (its configuration and current leaf), every variable, the
//! machine's own included — not only the ones a snapshot publishes — what each
//! `<history>` recorded, the delayed `<send>`s still waiting, and the external
//! queue, in order. Only the internal queue is empty at a macrostep
//! boundary; an event a host raised and has not yet driven the machine
//! through is part of the state.
//!
//! A delayed send is saved as the moment it comes due on the host's WALL clock
//! (`due`, milliseconds since the Unix epoch), not as a wait. A wait would
//! start again when the process came back, and a timer that ran out while it
//! was dead would be late by exactly as long as it was dead. The engine's own
//! clock is monotonic and has no epoch, so the host says what time it is on
//! the wall when it saves and when it restores
//! ([`crate::saved_state::save`], [`crate::saved_state::enter`]); an entry
//! already due when the machine comes back is armed as due now, in the order
//! it would have fired.
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
use crate::{Engine, EventQueueLike, EventType, EventWithMetadata, SceClock, StatePolicy};

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
    /// The delayed `<send>`s still waiting (§scxml-6.2), in the order they
    /// would be delivered: earliest first, entries due at the same moment in
    /// the order they were sent.
    pub pending: Vec<SavedSend>,
    /// The `<invoke>`s whose child session is running (§scxml-6.4), by the id
    /// the document gives each, in document order. A child is not saved: a
    /// restored machine starts each of these again from its beginning, under
    /// the same id. One whose child has ended is absent — its `done.invoke` is
    /// in the external queue, or already taken — so it is not started twice.
    pub invokes: Vec<String>,
    /// The `<invoke>`s a declared host invoker is running (§scxml-6.4.1), by
    /// `(type, id)`. A restored machine starts each again from the request it
    /// was started with, with the deadline it had left; the host is told it is a
    /// restart ([`crate::HostInvokeRequest::restarted`]).
    pub host_invokes: Vec<SavedHostInvoke>,
    /// The token the next host-run start receives. Carried on, so that a start
    /// a restored machine makes is never given a token an earlier run already
    /// handed to a host that may still answer with it.
    pub host_invoke_token: u64,
    /// How many ids the machine has generated for a `<send idlocation>`: the
    /// number the last one carried. Carried on, so that an id a restored machine
    /// generates is never one an earlier run already handed to the document,
    /// which may still hold it in a variable.
    pub auto_send_seq: u64,
    /// The external queue, front first: events a host raised and has not yet
    /// driven the machine through. Only the internal queue is empty at a
    /// macrostep boundary, so a state that left these out would lose them.
    pub external: Vec<SavedEvent>,
}

/// One delayed `<send>` that has not been delivered yet.
#[derive(Clone, Debug, PartialEq)]
pub struct SavedSend {
    /// When it comes due, in milliseconds since the Unix epoch on the wall
    /// clock of the host that saved it (module docs).
    pub due: u64,
    /// What it does when it comes due.
    pub act: SavedAct,
}

/// What a waiting `<send>` does when it comes due. The three the delayed sends
/// of a document that saves can be: its own event on this session's external
/// queue or on its internal queue (`#_internal`), or an act a host-served
/// processor performs (§scxml-6.2.5).
///
/// A delayed send to a parent, a child or another session is none of them, and
/// a document that makes one is generated without the save API (§2.15): such a
/// send is delivered through a session this state does not carry.
#[derive(Clone, Debug, PartialEq)]
pub enum SavedAct {
    /// An event for this session's external queue.
    Raise {
        /// The event's name, as the document spells it.
        event: String,
        /// `_event.data`, as the wire carries it.
        data: String,
        /// The `<send>`'s id, which `_event.sendid` carries and a `<cancel>`
        /// names.
        send_id: String,
        /// `_event.origin`: the session that sent it.
        origin: String,
    },
    /// An event for this session's internal queue.
    Internal {
        /// The event's name, as the document spells it.
        event: String,
        /// `_event.data`, as the wire carries it.
        data: String,
        /// The `<send>`'s id, which a `<cancel>` names.
        send_id: String,
        /// `_event.origin`: the session that sent it. Empty on a backend whose
        /// internal events carry none.
        origin: String,
    },
    /// A `<send>` a host-served processor performs.
    Host(SavedHostSend),
}

/// A delayed `<send>` addressed to a host-served processor, as it is performed
/// when it comes due (§scxml-6.2.5): every field is what the document wrote,
/// so a handler sees the request it would have seen had there been no delay.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SavedHostSend {
    /// The `type` the send named.
    pub processor_type: String,
    /// `<send event>`.
    pub event: String,
    /// `<send target>`, empty when the document named none.
    pub target: String,
    /// Inline `<content>`, empty when the document carried none.
    pub content: String,
    /// `<param>` values by name, ordered by name; a repeated name keeps every
    /// value in document order.
    pub params: Vec<(String, Vec<String>)>,
    /// The send's id.
    pub send_id: String,
    /// The event's `_event.data` as a local delivery would carry it.
    pub data: String,
    /// `_event.invokeid` of the event being processed when the `<send>`
    /// executed.
    pub invoke_id: String,
}

/// An `<invoke>` a declared host invoker is running, as it is started again
/// (§scxml-6.4.1): every field is what the request the host was handed carried,
/// so the restarted invocation reads as the one the document began.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SavedHostInvoke {
    /// The `type` the `<invoke>` named.
    pub processor_type: String,
    /// The invoke's id, which `done.invoke.<id>` names.
    pub invoke_id: String,
    /// `<invoke src>`, empty when the document named none.
    pub src: String,
    /// `<param>` values by name, ordered by name; a repeated name keeps every
    /// value in document order. Without the engine's own deadline parameter,
    /// which `due` holds.
    pub params: Vec<(String, Vec<String>)>,
    /// The namelist and `<param>` pairs as JSON, as the request carried them.
    pub data: String,
    /// Inline `<content>`, empty when the document carried none.
    pub content: String,
    /// When its deadline comes due, in milliseconds since the Unix epoch on the
    /// wall clock of the host that saved it; `None` for an invocation that has
    /// none. Already past for one that came due while the machine waited to be
    /// restarted.
    pub due: Option<u64>,
}

/// `params` as a saved state writes them: an object of arrays of texts, in the
/// order given.
fn params_to_value(params: &[(String, Vec<String>)]) -> Value {
    Value::Object(
        params
            .iter()
            .map(|(name, values)| {
                (
                    name.clone(),
                    Value::Array(values.iter().cloned().map(Value::Text).collect()),
                )
            })
            .collect(),
    )
}

/// The member `params` of `value`, read as an object of arrays of texts; `what`
/// names `value` in a refusal.
fn read_params(value: &Value, what: &str) -> Result<Vec<(String, Vec<String>)>, StateRefusal> {
    match value.member("params") {
        Some(Value::Object(members)) => members
            .iter()
            .map(|(name, values)| match values {
                Value::Array(items) => items
                    .iter()
                    .map(|v| match v {
                        Value::Text(s) => Ok(s.clone()),
                        _ => Err(StateRefusal::new(format!(
                            "'{what}.params.{name}' holds a value that is not a text"
                        ))),
                    })
                    .collect::<Result<Vec<_>, _>>()
                    .map(|values| (name.clone(), values)),
                _ => Err(StateRefusal::new(format!(
                    "'{what}.params.{name}' is not an array"
                ))),
            })
            .collect(),
        Some(_) => Err(StateRefusal::new(format!(
            "'{what}.params' is not an object"
        ))),
        None => Err(StateRefusal::new(format!("'{what}' has no 'params'"))),
    }
}

/// A moment, as the text a saved state writes it as — milliseconds since the
/// Unix epoch — which `what` (the member, named in a refusal) holds.
///
/// A text of digits, as every 64-bit integer is, and one a signed 64-bit reader
/// holds too: the format is the same on every backend.
fn read_moment(written: &str, what: &str) -> Result<u64, StateRefusal> {
    written
        .parse::<u64>()
        .ok()
        .filter(|d| {
            // `parse` also reads a leading `+`, which the other backends refuse,
            // and which no backend writes.
            *d <= i64::MAX as u64 && written.bytes().all(|b| b.is_ascii_digit())
        })
        .ok_or_else(|| {
            StateRefusal::new(format!(
                "'{what}' ({written}) is not a whole number of milliseconds"
            ))
        })
}

/// A host-run start's token, as the text a saved state writes it as: digits,
/// within a signed 64-bit count, as every backend reads one.
fn read_token(written: &str, what: &str) -> Result<u64, StateRefusal> {
    written
        .parse::<u64>()
        .ok()
        .filter(|t| *t <= i64::MAX as u64 && written.bytes().all(|b| b.is_ascii_digit()))
        .ok_or_else(|| {
            StateRefusal::new(format!(
                "'{what}' ({written}) is not a whole number a token can be"
            ))
        })
}

impl SavedHostInvoke {
    fn to_value(&self) -> Value {
        let text = |s: &str| Value::Text(s.to_string());
        Value::Object(vec![
            ("type".to_string(), text(&self.processor_type)),
            ("id".to_string(), text(&self.invoke_id)),
            ("src".to_string(), text(&self.src)),
            ("params".to_string(), params_to_value(&self.params)),
            ("data".to_string(), text(&self.data)),
            ("content".to_string(), text(&self.content)),
            (
                "due".to_string(),
                self.due
                    .map_or(Value::Null, |due| Value::Text(due.to_string())),
            ),
        ])
    }

    fn from_value(value: &Value, what: &str) -> Result<Self, StateRefusal> {
        let text = |key: &str| match value.member(key) {
            Some(Value::Text(s)) => Ok(s.clone()),
            Some(_) => Err(StateRefusal::new(format!("'{what}.{key}' is not a text"))),
            None => Err(StateRefusal::new(format!("'{what}' has no '{key}'"))),
        };
        let due = match value.member("due") {
            Some(Value::Null) => None,
            Some(Value::Text(written)) => Some(read_moment(written, &format!("{what}.due"))?),
            Some(_) => {
                return Err(StateRefusal::new(format!(
                    "'{what}.due' is neither a text nor null"
                )))
            }
            None => return Err(StateRefusal::new(format!("'{what}' has no 'due'"))),
        };
        Ok(Self {
            processor_type: text("type")?,
            invoke_id: text("id")?,
            src: text("src")?,
            params: read_params(value, what)?,
            data: text("data")?,
            content: text("content")?,
            due,
        })
    }
}

impl SavedSend {
    fn to_value(&self) -> Value {
        let text = |s: &str| Value::Text(s.to_string());
        let mut members = vec![
            ("due".to_string(), Value::Text(self.due.to_string())),
            (
                "act".to_string(),
                text(match self.act {
                    SavedAct::Raise { .. } => "raise",
                    SavedAct::Internal { .. } => "internal",
                    SavedAct::Host(_) => "host",
                }),
            ),
        ];
        match &self.act {
            SavedAct::Raise {
                event,
                data,
                send_id,
                origin,
            } => members.extend([
                ("event".to_string(), text(event)),
                ("data".to_string(), text(data)),
                ("sendid".to_string(), text(send_id)),
                ("origin".to_string(), text(origin)),
            ]),
            SavedAct::Internal {
                event,
                data,
                send_id,
                origin,
            } => members.extend([
                ("event".to_string(), text(event)),
                ("data".to_string(), text(data)),
                ("sendid".to_string(), text(send_id)),
                ("origin".to_string(), text(origin)),
            ]),
            SavedAct::Host(host) => members.extend([
                ("type".to_string(), text(&host.processor_type)),
                ("event".to_string(), text(&host.event)),
                ("target".to_string(), text(&host.target)),
                ("content".to_string(), text(&host.content)),
                ("params".to_string(), params_to_value(&host.params)),
                ("sendid".to_string(), text(&host.send_id)),
                ("data".to_string(), text(&host.data)),
                ("invokeid".to_string(), text(&host.invoke_id)),
            ]),
        }
        Value::Object(members)
    }

    fn from_value(value: &Value, what: &str) -> Result<Self, StateRefusal> {
        let text = |key: &str| match value.member(key) {
            Some(Value::Text(s)) => Ok(s.clone()),
            Some(_) => Err(StateRefusal::new(format!("'{what}.{key}' is not a text"))),
            None => Err(StateRefusal::new(format!("'{what}' has no '{key}'"))),
        };
        let due = read_moment(&text("due")?, &format!("{what}.due"))?;
        let act = match text("act")?.as_str() {
            "raise" => SavedAct::Raise {
                event: text("event")?,
                data: text("data")?,
                send_id: text("sendid")?,
                origin: text("origin")?,
            },
            "internal" => SavedAct::Internal {
                event: text("event")?,
                data: text("data")?,
                send_id: text("sendid")?,
                origin: text("origin")?,
            },
            "host" => SavedAct::Host(SavedHostSend {
                processor_type: text("type")?,
                event: text("event")?,
                target: text("target")?,
                content: text("content")?,
                params: read_params(value, what)?,
                send_id: text("sendid")?,
                data: text("data")?,
                invoke_id: text("invokeid")?,
            }),
            other => {
                return Err(StateRefusal::new(format!(
                    "'{what}.act' is '{other}', which is not raise, internal or host"
                )))
            }
        };
        Ok(Self { due, act })
    }
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
    /// The token a host-run invocation's completion or failure was stamped
    /// with when `Engine::complete_host_invoke` accepted it, `None` for any
    /// other event.
    ///
    /// The engine refuses a host `done.invoke` that carries none when it is
    /// dequeued, because it may be a cancelled run's late reply. A completion
    /// that was accepted and is still queued has to keep the stamp across a
    /// save, or the restored machine would refuse the one answer it was
    /// waiting for. Written as it was, not as a guess from the event's name:
    /// an event a host raised through the ordinary door and the engine had yet
    /// to refuse is one the restored machine must still refuse.
    pub host_invoke_token: Option<u64>,
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
        let mut members: Vec<(String, Value)> = Self::FIELDS
            .iter()
            .zip(values)
            .map(|(k, v)| ((*k).to_string(), Value::Text(v.clone())))
            .collect();
        members.push((
            "hostinvoketoken".to_string(),
            self.host_invoke_token
                .map_or(Value::Null, |token| Value::Text(token.to_string())),
        ));
        Value::Object(members)
    }

    fn from_value(value: &Value, what: &str) -> Result<Self, StateRefusal> {
        let text = |key: &str| match value.member(key) {
            Some(Value::Text(s)) => Ok(s.clone()),
            Some(_) => Err(StateRefusal::new(format!("'{what}.{key}' is not a text"))),
            None => Err(StateRefusal::new(format!("'{what}' has no '{key}'"))),
        };
        let host_invoke_token = match value.member("hostinvoketoken") {
            Some(Value::Null) => None,
            Some(Value::Text(written)) => {
                Some(read_token(written, &format!("{what}.hostinvoketoken"))?)
            }
            Some(_) => {
                return Err(StateRefusal::new(format!(
                    "'{what}.hostinvoketoken' is neither a text nor null"
                )))
            }
            None => {
                return Err(StateRefusal::new(format!(
                    "'{what}' has no 'hostinvoketoken'"
                )))
            }
        };
        Ok(Self {
            name: text("name")?,
            data: text("data")?,
            event_type: text("type")?,
            send_id: text("sendid")?,
            origin: text("origin")?,
            origin_type: text("origintype")?,
            invoke_id: text("invokeid")?,
            host_invoke_token,
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
                "pending".to_string(),
                Value::Array(self.pending.iter().map(SavedSend::to_value).collect()),
            ),
            (
                "invokes".to_string(),
                Value::Array(self.invokes.iter().cloned().map(Value::Text).collect()),
            ),
            (
                "hostinvokes".to_string(),
                Value::Array(
                    self.host_invokes
                        .iter()
                        .map(SavedHostInvoke::to_value)
                        .collect(),
                ),
            ),
            (
                "hostinvoketoken".to_string(),
                Value::Text(self.host_invoke_token.to_string()),
            ),
            (
                "sendseq".to_string(),
                Value::Text(self.auto_send_seq.to_string()),
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
        let pending = match field("pending")? {
            Value::Array(items) => items
                .iter()
                .enumerate()
                .map(|(i, v)| SavedSend::from_value(v, &format!("pending[{i}]")))
                .collect::<Result<Vec<_>, _>>()?,
            _ => return Err(StateRefusal::new("'pending' is not an array")),
        };
        let invokes = match field("invokes")? {
            Value::Array(items) => items
                .iter()
                .map(|v| text_of(v, "invokes"))
                .collect::<Result<Vec<_>, _>>()?,
            _ => return Err(StateRefusal::new("'invokes' is not an array")),
        };
        let host_invokes = match field("hostinvokes")? {
            Value::Array(items) => items
                .iter()
                .enumerate()
                .map(|(i, v)| SavedHostInvoke::from_value(v, &format!("hostinvokes[{i}]")))
                .collect::<Result<Vec<_>, _>>()?,
            _ => return Err(StateRefusal::new("'hostinvokes' is not an array")),
        };
        let host_invoke_token = match field("hostinvoketoken")? {
            Value::Text(written) => read_token(written, "hostinvoketoken")?,
            _ => return Err(StateRefusal::new("'hostinvoketoken' is not a text")),
        };
        let auto_send_seq = match field("sendseq")? {
            Value::Text(written) => read_token(written, "sendseq")?,
            _ => return Err(StateRefusal::new("'sendseq' is not a text")),
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
            pending,
            invokes,
            host_invokes,
            host_invoke_token,
            auto_send_seq,
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
//
// A 32-bit real is written as the 64-bit real it widens to, which is exact: the
// number a reader that holds JSON numbers as doubles finds is the number the
// machine holds, `0.30000001192092896` and not the shorter `0.3` a single would
// print for it, which is another number to that reader. Every engine writes it
// so, because the widening is the one spelling they can all make.
macro_rules! saved_real {
    ($($t:ty => $wide:ty),*) => {$(
        impl SavedValue for $t {
            fn to_saved(&self) -> Value {
                if self.is_nan() {
                    Value::Text("NaN".to_string())
                } else if self.is_infinite() {
                    Value::Text(if *self > 0.0 { "Infinity" } else { "-Infinity" }.to_string())
                } else {
                    Value::Number(format!("{:?}", <$wide>::from(*self)))
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
saved_real!(f32 => f64, f64 => f64);

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
    T::from_saved(member_of(value, record, name)?, &format!("{record}.{name}"))
}

/// The string field `name` of a saved record `value`, read back only if it holds
/// no more than the `capacity` UTF-8 bytes its schema bounds it by
/// ([`bounded_string`]) — a record never holds more, and a restored one must not
/// be the first to.
pub fn record_bounded_string(
    value: &Value,
    record: &str,
    name: &str,
    capacity: usize,
) -> Result<String, StateRefusal> {
    bounded_string(
        member_of(value, record, name)?,
        &format!("{record}.{name}"),
        capacity,
    )
}

/// The field `name` of the saved record `value`, which a record that lacks it is
/// refused for naming.
fn member_of<'v>(value: &'v Value, record: &str, name: &str) -> Result<&'v Value, StateRefusal> {
    value
        .member(name)
        .ok_or_else(|| StateRefusal::new(format!("'{record}' has no field '{name}'")))
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

/// A saved string `value`, read back only if it holds no more than the
/// `capacity` UTF-8 bytes the machine bounds it by — a machine never holds more,
/// and a restored one must not be the first to.
pub fn bounded_string(value: &Value, what: &str, capacity: usize) -> Result<String, StateRefusal> {
    let text = String::from_saved(value, what)?;
    if text.len() > capacity {
        return Err(StateRefusal::new(format!(
            "'{what}' holds {} UTF-8 bytes, past the {capacity} it is bounded by",
            text.len()
        )));
    }
    Ok(text)
}

/// The wall clock now, in milliseconds since the Unix epoch: what a host that
/// has no clock of its own to give [`save`] and [`enter`] gives them.
pub fn wall_clock_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| {
            u64::try_from(since.as_millis()).unwrap_or(u64::MAX)
        })
}

/// A moment as a saved state writes it, which a signed 64-bit reader holds.
fn clamp_to_i64(ms: u64) -> u64 {
    ms.min(i64::MAX as u64)
}

/// Save `engine`, a machine of the document whose shape is `shape`, with the
/// `variables` its generated code read from its fields. The configuration is
/// written in document order, and every waiting delayed `<send>` is written as
/// the moment it comes due on the wall clock whose reading now is
/// `wall_now_ms` (module docs).
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
    wall_now_ms: u64,
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
    // Read before anything is built, so a delayed send this state cannot carry
    // refuses the save instead of leaving it out.
    let pending = save_pending(engine, wall_now_ms)?;
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
        pending,
        // §scxml-6.4: the children that are running, which a restore starts
        // again. The policy lists them in document order.
        invokes: engine
            .policy()
            .running_invokes()
            .into_iter()
            .map(str::to_string)
            .collect(),
        host_invokes: save_host_invokes(engine, wall_now_ms),
        host_invoke_token: engine.next_host_invoke_token(),
        auto_send_seq: engine.auto_send_seq(),
        external: engine
            .external_queue
            .queued()
            .map(|queued| SavedEvent {
                // §scxml-5.10: a queued event is saved under the name it ARRIVED
                // under, so a restore gives the machine the same `_event.name`.
                name: if queued.metadata.name.is_empty() {
                    P::get_event_name(queued.event).to_string()
                } else {
                    queued.metadata.name.to_string()
                },
                data: queued.metadata.data.clone(),
                event_type: queued.metadata.event_type.as_str().to_string(),
                send_id: queued.metadata.send_id.clone(),
                origin: queued.metadata.origin.clone(),
                origin_type: queued.metadata.origin_type.clone(),
                invoke_id: queued.metadata.invoke_id.clone(),
                host_invoke_token: queued.metadata.host_invoke_token,
            })
            .collect(),
    })
}

/// Every host-run invocation of `engine` that is running, or was re-armed by a
/// restore and has yet to start, as a saved state holds it: the request it was
/// started with and the wall-clock moment its deadline comes due. Ordered by
/// `(type, id)`.
fn save_host_invokes<P: StatePolicy>(engine: &Engine<P>, wall_now_ms: u64) -> Vec<SavedHostInvoke> {
    let now = engine.now_ms();
    engine
        .running_host_invokes()
        .into_iter()
        .map(|running| {
            let request = running.request;
            let mut params: Vec<_> = request.params.into_iter().collect();
            params.sort_by(|a, b| a.0.cmp(&b.0));
            SavedHostInvoke {
                processor_type: request.processor_type,
                invoke_id: request.invoke_id,
                src: request.src,
                params,
                data: request.event_data,
                content: request.content,
                due: running.due_at.map(|ready_at| {
                    clamp_to_i64(wall_now_ms.saturating_add(ready_at.saturating_sub(now)))
                }),
            }
        })
        .collect()
}

/// Every delayed send of `engine` still waiting, as a saved state holds it:
/// written as the wall-clock moment it comes due, in the order the machine
/// would deliver them.
///
/// A send the format cannot carry — one routed to a parent or a child session —
/// refuses the save. A document that makes one is generated without the save
/// API, so no generated machine reaches this; the refusal is for a machine
/// built by hand, where leaving the entry out would restore a machine that
/// never delivers it and say nothing.
///
/// The deadline of a running host-run invocation is not a send: it is saved with
/// the invocation, as its `due` ([`save_host_invokes`]), and left out here. One
/// that belongs to no running invocation is refused like a send the format
/// cannot carry.
fn save_pending<P: StatePolicy>(
    engine: &Engine<P>,
    wall_now_ms: u64,
) -> Result<Vec<SavedSend>, StateRefusal> {
    use crate::engine::ScheduledAct;
    use crate::ScheduledRoute;

    let now = engine.now_ms();
    let name = |event: &P::Event| P::get_event_name(*event).to_string();
    let sends = engine
        .scheduler
        .pending()
        .into_iter()
        .map(|(ready_at, act)| {
            let due = clamp_to_i64(wall_now_ms.saturating_add(ready_at.saturating_sub(now)));
            let act = match act {
                // §scxml-5.10: a send is saved under its member's name. A name the
                // document computed (`<send eventexpr>`, `name`) needs a script
                // engine, which no saved machine has, so none is waiting here.
                ScheduledAct::Raise {
                    event,
                    event_data,
                    send_id,
                    origin,
                    ..
                } => SavedAct::Raise {
                    event: name(event),
                    data: event_data.clone(),
                    send_id: send_id.clone(),
                    origin: origin.clone(),
                },
                ScheduledAct::Routed {
                    event: Some(event),
                    event_data,
                    send_id,
                    origin,
                    route: ScheduledRoute::InternalQueue,
                    ..
                } => SavedAct::Internal {
                    event: name(event),
                    data: event_data.clone(),
                    send_id: send_id.clone(),
                    origin: origin.clone(),
                },
                ScheduledAct::Routed { send_id, route, .. } => {
                    return Err(StateRefusal::new(format!(
                        "the delayed send '{send_id}' is routed to {route:?}, a session a \
                         saved state does not carry"
                    )))
                }
                ScheduledAct::HostSend(request) => {
                    let mut params: Vec<_> = request
                        .params
                        .iter()
                        .map(|(name, values)| (name.clone(), values.clone()))
                        .collect();
                    params.sort_by(|a, b| a.0.cmp(&b.0));
                    SavedAct::Host(SavedHostSend {
                        processor_type: request.processor_type.clone(),
                        event: request.event_name.clone(),
                        target: request.target.clone(),
                        content: request.content.clone(),
                        params,
                        send_id: request.send_id.clone(),
                        data: request.event_data.clone(),
                        invoke_id: request.invoke_id.clone(),
                    })
                }
                ScheduledAct::HostInvokeDeadline {
                    processor_type,
                    invoke_id,
                    token,
                } => {
                    return if engine.host_invoke_is_running(processor_type, invoke_id, *token) {
                        Ok(None)
                    } else {
                        Err(StateRefusal::new(format!(
                            "the deadline of the invocation '{invoke_id}' belongs to no \
                             invocation this machine is running"
                        )))
                    };
                }
            };
            Ok(Some(SavedSend { due, act }))
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(sends.into_iter().flatten().collect())
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
///
/// The machine measures time by `clock`, which is installed before anything is
/// armed — a delayed send armed against one clock and judged against another
/// would never come due when it should. `wall_now_ms` is what time it is on
/// the wall clock the saved `due`s were written against: a waiting send is
/// armed to come due `due - wall_now_ms` after now, and one already due is
/// armed to come due now, behind the ones due before it.
pub fn enter<P: StatePolicy>(
    policy: P,
    saved: &SavedState,
    clock: SceClock,
    wall_now_ms: u64,
) -> Result<Engine<P>, StateRefusal> {
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
            // The name a queued event arrived under (§scxml-5.10): the document's
            // own event for it, or the one it extends (§scxml-3.12.1).
            let event = P::resolve_event_by_name(&e.name).ok_or_else(|| {
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
            if P::get_event_name(event) != e.name {
                queued.metadata.name = e.name.clone();
            }
            queued.metadata.data = e.data.clone();
            queued.metadata.event_type = event_type;
            queued.metadata.send_id = e.send_id.clone();
            queued.metadata.origin = e.origin.clone();
            queued.metadata.origin_type = e.origin_type.clone();
            queued.metadata.invoke_id = e.invoke_id.clone();
            queued.metadata.host_invoke_token = e.host_invoke_token;
            Ok(queued)
        })
        .collect::<Result<Vec<_>, StateRefusal>>()?;
    // The same for a waiting send: an event this document does not name refuses
    // the restore, not the sends before it.
    let pending = saved
        .pending
        .iter()
        .enumerate()
        .map(|(i, send)| ReadSend::<P::Event>::read::<P>(send, i))
        .collect::<Result<Vec<_>, StateRefusal>>()?;
    // The same for a running invocation: one the document does not have, one
    // whose state the saved configuration does not stand in, or one named
    // twice would start a child the saved machine could not have had.
    let mut restarts: Vec<&str> = Vec::with_capacity(saved.invokes.len());
    for (i, id) in saved.invokes.iter().enumerate() {
        let owner = P::invoke_owner(id).ok_or_else(|| {
            StateRefusal::new(format!(
                "invokes[{i}] is '{id}', which the document does not invoke"
            ))
        })?;
        if !configuration.contains(&owner) {
            return Err(StateRefusal::new(format!(
                "invokes[{i}] is '{id}', whose state the saved configuration does not stand in"
            )));
        }
        if restarts.contains(&id.as_str()) {
            return Err(StateRefusal::new(format!(
                "invokes[{i}] is '{id}', which an earlier entry already names"
            )));
        }
        restarts.push(id);
    }
    // And for a host-run one, by the `(type, id)` the document declares: one it
    // does not have, one whose state the saved configuration does not stand in,
    // or one named twice is a run the saved machine could not have had.
    for (i, host) in saved.host_invokes.iter().enumerate() {
        let (processor_type, id) = (host.processor_type.as_str(), host.invoke_id.as_str());
        let owner = P::host_invoke_owner(processor_type, id).ok_or_else(|| {
            StateRefusal::new(format!(
                "hostinvokes[{i}] is '{id}' of type '{processor_type}', which the document \
                 does not have a host invoker run"
            ))
        })?;
        if !configuration.contains(&owner) {
            return Err(StateRefusal::new(format!(
                "hostinvokes[{i}] is '{id}', whose state the saved configuration does not \
                 stand in"
            )));
        }
        if saved.host_invokes[..i]
            .iter()
            .any(|earlier| earlier.processor_type == processor_type && earlier.invoke_id == id)
        {
            return Err(StateRefusal::new(format!(
                "hostinvokes[{i}] is '{id}', which an earlier entry already names"
            )));
        }
    }
    let mut engine = Engine::new(policy);
    engine.set_clock(clock);
    engine
        .enter_at(&configuration, current)
        .map_err(|rejection| StateRefusal::new(describe::<P>(&rejection)))?;
    for queued in external {
        engine.raise_external_with_meta(queued);
    }
    arm_pending(&mut engine, pending, wall_now_ms);
    rearm_host_invokes(&mut engine, &saved.host_invokes, wall_now_ms);
    engine.set_next_host_invoke_token(saved.host_invoke_token);
    engine.set_auto_send_seq(saved.auto_send_seq);
    // Last, so what a child sends as it starts stands behind what was already
    // waiting: the saved machine's queue was ahead of it.
    engine.restart_invokes(&restarts);
    Ok(engine)
}

/// Re-arm each of `host_invokes` on `engine`, in the order a saved state lists
/// them. None starts here: the host registers its invokers on the engine this
/// restore returns, so each starts at the top of the first macrostep it drives
/// (`Engine::start_restored_host_invokes`). A deadline comes due
/// `due - wall_now_ms` after now on the engine's own clock; one already due
/// ends the invocation without a start.
fn rearm_host_invokes<P: StatePolicy>(
    engine: &mut Engine<P>,
    host_invokes: &[SavedHostInvoke],
    wall_now_ms: u64,
) {
    use crate::engine::RestoredDeadline;

    let now = engine.now_ms();
    for host in host_invokes {
        let deadline = match host.due {
            None => RestoredDeadline::None,
            Some(due) if due <= wall_now_ms => RestoredDeadline::Passed,
            Some(due) => RestoredDeadline::At(now.saturating_add(due - wall_now_ms)),
        };
        engine.rearm_host_invoke(
            crate::HostInvokeRequest {
                processor_type: host.processor_type.clone(),
                invoke_id: host.invoke_id.clone(),
                src: host.src.clone(),
                params: host.params.iter().cloned().collect(),
                event_data: host.data.clone(),
                content: host.content.clone(),
                token: 0,
                restarted: true,
            },
            deadline,
        );
    }
}

/// A waiting send read against the document: the event it names, resolved, so a
/// restore that cannot resolve one refuses before it arms any.
enum ReadSend<'s, E> {
    Raise {
        due: u64,
        event: E,
        data: &'s str,
        send_id: &'s str,
        origin: &'s str,
    },
    Internal {
        due: u64,
        event: E,
        data: &'s str,
        send_id: &'s str,
        origin: &'s str,
    },
    Host {
        due: u64,
        host: &'s SavedHostSend,
    },
}

impl<'s, E> ReadSend<'s, E> {
    /// `send`, the `index`th of a saved state's `pending`, with its event
    /// looked up in the document of `P`.
    fn read<P: StatePolicy<Event = E>>(
        send: &'s SavedSend,
        index: usize,
    ) -> Result<Self, StateRefusal> {
        let event_of = |name: &str| {
            P::get_event_from_name(name).ok_or_else(|| {
                StateRefusal::new(format!(
                    "pending[{index}] is '{name}', which the document does not name"
                ))
            })
        };
        let due = send.due;
        Ok(match &send.act {
            SavedAct::Raise {
                event,
                data,
                send_id,
                origin,
            } => Self::Raise {
                due,
                event: event_of(event)?,
                data,
                send_id,
                origin,
            },
            SavedAct::Internal {
                event,
                data,
                send_id,
                origin,
            } => Self::Internal {
                due,
                event: event_of(event)?,
                data,
                send_id,
                origin,
            },
            SavedAct::Host(host) => Self::Host { due, host },
        })
    }
}

/// Arm each of `pending`, already read, in the order a saved state lists them:
/// each comes due `due - wall_now_ms` after the engine's now, and one that came
/// due while the machine was away comes due now. Entries due at the same
/// moment are armed in order and the scheduler delivers those in the order they
/// were armed, which is how the saved machine would have.
fn arm_pending<P: StatePolicy>(
    engine: &mut Engine<P>,
    pending: Vec<ReadSend<'_, P::Event>>,
    wall_now_ms: u64,
) {
    use crate::ScheduledRoute;

    let now = engine.now_ms();
    let ready_at = |due: u64| now.saturating_add(due.saturating_sub(wall_now_ms));
    for read in pending {
        match read {
            ReadSend::Raise {
                due,
                event,
                data,
                send_id,
                origin,
            } => {
                engine
                    .scheduler
                    .schedule_event_at(event, ready_at(due), send_id, data, origin);
            }
            ReadSend::Internal {
                due,
                event,
                data,
                send_id,
                origin,
            } => {
                engine.scheduler.schedule_routed_at(
                    Some(event),
                    ready_at(due),
                    send_id,
                    data,
                    origin,
                    ScheduledRoute::InternalQueue,
                );
            }
            ReadSend::Host { due, host } => {
                let request = crate::HostSendRequest {
                    processor_type: host.processor_type.clone(),
                    event_name: host.event.clone(),
                    target: host.target.clone(),
                    content: host.content.clone(),
                    params: host.params.iter().cloned().collect(),
                    send_id: host.send_id.clone(),
                    event_data: host.data.clone(),
                    invoke_id: host.invoke_id.clone(),
                };
                engine
                    .scheduler
                    .schedule_host_send_at(request, ready_at(due), &host.send_id);
            }
        }
    }
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
            pending: vec![
                SavedSend {
                    due: 1_700_000_005_000,
                    act: SavedAct::Raise {
                        event: "timeout".to_string(),
                        data: String::new(),
                        send_id: "timer".to_string(),
                        origin: "s1".to_string(),
                    },
                },
                SavedSend {
                    due: 1_700_000_006_000,
                    act: SavedAct::Internal {
                        event: "inner".to_string(),
                        data: "{\"n\":2}".to_string(),
                        send_id: "__send_1".to_string(),
                        origin: String::new(),
                    },
                },
                SavedSend {
                    due: i64::MAX as u64,
                    act: SavedAct::Host(SavedHostSend {
                        processor_type: "BasicHTTP".to_string(),
                        event: "notify".to_string(),
                        target: "http://host/x".to_string(),
                        content: String::new(),
                        params: vec![
                            ("a".to_string(), vec!["1".to_string(), "2".to_string()]),
                            ("b".to_string(), vec![]),
                        ],
                        send_id: "__send_2".to_string(),
                        data: "{\"a\":[\"1\",\"2\"]}".to_string(),
                        invoke_id: String::new(),
                    }),
                },
            ],
            invokes: vec!["worker".to_string(), "spare".to_string()],
            host_invokes: vec![
                SavedHostInvoke {
                    processor_type: "x-host".to_string(),
                    invoke_id: "h".to_string(),
                    src: "job://1".to_string(),
                    params: vec![
                        ("a".to_string(), vec!["1".to_string(), "2".to_string()]),
                        ("b".to_string(), vec![]),
                    ],
                    data: "{\"a\":[1,2]}".to_string(),
                    content: "body".to_string(),
                    due: Some(i64::MAX as u64),
                },
                SavedHostInvoke {
                    processor_type: "x-host".to_string(),
                    invoke_id: "no_deadline".to_string(),
                    due: None,
                    ..SavedHostInvoke::default()
                },
            ],
            host_invoke_token: i64::MAX as u64,
            auto_send_seq: 41,
            external: vec![
                SavedEvent {
                    name: "tick".to_string(),
                    data: "{\"n\":1}".to_string(),
                    event_type: "external".to_string(),
                    ..SavedEvent::default()
                },
                SavedEvent {
                    name: "done.invoke.h".to_string(),
                    event_type: "external".to_string(),
                    invoke_id: "h".to_string(),
                    host_invoke_token: Some(7),
                    ..SavedEvent::default()
                },
            ],
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
    fn a_32_bit_real_is_written_as_the_64_bit_real_it_widens_to() {
        // The binary32 nearest 0.3 is not 0.3: a reader that holds a JSON number
        // as a double must find the value the machine holds.
        let single = 0.3f32;
        assert_eq!(
            single.to_saved(),
            Value::Number("0.30000001192092896".to_string())
        );
        assert_eq!(
            f32::from_saved(&single.to_saved(), "x").expect("reads back"),
            single
        );
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
        let wrong = r#"{"format":1,"shape":"d","configuration":[],"current":"s","variables":{},"history":{"h":"a"},"pending":[],"external":[]}"#;
        let refusal = SavedState::from_json(wrong).expect_err("not an array");
        assert!(refusal.reason().contains("history.h"), "{refusal}");
    }

    /// Every field is always present: a state with nothing waiting says so with
    /// `[]`, and one that does not say is not this format.
    #[test]
    fn a_saved_state_with_no_pending_field_is_refused() {
        let text = r#"{"format":1,"shape":"d","configuration":[],"current":"s","variables":{},"history":{},"external":[]}"#;
        let refusal = SavedState::from_json(text).expect_err("no pending");
        assert!(refusal.reason().contains("'pending'"), "{refusal}");
    }

    fn with_pending(pending: &str) -> String {
        format!(
            r#"{{"format":1,"shape":"d","configuration":[],"current":"s","variables":{{}},"history":{{}},"pending":[{pending}],"external":[]}}"#
        )
    }

    #[test]
    fn a_due_that_is_not_a_whole_number_of_milliseconds_is_refused() {
        for due in [
            "\"soon\"",
            "\"-1\"",
            "\"+5\"",
            "\"1.5\"",
            // One past the largest value a signed 64-bit reader holds.
            "\"9223372036854775808\"",
            // A number where a text belongs: the format writes a moment as a text.
            "5000",
        ] {
            let text = with_pending(&format!(
                r#"{{"due":{due},"act":"raise","event":"e","data":"","sendid":"","origin":""}}"#
            ));
            let refusal = SavedState::from_json(&text).expect_err(due);
            assert!(
                refusal.reason().contains("pending[0].due"),
                "{due}: {refusal}"
            );
        }
    }

    #[test]
    fn a_send_that_is_none_of_the_three_acts_is_refused() {
        let text = with_pending(r#"{"due":"1","act":"parent","event":"e"}"#);
        let refusal = SavedState::from_json(&text).expect_err("not an act");
        assert!(refusal.reason().contains("pending[0].act"), "{refusal}");
    }

    #[test]
    fn a_host_send_whose_params_are_not_lists_of_texts_is_refused() {
        let send = |params: &str| {
            with_pending(&format!(
                r#"{{"due":"1","act":"host","type":"t","event":"e","target":"","content":"","params":{params},"sendid":"","data":"","invokeid":""}}"#
            ))
        };
        for params in [r#"{"p":"v"}"#, r#"{"p":[1]}"#, r#"["p"]"#] {
            let refusal = SavedState::from_json(&send(params)).expect_err(params);
            assert!(
                refusal.reason().contains("pending[0].params"),
                "{params}: {refusal}"
            );
        }
    }
}

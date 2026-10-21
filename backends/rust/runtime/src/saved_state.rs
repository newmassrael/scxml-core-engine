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
//! Nor does one that added a field to a record schema (docs/adr/0044): the
//! shape names a record schema's first-revision fields, and a state names the
//! revision each schema past its first was saved at (`revisions`), so a later
//! build reads a state of an earlier revision with each added field at its
//! default ([`crate::saved_state::upgrade`]). Such a state carries a second
//! shape, which a reader that predates revisions does not know and refuses.
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

/// The format of a saved state that names each running child by its id alone
/// (`schemas/sce-saved-state.v1.schema.json`): a restore starts each again from
/// its beginning.
pub const FORMAT_1: u32 = 1;

/// The format of a saved state that is a tree of sessions
/// (`schemas/sce-saved-state.v2.schema.json`; docs/adr/0045): each running child
/// is saved inside its parent with its whole state, and a restore resumes it.
pub const FORMAT_2: u32 = 2;

/// The format version this runtime writes. It reads [`FORMAT_1`] and
/// [`FORMAT_2`].
pub const FORMAT: u32 = FORMAT_2;

/// The stability status of the saved-state wire surface, held in lockstep
/// with `x-sce-schema-status` in `schemas/sce-saved-state.v1.schema.json`
/// and `schemas/sce-saved-state.v2.schema.json`, the surface's two formats
/// (`SCE_WIRE_CONTRACTS.md`). A flip to `"stable"` changes all three in one
/// commit.
pub const SCHEMA_STATUS: &str = "pre-release";

/// Why a saved state cannot be restored, or a machine cannot be saved.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StateRefusal(String);

/// What a refusal inside a child session of a tree begins with, before the
/// session's path ([`StateRefusal::within`]).
const SESSION_AT: &str = "the session at ";

impl StateRefusal {
    /// Refuse, saying why.
    pub fn new(reason: impl Into<String>) -> Self {
        Self(reason.into())
    }

    /// Why.
    pub fn reason(&self) -> &str {
        &self.0
    }

    /// This refusal, of the child session at `path` of the session that reads
    /// it (`invokes[0].session`): a tree is refused as a whole, and the reason
    /// names the session it is about by its path from the root
    /// (docs/adr/0045, section 2), `invokes[0].session.invokes[1].session`
    /// for a grandchild.
    pub fn within(self, path: &str) -> Self {
        match self.0.strip_prefix(SESSION_AT) {
            Some(deeper) => Self(format!("{SESSION_AT}{path}.{deeper}")),
            None => Self(format!("{SESSION_AT}{path}: {}", self.0)),
        }
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
    /// The `<invoke>`s whose child session is running (§scxml-6.4), in
    /// document order: in format 2 each with the child's whole state, which a
    /// restore resumes; in format 1 by the id alone, which a restore starts
    /// again from its beginning ([`SavedInvokes`]). One whose child has ended is
    /// absent — its `done.invoke` is in the external queue, or already taken —
    /// so it is not started twice.
    pub invokes: SavedInvokes,
    /// The session's own id (`_sessionid`), which events held in the tree name
    /// sessions by and a restore binds again (docs/adr/0045). An XML Nmtoken in
    /// format 2; empty in a format-1 state, which holds none.
    pub session_id: String,
    /// How many times each `<invoke>` of the document has started in this
    /// session, by its id, for each started at least once, in document order:
    /// a start after a restore is numbered on from it. Empty in a format-1
    /// state, which holds none.
    pub invoke_starts: Vec<(String, u64)>,
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
    /// The revision each record schema past its first was saved at, by the
    /// event the schema declares, ordered by it (docs/adr/0044). Empty when
    /// every record schema the machine holds is in its first revision, which
    /// is every state saved before schemas had revisions; [`upgrade`] reads a
    /// state saved at an earlier revision as the machine's own.
    pub revisions: Vec<(String, SavedRevision)>,
}

/// The running child sessions of a saved session, as the format it was written
/// in holds them.
#[derive(Clone, Debug, PartialEq)]
pub enum SavedInvokes {
    /// Format 1: the id of each `<invoke>` whose child is running. The text
    /// holds nothing of a child, so a restore starts each again from its
    /// beginning, under the same id: all that state promised.
    Restarted(Vec<String>),
    /// Format 2: each running child with its whole state, resumed by a restore
    /// where it stood (docs/adr/0045).
    Resumed(Vec<SavedChild>),
}

impl SavedInvokes {
    /// The id the document gives each running `<invoke>`, in order.
    pub fn ids(&self) -> Vec<&str> {
        match self {
            Self::Restarted(ids) => ids.iter().map(String::as_str).collect(),
            Self::Resumed(children) => children.iter().map(|c| c.id.as_str()).collect(),
        }
    }
}

/// A running child session, saved inside the session that started it
/// (docs/adr/0045, section 1).
#[derive(Clone, Debug, PartialEq)]
pub struct SavedChild {
    /// The id the document gives the `<invoke>` that started it.
    pub id: String,
    /// Which start of that `<invoke>` this is in its parent, counted from 1:
    /// the count the parent's `invoke_starts` holds for it.
    pub start: u64,
    /// The id the child was started under, as `_event.invokeid` and an
    /// `idlocation` variable hold it: carried, not computed again.
    pub invoke_id: String,
    /// The document the child runs: the child's name for a static `<invoke>`,
    /// the candidate's for a hybrid one.
    pub document: String,
    /// The child's own state, its running children included.
    pub session: SavedState,
}

/// The revision of one record schema a state was saved at.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SavedRevision {
    /// The schema's `sce:revision` where the state was saved, 2 or later.
    pub revision: u32,
    /// The SHA-256, as lowercase hex, of what that revision declares: each
    /// field it has, with its type and its enum's variant names. A restoring
    /// machine holds the same digest of each of its own schema's revisions, so
    /// two builds that disagree about what one revision declares are told apart.
    pub fields: String,
}

impl SavedRevision {
    /// The revision `value` holds; `what` names it in a refusal.
    fn from_value(value: &Value, what: &str) -> Result<Self, StateRefusal> {
        let member = |name: &str| {
            value
                .member(name)
                .ok_or_else(|| StateRefusal::new(format!("'{what}' has no '{name}'")))
        };
        let revision = match member("revision")? {
            Value::Number(n) => n.parse::<u32>().ok().filter(|r| *r >= 2).ok_or_else(|| {
                StateRefusal::new(format!(
                    "'{what}.revision' ({n}) is not a revision past the first; a schema in its \
                     first revision is left out"
                ))
            })?,
            _ => {
                return Err(StateRefusal::new(format!(
                    "'{what}.revision' is not a number"
                )))
            }
        };
        let fields = match member("fields")? {
            Value::Text(digest)
                if digest.len() == 64
                    && digest
                        .bytes()
                        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)) =>
            {
                digest.clone()
            }
            _ => {
                return Err(StateRefusal::new(format!(
                    "'{what}.fields' is not a SHA-256 written as lowercase hex"
                )))
            }
        };
        Ok(Self { revision, fields })
    }
}

/// What a machine's saved state is bound to: the document's shape, and the
/// revisions of each schema a record variable of it is of (docs/adr/0044). The
/// generator computes it, and every saved state is written and judged against it.
#[derive(Clone, Copy, Debug)]
pub struct SavedShape {
    /// The shape a state names when every record schema it holds is in its
    /// first revision: the states, histories, invocations and variables, each
    /// record variable with the fields of its schema's first revision.
    pub first: &'static str,
    /// The shape a state names when it names a later revision of any record
    /// schema. A reader that predates revisions knows only the first shape, so it
    /// refuses such a state rather than read it without the fields it does not
    /// know.
    pub revised: &'static str,
    /// [`Self::first`] as a format-2 state names it (docs/adr/0045, section 2):
    /// each enum-typed variable and record field spelled by its enum's
    /// document ([`EnumLineage::key`]) rather than by the alias it is imported
    /// under, so renaming an alias leaves it as it was. The same as
    /// [`Self::first`] for a document that holds no enum.
    pub format2_first: &'static str,
    /// [`Self::revised`] as a format-2 state names it.
    pub format2_revised: &'static str,
    /// Each record schema a variable is of, by the event it declares, ordered by
    /// it.
    pub records: &'static [RecordLineage],
    /// Each enum a value of the state is of, by its key, ordered by it.
    pub enums: &'static [EnumLineage],
}

/// An enum a value of a machine's state is of, as a saved state names it
/// (docs/adr/0044).
#[derive(Clone, Copy, Debug)]
pub struct EnumLineage {
    /// `enum:` and the enum's document, which names it in `revisions`: a
    /// standard document by its `sce:std/...` name, any other by its path
    /// relative to the machine's own document, `/`-separated. The document,
    /// not an alias it is imported under, so renaming an alias leaves it as it
    /// was.
    pub key: &'static str,
    /// The enum's revision in this machine.
    pub revision: u32,
    /// The digest of what each revision from the second to [`Self::revision`]
    /// declares, in order ([`SavedRevision::fields`]).
    pub revisions: &'static [&'static str],
    /// Each variant a revision past the first added.
    pub added: &'static [AddedVariant],
    /// Where a value of it is held.
    pub holders: &'static [EnumHolder],
}

/// A variant a revision past the first added to an enum.
#[derive(Clone, Copy, Debug)]
pub struct AddedVariant {
    /// The variant's declared name, which a saved state holds it by.
    pub name: &'static str,
    /// The revision that added it.
    pub since: u32,
}

/// Where a value of an [`EnumLineage`] is held.
#[derive(Clone, Copy, Debug)]
pub struct EnumHolder {
    /// The variable's document id.
    pub variable: &'static str,
    /// The field of the record the variable holds, for a value held in a
    /// record; `None` for a variable of the enum.
    pub field: Option<&'static str>,
    /// Whether the variable holds a list of such records rather than one.
    pub list: bool,
}

/// A record schema a machine's variables are of, as a saved state names it.
#[derive(Clone, Copy, Debug)]
pub struct RecordLineage {
    /// The event the schema declares, which names it in `revisions`.
    pub event: &'static str,
    /// The schema's revision in this machine.
    pub revision: u32,
    /// The digest of what each revision from the second to [`Self::revision`]
    /// declares, in order ([`SavedRevision::fields`]).
    pub revisions: &'static [&'static str],
    /// Each field a revision past the first added, in the schema's order.
    pub added: &'static [AddedField],
    /// The variables that hold a record of it, or a list of them.
    pub holders: &'static [RecordHolder],
}

/// A field a revision past the first added to a record schema.
#[derive(Clone, Copy, Debug)]
pub struct AddedField {
    /// The field's id, the member a saved record holds it under.
    pub id: &'static str,
    /// The revision that added it.
    pub since: u32,
    /// Its `sce:default`, as saved-state JSON (the JSON a delivery carries,
    /// which a saved state writes the same way for every type a field can have).
    pub default: &'static str,
}

/// A variable that holds a record of a [`RecordLineage`].
#[derive(Clone, Copy, Debug)]
pub struct RecordHolder {
    /// The variable's document id.
    pub variable: &'static str,
    /// Whether it holds a list of records rather than one.
    pub list: bool,
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

/// What a waiting `<send>` does when it comes due: its own event on this
/// session's external queue or on its internal queue (`#_internal`), an act a
/// host-served processor performs (§scxml-6.2.5), or, in format 2, an event for
/// the parent or for a running child of this session, sessions the saved tree
/// holds (docs/adr/0045).
///
/// A delayed send to any other session is none of them, and a document that
/// makes one is generated without the save API (§2.15): such a send is
/// delivered through a session this state does not carry.
#[derive(Clone, Debug, PartialEq)]
pub enum SavedAct {
    /// An event for the session that started this one (`#_parent`).
    Parent {
        /// The event's name, as the parent resolves it.
        event: String,
        /// `_event.data`, as the wire carries it.
        data: String,
        /// The `<send>`'s id, which a `<cancel>` names.
        send_id: String,
        /// `_event.origin`: this session's id.
        origin: String,
    },
    /// An event for a running child of this session (`#_<invokeid>`).
    Child {
        /// The id the target names the child's `<invoke>` by.
        invoke_id: String,
        /// The event's name, as the child resolves it.
        event: String,
        /// `_event.data`, as the wire carries it.
        data: String,
        /// The `<send>`'s id, which a `<cancel>` names.
        send_id: String,
        /// `_event.origin`: this session's id.
        origin: String,
    },
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

/// A count of starts, as the text a format-2 saved state writes it as: a
/// positive whole number in digits, with no leading zero, within a signed
/// 64-bit count, as every backend reads one.
fn read_count(written: &str, what: &str) -> Result<u64, StateRefusal> {
    written
        .parse::<u64>()
        .ok()
        .filter(|count| {
            *count >= 1
                && *count <= i64::MAX as u64
                && !written.starts_with('0')
                && written.bytes().all(|b| b.is_ascii_digit())
        })
        .ok_or_else(|| {
            StateRefusal::new(format!(
                "'{what}' ({written}) is not a count of starts, a whole number from 1"
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
                    SavedAct::Parent { .. } => "parent",
                    SavedAct::Child { .. } => "child",
                }),
            ),
        ];
        match &self.act {
            SavedAct::Parent {
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
            SavedAct::Child {
                invoke_id,
                event,
                data,
                send_id,
                origin,
            } => members.extend([
                ("invokeid".to_string(), text(invoke_id)),
                ("event".to_string(), text(event)),
                ("data".to_string(), text(data)),
                ("sendid".to_string(), text(send_id)),
                ("origin".to_string(), text(origin)),
            ]),
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

    /// The waiting send `value`, of a state of `format`; `what` names it in a
    /// refusal. A send to the parent or to a child is an act of format 2 only.
    fn from_value(value: &Value, what: &str, format: u32) -> Result<Self, StateRefusal> {
        let text = |key: &str| match value.member(key) {
            Some(Value::Text(s)) => Ok(s.clone()),
            Some(_) => Err(StateRefusal::new(format!("'{what}.{key}' is not a text"))),
            None => Err(StateRefusal::new(format!("'{what}' has no '{key}'"))),
        };
        let due = read_moment(&text("due")?, &format!("{what}.due"))?;
        let act = match text("act")?.as_str() {
            "parent" if format == FORMAT_2 => SavedAct::Parent {
                event: text("event")?,
                data: text("data")?,
                send_id: text("sendid")?,
                origin: text("origin")?,
            },
            "child" if format == FORMAT_2 => SavedAct::Child {
                invoke_id: text("invokeid")?,
                event: text("event")?,
                data: text("data")?,
                send_id: text("sendid")?,
                origin: text("origin")?,
            },
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
            other if format == FORMAT_1 => {
                return Err(StateRefusal::new(format!(
                    "'{what}.act' is '{other}', which is not raise, internal or host, the acts \
                     of format 1"
                )))
            }
            other => {
                return Err(StateRefusal::new(format!(
                    "'{what}.act' is '{other}', which is not raise, internal, host, parent or \
                     child"
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

    /// The format this state is of: [`FORMAT_2`] for a tree of sessions,
    /// [`FORMAT_1`] for a state that names its running children by id alone.
    pub fn format(&self) -> u32 {
        match self.invokes {
            SavedInvokes::Restarted(_) => FORMAT_1,
            SavedInvokes::Resumed(_) => FORMAT_2,
        }
    }

    /// The id a restore binds the session to: the saved one, for a format-2
    /// state, since the session carries on and every child's id and a peer's
    /// address name it (docs/adr/0045); `None` for a format-1 state, which
    /// holds none, so the restored machine keeps the id it was created with.
    pub fn bound_session_id(&self) -> Option<&str> {
        (self.format() == FORMAT_2).then_some(self.session_id.as_str())
    }

    /// How many times the `<invoke>` `id` had started in this session when it
    /// was saved: 0 for one that had not, and for every one of a format-1
    /// state, which holds no count.
    pub fn invoke_start(&self, id: &str) -> u64 {
        self.invoke_starts
            .iter()
            .find(|(invoke, _)| invoke == id)
            .map_or(0, |(_, count)| *count)
    }

    /// The state as `sce-saved-state` JSON, in the format it is of
    /// ([`Self::format`]): the root of the text, with its `format`.
    pub fn to_json(&self) -> String {
        let mut members = vec![(
            "format".to_string(),
            Value::Number(self.format().to_string()),
        )];
        members.extend(self.members());
        json::write(&Value::Object(members))
    }

    /// The members of this session, in the order every backend writes them:
    /// format 1's, and in format 2 `sessionid` after `shape` and `invokestarts`
    /// after `invokes`, each running child's session written by the same rule.
    fn members(&self) -> Vec<(String, Value)> {
        let format2 = matches!(self.invokes, SavedInvokes::Resumed(_));
        let mut members = vec![("shape".to_string(), Value::Text(self.shape.clone()))];
        if format2 {
            members.push((
                "sessionid".to_string(),
                Value::Text(self.session_id.clone()),
            ));
        }
        members.extend([
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
        ]);
        match &self.invokes {
            SavedInvokes::Restarted(ids) => members.push((
                "invokes".to_string(),
                Value::Array(ids.iter().cloned().map(Value::Text).collect()),
            )),
            SavedInvokes::Resumed(children) => {
                members.push((
                    "invokes".to_string(),
                    Value::Array(children.iter().map(SavedChild::to_value).collect()),
                ));
                members.push((
                    "invokestarts".to_string(),
                    Value::Object(
                        self.invoke_starts
                            .iter()
                            .map(|(id, count)| (id.clone(), Value::Text(count.to_string())))
                            .collect(),
                    ),
                ));
            }
        }
        members.extend([
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
        ]);
        // Absent, not empty, when no record schema is past its first revision:
        // the text of such a state is the text it was before revisions existed.
        if !self.revisions.is_empty() {
            members.push((
                "revisions".to_string(),
                Value::Object(
                    self.revisions
                        .iter()
                        .map(|(event, saved)| {
                            (
                                event.clone(),
                                Value::Object(vec![
                                    (
                                        "revision".to_string(),
                                        Value::Number(saved.revision.to_string()),
                                    ),
                                    ("fields".to_string(), Value::Text(saved.fields.clone())),
                                ]),
                            )
                        })
                        .collect(),
                ),
            ));
        }
        members
    }

    /// Read `sce-saved-state` JSON, of format 1 or 2. The shape is not judged
    /// here — the machine that restores it knows its own ([`upgrade`]).
    pub fn from_json(text: &str) -> Result<Self, StateRefusal> {
        let value =
            json::parse(text).map_err(|e| StateRefusal::new(format!("the saved state is {e}")))?;
        let format = match value.member("format") {
            Some(Value::Number(n)) if n == &FORMAT_1.to_string() => FORMAT_1,
            Some(Value::Number(n)) if n == &FORMAT_2.to_string() => FORMAT_2,
            Some(other) => {
                return Err(StateRefusal::new(format!(
                    "the saved state is format {}, and this runtime reads formats {FORMAT_1} \
                     and {FORMAT_2}",
                    json::write(other)
                )))
            }
            None => return Err(StateRefusal::new("the saved state has no 'format'")),
        };
        Self::from_session(&value, format)
    }

    /// The session `value` of a text of `format`: the root, or a child's
    /// `session`, which states no `format` of its own.
    fn from_session(value: &Value, format: u32) -> Result<Self, StateRefusal> {
        let field = |name: &str| {
            value
                .member(name)
                .ok_or_else(|| StateRefusal::new(format!("the saved state has no '{name}'")))
        };
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
                .map(|(i, v)| SavedSend::from_value(v, &format!("pending[{i}]"), format))
                .collect::<Result<Vec<_>, _>>()?,
            _ => return Err(StateRefusal::new("'pending' is not an array")),
        };
        let invokes = match (field("invokes")?, format) {
            (Value::Array(items), FORMAT_1) => SavedInvokes::Restarted(
                items
                    .iter()
                    .map(|v| text_of(v, "invokes"))
                    .collect::<Result<Vec<_>, _>>()?,
            ),
            (Value::Array(items), _) => SavedInvokes::Resumed(
                items
                    .iter()
                    .enumerate()
                    .map(|(i, v)| SavedChild::from_value(v, i))
                    .collect::<Result<Vec<_>, _>>()?,
            ),
            _ => return Err(StateRefusal::new("'invokes' is not an array")),
        };
        // Format 2's own members: the session's id and its count of starts.
        let (session_id, invoke_starts) = if format == FORMAT_1 {
            (String::new(), Vec::new())
        } else {
            let session_id = text_of(field("sessionid")?, "sessionid")?;
            if !crate::helpers::session_id::is_nmtoken(&session_id) {
                return Err(StateRefusal::new(format!(
                    "'sessionid' ({session_id}) is not an XML Nmtoken, which a session id is"
                )));
            }
            let invoke_starts = match field("invokestarts")? {
                Value::Object(members) => members
                    .iter()
                    .map(|(id, count)| match count {
                        Value::Text(written) => read_count(written, &format!("invokestarts.{id}"))
                            .map(|count| (id.clone(), count)),
                        _ => Err(StateRefusal::new(format!(
                            "'invokestarts.{id}' is not a text"
                        ))),
                    })
                    .collect::<Result<Vec<_>, _>>()?,
                _ => return Err(StateRefusal::new("'invokestarts' is not an object")),
            };
            (session_id, invoke_starts)
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
        // Optional, unlike every member above: a state whose record schemas are
        // all in their first revision leaves it out, as every state saved before
        // schemas had revisions does.
        let revisions = match value.member("revisions") {
            None => Vec::new(),
            Some(Value::Object(members)) if members.is_empty() => {
                return Err(StateRefusal::new(
                    "'revisions' names no schema; a state whose record schemas are all in \
                     their first revision leaves it out",
                ))
            }
            Some(Value::Object(members)) => members
                .iter()
                .map(|(event, v)| {
                    SavedRevision::from_value(v, &format!("revisions.{event}"))
                        .map(|saved| (event.clone(), saved))
                })
                .collect::<Result<Vec<_>, _>>()?,
            Some(_) => return Err(StateRefusal::new("'revisions' is not an object")),
        };
        Ok(Self {
            shape: text_of(field("shape")?, "shape")?,
            revisions,
            configuration,
            current: text_of(field("current")?, "current")?,
            variables,
            history,
            pending,
            invokes,
            session_id,
            invoke_starts,
            host_invokes,
            host_invoke_token,
            auto_send_seq,
            external,
        })
    }
}

impl SavedChild {
    fn to_value(&self) -> Value {
        let text = |s: &str| Value::Text(s.to_string());
        Value::Object(vec![
            ("id".to_string(), text(&self.id)),
            ("start".to_string(), Value::Text(self.start.to_string())),
            ("invokeid".to_string(), text(&self.invoke_id)),
            ("document".to_string(), text(&self.document)),
            ("session".to_string(), Value::Object(self.session.members())),
        ])
    }

    /// The `index`th running child of a format-2 session. A refusal inside the
    /// child's own session names it by its path ([`StateRefusal::within`]).
    fn from_value(value: &Value, index: usize) -> Result<Self, StateRefusal> {
        let what = format!("invokes[{index}]");
        let text = |key: &str| match value.member(key) {
            Some(Value::Text(s)) if !s.is_empty() => Ok(s.clone()),
            Some(Value::Text(_)) => Err(StateRefusal::new(format!("'{what}.{key}' is empty"))),
            Some(_) => Err(StateRefusal::new(format!("'{what}.{key}' is not a text"))),
            None => Err(StateRefusal::new(format!("'{what}' has no '{key}'"))),
        };
        let start = read_count(&text("start")?, &format!("{what}.start"))?;
        let session = match value.member("session") {
            Some(session @ Value::Object(_)) => {
                if session.member("format").is_some() {
                    return Err(StateRefusal::new(format!(
                        "'{what}.session' states a format, which only the root of a tree does"
                    )));
                }
                SavedState::from_session(session, FORMAT_2)
                    .map_err(|refusal| refusal.within(&format!("{what}.session")))?
            }
            Some(_) => {
                return Err(StateRefusal::new(format!(
                    "'{what}.session' is not an object"
                )))
            }
            None => return Err(StateRefusal::new(format!("'{what}' has no 'session'"))),
        };
        Ok(Self {
            id: text("id")?,
            start,
            invoke_id: text("invokeid")?,
            document: text("document")?,
            session,
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

/// The byte-string field `name` of a saved record `value`, read back only if it is
/// Latin-1 text of no more than the `capacity` bytes its schema bounds it by
/// ([`bounded_bytes`]) — a record never holds more, and a restored one must not be
/// the first to.
pub fn record_bounded_bytes(
    value: &Value,
    record: &str,
    name: &str,
    capacity: usize,
) -> Result<Vec<u8>, StateRefusal> {
    bounded_bytes(
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

/// A byte string as a saved state writes it: its byte-exact Latin-1 text, each
/// byte the character of that code point (docs/adr/0005, decision 2) — the
/// spelling a `bytes` value takes on the wire, so one value is one text on every
/// backend, and a JSON reader that has no byte string reads it as a string.
pub fn bytes_to_saved(bytes: &[u8]) -> Value {
    Value::Text(bytes.iter().map(|&byte| char::from(byte)).collect())
}

/// A saved byte string `value`, read back only if it is Latin-1 text of no more
/// than the `capacity` bytes the machine bounds it by ([`bytes_to_saved`]) — a
/// machine never holds more, and a restored one must not be the first to. A
/// character past U+00FF is no byte, and is refused rather than truncated.
pub fn bounded_bytes(value: &Value, what: &str, capacity: usize) -> Result<Vec<u8>, StateRefusal> {
    let text = String::from_saved(value, what)?;
    let bytes = text
        .chars()
        .map(|c| {
            u8::try_from(u32::from(c)).map_err(|_| {
                StateRefusal::new(format!(
                    "'{what}' holds U+{:04X}, which is past U+00FF and so no byte",
                    u32::from(c)
                ))
            })
        })
        .collect::<Result<Vec<u8>, StateRefusal>>()?;
    if bytes.len() > capacity {
        return Err(StateRefusal::new(format!(
            "'{what}' holds {} bytes, past the {capacity} it is bounded by",
            bytes.len()
        )));
    }
    Ok(bytes)
}

/// The wall clock now, in milliseconds since the Unix epoch: what a host that
/// has no clock of its own to give [`save`] and [`enter`] gives them.
///
/// Absent on `wasm32-unknown-unknown`, which provides no wall clock to read. A
/// host there says what time it is — `save_at` and `restore_with` take it as an
/// argument — and the generated `save` / `restore` that would have read it here
/// are absent with it, so a call that could only have stopped the module is a
/// compile error instead.
// Reads the OS wall clock, and is absent where there is none (clippy.toml).
#[allow(clippy::disallowed_methods)]
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
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

/// Save `engine`, a machine of the document whose saved state is bound to
/// `shape`, with the `variables` its generated code read from its fields. The
/// configuration is written in document order, and every waiting delayed
/// `<send>` is written as the moment it comes due on the wall clock whose
/// reading now is `wall_now_ms` (module docs).
///
/// A machine any of whose record schemas is past its first revision names each
/// such revision in `revisions` and writes the revised shape; any other writes
/// the first shape and no `revisions`.
///
/// The state is format 2 (docs/adr/0045): the machine's own id, how many times
/// each `<invoke>` has started, and `children`, each running child with its
/// whole state, which the generated machine saved through the child's own
/// document, in document order. The shape is the format-2 one, which names an
/// enum by its document.
///
/// Refused for a machine that is not running — never started, or ended at a
/// top-level `<final>`, where there is nothing left to resume — for one whose
/// last macrostep stopped at the microstep ceiling: its configuration is not a
/// settled one, and a restore would resume it as if it were — for one with no
/// id (a machine on a target with no source of randomness whose host gave it
/// none), and when `children` are not the children the machine is running.
pub fn save<P: StatePolicy>(
    engine: &Engine<P>,
    shape: &SavedShape,
    variables: Vec<(String, Value)>,
    history: Vec<(String, Vec<String>)>,
    children: Vec<SavedChild>,
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
    let session_id = engine.policy().own_session_id().to_string();
    if session_id.is_empty() {
        return Err(StateRefusal::new(
            "the machine has no session id: on a target with no source of randomness its \
             host gives one when it creates the machine",
        ));
    }
    // What the generated machine saved is every child it runs, and nothing
    // else: one left out would restore a parent whose child is gone.
    let running = engine.policy().running_invokes();
    let saved_ids: Vec<&str> = children.iter().map(|child| child.id.as_str()).collect();
    if saved_ids != running {
        return Err(StateRefusal::new(format!(
            "the machine runs the children {running:?} and the save holds {saved_ids:?}, a \
             defect of its generated save"
        )));
    }
    // Read before anything is built, so a delayed send this state cannot carry
    // refuses the save instead of leaving it out.
    let pending = save_pending(engine, wall_now_ms)?;
    // Document order, so the same machine saves the same text on every
    // backend.
    let mut active = engine.get_active_states();
    active.sort_by_key(|s| P::get_document_order(*s));
    // The revision of each record schema past its first, whose digest list is
    // empty exactly when the schema is in its first revision.
    // The record schemas first, then the enums, each in the order the generator
    // gave them, so the same machine writes the same text on every backend.
    let records = shape
        .records
        .iter()
        .map(|record| (record.event, record.revision, record.revisions));
    let enums = shape
        .enums
        .iter()
        .map(|lineage| (lineage.key, lineage.revision, lineage.revisions));
    let revisions: Vec<(String, SavedRevision)> = records
        .chain(enums)
        .filter_map(|(key, revision, digests)| {
            let fields = digests.last()?;
            Some((
                key.to_string(),
                SavedRevision {
                    revision,
                    fields: (*fields).to_string(),
                },
            ))
        })
        .collect();
    Ok(SavedState {
        shape: if revisions.is_empty() {
            shape.format2_first
        } else {
            shape.format2_revised
        }
        .to_string(),
        revisions,
        configuration: active
            .iter()
            .map(|s| P::get_state_name(*s).to_string())
            .collect(),
        current: P::get_state_name(engine.get_current_state()).to_string(),
        variables,
        history,
        pending,
        // §scxml-6.4: the children that are running, each with its whole
        // state, which a restore resumes.
        invokes: SavedInvokes::Resumed(children),
        session_id,
        invoke_starts: engine
            .policy()
            .invoke_starts()
            .into_iter()
            .map(|(id, count)| (id.to_string(), count))
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
/// A send to the parent or to a child of this session is held as the act that
/// delivers it to the session the restored tree holds there (docs/adr/0045). A
/// send the format cannot carry — one routed to a session by its id — refuses
/// the save. A document that makes one is generated without the save API, so
/// no generated machine reaches this; the refusal is for a machine built by
/// hand, where leaving the entry out would restore a machine that never
/// delivers it and say nothing.
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
                // docs/adr/0045: the parent and a child of this session are
                // sessions the saved tree holds.
                ScheduledAct::Routed {
                    event_data,
                    send_id,
                    origin,
                    route: ScheduledRoute::Parent { event_name },
                    ..
                } => SavedAct::Parent {
                    event: event_name.clone(),
                    data: event_data.clone(),
                    send_id: send_id.clone(),
                    origin: origin.clone(),
                },
                ScheduledAct::Routed {
                    event_data,
                    send_id,
                    origin,
                    route:
                        ScheduledRoute::Invocation {
                            invoke_id,
                            event_name,
                        },
                    ..
                } => SavedAct::Child {
                    invoke_id: invoke_id.clone(),
                    event: event_name.clone(),
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

/// `saved` as a state of the machine whose saved state is bound to `shape`, or
/// the refusal that says why it is not one (docs/adr/0044) — judged before any
/// variable is read, as [`check_shape`] is.
///
/// A state that names no revision is of the first shape; one that does is of
/// the revised shape, and each revision it names is of a schema a record
/// variable of this machine is of, no later than this machine's revision of it,
/// and declares what this machine's revision of that number declares (the
/// digests agree). Such a state is then read as one of this machine: each record
/// of a schema saved at an earlier revision — a record variable, or each element
/// of a list of records — takes the default of every field a later revision
/// added. A record that already holds such a field is refused, since the
/// revision it was saved at does not declare it. What is not a record where a
/// record is held is left for the variable's own read to refuse.
///
/// The events of the external queue and of the waiting sends hold their data as
/// the wire carried it, and the lift reads it with the defaults when each is
/// delivered, so nothing of them is grown here.
///
/// A format-2 state is judged against the format-2 shapes, which name an enum
/// by its document rather than by its alias (docs/adr/0045, section 2), and a
/// format-1 state against the format-1 ones, exactly as before.
pub fn upgrade(saved: &SavedState, shape: &SavedShape) -> Result<SavedState, StateRefusal> {
    let (first, revised) = match saved.format() {
        FORMAT_1 => (shape.first, shape.revised),
        _ => (shape.format2_first, shape.format2_revised),
    };
    if saved.revisions.is_empty() {
        check_shape(saved, first)?;
    } else if saved.shape == first {
        return Err(StateRefusal::new(
            "the saved state names revisions of its record schemas and has the shape of a \
             state that names none",
        ));
    } else {
        check_shape(saved, revised)?;
    }
    for (key, named) in &saved.revisions {
        // A record schema by the event it declares, an enum by its key.
        let (revision, digests) = shape
            .records
            .iter()
            .find(|record| record.event == key.as_str())
            .map(|record| (record.revision, record.revisions))
            .or_else(|| {
                shape
                    .enums
                    .iter()
                    .find(|lineage| lineage.key == key.as_str())
                    .map(|lineage| (lineage.revision, lineage.revisions))
            })
            .ok_or_else(|| {
                StateRefusal::new(format!(
                    "'revisions' names '{key}', which no record or enum of this machine is"
                ))
            })?;
        if named.revision > revision {
            return Err(StateRefusal::new(format!(
                "the saved state was saved at revision {} of '{key}', later than this \
                 machine's {revision}: a machine does not restore what it does not know",
                named.revision
            )));
        }
        // The digest of revision r is the (r - 2)th: the first has none.
        let digest = named
            .revision
            .checked_sub(2)
            .and_then(|index| digests.get(index as usize))
            .copied();
        if digest != Some(named.fields.as_str()) {
            return Err(StateRefusal::new(format!(
                "revision {} of '{key}' declared other members where the state was saved than \
                 it does in this machine",
                named.revision
            )));
        }
    }
    // An enum value saved at an earlier revision of its enum is one of that
    // revision's variants. Judged on the values as saved, before a record is
    // grown by defaults, which are this machine's.
    for lineage in shape.enums {
        let at = saved
            .revisions
            .iter()
            .find(|(key, _)| key.as_str() == lineage.key)
            .map_or(1, |(_, named)| named.revision);
        let later: Vec<&AddedVariant> = lineage.added.iter().filter(|v| v.since > at).collect();
        if later.is_empty() {
            continue;
        }
        for holder in lineage.holders {
            check_enum_holder(saved, lineage, holder, at, &later)?;
        }
    }
    let mut grown = saved.clone();
    for record in shape.records {
        let at = saved
            .revisions
            .iter()
            .find(|(event, _)| event.as_str() == record.event)
            .map_or(1, |(_, named)| named.revision);
        let added: Vec<&AddedField> = record.added.iter().filter(|f| f.since > at).collect();
        if added.is_empty() {
            continue;
        }
        for holder in record.holders {
            let Some((_, value)) = grown
                .variables
                .iter_mut()
                .find(|(id, _)| id.as_str() == holder.variable)
            else {
                continue;
            };
            match value {
                Value::Array(items) if holder.list => {
                    for (i, item) in items.iter_mut().enumerate() {
                        grow_record(
                            item,
                            &format!("{}[{i}]", holder.variable),
                            record,
                            at,
                            &added,
                        )?;
                    }
                }
                _ if holder.list => {}
                _ => grow_record(value, holder.variable, record, at, &added)?,
            }
        }
    }
    Ok(grown)
}

/// Refuse a value `holder` holds, in `saved`, that names one of `later`, the
/// variants of `lineage` added after `at`, the revision it was saved at. What is
/// not a text where one is held is left for the variable's own read to refuse.
fn check_enum_holder(
    saved: &SavedState,
    lineage: &EnumLineage,
    holder: &EnumHolder,
    at: u32,
    later: &[&AddedVariant],
) -> Result<(), StateRefusal> {
    let Ok(value) = saved.variable(holder.variable) else {
        return Ok(());
    };
    // Each value the holder holds, with what names it in a refusal.
    let held: Vec<(String, &Value)> = match (holder.field, holder.list, value) {
        (None, _, value) => vec![(holder.variable.to_string(), value)],
        (Some(field), false, record) => record
            .member(field)
            .map(|v| (format!("{}.{field}", holder.variable), v))
            .into_iter()
            .collect(),
        (Some(field), true, Value::Array(items)) => items
            .iter()
            .enumerate()
            .filter_map(|(i, record)| {
                record
                    .member(field)
                    .map(|v| (format!("{}[{i}].{field}", holder.variable), v))
            })
            .collect(),
        (Some(_), true, _) => Vec::new(),
    };
    for (what, value) in held {
        let Value::Text(name) = value else {
            continue;
        };
        if let Some(variant) = later.iter().find(|v| v.name == name.as_str()) {
            return Err(StateRefusal::new(format!(
                "'{what}' holds '{name}', which revision {at} of '{}' does not declare: \
                 revision {} added it",
                lineage.key, variant.since
            )));
        }
    }
    Ok(())
}

/// Give the saved record `value`, of `record` saved at revision `at`, the
/// default of each of `added`, the fields later revisions added; `what` names it
/// in a refusal. A value that is not an object is left for the record's own read
/// to refuse.
fn grow_record(
    value: &mut Value,
    what: &str,
    record: &RecordLineage,
    at: u32,
    added: &[&AddedField],
) -> Result<(), StateRefusal> {
    let Value::Object(members) = value else {
        return Ok(());
    };
    for field in added {
        if members.iter().any(|(name, _)| name.as_str() == field.id) {
            return Err(StateRefusal::new(format!(
                "'{what}' holds '{}', which revision {at} of '{}' does not declare: revision {} \
                 added it",
                field.id, record.event, field.since
            )));
        }
        let default = json::parse(field.default).map_err(|e| {
            StateRefusal::new(format!(
                "the default of '{}' of '{}' is {e}",
                field.id, record.event
            ))
        })?;
        members.push((field.id.to_string(), default));
    }
    Ok(())
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
///
/// A format-1 state's running children are started again from their
/// beginning, last, so what a child sends as it starts stands behind what was
/// already waiting. A format-2 state's are judged with the rest before anything
/// is built ([`judge`]), and resumed, each where it stood, once this session
/// stands where it did ([`StatePolicy::resume_child`]).
pub fn enter<P: StatePolicy>(
    policy: P,
    saved: &SavedState,
    clock: SceClock,
    wall_now_ms: u64,
) -> Result<Engine<P>, StateRefusal> {
    let judged = judge_session::<P>(saved)?;
    // The session carries on under the id it was saved with; a format-1 state
    // holds none, and the machine keeps the one it was created with.
    let mut policy = policy;
    if let Some(id) = saved.bound_session_id() {
        policy.bind_session_id(id);
    }
    let mut engine = Engine::new(policy);
    engine.set_clock(clock);
    engine
        .enter_at(&judged.configuration, judged.current)
        .map_err(|rejection| StateRefusal::new(describe::<P>(&rejection)))?;
    for queued in judged.external {
        engine.raise_external_with_meta(queued);
    }
    arm_pending(&mut engine, judged.pending, wall_now_ms);
    rearm_host_invokes(&mut engine, &saved.host_invokes, wall_now_ms);
    engine.set_next_host_invoke_token(saved.host_invoke_token);
    engine.set_auto_send_seq(saved.auto_send_seq);
    match &saved.invokes {
        // Last, so what a child sends as it starts stands behind what was
        // already waiting: the saved machine's queue was ahead of it.
        SavedInvokes::Restarted(_) => engine.restart_invokes(&judged.restarts),
        SavedInvokes::Resumed(children) => engine.resume_children(children, wall_now_ms)?,
    }
    Ok(engine)
}

/// A saved session read against the document of `P`, before anything is built.
struct Judged<'s, P: StatePolicy> {
    configuration: crate::helpers::hierarchy::StateChain<P::State>,
    current: P::State,
    external: Vec<EventWithMetadata<P::Event, P::Payload>>,
    pending: Vec<ReadSend<'s, P::Event>>,
    /// The `<invoke>`s a format-1 state lists, each to be started again.
    restarts: Vec<&'s str>,
}

/// Refuse `saved` unless it can be a state of the document of `P`, building
/// nothing (docs/adr/0045, section 3, step 1): its configuration, its queue,
/// its waiting sends, its running and host-run invocations, its count of
/// starts, and each running child's session against the document that child
/// runs ([`StatePolicy::judge_child`]), to any depth.
///
/// The shape, the revisions, the history and the variables are the generated
/// machine's to judge, before this ([`upgrade`], [`restore_history`]); a
/// generated `<Machine>Persist::judge` does both, and is what a parent's judge
/// of a tree calls for each child it holds.
pub fn judge<P: StatePolicy>(saved: &SavedState) -> Result<(), StateRefusal> {
    judge_session::<P>(saved).map(|_| ())
}

/// [`judge`], keeping what it read for [`enter`] to build from.
fn judge_session<P: StatePolicy>(saved: &SavedState) -> Result<Judged<'_, P>, StateRefusal> {
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
    crate::helpers::configuration::validate::<P>(&configuration, current)
        .map_err(|rejection| StateRefusal::new(describe::<P>(&rejection)))?;
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
    // twice would be a child the saved machine could not have had.
    let ids = saved.invokes.ids();
    for (i, id) in ids.iter().enumerate() {
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
        if ids[..i].contains(id) {
            return Err(StateRefusal::new(format!(
                "invokes[{i}] is '{id}', which an earlier entry already names"
            )));
        }
    }
    // A count of starts names an `<invoke>` of the document.
    for (id, _) in &saved.invoke_starts {
        if P::invoke_owner(id).is_none() {
            return Err(StateRefusal::new(format!(
                "invokestarts counts '{id}', which the document does not invoke"
            )));
        }
    }
    let restarts: Vec<&str> = match &saved.invokes {
        SavedInvokes::Restarted(ids) => ids.iter().map(String::as_str).collect(),
        SavedInvokes::Resumed(children) => {
            for (i, child) in children.iter().enumerate() {
                judge_child_entry::<P>(saved, child, i)?;
            }
            Vec::new()
        }
    };
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
    Ok(Judged {
        configuration,
        current,
        external,
        pending,
        restarts,
    })
}

/// Refuse `child`, the `index`th running child of the format-2 session
/// `parent`, unless it is the child that session could be running
/// (docs/adr/0045, section 3, step 1): its start is the count the session holds
/// for its `<invoke>`, its session id is the session's and the id it was
/// started under, and its own session is a state of the document it names
/// ([`StatePolicy::judge_child`]).
fn judge_child_entry<P: StatePolicy>(
    parent: &SavedState,
    child: &SavedChild,
    index: usize,
) -> Result<(), StateRefusal> {
    let what = format!("invokes[{index}]");
    let count = parent.invoke_start(&child.id);
    if child.start != count {
        return Err(StateRefusal::new(format!(
            "{what} is start {} of '{}', and the session counts {count} starts of it: a state \
             re-entered cancels the earlier child before it starts the next one",
            child.start, child.id
        )));
    }
    let derived = format!("{}.{}", parent.session_id, child.invoke_id);
    if child.session.session_id != derived {
        return Err(StateRefusal::new(format!(
            "{what}.session's id is '{}', and a child of '{}' started under '{}' is '{derived}'",
            child.session.session_id, parent.session_id, child.invoke_id
        )));
    }
    P::judge_child(child).map_err(|refusal| refusal.within(&format!("{what}.session")))
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
    /// An event for the parent or a child, which resolves its name itself.
    Routed {
        due: u64,
        route: crate::ScheduledRoute,
        data: &'s str,
        send_id: &'s str,
        origin: &'s str,
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
            SavedAct::Parent {
                event,
                data,
                send_id,
                origin,
            } => Self::Routed {
                due,
                route: crate::ScheduledRoute::Parent {
                    event_name: event.clone(),
                },
                data,
                send_id,
                origin,
            },
            // The child is named by the `<invoke>` the target names, which must
            // be one of the document's: a send to any other is one the save
            // would have been refused for.
            SavedAct::Child {
                invoke_id,
                event,
                data,
                send_id,
                origin,
            } => {
                if P::invoke_owner(invoke_id).is_none() {
                    return Err(StateRefusal::new(format!(
                        "pending[{index}] is a send to the child '{invoke_id}', which the \
                         document does not invoke"
                    )));
                }
                Self::Routed {
                    due,
                    route: crate::ScheduledRoute::Invocation {
                        invoke_id: invoke_id.clone(),
                        event_name: event.clone(),
                    },
                    data,
                    send_id,
                    origin,
                }
            }
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
            ReadSend::Routed {
                due,
                route,
                data,
                send_id,
                origin,
            } => {
                engine.scheduler.schedule_routed_at(
                    None,
                    ready_at(due),
                    send_id,
                    data,
                    origin,
                    route,
                );
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
            invokes: SavedInvokes::Restarted(vec!["worker".to_string(), "spare".to_string()]),
            session_id: String::new(),
            invoke_starts: Vec::new(),
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
            revisions: vec![(
                "reading.taken".to_string(),
                SavedRevision {
                    revision: 3,
                    fields: "0".repeat(64),
                },
            )],
        };
        let back = SavedState::from_json(&state.to_json()).expect("reads");
        assert_eq!(back, state);
        // With no revision past the first, the member is left out, and the text
        // is the one every state saved before schemas had revisions is.
        let first = SavedState {
            revisions: Vec::new(),
            ..state.clone()
        };
        assert!(
            !first.to_json().contains("revisions"),
            "{}",
            first.to_json()
        );
        assert_eq!(SavedState::from_json(&first.to_json()), Ok(first));
        assert_eq!(
            u64::from_saved(back.variable("big").unwrap(), "big"),
            Ok(u64::MAX)
        );
    }

    // ── a tree of sessions (format 2, docs/adr/0045) ──

    /// A session of shape `shape` and id `session_id` standing in `working`,
    /// running `children`, with nothing waiting.
    fn session(shape: char, session_id: &str, children: Vec<SavedChild>) -> SavedState {
        SavedState {
            shape: shape.to_string().repeat(64),
            configuration: vec!["working".to_string()],
            current: "working".to_string(),
            variables: vec![("completed".to_string(), 0u32.to_saved())],
            history: Vec::new(),
            pending: Vec::new(),
            invokes: SavedInvokes::Resumed(children),
            session_id: session_id.to_string(),
            invoke_starts: Vec::new(),
            host_invokes: Vec::new(),
            host_invoke_token: 0,
            auto_send_seq: 0,
            external: Vec::new(),
            revisions: Vec::new(),
        }
    }

    /// A root running a child that runs a grandchild, with a send waiting each
    /// way across the first link.
    fn tree() -> SavedState {
        let grandchild = session('c', "planner-1.worker.busy.1.helper", Vec::new());
        let mut child = session(
            'b',
            "planner-1.worker",
            vec![SavedChild {
                id: "helper".to_string(),
                start: 1,
                invoke_id: "busy.1.helper".to_string(),
                document: "static_hybrid_holder".to_string(),
                session: grandchild,
            }],
        );
        child.invoke_starts = vec![("helper".to_string(), 1)];
        child.pending = vec![SavedSend {
            due: 1_700_000_005_000,
            act: SavedAct::Parent {
                event: "progress".to_string(),
                data: "{\"n\":1}".to_string(),
                send_id: "report".to_string(),
                origin: "planner-1.worker".to_string(),
            },
        }];
        let mut root = session(
            'a',
            "planner-1",
            vec![SavedChild {
                id: "worker".to_string(),
                start: 2,
                invoke_id: "worker".to_string(),
                document: "static_invoke_child".to_string(),
                session: child,
            }],
        );
        root.invoke_starts = vec![("worker".to_string(), 2)];
        root.pending = vec![SavedSend {
            due: 1_700_000_006_000,
            act: SavedAct::Child {
                invoke_id: "worker".to_string(),
                event: "nudge".to_string(),
                data: String::new(),
                send_id: "poke".to_string(),
                origin: "planner-1".to_string(),
            },
        }];
        root
    }

    #[test]
    fn a_tree_of_sessions_is_read_back_as_it_was_written() {
        let tree = tree();
        assert_eq!(tree.format(), FORMAT_2);
        let text = tree.to_json();
        // The format is stated once, at the root, and each session's id follows
        // its shape: the order every backend writes.
        let prefix = format!(
            r#"{{"format":2,"shape":"{}","sessionid":"planner-1","#,
            "a".repeat(64)
        );
        assert!(text.starts_with(&prefix), "{text}");
        assert_eq!(text.matches(r#""format""#).count(), 1, "{text}");
        assert!(text.contains(r#""invokestarts":{"worker":"2"}"#), "{text}");
        assert_eq!(SavedState::from_json(&text), Ok(tree.clone()));
        assert_eq!(tree.invokes.ids(), ["worker"]);
        assert_eq!(tree.invoke_start("worker"), 2);
        assert_eq!(
            tree.invoke_start("helper"),
            0,
            "counted in the child, not here"
        );
    }

    #[test]
    fn a_tree_that_is_not_one_is_refused_where_it_is_read() {
        let text = tree().to_json();
        for (what, from, to, wanted) in [
            (
                "a root with the empty id",
                r#""sessionid":"planner-1","#,
                r#""sessionid":"","#,
                "'sessionid' () is not an XML Nmtoken",
            ),
            (
                "a grandchild whose id holds a space",
                r#""sessionid":"planner-1.worker.busy.1.helper""#,
                r#""sessionid":"planner 1""#,
                "the session at invokes[0].session.invokes[0].session: 'sessionid'",
            ),
            (
                "a start counted from 0",
                r#""start":"2""#,
                r#""start":"0""#,
                "'invokes[0].start' (0) is not a count of starts",
            ),
            (
                "a child that names no document",
                r#""document":"static_invoke_child""#,
                r#""document":"""#,
                "'invokes[0].document' is empty",
            ),
            (
                "a count of starts written as a number",
                r#""invokestarts":{"worker":"2"}"#,
                r#""invokestarts":{"worker":2}"#,
                "'invokestarts.worker' is not a text",
            ),
            (
                "a waiting send of an act no session makes",
                r#""act":"child""#,
                r#""act":"sideways""#,
                "not raise, internal, host, parent or child",
            ),
        ] {
            let changed = text.replacen(from, to, 1);
            assert_ne!(changed, text, "{what}: the change did not apply");
            let refusal = SavedState::from_json(&changed).expect_err(what);
            assert!(refusal.reason().contains(wanted), "{what}: {refusal}");
        }
        // A child's session states no format of its own.
        let nested = text.replacen(
            r#""session":{"shape""#,
            r#""session":{"format":2,"shape""#,
            1,
        );
        let refusal = SavedState::from_json(&nested).expect_err("a nested format");
        assert!(refusal.reason().contains("states a format"), "{refusal}");
    }

    #[test]
    fn a_format_1_state_holds_no_send_to_a_parent_or_a_child() {
        let text = with_pending(
            r#"{"due":"1","act":"parent","event":"e","data":"","sendid":"","origin":""}"#,
        );
        let refusal = SavedState::from_json(&text).expect_err("not a format-1 act");
        assert!(
            refusal.reason().contains("the acts of format 1"),
            "{refusal}"
        );
    }

    #[test]
    fn a_refusal_inside_a_tree_names_the_session_by_its_path() {
        let refusal = StateRefusal::new("the saved state has no 'pending'")
            .within("invokes[1].session")
            .within("invokes[0].session");
        assert_eq!(
            refusal.reason(),
            "the session at invokes[0].session.invokes[1].session: the saved state has no \
             'pending'"
        );
    }

    // ── a state saved at an earlier revision of a record schema (docs/adr/0044) ──

    const FIRST_SHAPE: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const REVISED_SHAPE: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const FORMAT2_FIRST_SHAPE: &str =
        "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
    const FORMAT2_REVISED_SHAPE: &str =
        "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
    const SECOND: &str = "2222222222222222222222222222222222222222222222222222222222222222";
    const THIRD: &str = "3333333333333333333333333333333333333333333333333333333333333333";

    /// A machine holding a record `last` and a list `seen` of the schema of
    /// `reading.taken` at its third revision: `urgent` came with the second and
    /// `note` with the third.
    const READING_SHAPE: SavedShape = SavedShape {
        first: FIRST_SHAPE,
        revised: REVISED_SHAPE,
        // It holds no enum, so format 2 names its shape as format 1 does.
        format2_first: FIRST_SHAPE,
        format2_revised: REVISED_SHAPE,
        records: &[RecordLineage {
            event: "reading.taken",
            revision: 3,
            revisions: &[SECOND, THIRD],
            added: &[
                AddedField {
                    id: "urgent",
                    since: 2,
                    default: "true",
                },
                AddedField {
                    id: "note",
                    since: 3,
                    default: "7",
                },
            ],
            holders: &[
                RecordHolder {
                    variable: "last",
                    list: false,
                },
                RecordHolder {
                    variable: "seen",
                    list: true,
                },
            ],
        }],
        enums: &[],
    };

    /// A machine holding a variable `layout` of the enum `ViewMode` and a list
    /// `seen` of records whose field `layout` is of it, at the enum's second
    /// revision, which added `agenda`.
    const LAYOUT_SHAPE: SavedShape = SavedShape {
        first: FIRST_SHAPE,
        revised: REVISED_SHAPE,
        format2_first: FORMAT2_FIRST_SHAPE,
        format2_revised: FORMAT2_REVISED_SHAPE,
        records: &[],
        enums: &[EnumLineage {
            key: "enum:enum_view_mode.scxml",
            revision: 2,
            revisions: &[SECOND],
            added: &[AddedVariant {
                name: "agenda",
                since: 2,
            }],
            holders: &[
                EnumHolder {
                    variable: "layout",
                    field: None,
                    list: false,
                },
                EnumHolder {
                    variable: "seen",
                    field: Some("layout"),
                    list: true,
                },
            ],
        }],
    };

    /// A saved state of shape `shape` holding `layout` and `seen`, with the
    /// `revisions` member `revisions` (empty for none).
    fn layout_state(shape: &str, layout: &str, seen: &str, revisions: &str) -> SavedState {
        let text = format!(
            r#"{{"format":1,"shape":"{shape}","configuration":["s"],"current":"s","variables":{{"layout":"{layout}","seen":{seen}}},"history":{{}},"pending":[],"invokes":[],"hostinvokes":[],"hostinvoketoken":"0","sendseq":"0","external":[]{revisions}}}"#
        );
        SavedState::from_json(&text).expect("reads")
    }

    #[test]
    fn a_state_saved_at_an_earlier_revision_of_an_enum_restores_as_it_was() {
        let saved = layout_state(FIRST_SHAPE, "week", r#"[{"layout":"day"}]"#, "");
        assert_eq!(upgrade(&saved, &LAYOUT_SHAPE), Ok(saved));
        // At the enum's own revision, the variant it added is a value like any.
        let saved = layout_state(
            REVISED_SHAPE,
            "agenda",
            r#"[{"layout":"agenda"}]"#,
            &format!(
                r#","revisions":{{"enum:enum_view_mode.scxml":{{"revision":2,"fields":"{SECOND}"}}}}"#
            ),
        );
        assert_eq!(upgrade(&saved, &LAYOUT_SHAPE), Ok(saved));
    }

    #[test]
    fn a_value_an_enum_had_not_yet_declared_when_saved_is_refused() {
        let reason = |saved: SavedState| {
            upgrade(&saved, &LAYOUT_SHAPE)
                .expect_err("refused")
                .reason()
                .to_string()
        };
        let variable = reason(layout_state(FIRST_SHAPE, "agenda", "[]", ""));
        assert!(variable.contains("'layout' holds 'agenda'"), "{variable}");
        let element = reason(layout_state(
            FIRST_SHAPE,
            "week",
            r#"[{"layout":"day"},{"layout":"agenda"}]"#,
            "",
        ));
        assert!(
            element.contains("'seen[1].layout' holds 'agenda'"),
            "{element}"
        );
        // Saved by a build whose enum is past this one's.
        let later = reason(layout_state(
            REVISED_SHAPE,
            "week",
            "[]",
            &format!(
                r#","revisions":{{"enum:enum_view_mode.scxml":{{"revision":3,"fields":"{THIRD}"}}}}"#
            ),
        ));
        assert!(later.contains("later than"), "{later}");
    }

    /// A saved state of shape `shape` holding `last` and `seen`, with the
    /// `revisions` member `revisions` written after the others (empty for none).
    fn reading_state(shape: &str, last: &str, seen: &str, revisions: &str) -> SavedState {
        let text = format!(
            r#"{{"format":1,"shape":"{shape}","configuration":["s"],"current":"s","variables":{{"last":{last},"seen":{seen}}},"history":{{}},"pending":[],"invokes":[],"hostinvokes":[],"hostinvoketoken":"0","sendseq":"0","external":[]{revisions}}}"#
        );
        SavedState::from_json(&text).expect("reads")
    }

    /// The `revisions` member naming `revision` of `reading.taken` with `fields`.
    fn named(revision: u32, fields: &str) -> String {
        format!(r#","revisions":{{"reading.taken":{{"revision":{revision},"fields":"{fields}"}}}}"#)
    }

    fn json_of(text: &str) -> Value {
        json::parse(text).expect("JSON")
    }

    #[test]
    fn a_state_saved_at_the_first_revision_takes_every_later_default() {
        let saved = reading_state(
            FIRST_SHAPE,
            r#"{"level":4}"#,
            r#"[{"level":1},{"level":2}]"#,
            "",
        );
        let grown = upgrade(&saved, &READING_SHAPE).expect("an earlier revision restores");
        assert_eq!(
            grown.variable("last"),
            Ok(&json_of(r#"{"level":4,"urgent":true,"note":7}"#))
        );
        assert_eq!(
            grown.variable("seen"),
            Ok(&json_of(
                r#"[{"level":1,"urgent":true,"note":7},{"level":2,"urgent":true,"note":7}]"#
            ))
        );
    }

    #[test]
    fn a_state_saved_at_a_later_revision_keeps_what_it_holds_and_takes_the_rest() {
        let saved = reading_state(
            REVISED_SHAPE,
            r#"{"level":4,"urgent":false}"#,
            "[]",
            &named(2, SECOND),
        );
        let grown = upgrade(&saved, &READING_SHAPE).expect("the second revision restores");
        assert_eq!(
            grown.variable("last"),
            Ok(&json_of(r#"{"level":4,"urgent":false,"note":7}"#))
        );

        // At the machine's own revision nothing is added.
        let saved = reading_state(
            REVISED_SHAPE,
            r#"{"level":4,"urgent":false,"note":2}"#,
            "[]",
            &named(3, THIRD),
        );
        assert_eq!(upgrade(&saved, &READING_SHAPE), Ok(saved));
    }

    #[test]
    fn a_state_of_another_revision_or_another_shape_is_refused() {
        let reason = |saved: SavedState| {
            upgrade(&saved, &READING_SHAPE)
                .expect_err("refused")
                .reason()
                .to_string()
        };
        // Saved by a later build than this one.
        let later = reading_state(REVISED_SHAPE, r#"{"level":4}"#, "[]", &named(4, THIRD));
        assert!(reason(later).contains("later than"));
        // The two builds disagree about what the second revision declares.
        let other = reading_state(REVISED_SHAPE, r#"{"level":4}"#, "[]", &named(2, THIRD));
        assert!(reason(other).contains("declared other members"));
        // A record that holds a field its revision does not declare.
        let early = reading_state(FIRST_SHAPE, r#"{"level":4,"urgent":true}"#, "[]", "");
        let refusal = reason(early);
        assert!(refusal.contains("'last' holds 'urgent'"), "{refusal}");
        let early_element = reading_state(
            FIRST_SHAPE,
            r#"{"level":4}"#,
            r#"[{"level":1,"note":3}]"#,
            "",
        );
        assert!(reason(early_element).contains("'seen[0]' holds 'note'"));
        // Revisions named under the first shape, and none under the revised one.
        let mixed = reading_state(FIRST_SHAPE, r#"{"level":4}"#, "[]", &named(2, SECOND));
        assert!(reason(mixed).contains("names revisions"));
        let unnamed = reading_state(REVISED_SHAPE, r#"{"level":4}"#, "[]", "");
        assert!(reason(unnamed).contains("of shape"));
        // A schema no record of the machine is of.
        let stranger = reading_state(
            REVISED_SHAPE,
            r#"{"level":4}"#,
            "[]",
            &format!(r#","revisions":{{"other.event":{{"revision":2,"fields":"{SECOND}"}}}}"#),
        );
        assert!(reason(stranger).contains("no record"));
    }

    #[test]
    fn a_revisions_member_that_is_not_one_is_refused_where_it_is_read() {
        for (revisions, wanted) in [
            (r#","revisions":{}"#.to_string(), "names no schema"),
            (named(1, SECOND), "past the first"),
            (named(2, "abc"), "SHA-256"),
            (r#","revisions":[]"#.to_string(), "not an object"),
        ] {
            let text = format!(
                r#"{{"format":1,"shape":"{FIRST_SHAPE}","configuration":["s"],"current":"s","variables":{{}},"history":{{}},"pending":[],"invokes":[],"hostinvokes":[],"hostinvoketoken":"0","sendseq":"0","external":[]{revisions}}}"#
            );
            let refusal = SavedState::from_json(&text).expect_err("not a revisions member");
            assert!(refusal.reason().contains(wanted), "{revisions}: {refusal}");
        }
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
        for text in [
            include_str!("../../../../schemas/sce-saved-state.v1.schema.json"),
            include_str!("../../../../schemas/sce-saved-state.v2.schema.json"),
        ] {
            let schema = json::parse(text).expect("the schema is JSON");
            assert_eq!(
                schema.member("x-sce-schema-status"),
                Some(&Value::Text(SCHEMA_STATUS.to_string())),
                "SCHEMA_STATUS and every format's schema header move together \
                 (SCE_WIRE_CONTRACTS.md)"
            );
        }
    }

    #[test]
    fn another_format_is_refused() {
        let text = r#"{"format":3,"shape":"d","configuration":[],"current":"s","variables":{},"history":{},"external":[]}"#;
        let refusal = SavedState::from_json(text).expect_err("format 3");
        assert!(
            refusal.reason().contains("reads formats 1 and 2"),
            "{refusal}"
        );
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

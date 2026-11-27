// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! `<send type="sce:journal">`: a fact a keyed session says, made an entry of its
//! event journal by the write that keeps its state (docs/adr/0114, D1).
//!
//! ```xml
//! <send type="sce:journal" event="room.booked">
//!   <param name="room" expr="room"/>
//!   <param name="guests" expr="guests"/>
//! </send>
//! ```
//!
//! The send is of the event of an event schema the document imports, its
//! `<param>`s the fields of the schema's record, typed and lowered as a send of a
//! declared schema's event to another machine is. It does two things in the
//! macrostep that runs it: it makes an entry, the record's deterministic CBOR with
//! the schema's id (its event name) and revision and the valid time the schema marks
//! (`sce:valid-time="from"`, D5), which the engine keeps until the host core's write
//! of the checkpoint appends it; and it puts the event with the entry's data on the
//! session's internal queue, as `<raise>` does, so the transitions that apply the
//! fact run before that checkpoint.
//!
//! The build does three things with it:
//!
//! - **It refuses what an entry cannot be** ([`resolve`]): the send under any data
//!   model but `sce-static`; one that names its event by an expression, names a
//!   target, a delay, an id, a type expression, a `namelist` or a `<content>`; an
//!   event of no schema the document imports; a record with no deterministic CBOR;
//!   and a field the schema does not make optional that no `<param>` gives, or one
//!   two give. Each is `scxml/static-datamodel-rule`, on the send.
//! - **It writes what the entry is made by** onto the send
//!   (`Action::journal_record`, `journal_revision`, `journal_valid_time`), which
//!   the Rust backend's send reads.
//! - **It refuses a lane that keeps no journal** ([`refuse_unkept`]): the Rust host
//!   core keeps one; every other backend, and the Interpreter, refuses the machine by
//!   name, as does a Rust machine whose record the build could not describe, which
//!   a document parsed without its imports is.

use std::collections::BTreeMap;

use crate::forge::error::{ForgeError, GenerateError, Located};
use crate::forge::payload_record::describe;
use crate::forge::standard_host::HostLane;
use crate::forge::value_type::ValueType;
use crate::generator::Language;
use crate::host_processor_analyzer::{visit_actions_mut, walk_model_actions};
use crate::model::{Action, Datamodel, SCXMLModel};
use crate::scxml_semantic::ScxmlSemanticError;

/// The `type` of a send that says a fact into the session's journal.
pub const SEND_TYPE: &str = "sce:journal";

/// The lanes whose machine keeps an event journal (docs/adr/0114, Scope): the
/// Rust host core's.
pub const JOURNAL_LANES: &[HostLane] = &[HostLane::Generated(Language::Rust)];

/// How a refusal names the send.
const CONSTRUCT: &str = "<send type=\"sce:journal\">";

/// Whether `action` is a send that says a fact into the session's journal.
pub fn is_journal_send(action: &Action) -> bool {
    action.action_type == "send" && action.send_type == SEND_TYPE
}

/// The event of the first send of `model` that says a fact into its journal, in
/// the order the states are written; `None` for a document that says none.
pub fn first_journal_event(model: &SCXMLModel) -> Option<String> {
    let mut first = None;
    walk_model_actions(model, &mut |_state, action| {
        if first.is_none() && is_journal_send(action) {
            first = Some(action.event.clone());
        }
    });
    first
}

/// The events `model` says facts of with a journal send, each once.
pub fn journal_events(model: &SCXMLModel) -> std::collections::BTreeSet<String> {
    let mut events = std::collections::BTreeSet::new();
    walk_model_actions(model, &mut |_state, action| {
        if is_journal_send(action) {
            events.insert(action.event.clone());
        }
    });
    events
}

/// The description of the record of `event`, the event of an event schema `model`
/// imports, by which the runtime's record codec reads an entry of it into the data
/// the event carries (docs/adr/0114, D1 and D4): what a follower is delivered an
/// entry it follows by. `None` when no schema `model` imports declares `event`;
/// the reason, when its record has no deterministic CBOR (docs/adr/0101, section 2).
pub fn entry_record(model: &SCXMLModel, event: &str) -> Option<Result<String, String>> {
    let alias = model
        .imported_records
        .iter()
        .find(|(_, schema)| schema.event_name == event)
        .map(|(alias, _)| alias.clone())?;
    Some(describe(
        &ValueType::Record { alias },
        &model.imported_type_scope,
    ))
}

/// What the entry of a fact of one event is made by.
#[derive(Debug, Clone, PartialEq, Eq)]
struct EntryOf {
    /// The description of the record's type the runtime's record codec writes it by.
    record: String,
    /// The revision of the schema, which writes the record.
    revision: u32,
    /// The field the schema marks its valid time, if one.
    valid_time: Option<String>,
}

/// A refusal of one send: the rule it breaks, the attribute it stands on, and
/// the text it reads there.
struct Refused {
    rule: String,
    attr: Option<&'static str>,
    observed: Option<String>,
}

impl Refused {
    fn on(attr: &'static str, observed: &str, rule: impl Into<String>) -> Self {
        Self {
            rule: rule.into(),
            attr: Some(attr),
            observed: (!observed.is_empty()).then(|| observed.to_string()),
        }
    }

    fn whole(rule: impl Into<String>) -> Self {
        Self {
            rule: rule.into(),
            attr: None,
            observed: Some(SEND_TYPE.to_string()),
        }
    }
}

/// Judge every journal send of `model` against the schemas its imports resolved
/// to, and write onto each what its entry is made by.
///
/// Run where the imports are resolved, beside the other judgements of what an
/// imported schema is asked to carry.
pub fn resolve(model: &mut SCXMLModel, diag_label: &str) -> Result<(), Located<ForgeError>> {
    let mut entries: BTreeMap<String, EntryOf> = BTreeMap::new();
    let mut refusal: Option<Located<ForgeError>> = None;
    {
        let read: &SCXMLModel = model;
        walk_model_actions(read, &mut |state, action| {
            if refusal.is_some() || !is_journal_send(action) {
                return;
            }
            match judge(read, action) {
                Ok(entry) => {
                    entries.insert(action.event.clone(), entry);
                }
                Err(refused) => refusal = Some(locate(read, state, action, refused, diag_label)),
            }
        });
    }
    if let Some(refused) = refusal {
        return Err(refused);
    }
    visit_actions_mut(model, &mut |action| {
        if !is_journal_send(action) {
            return;
        }
        if let Some(entry) = entries.get(&action.event) {
            action.journal_record = entry.record.clone();
            action.journal_revision = Some(entry.revision);
            action.journal_valid_time = entry.valid_time.clone().unwrap_or_default();
        }
    });
    Ok(())
}

/// What the entry of the fact `action` says is made by, or why it has none.
fn judge(model: &SCXMLModel, action: &Action) -> Result<EntryOf, Refused> {
    if model.datamodel != Datamodel::SceStatic {
        return Err(Refused::whole(
            "an entry of the event journal is a record of a declared schema, typed where the \
             document is read, which only datamodel=\"sce-static\" types (docs/adr/0114, D1)",
        ));
    }
    if !action.eventexpr.is_empty() || action.event.is_empty() {
        return Err(Refused::on(
            "eventexpr",
            &action.eventexpr,
            "the send names its event by `event`, the event of a schema the document imports, \
             so the entry's schema is known where the document is read",
        ));
    }
    let written: [(&'static str, &str, &str); 8] = [
        (
            "target",
            action.target.as_str(),
            "an entry's stream is the session that says it (docs/adr/0114, D2), so the send \
             names no target",
        ),
        (
            "targetexpr",
            action.targetexpr.as_str(),
            "an entry's stream is the session that says it (docs/adr/0114, D2), so the send \
             names no target",
        ),
        (
            "typeexpr",
            action.typeexpr.as_str(),
            "the send's type is written: `sce:journal`",
        ),
        (
            "delay",
            action.delay.as_str(),
            "an entry is made by the macrostep that says it and kept with the state it made, so \
             it is not delayed",
        ),
        (
            "delayexpr",
            action.delayexpr.as_str(),
            "an entry is made by the macrostep that says it and kept with the state it made, so \
             it is not delayed",
        ),
        (
            "id",
            action.id.as_str(),
            "an entry is never cancelled, and a correction is an entry of its own, so the send \
             takes no id",
        ),
        (
            "idlocation",
            action.idlocation.as_str(),
            "an entry is never cancelled, and a correction is an entry of its own, so the send \
             takes no id",
        ),
        (
            "namelist",
            action.namelist.as_str(),
            "an entry's fields are each given by a <param> named as the schema's field",
        ),
    ];
    if let Some((attr, value, rule)) = written.into_iter().find(|(_, value, _)| !value.is_empty()) {
        return Err(Refused::on(attr, value, rule));
    }
    if !action.content.is_empty() || !action.contentexpr.is_empty() {
        return Err(Refused::whole(
            "an entry's fields are each given by a <param> named as the schema's field, and a \
             <content> gives none",
        ));
    }
    let event = action.event.as_str();
    let alias = model
        .imported_records
        .iter()
        .find(|(_, schema)| schema.event_name == event)
        .map(|(alias, _)| alias.clone());
    let (Some(schema), Some(alias)) = (model.imported_event_schemas.get(event), alias) else {
        return Err(Refused::on(
            "event",
            event,
            format!(
                "`{event}` is the event of no event schema the document imports, and an entry \
                 is a record of its schema"
            ),
        ));
    };
    let record =
        describe(&ValueType::Record { alias }, &model.imported_type_scope).map_err(|why| {
            Refused::on(
                "event",
                event,
                format!(
                    "the record of `{event}` has no deterministic CBOR an entry could hold \
                 (docs/adr/0101, section 2): {why}"
                ),
            )
        })?;
    let mut given = std::collections::BTreeSet::new();
    for param in &action.params {
        if !given.insert(param.name.as_str()) {
            return Err(Refused::whole(format!(
                "the field `{}` is given by two <param>s, and an entry holds one value of it",
                param.name
            )));
        }
    }
    if let Some(field) = schema.fields.iter().find(|field| {
        !matches!(field.sce_type, ValueType::Optional { .. }) && !given.contains(field.id.as_str())
    }) {
        return Err(Refused::whole(format!(
            "the field `{}` of `{event}` is given by no <param>, and an entry holds every \
             field its schema does not make optional",
            field.id
        )));
    }
    Ok(EntryOf {
        record,
        revision: schema.revision.unwrap_or(1),
        valid_time: schema
            .fields
            .iter()
            .find(|field| field.valid_time)
            .map(|field| field.id.clone()),
    })
}

/// `refused`, placed on the attribute it stands on, or on the send.
fn locate(
    model: &SCXMLModel,
    state: &str,
    action: &Action,
    refused: Refused,
    diag_label: &str,
) -> Located<ForgeError> {
    let (line, col) = match refused.attr.and_then(|attr| action.spellings.get(attr)) {
        Some(spelling) => (Some(spelling.row()), Some(spelling.col())),
        None => (
            action.source_location.as_ref().and_then(|at| at.line),
            action.source_location.as_ref().and_then(|at| at.col),
        ),
    };
    Located::new(
        ScxmlSemanticError::StaticDatamodelRule {
            construct: CONSTRUCT.to_string(),
            datamodel: model.datamodel.as_str().to_string(),
            rule: refused.rule,
            state: state.to_string(),
            observed: refused.observed,
        }
        .into(),
        diag_label,
        line,
        col,
    )
}

/// Whether a transition on `event` applies a fact: each of its descriptors is the
/// event of an imported schema, named exactly, so the transition takes no event
/// that is no entry's. An entry is always a record of a declared schema
/// (docs/adr/0114, D1); a deployment that runs the machine narrows the events to
/// the entries the machine says and follows.
fn applies_a_fact(model: &SCXMLModel, event: &str) -> bool {
    let mut descriptors = event.split_whitespace().peekable();
    descriptors.peek().is_some()
        && descriptors.all(|descriptor| {
            !descriptor.contains('*') && model.imported_event_schemas.contains_key(descriptor)
        })
}

/// Refuse a statement of `model` that writes a value held to the journal's facts
/// (`<data sce:from="journal">`, docs/adr/0114, D1), or a place inside one, where
/// no fact is applied: in an `<onentry>`, an `<onexit>`, an initial transition, a
/// history default, or a transition whose event is not that of an imported schema
/// named exactly ([`applies_a_fact`]). Its initial value is its own declaration's.
/// So the value is a fold of the entries the session said or took in, in their
/// order, which the same entries given to a fresh session compute again.
pub fn refuse_journal_values_written_elsewhere(
    model: &SCXMLModel,
    diag_label: &str,
) -> Result<(), Located<ForgeError>> {
    let held: Vec<&crate::model::Variable> =
        model.variables.iter().filter(|v| v.from_journal).collect();
    if held.is_empty() {
        return Ok(());
    }
    let mut states: Vec<_> = model.states.values().collect();
    states.sort_by_key(|state| state.document_order);
    for state in states {
        for transition in &state.transitions {
            if applies_a_fact(model, &transition.event) {
                continue;
            }
            let site = if transition.event.trim().is_empty() {
                format!("a transition of state '{}' that takes no event", state.id)
            } else {
                format!(
                    "the transition of state '{}' on '{}', which is no event of an imported \
                     schema named exactly",
                    state.id,
                    transition.event.trim()
                )
            };
            unwritten(&transition.actions, &held, &site, diag_label)?;
        }
        for block in &state.on_entry_blocks {
            unwritten(
                block,
                &held,
                &format!("the <onentry> of state '{}'", state.id),
                diag_label,
            )?;
        }
        for block in &state.on_exit_blocks {
            unwritten(
                block,
                &held,
                &format!("the <onexit> of state '{}'", state.id),
                diag_label,
            )?;
        }
        unwritten(
            &state.initial_transition_actions,
            &held,
            &format!("the initial transition of state '{}'", state.id),
            diag_label,
        )?;
        unwritten(
            &state.initial_history_default_actions,
            &held,
            &format!("the history default of state '{}'", state.id),
            diag_label,
        )?;
    }
    Ok(())
}

/// Each place `action` itself writes that is one of `held` or inside it, with
/// the statement, the attribute and the variable: every place a statement
/// writes ([`crate::forge::static_datamodel::places_written`]), what an
/// `<sce:call>` returns into its `location`, and a map or a set of `held` the
/// call hands an algorithm by name, which writes it in place.
fn held_places<'a>(
    action: &'a Action,
    held: &[&crate::model::Variable],
) -> Vec<(&'static str, &'static str, &'a str, String)> {
    let mut places = crate::forge::static_datamodel::places_written(action);
    if action.action_type == "sce_call" {
        places.push(("<sce:call>", "location", action.location.as_str()));
        let arguments = action
            .expr
            .split_once('(')
            .and_then(|(_, rest)| rest.rsplit_once(')'))
            .map_or("", |(arguments, _)| arguments);
        for argument in arguments.split(',').map(str::trim) {
            let in_place = held.iter().any(|var| {
                var.id == argument
                    && var
                        .value_type
                        .as_ref()
                        .is_some_and(|ty| ty.map_key().is_some())
            });
            if in_place {
                places.push(("<sce:call>", "expr", argument));
            }
        }
    }
    places
        .into_iter()
        .filter_map(|(element, attr, location)| {
            let root = location
                .trim()
                .split(['.', '['])
                .next()
                .unwrap_or("")
                .trim();
            held.iter()
                .find(|var| !root.is_empty() && var.id == root)
                .map(|var| (element, attr, location, var.id.clone()))
        })
        .collect()
}

/// The first value held to the journal's facts that `actions` write, at any
/// depth; `None` when they write none. What a deployment that runs the machine
/// holds to the entries the machine says and follows
/// (`crate::mesh::topology::check_journal`).
pub fn first_held_write(model: &SCXMLModel, actions: &[Action]) -> Option<String> {
    let held: Vec<&crate::model::Variable> =
        model.variables.iter().filter(|v| v.from_journal).collect();
    if held.is_empty() {
        return None;
    }
    fn walk(actions: &[Action], held: &[&crate::model::Variable]) -> Option<String> {
        actions.iter().find_map(|action| {
            held_places(action, held)
                .into_iter()
                .next()
                .map(|(_, _, _, value)| value)
                .or_else(|| {
                    action
                        .nested_blocks()
                        .into_iter()
                        .find_map(|block| walk(block.actions, held))
                })
        })
    }
    walk(actions, &held)
}

/// Refuse the first statement of `actions`, at any depth, that writes one of
/// `held` or a place inside it ([`held_places`]), standing in `site`.
fn unwritten(
    actions: &[Action],
    held: &[&crate::model::Variable],
    site: &str,
    diag_label: &str,
) -> Result<(), Located<ForgeError>> {
    for action in actions {
        if let Some((element, attr, location, value)) = held_places(action, held).into_iter().next()
        {
            let spelling = action.spellings.get(attr);
            return Err(Located::new(
                crate::forge::error::ValidationError::JournalValueAssigned {
                    element: element.to_string(),
                    attr: attr.to_string(),
                    location: location.trim().to_string(),
                    value,
                    site: site.to_string(),
                }
                .into(),
                diag_label,
                spelling.map(|s| s.row()),
                spelling.map(|s| s.col()),
            ));
        }
        for block in action.nested_blocks() {
            unwritten(block.actions, held, site, diag_label)?;
        }
    }
    Ok(())
}

/// The event of the first fact said into a journal by `model` or by a child it
/// can start, at any depth (docs/adr/0114, D1): its own sends first, then what
/// the parse recorded of each child it starts through an `<invoke type="scxml">`
/// and of each candidate of a hybrid one. `None` when the tree says none.
pub fn facts_said_in_tree(model: &SCXMLModel) -> Option<String> {
    first_journal_event(model).or_else(|| first_child_saying_facts(model).map(|facts| facts.event))
}

/// A child that says facts into a journal: its name, the event of its first
/// fact, and where the `<invoke>` that starts it stands.
struct ChildFacts {
    child: String,
    event: String,
    at: Option<crate::forge::error::SourceLocation>,
}

/// The first child `model` can start that says facts into a journal.
fn first_child_saying_facts(model: &SCXMLModel) -> Option<ChildFacts> {
    let mut states: Vec<_> = model.states.values().collect();
    states.sort_by_key(|state| state.document_order);
    states
        .into_iter()
        .flat_map(|state| &state.invokes)
        .find_map(|invoke| {
            let common = match invoke {
                crate::model::Invoke::Scxml(info) => &info.common,
                crate::model::Invoke::Hybrid(info) => &info.common,
                crate::model::Invoke::MeshRpc(_) | crate::model::Invoke::Unsupported(_) => {
                    return None
                }
            };
            let at = common.base.source_location.clone();
            if let Some(event) = &common.child_says_facts {
                return Some(ChildFacts {
                    child: common.child_name.clone(),
                    event: event.clone(),
                    at,
                });
            }
            match invoke {
                crate::model::Invoke::Hybrid(info) => {
                    info.candidates.iter().find_map(|candidate| {
                        candidate.child_says_facts.clone().map(|event| ChildFacts {
                            child: candidate.stem.clone(),
                            event,
                            at: at.clone(),
                        })
                    })
                }
                _ => None,
            }
        })
}

/// Refuse `model` when a child it starts locally (`<invoke type="scxml">`, a
/// candidate of a hybrid one among them) says facts into a journal, at any depth
/// (docs/adr/0114, D1): the entries a child makes are its own engine's, which no
/// host core's write takes, so they would be said and never kept. Only the
/// session of a keyed pool says facts. Placed on the `<invoke>` that starts it,
/// naming the child and its fact.
pub fn refuse_facts_of_children(model: &SCXMLModel) -> Result<(), GenerateError> {
    match first_child_saying_facts(model) {
        Some(ChildFacts { child, event, at }) => {
            Err(GenerateError::JournalFactsInChild { child, event, at })
        }
        None => Ok(()),
    }
}

/// Refuse `model` on `lane` when it says a fact into the journal and the lane
/// keeps none (docs/adr/0114, Scope), or when the Rust lane would make the entry of
/// a record the build could not describe: a send of a document read without its
/// imports, whose schema was never resolved.
pub fn refuse_unkept(model: &SCXMLModel, lane: HostLane) -> Result<(), GenerateError> {
    let mut refused: Option<GenerateError> = None;
    walk_model_actions(model, &mut |_state, action| {
        if refused.is_some() || !is_journal_send(action) {
            return;
        }
        let why = if !JOURNAL_LANES.contains(&lane) {
            format!(
                "the {} lane keeps no event journal; the Rust host core keeps one",
                lane.wire_name()
            )
        } else if action.journal_record.is_empty() {
            "the event's schema was not resolved, so the build has no description of the \
             record an entry holds: read the document with its imports"
                .to_string()
        } else {
            return;
        };
        refused = Some(GenerateError::unsupported_at(
            format!(
                "<send type=\"sce:journal\" event=\"{}\">: {why} (docs/adr/0114, D1)",
                action.event
            ),
            action.source_location.clone(),
        ));
    });
    refused.map_or(Ok(()), Err)
}

// ── A page of the event journal: `sce-host-journal-read` (docs/adr/0114, D5) ──
//
// ```xml
// <invoke type="sce-host-journal-read" id="page" sce:journal="rooms" sce:entries="Facts"
//         sce:request="JournalReadRequest" sce:result="JournalReadResult">
//   <param name="key" expr="room"/>
//   <param name="as_of_position" expr="as_of"/>
//   ...
// </invoke>
// ```
//
// The read names the stream it reads by `sce:journal` (a machine's: the
// document's own, or one its machine follows) or `sce:stream` (an observed
// stream it follows), and by `sce:entries` the imported union whose cases are
// the schemas of the facts it reads (docs/adr/0092). The build writes the
// request's `stream` and `observed` from the first ([`write_read_requests`]),
// derives the completion's record from the library's result with the second
// ([`derive_reads`]), and the Rust machine opens the page's bytes into that
// record where the completion is lifted (`sce_rust_runtime::journal::open_page`).
// Where the machine is generated with its deployment, a stream it neither keeps
// nor follows, and a union that holds no case of a schema the stream's facts
// are of, are refused (`crate::mesh::topology::check_journal_reads`).

/// The standard operation that reads a page of the event journal.
pub const JOURNAL_READ: &str = "sce-host-journal-read";

/// The fields of the read's request the build writes, from `sce:journal` or
/// `sce:stream`: the stream's name, and whether it is an observed one.
pub const READ_FIELDS_BY_BUILD: &[&str] = &[STREAM_FIELD, OBSERVED_FIELD];

/// The field of the request that names the stream.
pub const STREAM_FIELD: &str = "stream";

/// The field of the request that says whether the stream is an observed one.
pub const OBSERVED_FIELD: &str = "observed";

/// The field of the result that holds the page.
pub const ENTRIES_FIELD: &str = "entries";

/// The name of the bound of how many entries a page holds, which `--limit`
/// moves, and the number the library gives it.
pub const PAGE_LIMIT: &str = "journal_page";

/// The number of entries a page holds when no build moves [`PAGE_LIMIT`].
pub const PAGE_LIMIT_WRITTEN: u32 = 64;

/// How the completion of a read is opened where it is lifted (docs/adr/0114,
/// D5): the bounds of its page and the case of the union each schema's fact is.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct JournalReading {
    /// The most entries the page holds ([`PAGE_LIMIT`], as the build resolved it).
    pub page_entries: u32,
    /// The most bytes the page's deterministic CBOR holds (`journal_page_bytes`).
    pub page_bytes: u64,
    /// Each case of the union, as JSON: `[[event, tag, variant, description],
    /// ...]`, the schema's event a fact of it is kept under, the union's tag,
    /// the case's variant, and the description of its record the record codec
    /// reads the fact by ([`describe`]).
    pub cases: String,
    /// The completion the read's failure `not_a_record` is: a page whose bytes
    /// are no value of the page, or one with a fact no case holds.
    pub failure: String,
}

/// What an invoke of [`JOURNAL_READ`] says of the journal it reads, and what the
/// build derives of it (docs/adr/0114, D5): kept boxed on the invoke
/// (`crate::model::UnsupportedInvokeInfo::journal`), which names it only when it
/// writes `sce:journal`, `sce:stream` or `sce:entries`.
#[derive(Debug, Clone, Default)]
pub struct JournalReadInvoke {
    /// The machine whose journal, or the observed stream, the page is read of
    /// (`sce:journal` or `sce:stream`), which the build writes into the
    /// request's `stream`; empty when the invoke names none.
    pub stream: String,
    /// Whether [`Self::stream`] is an observed stream (`sce:stream`), written into
    /// the request's `observed`.
    pub observed: bool,
    /// Where `sce:journal` or `sce:stream` is spelled; `None` when neither is
    /// written.
    pub stream_at: Option<crate::forge::error::SourceLocation>,
    /// `sce:entries`: the alias of the imported union each fact of the page is a
    /// value of (docs/adr/0092).
    pub entries: String,
    /// Where `sce:entries`'s value is spelled.
    pub entries_at: Option<crate::forge::error::SourceLocation>,
    /// The alias of the record the build derives for the read's completion, which
    /// `done.invoke.<id>` is bound to ([`derive_reads`]); empty until derived.
    pub result: String,
    /// How that completion is opened where it is lifted; `None` until derived.
    pub reading: Option<JournalReading>,
}

/// What an invoke that writes none of the read's attributes says of the journal.
static NO_READ: JournalReadInvoke = JournalReadInvoke {
    stream: String::new(),
    observed: false,
    stream_at: None,
    entries: String::new(),
    entries_at: None,
    result: String::new(),
    reading: None,
};

/// What `info` says of the journal it reads, or [`NO_READ`].
fn read_of(info: &crate::model::UnsupportedInvokeInfo) -> &JournalReadInvoke {
    info.journal.as_deref().unwrap_or(&NO_READ)
}

/// A read of the event journal `model` makes, as the deployment judges it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JournalRead {
    /// The stream the read names.
    pub stream: String,
    /// Whether it is an observed stream (`sce:stream`).
    pub observed: bool,
    /// The events of the schemas the union's cases are records of.
    pub cases: std::collections::BTreeSet<String>,
    /// Where `sce:journal` or `sce:stream` is written.
    pub at: Option<crate::forge::error::SourceLocation>,
}

/// The host-run `<invoke>`s of `model`, in document order.
fn host_invokes(model: &SCXMLModel) -> Vec<&crate::model::UnsupportedInvokeInfo> {
    let mut states: Vec<_> = model.states.values().collect();
    states.sort_by_key(|state| state.document_order);
    states
        .into_iter()
        .flat_map(|state| state.invokes.iter())
        .filter_map(|invoke| match invoke {
            crate::model::Invoke::Unsupported(info) => Some(info),
            _ => None,
        })
        .collect()
}

/// Whether `name` may name a stream a read names: a letter, then letters,
/// digits, `_` or `-`, which a machine of a deployment and an observed stream
/// are named by.
fn is_journal_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars.next().is_some_and(|c| c.is_ascii_alphabetic())
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// A case of a read's union: the event of its record's schema, the union's
/// tag, the case's variant and the description of its record.
type UnionCase = (String, String, String, String);

/// The cases of the union `alias` of `model` ([`UnionCase`]), or why the union
/// holds no page's facts: it is not imported, a case is empty, two cases hold
/// one record, or a record has no deterministic CBOR.
fn union_cases(model: &SCXMLModel, alias: &str) -> Result<Vec<UnionCase>, String> {
    let Some(union) = model.imported_type_scope.unions.get(alias) else {
        return Err("names no union this document imports".to_string());
    };
    let mut cases = Vec::with_capacity(union.cases.len());
    let mut events = std::collections::BTreeSet::new();
    for case in &union.cases {
        let Some(record) = &case.record else {
            return Err(format!(
                "names {}, whose case `{}` holds no record, and a fact of the journal is a \
                 record of its schema",
                union.name, case.variant.name
            ));
        };
        let event = union
            .scope
            .records
            .get(record)
            .map(|scoped| scoped.lineage.event.clone())
            .unwrap_or_default();
        if event.is_empty() {
            return Err(format!(
                "names {}, whose case `{}` holds `{record}`, which is no event schema a fact \
                 is kept under",
                union.name, case.variant.name
            ));
        }
        if !events.insert(event.clone()) {
            return Err(format!(
                "names {}, two of whose cases hold the record of `{event}`, so a fact of it \
                 would be no one case",
                union.name
            ));
        }
        let description = describe(
            &ValueType::Record {
                alias: record.clone(),
            },
            &union.scope,
        )
        .map_err(|why| {
            format!(
                "names {}, whose case `{}` holds a record with no deterministic CBOR a fact \
                 is kept as (docs/adr/0101, section 2): {why}",
                union.name, case.variant.name
            )
        })?;
        cases.push((
            event,
            union.tag.clone(),
            case.variant.name.clone(),
            description,
        ));
    }
    Ok(cases)
}

/// Judge every read of the event journal of `model` (docs/adr/0114, D5): an
/// invoke of [`JOURNAL_READ`] names one stream, by `sce:journal` or
/// `sce:stream`, by a name, and by `sce:entries` an imported union whose every
/// case holds the record of one event schema, no two the same, each with a
/// deterministic CBOR; an invoke of any other type names none of the three.
/// Each refusal is `validation/journal-read-schema`, on the attribute.
///
/// Run where the imports are resolved, beside the other judgements of what an
/// imported schema is asked to carry.
pub fn resolve_reads(model: &SCXMLModel, diag_label: &str) -> Result<(), Located<ForgeError>> {
    for info in host_invokes(model) {
        let read = read_of(info);
        let refuse = |attr: &str,
                      value: &str,
                      at: &Option<crate::forge::error::SourceLocation>,
                      detail: String| {
            located(
                at.as_ref().or(info.base.source_location.as_ref()),
                diag_label,
                crate::forge::error::ValidationError::JournalReadSchema {
                    invoke_id: info.base.invoke_id.clone(),
                    attr: attr.to_string(),
                    value: value.to_string(),
                    detail,
                },
            )
        };
        let stream_attr = if read.observed {
            "sce:stream"
        } else {
            "sce:journal"
        };
        let names = read.stream_at.is_some() || read.entries_at.is_some();
        if info.invoke_type != JOURNAL_READ {
            if !names {
                continue;
            }
            let (attr, value, at) = if read.stream_at.is_some() {
                (stream_attr, &read.stream, &read.stream_at)
            } else {
                ("sce:entries", &read.entries, &read.entries_at)
            };
            return Err(refuse(
                attr,
                value,
                at,
                format!(
                    "is written on an <invoke> of type \"{}\", and only {JOURNAL_READ} reads a \
                     page of the event journal",
                    info.invoke_type
                ),
            ));
        }
        if read.stream_at.is_none() {
            return Err(refuse(
                "sce:journal",
                "",
                &None,
                format!(
                    "a {JOURNAL_READ} names the stream it reads, by sce:journal (a machine's \
                     journal) or sce:stream (an observed stream), and this one names none"
                ),
            ));
        }
        if !is_journal_name(&read.stream) {
            return Err(refuse(
                stream_attr,
                &read.stream,
                &read.stream_at,
                "names no stream: a machine and an observed stream are named by a letter, \
                 then letters, digits, `_` or `-`"
                    .to_string(),
            ));
        }
        if read.entries_at.is_none() || read.entries.is_empty() {
            return Err(refuse(
                "sce:entries",
                &read.entries,
                &read.entries_at,
                format!(
                    "a {JOURNAL_READ} names by sce:entries the union each fact it reads is a \
                     value of (docs/adr/0092), and this one names none"
                ),
            ));
        }
        union_cases(model, &read.entries)
            .map_err(|why| refuse("sce:entries", &read.entries, &read.entries_at, why))?;
    }
    Ok(())
}

/// A field of a record the build derives, of `ty`.
fn derived_field(id: &str, ty: ValueType) -> crate::forge::model::SchemaField {
    crate::forge::model::ForgeField {
        id: id.to_string(),
        sce_type: ty,
        direction: crate::forge::model::Direction::In,
        expr: None,
        expr_spelling: None,
        expr_splices: None,
        quantity: None,
        max_size: None,
        limit: None,
        default_covers: Vec::new(),
        retain: None,
        initial: None,
        initial_spelling: None,
        since: None,
        optional_since: None,
        default: None,
        default_spelling: None,
        sensitivity: None,
        valid_time: false,
    }
}

/// Derive the completion's record of every read of the event journal of
/// `model`, judged by [`resolve_reads`] (docs/adr/0114, D5): an entry record,
/// `JournalEntryOf<Invoke>`, of each entry's `position`, `recorded_ms`,
/// `holds_ms` (when the fact holds from), `origin` (the id another system gave
/// it, empty for a session's) and `fact`, a value of the read's union; and the
/// library's result whose `entries` is a list of at most [`PAGE_LIMIT`] of
/// them, which `done.invoke.<id>` is bound to. Writes how the page is opened
/// ([`JournalReading`]) onto the invoke.
pub fn derive_reads(model: &mut SCXMLModel, diag_label: &str) -> Result<(), Located<ForgeError>> {
    use crate::forge::value_type::{Bound, ScopedRecord, TypeScope};
    struct Derived {
        invoke_id: String,
        result_alias: String,
        entry_alias: String,
        entry_schema: crate::forge::model::EventSchemaModel,
        entry_scoped: ScopedRecord,
        result_schema: crate::forge::model::EventSchemaModel,
        result_scoped: ScopedRecord,
        reading: JournalReading,
    }
    let mut derived: Vec<Derived> = Vec::new();
    for info in host_invokes(model) {
        let read = read_of(info);
        if info.invoke_type != JOURNAL_READ || read.entries.is_empty() {
            continue;
        }
        let (Some(result), Some(scoped)) = (
            model.imported_records.get(&info.result_schema),
            model.imported_type_scope.records.get(&info.result_schema),
        ) else {
            // A result the library does not give is refused where the
            // standard type is held to its records.
            continue;
        };
        let Some(union) = model.imported_type_scope.unions.get(&read.entries) else {
            continue;
        };
        let Some(page_bytes) = result.fields.iter().find_map(|field| {
            match (&field.sce_type, field.id == ENTRIES_FIELD) {
                (ValueType::Bytes { max }, true) => Some(max.max),
                _ => None,
            }
        }) else {
            continue;
        };
        let suffix = crate::filters::to_pascal_case(info.base.field_suffix.clone());
        let snake = crate::filters::to_snake_case(info.base.field_suffix.clone());
        let entry_alias = format!("JournalEntryOf{suffix}");
        let result_alias = format!("{}Of{suffix}", info.result_schema);
        for alias in [&entry_alias, &result_alias] {
            if model.imported_records.contains_key(alias) {
                return Err(located(
                    read.entries_at
                        .as_ref()
                        .or(info.base.source_location.as_ref()),
                    diag_label,
                    crate::forge::error::ValidationError::JournalReadSchema {
                        invoke_id: info.base.invoke_id.clone(),
                        attr: "sce:entries".to_string(),
                        value: read.entries.clone(),
                        detail: format!(
                            "the completion's records are named `{entry_alias}` and \
                             `{result_alias}`, and the document imports another record as \
                             `{alias}`: rename the import"
                        ),
                    },
                ));
            }
        }
        let cases = union_cases(model, &read.entries).unwrap_or_default();
        let page_entries = crate::limits::resolve(PAGE_LIMIT, PAGE_LIMIT_WRITTEN);
        let entry_fields: Vec<(String, ValueType)> = vec![
            (
                "position".to_string(),
                ValueType::Scalar(crate::forge::model::SceType::Uint64),
            ),
            (
                "recorded_ms".to_string(),
                ValueType::Scalar(crate::forge::model::SceType::Uint64),
            ),
            (
                "holds_ms".to_string(),
                ValueType::Scalar(crate::forge::model::SceType::Int64),
            ),
            (
                "origin".to_string(),
                ValueType::Text {
                    max: Bound::of(page_bytes),
                },
            ),
            (
                "fact".to_string(),
                ValueType::Union {
                    alias: read.entries.clone(),
                },
            ),
        ];
        let entry_schema = crate::forge::model::EventSchemaModel {
            name: format!("journal_entry_{snake}"),
            event_name: format!("sce.journal.entry.{}", info.base.invoke_id),
            fields: entry_fields
                .iter()
                .map(|(id, ty)| derived_field(id, ty.clone()))
                .collect(),
            payload_none: false,
            datamodel_markers: Vec::new(),
            revision: None,
            source_location: None,
        };
        let mut entry_scope = TypeScope::default();
        entry_scope
            .unions
            .insert(read.entries.clone(), union.clone());
        let entry_scoped = ScopedRecord {
            identity: format!("sce:journal-entry#{}", info.base.invoke_id),
            name: entry_schema.name.clone(),
            fields: entry_fields,
            scope: entry_scope,
            added: Default::default(),
            lineage: Default::default(),
        };
        let page = ValueType::List {
            elem: Box::new(ValueType::Record {
                alias: entry_alias.clone(),
            }),
            max: Bound {
                max: page_entries,
                limit: Some(PAGE_LIMIT.to_string()),
                min: 0,
            },
        };
        let mut result_schema = result.clone();
        result_schema.name = format!("{}_{snake}", result.name);
        result_schema.event_name = format!("done.invoke.{}", info.base.invoke_id);
        for field in &mut result_schema.fields {
            if field.id == ENTRIES_FIELD {
                field.sce_type = page.clone();
                field.max_size = None;
                field.limit = None;
            }
        }
        let mut result_scope = scoped.scope.clone();
        result_scope
            .records
            .insert(entry_alias.clone(), entry_scoped.clone());
        let result_scoped = ScopedRecord {
            identity: format!("{}#{}", scoped.identity, info.base.invoke_id),
            name: result_schema.name.clone(),
            fields: scoped
                .fields
                .iter()
                .map(|(id, ty)| {
                    if id == ENTRIES_FIELD {
                        (id.clone(), page.clone())
                    } else {
                        (id.clone(), ty.clone())
                    }
                })
                .collect(),
            scope: result_scope,
            added: scoped.added.clone(),
            lineage: Default::default(),
        };
        let failure: serde_json::Map<String, serde_json::Value> = result_schema
            .fields
            .iter()
            .map(|field| {
                let zero = if field.id == "error" {
                    serde_json::Value::String(
                        crate::forge::payload_record::NOT_A_RECORD.to_string(),
                    )
                } else {
                    crate::forge::payload_record::zero_of(&field.sce_type)
                };
                (field.id.clone(), zero)
            })
            .collect();
        let cases_json: Vec<serde_json::Value> = cases
            .into_iter()
            .map(|(event, tag, variant, description)| {
                let description: serde_json::Value =
                    serde_json::from_str(&description).unwrap_or(serde_json::Value::Null);
                serde_json::json!([event, tag, variant, description])
            })
            .collect();
        derived.push(Derived {
            invoke_id: info.base.invoke_id.clone(),
            result_alias,
            entry_alias,
            entry_schema,
            entry_scoped,
            result_schema,
            result_scoped,
            reading: JournalReading {
                page_entries,
                page_bytes: u64::from(page_bytes),
                cases: serde_json::Value::Array(cases_json).to_string(),
                failure: serde_json::Value::Object(failure).to_string(),
            },
        });
    }
    for one in derived {
        model
            .imported_records
            .insert(one.entry_alias.clone(), one.entry_schema);
        model
            .imported_type_scope
            .records
            .insert(one.entry_alias, one.entry_scoped);
        model
            .imported_records
            .insert(one.result_alias.clone(), one.result_schema);
        model
            .imported_type_scope
            .records
            .insert(one.result_alias.clone(), one.result_scoped);
        for state in model.states.values_mut() {
            for invoke in &mut state.invokes {
                if let crate::model::Invoke::Unsupported(info) = invoke {
                    if info.base.invoke_id != one.invoke_id || info.invoke_type != JOURNAL_READ {
                        continue;
                    }
                    if let Some(journal) = info.journal.as_deref_mut() {
                        journal.result = one.result_alias.clone();
                        journal.reading = Some(one.reading.clone());
                    }
                }
            }
        }
    }
    Ok(())
}

/// Write the `stream` and `observed` of every host-served read of the event
/// journal: literal `<param>`s of the stream its `sce:journal` or `sce:stream`
/// names. Run where the build decides which invokes a host serves
/// (`crate::host_processor_analyzer::declare_host_surfaces`), so the
/// parameters are the build's: the document's own were refused where its
/// request was judged.
pub fn write_read_requests(model: &mut SCXMLModel) {
    for state in model.states.values_mut() {
        for invoke in &mut state.invokes {
            let crate::model::Invoke::Unsupported(info) = invoke else {
                continue;
            };
            if info.invoke_type != JOURNAL_READ || !info.host_served {
                continue;
            }
            let Some((stream, observed, stream_at)) = info
                .journal
                .as_deref()
                .filter(|read| read.stream_at.is_some())
                .map(|read| (read.stream.clone(), read.observed, read.stream_at.clone()))
            else {
                continue;
            };
            let at = stream_at.or_else(|| info.base.source_location.clone());
            let observed = if observed { "true" } else { "false" };
            for (name, expr, value) in [
                (STREAM_FIELD, format!("'{stream}'"), stream.clone()),
                (OBSERVED_FIELD, observed.to_string(), observed.to_string()),
            ] {
                if info.base.params.iter().any(|param| param.name == name) {
                    continue;
                }
                info.base.params.push(crate::model::Param {
                    name: name.to_string(),
                    expr,
                    is_static_literal: true,
                    static_value: value,
                    source_location: at.clone(),
                    ..crate::model::Param::default()
                });
            }
        }
    }
}

/// How the completion `event` is opened where it is lifted, for the
/// completion of a read of the event journal; `None` for any other event.
pub fn reading_of<'m>(model: &'m SCXMLModel, event: &str) -> Option<&'m JournalReading> {
    host_invokes(model)
        .into_iter()
        .find(|info| format!("done.invoke.{}", info.base.invoke_id) == event)
        .and_then(|info| read_of(info).reading.as_ref())
}

/// The reads of the event journal `model` makes, each with the events its
/// union's cases are of, as the deployment judges them
/// (`crate::mesh::topology::check_journal_reads`).
pub fn reads(model: &SCXMLModel) -> Vec<JournalRead> {
    host_invokes(model)
        .into_iter()
        .filter(|info| info.invoke_type == JOURNAL_READ && read_of(info).stream_at.is_some())
        .map(|info| JournalRead {
            stream: read_of(info).stream.clone(),
            observed: read_of(info).observed,
            cases: union_cases(model, &read_of(info).entries)
                .unwrap_or_default()
                .into_iter()
                .map(|(event, ..)| event)
                .collect(),
            at: read_of(info).stream_at.clone(),
        })
        .collect()
}

fn located(
    at: Option<&crate::forge::error::SourceLocation>,
    diag_label: &str,
    error: crate::forge::error::ValidationError,
) -> Located<ForgeError> {
    Located::new(
        error.into(),
        diag_label,
        at.and_then(|l| l.line),
        at.and_then(|l| l.col),
    )
}

#[cfg(test)]
mod documents;

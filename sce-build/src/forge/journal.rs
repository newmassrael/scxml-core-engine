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

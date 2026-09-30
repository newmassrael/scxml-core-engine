// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A closed interface: `<scxml sce:interface="closed">` (SCE Accepted Subset
//! §2.16).
//!
//! An event-schema types the payload of the one event it names, and an
//! event no imported schema names keeps the dynamic `_event.data` baseline
//! with no diagnostic. That is right for W3C conformance and stays the
//! default. It also means nothing holds a statechart to the interface its
//! owner accepted: measured 2026-09-29, five drafts of one specification
//! whose prose left the interface open invented five interfaces — where a
//! price came from, what an input was called, which fields it carried — and
//! all five sent their outputs to themselves, where nobody receives them.
//!
//! A statechart that declares its interface closed is held to its imported
//! schemas instead, in both directions:
//!
//! - every event a transition takes that does not come from the statechart
//!   itself or from the platform (`error.*`, `done.state.*`,
//!   `done.invoke.*`) matches an event a schema declares;
//! - every `<send>` that leaves the session sends an event a schema
//!   declares, and names it literally — an `eventexpr` cannot be checked;
//! - every `<send>` the session addresses to itself sends an event some
//!   transition takes, since otherwise it is a message the machine sends
//!   itself and discards, which is what an output with no destination looks
//!   like.
//!
//! Judged where the imports are read (the parser's import seam), so it runs
//! for every entry point that parses a document from a file. The in-memory
//! path, which reads no sibling documents, cannot resolve a schema and does
//! not judge the declaration, as it does not judge the typed payload paths.

use std::path::Path;

use crate::event_descriptor::EventDescriptor;
use crate::forge::error::{ForgeError, Located, SourceLocation};
use crate::forge::model::ForgeKind;
use crate::host_processor_analyzer::walk_model_actions;
use crate::model::SCXMLModel;
use crate::scxml_self_send::{is_discarded, sends_to_itself};
use crate::scxml_semantic::{InterfaceCrossing, ScxmlSemanticError};

/// Refuse the first event that crosses a closed interface undeclared. A
/// document that does not declare its interface closed passes untouched.
///
/// `base_dir` is the directory the imports resolve against, for
/// [`refuse_unread_schemas`].
pub fn validate(
    model: &SCXMLModel,
    base_dir: &Path,
    diag_label: &str,
) -> Result<(), Located<ForgeError>> {
    if !model.interface_closed {
        return Ok(());
    }
    refuse_unread_schemas(model, base_dir, diag_label)?;
    let declared = |descriptor: &EventDescriptor| {
        model
            .imported_event_schemas
            .keys()
            .any(|event| descriptor.matches(event))
    };

    // What the statechart gives itself: every event it puts on its internal
    // queue (captured at parse time, `<finalize>` included) and every `<send>`
    // addressed to itself.
    let mut own: Vec<String> = model.internal_queue_events.iter().cloned().collect();
    walk_model_actions(model, &mut |_state, action| {
        if action.action_type == "send" && action.eventexpr.is_empty() && sends_to_itself(action) {
            own.push(action.event.clone());
        }
    });
    let from_itself = |descriptor: &EventDescriptor| own.iter().any(|e| descriptor.matches(e));

    // Received: every descriptor of every transition, in document order.
    let mut states: Vec<_> = model.states.values().collect();
    states.sort_by_key(|s| s.document_order);
    for state in &states {
        for transition in &state.transitions {
            for token in transition.event.split_whitespace() {
                let descriptor = EventDescriptor::parse(token);
                let Some(prefix) = descriptor.prefix() else {
                    // `*` takes whatever arrives; it declares nothing.
                    continue;
                };
                if crate::analyzer::is_reserved_ingress_event(prefix)
                    || from_itself(&descriptor)
                    || declared(&descriptor)
                {
                    continue;
                }
                return Err(refuse(
                    model,
                    diag_label,
                    InterfaceCrossing::Receives,
                    token,
                    &state.id,
                    transition.source_location.as_ref(),
                ));
            }
        }
    }

    // Sent: every `<send>`, leaving the session or addressed to itself.
    let mut refusal: Option<(
        InterfaceCrossing,
        String,
        String,
        Option<crate::forge::error::SourceLocation>,
    )> = None;
    walk_model_actions(model, &mut |state, action| {
        if refusal.is_some() || action.action_type != "send" {
            return;
        }
        let at = action.source_location.clone();
        if !action.eventexpr.is_empty() {
            refusal = Some((
                InterfaceCrossing::SendsComputed,
                action.eventexpr.clone(),
                state.to_string(),
                at,
            ));
        } else if sends_to_itself(action) {
            // The lint's reading, so a closed document and an open one
            // cannot disagree about which self-send is lost.
            if is_discarded(model, action) {
                refusal = Some((
                    InterfaceCrossing::SendsToItself,
                    action.event.clone(),
                    state.to_string(),
                    at,
                ));
            }
        } else if !model.imported_event_schemas.contains_key(&action.event) {
            refusal = Some((
                InterfaceCrossing::Sends,
                action.event.clone(),
                state.to_string(),
                at,
            ));
        }
    });
    match refusal {
        None => Ok(()),
        Some((crossing, event, state, at)) => Err(refuse(
            model,
            diag_label,
            crossing,
            &event,
            &state,
            at.as_ref(),
        )),
    }
}

/// Refuse a closed interface whose declared event-schema could not be read,
/// in that schema's own words.
///
/// ⚠ The interface is judged against the schemas the imports resolved to,
/// and this seam resolves them quietly ([`crate::forge::import_source::parse_quietly`]):
/// an import that is missing, unreadable or refused simply is not there.
/// Judging on regardless says the imports declare nothing — measured
/// 2026-09-30, "the document imports no event-schema" over a document that
/// imports four, every one refused for a reason the author was never told,
/// because this refusal came first and the import enrichment that reports it
/// never ran. Here is the one place that knows both facts, so it says the
/// one that is true: what is wrong with the schema.
///
/// An import that reads and parses but is not an event-schema is the
/// enrichment's to name (`KindMismatch`), and is left to it.
fn refuse_unread_schemas(
    model: &SCXMLModel,
    base_dir: &Path,
    diag_label: &str,
) -> Result<(), Located<ForgeError>> {
    for import in &model.forge_imports {
        if !matches!(import.kind, ForgeKind::EventSchema)
            || model.imported_records.contains_key(&import.alias)
        {
            continue;
        }
        let at = SourceLocation {
            file: diag_label.to_string(),
            line: import.line,
            col: None,
        };
        let source = crate::forge::import_source::ImportSource::read(base_dir, import)
            .map_err(|error| model.locate(ForgeError::Import(error), Some(&at), diag_label))?;
        // The refusal carries the imported document's own file and row.
        source.parse()?;
    }
    Ok(())
}

fn refuse(
    model: &SCXMLModel,
    diag_label: &str,
    crossing: InterfaceCrossing,
    event: &str,
    state: &str,
    at: Option<&crate::forge::error::SourceLocation>,
) -> Located<ForgeError> {
    let err = ForgeError::Scxml(Box::new(ScxmlSemanticError::UndeclaredInterfaceEvent {
        crossing,
        event: event.to_string(),
        state: state.to_string(),
        declared: model.imported_event_schemas.keys().cloned().collect(),
    }));
    model.locate(err, at.or(model.source_location.as_ref()), diag_label)
}

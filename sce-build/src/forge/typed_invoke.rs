// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A host-run `<invoke>`'s typed interface — `sce:request` / `sce:result`
//! (SCE Accepted Subset §2.12) — held to the event schemas it names.
//!
//! The parser refuses the attributes where they cannot mean anything (an
//! invoke SCE runs itself, one without an `id`, an empty alias). What it
//! cannot judge is whether an alias names a schema, because imports are
//! resolved afterwards; this runs at the seam that resolves them, beside the
//! other typed-path validators, against the same map.
//!
//! A typed request is a record: the invoke's `<param>`s are that record's
//! fields, one each, so the host receives exactly one value per field and no
//! other parameter. Anything that would make the request something else — a
//! `<param>` the schema lacks, a field no `<param>` supplies, one name given
//! twice, or a `namelist` / `<content>` beside it — is refused here, on the
//! row that shows it, rather than reaching a host as a record missing a field.
//! The reserved deadline param is the engine's and is not part of the record.

use std::collections::{BTreeMap, BTreeSet};

use crate::forge::error::{ForgeError, Located, SourceLocation, ValidationError};
use crate::forge::model::{EventSchemaModel, SceType};
use crate::host_processor_analyzer::HOST_INVOKE_DEADLINE_PARAM;
use crate::model::{Invoke, SCXMLModel, UnsupportedInvokeInfo};

/// Validate every typed host-run `<invoke>` in `model` against
/// `imported_records`, the alias → schema map its imports resolved to.
pub fn validate(
    model: &SCXMLModel,
    imported_records: &BTreeMap<String, EventSchemaModel>,
    diag_label: &str,
) -> Result<(), Located<ForgeError>> {
    // The per-state lists, in document order: they are authoritative, and
    // the model-wide `invokes` view is only rebuilt from them after the
    // parse — this runs before that, at the import seam, where it is empty.
    let mut states: Vec<_> = model.states.values().collect();
    states.sort_by_key(|s| s.document_order);
    for invoke in states.iter().flat_map(|s| s.invokes.iter()) {
        let Invoke::Unsupported(info) = invoke else {
            continue;
        };
        for (attr, alias, at) in [
            ("request", &info.request_schema, &info.request_schema_at),
            ("result", &info.result_schema, &info.result_schema_at),
        ] {
            if alias.is_empty() {
                continue;
            }
            let refuse = |detail: String| {
                located(
                    at,
                    diag_label,
                    ValidationError::TypedInvokeSchema {
                        invoke_id: info.base.invoke_id.clone(),
                        attr: attr.to_string(),
                        alias: alias.clone(),
                        detail,
                    },
                )
            };
            let Some(schema) = imported_records.get(alias) else {
                return Err(refuse(format!(
                    "names no event schema this document imports — declare \
                     <sce:import kind=\"event-schema\" as=\"{alias}\" src=\"…\"/>"
                )));
            };
            // The record crosses to and from the host as text in every
            // backend, and an enumeration has no spelling there that all six
            // share: its generated type is per-language, and the payload
            // lift that reads a completion refuses it for the same reason.
            if let Some(field) = schema
                .fields
                .iter()
                .find(|f| matches!(f.sce_type, SceType::Enum(_)))
            {
                return Err(refuse(format!(
                    "field '{}' is enum-typed ({}), and a host-run record carries \
                     scalar fields only — declare it as its underlying integer type",
                    field.id,
                    field.sce_type.as_attr()
                )));
            }
        }
        if let Some(schema) = imported_records.get(&info.request_schema) {
            check_request(info, schema, diag_label)?;
        }
    }
    Ok(())
}

/// What every host-run completion's name begins with (§scxml-6.3.1).
const DONE_INVOKE_PREFIX: &str = "done.invoke.";

/// Bind each typed invoke's completion, `done.invoke.<id>`, to the schema
/// its `sce:result` names, in `model.imported_event_schemas` — the map every
/// typed-payload consumer reads, so a guard on the completion is checked
/// against the record and lowered natively like any schema'd event.
///
/// Bound by the invoke, not by an event name: an event schema may not name a
/// built-in event (`validation/event-schema-on-builtin-event`), which is why a
/// `done.invoke.*` key can only have come from here. The generic `done.invoke`
/// stays untyped — several invokes share it. Run after [`validate`], so every
/// alias it reads is known to resolve.
pub fn bind_results(model: &mut SCXMLModel) {
    let mut bindings = Vec::new();
    for state in model.states.values() {
        for invoke in &state.invokes {
            let Invoke::Unsupported(info) = invoke else {
                continue;
            };
            if let Some(schema) = model.imported_records.get(&info.result_schema) {
                let event = format!("{DONE_INVOKE_PREFIX}{}", info.base.invoke_id);
                let mut bound = schema.clone();
                bound.event_name = event.clone();
                bindings.push((event, bound));
            }
        }
    }
    model.imported_event_schemas.extend(bindings);
}

/// Whether `event` is a host-run invocation's completion bound to its result
/// schema by [`bind_results`].
///
/// Such an event has a typed payload but no typed inject seam: the engine
/// accepts a host-run completion only through `complete_host_invoke`, which
/// checks that the start is still running, so a seam that raised one directly
/// would raise an event the engine refuses.
pub fn is_completion_binding(event: &str) -> bool {
    event.starts_with(DONE_INVOKE_PREFIX)
}

/// The invoke's `<param>`s against the fields of `schema`, one each.
fn check_request(
    info: &UnsupportedInvokeInfo,
    schema: &EventSchemaModel,
    diag_label: &str,
) -> Result<(), Located<ForgeError>> {
    let refuse = |at: &Option<SourceLocation>, detail: String, observed: &str| {
        located(
            at,
            diag_label,
            ValidationError::TypedInvokeRequest {
                invoke_id: info.base.invoke_id.clone(),
                alias: info.request_schema.clone(),
                detail,
                observed: observed.to_string(),
            },
        )
    };
    // A typed request is one record; a second source of request data would
    // be a request the record does not describe.
    for (present, what) in [
        (!info.namelist.is_empty(), "namelist"),
        (
            !info.content.is_empty() || !info.contentexpr.is_empty(),
            "<content>",
        ),
    ] {
        if present {
            return Err(refuse(
                &info.request_schema_at,
                format!(
                    "a typed request is the record its schema declares, so {what} cannot \
                     be given beside it"
                ),
                &info.request_schema,
            ));
        }
    }
    let fields: BTreeSet<&str> = schema.fields.iter().map(|f| f.id.as_str()).collect();
    let mut supplied: BTreeSet<&str> = BTreeSet::new();
    for param in &info.base.params {
        if param.name == HOST_INVOKE_DEADLINE_PARAM {
            continue;
        }
        if !fields.contains(param.name.as_str()) {
            return Err(refuse(
                &param.source_location,
                format!(
                    "<param name=\"{}\"> is not a field of {} (fields: {})",
                    param.name,
                    info.request_schema,
                    fields.iter().copied().collect::<Vec<_>>().join(", ")
                ),
                &param.name,
            ));
        }
        if !supplied.insert(param.name.as_str()) {
            return Err(refuse(
                &param.source_location,
                format!(
                    "<param name=\"{}\"> is given twice; a record field has one value",
                    param.name
                ),
                &param.name,
            ));
        }
        // A string literal is the one value known here, so it is held to
        // its field now rather than where the invocation starts — the start
        // site checks evaluated values and passes a literal through as
        // written, which is only sound for a field that holds text.
        if param.is_static_literal {
            let field = schema
                .fields
                .iter()
                .find(|f| f.id == param.name)
                .expect("the field set was built from these fields");
            if let Some(detail) = literal_misfit(&param.static_value, field) {
                return Err(refuse(
                    &param.source_location,
                    format!("<param name=\"{}\"> {detail}", param.name),
                    &param.static_value,
                ));
            }
        }
    }
    let missing: Vec<&str> = fields.difference(&supplied).copied().collect();
    if !missing.is_empty() {
        return Err(refuse(
            &info.request_schema_at,
            format!(
                "no <param> supplies {} of {}",
                missing.join(", "),
                info.request_schema
            ),
            &info.request_schema,
        ));
    }
    Ok(())
}

/// Why the text literal `value` cannot be `field`'s value, if it cannot —
/// the judgement the runtimes' start sites make on an evaluated text, made
/// here on the one text known before run time.
fn literal_misfit(value: &str, field: &crate::forge::model::ForgeField) -> Option<String> {
    match &field.sce_type {
        SceType::String => None,
        SceType::Bytes => {
            let cap = crate::forge::limits::resolve_bytes_max(field.max_size) as usize;
            if value.chars().any(|c| (c as u32) > 0xFF) {
                Some(format!(
                    "carries a character above U+00FF, which no single byte of '{}' spells",
                    field.id
                ))
            } else if value.chars().count() > cap {
                Some(format!(
                    "is {} bytes, past the {cap} '{}' declares",
                    value.chars().count(),
                    field.id
                ))
            } else {
                None
            }
        }
        other => Some(format!(
            "is a text literal, and '{}' is {}",
            field.id,
            other.as_attr()
        )),
    }
}

fn located(
    at: &Option<SourceLocation>,
    diag_label: &str,
    error: ValidationError,
) -> Located<ForgeError> {
    Located::new(
        error.into(),
        diag_label,
        at.as_ref().and_then(|l| l.line),
        at.as_ref().and_then(|l| l.col),
    )
}

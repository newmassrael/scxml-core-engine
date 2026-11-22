// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! `sce:entry`: the record an invoke of `sce-host-audit-append` appends to a
//! durable audit log, and the marks its schema declares (docs/adr/0109, D14).
//!
//! ```xml
//! <invoke type="sce-host-audit-append" id="log" sce:entry="SignIn"
//!         sce:request="AuditAppendRequest" sce:result="AuditAppendResult">
//!   <param name="stream" expr="'sign-ins'"/>
//!   <param name="entry" expr="attempt"/>
//! </invoke>
//! ```
//!
//! The document names an imported event schema, and the build does three
//! things with it:
//!
//! - **It holds the request's `entry` to a record of that schema**: a variable,
//!   or a field of a record variable, of that very record type, which crosses
//!   to the host as the record's JSON payload, the form an event's payload is
//!   written in (`crate::forge::static_datamodel`).
//! - **It writes the request's `sensitivity`** ([`write_requests`]): the marks
//!   the schema declares on its fields (`sce:sensitivity`), each a field's JSON
//!   pointer and its mark, `/password=secret;/email=personal`, in declaration
//!   order; empty for a schema that marks none. The document never writes the
//!   field (a `<param name="sensitivity">` is refused where the request is
//!   judged, `crate::forge::typed_invoke`), so what the host redacts is what
//!   the schema says.
//! - **It refuses what it cannot carry** ([`resolve`]): an append that names
//!   no entry schema, a schema that holds a record or a union in a field (whose
//!   own marks the build does not carry into the request yet, so a mark there
//!   would not be honoured), a record whose JSON may be longer than the
//!   request's `entry` holds, and marks longer than its `sensitivity` holds.

use crate::forge::error::{ForgeError, Located, SourceLocation, ValidationError};
use crate::forge::model::{SceType, Sensitivity};
use crate::forge::value_type::{ScopedRecord, TypeOf, ValueType};
use crate::model::{Invoke, Param, SCXMLModel, UnsupportedInvokeInfo};

/// The standard operation that appends an entry to an audit log.
pub const AUDIT_APPEND: &str = "sce-host-audit-append";

/// The field of the operation's request the entry crosses in.
pub const ENTRY_FIELD: &str = "entry";

/// The field of the operation's request the build writes from the schema's
/// marks.
pub const SENSITIVITY_FIELD: &str = "sensitivity";

/// The most bytes of JSON a scalar that is neither a bool nor an enum value is
/// written as: a 64-bit integer as the quoted text of its digits (22), a real
/// as its shortest text, held under 32.
const NUMBER_JSON_MAX: u64 = 32;

/// The most bytes one byte of a text or a byte string is written as in JSON:
/// a control character escaped as `\u00XX`.
const ESCAPED_BYTE_MAX: u64 = 6;

/// The host-run `<invoke>`s of `model`, in document order: the per-state lists,
/// which are authoritative before the model-wide view is rebuilt.
fn host_invokes(model: &SCXMLModel) -> Vec<&UnsupportedInvokeInfo> {
    let mut states: Vec<_> = model.states.values().collect();
    states.sort_by_key(|s| s.document_order);
    states
        .into_iter()
        .flat_map(|s| s.invokes.iter())
        .filter_map(|invoke| match invoke {
            Invoke::Unsupported(info) => Some(info),
            _ => None,
        })
        .collect()
}

/// Judge every `sce:entry` of `model`, and every append, against the schemas
/// its imports resolved to.
///
/// Run where the imports are resolved, before the typed requests are judged,
/// so an alias read here names an import or is refused here.
pub fn resolve(model: &SCXMLModel, diag_label: &str) -> Result<(), Located<ForgeError>> {
    for info in host_invokes(model) {
        let names_entry = info.entry_schema_at.is_some() || !info.entry_schema.is_empty();
        let alias = info.entry_schema.as_str();
        let refuse = |detail: String| {
            located(
                info.entry_schema_at
                    .as_ref()
                    .or(info.base.source_location.as_ref()),
                diag_label,
                ValidationError::AuditEntrySchema {
                    invoke_id: info.base.invoke_id.clone(),
                    alias: alias.to_string(),
                    detail,
                },
            )
        };
        if info.invoke_type == AUDIT_APPEND && !names_entry {
            return Err(refuse(format!(
                "an entry of an audit log is a record of the event schema sce:entry names, \
                 written as its JSON payload and redacted by its marks, and this {AUDIT_APPEND} \
                 names none"
            )));
        }
        if !names_entry {
            continue;
        }
        if info.invoke_type != AUDIT_APPEND {
            return Err(refuse(format!(
                "is written on an <invoke> of type \"{}\", and only {AUDIT_APPEND} appends a \
                 record of an entry schema",
                info.invoke_type
            )));
        }
        if alias.is_empty() {
            return Err(refuse("the attribute names no event schema".to_string()));
        }
        let (Some(schema), Some(scoped)) = (
            model.imported_records.get(alias),
            model.imported_type_scope.records.get(alias),
        ) else {
            return Err(refuse(
                "names no event schema this document imports".to_string(),
            ));
        };
        if let Some((field, _)) = scoped.fields.iter().find(|(_, ty)| holds_composite(ty)) {
            return Err(refuse(format!(
                "names {}, whose field `{field}` holds a record, a union, a map or a set: the \
                 marks of a record inside an entry are not carried to the host yet, so a mark \
                 there would not be honoured; write the fields an entry keeps in its own record",
                schema.name
            )));
        }
        if let Some(bound) = request_text_bound(model, info, ENTRY_FIELD) {
            let most = record_json_max(scoped);
            if most > u64::from(bound) {
                return Err(refuse(format!(
                    "names {}, whose JSON may be {most} bytes, past the {bound} the request's \
                     {ENTRY_FIELD} holds: bound its texts tighter, or raise the bound \
                     `audit_entry` with --limit",
                    schema.name
                )));
            }
        }
        if let Some(bound) = request_text_bound(model, info, SENSITIVITY_FIELD) {
            let marks = marks_of(schema);
            if marks.len() > bound as usize {
                return Err(refuse(format!(
                    "names {}, whose marks `{marks}` are {} bytes, past the {bound} the \
                     request's {SENSITIVITY_FIELD} holds",
                    schema.name,
                    marks.len()
                )));
            }
        }
    }
    Ok(())
}

/// The marks of `schema`: each field it marks, as the JSON pointer of the
/// field and its mark, in declaration order, joined by `;`
/// (`/password=secret;/email=personal`); empty for a schema that marks none.
pub fn marks_of<T>(schema: &crate::forge::model::EventSchemaModel<T>) -> String {
    schema
        .fields
        .iter()
        .filter_map(|field| {
            field
                .sensitivity
                .map(|mark: Sensitivity| format!("/{}={}", field.id, mark.name()))
        })
        .collect::<Vec<_>>()
        .join(";")
}

/// Whether a value of `ty` holds a record, a union, a map or a set at any
/// depth.
fn holds_composite(ty: &ValueType) -> bool {
    match ty {
        TypeOf::Scalar(_) | TypeOf::Text { .. } | TypeOf::Bytes { .. } => false,
        TypeOf::Optional { elem } | TypeOf::List { elem, .. } => holds_composite(elem),
        TypeOf::Record { .. } | TypeOf::Union { .. } | TypeOf::Map { .. } | TypeOf::Set { .. } => {
            true
        }
    }
}

/// The most bytes of JSON a value of the record `record` is written as, every
/// text and byte string at its bound with every byte escaped. Called on a record
/// [`holds_composite`] admitted.
fn record_json_max(record: &ScopedRecord) -> u64 {
    let members: u64 = record
        .fields
        .iter()
        .map(|(id, ty)| 2 + id.len() as u64 + 1 + value_json_max(ty, record))
        .sum();
    let commas = record.fields.len().saturating_sub(1) as u64;
    2 + members + commas
}

/// The most bytes of JSON a value of `ty`, a type of a field of `record`, is
/// written as.
fn value_json_max(ty: &ValueType, record: &ScopedRecord) -> u64 {
    match ty {
        TypeOf::Scalar(SceType::Bool) => 5,
        // An enum value is written as its declared name, quoted; a name the
        // record's scope does not hold is bounded as a number would be.
        TypeOf::Scalar(SceType::Enum(reference)) => record
            .scope
            .enums
            .get(&reference.alias)
            .and_then(|scoped| {
                scoped
                    .variants
                    .iter()
                    .map(|variant| 2 + variant.name.len() as u64)
                    .max()
            })
            .unwrap_or(NUMBER_JSON_MAX),
        TypeOf::Scalar(_) => NUMBER_JSON_MAX,
        TypeOf::Text { max } | TypeOf::Bytes { max } => 2 + ESCAPED_BYTE_MAX * u64::from(max.max),
        TypeOf::Optional { elem } => value_json_max(elem, record).max(4),
        TypeOf::List { elem, max } => {
            let each = value_json_max(elem, record) + 1;
            2 + each * u64::from(max.max)
        }
        // Refused before a size is asked for ([`holds_composite`]).
        TypeOf::Record { .. } | TypeOf::Union { .. } | TypeOf::Map { .. } | TypeOf::Set { .. } => {
            u64::MAX / 2
        }
    }
}

/// The bound of the text field `name` of the request the invoke's
/// `sce:request` names; `None` when the alias names none of this document's
/// records, which the typed request's own judgement refuses.
fn request_text_bound(model: &SCXMLModel, info: &UnsupportedInvokeInfo, name: &str) -> Option<u32> {
    model
        .imported_records
        .get(&info.request_schema)?
        .typed_fields()
        .into_iter()
        .find(|(id, _)| id == name)
        .and_then(|(_, ty)| match ty {
            TypeOf::Text { max } => Some(max.max),
            _ => None,
        })
}

/// Write the `sensitivity` of every host-served invoke of [`AUDIT_APPEND`]: a
/// literal `<param>` holding the marks of its entry schema ([`marks_of`]).
/// Run where the build decides which invokes a host serves
/// (`crate::host_processor_analyzer::declare_host_surfaces`), so the parameter
/// is a lowering of the build and never a `<param>` the document wrote: the
/// document's own was refused where its request was judged.
pub fn write_requests(model: &mut SCXMLModel) {
    let mut marks: Vec<(String, String, String)> = Vec::new();
    for info in host_invokes(model) {
        if info.invoke_type == AUDIT_APPEND && info.host_served {
            let written = model
                .imported_records
                .get(&info.entry_schema)
                .map(marks_of)
                .unwrap_or_default();
            marks.push((
                info.base.state_name.clone(),
                info.base.invoke_id.clone(),
                written,
            ));
        }
    }
    for (state, invoke_id, written) in marks {
        let Some(state) = model.states.get_mut(&state) else {
            continue;
        };
        for invoke in &mut state.invokes {
            let Invoke::Unsupported(info) = invoke else {
                continue;
            };
            if info.base.invoke_id != invoke_id
                || info.base.params.iter().any(|p| p.name == SENSITIVITY_FIELD)
            {
                continue;
            }
            info.base.params.push(Param {
                name: SENSITIVITY_FIELD.to_string(),
                expr: format!("'{written}'"),
                is_static_literal: true,
                static_value: written.clone(),
                source_location: info
                    .entry_schema_at
                    .clone()
                    .or_else(|| info.base.source_location.clone()),
                ..Param::default()
            });
        }
    }
}

fn located(
    at: Option<&SourceLocation>,
    diag_label: &str,
    error: ValidationError,
) -> Located<ForgeError> {
    Located::new(
        error.into(),
        diag_label,
        at.and_then(|l| l.line),
        at.and_then(|l| l.col),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forge::value_type::{Bound, TypeScope};

    fn append(entry: &str, at: bool) -> UnsupportedInvokeInfo {
        let mut info = UnsupportedInvokeInfo {
            invoke_type: AUDIT_APPEND.to_string(),
            entry_schema: entry.to_string(),
            entry_schema_at: at.then(|| SourceLocation {
                file: "probe.scxml".into(),
                line: Some(3),
                col: Some(7),
            }),
            host_served: true,
            ..UnsupportedInvokeInfo::default()
        };
        info.base.invoke_id = "log".to_string();
        info.base.state_name = "logging".to_string();
        info
    }

    fn with(info: UnsupportedInvokeInfo) -> SCXMLModel {
        let mut model = SCXMLModel::default();
        let state = crate::model::State {
            id: "logging".to_string(),
            invokes: vec![Invoke::Unsupported(info)],
            ..crate::model::State::default()
        };
        model.states.insert("logging".to_string(), state);
        model
    }

    fn refusal(model: &SCXMLModel) -> String {
        match resolve(model, "probe.scxml") {
            Ok(()) => panic!("accepted"),
            Err(located) => located.to_string(),
        }
    }

    #[test]
    fn an_append_that_names_no_entry_schema_is_refused() {
        let why = refusal(&with(append("", false)));
        assert!(why.contains("names none"), "{why}");
    }

    #[test]
    fn an_entry_schema_on_another_type_is_refused() {
        let mut info = append("SignIn", true);
        info.invoke_type = "sce-host-kv-get".to_string();
        let why = refusal(&with(info));
        assert!(
            why.contains("sce-host-kv-get") && why.contains(AUDIT_APPEND),
            "{why}"
        );
    }

    #[test]
    fn an_empty_entry_schema_is_refused() {
        let why = refusal(&with(append("", true)));
        assert!(why.contains("names no event schema"), "{why}");
    }

    #[test]
    fn an_entry_schema_the_document_does_not_import_is_refused() {
        let why = refusal(&with(append("SignIn", true)));
        assert!(
            why.contains("names no event schema this document imports"),
            "{why}"
        );
    }

    #[test]
    fn an_invoke_of_another_type_that_names_none_is_left_alone() {
        let mut info = append("", false);
        info.invoke_type = "sce-host-kv-get".to_string();
        assert!(resolve(&with(info), "probe.scxml").is_ok());
    }

    fn record(fields: Vec<(&str, ValueType)>) -> ScopedRecord {
        ScopedRecord {
            identity: "/here/sign_in.scxml".into(),
            name: "sign_in".into(),
            fields: fields
                .into_iter()
                .map(|(id, ty)| (id.to_string(), ty))
                .collect(),
            scope: TypeScope::default(),
            added: Default::default(),
            lineage: Default::default(),
        }
    }

    #[test]
    fn a_records_json_is_bounded_by_its_fields_escaped_at_their_bounds() {
        let flat = record(vec![
            ("ok", TypeOf::Scalar(SceType::Bool)),
            ("who", TypeOf::Text { max: Bound::of(4) }),
        ]);
        // {"ok":true,"who":"\u0000\u0000\u0000\u0000"}: 2 braces, `"ok":` 5 and
        // 5, a comma, `"who":` 6 and 2 + 24.
        assert_eq!(record_json_max(&flat), 2 + (5 + 5) + 1 + (6 + 26));
        let optional = record(vec![(
            "n",
            TypeOf::Optional {
                elem: Box::new(TypeOf::Scalar(SceType::Uint8)),
            },
        )]);
        assert_eq!(record_json_max(&optional), 2 + 4 + NUMBER_JSON_MAX);
        assert!(holds_composite(&TypeOf::List {
            elem: Box::new(TypeOf::Record {
                alias: "Inner".into()
            }),
            max: Bound::of(2),
        }));
        assert!(!holds_composite(&TypeOf::List {
            elem: Box::new(TypeOf::Text { max: Bound::of(2) }),
            max: Bound::of(2),
        }));
    }

    #[test]
    fn the_build_writes_the_marks_into_the_request_once() {
        let mut model = with(append("", false));
        write_requests(&mut model);
        write_requests(&mut model);
        let Invoke::Unsupported(info) = &model.states["logging"].invokes[0] else {
            unreachable!("the invoke is host-run")
        };
        let written: Vec<&Param> = info
            .base
            .params
            .iter()
            .filter(|p| p.name == SENSITIVITY_FIELD)
            .collect();
        assert_eq!(written.len(), 1, "written once, however often asked");
        assert!(written[0].is_static_literal);
        assert_eq!(written[0].static_value, "", "no schema read, no marks");
    }
}

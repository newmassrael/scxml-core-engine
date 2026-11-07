// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! `sce:answer`: the answer schema an invoke of `sce-host-model-generate` asks
//! its text in (docs/adr/0087, section 4.1, item 2).
//!
//! ```xml
//! <invoke type="sce-host-model-generate" id="gen" sce:answer="Answer"
//!         sce:request="GenerateRequest" sce:result="GenerateResult">
//! ```
//!
//! The document names an imported event schema, and the build does two things
//! with it:
//!
//! - **It writes the request's `answer_schema`** ([`write_requests`]): the
//!   schema's id, `urn:sce:schema:<document name>:<revision>`, the `SCHEMA_ID`
//!   of its unit; empty for an invoke that names none. The document never
//!   writes the field (a `<param name="answer_schema">` is refused where the
//!   request is judged, `crate::forge::typed_invoke`), so the schema the model
//!   is asked for is the one the machine lifts its answer by.
//! - **It declares the machine's table of answer schemas** ([`resolve`]): for
//!   each schema the machine names, once, its id, its JSON Schema and its GBNF
//!   grammar, folded from the schema's type by
//!   [`crate::forge::payload_schema::PayloadSchema::of`], the one writer of
//!   either text. A host resolves the request's id through the table, with no
//!   file and no second copy of a grammar.
//!
//! The table's JSON Schema carries no `x-sce-source-hash`: that annotation
//! names the files a schema unit is written into, and the machine names its own
//! source set, the answer schema's document among it, in the header of the file
//! the table is written into.

use crate::forge::error::{ForgeError, Located, SourceLocation, ValidationError};
use crate::forge::payload_schema::{schema_id, PayloadSchema};
use crate::forge::value_type::TypeOf;
use crate::model::{Invoke, Param, SCXMLModel, UnsupportedInvokeInfo};

/// The standard operation whose request carries an answer schema.
pub const MODEL_GENERATE: &str = "sce-host-model-generate";

/// The field of the operation's request the build writes from `sce:answer`.
pub const ANSWER_SCHEMA_FIELD: &str = "answer_schema";

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

/// Judge every `sce:answer` of `model` against the schemas its imports
/// resolved to, and return the machine's table of answer schemas: each schema
/// once, in the order the document first names it.
///
/// Run where the imports are resolved, before the typed requests are judged,
/// so an alias read here names an import or is refused here.
pub fn resolve(
    model: &SCXMLModel,
    diag_label: &str,
) -> Result<Vec<PayloadSchema>, Located<ForgeError>> {
    let mut table: Vec<PayloadSchema> = Vec::new();
    for info in host_invokes(model) {
        if info.answer_schema_at.is_none() && info.answer_schema.is_empty() {
            continue;
        }
        let alias = info.answer_schema.as_str();
        let refuse = |detail: String| {
            located(
                info.answer_schema_at
                    .as_ref()
                    .or(info.base.source_location.as_ref()),
                diag_label,
                ValidationError::ModelAnswerSchema {
                    invoke_id: info.base.invoke_id.clone(),
                    alias: alias.to_string(),
                    detail,
                },
            )
        };
        if info.invoke_type != MODEL_GENERATE {
            return Err(refuse(format!(
                "is written on an <invoke> of type \"{}\", and only {MODEL_GENERATE} asks for a \
                 text under an answer schema",
                info.invoke_type
            )));
        }
        if alias.is_empty() {
            return Err(refuse("the attribute names no event schema".to_string()));
        }
        let (name, payload) = match answer_document(model, alias) {
            Some(AnswerDocument::Record { schema, scoped }) => (
                &schema.name,
                PayloadSchema::of(
                    &schema.name,
                    schema.revision(),
                    &scoped.fields,
                    &scoped.scope,
                    None,
                ),
            ),
            // docs/adr/0092, section 3.1 item 4: a union has a payload schema of
            // its own, named by its tag enum's revision (section 8.1).
            Some(AnswerDocument::Union { union, revision }) => {
                let Some(revision) = revision else {
                    return Err(refuse(format!(
                        "names union:{alias}, whose tag's enum this document does not import, \
                         and a union's revision is its tag enum's: import the enum `{}` tags \
                         its cases with",
                        union.name
                    )));
                };
                (&union.name, PayloadSchema::of_union(revision, union, None))
            }
            None => {
                return Err(refuse(
                    "names no event schema this document imports".to_string(),
                ))
            }
        };
        let payload = payload.map_err(|why| {
            refuse(format!(
                "names {name}, whose payload has no grammar a model can be held to: {why}"
            ))
        })?;
        if let Some(bound) = answer_schema_bound(model, info) {
            if payload.id.len() > bound as usize {
                return Err(refuse(format!(
                    "names {name}, whose id `{}` is {} bytes, past the {bound} the request's \
                     {ANSWER_SCHEMA_FIELD} holds: shorten the schema's document name",
                    payload.id,
                    payload.id.len()
                )));
            }
        }
        match table.iter().find(|entry| entry.id == payload.id) {
            Some(entry) if *entry == payload => {}
            Some(_) => {
                return Err(refuse(format!(
                    "names {name}, whose id `{}` is the id of another answer schema of this \
                     machine, and a host resolves one id to one schema",
                    payload.id
                )))
            }
            None => table.push(payload),
        }
    }
    Ok(table)
}

/// The bound of the request's `answer_schema` field, read from the record the
/// invoke's `sce:request` names; `None` when the alias names none of this
/// document's records, which the typed request's own judgement refuses.
fn answer_schema_bound(model: &SCXMLModel, info: &UnsupportedInvokeInfo) -> Option<u32> {
    model
        .imported_records
        .get(&info.request_schema)?
        .typed_fields()
        .into_iter()
        .find(|(id, _)| id == ANSWER_SCHEMA_FIELD)
        .and_then(|(_, ty)| match ty {
            TypeOf::Text { max } => Some(max.max),
            _ => None,
        })
}

/// The document an `sce:answer` names: an event schema the statechart imports,
/// with the record its type scope reads it as, or a union it imports
/// (docs/adr/0092, section 3.1 item 4), with its tag enum's revision where the
/// statechart imports that enum.
enum AnswerDocument<'m> {
    Record {
        schema: &'m crate::forge::model::EventSchemaModel,
        scoped: &'m crate::forge::value_type::ScopedRecord,
    },
    Union {
        union: &'m crate::forge::value_type::ScopedUnion,
        revision: Option<u32>,
    },
}

/// The document `alias` names in `model` ([`AnswerDocument`]), or `None` for
/// an alias that names no event schema and no union of its imports.
fn answer_document<'m>(model: &'m SCXMLModel, alias: &str) -> Option<AnswerDocument<'m>> {
    if let (Some(schema), Some(scoped)) = (
        model.imported_records.get(alias),
        model.imported_type_scope.records.get(alias),
    ) {
        return Some(AnswerDocument::Record { schema, scoped });
    }
    let union = model.imported_type_scope.unions.get(alias)?;
    let revision = crate::forge::value_path::tag_enum_alias(&model.imported_type_scope, union)
        .and_then(|tag| model.imported_enums.get(&tag))
        .map(|tag| tag.revision());
    Some(AnswerDocument::Union { union, revision })
}

/// The id the request of `info` carries: its answer schema's, or empty for an
/// invoke that names none.
fn answer_schema_id(model: &SCXMLModel, info: &UnsupportedInvokeInfo) -> String {
    if let Some(schema) = model.imported_records.get(&info.answer_schema) {
        return schema_id(&schema.name, schema.revision());
    }
    match answer_document(model, &info.answer_schema) {
        Some(AnswerDocument::Union {
            union,
            revision: Some(revision),
        }) => schema_id(&union.name, revision),
        _ => String::new(),
    }
}

/// Write the `answer_schema` of every host-served invoke of
/// [`MODEL_GENERATE`]: a literal `<param>` holding its answer schema's id, or
/// empty for one that names none. Run where the build decides which invokes a
/// host serves (`crate::host_processor_analyzer::declare_host_surfaces`), so
/// the parameter is a lowering of the build and never a `<param>` the document
/// wrote: the document's own was refused where its request was judged.
pub fn write_requests(model: &mut SCXMLModel) {
    let mut ids: Vec<(String, String, String)> = Vec::new();
    for info in host_invokes(model) {
        if info.invoke_type == MODEL_GENERATE && info.host_served {
            ids.push((
                info.base.state_name.clone(),
                info.base.invoke_id.clone(),
                answer_schema_id(model, info),
            ));
        }
    }
    for (state, invoke_id, id) in ids {
        let Some(state) = model.states.get_mut(&state) else {
            continue;
        };
        for invoke in &mut state.invokes {
            let Invoke::Unsupported(info) = invoke else {
                continue;
            };
            if info.base.invoke_id != invoke_id
                || info
                    .base
                    .params
                    .iter()
                    .any(|p| p.name == ANSWER_SCHEMA_FIELD)
            {
                continue;
            }
            info.base.params.push(Param {
                name: ANSWER_SCHEMA_FIELD.to_string(),
                expr: format!("'{id}'"),
                is_static_literal: true,
                static_value: id.clone(),
                source_location: info
                    .answer_schema_at
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

    fn model_invoke(answer: &str, at: bool) -> UnsupportedInvokeInfo {
        let mut info = UnsupportedInvokeInfo {
            invoke_type: MODEL_GENERATE.to_string(),
            answer_schema: answer.to_string(),
            answer_schema_at: at.then(|| SourceLocation {
                file: "probe.scxml".into(),
                line: Some(3),
                col: Some(7),
            }),
            host_served: true,
            ..UnsupportedInvokeInfo::default()
        };
        info.base.invoke_id = "gen".to_string();
        info.base.state_name = "asking".to_string();
        info
    }

    fn with(info: UnsupportedInvokeInfo) -> SCXMLModel {
        let mut model = SCXMLModel::default();
        let state = crate::model::State {
            id: "asking".to_string(),
            invokes: vec![Invoke::Unsupported(info)],
            ..crate::model::State::default()
        };
        model.states.insert("asking".to_string(), state);
        model
    }

    fn refusal(model: &SCXMLModel) -> String {
        match resolve(model, "probe.scxml") {
            Ok(table) => panic!("accepted, with {} answer schemas", table.len()),
            Err(located) => located.to_string(),
        }
    }

    #[test]
    fn an_invoke_that_names_no_answer_schema_adds_nothing_to_the_table() {
        let model = with(model_invoke("", false));
        assert!(resolve(&model, "probe.scxml").expect("accepted").is_empty());
    }

    #[test]
    fn an_answer_schema_on_another_type_is_refused() {
        let mut info = model_invoke("Answer", true);
        info.invoke_type = "sce-host-kv-get".to_string();
        let why = refusal(&with(info));
        assert!(
            why.contains("sce-host-kv-get") && why.contains(MODEL_GENERATE),
            "{why}"
        );
    }

    #[test]
    fn an_empty_answer_schema_is_refused() {
        let why = refusal(&with(model_invoke("", true)));
        assert!(why.contains("names no event schema"), "{why}");
    }

    #[test]
    fn an_answer_schema_the_document_does_not_import_is_refused() {
        let why = refusal(&with(model_invoke("Answr", true)));
        assert!(
            why.contains("names no event schema this document imports"),
            "{why}"
        );
    }

    /// A union named as an answer (docs/adr/0092, section 3.1 item 4) enters
    /// the table under its tag enum's revision (section 8.1), and the request
    /// carries that id; one whose tag's enum the document does not import has
    /// no revision to be named by, and is refused naming the repair.
    #[test]
    fn a_union_answer_is_named_by_its_tag_enums_revision() {
        use crate::forge::model::{EnumModel, EnumVariant, SceType};
        use crate::forge::value_type::{
            ScopedCase, ScopedEnum, ScopedRecord, ScopedUnion, ScopedVariant, TypeScope,
            WrittenType,
        };
        let variant = |name: &str, value: i128| ScopedVariant {
            name: name.to_string(),
            value,
        };
        let tag = ScopedEnum {
            name: "shape_kind".into(),
            variants: vec![variant("dot", 0), variant("wide", 1)],
            underlying: SceType::Uint8,
            strict: true,
        };
        let mut inner = TypeScope::default();
        inner.enums.insert("ShapeKind".into(), tag.clone());
        inner.records.insert(
            "Wide".into(),
            ScopedRecord {
                identity: "/here/wide.scxml".into(),
                name: "wide".into(),
                fields: vec![(
                    "breadth".into(),
                    WrittenType::read("uint8").unwrap().bounded().unwrap(),
                )],
                scope: TypeScope::default(),
            },
        );
        let union = ScopedUnion {
            identity: "/here/shape.scxml".into(),
            name: "shape".into(),
            tag: "kind".into(),
            tag_enum: "ShapeKind".into(),
            cases: vec![
                ScopedCase {
                    variant: variant("dot", 0),
                    record: None,
                },
                ScopedCase {
                    variant: variant("wide", 1),
                    record: Some("Wide".into()),
                },
            ],
            scope: inner,
        };
        let mut model = with(model_invoke("Shape", true));
        model
            .imported_type_scope
            .unions
            .insert("Shape".into(), union);
        let why = refusal(&model);
        assert!(
            why.contains("whose tag's enum this document does not import"),
            "{why}"
        );
        model.imported_type_scope.enums.insert("Kind".into(), tag);
        model.imported_enums.insert(
            "Kind".into(),
            EnumModel {
                name: "shape_kind".into(),
                underlying_type: SceType::Uint8,
                variants: ["dot", "wide"]
                    .iter()
                    .zip(0..)
                    .map(|(name, value)| EnumVariant {
                        name: (*name).to_string(),
                        value,
                        value_text: String::new(),
                        text: None,
                        since: None,
                        source_line: None,
                    })
                    .collect(),
                strict_variants: true,
                revision: Some(2),
                source_location: None,
            },
        );
        let table = resolve(&model, "probe.scxml").expect("a union is an answer");
        assert_eq!(table.len(), 1);
        assert_eq!(table[0].id, "urn:sce:schema:shape:2");
        write_requests(&mut model);
        let Invoke::Unsupported(info) = &model.states["asking"].invokes[0] else {
            unreachable!("the invoke is host-run")
        };
        let written = info
            .base
            .params
            .iter()
            .find(|p| p.name == ANSWER_SCHEMA_FIELD)
            .expect("the id is written");
        assert_eq!(written.static_value, "urn:sce:schema:shape:2");
    }

    #[test]
    fn the_build_writes_the_id_into_the_request_once() {
        let mut model = with(model_invoke("", false));
        write_requests(&mut model);
        write_requests(&mut model);
        let Invoke::Unsupported(info) = &model.states["asking"].invokes[0] else {
            unreachable!("the invoke is host-run")
        };
        let written: Vec<&Param> = info
            .base
            .params
            .iter()
            .filter(|p| p.name == ANSWER_SCHEMA_FIELD)
            .collect();
        assert_eq!(written.len(), 1, "written once, however often asked");
        assert!(written[0].is_static_literal);
        assert_eq!(written[0].static_value, "", "no answer schema, no id");
    }

    #[test]
    fn an_invoke_no_host_serves_is_not_written() {
        let mut info = model_invoke("", false);
        info.host_served = false;
        let mut model = with(info);
        write_requests(&mut model);
        let Invoke::Unsupported(info) = &model.states["asking"].invokes[0] else {
            unreachable!("the invoke is host-run")
        };
        assert!(info.base.params.is_empty());
    }
}

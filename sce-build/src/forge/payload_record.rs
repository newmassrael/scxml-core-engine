// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! `sce:payload`: the record a COSE operation of the standard host signs or
//! seals, and the record it reads back from what it verified or opened
//! (docs/adr/0107, Open question 1; docs/adr/0109, D9, "A record signed by
//! name").
//!
//! ```xml
//! <invoke type="sce-host-cose-sign1" id="sign" sce:payload="Note"
//!         sce:request="Sign1Request" sce:result="Sign1Result">
//!   <param name="key_id" expr="sign_key"/>
//!   <param name="payload" expr="note"/>
//!   <param name="external_aad" expr="aad"/>
//! </invoke>
//! <invoke type="sce-host-cose-verify1" id="verify" sce:payload="Note"
//!         sce:request="Verify1Request" sce:result="Verify1Result">
//!   ...
//! </invoke>
//! ```
//!
//! The document names an imported event schema, and the build does four
//! things with it:
//!
//! - **It holds the request's bytes to a record of that schema** on a signing
//!   and a seal: the `<param>` of `payload` (`plaintext`) is a variable, or a
//!   field of a record variable, of that very record type
//!   (`crate::forge::static_datamodel`), which crosses to the host as the
//!   record's deterministic CBOR (docs/adr/0101, section 2), encoded where the
//!   invocation starts by the runtime's record codec from the description the
//!   build writes of the record's type ([`describe`], [`write_descriptors`]).
//! - **It derives the record a verification or an opening completes with**
//!   ([`derive_completions`]): the library's result, its `payload`
//!   (`plaintext`) an `optional<record:...>` of that schema, present when the
//!   object verified (opened). `done.invoke.<id>` is bound to it, so the
//!   machine receives the record, never its bytes. The bytes the host answers
//!   are decoded where the completion is read; bytes that are no value of the
//!   record make the completion the operation's failure [`NOT_A_RECORD`].
//! - **It refuses what it cannot carry** ([`resolve`]): the attribute on an
//!   invoke of any other type, an alias that names no event schema the
//!   document imports, a record whose type has no CBOR form (a map or a set at
//!   any depth), a record that holds a float or an open enum at any depth
//!   (values the codec a peer prints from the schema writes and no payload
//!   spells, so the record codec could not write and read the record byte for
//!   byte as that codec does, [`describe`]), and a record whose largest
//!   encoding is past the bound of the
//!   field its bytes cross in (`cose_payload`, `cose_plaintext`, which
//!   `--limit` moves).
//! - **It refuses a lane that reads no record** ([`refuse_unread`]): Rust and
//!   C++ encode and decode one (docs/adr/0109, D0); every other lane refuses
//!   the machine by name.

use crate::forge::error::{ForgeError, GenerateError, Located, SourceLocation, ValidationError};
use crate::forge::model::SceType;
use crate::forge::standard_host::HostLane;
use crate::forge::value_type::{TypeEnv, TypeOf, TypeScope, ValueType};
use crate::generator::Language;
use crate::model::{Invoke, SCXMLModel, UnsupportedInvokeInfo};

/// Which record of an operation a record's bytes cross in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// The request: the build encodes the record where the invocation starts.
    Request,
    /// The result: the record is decoded where the completion is read.
    Result,
}

/// One COSE operation that carries a record by name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PayloadOperation {
    /// The operation's `type`.
    pub invoke_type: &'static str,
    /// The byte string of its request or its result the record's bytes are.
    pub field: &'static str,
    /// Which record [`Self::field`] is a field of.
    pub side: Side,
    /// For a result, the truth field that says the bytes are there (`valid`,
    /// `opened`); `None` for a request.
    pub answered: Option<&'static str>,
}

/// The operations `sce:payload` is written on, in the order of the table of
/// standard types.
pub const OPERATIONS: &[PayloadOperation] = &[
    PayloadOperation {
        invoke_type: "sce-host-cose-sign1",
        field: "payload",
        side: Side::Request,
        answered: None,
    },
    PayloadOperation {
        invoke_type: "sce-host-cose-verify1",
        field: "payload",
        side: Side::Result,
        answered: Some("valid"),
    },
    PayloadOperation {
        invoke_type: "sce-host-cose-encrypt0",
        field: "plaintext",
        side: Side::Request,
        answered: None,
    },
    PayloadOperation {
        invoke_type: "sce-host-cose-decrypt0",
        field: "plaintext",
        side: Side::Result,
        answered: Some("opened"),
    },
];

/// The failure a verification or an opening that names a record completes
/// with when the bytes it verified or opened are no value of the record
/// (docs/SCE_HOST_FEATURES.md, "Signing and sealing"): the object is the
/// signer's or the sealer's, and what it holds is not the document the machine
/// asked for, so the machine is not handed it.
pub const NOT_A_RECORD: &str = "not_a_record";

/// The lanes whose build encodes and decodes a record by name (docs/adr/0109,
/// D0): Rust and C++. A lane joins in the change that lands the record codec
/// in its runtime.
pub const PAYLOAD_RECORD_LANES: &[HostLane] = &[
    HostLane::Generated(Language::Rust),
    HostLane::Generated(Language::Cpp),
];

/// The operation `invoke_type` names, if `sce:payload` is written on it.
pub fn operation(invoke_type: &str) -> Option<&'static PayloadOperation> {
    OPERATIONS.iter().find(|op| op.invoke_type == invoke_type)
}

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

/// Judge every `sce:payload` of `model` against the schemas its imports
/// resolved to.
///
/// Run where the imports are resolved, before the typed requests are judged,
/// so an alias read here names an import or is refused here.
pub fn resolve(model: &SCXMLModel, diag_label: &str) -> Result<(), Located<ForgeError>> {
    for info in host_invokes(model) {
        let names_payload = info.payload_schema_at.is_some() || !info.payload_schema.is_empty();
        if !names_payload {
            continue;
        }
        let alias = info.payload_schema.as_str();
        let refuse = |detail: String| {
            located(
                info.payload_schema_at
                    .as_ref()
                    .or(info.base.source_location.as_ref()),
                diag_label,
                ValidationError::PayloadRecordSchema {
                    invoke_id: info.base.invoke_id.clone(),
                    alias: alias.to_string(),
                    detail,
                },
            )
        };
        let Some(op) = operation(&info.invoke_type) else {
            return Err(refuse(format!(
                "is written on an <invoke> of type \"{}\", and only {} sign, verify, seal or open \
                 a record by name",
                info.invoke_type,
                OPERATIONS
                    .iter()
                    .map(|op| op.invoke_type)
                    .collect::<Vec<_>>()
                    .join(", ")
            )));
        };
        if alias.is_empty() {
            return Err(refuse("the attribute names no event schema".to_string()));
        }
        let Some(schema) = model.imported_records.get(alias) else {
            return Err(refuse(
                "names no event schema this document imports".to_string(),
            ));
        };
        let record = ValueType::Record {
            alias: alias.to_string(),
        };
        if let Err(why) = describe(&record, &model.imported_type_scope) {
            return Err(refuse(format!(
                "names {}, whose type a record signed by name cannot hold (docs/adr/0101, \
                 section 2): {why}",
                schema.name
            )));
        }
        let most =
            crate::forge::value_type::codec_cbor_max_len(&record, &model.imported_type_scope)
                .map_err(|why| {
                    refuse(format!(
                        "names {}, whose largest encoding the build cannot count: {why:?}",
                        schema.name
                    ))
                })?;
        let carrier = match op.side {
            Side::Request => &info.request_schema,
            Side::Result => &info.result_schema,
        };
        if let Some((bound, limit)) = bytes_bound(model, carrier, op.field) {
            if most > u64::from(bound) {
                let raise = limit.map_or_else(
                    || "bound its texts, byte strings and lists tighter".to_string(),
                    |name| {
                        format!(
                            "bound its texts, byte strings and lists tighter, or raise the bound \
                             `{name}` with --limit"
                        )
                    },
                );
                return Err(refuse(format!(
                    "names {}, whose deterministic CBOR may be {most} bytes, past the {bound} \
                     the {} of {} holds: {raise}",
                    schema.name, op.field, op.invoke_type
                )));
            }
        }
    }
    Ok(())
}

/// The bound of the byte string `field` of the record `alias` names, as this
/// build resolved it, with the name of the bound where it has one; `None` when
/// the alias names none of this document's records, which the typed request's
/// own judgement refuses.
fn bytes_bound(model: &SCXMLModel, alias: &str, field: &str) -> Option<(u32, Option<String>)> {
    model
        .imported_records
        .get(alias)?
        .fields
        .iter()
        .find(|f| f.id == field)
        .and_then(|f| match &f.sce_type {
            TypeOf::Bytes { max } => Some((max.max, max.limit.clone().or_else(|| f.limit.clone()))),
            _ => None,
        })
}

/// The description of the type `ty` the runtime's record codec reads
/// (`sce_rust_runtime::record_cbor`, `common/RecordCbor.h`), each enum, record
/// and union read in `env`: one JSON value per type, of one member that names
/// the kind.
///
/// | Type | Description |
/// |---|---|
/// | `uint8`-`uint64` | `{"uint":<bits>}` |
/// | `int8`-`int64` | `{"int":<bits>}` |
/// | `bool` | `{"bool":{}}` |
/// | `string[B]` | `{"text":B}` |
/// | `bytes[B]` | `{"bytes":B}` |
/// | `enum:E`, closed | `{"enum":{"closed":true,"variants":[[<name>,<number>],...]}}` |
/// | `record:R` | `{"record":[[<id>,<optional>,<type>,<default>],...]}`, the fields in the order their keys' encodings sort (RFC 8949 section 4.2.1), each the type of its present value, and `<default>` what a map that lacks the field's key reads |
/// | `union:U` | `{"union":{"tag":<key>,"cases":[[<variant>,<fields or null>],...]}}` |
/// | `list<T>[M..B]` | `{"list":{"min":M,"max":B,"of":<type>}}` |
/// | `optional<T>` as a list's element | `{"nullable":<type>}` |
///
/// A field's `<default>` is the rule of a record whose schema grew
/// (docs/adr/0044; SCE_FORGE.md section 4.6.1, "A record whose schema grew"):
/// the deterministic CBOR, as lowercase hexadecimal text, of the value a map
/// written before a later revision added the field reads in its place, its
/// `sce:default`, or the empty list (`80`) for a list; `null` for a field of
/// the first revision, which a map that lacks it is refused for, and for an
/// optional field, which a missing key leaves absent. The bytes are the ones
/// the plan of the codec ADR 0101 prints for the same schema holds
/// (`crate::forge::encode_plan::CborField::default`), made by the same
/// function (`crate::forge::encode_plan::lacked_value`), and the record codec
/// reads them with the reader of the field's own value, as the printed codec
/// does: the two read a record of an earlier revision alike.
///
/// The order of a record's fields is decided here, once, so no runtime sorts.
/// The refusal of what has no CBOR form: a map or a set, a text or a byte
/// string with no bound, an alias `env` does not resolve, a record that holds
/// itself. And the refusal of what the record codec could not write and read
/// byte for byte as the codec a peer prints from the same schema
/// (docs/adr/0101) does: a float, whose values that are not finite no payload
/// spells, and an open enum, whose numbers no variant names no payload spells.
/// Every other type is written by the two alike, which
/// `tests/forge/conformance/numerical_reference.json`
/// (`codec_cbor_payload_record`) holds on Rust and C++.
pub fn describe(ty: &ValueType, env: &dyn TypeEnv) -> Result<String, String> {
    describe_value(ty, env, &mut Vec::new()).map(|value| value.to_string())
}

fn describe_value(
    ty: &ValueType,
    env: &dyn TypeEnv,
    records: &mut Vec<String>,
) -> Result<serde_json::Value, String> {
    use serde_json::json;
    Ok(match ty {
        TypeOf::Scalar(scalar) => describe_scalar(scalar, env)?,
        TypeOf::Text { max } => json!({ "text": max.max }),
        TypeOf::Bytes { max } => json!({ "bytes": max.max }),
        TypeOf::Optional { elem } => json!({ "nullable": describe_value(elem, env, records)? }),
        TypeOf::List { elem, max } => json!({
            "list": { "min": max.min, "max": max.max, "of": describe_value(elem, env, records)? }
        }),
        TypeOf::Record { alias } => json!({ "record": describe_record(alias, env, records)? }),
        TypeOf::Union { alias } => {
            let union = crate::forge::value_type::enter_union(alias, env, records)
                .map_err(|why| format!("`union:{alias}` {why:?}"))?;
            let mut cases = Vec::with_capacity(union.cases.len());
            for case in &union.cases {
                let fields = match &case.record {
                    None => serde_json::Value::Null,
                    Some(record) => describe_record(record, &union.scope, records)?,
                };
                cases.push(json!([case.variant.name, fields]));
            }
            records.pop();
            json!({ "union": { "tag": union.tag, "cases": cases } })
        }
        TypeOf::Map { .. } | TypeOf::Set { .. } => {
            return Err(format!(
                "`{ty}` is a map or a set, which has no CBOR form (docs/adr/0108, D7)"
            ))
        }
    })
}

/// The fields of the record `alias` names in `env`, read in its own document's
/// scope, in the order their keys' encodings sort, each with what a map that
/// lacks its key reads.
fn describe_record(
    alias: &str,
    env: &dyn TypeEnv,
    records: &mut Vec<String>,
) -> Result<serde_json::Value, String> {
    let (fields, inner) = crate::forge::value_type::enter_record(alias, env, records)
        .map_err(|why| format!("`record:{alias}` {why:?}"))?;
    // The fields a later revision of the record's schema added, read where the
    // alias resolves, as the CBOR plan of a printed codec reads them.
    let added = env.record_added(alias);
    let order = crate::forge::encode_plan::text_key_order(fields.iter().map(|(id, _)| id.as_str()));
    let mut described = Vec::with_capacity(fields.len());
    for at in order {
        let (id, ty) = &fields[at];
        let optional = ty.optional_elem().is_some();
        let present = ty.optional_elem().unwrap_or(ty);
        // What a map that lacks the field's key reads: the bytes the printed
        // codec's plan holds for it (`CborField::default`), made by the one
        // function both read, so the two readers of a record cannot part.
        let lacked =
            crate::forge::encode_plan::lacked_value(ty, added.and_then(|a| a.get(id)), inner)
                .map_or(serde_json::Value::Null, |bytes| {
                    serde_json::Value::String(bytes.iter().map(|b| format!("{b:02x}")).collect())
                });
        described.push(serde_json::json!([
            id,
            optional,
            describe_value(present, inner, records)?,
            lacked
        ]));
    }
    records.pop();
    Ok(serde_json::Value::Array(described))
}

fn describe_scalar(scalar: &SceType, env: &dyn TypeEnv) -> Result<serde_json::Value, String> {
    use serde_json::json;
    Ok(match scalar {
        SceType::Bool => json!({ "bool": {} }),
        // A float that is not finite is a value of its type, which the codec a
        // peer prints writes (`f9 7e 00` for a NaN, docs/adr/0101 section
        // 2.1), and no payload spells: the record codec would refuse bytes the
        // printed codec reads and a value the printed codec writes, so the two
        // writers of a record signed by name would differ.
        SceType::Float32 | SceType::Float64 => {
            return Err(format!(
                "`{}` holds values that are not finite, which the printed codec writes and \
                 no payload spells, so the record codec could not write or read every value \
                 of it as the printed codec does",
                scalar.as_attr()
            ))
        }
        SceType::Uint8 | SceType::Uint16 | SceType::Uint32 | SceType::Uint64 => {
            json!({ "uint": scalar.int_bit_width().unwrap_or(64) })
        }
        SceType::Int8 | SceType::Int16 | SceType::Int32 | SceType::Int64 => {
            json!({ "int": scalar.int_bit_width().unwrap_or(64) })
        }
        SceType::Enum(reference) => {
            let enumeration = env
                .enum_type(&reference.alias)
                .ok_or_else(|| format!("`enum:{}` names no enum", reference.alias))?;
            // An open enum holds every number of its carrier, and a number no
            // variant names is written by the printed codec and spelled by no
            // payload, which names a value by its variant.
            if !enumeration.strict {
                return Err(format!(
                    "`enum:{}` is open, and a number no variant of it names, which the printed \
                     codec writes, is spelled by no payload, so the record codec could not \
                     write or read every value of it as the printed codec does",
                    reference.alias
                ));
            }
            let mut variants = Vec::with_capacity(enumeration.variants.len());
            for variant in &enumeration.variants {
                let number = if variant.value < 0 {
                    i64::try_from(variant.value).map(serde_json::Value::from)
                } else {
                    u64::try_from(variant.value).map(serde_json::Value::from)
                }
                .map_err(|_| {
                    format!(
                        "the variant `{}` of `enum:{}` is numbered past 64 bits",
                        variant.name, reference.alias
                    )
                })?;
                variants.push(json!([variant.name, number]));
            }
            json!({ "enum": { "closed": enumeration.strict, "variants": variants } })
        }
        SceType::String | SceType::Bytes => {
            return Err(format!(
                "`{}` states no bound, and a value with no largest encoding has no CBOR form",
                scalar.as_attr()
            ))
        }
    })
}

/// Derive the record each verification and opening that names a record
/// completes with, and keep it beside the document's own records: the library's
/// result, its byte string an `optional<record:...>` of the record `sce:payload`
/// names, under an alias of its own (`<result alias>Of<invoke>`), and the
/// invoke's [`UnsupportedInvokeInfo::payload_result`] naming it. Run after the
/// standard types are held to the library's records and before each completion
/// is bound to its result ([`crate::forge::typed_invoke::bind_results`]), which
/// binds `done.invoke.<id>` to the derived record, so every reader of the
/// completion, a guard and an assignment among them, reads the record.
///
/// The derived record is the completion's view, as a codec's record view is a
/// codec's (docs/adr/0102, section 1.1): nothing the author writes can make it
/// disagree with the library's result, since it is made of it.
pub fn derive_completions(
    model: &mut SCXMLModel,
    diag_label: &str,
) -> Result<(), Located<ForgeError>> {
    // Each derivation: the invoke's id, its result's alias, the derived alias
    // and the identifier the invoke's generated names are built from.
    let mut derived: Vec<(String, String, String, String)> = Vec::new();
    for info in host_invokes(model) {
        let Some(op) = operation(&info.invoke_type) else {
            continue;
        };
        if op.side != Side::Result || info.payload_schema.is_empty() {
            continue;
        }
        let alias = format!(
            "{}Of{}",
            info.result_schema,
            crate::filters::to_pascal_case(info.base.field_suffix.clone())
        );
        if model.imported_records.contains_key(&alias) {
            return Err(located(
                info.payload_schema_at
                    .as_ref()
                    .or(info.base.source_location.as_ref()),
                diag_label,
                ValidationError::PayloadRecordSchema {
                    invoke_id: info.base.invoke_id.clone(),
                    alias: info.payload_schema.clone(),
                    detail: format!(
                        "the completion's record is named `{alias}`, and the document imports \
                         another record under that alias: rename the import"
                    ),
                },
            ));
        }
        derived.push((
            info.base.invoke_id.clone(),
            info.result_schema.clone(),
            alias,
            info.base.field_suffix.clone(),
        ));
    }
    for (invoke_id, result_alias, alias, suffix) in derived {
        let Some(info) = model
            .states
            .values()
            .flat_map(|s| s.invokes.iter())
            .find_map(|invoke| match invoke {
                Invoke::Unsupported(info) if info.base.invoke_id == invoke_id => Some(info),
                _ => None,
            })
        else {
            continue;
        };
        let Some(op) = operation(&info.invoke_type) else {
            continue;
        };
        let record_alias = info.payload_schema.clone();
        let (Some(result), Some(scoped)) = (
            model.imported_records.get(&result_alias),
            model.imported_type_scope.records.get(&result_alias),
        ) else {
            // A result the library does not give is refused where the
            // standard type is held to its records.
            continue;
        };
        let Some(held) = model
            .imported_type_scope
            .records
            .get(&record_alias)
            .cloned()
        else {
            continue;
        };
        let present = ValueType::Optional {
            elem: Box::new(ValueType::Record {
                alias: record_alias.clone(),
            }),
        };
        let mut schema = result.clone();
        schema.name = format!("{}_{}", result.name, crate::filters::to_snake_case(suffix));
        schema.event_name = format!("done.invoke.{invoke_id}");
        for field in &mut schema.fields {
            if field.id == op.field {
                field.sce_type = present.clone();
                field.max_size = None;
                field.limit = None;
            }
        }
        let mut scope = scoped.scope.clone();
        scope.records.insert(record_alias.clone(), held);
        let view = crate::forge::value_type::ScopedRecord {
            identity: format!("{}#{invoke_id}", scoped.identity),
            name: schema.name.clone(),
            fields: scoped
                .fields
                .iter()
                .map(|(id, ty)| {
                    if id == op.field {
                        (id.clone(), present.clone())
                    } else {
                        (id.clone(), ty.clone())
                    }
                })
                .collect(),
            scope,
            // The result keeps its own revision facts.
            added: scoped.added.clone(),
            lineage: Default::default(),
        };
        model.imported_records.insert(alias.clone(), schema);
        model
            .imported_type_scope
            .records
            .insert(alias.clone(), view);
        for state in model.states.values_mut() {
            for invoke in &mut state.invokes {
                if let Invoke::Unsupported(info) = invoke {
                    if info.base.invoke_id == invoke_id {
                        info.payload_result = alias.clone();
                    }
                }
            }
        }
    }
    Ok(())
}

/// Write the description of the record each host-served invoke that names one
/// carries ([`describe`]) into [`UnsupportedInvokeInfo::payload_cbor`], and for
/// a signing and a seal the request field its bytes cross in into
/// [`UnsupportedInvokeInfo::payload_param`], which the start site encodes the
/// `<param>` of. Run where the build decides which invokes a host serves
/// (`crate::host_processor_analyzer::declare_host_surfaces`).
pub fn write_descriptors(model: &mut SCXMLModel) {
    let scope: TypeScope = model.imported_type_scope.clone();
    for state in model.states.values_mut() {
        for invoke in &mut state.invokes {
            let Invoke::Unsupported(info) = invoke else {
                continue;
            };
            if !info.host_served || info.payload_schema.is_empty() {
                continue;
            }
            let Some(op) = operation(&info.invoke_type) else {
                continue;
            };
            let record = ValueType::Record {
                alias: info.payload_schema.clone(),
            };
            info.payload_cbor = describe(&record, &scope)
                .expect("resolve refuses a record whose type has no CBOR form at the parse");
            if op.side == Side::Request {
                info.payload_param = op.field.to_string();
            }
        }
    }
}

/// How the completion `event` of a verification or an opening that names a
/// record is read: what the runtime's record codec is handed before the
/// completion is lifted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionReading {
    /// The result's byte string the record's bytes are.
    pub field: &'static str,
    /// The result's truth field that says the bytes are there.
    pub answered: &'static str,
    /// The description of the record's type ([`describe`]).
    pub descriptor: String,
    /// The completion, as JSON, the operation's failure [`NOT_A_RECORD`] is:
    /// `error` the code, every other field at its zero, the record absent.
    pub failure: String,
}

/// The reading of the completion `event` ([`CompletionReading`]), or `None`
/// for an event that is no completion of a verification or an opening that
/// names a record.
pub fn completion_reading(model: &SCXMLModel, event: &str) -> Option<CompletionReading> {
    let info = host_invokes(model).into_iter().find(|info| {
        !info.payload_result.is_empty() && format!("done.invoke.{}", info.base.invoke_id) == event
    })?;
    let op = operation(&info.invoke_type)?;
    let answered = op.answered?;
    let derived = model.imported_records.get(&info.payload_result)?;
    let members: serde_json::Map<String, serde_json::Value> = derived
        .fields
        .iter()
        .map(|field| {
            let zero = if field.id == "error" {
                serde_json::Value::String(NOT_A_RECORD.to_string())
            } else {
                zero_of(&field.sce_type)
            };
            (field.id.clone(), zero)
        })
        .collect();
    let descriptor = if info.payload_cbor.is_empty() {
        describe(
            &ValueType::Record {
                alias: info.payload_schema.clone(),
            },
            &model.imported_type_scope,
        )
        .ok()?
    } else {
        info.payload_cbor.clone()
    };
    Some(CompletionReading {
        field: op.field,
        answered,
        descriptor,
        failure: serde_json::Value::Object(members).to_string(),
    })
}

/// The JSON of the zero of a field of `ty` in a result record: an empty text
/// or byte string, `false`, `0` (a 64-bit integer as the text of its digits,
/// as a payload writes one), and `null` for a value that may be absent.
pub(crate) fn zero_of(ty: &ValueType) -> serde_json::Value {
    use serde_json::json;
    match ty {
        TypeOf::Scalar(SceType::Bool) => json!(false),
        TypeOf::Scalar(SceType::Int64 | SceType::Uint64) => json!("0"),
        TypeOf::Scalar(_) => json!(0),
        TypeOf::Text { .. } | TypeOf::Bytes { .. } => json!(""),
        TypeOf::List { .. } => json!([]),
        TypeOf::Optional { .. }
        | TypeOf::Record { .. }
        | TypeOf::Union { .. }
        | TypeOf::Map { .. }
        | TypeOf::Set { .. } => serde_json::Value::Null,
    }
}

/// Whether `model` runs an invoke that names a record by `sce:payload`: the
/// one whose generated machine reads the runtime's record codec.
pub fn machine_reads_records(model: &SCXMLModel) -> bool {
    host_invokes(model)
        .iter()
        .any(|info| !info.payload_schema.is_empty())
}

/// Refuse `model` for `lane` when the lane is not one of
/// [`PAYLOAD_RECORD_LANES`] and an `<invoke>` of `model` names a record by
/// `sce:payload`: the first such `<invoke>`, in document order, as
/// `generate/unsupported-feature`, naming the invoke, the record, the lane and
/// the lanes that read one (docs/adr/0109, D0). Asked where
/// [`crate::forge::standard_host::refuse_unserved`] is, and after it, so a lane
/// that serves no COSE operation is refused as that first.
pub fn refuse_unread(model: &SCXMLModel, lane: HostLane) -> Result<(), GenerateError> {
    if PAYLOAD_RECORD_LANES.contains(&lane) {
        return Ok(());
    }
    let Some(info) = host_invokes(model)
        .into_iter()
        .find(|info| !info.payload_schema.is_empty())
    else {
        return Ok(());
    };
    let readers: Vec<&str> = PAYLOAD_RECORD_LANES
        .iter()
        .map(|reader| reader.wire_name())
        .collect();
    Err(GenerateError::unsupported_at(
        format!(
            "<invoke id=\"{}\" sce:payload=\"{}\">: the {} lane does not encode or decode a record \
             a signing or a seal carries by name; it is read on {}",
            info.base.invoke_id,
            info.payload_schema,
            lane.wire_name(),
            readers.join(", ")
        ),
        info.payload_schema_at
            .clone()
            .or_else(|| info.base.source_location.clone()),
    ))
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
    use crate::forge::value_type::{Bound, ScopedEnum, ScopedRecord, ScopedVariant};

    fn signing(invoke_type: &str, alias: &str, at: bool) -> UnsupportedInvokeInfo {
        let mut info = UnsupportedInvokeInfo {
            invoke_type: invoke_type.to_string(),
            payload_schema: alias.to_string(),
            payload_schema_at: at.then(|| SourceLocation {
                file: "probe.scxml".into(),
                line: Some(4),
                col: Some(9),
            }),
            host_served: true,
            ..UnsupportedInvokeInfo::default()
        };
        info.base.invoke_id = "sign".to_string();
        info.base.state_name = "signing".to_string();
        info
    }

    fn with(info: UnsupportedInvokeInfo) -> SCXMLModel {
        let mut model = SCXMLModel::default();
        let state = crate::model::State {
            id: "signing".to_string(),
            invokes: vec![Invoke::Unsupported(info)],
            ..crate::model::State::default()
        };
        model.states.insert("signing".to_string(), state);
        model
    }

    fn refusal(model: &SCXMLModel) -> String {
        match resolve(model, "probe.scxml") {
            Ok(()) => panic!("accepted"),
            Err(located) => located.to_string(),
        }
    }

    #[test]
    fn a_record_named_on_another_type_is_refused() {
        let why = refusal(&with(signing("sce-host-kv-get", "Note", true)));
        assert!(
            why.contains("sce-host-kv-get") && why.contains("sce-host-cose-sign1"),
            "{why}"
        );
    }

    #[test]
    fn an_empty_record_name_is_refused() {
        let why = refusal(&with(signing("sce-host-cose-sign1", "", true)));
        assert!(why.contains("names no event schema"), "{why}");
    }

    #[test]
    fn a_record_the_document_does_not_import_is_refused() {
        let why = refusal(&with(signing("sce-host-cose-verify1", "Note", true)));
        assert!(
            why.contains("names no event schema this document imports"),
            "{why}"
        );
    }

    #[test]
    fn an_invoke_that_names_no_record_is_left_alone() {
        let model = with(signing("sce-host-cose-sign1", "", false));
        assert!(resolve(&model, "probe.scxml").is_ok());
        assert!(!machine_reads_records(&model));
    }

    #[test]
    fn each_operation_carries_its_record_in_the_field_the_library_names() {
        let fields: Vec<(&str, &str, Side, Option<&str>)> = OPERATIONS
            .iter()
            .map(|op| (op.invoke_type, op.field, op.side, op.answered))
            .collect();
        assert_eq!(
            fields,
            vec![
                ("sce-host-cose-sign1", "payload", Side::Request, None),
                (
                    "sce-host-cose-verify1",
                    "payload",
                    Side::Result,
                    Some("valid")
                ),
                ("sce-host-cose-encrypt0", "plaintext", Side::Request, None),
                (
                    "sce-host-cose-decrypt0",
                    "plaintext",
                    Side::Result,
                    Some("opened")
                ),
            ]
        );
    }

    /// A scope that holds the record `Note`: an id, an optional text, a level
    /// of a closed enum, and a list of byte strings.
    fn note_scope() -> TypeScope {
        let mut scope = TypeScope::default();
        scope.enums.insert(
            "Level".to_string(),
            ScopedEnum {
                name: "level".to_string(),
                variants: vec![
                    ScopedVariant {
                        name: "low".to_string(),
                        value: 0,
                    },
                    ScopedVariant {
                        name: "fault".to_string(),
                        value: -1,
                    },
                ],
                underlying: SceType::Int8,
                strict: true,
            },
        );
        let level = SceType::Enum(crate::forge::model::EnumRef {
            alias: "Level".to_string(),
        });
        let inner = TypeScope {
            enums: scope.enums.clone(),
            ..TypeScope::default()
        };
        scope.records.insert(
            "Note".to_string(),
            ScopedRecord {
                identity: "/here/note.scxml".to_string(),
                name: "note".to_string(),
                fields: vec![
                    ("title".to_string(), TypeOf::Text { max: Bound::of(16) }),
                    ("id".to_string(), TypeOf::Scalar(SceType::Uint32)),
                    (
                        "memo".to_string(),
                        TypeOf::Optional {
                            elem: Box::new(TypeOf::Text { max: Bound::of(8) }),
                        },
                    ),
                    ("level".to_string(), TypeOf::Scalar(level)),
                    (
                        "keys".to_string(),
                        TypeOf::List {
                            elem: Box::new(TypeOf::Bytes { max: Bound::of(4) }),
                            max: Bound::of(2),
                        },
                    ),
                ],
                scope: inner,
                added: Default::default(),
                lineage: Default::default(),
            },
        );
        scope
    }

    #[test]
    fn a_record_is_described_in_the_order_its_keys_encode() {
        let described = describe(
            &ValueType::Record {
                alias: "Note".to_string(),
            },
            &note_scope(),
        )
        .expect("a record of bounded fields has a CBOR form");
        // `id` (2 bytes) first, then the 4-byte ids in byte order (`keys`,
        // `memo`), then the 5-byte ones (`level`, `title`).
        assert_eq!(
            described,
            r#"{"record":[["id",false,{"uint":32},null],["keys",false,{"list":{"max":2,"min":0,"of":{"bytes":4}}},null],["memo",true,{"text":8},null],["level",false,{"enum":{"closed":true,"variants":[["low",0],["fault",-1]]}},null],["title",false,{"text":16},null]]}"#
        );
    }

    /// A scope that holds the record `Revised`, whose schema's first revision
    /// carried `id` alone and whose second added a number, a text and an enum,
    /// each with its `sce:default`, a list and an optional text.
    fn revised_scope() -> TypeScope {
        let mut inner = TypeScope::default();
        inner.enums.insert(
            "Level".to_string(),
            ScopedEnum {
                name: "level".to_string(),
                variants: vec![
                    ScopedVariant {
                        name: "low".to_string(),
                        value: 0,
                    },
                    ScopedVariant {
                        name: "fault".to_string(),
                        value: -1,
                    },
                ],
                underlying: SceType::Int8,
                strict: true,
            },
        );
        let level = SceType::Enum(crate::forge::model::EnumRef {
            alias: "Level".to_string(),
        });
        let added = |default: Option<&str>| crate::forge::value_type::AddedField {
            since: 2,
            default: default.map(str::to_string),
        };
        let mut scope = TypeScope::default();
        scope.records.insert(
            "Revised".to_string(),
            ScopedRecord {
                identity: "/here/revised.scxml".to_string(),
                name: "revised".to_string(),
                fields: vec![
                    ("id".to_string(), TypeOf::Scalar(SceType::Uint32)),
                    ("rank".to_string(), TypeOf::Scalar(SceType::Int8)),
                    ("label".to_string(), TypeOf::Text { max: Bound::of(8) }),
                    ("level".to_string(), TypeOf::Scalar(level)),
                    (
                        "tags".to_string(),
                        TypeOf::List {
                            elem: Box::new(TypeOf::Scalar(SceType::Uint8)),
                            max: Bound::of(2),
                        },
                    ),
                    (
                        "memo".to_string(),
                        TypeOf::Optional {
                            elem: Box::new(TypeOf::Text { max: Bound::of(8) }),
                        },
                    ),
                ],
                scope: inner,
                added: [
                    ("rank", added(Some("-2"))),
                    ("label", added(Some("none"))),
                    ("level", added(Some("fault"))),
                    ("tags", added(None)),
                    ("memo", added(None)),
                ]
                .into_iter()
                .map(|(id, field)| (id.to_string(), field))
                .collect(),
                lineage: Default::default(),
            },
        );
        scope
    }

    /// A field a later revision added is described with the deterministic CBOR
    /// of what a map that lacks its key reads, the very bytes the plan of the
    /// printed codec of the same record holds for it; a field of the first
    /// revision and an optional field with none (docs/adr/0044).
    #[test]
    fn a_field_a_later_revision_added_is_described_with_the_bytes_its_plan_reads() {
        let scope = revised_scope();
        let record = ValueType::Record {
            alias: "Revised".to_string(),
        };
        let described: serde_json::Value = serde_json::from_str(
            &describe(&record, &scope).expect("a record of bounded fields has a CBOR form"),
        )
        .expect("a description is JSON");
        let fields = described["record"]
            .as_array()
            .expect("a record's fields are an array");
        assert!(
            fields
                .iter()
                .all(|f| f.as_array().is_some_and(|f| f.len() == 4)),
            "each field is an id, an optional mark, a type and a default: {described}"
        );
        let default_of = |id: &str| -> serde_json::Value {
            fields
                .iter()
                .find(|f| f[0] == id)
                .unwrap_or_else(|| panic!("`{id}` is described"))[3]
                .clone()
        };
        // A field of the first revision: a map that lacks it is refused.
        assert_eq!(default_of("id"), serde_json::Value::Null);
        // -2, major type 1 of argument 1; the text "none"; `fault`, the number
        // -1 its variant declares; the empty list.
        assert_eq!(default_of("rank"), serde_json::json!("21"));
        assert_eq!(default_of("label"), serde_json::json!("646e6f6e65"));
        assert_eq!(default_of("level"), serde_json::json!("20"));
        assert_eq!(default_of("tags"), serde_json::json!("80"));
        // An optional field's missing key reads absent.
        assert_eq!(default_of("memo"), serde_json::Value::Null);
        // Each is what the plan of the printed codec holds for the field.
        let crate::forge::encode_plan::CborShape::Record(plan) =
            crate::forge::encode_plan::CborShape::of(&record, &scope)
        else {
            panic!("a record is planned as a record")
        };
        assert_eq!(plan.fields.len(), fields.len());
        for field in &plan.fields {
            let planned = field
                .default
                .as_ref()
                .map_or(serde_json::Value::Null, |bytes| {
                    serde_json::Value::String(bytes.iter().map(|b| format!("{b:02x}")).collect())
                });
            assert_eq!(
                default_of(&field.id),
                planned,
                "`{}`: the description and the plan read one value",
                field.id
            );
        }
    }

    #[test]
    fn a_map_has_no_cbor_form() {
        let mut scope = TypeScope::default();
        scope.records.insert(
            "Table".to_string(),
            ScopedRecord {
                identity: "/here/table.scxml".to_string(),
                name: "table".to_string(),
                fields: vec![(
                    "cells".to_string(),
                    TypeOf::Map {
                        key: Box::new(TypeOf::Scalar(SceType::Uint8)),
                        value: Box::new(TypeOf::Scalar(SceType::Uint8)),
                        max: Bound::of(4),
                    },
                )],
                scope: TypeScope::default(),
                added: Default::default(),
                lineage: Default::default(),
            },
        );
        let why = describe(
            &ValueType::Record {
                alias: "Table".to_string(),
            },
            &scope,
        )
        .expect_err("a map has no CBOR form");
        assert!(why.contains("map or a set"), "{why}");
    }

    /// A scope of the record `Probe`, whose one field `v` is of `ty`.
    fn probe_scope(ty: ValueType, enums: TypeScope) -> TypeScope {
        let mut scope = enums.clone();
        scope.records.insert(
            "Probe".to_string(),
            ScopedRecord {
                identity: "/here/probe.scxml".to_string(),
                name: "probe".to_string(),
                fields: vec![("v".to_string(), ty)],
                scope: enums,
                added: Default::default(),
                lineage: Default::default(),
            },
        );
        scope
    }

    fn described(scope: &TypeScope) -> Result<String, String> {
        describe(
            &ValueType::Record {
                alias: "Probe".to_string(),
            },
            scope,
        )
    }

    #[test]
    fn a_float_is_refused_since_no_payload_spells_one_that_is_not_finite() {
        for float in [SceType::Float32, SceType::Float64] {
            let why = described(&probe_scope(TypeOf::Scalar(float), TypeScope::default()))
                .expect_err("a float is refused");
            assert!(why.contains("not finite"), "{why}");
        }
        // Inside a list it is refused as well.
        let listed = TypeOf::List {
            elem: Box::new(TypeOf::Scalar(SceType::Float64)),
            max: Bound::of(2),
        };
        assert!(described(&probe_scope(listed, TypeScope::default())).is_err());
    }

    #[test]
    fn an_open_enum_is_refused_and_a_closed_one_described() {
        let level = |strict: bool| {
            let mut enums = TypeScope::default();
            enums.enums.insert(
                "Level".to_string(),
                ScopedEnum {
                    name: "level".to_string(),
                    variants: vec![ScopedVariant {
                        name: "low".to_string(),
                        value: 0,
                    }],
                    underlying: SceType::Uint8,
                    strict,
                },
            );
            let ty = TypeOf::Scalar(SceType::Enum(crate::forge::model::EnumRef {
                alias: "Level".to_string(),
            }));
            probe_scope(ty, enums)
        };
        let why = described(&level(false)).expect_err("an open enum is refused");
        assert!(why.contains("is open"), "{why}");
        assert_eq!(
            described(&level(true)).expect("a closed enum is described"),
            r#"{"record":[["v",false,{"enum":{"closed":true,"variants":[["low",0]]}},null]]}"#
        );
    }

    #[test]
    fn a_lane_that_reads_no_record_refuses_one_by_name() {
        let model = with(signing("sce-host-cose-sign1", "Note", true));
        for lane in HostLane::all() {
            let refused = refuse_unread(&model, lane);
            if PAYLOAD_RECORD_LANES.contains(&lane) {
                assert!(refused.is_ok(), "{} refused a record", lane.wire_name());
            } else {
                let why = refused
                    .expect_err("a lane that reads no record")
                    .to_string();
                assert!(
                    why.contains("sce:payload=\"Note\"") && why.contains("rust, cpp"),
                    "{why}"
                );
            }
        }
    }

    #[test]
    fn the_zero_of_a_result_field_is_its_types() {
        assert_eq!(
            zero_of(&TypeOf::Scalar(SceType::Bool)),
            serde_json::json!(false)
        );
        assert_eq!(
            zero_of(&TypeOf::Text { max: Bound::of(4) }),
            serde_json::json!("")
        );
        assert_eq!(
            zero_of(&TypeOf::Scalar(SceType::Uint64)),
            serde_json::json!("0")
        );
        assert_eq!(
            zero_of(&TypeOf::Optional {
                elem: Box::new(TypeOf::Record {
                    alias: "Note".to_string()
                })
            }),
            serde_json::Value::Null
        );
    }
}

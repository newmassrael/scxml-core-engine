// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The algorithms a `sce-static` statechart imports (SCE Accepted Subset
// §2.15), resolved where the document is parsed — and the transforms,
// conditions, lookups, interpolations and validators it imports, which it calls
// the same way as the functions of their inputs they are (docs/adr/0048).
//
// An expression of the document calls one as `Alias(args)`, the spelling a
// forge kind calls an imported algorithm by, and it is judged against the
// signature the forge import pass discovers (`discover_stateless_signature`)
// — one reading of a callee, not a second. Unlike the event-schema and enum
// siblings, which the parser reads quietly because a schema it cannot read
// only leaves an event untyped, an algorithm import is read loudly: nothing
// else speaks for it, and a call to a callee nobody resolved would be refused
// as an unknown name, pointing at the call rather than at the import.

use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use crate::forge::error::{ForgeError, ImportError, Located};
use crate::forge::generator::StatelessFunction;
use crate::forge::model::{
    AlgorithmModel, AlgorithmValueType, EnumRef, FieldType, ForgeDocument, ForgeField, ForgeImport,
    ForgeKind, ListElemType, OptionalElemType, OutOfBounds, ParsedForge, SceType,
};
use crate::forge::type_ctx::{CalleeKind, StaticCallee, StaticSlot, StaticValueSignature};
use crate::forge::types::{CallReading, RecordId};
use crate::model::{Datamodel, SCXMLModel};
use crate::scxml_semantic::ScxmlSemanticError;

/// The document a `record:<alias>` of `imports` names, as the identity every
/// reader of that document derives (SCE_FORGE.md §4.12): two imports of one file,
/// by whatever alias or path, name one record, and two schemas that agree field for
/// field but live in two files are two. `None` for an alias no event-schema import
/// of `imports` carries, or a document that cannot be read.
fn record_identity(imports: &[ForgeImport], base_dir: &Path, alias: &str) -> Option<RecordId> {
    record_schema(imports, base_dir, alias).map(|(id, _)| id)
}

/// An enum document as an import of it reads it: the identity every reader of the
/// document derives, by which a record is named too (SCE_FORGE.md §4.12) — two
/// imports of one file, by whatever alias or path, name one enum, and two enums that
/// agree variant for variant but live in two files are two — and whether its
/// variants are a closed set.
#[derive(Debug, Clone)]
struct EnumDocument {
    identity: String,
    strict: bool,
}

/// The identity of the enum document the import `alias` of `imports` names, read
/// from `base_dir` — the name every reader of an enum names it by (SCE_FORGE.md
/// §4.12). `None` for an alias no enum import of `imports` carries, or a document
/// that cannot be read.
pub(crate) fn enum_identity(
    imports: &[ForgeImport],
    base_dir: &Path,
    alias: &str,
) -> Option<String> {
    let import = imports
        .iter()
        .find(|i| i.alias == alias && i.kind == ForgeKind::Enum)?;
    let source = crate::forge::import_source::ImportSource::read(base_dir, import).ok()?;
    Some(crate::forge::stdlib::identity(&source.path))
}

/// The enum document the import `alias` of `imports` names, read from `base_dir`;
/// `None` for an alias no enum import of `imports` carries, or a document that
/// cannot be read.
fn enum_document(imports: &[ForgeImport], base_dir: &Path, alias: &str) -> Option<EnumDocument> {
    let import = imports
        .iter()
        .find(|i| i.alias == alias && i.kind == ForgeKind::Enum)?;
    let source = crate::forge::import_source::ImportSource::read(base_dir, import).ok()?;
    let parsed = source.parse().ok()??;
    let ForgeDocument::Enum(model) = &parsed.document else {
        return None;
    };
    Some(EnumDocument {
        identity: crate::forge::stdlib::identity(&source.path),
        strict: model.strict_variants,
    })
}

/// A field of a record schema, as a record handed to an algorithm is judged by it.
struct SchemaField {
    id: String,
    ty: FieldType,
    /// The `sce:max-size` the schema declares on it, if any.
    max_size: Option<u32>,
    /// The enum document an enum field holds a value of, read from the schema's
    /// own imports; `None` for any other field, or one whose document cannot be
    /// read.
    enum_document: Option<EnumDocument>,
}

/// [`record_identity`], and the fields of the schema it names, each with the
/// `sce:max-size` the schema declares on it and the enum document an enum field
/// holds.
fn record_schema(
    imports: &[ForgeImport],
    base_dir: &Path,
    alias: &str,
) -> Option<(RecordId, Vec<SchemaField>)> {
    let import = imports
        .iter()
        .find(|i| i.alias == alias && i.kind == ForgeKind::EventSchema)?;
    let source = crate::forge::import_source::ImportSource::read(base_dir, import).ok()?;
    let parsed = source.parse().ok()??;
    let ForgeDocument::EventSchema(schema) = &parsed.document else {
        return None;
    };
    // The schema's own imports are written relative to the schema.
    let schema_base = source.path.parent().unwrap_or(base_dir);
    Some((
        RecordId::intern(&crate::forge::stdlib::identity(&source.path), &schema.name),
        schema
            .fields
            .iter()
            .map(|f| SchemaField {
                id: f.id.clone(),
                ty: f.sce_type.clone(),
                max_size: f.max_size,
                enum_document: match f.sce_type.present_type() {
                    SceType::Enum(reference) => {
                        enum_document(&parsed.imports, schema_base, &reference.alias)
                    }
                    _ => None,
                },
            })
            .collect(),
    ))
}

/// How many algorithms deep a callee's refusal is read through the algorithms it calls
/// in turn: past it the callee is refused, where a cycle of imports would otherwise be
/// read without end.
const CALLEE_DEPTH: usize = 16;

/// What of the algorithm `document` (whose own imports are `imports`, written relative
/// to `base_dir`) the Swift backend does not write, as the clause a caller's refusal
/// names it by — `holds a <sce:return> of …` — or `None` when it writes all of it.
/// The algorithm is read through the algorithms it calls, each of which is a unit the
/// Swift backend must write for the call to name one.
pub(crate) fn swift_callee_unlowered(
    document: &ForgeDocument,
    imports: &[ForgeImport],
    base_dir: &Path,
) -> Option<String> {
    swift_callee_unlowered_at(document, imports, base_dir, 0)
}

fn swift_callee_unlowered_at(
    document: &ForgeDocument,
    imports: &[ForgeImport],
    base_dir: &Path,
    depth: usize,
) -> Option<String> {
    let ForgeDocument::Algorithm(algorithm) = document else {
        return None;
    };
    if depth >= CALLEE_DEPTH {
        return Some("calls algorithms nested too deeply to read".to_string());
    }
    crate::forge::generator::swift_unlowered_construct(
        algorithm,
        imports,
        &|imp| {
            let (_, fields) = record_schema(imports, base_dir, &imp.alias)?;
            Some(fields.into_iter().map(|f| (f.id, f.ty)).collect())
        },
        &|imp| swift_imported_algorithm_unlowered(imp, base_dir, depth + 1),
    )
    .map(|construct| format!("holds {construct}"))
}

/// [`swift_callee_unlowered`] of the algorithm `import` names, found relative to
/// `base_dir`, `depth` algorithms down; `None` for one that cannot be read, which the
/// resolution of the import has already refused.
pub(crate) fn swift_imported_algorithm_unlowered(
    import: &ForgeImport,
    base_dir: &Path,
    depth: usize,
) -> Option<String> {
    let source = crate::forge::import_source::ImportSource::read(base_dir, import).ok()?;
    let parsed = source.parse().ok()??;
    let callee_base = source.path.parent().unwrap_or(base_dir);
    swift_callee_unlowered_at(&parsed.document, &parsed.imports, callee_base, depth)
}

/// The alias `model` imports each event-schema under, by the document's identity —
/// what a record slot of an imported algorithm is named by in the statechart.
fn own_record_aliases(model: &SCXMLModel, base_dir: &Path) -> HashMap<RecordId, String> {
    let mut aliases = HashMap::new();
    for import in model
        .forge_imports
        .iter()
        .filter(|i| i.kind == ForgeKind::EventSchema)
    {
        if let Some(id) = record_identity(&model.forge_imports, base_dir, &import.alias) {
            aliases.entry(id).or_insert_with(|| import.alias.clone());
        }
    }
    aliases
}

/// Each enum `model` imports, by the alias it imports it under: what an enum a
/// record's field or a slot of an imported algorithm holds is matched against.
fn own_enum_documents(model: &SCXMLModel, base_dir: &Path) -> BTreeMap<String, EnumDocument> {
    model
        .forge_imports
        .iter()
        .filter(|i| i.kind == ForgeKind::Enum)
        .filter_map(|import| {
            let document = enum_document(&model.forge_imports, base_dir, &import.alias)?;
            Some((import.alias.clone(), document))
        })
        .collect()
}

/// What the statechart is told an algorithm's enum is, when it is not one it can
/// hand over or take back by its declared name. `what` says where the algorithm
/// holds it; `held` is the document, `None` when it cannot be read.
fn crossable_enum<'d>(
    what: &str,
    alias: &str,
    held: Option<&'d EnumDocument>,
) -> Result<&'d EnumDocument, String> {
    let held = held.ok_or_else(|| {
        format!("{what} holds the enum `{alias}`, whose document this call cannot read")
    })?;
    if !held.strict {
        return Err(format!(
            "{what} holds the enum `{alias}`, which declares sce:strict-variants=\"false\": a \
             statechart holds a closed enum, and a value no variant names has no declared name \
             to cross by"
        ));
    }
    Ok(held)
}

/// The signature of `algorithm` as a statechart calls it, when it takes or returns a
/// record, a list or a value of an enum: `Ok(None)` for one with only scalar slots
/// that are no enum (the list of numbers a statechart takes whole, which has its own
/// reading, is among them), and `Err` with the reason when a slot cannot be one a
/// statechart passes or takes.
///
/// A record slot is the statechart's own import of the same schema document; one
/// the statechart does not import has no variable of that type to be passed, and is
/// refused naming the schema. An enum crosses the same way, by document: an enum
/// slot is the statechart's own import of the enum document the algorithm imports,
/// and an enum field of a record is held by the statechart in its type for the
/// enum document the schema imports (docs/adr/0054), which is the record's own and
/// so the algorithm's — the value crosses by the variant's declared name, which
/// only one document gives both sides. A list
/// returned is held to `may-fail` and `returns-max-size`, the two things that let a
/// statechart receive a list that could not be built and know how much room it needs
/// (the rule a list of numbers is already held to).
fn value_signature(
    algorithm: &AlgorithmModel,
    callee_imports: &[ForgeImport],
    callee_base: &Path,
    own: &HashMap<RecordId, String>,
    own_enums: &BTreeMap<String, EnumDocument>,
) -> Result<Option<StaticValueSignature>, String> {
    let signature = &algorithm.signature;
    let holds_enum = |t: &AlgorithmValueType| matches!(t.scalar(), Some(SceType::Enum(_)));
    let state_param = signature
        .params
        .iter()
        .any(|p| p.sce_type.scalar().is_none() || holds_enum(&p.sce_type));
    let state_return = signature.return_type.as_ref().is_some_and(|t| {
        holds_enum(t)
            || (t.scalar().is_none()
                && !matches!(t.list_elem(), Some(ListElemType::Scalar(_)) if !state_param))
    });
    if !state_param && !state_return {
        return Ok(None);
    }
    let record = |alias: &str| -> Result<String, String> {
        let (id, fields) = record_schema(callee_imports, callee_base, alias)
            .ok_or_else(|| format!("its record `{alias}` is a schema this call cannot resolve"))?;
        // A record is copied into the algorithm's own type and back, field by field; the
        // fields of a number or a truth value copy the same way on every backend, a
        // string the schema bounds is held within the same bound on both sides, and an
        // enum field crosses by the declared name of its variant.
        for field in &fields {
            // An optional field is held in the same optional type on both sides,
            // and copies as it is (docs/adr/0055); the present value of an optional
            // value of an enum crosses by the declared name of its variant, as an
            // enum field does.
            let ty = match &field.ty {
                FieldType::Scalar(ty) => ty,
                FieldType::Optional { elem } => {
                    if let SceType::Enum(reference) = elem {
                        let what = format!("the field `{}` of its record `{alias}`", field.id);
                        crossable_enum(&what, &reference.alias, field.enum_document.as_ref())?;
                    }
                    continue;
                }
            };
            if crate::forge::static_lowering::record_field_copies(ty, field.max_size) {
                if let SceType::Enum(reference) = ty {
                    let what = format!("the field `{}` of its record `{alias}`", field.id);
                    // The machine holds the field in its own type for the enum
                    // document the schema imports, whether or not the statechart
                    // imports it too (docs/adr/0054), so that is the document the
                    // value crosses from and back to, and it needs no import here.
                    crossable_enum(&what, &reference.alias, field.enum_document.as_ref())?;
                }
                continue;
            }
            return Err(match (ty, field.max_size) {
                (SceType::String, None) => format!(
                    "its record `{alias}` has a string field with no `sce:max-size`, which \
                     bounds the text an algorithm's record holds"
                ),
                (held, _) => format!(
                    "its record `{alias}` has a field of type {}, and only records of numbers, \
                     truth values, bounded strings and enums are handed to an algorithm",
                    held.as_attr()
                ),
            });
        }
        own.get(&id).cloned().ok_or_else(|| {
            format!(
                "it names a record of the schema `{alias}` stands for, and the statechart \
                 does not import that document as an event-schema"
            )
        })
    };
    // An enum slot is named, as a record slot is, by the alias the statechart imports
    // the same document under: the type of a value of that enum, which a slot of an
    // optional value of it holds too.
    let enum_slot = |name: &str, alias: &str| -> Result<SceType, String> {
        let what = format!("its {name}");
        let held = crossable_enum(
            &what,
            alias,
            enum_document(callee_imports, callee_base, alias).as_ref(),
        )?
        .clone();
        let own_alias = own_enums
            .iter()
            .find(|(_, own)| own.identity == held.identity)
            .map(|(own_alias, _)| own_alias.clone())
            .ok_or_else(|| {
                format!(
                    "{what} holds the enum `{alias}` of {}, which the statechart does not \
                     import as an enum",
                    held.identity
                )
            })?;
        Ok(SceType::Enum(EnumRef { alias: own_alias }))
    };
    let slot = |name: &str, ty: &AlgorithmValueType| -> Result<StaticSlot, String> {
        match ty {
            AlgorithmValueType::Scalar(SceType::Enum(reference)) => {
                enum_slot(name, &reference.alias).map(StaticSlot::Scalar)
            }
            AlgorithmValueType::Scalar(t) => Ok(StaticSlot::Scalar(t.clone())),
            AlgorithmValueType::Record { alias } => Ok(StaticSlot::Record(record(alias)?)),
            AlgorithmValueType::List {
                elem: ListElemType::Scalar(t),
            } => {
                if crate::forge::types::ListElem::of(t).is_none() {
                    return Err(format!(
                        "a list of {} is not one a statechart holds",
                        t.as_attr()
                    ));
                }
                Ok(StaticSlot::ScalarList(t.clone()))
            }
            AlgorithmValueType::List {
                elem: ListElemType::Record { alias },
            } => Ok(StaticSlot::RecordList(record(alias)?)),
            // A value that may be absent, of a number, a `bool` or a text: the same
            // type on both sides, handed over and taken back as it is
            // (docs/adr/0055).
            //
            // One of an enum is named, as an enum slot is, by the alias the
            // statechart imports the document under, and crosses by the declared name
            // of its variant where it is present, as a value of the enum does.
            AlgorithmValueType::Optional {
                elem: OptionalElemType::Scalar(SceType::Enum(reference)),
            } => enum_slot(name, &reference.alias).map(StaticSlot::Optional),
            AlgorithmValueType::Optional {
                elem: OptionalElemType::Scalar(SceType::Bytes),
            } => Err(format!(
                "its {name} is of type {}, and a byte string an algorithm holds is not the one \
                 a statechart's variable holds, so no call crosses an optional one \
                 (docs/adr/0055)",
                ty.as_attr()
            )),
            AlgorithmValueType::Optional {
                elem: OptionalElemType::Scalar(t),
            } => Ok(StaticSlot::Optional(t.clone())),
            // An optional record is named, as a record is, by the alias the
            // statechart imports the same schema under (docs/adr/0055, step 9).
            AlgorithmValueType::Optional {
                elem: OptionalElemType::Record { alias },
            } => Ok(StaticSlot::OptionalRecord(record(alias)?)),
        }
    };
    let params = signature
        .params
        .iter()
        .map(|p| slot(&format!("parameter `{}`", p.name), &p.sce_type))
        .collect::<Result<Vec<_>, _>>()?;
    let ret = signature
        .return_type
        .as_ref()
        .map(|t| slot("return", t))
        .transpose()?;
    let max_size = match &ret {
        Some(StaticSlot::ScalarList(_) | StaticSlot::RecordList(_)) => {
            if !signature.may_fail {
                return Err("it returns a list and does not declare may-fail on its \
                            <sce:return>, which is how a statechart receives the failure of a \
                            list that cannot be built"
                    .to_string());
            }
            Some(signature.returns_max_size.ok_or_else(|| {
                "it returns a list and declares no returns-max-size, and the list variable \
                 it is assigned to must be able to hold all the algorithm may return"
                    .to_string()
            })?)
        }
        _ => None,
    };
    Ok(Some(StaticValueSignature {
        params,
        ret,
        max_size,
    }))
}

/// The kinds a `sce-static` statechart calls as `Alias(args)` (docs/adr/0048): the
/// algorithm, and the kinds that are functions of their inputs — a transform, a
/// condition, a lookup, an interpolation and a validator. Every other kind keeps
/// state or declares a type, and is not called.
const CALLED_KINDS: [ForgeKind; 6] = [
    ForgeKind::Algorithm,
    ForgeKind::Transform,
    ForgeKind::Condition,
    ForgeKind::Lookup,
    ForgeKind::Interpolation,
    ForgeKind::Validator,
];

/// Each import of `model` of a kind a statechart calls ([`CALLED_KINDS`]),
/// resolved against the files beside it in `base_dir`.
///
/// Refused: an algorithm import under any data model but `sce-static`, which
/// is the only one that lowers a call; an import whose file is missing, is
/// not a forge document, or is a document of another kind; and an import of
/// another kind a statechart cannot call as it is declared ([`function_callee`]).
/// An import of another kind under any other data model is not read here.
pub(crate) fn resolve(
    model: &SCXMLModel,
    base_dir: &Path,
    diag_label: &str,
) -> Result<Vec<StaticCallee>, Located<ForgeError>> {
    let mut callees = Vec::new();
    let own = own_record_aliases(model, base_dir);
    let own_enums = own_enum_documents(model, base_dir);
    for import in model
        .forge_imports
        .iter()
        .filter(|i| CALLED_KINDS.contains(&i.kind))
    {
        if model.datamodel != Datamodel::SceStatic {
            if import.kind != ForgeKind::Algorithm {
                continue;
            }
            return Err(Located::new(
                ScxmlSemanticError::StaticDatamodelRule {
                    construct: format!("<sce:import kind=\"algorithm\" as=\"{}\">", import.alias),
                    datamodel: model.datamodel.as_str().to_string(),
                    rule: "a statechart calls an imported algorithm only under \
                           datamodel=\"sce-static\", the one data model that lowers the call"
                        .to_string(),
                    state: String::new(),
                    observed: Some(import.alias.clone()),
                }
                .into(),
                diag_label,
                import.line,
                None,
            ));
        }
        let source = crate::forge::import_source::ImportSource::read(base_dir, import)
            .map_err(|e| Located::new(e.into(), diag_label, import.line, None))?;
        let not_forge = || {
            Located::new(
                ForgeError::Import(ImportError::NotForge {
                    src: import.src.clone(),
                }),
                diag_label,
                import.line,
                None,
            )
        };
        let parsed = source.parse()?.ok_or_else(not_forge)?;
        if parsed.document.kind() != import.kind {
            return Err(Located::new(
                ImportError::KindMismatch {
                    src: import.src.clone(),
                    declared: import.kind.to_string(),
                    actual: parsed.document.kind().to_string(),
                }
                .into(),
                diag_label,
                import.line,
                None,
            ));
        }
        let ForgeDocument::Algorithm(algorithm) = &parsed.document else {
            // A transform, a condition, a lookup, an interpolation or a validator:
            // called with the arguments as they are, or refused where it is
            // imported, naming why. Its own imports are written relative to it.
            let function_base = source.path.parent().unwrap_or(base_dir);
            let callee =
                function_callee(import, &parsed, function_base, &own_enums).map_err(|reason| {
                    Located::new(
                        ScxmlSemanticError::StaticDatamodelRule {
                            construct: format!(
                                "<sce:import kind=\"{}\" as=\"{}\">",
                                import.kind, import.alias
                            ),
                            datamodel: model.datamodel.as_str().to_string(),
                            rule: format!(
                                "a statechart calls an imported {} as a function of its \
                             arguments, and `{}` is not one: {reason}",
                                import.kind, import.alias
                            ),
                            state: String::new(),
                            observed: Some(import.alias.clone()),
                        }
                        .into(),
                        diag_label,
                        import.line,
                        None,
                    )
                })?;
            callees.push(callee);
            continue;
        };
        let signature = crate::discover_stateless_signature(&parsed.document);
        // An algorithm that takes or returns a record or a list is called with the
        // statechart's own variables named at the call; the signature it is called by
        // is the statechart's reading of that, and replaces the host-only refusal
        // when it holds. When it does not, the reason is added to the refusal.
        let callee_base = source.path.parent().unwrap_or(base_dir);
        let (value, host_only, refused_why) =
            match value_signature(algorithm, &parsed.imports, callee_base, &own, &own_enums) {
                Ok(Some(value)) => (Some(value), None, None),
                Ok(None) => (None, signature.host_only, None),
                Err(reason) => (
                    None,
                    Some(format!(
                        "{}: {reason}",
                        signature
                            .host_only
                            .unwrap_or_else(|| "has a slot a statechart cannot pass".to_string())
                    )),
                    Some(reason),
                ),
            };
        callees.push(StaticCallee {
            alias: import.alias.clone(),
            document_name: parsed.document.name().to_string(),
            kind: CalleeKind::Function(StatelessFunction::Algorithm {
                name: algorithm.name.clone(),
            }),
            reading: CallReading::Value,
            params: signature.params,
            ret: signature.ret,
            host_only,
            may_fail: signature.may_fail,
            // A list of numbers returned by an algorithm that takes a value of an enum
            // is taken through the adapter that maps the enum, by the signature above,
            // and not as the scalars-only list it would otherwise be.
            list_return: signature
                .list_return
                .filter(|_| value.is_none())
                .map(|returned| crate::forge::type_ctx::StaticListReturn {
                    params: returned.params,
                    elem: returned.elem,
                    max_size: returned.max_size,
                }),
            // A list the statechart now takes with a record in the signature is not
            // a refused one.
            // ... and one the statechart cannot take says why, in place of the general
            // reason that a statechart hands an algorithm scalars only.
            list_refusal: if value.is_some() {
                None
            } else {
                match (signature.list_refusal, refused_why) {
                    (Some(_), Some(why)) => Some(why),
                    (general, _) => general,
                }
            },
            value,
            line: import.line,
            // The schema an import of the algorithm names is read where the algorithm
            // is, as its own generation reads it.
            unlowered_by: crate::forge::generator::unlowered_in_part(
                &parsed.document,
                &parsed.imports,
                &|imp| {
                    let (_, fields) = record_schema(&parsed.imports, callee_base, &imp.alias)?;
                    Some(fields.into_iter().map(|f| (f.id, f.ty)).collect())
                },
                // The algorithms it calls in turn are read where it is.
                &|imp| swift_imported_algorithm_unlowered(imp, callee_base, 1),
            ),
        });
    }
    Ok(callees)
}

/// The callee `import` is to a statechart when it names a transform, a condition, a
/// lookup, an interpolation or a validator (docs/adr/0048), `parsed` being the
/// document it names (its own imports read from `callee_base`): the function the
/// document emits, called with the statechart's values and answering one, with the
/// signature a forge kind that imports the document calls it by
/// ([`crate::discover_stateless_signature`]) — a validator's check takes its inputs
/// and answers whether they are valid. A slot is a number or a truth value, a value
/// of an enum the statechart imports from the same document (`own_enums`), which
/// crosses by the name its variant declares, or, for an input, a string
/// (docs/adr/0051). `Err` says why a statechart cannot call it as the document
/// declares it:
///
/// - a transform of more than one output, which no one call names the value of, and
///   one that reads `previous()`, which keeps a value from one activation to the next
///   that a call keeps nowhere;
/// - a lookup whose output is a string, which it answers as a type of its own that no
///   statechart variable holds, and one keyed by a string, which no backend's table
///   compares alike;
/// - an interpolation whose `sce:out-of-bounds` is not `clamp`, which its generated
///   function does not do on any backend;
/// - a validator that keeps state — a rate-of-change rule, a stateful import — which a
///   validator built for each call would forget;
/// - a slot of an enum the statechart does not import from the same document, or of
///   an open enum; an output that is a string, which a call answers with no bound;
///   and a slot that is a byte string.
///
/// A lookup whose miss is an error may fail: a key its table does not hold is the
/// call's `out-of-range` failure ([`CallReading::Found`]), received as any call's.
fn function_callee(
    import: &ForgeImport,
    parsed: &ParsedForge,
    callee_base: &Path,
    own_enums: &BTreeMap<String, EnumDocument>,
) -> Result<StaticCallee, String> {
    let document = &parsed.document;
    // The slots, by the names the document gives them, for a refusal to name.
    let (inputs, output): (Vec<&ForgeField>, Option<&ForgeField>) = match document {
        ForgeDocument::Transform(m) => {
            if m.outputs.len() != 1 {
                return Err(format!(
                    "it computes {} outputs, and a call is the value of one",
                    m.outputs.len()
                ));
            }
            if let Some(read) = crate::forge::previous_value::first_read(m, |_| true) {
                return Err(format!(
                    "its output `{}` reads `previous({})`, a value it keeps from one \
                     activation to the next, and a call keeps nothing",
                    read.output, read.field
                ));
            }
            (m.inputs.iter().collect(), m.outputs.first())
        }
        ForgeDocument::Condition(m) => (m.inputs.iter().collect(), None),
        ForgeDocument::Lookup(m) => {
            if m.output_is_string() {
                return Err(format!(
                    "its output `{}` is a string, which a lookup answers as a type of its \
                     own that no statechart variable holds",
                    m.output.id
                ));
            }
            (vec![&m.input], Some(&m.output))
        }
        ForgeDocument::Interpolation(m) => {
            if !matches!(m.out_of_bounds, OutOfBounds::Clamp) {
                return Err(format!(
                    "it declares sce:out-of-bounds=\"{}\", and the function generated for \
                     it clamps on every backend",
                    m.out_of_bounds.as_str()
                ));
            }
            (m.inputs.iter().collect(), Some(&m.output))
        }
        ForgeDocument::Validator(m) => {
            if let Some(rule) = m.rules.rate_of_changes.first() {
                return Err(format!(
                    "its input `{}` declares a rate of change, which compares a value with \
                     the one read before it, and a validator a statechart calls is built \
                     for the call",
                    rule.id
                ));
            }
            if let Some(held) = parsed.imports.iter().find(|i| i.kind.needs_instance()) {
                return Err(format!(
                    "it imports the {} `{}`, which keeps state from one check to the next, \
                     and a validator a statechart calls is built for the call",
                    held.kind, held.alias
                ));
            }
            (m.inputs.iter().collect(), None)
        }
        other => {
            return Err(format!(
                "it is a document of the kind {}, which a statechart does not call",
                other.kind()
            ))
        }
    };
    // Each slot as the statechart holds it: a value of an enum by the alias the
    // statechart imports the same document under, any other slot as it is.
    let is_lookup = matches!(document, ForgeDocument::Lookup(_));
    let slot = |what: &str, field: &ForgeField| -> Result<SceType, String> {
        match &field.sce_type {
            SceType::Enum(reference) => {
                let what = format!("its {what} `{}`", field.id);
                let read = enum_document(&parsed.imports, callee_base, &reference.alias);
                let held = crossable_enum(&what, &reference.alias, read.as_ref())?;
                let own = own_enums
                    .iter()
                    .find(|(_, own)| own.identity == held.identity)
                    .map(|(own_alias, _)| own_alias.clone())
                    .ok_or_else(|| {
                        format!(
                            "{what} holds the enum `{}` of {}, which the statechart does not \
                             import as an enum",
                            reference.alias, held.identity
                        )
                    })?;
                Ok(SceType::Enum(EnumRef { alias: own }))
            }
            SceType::String if what == "input" && is_lookup => Err(format!(
                "its input `{}` is a string, and a lookup's table is keyed by numbers or by \
                 the variants of an enum, which every backend compares alike (key it by an \
                 enum)",
                field.id
            )),
            SceType::String if what == "input" => Ok(SceType::String),
            SceType::String => Err(format!(
                "its output `{}` is a string, which a call answers with no bound a statechart \
                 holds it within",
                field.id
            )),
            SceType::Bytes => Err(format!(
                "its {what} `{}` is a byte string, which a statechart hands such a document \
                 on no backend alike",
                field.id
            )),
            other => Ok(other.clone()),
        }
    };
    let params = inputs
        .iter()
        .map(|field| slot("input", field))
        .collect::<Result<Vec<_>, _>>()?;
    let output = output.map(|field| slot("output", field)).transpose()?;
    let (kind, ret, may_fail, reading) = match document {
        ForgeDocument::Validator(_) => (
            CalleeKind::Validator,
            SceType::Bool,
            false,
            CallReading::Validity,
        ),
        _ => {
            let function = StatelessFunction::of(document)
                .ok_or_else(|| "it emits no function a caller is named".to_string())?;
            // A condition answers its truth value; every other kind its one output.
            let ret = match (document, output) {
                (ForgeDocument::Condition(_), _) => SceType::Bool,
                (_, Some(ty)) => ty,
                (_, None) => return Err("it answers no value".to_string()),
            };
            let misses = matches!(document, ForgeDocument::Lookup(m) if m.miss_policy.is_error());
            let reading = if misses {
                CallReading::Found
            } else {
                CallReading::Value
            };
            (CalleeKind::Function(function), ret, misses, reading)
        }
    };
    Ok(StaticCallee {
        alias: import.alias.clone(),
        document_name: document.name().to_string(),
        kind,
        reading,
        params,
        ret: Some(ret),
        host_only: None,
        may_fail,
        line: import.line,
        list_return: None,
        list_refusal: None,
        value: None,
        // Only an algorithm is written in part for a language, and this is not one.
        unlowered_by: crate::forge::generator::unlowered_in_part(
            document,
            &parsed.imports,
            &|_| None,
            &|_| None,
        ),
    })
}

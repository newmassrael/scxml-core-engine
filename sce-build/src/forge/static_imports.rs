// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The algorithms a `sce-static` statechart imports (SCE Accepted Subset
// §2.15), resolved where the document is parsed.
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
use crate::forge::model::{
    AlgorithmModel, AlgorithmValueType, EnumRef, ForgeDocument, ForgeImport, ForgeKind,
    ListElemType, SceType,
};
use crate::forge::type_ctx::{StaticCallee, StaticSlot, StaticValueSignature};
use crate::forge::types::RecordId;
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
    ty: SceType,
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
                enum_document: match &f.sce_type {
                    SceType::Enum(reference) => {
                        enum_document(&parsed.imports, schema_base, &reference.alias)
                    }
                    _ => None,
                },
            })
            .collect(),
    ))
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
/// and an enum field of a record is held by the statechart under the alias the
/// schema writes, which must name that same document — the value crosses by the
/// variant's declared name, which only one document gives both sides. A list
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
            if crate::forge::static_lowering::record_field_copies(&field.ty, field.max_size) {
                if let SceType::Enum(reference) = &field.ty {
                    let what = format!("the field `{}` of its record `{alias}`", field.id);
                    let held =
                        crossable_enum(&what, &reference.alias, field.enum_document.as_ref())?;
                    // The machine holds the field in its own type for the enum it
                    // imports under the alias the schema writes, so that is the
                    // document the value crosses from and back to.
                    match own_enums.get(&reference.alias) {
                        Some(own) if own.identity == held.identity => {}
                        Some(own) => {
                            return Err(format!(
                                "{what} holds the enum `{}` of {}, and the statechart imports \
                                 `{}` from {}: an enum crosses to an algorithm only as the one \
                                 enum document both import",
                                reference.alias, held.identity, reference.alias, own.identity
                            ))
                        }
                        None => {
                            return Err(format!(
                                "{what} holds the enum `{}` of {}, and the statechart imports no \
                                 enum under that alias, which its record holds the field in",
                                reference.alias, held.identity
                            ))
                        }
                    }
                }
                continue;
            }
            return Err(match (&field.ty, field.max_size) {
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
    // the same document under.
    let enum_slot = |name: &str, alias: &str| -> Result<StaticSlot, String> {
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
        Ok(StaticSlot::Scalar(SceType::Enum(EnumRef {
            alias: own_alias,
        })))
    };
    let slot = |name: &str, ty: &AlgorithmValueType| -> Result<StaticSlot, String> {
        match ty {
            AlgorithmValueType::Scalar(SceType::Enum(reference)) => {
                enum_slot(name, &reference.alias)
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

/// Each `<sce:import kind="algorithm">` of `model`, resolved against the
/// files beside it in `base_dir`.
///
/// Refused: an algorithm import under any data model but `sce-static`, which
/// is the only one that lowers a call; an import whose file is missing, is
/// not a forge document, or is a document of another kind.
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
        .filter(|i| i.kind == ForgeKind::Algorithm)
    {
        if model.datamodel != Datamodel::SceStatic {
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
        let ForgeDocument::Algorithm(algorithm) = &parsed.document else {
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
        });
    }
    Ok(callees)
}

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

use std::collections::HashMap;
use std::path::Path;

use crate::forge::error::{ForgeError, ImportError, Located};
use crate::forge::model::{
    AlgorithmModel, AlgorithmValueType, ForgeDocument, ForgeImport, ForgeKind, ListElemType,
    SceType,
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

/// [`record_identity`], and the fields of the schema it names, each with the
/// `sce:max-size` the schema declares on it.
fn record_schema(
    imports: &[ForgeImport],
    base_dir: &Path,
    alias: &str,
) -> Option<(RecordId, Vec<(SceType, Option<u32>)>)> {
    let import = imports
        .iter()
        .find(|i| i.alias == alias && i.kind == ForgeKind::EventSchema)?;
    let source = crate::forge::import_source::ImportSource::read(base_dir, import).ok()?;
    let parsed = source.parse().ok()??;
    let ForgeDocument::EventSchema(schema) = &parsed.document else {
        return None;
    };
    Some((
        RecordId::intern(&crate::forge::stdlib::identity(&source.path), &schema.name),
        schema
            .fields
            .iter()
            .map(|f| (f.sce_type.clone(), f.max_size))
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

/// The signature of `algorithm` as a statechart calls it, when it takes or returns a
/// record or a list: `Ok(None)` for one with only scalar slots (the list of numbers
/// a statechart takes whole, which has its own reading, is among them), and `Err`
/// with the reason when a slot cannot be one a statechart passes or takes.
///
/// A record slot is the statechart's own import of the same schema document; one
/// the statechart does not import has no variable of that type to be passed, and is
/// refused naming the schema. A list returned is held to `may-fail` and
/// `returns-max-size`, the two things that let a statechart receive a list that
/// could not be built and know how much room it needs (the rule a list of numbers is
/// already held to).
fn value_signature(
    algorithm: &AlgorithmModel,
    callee_imports: &[ForgeImport],
    callee_base: &Path,
    own: &HashMap<RecordId, String>,
) -> Result<Option<StaticValueSignature>, String> {
    let signature = &algorithm.signature;
    let state_param = signature
        .params
        .iter()
        .any(|p| p.sce_type.scalar().is_none());
    let state_return = signature.return_type.as_ref().is_some_and(|t| {
        t.scalar().is_none()
            && !matches!(t.list_elem(), Some(ListElemType::Scalar(_)) if !state_param)
    });
    if !state_param && !state_return {
        return Ok(None);
    }
    let record = |alias: &str| -> Result<String, String> {
        let (id, fields) = record_schema(callee_imports, callee_base, alias)
            .ok_or_else(|| format!("its record `{alias}` is a schema this call cannot resolve"))?;
        // A record is copied into the algorithm's own type and back, field by field; the
        // fields of a number or a truth value copy the same way on every backend, and a
        // string the schema bounds is held within the same bound on both sides.
        if let Some((held, max_size)) = fields
            .iter()
            .find(|(ty, max)| !crate::forge::static_lowering::record_field_copies(ty, *max))
        {
            return Err(match (held, max_size) {
                (SceType::String, None) => format!(
                    "its record `{alias}` has a string field with no `sce:max-size`, which \
                     bounds the text an algorithm's record holds"
                ),
                _ => format!(
                    "its record `{alias}` has a field of type {}, and only records of numbers, \
                     truth values and bounded strings are handed to an algorithm",
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
    let slot = |ty: &AlgorithmValueType| -> Result<StaticSlot, String> {
        match ty {
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
        .map(|p| slot(&p.sce_type))
        .collect::<Result<Vec<_>, _>>()?;
    let ret = signature.return_type.as_ref().map(slot).transpose()?;
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
            match value_signature(algorithm, &parsed.imports, callee_base, &own) {
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
            list_return: signature.list_return.map(|returned| {
                crate::forge::type_ctx::StaticListReturn {
                    params: returned.params,
                    elem: returned.elem,
                    max_size: returned.max_size,
                }
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

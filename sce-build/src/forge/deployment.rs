// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A deployment of a Rust codec tree, generated into a file of its own
//! (docs/adr/0077 §7).
//!
//! A Rust tree takes a storage profile and serves every deployment in a build,
//! so no capacity is written into it. A deployment is a type: its file declares
//! it and answers, for each field of each codec of the set the tree declares a
//! tag for, what it holds that field in. That is the capacity the deploy input
//! gives the field, or the field's bound where it gives none, which is what
//! `Inline` holds it at. Several deployments coexist in one build, each its own
//! type in its own file.
//!
//! The file sits beside the tree's modules and reaches each codec's tags as the
//! tree's modules reach one another, `super::<codec>`. A codec the consumer
//! compiles under a `cfg` of its own is answered for under the same one, which
//! the caller names: the gate is the build's layout, and neither the documents
//! nor the deploy input say it.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::filters;
use crate::forge::error::{ForgeError, Located, ValidationError};
use crate::forge::import_source::ImportSource;
use crate::forge::model::{CodecCapacities, CodecModel, ForgeDocument, ForgeImport, ForgeKind};

/// Which kind of node a deployment's storing forms run on (docs/adr/0077 §7),
/// the deploy input's `codec_storage:`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NodeStorage {
    /// A node with no allocator, or a path that must not allocate: every field
    /// is held in an array, at its capacity or its bound, and a field with
    /// neither has no storage and is refused.
    #[default]
    Inline,
    /// A node with an allocator: a field the deployment gives a capacity is held
    /// on the heap to that capacity, its policy limit, and refused past it as an
    /// inline node refuses past its array; every other field grows.
    Heap,
}

/// What a deployment's file is generated from, beside the document set.
#[derive(Debug, Clone, Default)]
pub struct RustDeployment {
    /// The type the file declares, a Rust type name. The file is named by it.
    pub name: String,
    /// What the deployment's storing forms hold, from its deploy input.
    pub capacities: CodecCapacities,
    /// Which kind of node they run on, from its deploy input.
    pub storage: NodeStorage,
    /// The `cfg` predicate each codec of the set is compiled under, by codec
    /// name, for a codec the consumer compiles under one: `feature = "serial"`.
    pub gates: BTreeMap<String, String>,
    /// What the deploy input is called, for the file's header and for a
    /// refusal of what it says.
    pub source: String,
}

/// Why a deployment's file is not generated.
#[derive(Debug)]
pub enum RustDeploymentError {
    /// A document of the set, or the deploy input, is refused.
    Refused(Located<ForgeError>),
    /// What the caller asked for is not one the set can answer: a type name
    /// that is not one, or a gate for a codec the set does not hold.
    Usage(String),
}

impl std::fmt::Display for RustDeploymentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Refused(e) => e.fmt(f),
            Self::Usage(detail) => f.write_str(detail),
        }
    }
}

impl std::error::Error for RustDeploymentError {}

impl From<Located<ForgeError>> for RustDeploymentError {
    fn from(e: Located<ForgeError>) -> Self {
        Self::Refused(e)
    }
}

/// A codec of the set, with the path of the document it was read from.
struct Member {
    model: CodecModel,
    label: String,
}

/// The codecs a set holds or imports, by name, and the paths of every document
/// read for them.
struct Closure {
    members: BTreeMap<String, Member>,
    deps: Vec<PathBuf>,
}

/// A document read and not yet walked: where its imports resolve from, what it
/// is called, what it imports, and the codec it is, if it is one.
struct Read {
    base: PathBuf,
    label: String,
    imports: Vec<ForgeImport>,
    codec: Option<CodecModel>,
}

/// Every codec `documents` hold or import, each read once. A codec imported by
/// two documents is the same codec, as it is one module of the tree.
fn codec_closure(
    documents: &[&Path],
    include_dirs: &[PathBuf],
) -> Result<Closure, Located<ForgeError>> {
    let mut members = BTreeMap::new();
    let mut deps = Vec::new();
    let mut seen = BTreeSet::new();
    let mut pending: Vec<Read> = Vec::new();
    for path in documents {
        let loaded = crate::load_forge_source(path, include_dirs)?;
        let label = path.to_string_lossy().into_owned();
        deps.extend(loaded.deps.iter().cloned());
        deps.push(path.to_path_buf());
        let Some(parsed) = crate::forge::parser::parse_forge_with_imports(
            loaded.text(),
            crate::DocumentLabel::for_input_path(&label),
        )
        .map_err(|e| loaded.positions.authored(e))?
        else {
            continue;
        };
        seen.insert(path.to_path_buf());
        let base = path.parent().map(Path::to_path_buf).unwrap_or_default();
        pending.push(Read {
            base,
            label,
            imports: parsed.imports,
            codec: codec_of(parsed.document),
        });
    }
    while let Some(Read {
        base,
        label,
        imports,
        codec,
    }) = pending.pop()
    {
        if let Some(model) = codec {
            members.entry(model.name.clone()).or_insert(Member {
                model,
                label: label.clone(),
            });
        }
        for import in imports.iter().filter(|i| i.kind == ForgeKind::Codec) {
            let source = ImportSource::read(&base, import)
                .map_err(|e| Located::new(e.into(), label.clone(), import.line, None))?;
            if !seen.insert(source.path.clone()) {
                continue;
            }
            let Some(parsed) = source.parse()? else {
                continue;
            };
            deps.push(source.path.clone());
            pending.push(Read {
                base: source
                    .path
                    .parent()
                    .map(Path::to_path_buf)
                    .unwrap_or_default(),
                label: source.label().diagnostic_label.to_string(),
                imports: parsed.imports,
                codec: codec_of(parsed.document),
            });
        }
    }
    Ok(Closure { members, deps })
}

fn codec_of(document: ForgeDocument) -> Option<CodecModel> {
    match document {
        ForgeDocument::Codec(m) => Some(m),
        _ => None,
    }
}

/// The context the deployment's template renders, for the set `documents`
/// under `deployment`, and the paths of every document read.
pub(crate) fn context(
    documents: &[&Path],
    include_dirs: &[PathBuf],
    deployment: &RustDeployment,
) -> Result<(serde_json::Map<String, serde_json::Value>, Vec<PathBuf>), RustDeploymentError> {
    if !is_type_name(&deployment.name) {
        return Err(RustDeploymentError::Usage(format!(
            "'{}' is not a Rust type name: a deployment is a type, named in upper camel case",
            deployment.name
        )));
    }
    let Closure { mut members, deps } = codec_closure(documents, include_dirs)?;
    if let Some(codec) = deployment
        .gates
        .keys()
        .find(|codec| !members.contains_key(*codec))
    {
        return Err(RustDeploymentError::Usage(format!(
            "a gate is given for '{codec}', and no document of the set is a codec of that name"
        )));
    }
    if let Some((codec, fields)) = deployment
        .capacities
        .iter()
        .find(|(codec, _)| !members.contains_key(*codec))
    {
        return Err(Located::new(
            ValidationError::CodecCapacityInvalid {
                codec: codec.clone(),
                field: fields.keys().next().cloned().unwrap_or_default(),
                rule: "no document of the set is a codec of this name".to_string(),
            }
            .into(),
            deployment.source.clone(),
            None,
            None,
        )
        .into());
    }
    let mut codecs = Vec::new();
    for (name, member) in &mut members {
        member
            .model
            .take_capacities(&deployment.capacities)
            .map_err(|e| Located::in_file(e, member.label.clone()))?;
        let fields = crate::forge::generator::rust_storage_fields(&member.model);
        if fields.is_empty() {
            continue;
        }
        // An inline deployment holds every field in an array, so a field the
        // protocol bounds by nothing needs a capacity from it; on a heap node
        // such a field grows (docs/adr/0077 §7).
        let unheld = (deployment.storage == NodeStorage::Inline)
            .then(|| fields.iter().find(|f| f["capacity"].is_null()))
            .flatten();
        if let Some(unheld) = unheld {
            return Err(Located::in_file(
                ValidationError::CodecCapacityInvalid {
                    codec: name.clone(),
                    field: unheld["id"].as_str().unwrap_or_default().to_string(),
                    rule: "the protocol states no bound of this field, and a deployment holds \
                           every field inline, so it needs a capacity"
                        .to_string(),
                }
                .into(),
                deployment.source.clone(),
            )
            .into());
        }
        codecs.push(serde_json::json!({
            "name": name,
            "module": format!("super::{}", filters::to_snake_case(name.clone())),
            "cfg": deployment.gates.get(name).map(|predicate| format!("#[cfg({predicate})]")),
            "fields": fields,
        }));
    }
    let mut ctx = serde_json::Map::new();
    ctx.insert("deployment".into(), deployment.name.clone().into());
    ctx.insert("source".into(), deployment.source.clone().into());
    ctx.insert(
        "heap".into(),
        (deployment.storage == NodeStorage::Heap).into(),
    );
    ctx.insert("codecs".into(), codecs.into());
    Ok((ctx, deps))
}

/// Whether `name` is a type name in upper camel case: a letter in upper case,
/// then letters and digits.
fn is_type_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars.next().is_some_and(|c| c.is_ascii_uppercase()) && chars.all(|c| c.is_ascii_alphanumeric())
}

/// The file a deployment named `name` is written to.
pub(crate) fn file_name(name: &str) -> String {
    format!("{}.rs", filters::to_snake_case(name.to_string()))
}

#[cfg(test)]
mod tests {
    use super::is_type_name;

    #[test]
    fn a_deployment_is_named_as_a_type_is() {
        assert!(is_type_name("Mcu"));
        assert!(is_type_name("Router2"));
        assert!(!is_type_name("mcu"));
        assert!(!is_type_name("Mcu_Node"));
        assert!(!is_type_name(""));
    }
}

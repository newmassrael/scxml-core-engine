// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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

use std::path::Path;

use crate::forge::error::{ForgeError, ImportError, Located};
use crate::forge::model::{ForgeDocument, ForgeKind};
use crate::forge::type_ctx::StaticCallee;
use crate::model::{Datamodel, SCXMLModel};
use crate::scxml_semantic::ScxmlSemanticError;

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
        let ForgeDocument::Algorithm(_) = &parsed.document else {
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
        callees.push(StaticCallee {
            alias: import.alias.clone(),
            document_name: parsed.document.name().to_string(),
            params: signature.params,
            ret: signature.ret,
            host_only: signature.host_only,
            line: import.line,
        });
    }
    Ok(callees)
}

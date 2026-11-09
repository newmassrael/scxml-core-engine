// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The documents a test suite reaches, read once each and held to the rules
//! of their kinds (docs/SCE_TEST_PROFILE.md §9.1, §9.5).
//!
//! The walk follows the imports of the suite's test cases, their
//! configurations, the test components those name and the port types,
//! templates, event schemas, enums and unions below them: every document the
//! translation `T` of §9.1 reads. A feature model, and any document of
//! another kind a component imports, is not followed: no rule of §9.2 reads
//! one into the types module or into a template.
//!
//! Each forge document reached is held to the rules of its kind that no
//! backend decides, the ones `check` holds it to before any language is
//! asked ([`check_document`]), so the export never translates a document
//! `check` refuses. A test component is a statechart, read by the statechart
//! parser, which holds it to the rules of the profile as it reads it.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use super::names::{unit_name, Origin};
use crate::forge::error::{ForgeError, ImportError, Located};
use crate::forge::import_source::{type_scope, ImportSource};
use crate::forge::model::{ForgeDocument, ForgeImport, ForgeKind, ParsedForge};
use crate::forge::stdlib;
use crate::forge::value_type::TypeScope;

/// The kinds whose documents the walk reads, and whose imports it follows.
const FOLLOWED: [ForgeKind; 8] = [
    ForgeKind::TestCase,
    ForgeKind::TestConfiguration,
    ForgeKind::TestComponent,
    ForgeKind::TestPortType,
    ForgeKind::TestTemplate,
    ForgeKind::EventSchema,
    ForgeKind::Enum,
    ForgeKind::Union,
];

/// A forge document the suite reaches.
pub(super) struct Reached {
    /// Its one spelling, whatever import reaches it ([`stdlib::identity`]).
    pub(super) identity: String,
    /// The name its definition takes in the export ([`unit_name`]).
    pub(super) name: String,
    /// The label its refusals carry: its path.
    pub(super) label: String,
    /// The directory its own imports resolve against.
    pub(super) dir: PathBuf,
    pub(super) parsed: ParsedForge,
    /// The scope its own imports name enums, records and unions in.
    pub(super) scope: TypeScope,
}

impl Reached {
    /// Where a name written in this document is refused: at its root.
    pub(super) fn origin(&self) -> Origin {
        Origin {
            kind: self.parsed.document.kind(),
            document: self.parsed.document.name().to_string(),
            label: self.label.clone(),
            line: root_line(&self.parsed.document),
        }
    }
}

/// The row of the root element of `document`, for the kinds the export
/// writes a definition of.
pub(super) fn root_line(document: &ForgeDocument) -> Option<u32> {
    let at = match document {
        ForgeDocument::Enum(m) => &m.source_location,
        ForgeDocument::EventSchema(m) => &m.source_location,
        ForgeDocument::Union(m) => &m.source_location,
        ForgeDocument::TestTemplate(m) => &m.source_location,
        ForgeDocument::TestPortType(m) => &m.source_location,
        ForgeDocument::TestSuite(m) => &m.source_location,
        _ => return None,
    };
    at.as_ref().and_then(|at| at.line)
}

/// Every forge document a suite reaches, by its one spelling.
pub(super) struct Reach {
    documents: BTreeMap<String, Reached>,
}

impl Reach {
    /// The documents in the order of their names, two of one name in the
    /// order of their spellings: the order every definition is written in.
    pub(super) fn sorted(&self) -> Vec<&Reached> {
        let mut documents: Vec<&Reached> = self.documents.values().collect();
        documents.sort_by(|a, b| (&a.name, &a.identity).cmp(&(&b.name, &b.identity)));
        documents
    }

    /// The document an import of the document read in `dir` names, when the
    /// suite reaches it.
    pub(super) fn imported(&self, dir: &Path, import: &ForgeImport) -> Option<&Reached> {
        self.documents
            .get(&stdlib::identity(&stdlib::resolve(dir, &import.src)))
    }
}

/// Hold `parsed`, the suite or a document it reaches, to the rules of its
/// kind that no backend decides: its names, its import graph, a schema's
/// revisions, a union's cases, a template's matchers and references, and how
/// a port type, a configuration, a test case and a suite fit the documents
/// they import. `dir` is the directory its imports resolve against.
pub(super) fn check_document(
    parsed: &ParsedForge,
    dir: &Path,
    label: &str,
) -> Result<(), Located<ForgeError>> {
    crate::forge::provenance::validate_forge_emission_provenance(&parsed.document, label)?;
    crate::forge::namespace::check(parsed, label)?;
    crate::forge::cross_kind_check::check(parsed, dir, label)?;
    crate::forge::schema_revision::check(parsed, dir, label)?;
    crate::forge::union_check::check(parsed, dir, label)?;
    crate::forge::test_template_check::check(parsed, dir, label)?;
    crate::forge::test_profile_assembly::check(parsed, dir, label)?;
    Ok(())
}

/// Every document `suite`, read in `dir` and labelled `label`, reaches, each
/// held to [`check_document`]. A document that cannot be read, or that is of
/// another kind than its import declares, is refused at the import.
pub(super) fn reach(
    suite: &ParsedForge,
    dir: &Path,
    label: &str,
) -> Result<Reach, Located<ForgeError>> {
    let mut documents: BTreeMap<String, Reached> = BTreeMap::new();
    let mut components: BTreeSet<String> = BTreeSet::new();
    let mut pending: Vec<(Vec<ForgeImport>, PathBuf, String)> =
        vec![(suite.imports.clone(), dir.to_path_buf(), label.to_string())];
    while let Some((imports, dir, label)) = pending.pop() {
        for import in imports.iter().filter(|i| FOLLOWED.contains(&i.kind)) {
            let identity = stdlib::identity(&stdlib::resolve(&dir, &import.src));
            if documents.contains_key(&identity) || components.contains(&identity) {
                continue;
            }
            let at_import =
                |error: ForgeError| Located::new(error, label.clone(), import.line, None);
            let source = ImportSource::read(&dir, import).map_err(|e| at_import(e.into()))?;
            let source_dir = source
                .path
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| dir.clone());
            let source_label = source.label().diagnostic_label.to_string();
            let stem = source.label().identifier.to_string();
            if import.kind == ForgeKind::TestComponent {
                let Some(model) = source.parse_test_component()? else {
                    return Err(at_import(other_kind(import, &source).into()));
                };
                components.insert(identity);
                pending.push((model.forge_imports, source_dir, source_label));
                continue;
            }
            let Some(parsed) = source.parse()? else {
                return Err(at_import(other_kind(import, &source).into()));
            };
            if parsed.document.kind() != import.kind {
                return Err(at_import(
                    ImportError::KindMismatch {
                        src: import.src.clone(),
                        declared: import.kind.to_string(),
                        actual: parsed.document.kind().to_string(),
                    }
                    .into(),
                ));
            }
            check_document(&parsed, &source_dir, &source_label)?;
            let scope = type_scope(&parsed.imports, &source_dir);
            pending.push((
                parsed.imports.clone(),
                source_dir.clone(),
                source_label.clone(),
            ));
            documents.insert(
                identity.clone(),
                Reached {
                    identity,
                    name: unit_name(&stem),
                    label: source_label,
                    dir: source_dir,
                    parsed,
                    scope,
                },
            );
        }
    }
    Ok(Reach { documents })
}

/// Why the document `source` is not the document `import` declares: it is of
/// another kind, or it is no forge document at all.
fn other_kind(import: &ForgeImport, source: &ImportSource) -> ImportError {
    let actual = match crate::forge::parser::detect_kind(&source.content) {
        Ok(Some(kind)) => Some(kind),
        // A document with no `sce:kind` is a statechart.
        Ok(None) => Some(ForgeKind::Statechart),
        Err(_) => None,
    };
    match actual {
        Some(actual) => ImportError::KindMismatch {
            src: import.src.clone(),
            declared: import.kind.to_string(),
            actual: actual.to_string(),
        },
        None => ImportError::NotForge {
            src: import.src.clone(),
        },
    }
}

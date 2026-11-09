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
//! one into the types module, a template or a component. The suite's feature
//! model is read apart, by rule E15 alone ([`feature_model`]).
//!
//! Each forge document reached is held to the rules of its kind that no
//! backend decides, the ones `check` holds it to before any language is
//! asked ([`check_document`]), so the export never translates a document
//! `check` refuses. A test component is a statechart, read by the statechart
//! parser, which holds it to the rules of the profile as it reads it.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use super::names::{unit_name, Origin};
use crate::forge::error::{ForgeError, ImportError, Located};
use crate::forge::import_source::{type_scope, ImportSource};
use crate::forge::model::{ForgeDocument, ForgeImport, ForgeKind, ParsedForge};
use crate::forge::stdlib;
use crate::forge::value_type::TypeScope;
use crate::model::SCXMLModel;

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
        ForgeDocument::TestCase(m) => &m.source_location,
        ForgeDocument::TestConfiguration(m) => &m.source_location,
        ForgeDocument::FeatureModel(m) => &m.source_location,
        _ => return None,
    };
    at.as_ref().and_then(|at| at.line)
}

/// A test component the suite reaches: a statechart, read by the statechart
/// parser, which holds it to the rules of the profile as it reads it
/// (docs/SCE_TEST_PROFILE.md §11.5).
pub(super) struct ReachedComponent {
    /// Its one spelling, whatever import reaches it ([`stdlib::identity`]).
    pub(super) identity: String,
    /// The name its component type takes in the export ([`unit_name`]).
    pub(super) name: String,
    /// The label its refusals carry: its path.
    pub(super) label: String,
    /// The directory its own imports resolve against.
    pub(super) dir: PathBuf,
    pub(super) model: SCXMLModel,
}

impl ReachedComponent {
    /// Where a name written in this component is refused, on `line`.
    pub(super) fn origin(&self, line: Option<u32>) -> Origin {
        Origin {
            kind: ForgeKind::TestComponent,
            document: self.model.name.clone(),
            label: self.label.clone(),
            line,
        }
    }
}

/// Every forge document a suite reaches, by its one spelling, and every test
/// component.
pub(super) struct Reach {
    documents: BTreeMap<String, Reached>,
    components: BTreeMap<String, ReachedComponent>,
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

    /// The test components in the order of their names, two of one name in
    /// the order of their spellings.
    pub(super) fn components(&self) -> Vec<&ReachedComponent> {
        let mut components: Vec<&ReachedComponent> = self.components.values().collect();
        components.sort_by(|a, b| (&a.name, &a.identity).cmp(&(&b.name, &b.identity)));
        components
    }

    /// The test component an import of the document read in `dir` names,
    /// when the suite reaches it.
    pub(super) fn component(&self, dir: &Path, import: &ForgeImport) -> Option<&ReachedComponent> {
        self.components
            .get(&stdlib::identity(&stdlib::resolve(dir, &import.src)))
    }
}

/// Hold `parsed`, the suite or a document it reaches, to the rules of its
/// kind that no backend decides, in the order `check` asks them: its names,
/// its import graph, the units of its quantities, its cycles, a schema's
/// revisions, a union's cases, a template's matchers and references, and how
/// a port type, a configuration, a test case and a suite fit the documents
/// they import. The rules of the kinds the export reads no document of (a
/// transform's, a codec's) are not asked. `dir` is the directory its imports
/// resolve against.
pub(super) fn check_document(
    parsed: &ParsedForge,
    dir: &Path,
    label: &str,
) -> Result<(), Located<ForgeError>> {
    crate::forge::provenance::validate_forge_emission_provenance(&parsed.document, label)?;
    crate::forge::namespace::check(parsed, label)?;
    crate::forge::cross_kind_check::check(parsed, dir, label)?;
    crate::forge::quantity_check::check(parsed, label)?;
    crate::forge::cycle_check::check(parsed, dir, label)?;
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
    let mut components: BTreeMap<String, ReachedComponent> = BTreeMap::new();
    let mut pending: Vec<(Vec<ForgeImport>, PathBuf, String)> =
        vec![(suite.imports.clone(), dir.to_path_buf(), label.to_string())];
    while let Some((imports, dir, label)) = pending.pop() {
        for import in imports.iter().filter(|i| FOLLOWED.contains(&i.kind)) {
            let identity = stdlib::identity(&stdlib::resolve(&dir, &import.src));
            if documents.contains_key(&identity) || components.contains_key(&identity) {
                continue;
            }
            let source = ImportSource::read(&dir, import)
                .map_err(|e| Located::new(ForgeError::from(e), label.clone(), import.line, None))?;
            if import.kind == ForgeKind::TestComponent {
                let Some(model) = source.parse_test_component()? else {
                    return Err(Located::new(
                        other_kind(import, &source).into(),
                        label.clone(),
                        import.line,
                        None,
                    ));
                };
                let source_dir = directory_of(&source, &dir);
                let source_label = source.label().diagnostic_label.to_string();
                pending.push((
                    model.forge_imports.clone(),
                    source_dir.clone(),
                    source_label.clone(),
                ));
                components.insert(
                    identity.clone(),
                    ReachedComponent {
                        identity,
                        name: unit_name(source.label().identifier),
                        label: source_label,
                        dir: source_dir,
                        model,
                    },
                );
                continue;
            }
            let reached = forge_document(import, &source, identity.clone(), &dir, &label)?;
            pending.push((
                reached.parsed.imports.clone(),
                reached.dir.clone(),
                reached.label.clone(),
            ));
            documents.insert(identity, reached);
        }
    }
    Ok(Reach {
        documents,
        components,
    })
}

/// The feature model a suite imports by `import` (docs/SCE_TEST_PROFILE.md
/// §11.8), the suite read in `dir` and labelled `label`: the one document the
/// walk does not follow and the export reads, for rule E15 alone, which
/// declares a module parameter of each of its features. It is held to
/// [`check_document`] as every document the export reads is.
pub(super) fn feature_model(
    import: &ForgeImport,
    dir: &Path,
    label: &str,
) -> Result<Reached, Located<ForgeError>> {
    let source = ImportSource::read(dir, import)
        .map_err(|e| Located::new(ForgeError::from(e), label.to_string(), import.line, None))?;
    let identity = stdlib::identity(&stdlib::resolve(dir, &import.src));
    forge_document(import, &source, identity, dir, label)
}

/// The directory the imports of `source`, read from the document in `dir`,
/// resolve against.
fn directory_of(source: &ImportSource, dir: &Path) -> PathBuf {
    source
        .path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| dir.to_path_buf())
}

/// The forge document `source`, which `import` of the document read in `dir`
/// and labelled `label` names, known by `identity`: parsed, of the kind the
/// import declares, and held to [`check_document`].
fn forge_document(
    import: &ForgeImport,
    source: &ImportSource,
    identity: String,
    dir: &Path,
    label: &str,
) -> Result<Reached, Located<ForgeError>> {
    let at_import = |error: ForgeError| Located::new(error, label.to_string(), import.line, None);
    let Some(parsed) = source.parse()? else {
        return Err(at_import(other_kind(import, source).into()));
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
    let source_dir = directory_of(source, dir);
    let source_label = source.label().diagnostic_label.to_string();
    check_document(&parsed, &source_dir, &source_label)?;
    let scope = type_scope(&parsed.imports, &source_dir);
    Ok(Reached {
        identity,
        name: unit_name(source.label().identifier),
        label: source_label,
        dir: source_dir,
        parsed,
        scope,
    })
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

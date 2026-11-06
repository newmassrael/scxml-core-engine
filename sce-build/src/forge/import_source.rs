// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The document an `<sce:import>` names, read the one way every reader
//! reads it.
//!
//! ⚠ WHY THIS IS ONE PLACE. Six readers resolved, read, labelled and
//! parsed an imported document each on its own: the import enrichment
//! that reports what is wrong with an import, the member-surface table of
//! [`crate::forge::cross_kind_check`], the statechart parser's sibling
//! schemas and enums, and one `variants_of` apiece in
//! [`crate::forge::coverage`], [`crate::forge::retention`] and
//! [`crate::forge::cycle_check`] — three bodies identical but for their
//! comments. Two of the six labelled the document by its path and four by
//! its basename, so what an imported document was called depended on who
//! asked (measured 2026-09-21).
//!
//! The label is the path, resolved against the importing document's
//! directory, because the one reader that reports an imported document's
//! failures — the enrichment — always used it: an imported document is
//! refused under that name, and every other reader stays silent.

use std::path::{Path, PathBuf};

use crate::forge::error::{ForgeError, ImportError, Located};
use crate::forge::model::{ForgeDocument, ForgeImport, ForgeKind, ParsedForge};
use crate::forge::value_type::{
    ScopedCase, ScopedEnum, ScopedRecord, ScopedUnion, ScopedVariant, TypeScope,
};
use crate::DocumentLabel;

/// An imported document's text, and the path it was read from.
pub struct ImportSource {
    pub path: PathBuf,
    pub content: String,
}

impl ImportSource {
    /// Read the document `import` names, resolved against `base_dir` —
    /// the importing document's directory.
    ///
    /// An `sce:std/...` document — or a relative import made by one — is
    /// read from the standard library this generator embeds, never from
    /// disk ([`crate::forge::stdlib`]).
    pub fn read(base_dir: &Path, import: &ForgeImport) -> Result<Self, ImportError> {
        Self::read_src(base_dir, &import.src)
    }

    /// [`Self::read`] for an import known by its `src` alone — what a reader
    /// that walks a document's references without parsing it as a forge
    /// document holds (`crate::import_closure`).
    pub fn read_src(base_dir: &Path, src: &str) -> Result<Self, ImportError> {
        let path = crate::forge::stdlib::resolve(base_dir, src);
        if crate::forge::stdlib::names_standard(&path) {
            return match crate::forge::stdlib::lookup(&path) {
                Some(content) => Ok(Self {
                    path,
                    content: content.to_string(),
                }),
                None => Err(ImportError::FileNotFound {
                    src: src.to_string(),
                    searched: format!(
                        "{} (the SCE standard library built into this sce-codegen)",
                        path.display()
                    ),
                }),
            };
        }
        // A document outside the folder a caller confined the generator to is answered as a file
        // that is not there (`crate::confine`), whichever of the two reads it.
        if !crate::confine::exists(&path) {
            return Err(ImportError::FileNotFound {
                src: src.to_string(),
                searched: path.display().to_string(),
            });
        }
        let content =
            crate::confine::read_to_string(&path).map_err(|source| ImportError::ReadError {
                src: src.to_string(),
                source,
            })?;
        Ok(Self { path, content })
    }

    /// The label the document parses under: its file stem identifies it,
    /// and its path labels its diagnostics.
    pub fn label(&self) -> DocumentLabel<'_> {
        let identifier = self
            .path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");
        DocumentLabel {
            identifier,
            diagnostic_label: self.path.to_str().unwrap_or(identifier),
        }
    }

    /// The document parsed, or `None` when it is a plain statechart —
    /// one this pipeline does not own.
    pub fn parse(&self) -> Result<Option<ParsedForge>, Located<ForgeError>> {
        crate::forge::parser::parse_forge_with_imports(&self.content, self.label())
    }

    /// Whether the document is a `test-component` (docs/SCE_TEST_PROFILE.md
    /// §11.5): a statechart, which [`Self::parse`] leaves to the statechart
    /// parser, and which a test configuration names by an `<sce:import>`
    /// (§11.6).
    pub fn is_test_component(&self) -> bool {
        matches!(
            crate::forge::parser::detect_kind(&self.content),
            Ok(Some(ForgeKind::TestComponent))
        )
    }

    /// The document read by the statechart parser when it is a
    /// `test-component`, or `None` when it is of another kind: the record a
    /// test configuration reads of a component it names, its ports, the PTCs
    /// it invokes and the module parameters it reads
    /// ([`crate::test_component`]). Read from its path, as every statechart
    /// is, so its fragments are expanded and a standard document is read from
    /// the library.
    pub fn parse_test_component(
        &self,
    ) -> Result<Option<crate::model::SCXMLModel>, Located<ForgeError>> {
        if !self.is_test_component() {
            return Ok(None);
        }
        crate::parser::SCXMLParser::new()
            .parse_file(&self.path.to_string_lossy())
            .map(Some)
    }
}

/// The document `import` names, parsed — or `None` when it cannot be
/// read, is refused, or is not a forge document.
///
/// ⚠ SILENT BY DESIGN, and only for readers that run where the import
/// enrichment has already had its say: that pass reports a missing,
/// unreadable or refused import, and a second reader saying it again would
/// double-emit. A reader that is the ONLY voice for what it resolves — a
/// cycle's `of=` is one, since the enrichment never looks at it — must
/// turn `None` into its own refusal rather than skip.
pub fn parse_quietly(base_dir: &Path, import: &ForgeImport) -> Option<ParsedForge> {
    ImportSource::read(base_dir, import).ok()?.parse().ok()?
}

/// The variant names the enum imported as `alias` declares, in document
/// order — or `None` when `alias` names no readable enum. Silent as
/// [`parse_quietly`] is, for the same reason.
pub fn enum_variants(parsed: &ParsedForge, base_dir: &Path, alias: &str) -> Option<Vec<String>> {
    enum_model(parsed, base_dir, alias).map(|e| e.variants.into_iter().map(|v| v.name).collect())
}

/// The enum imported as `alias`, whole — or `None` when `alias` names no
/// readable enum. Silent as [`parse_quietly`] is, for the same reason.
pub fn enum_model(
    parsed: &ParsedForge,
    base_dir: &Path,
    alias: &str,
) -> Option<crate::forge::model::EnumModel> {
    enum_model_in(&parsed.imports, base_dir, alias)
}

/// The enum `imports` (written relative to `base_dir`) name as `alias`, as
/// [`enum_model`] reads it from a document's imports.
pub fn enum_model_in(
    imports: &[ForgeImport],
    base_dir: &Path,
    alias: &str,
) -> Option<crate::forge::model::EnumModel> {
    let import = imports.iter().find(|i| i.alias == alias)?;
    match parse_quietly(base_dir, import)?.document {
        ForgeDocument::Enum(e) => Some(e),
        _ => None,
    }
}

/// The enum an enum document is, as a type that holds it is sized
/// (docs/adr/0086, sections 2 and 9.2): each variant with its declared name and
/// number (`value=`), its underlying type, and whether it is closed.
pub fn scoped_enum(model: &crate::forge::model::EnumModel) -> ScopedEnum {
    ScopedEnum {
        variants: model
            .variants
            .iter()
            .map(|v| ScopedVariant {
                name: v.name.clone(),
                value: v.value,
            })
            .collect(),
        underlying: model.underlying_type.clone(),
        strict: model.strict_variants,
    }
}

/// The record an event-schema document is, as a type that holds it is sized
/// (docs/adr/0086, section 2): its identity, its fields of the one value type,
/// and the scope its own imports (`imports`, written relative to `base_dir`,
/// the schema's directory) name enums and records in, at every depth.
pub fn scoped_record(
    path: &Path,
    schema: &crate::forge::model::EventSchemaModel,
    imports: &[ForgeImport],
    base_dir: &Path,
) -> ScopedRecord {
    let mut on_path = vec![path.to_path_buf()];
    scoped_record_at(path, schema, imports, base_dir, &mut on_path)
}

fn scoped_record_at(
    path: &Path,
    schema: &crate::forge::model::EventSchemaModel,
    imports: &[ForgeImport],
    base_dir: &Path,
    on_path: &mut Vec<PathBuf>,
) -> ScopedRecord {
    ScopedRecord {
        identity: crate::forge::stdlib::identity(path),
        name: schema.name.clone(),
        fields: schema
            .fields
            .iter()
            .map(|f| (f.id.clone(), f.sce_type.clone()))
            .collect(),
        scope: type_scope_at(imports, base_dir, on_path),
    }
}

/// Each enum, event schema and union `imports` (written relative to
/// `base_dir`) names, by alias, each record and union with the scope of its own
/// document at every depth: the scope a document's types are read in
/// (docs/adr/0086, sections 2 and 6.2; docs/adr/0092, section 1.1). Silent as
/// [`parse_quietly`] is, for the same reason.
pub fn type_scope(imports: &[ForgeImport], base_dir: &Path) -> TypeScope {
    type_scope_at(imports, base_dir, &mut Vec::new())
}

/// Each enum and event schema `imports` names (written relative to
/// `base_dir`), read as a size reads it. Silent as [`parse_quietly`] is: an
/// import that cannot be read is left out, and a type that names it has no
/// size, refused where the size is asked. A schema already on the way down is
/// not read again: a record that holds itself through its documents is an
/// import cycle, refused where the imports are walked
/// (`crate::forge::cross_kind_check`).
fn type_scope_at(
    imports: &[ForgeImport],
    base_dir: &Path,
    on_path: &mut Vec<PathBuf>,
) -> TypeScope {
    let mut scope = TypeScope::default();
    for import in imports {
        match import.kind {
            ForgeKind::Enum => {
                if let Some(ForgeDocument::Enum(model)) =
                    parse_quietly(base_dir, import).map(|p| p.document)
                {
                    scope
                        .enums
                        .insert(import.alias.clone(), scoped_enum(&model));
                }
            }
            ForgeKind::EventSchema => {
                let Ok(source) = ImportSource::read(base_dir, import) else {
                    continue;
                };
                if on_path.contains(&source.path) {
                    continue;
                }
                let Ok(Some(parsed)) = source.parse() else {
                    continue;
                };
                let ForgeDocument::EventSchema(schema) = &parsed.document else {
                    continue;
                };
                let dir = source.path.parent().unwrap_or(base_dir);
                on_path.push(source.path.clone());
                let record = scoped_record_at(&source.path, schema, &parsed.imports, dir, on_path);
                on_path.pop();
                scope.records.insert(import.alias.clone(), record);
            }
            // docs/adr/0092, section 1.1: a union, with its own document's
            // scope, read as a record is.
            ForgeKind::Union => {
                let Ok(source) = ImportSource::read(base_dir, import) else {
                    continue;
                };
                if on_path.contains(&source.path) {
                    continue;
                }
                let Ok(Some(parsed)) = source.parse() else {
                    continue;
                };
                let ForgeDocument::Union(model) = &parsed.document else {
                    continue;
                };
                let dir = source.path.parent().unwrap_or(base_dir);
                on_path.push(source.path.clone());
                let union = scoped_union_at(&source.path, model, &parsed.imports, dir, on_path);
                on_path.pop();
                if let Some(union) = union {
                    scope.unions.insert(import.alias.clone(), union);
                }
            }
            _ => {}
        }
    }
    scope
}

/// The union a union document is, as a type that holds it is sized
/// (docs/adr/0092, sections 1.1 and 5.1): its identity, its tag's key, each
/// case in its tag enum's declaration order with the variant's declared name
/// and number, and the scope its own imports name the enum and the cases'
/// records in. `None` when its tag's enum cannot be read, which the import
/// pass reports: a type that names the union then has no size.
pub fn scoped_union(
    path: &Path,
    model: &crate::forge::model::UnionModel,
    imports: &[ForgeImport],
    base_dir: &Path,
) -> Option<ScopedUnion> {
    let mut on_path = vec![path.to_path_buf()];
    scoped_union_at(path, model, imports, base_dir, &mut on_path)
}

fn scoped_union_at(
    path: &Path,
    model: &crate::forge::model::UnionModel,
    imports: &[ForgeImport],
    base_dir: &Path,
    on_path: &mut Vec<PathBuf>,
) -> Option<ScopedUnion> {
    let scope = type_scope_at(imports, base_dir, on_path);
    union_in_scope(crate::forge::stdlib::identity(path), model, scope)
}

/// The union `model` is, read in `scope`, the scope its own imports name its
/// tag's enum and its cases' records in, under `identity`: each case in its tag
/// enum's declaration order, with the variant's declared name and number.
/// `None` when `scope` holds no tag enum by the alias `model` names. What
/// [`scoped_union`] makes of a document it reads, and what a backend makes of
/// a union document whose imports it already holds as a scope.
pub fn union_in_scope(
    identity: String,
    model: &crate::forge::model::UnionModel,
    scope: TypeScope,
) -> Option<ScopedUnion> {
    let tag_enum = scope.enums.get(&model.tag_enum)?;
    // The enum's order, each variant's case as the document gives it; a
    // variant with no case is refused where the union is checked
    // (`crate::forge::union_check`), so it holds no case here.
    let cases = tag_enum
        .variants
        .iter()
        .filter_map(|variant| {
            model.case(&variant.name).map(|case| ScopedCase {
                variant: variant.clone(),
                record: case.record.clone(),
            })
        })
        .collect();
    Some(ScopedUnion {
        identity,
        name: model.name.clone(),
        tag: model.tag.clone(),
        tag_enum: model.tag_enum.clone(),
        cases,
        scope,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCXML_HEAD: &str =
        r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext""#;

    /// A test component is a statechart (docs/SCE_TEST_PROFILE.md §11.5): an
    /// import of one reads it with the statechart parser, with the profile's
    /// record of what it declares; a forge document is no test component.
    #[test]
    fn a_test_component_is_read_by_the_statechart_parser() {
        let dir = tempfile::tempdir().expect("temp dir");
        std::fs::write(
            dir.path().join("probe_component.scxml"),
            format!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n{SCXML_HEAD} version=\"1.0\" \
                 sce:kind=\"test-component\" name=\"probe_component\" \
                 datamodel=\"sce-static\" initial=\"waiting\">\n  <sce:test-timer id=\"wait\" \
                 duration=\"1s\"/>\n  <state id=\"waiting\">\n    <invoke \
                 type=\"http://sce.dev/ext/test-component\" src=\"probe_component.scxml\" \
                 id=\"again\"/>\n    <onentry>\n      <sce:start-timer timer=\"wait\"/>\n    \
                 </onentry>\n    <transition sce:timeout=\"wait\" target=\"done\"/>\n  \
                 </state>\n  <final id=\"done\"/>\n</scxml>\n"
            ),
        )
        .expect("write the component");
        std::fs::write(
            dir.path().join("probe_features.scxml"),
            format!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n{SCXML_HEAD} version=\"1.0\" \
                 sce:kind=\"feature-model\" name=\"probe_features\">\n  <sce:feature \
                 name=\"probe\"/>\n</scxml>\n"
            ),
        )
        .expect("write the feature model");

        let component =
            ImportSource::read_src(dir.path(), "probe_component.scxml").expect("it reads");
        assert!(component.is_test_component());
        assert!(
            component
                .parse()
                .unwrap_or_else(|e| panic!("{e:?}"))
                .is_none(),
            "the forge parser leaves a statechart to the statechart parser"
        );
        let model = component
            .parse_test_component()
            .unwrap_or_else(|e| panic!("the component parses: {e:?}"))
            .expect("a test component");
        let profile = model.test_component.as_ref().expect("its profile");
        assert_eq!(profile.timers.len(), 1);
        assert_eq!(profile.invocations.len(), 1);
        assert_eq!(profile.invocations[0].id.as_deref(), Some("again"));

        let features =
            ImportSource::read_src(dir.path(), "probe_features.scxml").expect("it reads");
        assert!(!features.is_test_component());
        assert!(features
            .parse_test_component()
            .unwrap_or_else(|e| panic!("{e:?}"))
            .is_none());
    }
}

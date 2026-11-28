// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A view (docs/adr/0151): a read model, a relational query over a machine's
//! bounded keyed tables, read where its document is read into one typed plan,
//! or a document of relations, the rewrites a viewer's relations are derived by.
//!
//! [`plan`] reads a view document, and every view it imports, into a
//! [`ViewPlan`]: plain data a reference evaluator, the translator of a model and the
//! lane that prints the maintenance circuit each take whole, none reading the
//! document again, so none of them depends on code generation. Every refusal of
//! the kind that reads more than the document alone is made here, each by its
//! code (D11): a result key the build does not infer, a cut with no order, a
//! cycle of views, a relation that depends on its own exclusion, a table that
//! says nobody may read it, a view that reads rows its viewer may not, an
//! expression the build does not prove total, an aggregate its type may not
//! hold, and an aggregate or an order over floating numbers. The parser holds the
//! document to its shape first ([`crate::forge::parser`]).
//!
//! No backend lowers a view yet: Rust is its lane (D10), from this plan, once
//! the operators of its forge runtime land, and every other backend and the
//! Interpreter refuse it by name ([`backend_unsupported`]).

pub mod plan;

mod build;
mod relations;
mod scope;

use std::path::{Path, PathBuf};

use crate::forge::error::{ForgeError, GenerateError, ImportError, Located, ValidationError};
use crate::forge::import_source::{self, ImportSource};
use crate::forge::model::{
    ForgeDocument, ForgeImport, ForgeKind, ParsedForge, ViewBody, ViewModel, ViewTableSource,
};
use crate::forge::value_type::{ScopedRecord, TypeScope, ValueType};
use crate::generator::Language;
use plan::{PlanBody, PlanColumn, ReadView, ViewPlan};

/// The plan of `parsed`, read from `base_dir` under the label `document`, and of
/// every view it imports; `None` for a document of another kind.
///
/// An imported view is planned first, and a refusal of it is placed in its own
/// document. A view that reads itself, through any number of views, is refused
/// (`view/cycle`) where the cycle closes.
pub fn plan(
    parsed: &ParsedForge,
    base_dir: &Path,
    document: &str,
) -> Result<Option<ViewPlan>, Located<ForgeError>> {
    let ForgeDocument::View(view) = &parsed.document else {
        return Ok(None);
    };
    let mut planner = Planner {
        stack: vec![(entry_identity(document, base_dir), view.name.clone())],
    };
    planner
        .plan(view, &parsed.imports, base_dir, document)
        .map(|planned| Some(planned.plan))
}

/// Refuse a view document whose plan the build cannot make; a document of
/// any other kind passes. Judged once, before any backend is asked, so the
/// verdict does not depend on the language.
pub fn check(
    parsed: &ParsedForge,
    base_dir: &Path,
    document: &str,
) -> Result<(), Located<ForgeError>> {
    plan(parsed, base_dir, document).map(|_| ())
}

/// The refusal of a view document on `language` (docs/adr/0151, D10):
/// `view/backend-unsupported`, by name, after the document is read and its plan
/// made.
pub fn backend_unsupported(view: &ViewModel, language: Language) -> GenerateError {
    GenerateError::ViewBackendUnsupported {
        view: view.name.clone(),
        language: crate::forge::codegen_matrix::language_wire_name(language).to_string(),
    }
}

/// A view a view reads, planned, with what the view that reads it types its
/// rows and its relations' objects by.
struct Read {
    alias: String,
    plan: ViewPlan,
    /// The record a row of its result is, in its own document's scope.
    result: Option<ScopedRecord>,
    /// The type of a tuple's object, where it is a document of relations.
    tuple_object: Option<ValueType>,
}

/// One view planned, and what a view that reads it needs of it.
struct Planned {
    plan: ViewPlan,
    result: Option<ScopedRecord>,
    tuple_object: Option<ValueType>,
}

/// The views being planned, each by the path it was read from, from the one
/// asked for to the one being read: a view met again on the way is a cycle.
struct Planner {
    stack: Vec<(PathBuf, String)>,
}

impl Planner {
    fn plan(
        &mut self,
        view: &ViewModel,
        imports: &[ForgeImport],
        base_dir: &Path,
        document: &str,
    ) -> Result<Planned, Located<ForgeError>> {
        let mut reads: Vec<Read> = Vec::new();
        for import in imports.iter().filter(|i| i.kind == ForgeKind::View) {
            let at = |error: ForgeError| Located::new(error, document, import.line, None);
            let source = ImportSource::read(base_dir, import).map_err(|e| at(e.into()))?;
            let identity =
                crate::confine::canonicalize(&source.path).unwrap_or_else(|_| source.path.clone());
            if let Some(start) = self.stack.iter().position(|(seen, _)| *seen == identity) {
                let mut cycle: Vec<String> = self.stack[start..]
                    .iter()
                    .map(|(_, name)| name.clone())
                    .collect();
                cycle.push(self.stack[start].1.clone());
                return Err(at(ValidationError::ViewCycle {
                    view: self.stack[start].1.clone(),
                    cycle,
                }
                .into()));
            }
            let parsed = source.parse()?.ok_or_else(|| {
                at(ImportError::NotForge {
                    src: import.src.clone(),
                }
                .into())
            })?;
            let ForgeDocument::View(read) = &parsed.document else {
                return Err(at(ImportError::KindMismatch {
                    src: import.src.clone(),
                    declared: ForgeKind::View.to_string(),
                    actual: parsed.document.kind().to_string(),
                }
                .into()));
            };
            let read_dir = source.path.parent().unwrap_or(base_dir).to_path_buf();
            let label = source.label();
            self.stack.push((identity, read.name.clone()));
            let planned = self.plan(read, &parsed.imports, &read_dir, label.diagnostic_label);
            self.stack.pop();
            let planned = planned?;
            reads.push(Read {
                alias: import.alias.clone(),
                plan: planned.plan,
                result: planned.result,
                tuple_object: planned.tuple_object,
            });
        }

        let types = import_source::type_scope(imports, base_dir);
        let (body, result, tuple_object) = match &view.body {
            ViewBody::Query(query) => {
                let built = build::query(view, query, &reads, &types, document)?;
                (
                    PlanBody::Query(Box::new(built)),
                    types.records.get(&query.result.record).cloned(),
                    None,
                )
            }
            ViewBody::Relations { relations } => {
                let planned = relations::plan(view, relations, &types, document)?;
                (
                    PlanBody::Relations(planned),
                    None,
                    tuple_object_type(view, &types),
                )
            }
        };
        let plan = ViewPlan {
            view: view.name.clone(),
            viewer: view.viewer.as_ref().map(|viewer| viewer.of),
            records: types
                .records
                .iter()
                .map(|(alias, record)| {
                    let fields = record
                        .fields
                        .iter()
                        .map(|(name, ty)| PlanColumn {
                            name: name.clone(),
                            ty: ty.to_string(),
                        })
                        .collect();
                    (alias.clone(), fields)
                })
                .collect(),
            reads: reads
                .into_iter()
                .map(|read| ReadView {
                    alias: read.alias,
                    plan: read.plan,
                })
                .collect(),
            body,
        };
        if let Err(defect) = plan.verify() {
            unreachable!("the build made a plan it refuses: {defect}");
        }
        Ok(Planned {
            plan,
            result,
            tuple_object,
        })
    }
}

/// The type of a tuple's object in the document of relations `view`.
fn tuple_object_type(view: &ViewModel, types: &TypeScope) -> Option<ValueType> {
    let table = view.tables.first()?;
    let ViewTableSource::Type(ValueType::Set { key, .. }) = &table.source else {
        return None;
    };
    let ValueType::Record { alias } = key.as_ref() else {
        return None;
    };
    types
        .records
        .get(alias)?
        .fields
        .iter()
        .find(|(name, _)| name == "object_id")
        .map(|(_, ty)| ty.clone())
}

/// The path the document asked for is read from: the file its label names,
/// beside `base_dir`, as the import graph's walk names it.
fn entry_identity(document: &str, base_dir: &Path) -> PathBuf {
    let file = Path::new(document)
        .file_name()
        .map_or_else(|| PathBuf::from(document), |name| base_dir.join(name));
    crate::confine::canonicalize(&file).unwrap_or(file)
}

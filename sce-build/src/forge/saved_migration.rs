// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A saved state carried into a new version of its document by a migration the
//! document declares (docs/adr/0109, D11; SCE Accepted Subset §2.15, "Saving and
//! restoring").
//!
//! A saved state is bound to its document's shape
//! ([`crate::forge::static_lowering::SavedShape`]): a state, history, invocation or
//! variable renamed, re-typed, re-parented or re-bounded moves the shape, and a
//! restore refuses a state of another shape. Schema evolution is a reader's schema
//! meeting a writer's: the writer's schema of every state one released version
//! saved is that version's SHAPE FILE, the text the shape is the digest of, which
//! the author commits (`shapes/<version>.shape`, written by `sce-codegen shape`).
//! A document then declares each step from one released shape to the next:
//!
//! ```xml
//! <sce:migration from="shapes/2.shape" to="shapes/3.shape">
//!   <sce:rename kind="state" from="waiting" to="pending"/>
//!   <sce:map-state from="legacy" to="idle"/>
//!   <sce:drop-data id="scratch"/>
//!   <sce:add-data id="limit" expr="16"/>
//!   <sce:convert data="due" algorithm="DueFromSeconds"/>
//!   <sce:drop-invoke id="poller"/>
//! </sce:migration>
//! ```
//!
//! The build holds each step to its two shape files ([`plan`]): every old state,
//! history, invocation and variable is kept, renamed, mapped or dropped, and every
//! new variable is carried or added, each carried variable of a type that is its
//! old one or widens it; the steps form one chain whose last `to` is the shape of
//! the document being built. What it accepts it writes into the machine as a plan
//! ([`SavedMigrationStep`]), which the runtime's restore applies to a state saved
//! under a step's `from` before it judges the shape (Rust
//! `saved_state::upgrade`, C++ `upgradeSavedState`). Rust and C++ apply it; every
//! other backend refuses a document that declares a migration, by name (D0).
//!
//! A shape file holds the text a state of format 2 or later names its shape by
//! (`SavedShape::format2_first` is its digest): every state a runtime writes today
//! is of format 4. A state of format 1, which spells an enum by the alias it was
//! imported under, is not migrated, and is judged as it always was.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::forge::error::{ForgeError, GenerateError, Located, SourceLocation, ValidationError};
use crate::forge::static_lowering::{CalleeImport, SavedShape, StaticTarget};
use crate::forge::type_ctx::StaticScope;
use crate::model::SCXMLModel;

/// The first line of every shape text and every shape file.
pub const SHAPE_HEADER: &str = "sce-saved-state-shape 1";

/// The line a shape text gains when a state names a revision of a record schema
/// or an enum past the first (docs/adr/0044): the revised shape is the digest of
/// the shape's text and this line.
const REVISED_LINE: &str = "revisions\n";

// ── The grammar ─────────────────────────────────────────────────────────

/// A `<sce:migration from to>`: one step from the shape of one released version of
/// the document to the next.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct Migration {
    /// The shape a state the step starts from was saved under.
    pub from: ShapeFile,
    /// The shape the step carries such a state to.
    pub to: ShapeFile,
    /// What the step does, in document order.
    pub changes: Vec<MigrationChange>,
    /// Where the `<sce:migration>` element is written.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_location: Option<SourceLocation>,
}

/// A shape file a `<sce:migration>` names, as the document writes its path, and its
/// text as the parse read it beside the document.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct ShapeFile {
    /// The path as the document writes it, relative to the document's directory.
    pub path: String,
    /// The file's text, or why it could not be read. Read when the document is
    /// parsed and judged when the machine is built, where the document's own
    /// shape is known ([`plan`]): a parse with no directory reads none.
    #[serde(skip)]
    pub text: Result<String, String>,
}

/// One child of a `<sce:migration>`.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct MigrationChange {
    /// What it does.
    pub op: MigrationOp,
    /// Where it is written.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_location: Option<SourceLocation>,
}

/// What a child of a `<sce:migration>` does to a state saved under the step's `from`.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[serde(tag = "element", rename_all = "kebab-case")]
pub enum MigrationOp {
    /// `<sce:rename kind from to>`: a state, history, variable or invocation of the
    /// old shape is the new shape's `to`, or a waiting send's event is now `to`.
    Rename {
        kind: RenameKind,
        from: String,
        to: String,
    },
    /// `<sce:map-state from to>`: a configuration that holds `from`, a state the new
    /// shape no longer holds, holds `to` in its place.
    MapState { from: String, to: String },
    /// `<sce:drop-data id>`: a variable of the old shape the new one does not carry.
    DropData { id: String },
    /// `<sce:add-data id expr>`: a variable of the new shape, given the constant
    /// `expr`; in the last step a variable whose `<data>` initialiser is a constant
    /// may leave `expr` out.
    AddData { id: String, expr: Option<String> },
    /// `<sce:convert data algorithm>`: the variable `data` of the new shape is the
    /// pure algorithm `algorithm` (an import's alias) of the old value.
    Convert { data: String, algorithm: String },
    /// `<sce:drop-invoke id>`: an invocation the new document no longer runs ends
    /// without a start.
    DropInvoke { id: String },
}

/// What a `<sce:rename>` renames.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum RenameKind {
    State,
    History,
    Data,
    Invoke,
    Event,
}

impl RenameKind {
    /// The values `kind` takes, as a document writes them.
    pub const WRITTEN: [&'static str; 5] = ["state", "history", "data", "invoke", "event"];

    fn read(written: &str) -> Option<Self> {
        match written {
            "state" => Some(Self::State),
            "history" => Some(Self::History),
            "data" => Some(Self::Data),
            "invoke" => Some(Self::Invoke),
            "event" => Some(Self::Event),
            _ => None,
        }
    }
}

/// The children a `<sce:migration>` takes.
const CHANGES: &[&str] = &[
    "rename",
    "map-state",
    "drop-data",
    "add-data",
    "convert",
    "drop-invoke",
];

/// Every `<sce:migration>` of the document `root`, each shape file it names read
/// from `dir` (the directory the document's imports are read from), and the paths
/// of the files read, which a build that reruns on a changed input depends on.
///
/// A migration carries a saved state, which only a machine of the static data
/// model has (§2.15), so a document of another data model that declares one is
/// refused. A shape file that cannot be read is not refused here: the step that
/// names it is, where the machine is built ([`plan`]), so a document whose shape
/// files are not committed yet still parses, and its own shape can be written.
pub(crate) fn read(
    root: &roxmltree::Node,
    datamodel: crate::model::Datamodel,
    dir: Option<&Path>,
    diag_label: &str,
) -> Result<(Vec<Migration>, Vec<PathBuf>), Located<ForgeError>> {
    let mut migrations = Vec::new();
    let mut files = Vec::new();
    for element in crate::forge::parser::sce_children(root, "migration") {
        if datamodel != crate::model::Datamodel::SceStatic {
            return Err(crate::forge::parser::located(
                root,
                diag_label,
                ValidationError::AttributeRuleViolated {
                    element: "<scxml>".to_string(),
                    attr: "datamodel".to_string(),
                    value: datamodel.as_str().to_string(),
                    rule: "a document that declares <sce:migration> is of the static data model \
                           (`sce-static`): a migration carries a saved state, which only a machine \
                           of that data model has (docs/adr/0109, D11)"
                        .to_string(),
                },
            ));
        }
        let from = required(&element, "<sce:migration>", "from", diag_label)?;
        let to = required(&element, "<sce:migration>", "to", diag_label)?;
        let mut changes = Vec::new();
        for child in crate::forge::parser::sce_children_among(&element, CHANGES) {
            let tag = format!("<sce:{}>", child.tag_name().name());
            let attr = |name: &str| required(&child, &tag, name, diag_label);
            let op = match child.tag_name().name() {
                "rename" => {
                    let written = attr("kind")?;
                    let kind = RenameKind::read(&written).ok_or_else(|| {
                        crate::forge::parser::located(
                            &child,
                            diag_label,
                            ValidationError::InvalidAttribute {
                                element: tag.clone(),
                                attr: "kind".to_string(),
                                value: written.clone(),
                                allowed: RenameKind::WRITTEN
                                    .iter()
                                    .map(|k| k.to_string())
                                    .collect(),
                            },
                        )
                    })?;
                    MigrationOp::Rename {
                        kind,
                        from: attr("from")?,
                        to: attr("to")?,
                    }
                }
                "map-state" => MigrationOp::MapState {
                    from: attr("from")?,
                    to: attr("to")?,
                },
                "drop-data" => MigrationOp::DropData { id: attr("id")? },
                "add-data" => MigrationOp::AddData {
                    id: attr("id")?,
                    expr: child.attribute("expr").map(str::to_string),
                },
                "convert" => MigrationOp::Convert {
                    data: attr("data")?,
                    algorithm: attr("algorithm")?,
                },
                // `sce_children_among` hands out only the names it was given.
                _ => MigrationOp::DropInvoke { id: attr("id")? },
            };
            changes.push(MigrationChange {
                op,
                source_location: Some(location_of(&child, diag_label)),
            });
        }
        migrations.push(Migration {
            from: shape_file(from, dir, &mut files),
            to: shape_file(to, dir, &mut files),
            changes,
            source_location: Some(location_of(&element, diag_label)),
        });
    }
    Ok((migrations, files))
}

/// The attribute `name` of `node`, the element `tag`, or its refusal.
fn required(
    node: &roxmltree::Node,
    tag: &str,
    name: &str,
    diag_label: &str,
) -> Result<String, Located<ForgeError>> {
    node.attribute(name).map(str::to_string).ok_or_else(|| {
        crate::forge::parser::located(
            node,
            diag_label,
            ValidationError::MissingAttribute {
                element: tag.to_string(),
                attr: name.to_string(),
            },
        )
    })
}

/// Where `node` is written.
fn location_of(node: &roxmltree::Node, diag_label: &str) -> SourceLocation {
    let pos = node.document().text_pos_at(node.range().start);
    SourceLocation {
        file: crate::parser::artifact_label(diag_label),
        line: Some(pos.row),
        col: Some(pos.col),
    }
}

/// The shape file `path` names, read from `dir`; a file read is added to `files`.
fn shape_file(path: String, dir: Option<&Path>, files: &mut Vec<PathBuf>) -> ShapeFile {
    let text = match dir {
        None => Err("the document was read with no directory to read it from".to_string()),
        Some(dir) => {
            let file = dir.join(&path);
            match crate::confine::read_to_string(&file) {
                Ok(text) => {
                    files.push(crate::confine::canonicalize(&file).unwrap_or(file));
                    Ok(text)
                }
                Err(e) => Err(e.to_string()),
            }
        }
    };
    ShapeFile { path, text }
}

// ── A shape text, read ───────────────────────────────────────────────────

/// A state a shape names: its kind and its parent (`-` for none).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShapeState {
    pub kind: String,
    pub parent: String,
}

/// A `<history>` a shape names: shallow or deep, and the state it is declared in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShapeHistory {
    pub kind: String,
    pub parent: String,
}

/// An `<invoke>` a shape names: how it is started (`invoke`, `hybridinvoke`, or
/// `hostinvoke` and the type a host serves it under), and the state that holds it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShapeInvoke {
    pub how: String,
    pub state: String,
}

/// A variable a shape names: its type and bound as the shape spells them, and the
/// lines that spell what its type holds (a record's fields, an enum's variants, a
/// union's cases), each with the variable's own id written `$`, so a variable
/// renamed holds the same lines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShapeVariable {
    pub ty: String,
    pub bound: String,
    pub detail: Vec<String>,
}

/// A shape text, read: what a saved state of one version of a document names.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Shape {
    pub states: BTreeMap<String, ShapeState>,
    pub histories: BTreeMap<String, ShapeHistory>,
    pub invokes: BTreeMap<String, ShapeInvoke>,
    pub variables: BTreeMap<String, ShapeVariable>,
}

impl Shape {
    /// `text` read as a shape, or why it is not one: the header, then one line per
    /// state, history, invocation and variable and the lines of a variable's type
    /// after it, each ending in a newline, as `static_lowering::saved_shape` writes
    /// them. A line of any other kind is refused, so a shape file a later build wrote
    /// with lines this one does not know is not read as one it does.
    pub fn read(text: &str) -> Result<Self, String> {
        let Some(body) = text.strip_suffix('\n') else {
            return Err("its last line does not end in a newline".to_string());
        };
        let mut lines = body.split('\n').enumerate();
        match lines.next() {
            Some((_, SHAPE_HEADER)) => {}
            Some((_, other)) => {
                return Err(format!("it begins `{other}`, not `{SHAPE_HEADER}`"));
            }
            None => return Err("it is empty".to_string()),
        }
        let mut shape = Shape::default();
        let mut last_variable: Option<String> = None;
        for (index, line) in lines {
            let at = index + 1;
            let refused = |why: &str| format!("line {at} (`{line}`) {why}");
            if line.contains('\r') {
                return Err(refused("ends in a carriage return"));
            }
            let words: Vec<&str> = line.split(' ').collect();
            let twice = |what: &str, id: &str| refused(&format!("names the {what} '{id}' twice"));
            match words[..] {
                ["state", id, kind, parent] => {
                    if !["atomic", "compound", "parallel", "final"].contains(&kind) {
                        return Err(refused("names no kind of state"));
                    }
                    let state = ShapeState {
                        kind: kind.to_string(),
                        parent: parent.to_string(),
                    };
                    if shape.states.insert(id.to_string(), state).is_some() {
                        return Err(twice("state", id));
                    }
                    last_variable = None;
                }
                ["history", id, kind, parent] => {
                    if !["shallow", "deep"].contains(&kind) {
                        return Err(refused("names no kind of history"));
                    }
                    let history = ShapeHistory {
                        kind: kind.to_string(),
                        parent: parent.to_string(),
                    };
                    if shape.histories.insert(id.to_string(), history).is_some() {
                        return Err(twice("history", id));
                    }
                    last_variable = None;
                }
                [how @ ("invoke" | "hybridinvoke"), id, state] => {
                    let invoke = ShapeInvoke {
                        how: how.to_string(),
                        state: state.to_string(),
                    };
                    if shape.invokes.insert(id.to_string(), invoke).is_some() {
                        return Err(twice("invocation", id));
                    }
                    last_variable = None;
                }
                ["hostinvoke", ty, id, state] => {
                    let invoke = ShapeInvoke {
                        how: format!("hostinvoke {ty}"),
                        state: state.to_string(),
                    };
                    if shape.invokes.insert(id.to_string(), invoke).is_some() {
                        return Err(twice("invocation", id));
                    }
                    last_variable = None;
                }
                ["variable", id, ref ty @ .., bound] if !ty.is_empty() => {
                    let variable = ShapeVariable {
                        ty: ty.join(" "),
                        bound: bound.to_string(),
                        detail: Vec::new(),
                    };
                    if shape.variables.insert(id.to_string(), variable).is_some() {
                        return Err(twice("variable", id));
                    }
                    last_variable = Some(id.to_string());
                }
                [kind @ ("union" | "case" | "record"), holder, ref rest @ ..] => {
                    let variable = last_variable
                        .as_deref()
                        .filter(|variable| *variable == holder)
                        .ok_or_else(|| refused("follows no line of the variable it names"))?;
                    let detail = [kind, "$"]
                        .into_iter()
                        .chain(rest.iter().copied())
                        .collect::<Vec<_>>()
                        .join(" ");
                    if let Some(held) = shape.variables.get_mut(variable) {
                        held.detail.push(detail);
                    }
                }
                [kind @ ("field" | "variant"), holder, ref rest @ ..] => {
                    let variable = last_variable
                        .as_deref()
                        .filter(|variable| {
                            holder
                                .strip_prefix(*variable)
                                .is_some_and(|tail| tail.starts_with('.'))
                        })
                        .ok_or_else(|| refused("follows no line of the variable it names"))?;
                    let held_at = format!("${}", &holder[variable.len()..]);
                    let detail = [kind, held_at.as_str()]
                        .into_iter()
                        .chain(rest.iter().copied())
                        .collect::<Vec<_>>()
                        .join(" ");
                    if let Some(held) = shape.variables.get_mut(variable) {
                        held.detail.push(detail);
                    }
                }
                _ => return Err(refused("is no line a shape holds")),
            }
        }
        Ok(shape)
    }
}

/// The digests a saved state names a shape text by: the text's own, and the
/// revised one a state that names revisions carries (docs/adr/0044), each the
/// SHA-256 of its text as lowercase hex, as `static_lowering::saved_shape` takes
/// them.
pub fn digests(text: &str) -> (String, String) {
    use sha2::{Digest, Sha256};
    let digest = |text: &str| format!("{:x}", Sha256::digest(text.as_bytes()));
    (digest(text), digest(&format!("{text}{REVISED_LINE}")))
}

// ── The plan ────────────────────────────────────────────────────────────

/// One name a step carries to another.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Renaming {
    pub from: String,
    pub to: String,
}

/// A variable a step adds, with its value as saved-state JSON.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AddedValue {
    pub id: String,
    pub value: String,
}

/// A variable a step converts by a pure algorithm: the variable as the new shape
/// names it, the call of the algorithm as the target spells it, and the types of
/// the old value and of the new one, as a shape spells them (a fixed-width number
/// or `bool`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PlannedConversion {
    pub data: String,
    pub call: String,
    pub param: String,
    pub ret: String,
}

/// One step of a migration as a machine applies it to a saved state (docs/adr/0109,
/// D11): the shape it applies to, the shape it leaves, and what it carries, by
/// name. A runtime applies the steps in order to a state of format 2 or later
/// whose shape is a step's `from` (or `from_revised`, for a state that names
/// revisions), before it judges the shape.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct SavedMigrationStep {
    /// The digest of the `from` shape file's text.
    pub from: String,
    /// The digest of that text and the line `revisions`.
    pub from_revised: String,
    /// The digest of the `to` shape file's text.
    pub to: String,
    /// The digest of that text and the line `revisions`.
    pub to_revised: String,
    /// Each state renamed, and each removed state mapped to one of the new shape.
    pub states: Vec<Renaming>,
    /// Each history renamed.
    pub histories: Vec<Renaming>,
    /// Each history whose state was removed and mapped away: what it recorded has
    /// nowhere to stand.
    pub dropped_histories: Vec<String>,
    /// Each invocation renamed.
    pub invokes: Vec<Renaming>,
    /// Each invocation the new document no longer runs.
    pub dropped_invokes: Vec<String>,
    /// Each event a waiting send names that was renamed.
    pub events: Vec<Renaming>,
    /// Each variable renamed.
    pub data: Vec<Renaming>,
    /// Each variable the new shape does not carry.
    pub dropped_data: Vec<String>,
    /// Each variable, by its new name, whose integer widened from 32 bits or fewer to
    /// 64: a saved state writes the first as a number and the second as a text.
    pub widened_to_text: Vec<String>,
    /// Each variable the step adds, with its value.
    pub added_data: Vec<AddedValue>,
    /// Each variable the step converts.
    pub converted: Vec<PlannedConversion>,
}

/// The plan of the migrations `model` declares, for `target`, judged against
/// `shape`, the shape the machine being built is bound to (`None` for a machine
/// generated without the save API), or the refusal that names what is wrong.
///
/// A target that does not apply a migration refuses a document that declares one,
/// by name (docs/adr/0109, D0). Otherwise each step's shape files are read, the
/// steps are held to form one chain from the first `from` to the document's own
/// shape, and each step is held to account for every state, history, invocation
/// and variable of its `from` and every variable of its `to` ([`judge_step`]).
pub fn plan(
    model: &SCXMLModel,
    scope: &StaticScope,
    shape: Option<&SavedShape>,
    target: &dyn StaticTarget,
) -> Result<Vec<SavedMigrationStep>, GenerateError> {
    let Some(first) = model.migrations.first() else {
        return Ok(Vec::new());
    };
    if !target.lowers_migrations() {
        return Err(GenerateError::unsupported_at(
            format!(
                "<sce:migration> has no {} lowering: a saved state is migrated by the Rust and \
                 C++ restores (docs/adr/0109, D0 and D11)",
                target.name()
            ),
            first.source_location.clone(),
        ));
    }
    let Some(shape) = shape else {
        return Err(refusal(
            ValidationError::MigrationChainBroken {
                path: first.from.path.clone(),
                detail: "the machine has no saved state, so no step ends at its shape: a \
                         document that waits on a session outside its tree is generated \
                         without the save API"
                    .to_string(),
            },
            first.source_location.clone(),
        ));
    };
    // Each step's two shapes, read.
    let mut read = Vec::with_capacity(model.migrations.len());
    for migration in &model.migrations {
        let from = read_shape(&migration.from, migration)?;
        let to = read_shape(&migration.to, migration)?;
        read.push((from, to));
    }
    judge_chain(&model.migrations, shape)?;
    let last = model.migrations.len() - 1;
    model
        .migrations
        .iter()
        .zip(&read)
        .enumerate()
        .map(|(index, (migration, ((from_text, from), (to_text, to))))| {
            let mut step = judge_step(
                migration,
                from,
                to,
                Context {
                    scope,
                    target,
                    last: index == last,
                },
            )?;
            (step.from, step.from_revised) = digests(from_text);
            (step.to, step.to_revised) = digests(to_text);
            Ok(step)
        })
        .collect()
}

/// A refusal of a migration, placed at `at`.
fn refusal(refused: ValidationError, at: Option<SourceLocation>) -> GenerateError {
    GenerateError::Migration {
        refusal: Box::new(refused),
        at,
    }
}

/// The text of `file` and the shape it reads as, or the refusal that it is none.
fn read_shape<'m>(
    file: &'m ShapeFile,
    migration: &Migration,
) -> Result<(&'m str, Shape), GenerateError> {
    let unreadable = |detail: String| {
        refusal(
            ValidationError::MigrationShapeUnreadable {
                path: file.path.clone(),
                detail,
            },
            migration.source_location.clone(),
        )
    };
    let text = file
        .text
        .as_deref()
        .map_err(|why| unreadable(why.clone()))?;
    let shape = Shape::read(text).map_err(unreadable)?;
    Ok((text, shape))
}

/// Refuse `migrations` unless they form one chain, in document order, whose last
/// `to` is `shape`, the document's own: each step's `from` is the text the step
/// before it ends at, no two steps start from one shape, and no step starts from the
/// document's own shape, so a state the machine itself saved is never migrated.
fn judge_chain(migrations: &[Migration], shape: &SavedShape) -> Result<(), GenerateError> {
    // Every file was read by the time the chain is judged ([`read_shape`]).
    fn text(file: &ShapeFile) -> &str {
        file.text.as_deref().unwrap_or("")
    }
    // The chain breaks at `file` of `migration`.
    let broken = |migration: &Migration, file: &ShapeFile, detail: String| {
        refusal(
            ValidationError::MigrationChainBroken {
                path: file.path.clone(),
                detail,
            },
            migration.source_location.clone(),
        )
    };
    let mut starts: BTreeSet<String> = BTreeSet::new();
    for (index, migration) in migrations.iter().enumerate() {
        let (from, _) = digests(text(&migration.from));
        if from == shape.format2_first {
            return Err(broken(
                migration,
                &migration.from,
                format!(
                    "the step from '{}' starts from this document's own shape, so a state this \
                     machine saved would be migrated",
                    migration.from.path
                ),
            ));
        }
        if !starts.insert(from) {
            return Err(broken(
                migration,
                &migration.from,
                format!(
                    "two steps start from the shape of '{}': a state saved under it would be \
                     carried two ways",
                    migration.from.path
                ),
            ));
        }
        if let Some(previous) = index.checked_sub(1).map(|i| &migrations[i]) {
            if text(&migration.from) != text(&previous.to) {
                return Err(broken(
                    migration,
                    &migration.from,
                    format!(
                        "the step from '{}' does not start where the step before it ends, '{}': \
                         the steps form one chain, in the order they are written",
                        migration.from.path, previous.to.path
                    ),
                ));
            }
        }
    }
    let Some(last) = migrations.last() else {
        return Ok(());
    };
    let (ends, _) = digests(text(&last.to));
    if ends != shape.format2_first {
        return Err(broken(
            last,
            &last.to,
            format!(
                "the last step ends at '{}', of shape {ends}, and this document's shape is {}: \
                 write the document's shape to that file (`sce-codegen shape`), or add the step \
                 from it to the document's",
                last.to.path, shape.format2_first
            ),
        ));
    }
    Ok(())
}

/// The refusal of a step that leaves the name it is given unaccounted for, placed
/// at a child of the step or, for `None`, at the step.
type NotTotal<'a> = &'a dyn Fn(&Option<SourceLocation>, &str, String) -> GenerateError;

/// The refusal of a step that carries a variable to a type its old one does not
/// widen to, by the variable's new name.
type TypeChange<'a> = &'a dyn Fn(&Option<SourceLocation>, &str, String) -> GenerateError;

/// What judging one step reads beside its two shapes.
struct Context<'a> {
    scope: &'a StaticScope,
    target: &'a dyn StaticTarget,
    /// Whether the step is the last, whose `to` is the document being built: an
    /// added variable whose `<data>` initialiser is a constant may give no `expr`.
    last: bool,
}

/// The names of one kind a step carries, from the old shape to the new.
#[derive(Default)]
struct Carried {
    /// Each old name renamed or mapped, with the new name.
    to: BTreeMap<String, String>,
}

impl Carried {
    /// The new name of `old`: what the step renames or maps it to, or itself.
    fn of<'a>(&'a self, old: &'a str) -> &'a str {
        self.to.get(old).map_or(old, String::as_str)
    }
}

/// The plan of one step, or the refusal that names what it leaves unaccounted for.
fn judge_step(
    migration: &Migration,
    from: &Shape,
    to: &Shape,
    context: Context<'_>,
) -> Result<SavedMigrationStep, GenerateError> {
    let not_total = |at: &Option<SourceLocation>, subject: &str, detail: String| {
        refusal(
            ValidationError::MigrationNotTotal {
                from: migration.from.path.clone(),
                to: migration.to.path.clone(),
                subject: subject.to_string(),
                detail,
            },
            at.clone().or_else(|| migration.source_location.clone()),
        )
    };
    let type_change = |at: &Option<SourceLocation>, variable: &str, detail: String| {
        refusal(
            ValidationError::MigrationTypeChange {
                from: migration.from.path.clone(),
                to: migration.to.path.clone(),
                variable: variable.to_string(),
                detail,
            },
            at.clone().or_else(|| migration.source_location.clone()),
        )
    };
    let whole = migration.source_location.clone();
    let mut step = SavedMigrationStep::default();

    // What each child names, gathered by kind, each held to the shape it names.
    let mut states = Carried::default();
    let mut mapped: BTreeSet<String> = BTreeSet::new();
    let mut histories = Carried::default();
    let mut invokes = Carried::default();
    let mut dropped_invokes: BTreeSet<String> = BTreeSet::new();
    let mut data = Carried::default();
    let mut dropped_data: BTreeSet<String> = BTreeSet::new();
    let mut added: BTreeMap<String, (Option<String>, Option<SourceLocation>)> = BTreeMap::new();
    let mut converted: BTreeMap<String, (String, Option<SourceLocation>)> = BTreeMap::new();
    let mut events = Carried::default();
    for change in &migration.changes {
        let at = &change.source_location;
        let named_twice =
            |what: &str, id: &str| not_total(at, id, format!("names the {what} '{id}' twice"));
        let old_lacks = |what: &str, id: &str| {
            not_total(
                at,
                id,
                format!(
                    "names the {what} '{id}', which '{}' does not hold",
                    migration.from.path
                ),
            )
        };
        let new_lacks = |what: &str, id: &str| {
            not_total(
                at,
                id,
                format!(
                    "names the {what} '{id}', which '{}' does not hold",
                    migration.to.path
                ),
            )
        };
        match &change.op {
            MigrationOp::Rename {
                kind,
                from: old,
                to: new,
            } => {
                let (table, what, old_holds, new_holds) = match kind {
                    RenameKind::State => (
                        &mut states,
                        "state",
                        from.states.contains_key(old),
                        to.states.contains_key(new),
                    ),
                    RenameKind::History => (
                        &mut histories,
                        "history",
                        from.histories.contains_key(old),
                        to.histories.contains_key(new),
                    ),
                    RenameKind::Data => (
                        &mut data,
                        "variable",
                        from.variables.contains_key(old),
                        to.variables.contains_key(new),
                    ),
                    RenameKind::Invoke => (
                        &mut invokes,
                        "invocation",
                        from.invokes.contains_key(old),
                        to.invokes.contains_key(new),
                    ),
                    // An event is named by no shape: a waiting send names it.
                    RenameKind::Event => (&mut events, "event", true, true),
                };
                if !old_holds {
                    return Err(old_lacks(what, old));
                }
                if !new_holds {
                    return Err(new_lacks(what, new));
                }
                let mapped_too = *kind == RenameKind::State && mapped.contains(old);
                if table.to.insert(old.clone(), new.clone()).is_some() || mapped_too {
                    return Err(named_twice(what, old));
                }
            }
            MigrationOp::MapState { from: old, to: new } => {
                if !from.states.contains_key(old) {
                    return Err(old_lacks("state", old));
                }
                if !to.states.contains_key(new) {
                    return Err(new_lacks("state", new));
                }
                if to.states.contains_key(old) {
                    return Err(not_total(
                        at,
                        old,
                        format!(
                            "maps the state '{old}', which '{}' still holds: a state that \
                             changed is renamed, and only a removed one is mapped",
                            migration.to.path
                        ),
                    ));
                }
                if states.to.insert(old.clone(), new.clone()).is_some()
                    || !mapped.insert(old.clone())
                {
                    return Err(named_twice("state", old));
                }
            }
            MigrationOp::DropData { id } => {
                if !from.variables.contains_key(id) {
                    return Err(old_lacks("variable", id));
                }
                if !dropped_data.insert(id.clone()) {
                    return Err(named_twice("variable", id));
                }
            }
            MigrationOp::AddData { id, expr } => {
                if !to.variables.contains_key(id) {
                    return Err(new_lacks("variable", id));
                }
                if added
                    .insert(id.clone(), (expr.clone(), at.clone()))
                    .is_some()
                {
                    return Err(named_twice("variable", id));
                }
            }
            MigrationOp::Convert {
                data: id,
                algorithm,
            } => {
                if !to.variables.contains_key(id) {
                    return Err(new_lacks("variable", id));
                }
                if converted
                    .insert(id.clone(), (algorithm.clone(), at.clone()))
                    .is_some()
                {
                    return Err(named_twice("variable", id));
                }
            }
            MigrationOp::DropInvoke { id } => {
                if !from.invokes.contains_key(id) {
                    return Err(old_lacks("invocation", id));
                }
                if !dropped_invokes.insert(id.clone()) {
                    return Err(named_twice("invocation", id));
                }
            }
        }
    }

    // Every old state is kept (same name, kind and carried parent), renamed to one of
    // the same kind and carried parent, or mapped; no two old states become one.
    let carried_parent = |parent: &str| -> String {
        if parent == "-" {
            parent.to_string()
        } else {
            states.of(parent).to_string()
        }
    };
    let mut new_states: BTreeMap<&str, &str> = BTreeMap::new();
    for (id, old) in &from.states {
        let target = states.of(id);
        if !mapped.contains(id) {
            let fits = to.states.get(target).is_some_and(|new| {
                new.kind == old.kind && new.parent == carried_parent(&old.parent)
            });
            if !fits {
                let detail = match to.states.get(target) {
                    Some(new) => format!(
                        "the state '{id}' is {} under '{}' in '{}' and '{target}' is {} under '{}' \
                         in '{}': a state is carried with its kind and its parent, and one that \
                         changed either is mapped",
                        old.kind,
                        old.parent,
                        migration.from.path,
                        new.kind,
                        new.parent,
                        migration.to.path
                    ),
                    None => format!(
                        "the state '{id}' of '{}' is neither kept, renamed nor mapped",
                        migration.from.path
                    ),
                };
                return Err(not_total(&whole, id, detail));
            }
            if let Some(other) = new_states.insert(target, id.as_str()) {
                return Err(not_total(
                    &whole,
                    id,
                    format!("the states '{other}' and '{id}' both become '{target}'"),
                ));
            }
        }
    }
    // Every old history is kept or renamed, with its kind and its carried parent, or
    // dropped with the state it was declared in, when that state was mapped away.
    for (id, old) in &from.histories {
        if mapped.contains(&old.parent) && !histories.to.contains_key(id) {
            step.dropped_histories.push(id.clone());
            continue;
        }
        let target = histories.of(id);
        let fits = to
            .histories
            .get(target)
            .is_some_and(|new| new.kind == old.kind && new.parent == carried_parent(&old.parent));
        if !fits {
            return Err(not_total(
                &whole,
                id,
                format!(
                    "the history '{id}' of '{}' is neither kept nor renamed with its kind and its \
                     state, and the state it is declared in is not mapped",
                    migration.from.path
                ),
            ));
        }
    }
    // Every old invocation is kept or renamed, with how it is started and its carried
    // state, or dropped.
    for (id, old) in &from.invokes {
        if dropped_invokes.contains(id) {
            continue;
        }
        let target = invokes.of(id);
        let fits = to
            .invokes
            .get(target)
            .is_some_and(|new| new.how == old.how && new.state == states.of(&old.state));
        if !fits {
            return Err(not_total(
                &whole,
                id,
                format!(
                    "the invocation '{id}' of '{}' is neither kept nor renamed, started as it was \
                     in the state that held it, nor dropped",
                    migration.from.path
                ),
            ));
        }
    }
    // Every old variable is dropped, or carried (kept or renamed) to a variable of the
    // new shape, of a type its old one widens to unless a conversion gives it.
    let mut carried_to: BTreeMap<String, String> = BTreeMap::new();
    for (id, old) in &from.variables {
        if dropped_data.contains(id) {
            if data.to.contains_key(id) {
                return Err(not_total(
                    &whole,
                    id,
                    format!("the variable '{id}' is both dropped and renamed"),
                ));
            }
            continue;
        }
        let target = data.of(id).to_string();
        let Some(new) = to.variables.get(&target) else {
            return Err(not_total(
                &whole,
                id,
                format!(
                    "the variable '{id}' of '{}' is neither kept, renamed nor dropped",
                    migration.from.path
                ),
            ));
        };
        if let Some((_, at)) = added.get(&target) {
            return Err(not_total(
                at,
                &target,
                format!("adds the variable '{target}', which the step carries from '{id}'"),
            ));
        }
        if let Some(other) = carried_to.insert(target.clone(), id.clone()) {
            return Err(not_total(
                &whole,
                &target,
                format!("the variables '{other}' and '{id}' both become '{target}'"),
            ));
        }
        if let Some((algorithm, at)) = converted.get(&target) {
            step.converted.push(conversion(
                &target,
                algorithm,
                old,
                new,
                at,
                &context,
                &type_change,
            )?);
            continue;
        }
        match widening(old, new) {
            Ok(Widening::Same) => {}
            Ok(Widening::NumberToText) => step.widened_to_text.push(target),
            Err(why) => {
                return Err(type_change(
                    &whole,
                    &target,
                    format!(
                        "is {} in '{}' and {} in '{}': {why}, which is no widening, and no \
                         <sce:convert> converts it",
                        spelled(old),
                        migration.from.path,
                        spelled(new),
                        migration.to.path
                    ),
                ));
            }
        }
    }
    // A conversion gives a variable carried from an old one: one that names a variable
    // nothing carries converts nothing.
    for (id, (_, at)) in &converted {
        if !carried_to.contains_key(id) {
            return Err(not_total(
                at,
                id,
                format!(
                    "converts the variable '{id}', which no variable of '{}' is carried to",
                    migration.from.path
                ),
            ));
        }
    }
    // Every new variable is carried or added.
    for (id, new) in &to.variables {
        if carried_to.contains_key(id) {
            continue;
        }
        let Some((expr, at)) = added.get(id) else {
            return Err(not_total(
                &whole,
                id,
                format!(
                    "the variable '{id}' of '{}' is neither kept, renamed nor added",
                    migration.to.path
                ),
            ));
        };
        let value = added_value(
            id,
            expr.as_deref(),
            new,
            at,
            &context,
            &not_total,
            &type_change,
        )?;
        step.added_data.push(AddedValue {
            id: id.clone(),
            value,
        });
    }

    let pairs = |carried: Carried| {
        carried
            .to
            .into_iter()
            .map(|(from, to)| Renaming { from, to })
            .collect::<Vec<_>>()
    };
    step.states = pairs(states);
    step.histories = pairs(histories);
    step.invokes = pairs(invokes);
    step.events = pairs(events);
    step.data = pairs(data);
    step.dropped_invokes = dropped_invokes.into_iter().collect();
    step.dropped_data = dropped_data.into_iter().collect();
    Ok(step)
}

/// A variable's type and bound as a refusal writes them.
fn spelled(variable: &ShapeVariable) -> String {
    if variable.bound == "-" {
        variable.ty.clone()
    } else {
        format!("{}[{}]", variable.ty, variable.bound)
    }
}

/// How a saved value of a variable carried to a type its old one widens to is read
/// as the new one.
#[derive(Debug, PartialEq, Eq)]
enum Widening {
    /// As it is written.
    Same,
    /// A number written as a text: an integer of 32 bits or fewer widened to 64,
    /// which a saved state writes as a text so no reader loses its low bits.
    NumberToText,
}

/// The sign and the width of an integer type a shape spells, or `None` for another.
fn integer(ty: &str) -> Option<(bool, u32)> {
    match ty {
        "uint8" => Some((false, 8)),
        "uint16" => Some((false, 16)),
        "uint32" => Some((false, 32)),
        "uint64" => Some((false, 64)),
        "int8" => Some((true, 8)),
        "int16" => Some((true, 16)),
        "int32" => Some((true, 32)),
        "int64" => Some((true, 64)),
        _ => None,
    }
}

/// Whether `old` widens to `new` (docs/adr/0109, D11): the same type, or a larger
/// bound of the same text, byte string or list; an integer to a wider one of the
/// same sign, or an unsigned one to a wider signed one; `float32` to `float64`. A
/// widening from `T` to `optional<T>` leaves the shape as it was, and a record schema
/// or an enum grown by revision is the restore's to read (docs/adr/0044), so neither
/// is judged here. Anything else is why it is not one.
fn widening(old: &ShapeVariable, new: &ShapeVariable) -> Result<Widening, String> {
    if old.detail != new.detail {
        return Err("what the type holds changed".to_string());
    }
    if old.ty == new.ty {
        return match (old.bound.as_str(), new.bound.as_str()) {
            ("-", "-") => Ok(Widening::Same),
            (was, now) => match (was.parse::<u64>(), now.parse::<u64>()) {
                (Ok(was), Ok(now)) if now >= was => Ok(Widening::Same),
                (Ok(was), Ok(now)) => Err(format!("the bound shrank from {was} to {now}")),
                _ => Err("the bound changed".to_string()),
            },
        };
    }
    if old.bound != "-" || new.bound != "-" {
        return Err("the type changed".to_string());
    }
    if let (Some((was_signed, was)), Some((now_signed, now))) = (integer(&old.ty), integer(&new.ty))
    {
        let wider =
            (was_signed == now_signed && now >= was) || (!was_signed && now_signed && now > was);
        if !wider {
            return Err("the integer narrowed or changed its sign".to_string());
        }
        return Ok(if was <= 32 && now == 64 {
            Widening::NumberToText
        } else {
            Widening::Same
        });
    }
    if old.ty == "float32" && new.ty == "float64" {
        return Ok(Widening::Same);
    }
    Err("the type changed".to_string())
}

/// Whether a shape's type is one a conversion reads and writes: a fixed-width number
/// or `bool`, each of which every runtime reads and writes by its type alone.
fn converts(ty: &str) -> bool {
    integer(ty).is_some() || matches!(ty, "float32" | "float64" | "bool")
}

/// The conversion of the variable `data`, from `old` to `new`, by the algorithm the
/// document imports as `algorithm`: a pure algorithm of one parameter whose type is
/// the old value's and whose return is the new one's.
fn conversion(
    data: &str,
    algorithm: &str,
    old: &ShapeVariable,
    new: &ShapeVariable,
    at: &Option<SourceLocation>,
    context: &Context<'_>,
    type_change: TypeChange<'_>,
) -> Result<PlannedConversion, GenerateError> {
    let lang = context.target.name();
    if !converts(&old.ty) || !converts(&new.ty) || old.bound != "-" || new.bound != "-" {
        return Err(GenerateError::unsupported_at(
            format!(
                "<sce:convert data=\"{data}\"> converts {} to {}, and a conversion has a {lang} \
                 lowering for a fixed-width number or bool only (docs/adr/0109, D11)",
                spelled(old),
                spelled(new)
            ),
            at.clone(),
        ));
    }
    let callee = context
        .scope
        .callees
        .iter()
        .find(|callee| callee.alias == algorithm && callee.kind.is_algorithm())
        .ok_or_else(|| {
            type_change(
                at,
                data,
                format!(
                    "is converted by '{algorithm}', which is no algorithm the document imports"
                ),
            )
        })?;
    let takes: Vec<String> = callee.params.iter().map(|p| p.as_attr()).collect();
    let gives = callee.ret.as_ref().map(|r| r.as_attr());
    let fits = matches!(takes.as_slice(), [param] if *param == old.ty)
        && gives.as_deref() == Some(new.ty.as_str());
    if !fits {
        return Err(type_change(
            at,
            data,
            format!(
                "is {} and becomes {}, and '{algorithm}' takes ({}) and gives {}",
                old.ty,
                new.ty,
                takes.join(", "),
                gives.as_deref().unwrap_or("nothing")
            ),
        ));
    }
    let pure = matches!(callee.reading, crate::forge::types::CallReading::Value)
        && !callee.may_fail
        && callee.host_only.is_none();
    let call = context
        .target
        .callee(callee)
        .filter(|_| pure)
        .filter(|call| !matches!(call.import, CalleeImport::Unwritable))
        .ok_or_else(|| {
            GenerateError::unsupported_at(
                format!(
                    "<sce:convert data=\"{data}\" algorithm=\"{algorithm}\"> has no {lang} \
                     lowering: a conversion calls an algorithm that cannot fail, of one value, \
                     which {lang} reaches by its import (docs/adr/0109, D11)"
                ),
                at.clone(),
            )
        })?;
    Ok(PlannedConversion {
        data: data.to_string(),
        call: call.call,
        param: old.ty.clone(),
        ret: new.ty.clone(),
    })
}

/// Why a written value is not a constant of a variable's type.
enum NotAConstant {
    /// It is no literal.
    Written,
    /// It is a literal of another type, for the reason given.
    OfType(String),
}

/// The value of the variable `id` a step adds, as saved-state JSON: the step's
/// `expr`, or in the last step the variable's `<data>` initialiser, each a constant
/// of the variable's type.
fn added_value(
    id: &str,
    expr: Option<&str>,
    variable: &ShapeVariable,
    at: &Option<SourceLocation>,
    context: &Context<'_>,
    not_total: NotTotal<'_>,
    type_change: TypeChange<'_>,
) -> Result<String, GenerateError> {
    // The variable of the document being built, read for the last step alone: an
    // earlier step's `to` is a version whose document is not at hand.
    let declared = context
        .scope
        .variables
        .iter()
        .find(|v| v.id == id)
        .filter(|_| context.last);
    let optional = declared.map(|v| {
        v.value_type
            .as_ref()
            .and_then(crate::forge::model::AlgorithmValueType::optional_elem)
            .is_some()
    });
    let written = match expr {
        Some(expr) => expr,
        None => match declared
            .map(|v| v.expr.as_str())
            .filter(|e| !e.trim().is_empty())
        {
            Some(initialiser) => initialiser,
            None => {
                return Err(not_total(
                    at,
                    id,
                    format!(
                        "adds the variable '{id}' with no value: write `expr`, a constant (in the \
                         last step a <data> initialiser that is a constant serves)"
                    ),
                ));
            }
        },
    };
    constant(written, variable, optional).map_err(|refused| match refused {
        NotAConstant::Written => not_total(
            at,
            id,
            format!(
                "adds the variable '{id}' as `{written}`, which is no constant: a step gives an \
                 added variable a literal of its type"
            ),
        ),
        NotAConstant::OfType(why) => {
            type_change(at, id, format!("is added as `{written}`, which {why}"))
        }
    })
}

/// `written`, a literal, as saved-state JSON of `variable`'s type: a whole number or
/// a real, `true` or `false`, a quoted text, an enum's `Alias.Variant`, or `null`
/// where the variable is known to be optional (`optional`). A 64-bit integer is
/// written as a text and a 32-bit real as the 64-bit real it widens to, as every
/// runtime writes them.
fn constant(
    written: &str,
    variable: &ShapeVariable,
    optional: Option<bool>,
) -> Result<String, NotAConstant> {
    let written = written.trim();
    let ty = variable.ty.as_str();
    let not_of = |why: String| Err(NotAConstant::OfType(why));
    if written == "null" {
        return match optional {
            Some(true) => Ok("null".to_string()),
            Some(false) => not_of(format!("is no value of {ty}, which is not optional")),
            None => not_of(
                "is a value only a variable known to be optional holds, and an earlier step's \
                 variable is not known"
                    .to_string(),
            ),
        };
    }
    if written == "true" || written == "false" {
        return if ty == "bool" {
            Ok(written.to_string())
        } else {
            not_of(format!("is a bool, and the variable is {ty}"))
        };
    }
    if let Some(text) = quoted(written) {
        if ty != "string" {
            return not_of(format!("is a text, and the variable is {ty}"));
        }
        if let Ok(bound) = variable.bound.parse::<usize>() {
            if text.len() > bound {
                return not_of(format!(
                    "is {} bytes, and the variable holds at most {bound}",
                    text.len()
                ));
            }
        }
        return serde_json::to_string(text).map_err(|e| NotAConstant::OfType(e.to_string()));
    }
    if let Some((_, name)) = written
        .split_once('.')
        .filter(|(alias, name)| identifier(alias) && identifier(name))
    {
        if !ty.starts_with("enum:") {
            return not_of(format!("is a value of an enum, and the variable is {ty}"));
        }
        if !variable
            .detail
            .iter()
            .any(|line| line == &format!("variant $.{name}"))
        {
            return not_of(format!("names no variant '{name}' of the variable's enum"));
        }
        return serde_json::to_string(name).map_err(|e| NotAConstant::OfType(e.to_string()));
    }
    if let Some(whole) = whole_number(written) {
        if let Some((signed, bits)) = integer(ty) {
            let (least, most): (i128, i128) = match (signed, bits) {
                (true, bits) => (-(1i128 << (bits - 1)), (1i128 << (bits - 1)) - 1),
                (false, bits) => (0, (1i128 << bits) - 1),
            };
            if whole < least || whole > most {
                return not_of(format!("is not a value {ty} can hold"));
            }
            return Ok(if bits == 64 {
                format!("\"{whole}\"")
            } else {
                whole.to_string()
            });
        }
    }
    let real = written
        .parse::<f64>()
        .ok()
        .filter(|r| r.is_finite())
        .filter(|_| {
            written
                .bytes()
                .all(|b| b.is_ascii_digit() || b"+-.eE".contains(&b))
        });
    if let Some(real) = real {
        return match ty {
            "float64" => Ok(format!("{real:?}")),
            "float32" => Ok(format!("{:?}", f64::from(real as f32))),
            _ => not_of(format!("is a number, and the variable is {ty}")),
        };
    }
    Err(NotAConstant::Written)
}

/// The text a literal quoted in `'` or `"` holds, when it holds no quote of its kind
/// and no escape.
fn quoted(written: &str) -> Option<&str> {
    ['\'', '"'].iter().find_map(|&quote| {
        written
            .strip_prefix(quote)
            .and_then(|rest| rest.strip_suffix(quote))
            .filter(|text| !text.contains(quote) && !text.contains('\\'))
    })
}

/// Whether `word` is an identifier: a letter or `_`, then letters, digits or `_`.
fn identifier(word: &str) -> bool {
    let mut chars = word.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// `written` as a whole number: an optional minus, then digits.
fn whole_number(written: &str) -> Option<i128> {
    let digits = written.strip_prefix('-').unwrap_or(written);
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    written.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::SCXMLParser;

    /// A `sce-static` document of `body`, with `migration` (a `<sce:migration>`, or
    /// nothing) on its root.
    fn document(migration: &str, body: &str) -> String {
        format!(
            r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" initial="idle" datamodel="sce-static" name="m">{migration}{body}</scxml>"#
        )
    }

    /// The version a state is saved by: `waiting` is renamed in the next, `legacy`
    /// removed, `scratch` dropped, and `total` widened.
    const VERSION_1: &str = r#"<datamodel><data id="count" sce:type="uint32" expr="0"/><data id="total" sce:type="int32" expr="0"/><data id="scratch" sce:type="uint8" expr="0"/></datamodel><state id="idle"><transition event="go" target="waiting"/></state><state id="waiting"><transition event="finish" target="done"/></state><state id="legacy"/><final id="done"/>"#;

    /// The next version, which adds `ceiling`.
    const VERSION_2: &str = r#"<datamodel><data id="count" sce:type="uint32" expr="0"/><data id="total" sce:type="int64" expr="0"/><data id="ceiling" sce:type="uint32" expr="16"/></datamodel><state id="idle"><transition event="go" target="pending"/></state><state id="pending"><transition event="finish" target="done"/></state><final id="done"/>"#;

    /// The step from the first version to the second, accounting for every name.
    const STEP: &str = r#"<sce:migration from="1.shape" to="2.shape"><sce:rename kind="state" from="waiting" to="pending"/><sce:map-state from="legacy" to="idle"/><sce:drop-data id="scratch"/><sce:add-data id="ceiling" expr="16"/></sce:migration>"#;

    fn parsed(source: &str) -> SCXMLModel {
        let mut model = SCXMLParser::new()
            .parse_string(source, "m")
            .expect("parses");
        crate::analyzer::analyze(&mut model, "m.scxml");
        model
    }

    /// The shape file of a document of `body`.
    fn shape_of(body: &str) -> String {
        crate::forge::static_lowering::saved_shape_text(&parsed(&document("", body)))
            .expect("a machine of fields alone has a shape")
    }

    /// The shape files of the two versions, by the paths `STEP` names them by.
    fn files() -> Vec<(&'static str, String)> {
        vec![
            ("1.shape", shape_of(VERSION_1)),
            ("2.shape", shape_of(VERSION_2)),
        ]
    }

    /// The plan of `migration` on a document of `body`, lowered for Rust, each shape
    /// file it names read as `files` gives it; a path `files` does not give is one a
    /// parse with no directory did not read.
    fn planned(
        migration: &str,
        body: &str,
        files: &[(&str, String)],
    ) -> Result<Vec<SavedMigrationStep>, GenerateError> {
        let mut model = parsed(&document(migration, body));
        for step in &mut model.migrations {
            for file in [&mut step.from, &mut step.to] {
                if let Some((_, text)) = files.iter().find(|(path, _)| *path == file.path) {
                    file.text = Ok(text.clone());
                }
            }
        }
        crate::forge::static_lowering::lower_rust(&mut model, "M")
            .map(|lowered| lowered.saved_shape.expect("a shape").migrations)
    }

    /// The validation refusal a plan was refused with.
    fn refused(planned: Result<Vec<SavedMigrationStep>, GenerateError>) -> ValidationError {
        match planned {
            Err(GenerateError::Migration { refusal, .. }) => *refusal,
            other => panic!("expected a refused migration, got {other:?}"),
        }
    }

    /// The name a `validation/migration-not-total` refusal is about.
    fn not_total(planned: Result<Vec<SavedMigrationStep>, GenerateError>) -> String {
        match refused(planned) {
            ValidationError::MigrationNotTotal { subject, .. } => subject,
            other => panic!("expected a step that is not total, got {other}"),
        }
    }

    fn renaming(from: &str, to: &str) -> Renaming {
        Renaming {
            from: from.to_string(),
            to: to.to_string(),
        }
    }

    #[test]
    fn the_parse_reads_every_child_of_a_migration_in_order() {
        let model = parsed(&document(
            r#"<sce:migration from="a.shape" to="b.shape"><sce:rename kind="event" from="tock" to="tick"/><sce:convert data="due" algorithm="DueOf"/><sce:drop-invoke id="poller"/><sce:add-data id="limit"/></sce:migration>"#,
            VERSION_2,
        ));
        assert_eq!(model.migrations.len(), 1);
        let migration = &model.migrations[0];
        assert_eq!(migration.from.path, "a.shape");
        assert_eq!(migration.to.path, "b.shape");
        assert!(
            migration.from.text.is_err(),
            "a parse with no directory reads no file"
        );
        let ops: Vec<String> = migration
            .changes
            .iter()
            .map(|change| format!("{:?}", change.op))
            .collect();
        assert_eq!(
            ops,
            vec![
                r#"Rename { kind: Event, from: "tock", to: "tick" }"#,
                r#"Convert { data: "due", algorithm: "DueOf" }"#,
                r#"DropInvoke { id: "poller" }"#,
                r#"AddData { id: "limit", expr: None }"#,
            ]
        );
    }

    #[test]
    fn a_rename_of_no_known_kind_is_refused_by_the_parse() {
        let parse = SCXMLParser::new().parse_string(
            &document(
                r#"<sce:migration from="a.shape" to="b.shape"><sce:rename kind="table" from="a" to="b"/></sce:migration>"#,
                VERSION_2,
            ),
            "m",
        );
        assert!(parse.is_err(), "a `kind` outside the closed set is refused");
    }

    #[test]
    fn a_migration_in_a_document_of_another_data_model_is_refused_by_the_parse() {
        let source = document(STEP, r#"<state id="idle"/>"#).replace("sce-static", "ecmascript");
        let refusal = SCXMLParser::new()
            .parse_string(&source, "m")
            .map(|_| ())
            .expect_err("a migration carries a saved state, which only sce-static has");
        assert!(refusal.to_string().contains("datamodel"), "{refusal}");
    }

    #[test]
    fn a_step_that_accounts_for_every_name_is_planned() {
        let plan = planned(STEP, VERSION_2, &files()).expect("a total step is planned");
        assert_eq!(plan.len(), 1);
        let step = &plan[0];
        assert_eq!(step.from, digests(&shape_of(VERSION_1)).0);
        assert_eq!(step.from_revised, digests(&shape_of(VERSION_1)).1);
        assert_eq!(step.to, digests(&shape_of(VERSION_2)).0);
        assert_eq!(
            step.states,
            vec![renaming("legacy", "idle"), renaming("waiting", "pending")],
            "a rename and a map both carry a state to one of the new document"
        );
        assert_eq!(step.dropped_data, vec!["scratch".to_string()]);
        assert_eq!(
            step.widened_to_text,
            vec!["total".to_string()],
            "an int32 widened to an int64 is read from a number as a text"
        );
        assert_eq!(
            step.added_data,
            vec![AddedValue {
                id: "ceiling".to_string(),
                value: "16".to_string()
            }]
        );
    }

    #[test]
    fn the_last_step_adds_a_variable_with_its_constant_initialiser() {
        let step = STEP.replace(r#" expr="16""#, "");
        let plan = planned(&step, VERSION_2, &files()).expect("the `<data>` initialiser serves");
        assert_eq!(plan[0].added_data[0].value, "16");
    }

    #[test]
    fn each_name_a_step_leaves_unaccounted_for_is_refused_by_name() {
        for (left_out, subject) in [
            (
                r#"<sce:rename kind="state" from="waiting" to="pending"/>"#,
                "waiting",
            ),
            (r#"<sce:map-state from="legacy" to="idle"/>"#, "legacy"),
            (r#"<sce:drop-data id="scratch"/>"#, "scratch"),
            (r#"<sce:add-data id="ceiling" expr="16"/>"#, "ceiling"),
        ] {
            let step = STEP.replace(left_out, "");
            assert_eq!(
                not_total(planned(&step, VERSION_2, &files())),
                subject,
                "without {left_out}"
            );
        }
    }

    #[test]
    fn a_child_that_names_what_its_shape_does_not_hold_is_refused() {
        // Each child written beside the step's own, which account for every name.
        let beside =
            |child: &str| STEP.replace("</sce:migration>", &format!("{child}</sce:migration>"));
        for (step, subject) in [
            (STEP.replace(r#"to="pending""#, r#"to="parked""#), "parked"),
            (
                beside(r#"<sce:rename kind="data" from="missing" to="count"/>"#),
                "missing",
            ),
            (beside(r#"<sce:drop-invoke id="poller"/>"#), "poller"),
            (beside(r#"<sce:map-state from="idle" to="done"/>"#), "idle"),
            (beside(r#"<sce:add-data id="count" expr="1"/>"#), "count"),
            (beside(r#"<sce:drop-data id="scratch"/>"#), "scratch"),
        ] {
            assert_eq!(
                not_total(planned(&step, VERSION_2, &files())),
                subject,
                "{step}"
            );
        }
    }

    #[test]
    fn an_added_variable_given_no_constant_is_refused() {
        let step = STEP.replace(r#"expr="16""#, r#"expr="count + 1""#);
        assert_eq!(not_total(planned(&step, VERSION_2, &files())), "ceiling");
    }

    #[test]
    fn a_type_that_does_not_widen_is_refused_unless_converted() {
        let narrowed = VERSION_2.replace(
            r#"id="total" sce:type="int64""#,
            r#"id="total" sce:type="int16""#,
        );
        let files = vec![
            ("1.shape", shape_of(VERSION_1)),
            ("2.shape", shape_of(&narrowed)),
        ];
        match refused(planned(STEP, &narrowed, &files)) {
            ValidationError::MigrationTypeChange { variable, .. } => assert_eq!(variable, "total"),
            other => panic!("expected a type change, got {other}"),
        }
        // A conversion names an algorithm the document imports; one it does not is
        // no conversion.
        let converted = STEP.replace(
            "</sce:migration>",
            r#"<sce:convert data="total" algorithm="Narrow"/></sce:migration>"#,
        );
        match refused(planned(&converted, &narrowed, &files)) {
            ValidationError::MigrationTypeChange {
                variable, detail, ..
            } => {
                assert_eq!(variable, "total");
                assert!(
                    detail.contains("no algorithm the document imports"),
                    "{detail}"
                );
            }
            other => panic!("expected a type change, got {other}"),
        }
    }

    #[test]
    fn a_constant_of_another_type_is_refused_as_a_type_change() {
        let step = STEP.replace(r#"expr="16""#, r#"expr="'sixteen'""#);
        match refused(planned(&step, VERSION_2, &files())) {
            ValidationError::MigrationTypeChange { variable, .. } => {
                assert_eq!(variable, "ceiling")
            }
            other => panic!("expected a type change, got {other}"),
        }
    }

    #[test]
    fn a_shape_file_that_was_not_read_or_is_no_shape_is_refused() {
        match refused(planned(STEP, VERSION_2, &[])) {
            ValidationError::MigrationShapeUnreadable { path, .. } => assert_eq!(path, "1.shape"),
            other => panic!("expected an unreadable shape, got {other}"),
        }
        let files = vec![
            ("1.shape", "a shape\n".to_string()),
            ("2.shape", shape_of(VERSION_2)),
        ];
        match refused(planned(STEP, VERSION_2, &files)) {
            ValidationError::MigrationShapeUnreadable { path, detail } => {
                assert_eq!(path, "1.shape");
                assert!(detail.contains(SHAPE_HEADER), "{detail}");
            }
            other => panic!("expected an unreadable shape, got {other}"),
        }
    }

    #[test]
    fn a_last_step_that_does_not_end_at_the_documents_shape_is_refused() {
        let files = vec![
            ("1.shape", shape_of(VERSION_1)),
            ("2.shape", shape_of(VERSION_1)),
        ];
        match refused(planned(STEP, VERSION_2, &files)) {
            ValidationError::MigrationChainBroken { path, .. } => assert_eq!(path, "2.shape"),
            other => panic!("expected a broken chain, got {other}"),
        }
    }

    #[test]
    fn steps_that_do_not_meet_are_refused_as_a_chain_with_a_gap() {
        // Three versions of shapes, none another: the first step ends at the first
        // version's, and the second starts from neither it nor any other.
        let version_0 = VERSION_1.replace(r#"<state id="legacy"/>"#, "");
        let elsewhere = VERSION_1.replace(r#"<state id="legacy"/>"#, r#"<state id="older"/>"#);
        let two = format!(
            r#"<sce:migration from="0.shape" to="1.shape"/>{}"#,
            STEP.replace(r#"from="1.shape""#, r#"from="gap.shape""#)
        );
        let files = vec![
            ("0.shape", shape_of(&version_0)),
            ("1.shape", shape_of(VERSION_1)),
            ("gap.shape", shape_of(&elsewhere)),
            ("2.shape", shape_of(VERSION_2)),
        ];
        match refused(planned(&two, VERSION_2, &files)) {
            ValidationError::MigrationChainBroken { path, .. } => assert_eq!(path, "gap.shape"),
            other => panic!("expected a broken chain, got {other}"),
        }
    }

    #[test]
    fn a_step_from_the_documents_own_shape_is_refused() {
        let files = vec![
            ("1.shape", shape_of(VERSION_2)),
            ("2.shape", shape_of(VERSION_2)),
        ];
        match refused(planned(STEP, VERSION_2, &files)) {
            ValidationError::MigrationChainBroken { path, .. } => assert_eq!(path, "1.shape"),
            other => panic!("expected a broken chain, got {other}"),
        }
    }

    #[test]
    fn a_backend_that_does_not_migrate_refuses_the_document_by_name() {
        let mut model = parsed(&document(STEP, VERSION_2));
        let refusal = crate::forge::static_lowering::lower_kotlin(&mut model, "M")
            .map(|_| ())
            .expect_err("Kotlin applies no migration (docs/adr/0109, D0)");
        assert!(
            matches!(&refusal, GenerateError::UnsupportedFeature { detail, .. } if detail.contains("<sce:migration>")),
            "{refusal}"
        );
    }

    const V1: &str = "sce-saved-state-shape 1\n\
                      state idle atomic -\n\
                      state waiting atomic -\n\
                      state done final -\n\
                      variable count uint32 -\n";

    #[test]
    fn a_shape_text_reads_each_line_it_holds() {
        let shape = Shape::read(V1).expect("a shape");
        assert_eq!(shape.states.len(), 3);
        assert_eq!(
            shape.states["waiting"],
            ShapeState {
                kind: "atomic".into(),
                parent: "-".into()
            }
        );
        assert_eq!(shape.variables["count"].ty, "uint32");
        assert_eq!(shape.variables["count"].bound, "-");
    }

    #[test]
    fn a_variable_holds_the_lines_of_its_type_under_its_own_name() {
        let text = "sce-saved-state-shape 1\n\
                    variable shown record:Day -\n\
                    field shown.year uint16\n\
                    variable status enum:Status -\n\
                    variant status.done\n\
                    variant status.open\n";
        let shape = Shape::read(text).expect("a shape");
        assert_eq!(shape.variables["shown"].detail, vec!["field $.year uint16"]);
        assert_eq!(
            shape.variables["status"].detail,
            vec!["variant $.done", "variant $.open"]
        );
    }

    #[test]
    fn a_text_that_is_no_shape_is_refused_with_its_reason() {
        for (text, why) in [
            ("state idle atomic -\n", "begins"),
            ("sce-saved-state-shape 1\nstate idle atomic -", "newline"),
            (
                "sce-saved-state-shape 1\nstate idle blue -\n",
                "no kind of state",
            ),
            (
                "sce-saved-state-shape 1\nrevisions\n",
                "no line a shape holds",
            ),
            (
                "sce-saved-state-shape 1\nfield x.y uint8\n",
                "follows no line",
            ),
            (
                "sce-saved-state-shape 1\nstate a atomic -\nstate a atomic -\n",
                "twice",
            ),
        ] {
            let refused = Shape::read(text).expect_err(text);
            assert!(refused.contains(why), "{text:?}: {refused}");
        }
    }

    #[test]
    fn the_digests_are_those_of_the_text_and_of_the_text_revised() {
        let (first, revised) = digests(V1);
        assert_eq!(first.len(), 64);
        assert_ne!(first, revised);
        assert_ne!(digests(&V1.replace("count", "total")).0, first);
    }

    fn variable(ty: &str, bound: &str) -> ShapeVariable {
        ShapeVariable {
            ty: ty.into(),
            bound: bound.into(),
            detail: Vec::new(),
        }
    }

    #[test]
    fn a_widening_is_read_as_it_is_written_or_as_a_text() {
        let widens = |old: (&str, &str), new: (&str, &str)| {
            widening(&variable(old.0, old.1), &variable(new.0, new.1))
        };
        assert_eq!(widens(("uint8", "-"), ("uint16", "-")), Ok(Widening::Same));
        assert_eq!(
            widens(("int32", "-"), ("int64", "-")),
            Ok(Widening::NumberToText)
        );
        assert_eq!(
            widens(("uint32", "-"), ("int64", "-")),
            Ok(Widening::NumberToText)
        );
        assert_eq!(
            widens(("float32", "-"), ("float64", "-")),
            Ok(Widening::Same)
        );
        assert_eq!(
            widens(("string", "8"), ("string", "16")),
            Ok(Widening::Same)
        );
        assert!(widens(("string", "16"), ("string", "8")).is_err());
        assert!(widens(("int32", "-"), ("uint64", "-")).is_err());
        assert!(widens(("uint32", "-"), ("int32", "-")).is_err());
        assert!(widens(("int64", "-"), ("int32", "-")).is_err());
        assert!(widens(("float64", "-"), ("float32", "-")).is_err());
        assert!(widens(("uint32", "-"), ("bool", "-")).is_err());
    }

    #[test]
    fn a_constant_is_written_as_a_saved_state_writes_its_type() {
        let of = |written: &str, ty: &str, bound: &str| {
            constant(written, &variable(ty, bound), Some(false)).ok()
        };
        assert_eq!(of("16", "uint32", "-").as_deref(), Some("16"));
        assert_eq!(of("-3", "int64", "-").as_deref(), Some("\"-3\""));
        assert_eq!(of("2", "float64", "-").as_deref(), Some("2.0"));
        assert_eq!(of("true", "bool", "-").as_deref(), Some("true"));
        assert_eq!(of("'ab'", "string", "4").as_deref(), Some("\"ab\""));
        assert_eq!(of("256", "uint8", "-"), None);
        assert_eq!(of("'abcde'", "string", "4"), None);
        assert_eq!(of("16", "string", "4"), None);
        assert!(matches!(
            constant("count + 1", &variable("uint32", "-"), None),
            Err(NotAConstant::Written)
        ));
        assert_eq!(
            constant("null", &variable("uint32", "-"), Some(true))
                .ok()
                .as_deref(),
            Some("null")
        );
        let mut status = variable("enum:Status", "-");
        status.detail.push("variant $.open".into());
        assert_eq!(
            constant("Status.open", &status, None).ok().as_deref(),
            Some("\"open\"")
        );
        assert!(constant("Status.shut", &status, None).is_err());
    }
}

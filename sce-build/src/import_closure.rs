// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The documents a set of documents is generated from, each once, and where
//! each one's generated unit goes.
//!
//! `sce-codegen generate` generates the document it is given and nothing that
//! document imports: a forge kind an `<sce:import>` names is a unit of its own,
//! which the importer's generated code names by package and does not contain.
//! A build of a design that imports documents, its own and the standard
//! library's, has to generate each of them as well, and each where the import
//! line that names it looks for it. This is the walk that finds them, written
//! once, so a build does not keep its own copy of it in a script beside the
//! generator (`generate --with-imports`).
//!
//! What the walk follows from a document:
//!
//! * every `<sce:import>`, resolved as the generator resolves it: an
//!   `sce:std/...` name in the library, any other `src` beside the importing
//!   document ([`stdlib::resolve`]). A statechart's imports are read wherever
//!   they are written, an in-line child's among them, since the statechart's
//!   own run generates its in-line children and so has to find theirs;
//! * every `<invoke src="sce:std/...">`: a standard statechart a statechart
//!   starts is generated in-line by its parent's own run, so it is not a unit
//!   here, but what it imports is;
//! * every entry of a hybrid `<invoke>`'s `sce:candidates`: a document the
//!   statechart may start, generated as its child into its package, as
//!   `generate --as-child --parent-stem` generates one.
//!
//! A reference that names no document is refused where it is written, and the
//! refusal carries the chain of references that brought that document into the
//! run ([`Refusal::reached_from`]): an import three documents down is
//! otherwise reported in a file the caller never named, with nothing saying
//! why it was read.

use std::collections::{BTreeSet, VecDeque};
use std::path::{Component, Path, PathBuf};

use crate::forge::error::{
    ForgeError, ImportError, Located, RelatedRole, RelatedSite, SourceLocation, ValidationError,
    XmlError,
};
use crate::forge::import_source::ImportSource;
use crate::forge::stdlib;
use crate::generator::Language;
use crate::DocumentLabel;

/// Which pipeline generates a unit, and so which rule names its package.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnitKind {
    /// A statechart: its machine's package is named for the document's stem.
    Statechart,
    /// A forge kind: its package is named for the document's own name.
    Forge,
}

/// Why a unit is in the run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Role {
    /// The caller named it.
    Named,
    /// An `<sce:import>` of a document of the run names it.
    Imported,
    /// A hybrid `<invoke>` of a statechart of the run may start it: it is
    /// generated as a child into the package of the statechart `parent`
    /// names, which names its children as members of its own package.
    Candidate { parent: String },
}

/// One document the run generates.
#[derive(Debug, Clone)]
pub struct Unit {
    /// The document as the generator takes it: the caller's spelling for a
    /// document the caller named, the library name (`sce:std/...`) for a
    /// standard one, and otherwise the path its reference resolved to, with
    /// `.` and `..` folded.
    pub document: String,
    /// One spelling per document ([`stdlib::identity`]), whatever reference
    /// reached it.
    pub identity: String,
    /// The name its unit is generated under: a forge document's own name
    /// (`name=`, which an algorithm may set apart from its file), a
    /// statechart's stem.
    pub name: String,
    pub kind: UnitKind,
    pub role: Role,
    /// The references that brought it into the run, the document named on
    /// the command line first; empty for a document the caller named.
    pub reached_from: Vec<RelatedSite>,
}

impl Unit {
    /// Whether the document is the standard library's, whose source set is
    /// the library's rather than a directory's.
    pub fn is_standard(&self) -> bool {
        stdlib::names_standard(Path::new(&self.document))
    }

    /// Where the unit is generated, below the root every unit of the run
    /// shares: the package its importers' import lines name, or, for a child,
    /// its parent's package.
    pub fn directory(&self, lang: Language, kotlin_package_prefix: Option<&str>) -> PathBuf {
        match (&self.role, self.kind) {
            (Role::Candidate { parent }, _) => {
                crate::generator::machine_unit_dir(parent, lang, kotlin_package_prefix)
            }
            (Role::Named | Role::Imported, UnitKind::Statechart) => {
                crate::generator::machine_unit_dir(&self.name, lang, kotlin_package_prefix)
            }
            (Role::Named | Role::Imported, UnitKind::Forge) => {
                crate::forge::generator::forge_unit_dir(&self.name, lang)
            }
        }
    }
}

/// A walk that met a reference naming no document it could read.
#[derive(Debug)]
pub struct Refusal {
    /// The refusal, located where the reference is written, in the words the
    /// importing document's own generation would use.
    ///
    /// Boxed, as the crate keeps every returned error under clippy's
    /// `result_large_err` threshold (128 B, `forge::error::ForgeError`): a
    /// `Located<ForgeError>` already sits near it, and the chain beside it
    /// would carry the whole `Result` of every walk over it.
    pub error: Box<Located<ForgeError>>,
    /// The references that brought the document holding that reference into
    /// the run, the document named on the command line first, each a
    /// `reached-from` site (SCE_ERROR_CONTRACT §2.4). Empty when the
    /// reference is in a document the caller named.
    pub reached_from: Vec<RelatedSite>,
}

impl Refusal {
    /// The refusal as one record, the chain carried as its related sites.
    pub fn into_located(self) -> Located<ForgeError> {
        self.reached_from
            .into_iter()
            .fold(*self.error, |error, site| error.related_to(site))
    }
}

/// Where the walk reads what a document names, beside the document itself.
#[derive(Debug, Clone, Copy, Default)]
pub struct WalkOptions<'a> {
    /// The `-I` search path a document's `<xi:include>` and `<sce:use>`
    /// fragments are found on, as `generate` is given it: a reference the
    /// expansion brings in is read where the generation would read it.
    pub include_dirs: &'a [PathBuf],
    /// The `--input-root` a candidate is looked for under first, then beside
    /// its statechart — the order `generate` stages one in.
    pub input_root: Option<&'a Path>,
}

/// Every document `named` reaches, each once, in the order the walk reached
/// it: the named ones first, then each reference of each, breadth first.
///
/// A document named by the caller that a statechart of the run also declares
/// as a candidate is generated as that statechart's child only: generated as
/// a machine of its own as well, it would be a second copy nothing starts.
pub fn walk(named: &[String], options: WalkOptions<'_>) -> Result<Vec<Unit>, Refusal> {
    let mut pending: VecDeque<Pending> = named
        .iter()
        .map(|document| Pending {
            document: document.clone(),
            role: PendingRole::Named,
            reached_from: Vec::new(),
        })
        .collect();
    let mut seen: BTreeSet<(String, Option<String>)> = BTreeSet::new();
    let mut units: Vec<Unit> = Vec::new();
    while let Some(next) = pending.pop_front() {
        let identity = stdlib::identity(Path::new(&next.document));
        // A child is generated into its parent's package, so one document is
        // a unit once per package it is started in.
        let package = package_of(&next.role);
        if !seen.insert((identity.clone(), package.clone())) {
            continue;
        }
        let refused = |error: Located<ForgeError>| Refusal {
            error: Box::new(error),
            reached_from: next.reached_from.clone(),
        };
        let loaded = crate::load_forge_source(Path::new(&next.document), options.include_dirs)
            .map_err(refused)?;
        let text = loaded.text();
        let label = DocumentLabel::for_input_path(&next.document);
        let (kind, name, references) = match crate::classify_document(text) {
            crate::Pipeline::Forge => {
                let parsed = crate::forge::parser::parse_forge_with_imports(text, label)
                    .map_err(|error| refused(loaded.positions.authored(error)))?
                    .ok_or_else(|| {
                        refused(Located::new(
                            ValidationError::WrongPipeline {
                                kind: crate::forge::model::ForgeKind::Statechart,
                                pipeline: crate::Pipeline::Forge,
                            }
                            .into(),
                            label.diagnostic_label,
                            None,
                            None,
                        ))
                    })?;
                let references = parsed
                    .imports
                    .iter()
                    .map(|import| Reference {
                        src: import.src.clone(),
                        line: import.line,
                        by: By::Import,
                    })
                    .collect();
                (
                    UnitKind::Forge,
                    parsed.document.name().to_string(),
                    references,
                )
            }
            crate::Pipeline::Scxml => (
                UnitKind::Statechart,
                label.identifier.to_string(),
                statechart_references(text),
            ),
        };
        // The package this document's children are generated into: its own,
        // or, for a child, the one it was generated into itself.
        let children_package = package.unwrap_or_else(|| name.clone());
        let base = containing_dir(&next.document);
        for reference in references {
            let site = loaded.positions.authored_location(&SourceLocation {
                file: label.diagnostic_label.to_string(),
                line: reference.line,
                col: None,
            });
            let at_site = |error: ForgeError| {
                refused(Located::new(error, site.file.clone(), site.line, site.col))
            };
            let mut reached_from = next.reached_from.clone();
            reached_from.push(RelatedSite {
                role: RelatedRole::ReachedFrom,
                location: site.clone(),
                actual: None,
            });
            let (document, role) = match reference.by {
                By::Import => {
                    let source = ImportSource::read_src(&base, &reference.src)
                        .map_err(|error| at_site(error.into()))?;
                    // What the importer's own generation refuses, refused
                    // here first: a statechart is not a unit an import names.
                    if matches!(
                        crate::classify_document(&source.content),
                        crate::Pipeline::Scxml
                    ) {
                        return Err(at_site(
                            ImportError::NotForge {
                                src: reference.src.clone(),
                            }
                            .into(),
                        ));
                    }
                    (as_generated(&source.path), PendingRole::Imported)
                }
                By::StandardChild => {
                    let path = stdlib::resolve(&base, &reference.src);
                    if stdlib::lookup(&path).is_none() {
                        return Err(at_site(
                            XmlError::FileNotFound {
                                path: format!(
                                    "{} (the SCE standard library built into this sce-codegen)",
                                    path.display()
                                ),
                            }
                            .into(),
                        ));
                    }
                    (
                        as_generated(&path),
                        PendingRole::InLine {
                            parent: children_package.clone(),
                        },
                    )
                }
                By::Candidate => {
                    let path = candidate_document(&base, options.input_root, &reference.src)
                        .ok_or_else(|| {
                            at_site(
                                XmlError::FileNotFound {
                                    path: format!(
                                        "{} (an sce:candidates entry, looked for under the \
                                         input root and beside {})",
                                        reference.src, label.diagnostic_label
                                    ),
                                }
                                .into(),
                            )
                        })?;
                    (
                        as_generated(&path),
                        PendingRole::Candidate {
                            parent: children_package.clone(),
                        },
                    )
                }
            };
            pending.push_back(Pending {
                document,
                role,
                reached_from,
            });
        }
        let role = match next.role {
            PendingRole::Named => Role::Named,
            PendingRole::Imported => Role::Imported,
            PendingRole::Candidate { parent } => Role::Candidate { parent },
            // Generated by its parent's own run, in-line.
            PendingRole::InLine { .. } => continue,
        };
        units.push(Unit {
            document: next.document,
            identity,
            name,
            kind,
            role,
            reached_from: next.reached_from,
        });
    }
    let started: BTreeSet<String> = units
        .iter()
        .filter(|unit| matches!(unit.role, Role::Candidate { .. }))
        .map(|unit| unit.identity.clone())
        .collect();
    units.retain(|unit| {
        !(unit.role == Role::Named
            && unit.kind == UnitKind::Statechart
            && started.contains(&unit.identity))
    });
    Ok(units)
}

/// A document waiting to be read.
struct Pending {
    document: String,
    role: PendingRole,
    reached_from: Vec<RelatedSite>,
}

/// Why a document is waiting to be read.
enum PendingRole {
    Named,
    Imported,
    Candidate {
        parent: String,
    },
    /// A standard statechart a statechart of the run starts by its library
    /// name: generated in-line with its parent, and read here only for what
    /// it names in turn.
    InLine {
        parent: String,
    },
}

/// The package a document of `role` is generated into when it is not its
/// own: a child's parent's.
fn package_of(role: &PendingRole) -> Option<String> {
    match role {
        PendingRole::Candidate { parent } | PendingRole::InLine { parent } => Some(parent.clone()),
        PendingRole::Named | PendingRole::Imported => None,
    }
}

/// How a reference names a document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum By {
    /// `<sce:import src>`.
    Import,
    /// `<invoke src="sce:std/...">`.
    StandardChild,
    /// An entry of `<invoke sce:candidates>`.
    Candidate,
}

/// One reference of a document, with the row it is written on in the text the
/// walk read.
struct Reference {
    src: String,
    line: Option<u32>,
    by: By,
}

/// Every reference a statechart's text makes, in document order: each
/// `<sce:import>` (an in-line child's among them), each `<invoke>` that starts
/// a standard statechart by its library name, and each entry of each
/// `sce:candidates`.
///
/// Read from its elements, never from its text, as
/// [`stdlib::import_sources`] reads them. A text that is not well-formed
/// names nothing here; its own generation reports why.
fn statechart_references(text: &str) -> Vec<Reference> {
    let Ok(document) = roxmltree::Document::parse(text) else {
        return Vec::new();
    };
    let mut found = Vec::new();
    for node in document.descendants().filter(|node| node.is_element()) {
        let line = Some(document.text_pos_at(node.range().start).row);
        let tag = node.tag_name();
        if tag.name() == "import" && tag.namespace() == Some(crate::template::SCE_EXT_NS) {
            if let Some(src) = node.attribute("src") {
                found.push(Reference {
                    src: src.to_string(),
                    line,
                    by: By::Import,
                });
            }
        } else if tag.name() == "invoke" && tag.namespace() == Some(crate::model::SCXML_NAMESPACE) {
            if let Some(src) = node
                .attribute("src")
                .filter(|src| src.starts_with(stdlib::SCHEME))
            {
                found.push(Reference {
                    src: src.to_string(),
                    line,
                    by: By::StandardChild,
                });
            }
            if let Some(entries) = crate::sce_attr::read(&node, "candidates") {
                for entry in entries.split_whitespace() {
                    if let Some(candidate) = crate::model::InvokeCandidate::from_path(entry) {
                        found.push(Reference {
                            src: candidate.path,
                            line,
                            by: By::Candidate,
                        });
                    }
                }
            }
        }
    }
    found
}

/// The document a candidate entry names: under `input_root` first, then
/// beside its statechart, as `generate` stages one; a standard statechart's
/// candidate inside the library.
fn candidate_document(base: &Path, input_root: Option<&Path>, entry: &str) -> Option<PathBuf> {
    let beside = stdlib::resolve(base, entry);
    if stdlib::names_standard(&beside) {
        return stdlib::lookup(&beside).map(|_| beside);
    }
    input_root
        .map(|root| root.join(entry))
        .into_iter()
        .chain(std::iter::once(beside))
        .find(|path| crate::confine::exists(path))
}

/// The directory a document named as the generator takes it lives in: its
/// library directory for a standard one, and an empty path, which joins as the
/// working directory, for a bare file name.
fn containing_dir(document: &str) -> PathBuf {
    Path::new(document)
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_default()
}

/// A resolved reference spelled as the generator is handed it: a standard
/// document by its library name, folded ([`stdlib::identity`]); a file by its
/// path with `.` and `..` folded, so the provenance a generated file carries
/// does not spell the route an import took to it.
fn as_generated(path: &Path) -> String {
    if stdlib::names_standard(path) {
        return stdlib::identity(path);
    }
    lexically_folded(path).display().to_string()
}

/// `path` with every `.` dropped and every `..` folded into the directory
/// before it, where there is one: lexical, since a document a reference names
/// is spelled, not opened, here.
fn lexically_folded(path: &Path) -> PathBuf {
    let mut folded = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if matches!(folded.components().next_back(), Some(Component::Normal(_))) {
                    folded.pop();
                } else {
                    folded.push("..");
                }
            }
            Component::Normal(_) | Component::RootDir | Component::Prefix(_) => {
                folded.push(component.as_os_str());
            }
        }
    }
    folded
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    const SCXML_HEAD: &str =
        r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext""#;

    /// An algorithm document named `name` that imports `imports`, each
    /// `(src, alias)`.
    fn algorithm(name: &str, imports: &[(&str, &str)]) -> String {
        let lines: String = imports
            .iter()
            .map(|(src, alias)| {
                format!("  <sce:import kind=\"algorithm\" src=\"{src}\" as=\"{alias}\"/>\n")
            })
            .collect();
        format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n{SCXML_HEAD} sce:kind=\"algorithm\" \
             name=\"{name}\" version=\"1.0\">\n{lines}  <sce:signature>\n    <sce:param \
             name=\"n\" type=\"uint8\"/>\n    <sce:return type=\"bool\"/>\n  \
             </sce:signature>\n  <sce:body>\n    <sce:return expr=\"n % 2 === 0\"/>\n  \
             </sce:body>\n</scxml>\n"
        )
    }

    /// A statechart whose body is `body`, written after its root element.
    fn statechart(name: &str, body: &str) -> String {
        format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n{SCXML_HEAD} version=\"1.0\" \
             initial=\"idle\" name=\"{name}\">\n{body}  <state id=\"idle\"/>\n</scxml>\n"
        )
    }

    fn write(dir: &Path, file: &str, text: &str) -> String {
        let path = dir.join(file);
        fs::write(&path, text).expect("write a document");
        path.display().to_string()
    }

    fn walked(named: &[String]) -> Vec<Unit> {
        walk(named, WalkOptions::default())
            .unwrap_or_else(|refusal| panic!("the walk was refused: {}", refusal.error))
    }

    #[test]
    fn an_import_of_an_import_and_the_library_behind_it_are_reached_once() {
        let dir = tempfile::tempdir().expect("temp dir");
        write(
            dir.path(),
            "algorithm_inner.scxml",
            &algorithm("inner_even", &[]),
        );
        write(
            dir.path(),
            "algorithm_outer.scxml",
            &algorithm("outer_pick", &[("algorithm_inner.scxml", "inner")]),
        );
        let machine = write(
            dir.path(),
            "machine.scxml",
            &statechart(
                "machine",
                "  <sce:import kind=\"algorithm\" src=\"algorithm_outer.scxml\" as=\"Outer\"/>\n  \
                 <sce:import kind=\"algorithm\" src=\"algorithm_inner.scxml\" as=\"Inner\"/>\n  \
                 <sce:import kind=\"algorithm\" src=\"sce:std/time/days_from_civil.scxml\" \
                 as=\"Civil\"/>\n",
            ),
        );

        let units = walked(std::slice::from_ref(&machine));
        let names: Vec<(&str, &Role)> = units
            .iter()
            .map(|unit| (unit.name.as_str(), &unit.role))
            .collect();
        assert_eq!(
            names,
            [
                ("machine", &Role::Named),
                ("outer_pick", &Role::Imported),
                ("inner_even", &Role::Imported),
                ("days_from_civil", &Role::Imported),
                ("days_in_month", &Role::Imported),
                ("is_leap_year", &Role::Imported),
            ],
            "each document once, breadth first, the library's by its own imports"
        );
        let standard: Vec<&str> = units
            .iter()
            .filter(|unit| unit.is_standard())
            .map(|unit| unit.document.as_str())
            .collect();
        assert_eq!(
            standard,
            [
                "sce:std/time/days_from_civil.scxml",
                "sce:std/time/days_in_month.scxml",
                "sce:std/time/is_leap_year.scxml",
            ]
        );
        let inner = units
            .iter()
            .find(|unit| unit.name == "inner_even")
            .expect("the inner algorithm is reached");
        assert_eq!(inner.kind, UnitKind::Forge);
        assert_eq!(
            inner.reached_from.len(),
            1,
            "reached first from the machine, which imports it directly"
        );
        assert_eq!(inner.reached_from[0].location.file, machine);
        assert_eq!(inner.reached_from[0].role, RelatedRole::ReachedFrom);
    }

    #[test]
    fn a_missing_import_is_refused_where_it_is_written_with_the_chain_above_it() {
        let dir = tempfile::tempdir().expect("temp dir");
        let outer = write(
            dir.path(),
            "algorithm_outer.scxml",
            &algorithm("outer_pick", &[("algorithm_missing.scxml", "missing")]),
        );
        let machine = write(
            dir.path(),
            "machine.scxml",
            &statechart(
                "machine",
                "  <sce:import kind=\"algorithm\" src=\"algorithm_outer.scxml\" as=\"Outer\"/>\n",
            ),
        );

        let refusal = walk(std::slice::from_ref(&machine), WalkOptions::default())
            .expect_err("an import of a file that is not there is refused");
        assert!(
            matches!(
                refusal.error.error,
                ForgeError::Import(ImportError::FileNotFound { .. })
            ),
            "refused as the importer's own generation refuses it: {}",
            refusal.error
        );
        assert_eq!(refusal.error.location.file, outer);
        assert_eq!(
            refusal.error.location.line,
            Some(3),
            "at the <sce:import> that names it"
        );
        let chain: Vec<(&str, Option<u32>)> = refusal
            .reached_from
            .iter()
            .map(|site| (site.location.file.as_str(), site.location.line))
            .collect();
        assert_eq!(
            chain,
            [(machine.as_str(), Some(3))],
            "the machine's import of the algorithm that holds the broken one"
        );
        let located = refusal.into_located();
        assert_eq!(located.related().len(), 1, "the chain rides on the record");
    }

    #[test]
    fn a_candidate_is_generated_into_its_parents_package_and_only_there() {
        let dir = tempfile::tempdir().expect("temp dir");
        let child = write(dir.path(), "child.scxml", &statechart("child", ""));
        let parent = write(
            dir.path(),
            "parent.scxml",
            &statechart(
                "parent",
                "  <state id=\"running\">\n    <invoke srcexpr=\"which\" \
                 sce:candidates=\"child.scxml\"/>\n  </state>\n",
            ),
        );

        let units = walked(&[parent, child]);
        let roles: Vec<(&str, &Role)> = units
            .iter()
            .map(|unit| (unit.name.as_str(), &unit.role))
            .collect();
        assert_eq!(
            roles,
            [
                ("parent", &Role::Named),
                (
                    "child",
                    &Role::Candidate {
                        parent: "parent".to_string()
                    }
                ),
            ],
            "named as well, the candidate is generated as the parent's child alone"
        );
        let candidate = &units[1];
        assert_eq!(
            candidate.directory(Language::Kotlin, Some("com.acme.app")),
            Path::new("com/acme/app/parent"),
            "in the parent's package"
        );
        assert_eq!(candidate.directory(Language::Go, None), Path::new("parent"));
    }

    #[test]
    fn a_unit_is_placed_in_the_package_its_import_line_names() {
        let forge = Unit {
            document: "x.scxml".to_string(),
            identity: "x".to_string(),
            name: "OuterPick".to_string(),
            kind: UnitKind::Forge,
            role: Role::Imported,
            reached_from: Vec::new(),
        };
        assert_eq!(
            forge.directory(Language::Kotlin, Some("com.acme.app")),
            Path::new("com/sce/generated/outer_pick"),
            "a forge unit's Kotlin package does not follow the machine's prefix"
        );
        assert_eq!(forge.directory(Language::Go, None), Path::new("outer_pick"));
        assert_eq!(forge.directory(Language::Rust, None), Path::new(""));
        let machine = Unit {
            name: "door".to_string(),
            kind: UnitKind::Statechart,
            role: Role::Named,
            ..forge
        };
        assert_eq!(
            machine.directory(Language::Kotlin, None),
            Path::new("com/sce/generated/door")
        );
        assert_eq!(machine.directory(Language::Python, None), Path::new(""));
    }

    #[test]
    fn a_folded_path_keeps_a_parent_it_cannot_fold() {
        assert_eq!(
            lexically_folded(Path::new("a/./b/../c.scxml")),
            Path::new("a/c.scxml")
        );
        assert_eq!(
            lexically_folded(Path::new("../shared/x.scxml")),
            Path::new("../shared/x.scxml")
        );
        assert_eq!(
            lexically_folded(Path::new("/p/a/../b.scxml")),
            Path::new("/p/b.scxml")
        );
    }
}

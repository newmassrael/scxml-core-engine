// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What a materialized tree was made from (ADR 0097, section 7):
//! `generated-provenance.json`, the record a package that carries generated
//! code ships beside it, so that anyone with the commit can rebuild the tree
//! and compare digests.
//!
//! For each tree it holds the generator that made it (the digest of its
//! source closure, the compiler and how it was built, as an action key names
//! it), every action by its key, the Merkle roots of what the action read and
//! wrote, the digest of every file a generation read (the templates and the
//! documents' closures among them), and the Merkle root of the tree as it
//! was written. Every digest is a REAPI `Digest` written `<sha256 hex>/<size>`,
//! and every root is the digest of a REAPI `Directory`, so a tree's root is
//! the value a REAPI server computes for the same files.
//!
//! One file holds the trees of one package. A materialization records its
//! tree in it beside the trees recorded before, replacing an earlier record
//! of the same tree; the trees of one file come from one generator, since a
//! package made by two is the release of no one commit.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::identity::{GeneratorIdentity, GENERATOR_FEATURES, GENERATOR_PROFILE};
use super::input_root::{input_root, InputFile};
use super::materialize::{ActionKind, Report};
use super::reapi::{Digest, DIGEST_FUNCTION};

/// The name of the file a package carries.
pub const PROVENANCE_FILE: &str = "generated-provenance.json";

/// The schema of the file: its name and the version of its shape.
pub const SCHEMA: &str = "sce.generated-provenance/1";

/// A provenance file: the generator and the trees it made.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GeneratedProvenance {
    /// [`SCHEMA`].
    pub schema: String,
    /// The function of every digest of the file, [`DIGEST_FUNCTION`].
    pub digest_function: String,
    pub generator: GeneratorRecord,
    /// The trees, by name, then by script engine.
    pub trees: Vec<TreeProvenance>,
}

/// The generator of every action of a file, as an action key names it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GeneratorRecord {
    /// The digest of the generator's source closure.
    pub source: String,
    /// The compiler that builds it: release, commit and host.
    pub rustc: String,
    /// The operating system it ran on.
    pub os_family: String,
    /// The features it is built with.
    pub features: String,
    /// The profile it is built in.
    pub profile: String,
}

impl GeneratorRecord {
    /// The record of `identity`.
    pub fn of(identity: &GeneratorIdentity) -> GeneratorRecord {
        GeneratorRecord {
            source: identity.source_digest.clone(),
            rustc: identity.rustc.clone(),
            os_family: identity.os_family.clone(),
            features: GENERATOR_FEATURES.to_string(),
            profile: GENERATOR_PROFILE.to_string(),
        }
    }
}

/// One tree as it was materialized.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TreeProvenance {
    /// The tree, as its manifest names it.
    pub tree: String,
    /// The script engine the tree was written for, when it was given one
    /// (`materialize --script-engine`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub script_engine: Option<String>,
    /// The Merkle root of every file the materialization wrote, by its path
    /// under the directory it was written into.
    pub output_tree: String,
    /// How many files that root holds.
    pub files: usize,
    /// Every action the tree was made by.
    pub actions: Vec<ActionProvenance>,
    /// The generations whose member the backend declined by name.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub declined: Vec<String>,
    /// The digest of every file a generation read, by its path under the
    /// checkout's root.
    pub inputs: BTreeMap<String, String>,
}

/// One action of a tree.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ActionProvenance {
    /// A generator invocation.
    Generation {
        /// Its arguments as the manifest declares them.
        arguments: Vec<String>,
        key: String,
        input_root: String,
        output_root: String,
    },
    /// A format pass.
    Format {
        tool: String,
        /// The directory it formatted, under the directory the tree was
        /// written into.
        directory: String,
        key: String,
        input_root: String,
        output_root: String,
    },
}

/// Why a provenance file was not read or written.
#[derive(Debug, thiserror::Error)]
pub enum ProvenanceError {
    #[error("{path}: {source}")]
    Io { path: PathBuf, source: io::Error },
    #[error("{path} is not a provenance file: {detail}")]
    Malformed { path: PathBuf, detail: String },
    #[error("{path} is of the schema `{found}`, not `{SCHEMA}`")]
    Schema { path: PathBuf, found: String },
    #[error(
        "{path} records trees of the generator {recorded}, not of {this}: the trees of one \
         package are made by one generator"
    )]
    OtherGenerator {
        path: PathBuf,
        recorded: String,
        this: String,
    },
    #[error("tree `{tree}`: {detail}")]
    Tree { tree: String, detail: String },
}

/// The Merkle root of the files `files` names under `into`, as they are on
/// disk now, each with its executable bit.
pub fn output_tree(into: &Path, files: &BTreeSet<String>) -> io::Result<Digest> {
    let mut held = BTreeMap::new();
    for file in files {
        let path = into.join(file);
        held.insert(
            file.clone(),
            InputFile {
                digest: Digest::of(&fs::read(&path)?),
                is_executable: is_executable(&path)?,
            },
        );
    }
    input_root(&held)
        .map(|root| root.digest)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))
}

/// The provenance of the tree `tree`, written into `into` by the
/// materialization `report` describes.
pub fn tree_provenance(
    tree: &str,
    script_engine: Option<&str>,
    report: &Report,
    into: &Path,
) -> Result<TreeProvenance, ProvenanceError> {
    let output = output_tree(into, &report.files).map_err(|e| ProvenanceError::Tree {
        tree: tree.to_string(),
        detail: e.to_string(),
    })?;
    let actions = report
        .actions
        .iter()
        .map(|action| {
            let (key, input_root, output_root) = (
                action.key.to_string(),
                action.input_root.to_string(),
                action.output_root.to_string(),
            );
            match &action.kind {
                ActionKind::Generation { arguments } => ActionProvenance::Generation {
                    arguments: arguments.clone(),
                    key,
                    input_root,
                    output_root,
                },
                ActionKind::Format { tool, directory } => ActionProvenance::Format {
                    tool: (*tool).to_string(),
                    directory: directory.clone(),
                    key,
                    input_root,
                    output_root,
                },
            }
        })
        .collect();
    Ok(TreeProvenance {
        tree: tree.to_string(),
        script_engine: script_engine.map(str::to_string),
        output_tree: output.to_string(),
        files: report.files.len(),
        actions,
        declined: report.declined.clone(),
        inputs: report
            .inputs
            .iter()
            .map(|(path, digest)| (path.clone(), digest.to_string()))
            .collect(),
    })
}

impl GeneratedProvenance {
    /// A file of `generator` that records no tree yet.
    pub fn new(generator: GeneratorRecord) -> GeneratedProvenance {
        GeneratedProvenance {
            schema: SCHEMA.to_string(),
            digest_function: DIGEST_FUNCTION.to_string(),
            generator,
            trees: Vec::new(),
        }
    }

    /// Record `tree`, in place of an earlier record of the same tree for the
    /// same script engine, keeping the trees in order.
    pub fn record(&mut self, tree: TreeProvenance) {
        self.trees
            .retain(|held| (&held.tree, &held.script_engine) != (&tree.tree, &tree.script_engine));
        self.trees.push(tree);
        self.trees
            .sort_by(|a, b| (&a.tree, &a.script_engine).cmp(&(&b.tree, &b.script_engine)));
    }

    /// The text of the file: pretty JSON, a newline at its end.
    pub fn render(&self) -> String {
        let mut text =
            serde_json::to_string_pretty(self).expect("a provenance record is plain data");
        text.push('\n');
        text
    }

    /// The file at `path`, refused when it is not one of [`SCHEMA`].
    pub fn read(path: &Path) -> Result<GeneratedProvenance, ProvenanceError> {
        let text = fs::read_to_string(path).map_err(|source| ProvenanceError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        GeneratedProvenance::parse(&text, path)
    }

    fn parse(text: &str, path: &Path) -> Result<GeneratedProvenance, ProvenanceError> {
        // The schema is read first and alone, so that a file of another
        // version is named as such rather than as a shape this one refuses.
        let value: serde_json::Value =
            serde_json::from_str(text).map_err(|e| ProvenanceError::Malformed {
                path: path.to_path_buf(),
                detail: e.to_string(),
            })?;
        let found = value
            .get("schema")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();
        if found != SCHEMA {
            return Err(ProvenanceError::Schema {
                path: path.to_path_buf(),
                found: found.to_string(),
            });
        }
        serde_json::from_value(value).map_err(|e| ProvenanceError::Malformed {
            path: path.to_path_buf(),
            detail: e.to_string(),
        })
    }
}

/// Record `tree`, made by `generator`, in the provenance file at `path`:
/// beside the trees the file records already, or in a new file when there is
/// none ([`write`]). Refused when the file records trees of another
/// generator.
pub fn record(
    path: &Path,
    generator: GeneratorRecord,
    tree: TreeProvenance,
) -> Result<(), ProvenanceError> {
    let mut provenance = if path.exists() {
        let held = GeneratedProvenance::read(path)?;
        if held.generator != generator {
            return Err(ProvenanceError::OtherGenerator {
                path: path.to_path_buf(),
                recorded: held.generator.source,
                this: generator.source,
            });
        }
        held
    } else {
        GeneratedProvenance::new(generator)
    };
    provenance.record(tree);
    write(path, &provenance)
}

/// Write `provenance` to `path` whole, so that a reader sees the old file or
/// the new one, never a part of either.
pub fn write(path: &Path, provenance: &GeneratedProvenance) -> Result<(), ProvenanceError> {
    let io_error = |source| ProvenanceError::Io {
        path: path.to_path_buf(),
        source,
    };
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent).map_err(io_error)?;
    }
    let staged = path.with_extension(format!("json.{}.partial", std::process::id()));
    let written = fs::write(&staged, provenance.render()).and_then(|()| fs::rename(&staged, path));
    if written.is_err() {
        let _ = fs::remove_file(&staged);
    }
    written.map_err(io_error)
}

#[cfg(unix)]
fn is_executable(path: &Path) -> io::Result<bool> {
    use std::os::unix::fs::PermissionsExt;
    Ok(fs::metadata(path)?.permissions().mode() & 0o111 != 0)
}

#[cfg(not(unix))]
fn is_executable(_path: &Path) -> io::Result<bool> {
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::super::materialize::ActionRecord;
    use super::*;

    fn generator(source: &str) -> GeneratorRecord {
        GeneratorRecord::of(&GeneratorIdentity {
            source_digest: source.to_string(),
            rustc: "1.91.0 abc x86_64-unknown-linux-gnu".to_string(),
            os_family: "linux".to_string(),
        })
    }

    /// A materialization of two files, one generation and one format pass,
    /// written into `into`.
    fn materialized(into: &Path) -> Report {
        fs::create_dir_all(into.join("gen")).unwrap();
        fs::write(into.join("gen/a.rs"), b"fn a() {}\n").unwrap();
        fs::write(into.join("gen/mod.rs"), b"pub mod a;\n").unwrap();
        Report {
            files: ["gen/a.rs", "gen/mod.rs"]
                .iter()
                .map(|f| f.to_string())
                .collect(),
            inputs: [
                ("t/rust/sm.jinja2".to_string(), Digest::of(b"template")),
                ("x/a.scxml".to_string(), Digest::of(b"<scxml/>")),
            ]
            .into_iter()
            .collect(),
            actions: vec![
                ActionRecord {
                    kind: ActionKind::Generation {
                        arguments: vec!["generate".into(), "x/a.scxml".into(), "{out}".into()],
                    },
                    key: Digest::of(b"key-1"),
                    input_root: Digest::of(b"in-1"),
                    output_root: Digest::of(b"out-1"),
                },
                ActionRecord {
                    kind: ActionKind::Format {
                        tool: "rustfmt",
                        directory: "gen".into(),
                    },
                    key: Digest::of(b"key-2"),
                    input_root: Digest::of(b"in-2"),
                    output_root: Digest::of(b"out-2"),
                },
            ],
            declined: vec!["generate x/b.scxml {out}".to_string()],
            ..Report::default()
        }
    }

    /// A tree's record names every action by its key and roots, every input
    /// by its digest, and the root of the files as they were written.
    #[test]
    fn a_tree_is_recorded_by_its_actions_inputs_and_output_root() {
        let into = tempfile::tempdir().unwrap();
        let report = materialized(into.path());
        let tree = tree_provenance("rust-w3c", None, &report, into.path()).unwrap();

        let files: BTreeMap<String, InputFile> = [
            ("gen/a.rs", &b"fn a() {}\n"[..]),
            ("gen/mod.rs", &b"pub mod a;\n"[..]),
        ]
        .iter()
        .map(|(p, b)| {
            (
                p.to_string(),
                InputFile {
                    digest: Digest::of(b),
                    is_executable: false,
                },
            )
        })
        .collect();
        assert_eq!(
            tree.output_tree,
            input_root(&files).unwrap().digest.to_string()
        );
        assert_eq!(tree.files, 2);
        assert_eq!(
            tree.actions,
            vec![
                ActionProvenance::Generation {
                    arguments: vec!["generate".into(), "x/a.scxml".into(), "{out}".into()],
                    key: Digest::of(b"key-1").to_string(),
                    input_root: Digest::of(b"in-1").to_string(),
                    output_root: Digest::of(b"out-1").to_string(),
                },
                ActionProvenance::Format {
                    tool: "rustfmt".into(),
                    directory: "gen".into(),
                    key: Digest::of(b"key-2").to_string(),
                    input_root: Digest::of(b"in-2").to_string(),
                    output_root: Digest::of(b"out-2").to_string(),
                },
            ]
        );
        assert_eq!(
            tree.inputs.get("x/a.scxml").map(String::as_str),
            Some(Digest::of(b"<scxml/>").to_string().as_str())
        );
        assert_eq!(tree.declined, ["generate x/b.scxml {out}"]);
    }

    /// The output root is of the bytes on disk: a file changed after the
    /// generation (a format pass) moves it.
    #[test]
    fn the_output_root_is_of_the_files_as_written() {
        let into = tempfile::tempdir().unwrap();
        let report = materialized(into.path());
        let before = output_tree(into.path(), &report.files).unwrap();
        fs::write(into.path().join("gen/a.rs"), b"fn a() {}\n\n").unwrap();
        let after = output_tree(into.path(), &report.files).unwrap();
        assert_ne!(before, after);
    }

    /// A file records each tree once, in order, a second materialization of
    /// a tree replacing the first, and reads back as it was written.
    #[test]
    fn a_file_records_each_tree_once_and_reads_back() {
        let into = tempfile::tempdir().unwrap();
        let report = materialized(into.path());
        let file = into.path().join("pkg").join(PROVENANCE_FILE);
        let tree = |name: &str, engine: Option<&str>| {
            tree_provenance(name, engine, &report, into.path()).unwrap()
        };
        record(&file, generator("s"), tree("mesh-rust", None)).unwrap();
        record(&file, generator("s"), tree("kotlin-w3c", None)).unwrap();
        record(&file, generator("s"), tree("kotlin-w3c", Some("lua"))).unwrap();
        let mut again = tree("mesh-rust", None);
        again.files = 7;
        record(&file, generator("s"), again).unwrap();

        let read = GeneratedProvenance::read(&file).unwrap();
        assert_eq!(read.schema, SCHEMA);
        assert_eq!(read.digest_function, "sha256");
        let names: Vec<(&str, Option<&str>)> = read
            .trees
            .iter()
            .map(|t| (t.tree.as_str(), t.script_engine.as_deref()))
            .collect();
        assert_eq!(
            names,
            [
                ("kotlin-w3c", None),
                ("kotlin-w3c", Some("lua")),
                ("mesh-rust", None)
            ]
        );
        assert_eq!(
            read.trees[2].files, 7,
            "the second record did not replace the first"
        );
        assert_eq!(fs::read_to_string(&file).unwrap(), read.render());
    }

    /// The trees of one file come from one generator.
    #[test]
    fn a_tree_of_another_generator_is_refused() {
        let into = tempfile::tempdir().unwrap();
        let report = materialized(into.path());
        let file = into.path().join(PROVENANCE_FILE);
        let tree = tree_provenance("mesh-rust", None, &report, into.path()).unwrap();
        record(&file, generator("s"), tree.clone()).unwrap();
        assert!(matches!(
            record(&file, generator("other"), tree),
            Err(ProvenanceError::OtherGenerator { .. })
        ));
    }

    /// A file of another schema is named as such, not read as this one.
    #[test]
    fn a_file_of_another_schema_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join(PROVENANCE_FILE);
        fs::write(&file, "{\"schema\": \"sce.generated-provenance/2\"}").unwrap();
        assert!(matches!(
            GeneratedProvenance::read(&file),
            Err(ProvenanceError::Schema { found, .. }) if found == "sce.generated-provenance/2"
        ));
        fs::write(&file, "not json").unwrap();
        assert!(matches!(
            GeneratedProvenance::read(&file),
            Err(ProvenanceError::Malformed { .. })
        ));
    }
}

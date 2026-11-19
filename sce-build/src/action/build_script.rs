// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A package's generated trees, written into its `OUT_DIR` by its build
//! script (ADR 0097, section 4).
//!
//! # The build script is its own generator
//!
//! Cargo builds no binary of another package for a build script, and a
//! nested `cargo build` would wait on the lock of the build that runs it. So
//! the build script links this library, which carries every command of
//! `sce-codegen` ([`crate::cli`]), and is the program each generation runs:
//! the action is run as REAPI runs one and as [`Generator`] runs every
//! generation, a command in the checkout with the environment the action
//! declares and nothing else, in a process of its own. Its read set, its exit
//! status and its output are those of `sce-codegen` run on the same
//! arguments, since the same code reads the same arguments.
//!
//! A run is told apart from the build by its first argument, [`READS_FLAG`]:
//! the generator is always started with it, and Cargo starts a build script
//! with none.
//!
//! # What the build watches
//!
//! A build script runs again when a file it names changes. It names the
//! manifests of its trees, every file a generation read, the nearest
//! directory of every file a generation looked for and did not find, and the
//! directories whose listing decided a tree's members
//! ([`ExpandedTree::listed`]), so that a document added to a set reaches the
//! build that compiles the set.
//!
//! # How the crate declares what was placed
//!
//! `#[path]` cannot take `env!("OUT_DIR")`, so the build script writes the
//! declarations of the modules it placed (`#[path = "<absolute path>"] pub mod
//! <name>;`) into `OUT_DIR` ([`write_module_declarations`]), and the crate
//! `include!`s them where it would declare the modules by hand. The modules
//! are what was placed ([`Materialized::modules_in`]), not what a directory
//! holds, so a file the tree no longer writes is no module of the crate.

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsStr;
use std::io;
use std::path::{Path, PathBuf};

use super::generation::{GenerationError, Generator, READS_FLAG};
use super::identity::{GeneratorIdentity, IdentityError};
use super::materialize::{materialize, remove_unproduced, MaterializeError};
use super::provenance::{
    self, tree_provenance, GeneratedProvenance, GeneratorRecord, ProvenanceError, PROVENANCE_FILE,
};
use super::store::LocalStore;
use super::tree::{self, ExpandedTree, ManifestError, MANIFEST_DIRECTORY};

/// The directory of `OUT_DIR` the trees are written into, each file at the
/// path it has under the checkout's root.
pub const TREES_DIRECTORY: &str = "sce-trees";

/// What a build script materialized.
#[derive(Clone, Debug)]
pub struct Materialized {
    /// The directory the trees were written into.
    pub into: PathBuf,
    /// Every file written, relative to [`Materialized::into`].
    pub files: BTreeSet<String>,
}

/// A module a crate declares over a file a build script materialized.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModuleDeclaration {
    /// The module's name.
    pub name: String,
    /// The file the module is read from, absolute.
    pub path: PathBuf,
}

impl Materialized {
    /// The modules of what the trees placed directly in `dir`, a directory
    /// named by its path under the checkout's root, in the order of their
    /// names: a file `<name>.rs` is the module `name`, and a directory that
    /// holds a placed `mod.rs` is the module of its name, read from that
    /// file. The `mod.rs` of `dir` itself is no module of it, and neither is
    /// a file of any other kind or a directory without a placed `mod.rs`.
    pub fn modules_in(&self, dir: &str) -> Vec<ModuleDeclaration> {
        let prefix = format!("{}/", dir.trim_end_matches('/'));
        let mut modules = BTreeMap::new();
        for file in &self.files {
            let Some(rest) = file.strip_prefix(&prefix) else {
                continue;
            };
            let name = match rest.split_once('/') {
                Some((directory, "mod.rs")) => directory,
                Some(_) => continue,
                None => match rest.strip_suffix(".rs") {
                    Some("mod") | None => continue,
                    Some(stem) => stem,
                },
            };
            modules.insert(name.to_string(), self.into.join(file));
        }
        modules
            .into_iter()
            .map(|(name, path)| ModuleDeclaration { name, path })
            .collect()
    }
}

/// Why a build script wrote no trees.
#[derive(Debug, thiserror::Error)]
pub enum BuildScriptError {
    #[error("{0} is not set: the trees are materialized by a build script, which Cargo gives it")]
    NotABuildScript(&'static str),
    #[error(
        "no checkout above {0} declares its trees in {MANIFEST_DIRECTORY}: a package that \
         materializes trees builds from the checkout that declares them"
    )]
    NoCheckout(PathBuf),
    #[error("no action cache: none of SCE_ACTION_CACHE, XDG_CACHE_HOME and HOME is set")]
    NoStore,
    #[error("the build script cannot name its own executable, which is the generator: {0}")]
    NoExecutable(io::Error),
    #[error("{0} is not a path of text, so no `#[path]` attribute can name it")]
    NotText(PathBuf),
    #[error("writing {path}: {source}")]
    Write { path: PathBuf, source: io::Error },
    #[error("{0}")]
    Identity(#[from] IdentityError),
    #[error("{0}")]
    Generation(#[from] GenerationError),
    #[error("{0}")]
    Manifest(#[from] ManifestError),
    #[error("{0}")]
    Materialize(#[from] MaterializeError),
    #[error("{0}")]
    Provenance(#[from] ProvenanceError),
}

/// When this process was started as the generator of a run, run that
/// command and end the process with its status; otherwise return.
///
/// [`materialize_trees`] calls this first. A build script that does work
/// before it materializes calls it at the top of `main`, so that a run does
/// not repeat that work.
pub fn serve_generator_runs() {
    if std::env::args_os().nth(1).as_deref() == Some(OsStr::new(READS_FLAG)) {
        crate::cli::main();
        std::process::exit(0);
    }
}

/// Write the trees named `trees` into `OUT_DIR` from their actions, through
/// this machine's action cache, and tell Cargo what to watch.
///
/// Each tree is written at the paths it has under the checkout's root,
/// beneath `OUT_DIR/`[`TREES_DIRECTORY`]; a file an earlier build wrote that
/// the tree no longer produces is removed. Nothing is formatted but by the
/// tree's own `[[format]]` rules. What the trees were made from is recorded
/// beside them, in [`PROVENANCE_FILE`] of that directory.
pub fn materialize_trees(trees: &[&str]) -> Result<Materialized, BuildScriptError> {
    serve_generator_runs();
    let manifest_dir = build_variable("CARGO_MANIFEST_DIR")?;
    let out_dir = build_variable("OUT_DIR")?;
    let root = checkout_of(&manifest_dir).ok_or(BuildScriptError::NoCheckout(manifest_dir))?;
    let identity = GeneratorIdentity::of_checkout(&root)?;
    let binary = std::env::current_exe().map_err(BuildScriptError::NoExecutable)?;
    let generator = Generator::new(&root, &binary, crate::GENERATOR_SOURCE_DIGEST, identity)?;
    let store = LocalStore::for_this_machine().ok_or(BuildScriptError::NoStore)?;
    // The checkout as the generator resolved it, so that every watched path
    // is spelled under the same root as the files the runs read.
    let root = generator.root().to_path_buf();
    let into = out_dir.join(TREES_DIRECTORY);

    let mut files = BTreeSet::new();
    let mut watched = BTreeSet::new();
    let mut made = GeneratedProvenance::new(GeneratorRecord::of(generator.identity()));
    for name in trees {
        watched.insert(root.join(MANIFEST_DIRECTORY).join(format!("{name}.toml")));
        let expanded = tree::tree(&root, name)?.expand(&root)?;
        let report = materialize(&generator, &store, &expanded, &into)?;
        remove_unproduced(&expanded, &into, &report)?;
        made.record(tree_provenance(name, None, &report, &into)?);
        watched.extend(watched_paths(
            &root,
            &expanded,
            &report.reads,
            &report.absent,
        ));
        files.extend(report.files);
    }
    // The trees of this build and no other: written whole, so a tree the
    // package no longer compiles leaves the record with it.
    provenance::write(&into.join(PROVENANCE_FILE), &made)?;
    for path in &watched {
        println!("cargo:rerun-if-changed={}", path.display());
    }
    Ok(Materialized { into, files })
}

/// Write the declarations of `modules` into `OUT_DIR/<file>`, for the crate
/// to `include!` where it would declare the modules by hand, and return the
/// file written. Each is `#[path = "<absolute path>"] pub mod <name>;`, since
/// `#[path]` cannot take `env!("OUT_DIR")`.
pub fn write_module_declarations(
    file: &str,
    modules: &[ModuleDeclaration],
) -> Result<PathBuf, BuildScriptError> {
    let path = build_variable("OUT_DIR")?.join(file);
    let text = module_declarations(modules)?;
    std::fs::write(&path, text).map_err(|source| BuildScriptError::Write {
        path: path.clone(),
        source,
    })?;
    Ok(path)
}

/// The text of the declarations of `modules`, one module each, in the order
/// given. A path is written as a Rust string literal, its escapes those of
/// `Debug`, which spells every character a literal can hold.
fn module_declarations(modules: &[ModuleDeclaration]) -> Result<String, BuildScriptError> {
    let mut text = String::from("// GENERATED -- DO NOT EDIT (sce_build::action::build_script)\n");
    for module in modules {
        let path = module
            .path
            .to_str()
            .ok_or_else(|| BuildScriptError::NotText(module.path.clone()))?;
        text.push_str(&format!("#[path = {path:?}]\npub mod {};\n", module.name));
    }
    Ok(text)
}

/// The value of the variable Cargo gives a build script, as a path.
fn build_variable(name: &'static str) -> Result<PathBuf, BuildScriptError> {
    std::env::var_os(name)
        .map(PathBuf::from)
        .ok_or(BuildScriptError::NotABuildScript(name))
}

/// The nearest directory at or above `dir` that declares trees: the checkout
/// the package belongs to.
fn checkout_of(dir: &Path) -> Option<PathBuf> {
    dir.ancestors()
        .find(|candidate| candidate.join(MANIFEST_DIRECTORY).is_dir())
        .map(Path::to_path_buf)
}

/// What a build of `tree` watches, as paths under `root`: each file its
/// generations read; for each file one looked for and did not find, the
/// nearest directory above it that exists; and each directory its expansion
/// listed. The root itself is never watched, since Cargo would read a watched
/// directory whole, the build's own output among it.
fn watched_paths(
    root: &Path,
    tree: &ExpandedTree,
    reads: &BTreeSet<String>,
    absent: &BTreeSet<String>,
) -> BTreeSet<PathBuf> {
    let mut watched: BTreeSet<PathBuf> = reads.iter().map(|path| root.join(path)).collect();
    for path in absent {
        let nearest = Path::new(path)
            .ancestors()
            .skip(1)
            .find(|dir| !dir.as_os_str().is_empty() && root.join(dir).is_dir());
        if let Some(dir) = nearest {
            watched.insert(root.join(dir));
        }
    }
    watched.extend(
        tree.listed
            .iter()
            .filter(|path| !path.is_empty())
            .map(|path| root.join(path)),
    );
    watched
}

#[cfg(test)]
mod tests {
    use super::*;

    fn expanded(listed: &[&str]) -> ExpandedTree {
        ExpandedTree {
            name: "t".to_string(),
            roots: Vec::new(),
            except: Vec::new(),
            committed: false,
            generations: Vec::new(),
            indexes: Vec::new(),
            formats: Vec::new(),
            listed: listed.iter().map(|p| p.to_string()).collect(),
        }
    }

    /// A build watches each file read, the nearest existing directory of a
    /// file found absent, and each listed directory, and never the root.
    #[test]
    fn a_build_watches_what_its_generations_read_and_what_decided_its_members() {
        let root = tempfile::tempdir().expect("a checkout");
        std::fs::create_dir_all(root.path().join("docs/a")).expect("a directory");
        let reads: BTreeSet<String> = ["docs/a/x.scxml".to_string()].into_iter().collect();
        let absent: BTreeSet<String> = [
            "docs/a/deploy.yaml".to_string(),
            "docs/b/c/deploy.yaml".to_string(),
            "nowhere.yaml".to_string(),
        ]
        .into_iter()
        .collect();
        let watched = watched_paths(root.path(), &expanded(&["docs", ""]), &reads, &absent);
        let expected: BTreeSet<PathBuf> = ["docs/a/x.scxml", "docs/a", "docs"]
            .iter()
            .map(|p| root.path().join(p))
            .collect();
        assert_eq!(watched, expected);
    }

    fn placed(files: &[&str]) -> Materialized {
        Materialized {
            into: PathBuf::from("/out/sce-trees"),
            files: files.iter().map(|f| f.to_string()).collect(),
        }
    }

    /// The modules of a directory are its placed `.rs` files and its
    /// directories with a placed `mod.rs`, by name; its own `mod.rs`, a file
    /// of another kind, a file deeper down, a directory without one and a
    /// file beside it whose name it begins are none of them.
    #[test]
    fn the_modules_of_a_directory_are_what_was_placed_in_it() {
        let materialized = placed(&[
            "crate/src/generated/mod.rs",
            "crate/src/generated/test144/mod.rs",
            "crate/src/generated/test144/test144_sm.rs",
            "crate/src/generated/test144/sce_sourcemap.json",
            "crate/src/integration/b_stem/mod.rs",
            "crate/src/integration/a_stem/mod.rs",
            "crate/src/integration/a_stem/a_stem_sm.rs",
            "crate/src/integration/no_index/x_sm.rs",
            "crate/tests_more.rs",
            "crate/tests/test_150.rs",
            "crate/tests/test_144.rs",
            "crate/tests/notes.txt",
        ]);
        let named = |dir: &str| -> Vec<(String, PathBuf)> {
            materialized
                .modules_in(dir)
                .into_iter()
                .map(|m| (m.name, m.path))
                .collect()
        };
        let at = |file: &str| Path::new("/out/sce-trees").join(file);
        assert_eq!(
            named("crate/src"),
            [("generated".to_string(), at("crate/src/generated/mod.rs"))]
        );
        assert_eq!(
            named("crate/src/generated"),
            [(
                "test144".to_string(),
                at("crate/src/generated/test144/mod.rs")
            )]
        );
        assert_eq!(
            named("crate/src/integration/"),
            [
                (
                    "a_stem".to_string(),
                    at("crate/src/integration/a_stem/mod.rs")
                ),
                (
                    "b_stem".to_string(),
                    at("crate/src/integration/b_stem/mod.rs")
                ),
            ]
        );
        assert_eq!(
            named("crate/tests"),
            [
                ("test_144".to_string(), at("crate/tests/test_144.rs")),
                ("test_150".to_string(), at("crate/tests/test_150.rs")),
            ]
        );
        assert!(named("crate/missing").is_empty());
    }

    /// A declaration names its module and the file it is read from, the path
    /// written as a literal that holds it whatever characters it has.
    #[test]
    fn a_declaration_names_its_module_at_the_file_it_was_placed_at() {
        let text = module_declarations(&[
            ModuleDeclaration {
                name: "test144".to_string(),
                path: PathBuf::from("/out/sce-trees/generated/mod.rs"),
            },
            ModuleDeclaration {
                name: "odd".to_string(),
                path: PathBuf::from("/out/a \"quoted\" dir\\x/mod.rs"),
            },
        ])
        .expect("the declarations are written");
        assert_eq!(
            text,
            "// GENERATED -- DO NOT EDIT (sce_build::action::build_script)\n\
             #[path = \"/out/sce-trees/generated/mod.rs\"]\npub mod test144;\n\
             #[path = \"/out/a \\\"quoted\\\" dir\\\\x/mod.rs\"]\npub mod odd;\n"
        );
    }

    /// The checkout of a package is the nearest directory above it that
    /// declares trees.
    #[test]
    fn the_checkout_of_a_package_is_the_nearest_that_declares_trees() {
        let root = tempfile::tempdir().expect("a checkout");
        std::fs::create_dir_all(root.path().join(MANIFEST_DIRECTORY)).expect("the manifests");
        let package = root.path().join("backends/rust/tests");
        std::fs::create_dir_all(&package).expect("a package");
        assert_eq!(checkout_of(&package).as_deref(), Some(root.path()));
    }
}

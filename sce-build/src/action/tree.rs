// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The action manifest (ADR 0097, section 2): which generations make a
//! generated tree, and where their files go.
//!
//! A tree is declared by one file, `tools/codegen/trees/<tree>.toml`. It names
//! the directories the tree owns (`roots`), the generations that write it, the
//! index files the tree's language needs to compile what they wrote, and the
//! formatter its files are kept in. Everything a regeneration script did by
//! hand is one of those four, so a build hook, a bx action and the check that
//! the declaration says what the scripts did all read this one file and none
//! of them holds generation logic of its own.
//!
//! ```toml
//! [tree]
//! name = "specs"
//! roots = ["sce-specs/src/request_life/generated", "sce-specs/src/import_life/generated"]
//!
//! [[generation]]
//! each = { list = ["request_life", "import_life"], as = "machine" }
//! arguments = ["generate", "sce-specs/machines/{machine}.scxml", "-l", "rust", "-o", "{out}"]
//! place = "sce-specs/src/{machine}/generated"
//! keep = ["*.rs"]
//!
//! [[format]]
//! tool = "rustfmt"
//! directories = ["sce-specs/src/*/generated"]
//! ```
//!
//! A rule's members (`each`) are the directories, the files (by stem or by
//! path, below the root or a directory `under` it, of those holding a text),
//! the documents of a fixture catalog, or a list. A rule may hand one
//! generation a set of documents it compiles together (`set`). A tree the
//! checkout does not commit says so (`committed = false`).

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use super::generation::{Generation, OUTPUT_PLACEHOLDER};

/// The directory of the tree manifests, relative to the checkout's root.
pub const MANIFEST_DIRECTORY: &str = "tools/codegen/trees";

/// One tree's manifest, as written.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TreeManifest {
    pub tree: TreeHead,
    #[serde(default)]
    pub generation: Vec<GenerationRule>,
    #[serde(default)]
    pub index: Vec<IndexRule>,
    #[serde(default)]
    pub format: Vec<FormatRule>,
}

/// The name of a tree and the directories it owns, relative to the root,
/// each a pattern whose components may hold `*`; a directory an `except`
/// pattern matches is another tree's, though a pattern of this one matches
/// it.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TreeHead {
    pub name: String,
    pub roots: Vec<String>,
    #[serde(default)]
    pub except: Vec<String>,
    /// Whether the checkout commits the tree. A tree that is not committed
    /// (its files ignored by version control, written by the lane that
    /// compiles them) is a build output and nothing else: it has no committed
    /// copy to be compared with, and is held only to being the same from
    /// every run of its actions.
    #[serde(default = "committed_by_default")]
    pub committed: bool,
}

/// A tree is committed unless its manifest says it is not, which every tree
/// was before ADR 0097.
fn committed_by_default() -> bool {
    true
}

/// One generation, or one for each member of a set.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GenerationRule {
    #[serde(default)]
    pub each: Option<Each>,
    /// The generator's arguments after its name, paths relative to the root,
    /// the output directory written `{out}`; `{<as>}` is the member.
    pub arguments: Vec<String>,
    #[serde(default)]
    pub environment: BTreeMap<String, String>,
    /// Where the kept files go, relative to the root.
    pub place: String,
    /// Which of the files the generation wrote the tree keeps, by patterns on
    /// their paths relative to the output directory; every file when empty.
    #[serde(default)]
    pub keep: Vec<String>,
    /// Whether a member the generator refuses by name (exit status 7,
    /// `generate/unsupported-feature`: the backend does not lower it yet) is
    /// left out of the tree rather than failing it. Any other refusal fails
    /// the tree.
    #[serde(default)]
    pub skip_unsupported: bool,
    /// Documents one generation compiles together, each handed to the
    /// generator after its own flag, appended to the arguments.
    #[serde(default)]
    pub set: Option<DocumentSet>,
}

/// The documents of a set one generation compiles together
/// (`orchestrate --forge <document>...`): the files a pattern matches, in
/// byte order, each given after `flag`.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DocumentSet {
    /// The files, relative to the root, a pattern whose components may hold
    /// `*`.
    pub paths: String,
    /// The flag each document is given after.
    pub flag: String,
}

impl DocumentSet {
    /// `flag document` for each document of the set, in byte order.
    fn arguments(&self, root: &Path) -> Result<Vec<String>, String> {
        let documents = matching(root, &self.paths, Kind::File)?;
        if documents.is_empty() {
            return Err(format!(
                "the set `{}` matched no document, so the generation would compile nothing",
                self.paths
            ));
        }
        Ok(documents
            .into_iter()
            .flat_map(|document| [self.flag.clone(), document])
            .collect())
    }
}

/// The text a document has to hold to be a member: one text, or any of
/// several.
#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum Containing {
    One(String),
    Any(Vec<String>),
}

impl Containing {
    /// Whether `content` holds the text, or one of the texts.
    fn holds(&self, content: &str) -> bool {
        match self {
            Containing::One(text) => content.contains(text.as_str()),
            Containing::Any(texts) => texts.iter().any(|text| content.contains(text.as_str())),
        }
    }
}

/// The members a rule is repeated for, each named `{<as>}` in it. Exactly one
/// of `directories`, `files`, `paths`, `catalog` and `list` says what they
/// are.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Each {
    /// The names of the directories a pattern matches.
    #[serde(default)]
    pub directories: Option<String>,
    /// The stems of the files a pattern matches (the name less its last
    /// extension).
    #[serde(default)]
    pub files: Option<String>,
    /// The paths of the files a pattern matches, relative to `under` (the
    /// root when `under` is not given), their extension kept.
    #[serde(default)]
    pub paths: Option<String>,
    /// The documents of the fixtures a fixture catalog lists
    /// (`tests/forge/conformance/fixtures.json`), each as the generator takes
    /// it: a file of the resources directory beside the catalog's directory,
    /// relative to the root, or a standard document by its library name.
    #[serde(default)]
    pub catalog: Option<String>,
    /// The values written.
    #[serde(default)]
    pub list: Option<Vec<String>>,
    /// The directory, relative to the root, a pattern is matched in and its
    /// members are named from.
    #[serde(default)]
    pub under: Option<String>,
    /// Of the documents a pattern or a catalog yields, only those holding
    /// this text, or one of these texts.
    #[serde(default)]
    pub containing: Option<Containing>,
    /// Members left out: a rule of their own generates them.
    #[serde(default)]
    pub except: Vec<String>,
    /// The name a member is written by, `{<as>}`.
    pub r#as: String,
}

impl Each {
    /// What decides the members, relative to the root: the directory a
    /// pattern lists, up to its first wildcard, and the catalog a set is read
    /// from. A list is decided by the manifest alone.
    fn listed(&self) -> Vec<String> {
        let under = self.under.as_deref();
        let mut listed: Vec<String> = [&self.directories, &self.files, &self.paths]
            .into_iter()
            .flatten()
            .map(|pattern| listed_directory(under, pattern))
            .collect();
        listed.extend(self.catalog.clone());
        listed
    }

    /// The members, in byte order for a pattern or a catalog and as written
    /// for a list, less those `except` names.
    fn members(&self, root: &Path) -> Result<Vec<String>, String> {
        let name = &self.r#as;
        if self.containing.is_some()
            && self.files.is_none()
            && self.paths.is_none()
            && self.catalog.is_none()
        {
            return Err(format!(
                "the set named `{name}` filters by content, which only `files`, `paths` and \
                 `catalog` have"
            ));
        }
        if self.under.is_some() && self.list.is_some() {
            return Err(format!(
                "the set named `{name}` matches no pattern under `under`, since it is a `list`"
            ));
        }
        if self.under.is_some() && self.catalog.is_some() {
            return Err(format!(
                "the set named `{name}` names its documents from its catalog, not from `under`"
            ));
        }
        let base = match &self.under {
            Some(under) => {
                if under.starts_with('/') || under.contains("..") {
                    return Err(format!(
                        "`{under}` is not a directory under the root (the set named `{name}`)"
                    ));
                }
                root.join(under)
            }
            None => root.to_path_buf(),
        };
        let sources = [
            self.directories.is_some(),
            self.files.is_some(),
            self.paths.is_some(),
            self.catalog.is_some(),
            self.list.is_some(),
        ];
        if sources.iter().filter(|given| **given).count() != 1 {
            return Err(format!(
                "the set named `{name}` has to say exactly one of `directories`, `files`, \
                 `paths`, `catalog` and `list`"
            ));
        }
        let members: Vec<String> = if let Some(pattern) = &self.directories {
            matching(&base, pattern, Kind::Directory)?
                .iter()
                .map(|p| leaf(p).to_string())
                .collect()
        } else if let Some(pattern) = &self.files {
            self.holding(&base, matching(&base, pattern, Kind::File)?)?
                .iter()
                .map(|p| stem(p).to_string())
                .collect()
        } else if let Some(pattern) = &self.paths {
            self.holding(&base, matching(&base, pattern, Kind::File)?)?
        } else if let Some(catalog) = &self.catalog {
            self.holding(root, catalog_documents(root, catalog)?)?
        } else {
            self.list.clone().unwrap_or_default()
        };
        for name in &self.except {
            if !members.contains(name) {
                return Err(format!(
                    "the set named `{}` excepts `{name}`, which is not one of its members",
                    self.r#as
                ));
            }
        }
        Ok(members
            .into_iter()
            .filter(|m| !self.except.contains(m))
            .collect())
    }

    /// Of `documents`, each named relative to `base` or by its library name,
    /// those holding the text `containing` asks for; all of them when it asks
    /// for none.
    fn holding(&self, base: &Path, documents: Vec<String>) -> Result<Vec<String>, String> {
        let Some(containing) = &self.containing else {
            return Ok(documents);
        };
        let mut kept = Vec::new();
        for document in documents {
            let path = crate::forge::stdlib::resolve(base, &document);
            let content =
                crate::conformance::read_document(&path).map_err(|e| format!("{document}: {e}"))?;
            if containing.holds(&content) {
                kept.push(document);
            }
        }
        Ok(kept)
    }
}

/// The document of every fixture the catalog at `catalog` (relative to
/// `root`) lists, in byte order, each once: as `list-fixtures --documents`
/// names it, a file of the resources directory beside the catalog's
/// directory, relative to the root, or a standard document by its library
/// name.
fn catalog_documents(root: &Path, catalog: &str) -> Result<Vec<String>, String> {
    if catalog.starts_with('/') || catalog.contains("..") {
        return Err(format!("`{catalog}` is not a catalog under the root"));
    }
    let manifest = crate::conformance::Manifest::load(&root.join(catalog))?;
    let resources = Path::new(catalog)
        .parent()
        .and_then(Path::parent)
        .map(|dir| dir.join("resources"))
        .ok_or_else(|| format!("`{catalog}` has no directory beside which its resources are"))?;
    let documents: std::collections::BTreeSet<String> = manifest
        .fixtures
        .iter()
        .map(|fixture| fixture.document_path(&resources).display().to_string())
        .collect();
    Ok(documents.into_iter().collect())
}

/// The index file each of some directories holds: a header, then a line for
/// each module the directory holds, entry by entry, each entry's modules in
/// byte order.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IndexRule {
    /// The directories, relative to the root, a pattern whose components may
    /// hold `*`.
    pub directories: String,
    /// Directories the pattern matches that are not indexed this way.
    #[serde(default)]
    pub except: Vec<String>,
    /// The index file's name in each directory, or, holding `{dir}`, its path
    /// relative to the root with `{dir}` the directory's (`{dir}.rs` is the
    /// Rust index beside a directory of modules).
    pub file: String,
    /// The text before the first module, written as it is.
    pub header: String,
    pub entries: Vec<IndexEntry>,
}

/// The modules of one kind an index names.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IndexEntry {
    /// The files of the directory that are modules of this kind, a pattern
    /// on their names, each module named by its stem.
    pub files: String,
    /// The line written for each, `{module}` its name.
    pub line: String,
}

/// A formatter the files of some directories are kept in.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FormatRule {
    pub tool: FormatTool,
    /// The directories whose files it formats, each a pattern with `*`
    /// standing for whole components.
    pub directories: Vec<String>,
}

/// The formatters a tree may be kept in.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FormatTool {
    /// `rustfmt`, with the workspace's edition, over every `.rs` directly in a
    /// directory.
    Rustfmt,
}

/// Why a manifest declares no tree.
#[derive(Debug, thiserror::Error)]
pub enum ManifestError {
    #[error("{path}: {source}")]
    Io { path: PathBuf, source: io::Error },
    #[error("{path}: {source}")]
    Parse {
        path: PathBuf,
        source: toml::de::Error,
    },
    #[error("{path} declares the tree `{name}`; a manifest is named by its tree")]
    Misnamed { path: PathBuf, name: String },
    #[error("there is no tree named `{0}` (the trees are the files of {MANIFEST_DIRECTORY})")]
    NoSuchTree(String),
    #[error("tree `{tree}`: {detail}")]
    Invalid { tree: String, detail: String },
}

/// Every tree of the checkout at `root`, in name order.
pub fn trees(root: &Path) -> Result<Vec<TreeManifest>, ManifestError> {
    let dir = root.join(MANIFEST_DIRECTORY);
    let io = |path: &Path| {
        let path = path.to_path_buf();
        move |source| ManifestError::Io { path, source }
    };
    let mut paths: Vec<PathBuf> = fs::read_dir(&dir)
        .map_err(io(&dir))?
        .map(|entry| entry.map(|e| e.path()))
        .collect::<Result<_, _>>()
        .map_err(io(&dir))?;
    paths.retain(|p| p.extension().is_some_and(|e| e == "toml"));
    paths.sort();
    let mut trees = Vec::new();
    for path in paths {
        let text = fs::read_to_string(&path).map_err(io(&path))?;
        let manifest: TreeManifest =
            toml::from_str(&text).map_err(|source| ManifestError::Parse {
                path: path.clone(),
                source,
            })?;
        if path.file_stem().and_then(|s| s.to_str()) != Some(manifest.tree.name.as_str()) {
            return Err(ManifestError::Misnamed {
                path,
                name: manifest.tree.name,
            });
        }
        trees.push(manifest);
    }
    Ok(trees)
}

/// The tree named `name`.
pub fn tree(root: &Path, name: &str) -> Result<TreeManifest, ManifestError> {
    trees(root)?
        .into_iter()
        .find(|t| t.tree.name == name)
        .ok_or_else(|| ManifestError::NoSuchTree(name.to_string()))
}

/// A generation of a tree, with where its kept files go.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlacedGeneration {
    pub generation: Generation,
    pub place: String,
    pub keep: Vec<String>,
    /// [`GenerationRule::skip_unsupported`].
    pub skip_unsupported: bool,
}

/// A tree with every rule expanded against a checkout.
#[derive(Clone, Debug)]
pub struct ExpandedTree {
    pub name: String,
    pub roots: Vec<String>,
    /// [`TreeHead::except`].
    pub except: Vec<String>,
    /// [`TreeHead::committed`].
    pub committed: bool,
    pub generations: Vec<PlacedGeneration>,
    pub indexes: Vec<IndexRule>,
    pub formats: Vec<FormatRule>,
    /// What the expansion listed to find its members, relative to the root:
    /// the directory each pattern lists and each catalog read. A member that
    /// arrives or leaves changes one of them, so a build that watches them
    /// with the files its generations read knows when the tree has moved. An
    /// empty path is the root itself.
    pub listed: BTreeSet<String>,
}

/// The directory `pattern` lists below `under`, relative to the root: its
/// components before the first that holds `*`, or the whole pattern when
/// none does. Empty when the pattern lists the root itself.
fn listed_directory(under: Option<&str>, pattern: &str) -> String {
    let fixed: Vec<&str> = under
        .into_iter()
        .flat_map(|u| u.split('/'))
        .chain(pattern.split('/').take_while(|c| !c.contains('*')))
        .filter(|c| !c.is_empty() && *c != ".")
        .collect();
    fixed.join("/")
}

impl TreeManifest {
    /// The generations this manifest names in the checkout at `root`, each
    /// member of a set in byte order of its name.
    pub fn expand(&self, root: &Path) -> Result<ExpandedTree, ManifestError> {
        let invalid = |detail: String| ManifestError::Invalid {
            tree: self.tree.name.clone(),
            detail,
        };
        let mut generations = Vec::new();
        let mut listed = BTreeSet::new();
        for rule in &self.generation {
            if let Some(set) = &rule.set {
                listed.insert(listed_directory(None, &set.paths));
            }
            if let Some(each) = &rule.each {
                listed.extend(each.listed());
            }
            let set = match &rule.set {
                Some(set) => set.arguments(root).map_err(invalid)?,
                None => Vec::new(),
            };
            let members: Vec<(Option<&str>, String)> = match &rule.each {
                None => vec![(None, String::new())],
                Some(each) => {
                    let name = &each.r#as;
                    let values = each.members(root).map_err(invalid)?;
                    if values.is_empty() {
                        return Err(invalid(format!(
                            "a rule over `{name}` matched nothing, so it would declare no \
                             generation; a set that is empty on purpose is no rule"
                        )));
                    }
                    values
                        .into_iter()
                        .map(|value| (Some(name.as_str()), value))
                        .collect()
                }
            };
            for (name, value) in members {
                let fill = |text: &str| match name {
                    Some(name) => text.replace(&format!("{{{name}}}"), &value),
                    None => text.to_string(),
                };
                let mut arguments: Vec<String> = rule.arguments.iter().map(|a| fill(a)).collect();
                arguments.extend(set.iter().cloned());
                if !arguments.iter().any(|a| a.contains(OUTPUT_PLACEHOLDER)) {
                    return Err(invalid(format!(
                        "`{}` writes to no `{OUTPUT_PLACEHOLDER}`",
                        arguments.join(" ")
                    )));
                }
                generations.push(PlacedGeneration {
                    generation: Generation {
                        arguments,
                        environment: rule.environment.clone(),
                    },
                    place: fill(&rule.place),
                    keep: rule.keep.clone(),
                    skip_unsupported: rule.skip_unsupported,
                });
            }
        }
        Ok(ExpandedTree {
            name: self.tree.name.clone(),
            roots: self.tree.roots.clone(),
            except: self.tree.except.clone(),
            committed: self.tree.committed,
            generations,
            indexes: self.index.clone(),
            formats: self.format.clone(),
            listed,
        })
    }
}

/// The argument by which a generation chooses the language its machines hand
/// their script engine (`generate --script-engine`, `generate-w3c
/// --script-engine`).
const SCRIPT_ENGINE_ARGUMENT: &str = "--script-engine";

impl ExpandedTree {
    /// Whether the directory `dir`, relative to the root, is another tree's
    /// though a root pattern of this one matches it: an `except` pattern
    /// matches it component by component.
    pub fn excepts(&self, dir: &str) -> bool {
        self.except
            .iter()
            .any(|pattern| matches_components(pattern, dir))
    }

    /// This tree as a host whose script engine reads `language` compiles it:
    /// every generation that leaves the language to its backend's default is
    /// also given `--script-engine <language>`, and the children it writes
    /// inherit that from their parent's run.
    ///
    /// The variant is a configuration of the whole tree rather than of one
    /// generation, since a machine handed text in a language its engine does
    /// not read does not fail, it hangs (`scripts/gates/w3c-kotlin.sh`). A
    /// generation whose manifest names its engine keeps it: its document is
    /// written for that engine, and its test hands the machine one that reads
    /// it whatever the host's default. Every action is keyed by its arguments,
    /// so the variant's are cached apart from the tree's own.
    pub fn for_script_engine(mut self, language: &str) -> Self {
        for placed in &mut self.generations {
            let arguments = &mut placed.generation.arguments;
            let names_its_engine = arguments.iter().any(|a| {
                a == SCRIPT_ENGINE_ARGUMENT || a.starts_with(&format!("{SCRIPT_ENGINE_ARGUMENT}="))
            });
            if !names_its_engine {
                arguments.push(SCRIPT_ENGINE_ARGUMENT.to_string());
                arguments.push(language.to_string());
            }
        }
        self
    }
}

/// What a pattern matches.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    File,
    Directory,
}

/// The paths, relative to `root` and in byte order, of the entries of `kind`
/// that `pattern` matches. A component of the pattern may hold `*`, which
/// matches any run of characters within that component; `**` is not a
/// pattern, since a tree is declared directory by directory.
pub(crate) fn matching(root: &Path, pattern: &str, kind: Kind) -> Result<Vec<String>, String> {
    if pattern.contains("**") || pattern.starts_with('/') || pattern.contains("..") {
        return Err(format!(
            "`{pattern}` is not a pattern of components under the root"
        ));
    }
    let components: Vec<&str> = pattern.split('/').collect();
    let mut found = vec![String::new()];
    for (i, component) in components.iter().enumerate() {
        let last = i + 1 == components.len();
        let mut next = Vec::new();
        for prefix in &found {
            let dir = if prefix.is_empty() {
                root.to_path_buf()
            } else {
                root.join(prefix)
            };
            let join = |name: &str| {
                if prefix.is_empty() {
                    name.to_string()
                } else {
                    format!("{prefix}/{name}")
                }
            };
            if !component.contains('*') {
                let path = dir.join(component);
                let fits = if last {
                    match kind {
                        Kind::File => path.is_file(),
                        Kind::Directory => path.is_dir(),
                    }
                } else {
                    path.is_dir()
                };
                if fits {
                    next.push(join(component));
                }
                continue;
            }
            let entries = match fs::read_dir(&dir) {
                Ok(entries) => entries,
                Err(e) if e.kind() == io::ErrorKind::NotFound => continue,
                Err(e) => return Err(format!("{}: {e}", dir.display())),
            };
            for entry in entries {
                let entry = entry.map_err(|e| format!("{}: {e}", dir.display()))?;
                let Some(name) = entry.file_name().to_str().map(str::to_string) else {
                    continue;
                };
                if !wildcard(component, &name) {
                    continue;
                }
                let file_type = entry
                    .file_type()
                    .map_err(|e| format!("{}: {e}", entry.path().display()))?;
                let fits = if last {
                    match kind {
                        Kind::File => file_type.is_file(),
                        Kind::Directory => file_type.is_dir(),
                    }
                } else {
                    file_type.is_dir()
                };
                if fits {
                    next.push(join(&name));
                }
            }
        }
        found = next;
    }
    found.sort();
    Ok(found)
}

/// Whether `name` matches `pattern`, in which `*` matches any run of
/// characters.
pub(crate) fn wildcard(pattern: &str, name: &str) -> bool {
    let parts: Vec<&str> = pattern.split('*').collect();
    if parts.len() == 1 {
        return pattern == name;
    }
    let (first, rest) = parts.split_first().expect("split yields a part");
    let (last, middle) = rest.split_last().expect("a pattern with `*` has two parts");
    let Some(mut remaining) = name.strip_prefix(first) else {
        return false;
    };
    for part in middle {
        let Some(at) = remaining.find(part) else {
            return false;
        };
        remaining = &remaining[at + part.len()..];
    }
    remaining.len() >= last.len() && remaining.ends_with(last)
}

/// Whether `path` matches `pattern` component by component, each component
/// of the pattern a [`wildcard`] of the path's.
pub(crate) fn matches_components(pattern: &str, path: &str) -> bool {
    let p: Vec<&str> = pattern.split('/').collect();
    let r: Vec<&str> = path.split('/').collect();
    p.len() == r.len() && p.iter().zip(&r).all(|(p, r)| wildcard(p, r))
}

/// The last component of a path.
pub(crate) fn leaf(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

/// The name of a file less its last extension.
pub(crate) fn stem(path: &str) -> &str {
    let name = leaf(path);
    name.rsplit_once('.').map_or(name, |(stem, _)| stem)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A pattern lists the directory before its first wildcard, below the
    /// set's `under`; one with no wildcard names its own path, and one that
    /// begins with a wildcard lists the root, which is the empty path.
    #[test]
    fn a_pattern_lists_the_directory_before_its_first_wildcard() {
        assert_eq!(
            listed_directory(None, "integration_resources/*"),
            "integration_resources"
        );
        assert_eq!(
            listed_directory(None, "sce-build/tests/fixtures/static_datamodel/*.scxml"),
            "sce-build/tests/fixtures/static_datamodel"
        );
        assert_eq!(listed_directory(Some("stdlib"), "*/x/*.scxml"), "stdlib");
        assert_eq!(listed_directory(None, "a/b.scxml"), "a/b.scxml");
        assert_eq!(listed_directory(None, "*.scxml"), "");
    }

    #[test]
    fn a_wildcard_matches_within_a_component() {
        assert!(wildcard("*.rs", "a_sm.rs"));
        assert!(wildcard("*_sm.rs", "a_sm.rs"));
        assert!(!wildcard("*_sm.rs", "mod.rs"));
        assert!(wildcard("a*b*c", "aXbYc"));
        assert!(!wildcard("a*b*c", "aXc"));
        assert!(wildcard("*", ""));
        assert!(!wildcard("ab*ab", "ab"));
        assert!(wildcard("plain", "plain"));
        assert!(!wildcard("plain", "plainer"));
    }

    #[test]
    fn a_pattern_matches_whole_components_in_byte_order() {
        let tmp = tempfile::tempdir().unwrap();
        for dir in ["a/x", "a/y", "b/x", "c"] {
            fs::create_dir_all(tmp.path().join(dir)).unwrap();
        }
        fs::write(tmp.path().join("a/x/f.scxml"), "").unwrap();
        fs::write(tmp.path().join("a/y/f.scxml"), "").unwrap();
        fs::write(tmp.path().join("a/y/g.txt"), "").unwrap();
        fs::write(tmp.path().join("c/Z.scxml"), "").unwrap();
        fs::write(tmp.path().join("c/a.scxml"), "").unwrap();

        assert_eq!(
            matching(tmp.path(), "*/x", Kind::Directory).unwrap(),
            vec!["a/x", "b/x"]
        );
        assert_eq!(
            matching(tmp.path(), "a/*/*.scxml", Kind::File).unwrap(),
            vec!["a/x/f.scxml", "a/y/f.scxml"]
        );
        // Byte order: an upper-case letter sorts before a lower-case one,
        // whatever the locale of the machine says.
        assert_eq!(
            matching(tmp.path(), "c/*.scxml", Kind::File).unwrap(),
            vec!["c/Z.scxml", "c/a.scxml"]
        );
        assert!(matching(tmp.path(), "a/**", Kind::File).is_err());
        assert!(matching(tmp.path(), "../x", Kind::File).is_err());
    }

    #[test]
    fn a_rule_over_a_set_is_one_generation_per_member() {
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir_all(tmp.path().join("in/b")).unwrap();
        fs::create_dir_all(tmp.path().join("in/a")).unwrap();
        let manifest: TreeManifest = toml::from_str(
            r#"
            [tree]
            name = "t"
            roots = ["out"]

            [[generation]]
            each = { directories = "in/*", as = "stem" }
            arguments = ["generate", "in/{stem}/{stem}.scxml", "-l", "rust", "-o", "{out}"]
            place = "out/{stem}"
            keep = ["*.rs"]
            "#,
        )
        .unwrap();
        let tree = manifest.expand(tmp.path()).unwrap();
        let placed: Vec<(&str, &str)> = tree
            .generations
            .iter()
            .map(|g| (g.generation.arguments[1].as_str(), g.place.as_str()))
            .collect();
        assert_eq!(
            placed,
            vec![("in/a/a.scxml", "out/a"), ("in/b/b.scxml", "out/b")]
        );
    }

    #[test]
    fn a_script_engine_variant_gives_every_generation_its_engine() {
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir_all(tmp.path().join("in/b")).unwrap();
        fs::create_dir_all(tmp.path().join("in/a")).unwrap();
        let manifest: TreeManifest = toml::from_str(
            r#"
            [tree]
            name = "t"
            roots = ["out"]

            [[generation]]
            each = { directories = "in/*", as = "stem" }
            arguments = ["generate", "in/{stem}/{stem}.scxml", "-l", "kotlin", "-o", "{out}"]
            place = "out/{stem}"

            [[generation]]
            arguments = ["generate-w3c", "-l", "kotlin", "--output-dir", "{out}"]
            place = ""
            "#,
        )
        .unwrap();
        let tree = manifest.expand(tmp.path()).unwrap();
        let plain: Vec<Vec<String>> = tree
            .generations
            .iter()
            .map(|g| g.generation.arguments.clone())
            .collect();

        let variant = tree.for_script_engine("lua");
        assert_eq!(variant.generations.len(), plain.len());
        for (placed, before) in variant.generations.iter().zip(&plain) {
            let (kept, added) = placed.generation.arguments.split_at(before.len());
            assert_eq!(kept, before.as_slice());
            assert_eq!(added, ["--script-engine", "lua"]);
        }
    }

    /// A generation that names its engine in the manifest is written for that
    /// engine and keeps it in every variant; given a second `--script-engine`,
    /// the generator would refuse the run for a repeated option.
    #[test]
    fn a_script_engine_variant_keeps_the_engine_a_generation_names() {
        let tmp = tempfile::tempdir().unwrap();
        for fixed in [
            r#"["generate", "a.scxml", "-l", "kotlin", "--script-engine", "ecmascript", "-o", "{out}"]"#,
            r#"["generate", "a.scxml", "-l", "kotlin", "--script-engine=ecmascript", "-o", "{out}"]"#,
        ] {
            let manifest: TreeManifest = toml::from_str(&format!(
                r#"
                [tree]
                name = "t"
                roots = ["out"]

                [[generation]]
                arguments = {fixed}
                place = "out"
                "#
            ))
            .unwrap();
            let tree = manifest.expand(tmp.path()).unwrap();
            let before = tree.generations[0].generation.arguments.clone();
            let variant = tree.for_script_engine("lua");
            assert_eq!(variant.generations[0].generation.arguments, before);
        }
    }

    /// A rule whose set is empty declares nothing, which is a defect of the
    /// rule (a moved directory) far more often than an intent.
    #[test]
    fn a_rule_over_an_empty_set_is_refused() {
        let tmp = tempfile::tempdir().unwrap();
        let manifest: TreeManifest = toml::from_str(
            r#"
            [tree]
            name = "t"
            roots = ["out"]

            [[generation]]
            each = { files = "nowhere/*.scxml", as = "doc" }
            arguments = ["generate", "nowhere/{doc}.scxml", "-o", "{out}"]
            place = "out"
            "#,
        )
        .unwrap();
        assert!(manifest.expand(tmp.path()).is_err());
    }

    /// A tree is committed unless its manifest says otherwise; one that says
    /// so is expanded as any other, carrying the word.
    #[test]
    fn a_tree_is_committed_unless_its_manifest_says_it_is_not() {
        let tmp = tempfile::tempdir().unwrap();
        let head = |extra: &str| {
            let manifest: TreeManifest = toml::from_str(&format!(
                r#"
                [tree]
                name = "t"
                roots = ["out"]
                {extra}
                "#
            ))
            .unwrap();
            manifest.expand(tmp.path()).unwrap().committed
        };
        assert!(head(""));
        assert!(!head("committed = false"));
    }

    /// An `except` is a pattern: every directory it matches is another
    /// tree's, and one it does not match is this tree's.
    #[test]
    fn an_except_pattern_leaves_every_directory_it_matches_to_another_tree() {
        let tmp = tempfile::tempdir().unwrap();
        let manifest: TreeManifest = toml::from_str(
            r#"
            [tree]
            name = "t"
            roots = ["gen/*"]
            except = ["gen/test*", "gen/fixed"]
            "#,
        )
        .unwrap();
        let tree = manifest.expand(tmp.path()).unwrap();
        assert!(tree.excepts("gen/test144"));
        assert!(tree.excepts("gen/fixed"));
        assert!(!tree.excepts("gen/ledger_add"));
        assert!(!tree.excepts("gen/fixed/deeper"));
        assert!(!tree.excepts("other/test144"));
    }

    /// `paths` names a member by its path below `under`, extension kept, and
    /// `containing` may ask for any of several texts.
    #[test]
    fn a_path_under_a_directory_is_named_from_it_and_filtered_by_any_text() {
        let tmp = tempfile::tempdir().unwrap();
        for dir in ["lib/a", "lib/b"] {
            fs::create_dir_all(tmp.path().join(dir)).unwrap();
        }
        fs::write(tmp.path().join("lib/a/one.scxml"), "kind=\"algorithm\"").unwrap();
        fs::write(tmp.path().join("lib/a/two.scxml"), "kind=\"enum\"").unwrap();
        fs::write(tmp.path().join("lib/b/three.scxml"), "kind=\"codec\"").unwrap();
        let manifest: TreeManifest = toml::from_str(
            r#"
            [tree]
            name = "t"
            roots = ["out"]

            [[generation]]
            each = { paths = "*/*.scxml", under = "lib", containing = ["kind=\"algorithm\"", "kind=\"codec\""], as = "doc" }
            arguments = ["generate", "sce:std/{doc}", "-o", "{out}"]
            place = "out"
            "#,
        )
        .unwrap();
        let tree = manifest.expand(tmp.path()).unwrap();
        let documents: Vec<&str> = tree
            .generations
            .iter()
            .map(|g| g.generation.arguments[1].as_str())
            .collect();
        assert_eq!(documents, ["sce:std/a/one.scxml", "sce:std/b/three.scxml"]);
    }

    /// A set is one generation that is handed every document of a pattern,
    /// each after its flag, in byte order; a set that matches nothing is
    /// refused, as an empty `each` is.
    #[test]
    fn a_document_set_is_handed_to_one_generation_after_its_flag() {
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir_all(tmp.path().join("set")).unwrap();
        fs::write(tmp.path().join("set/log.scxml"), "").unwrap();
        fs::write(tmp.path().join("set/element.scxml"), "").unwrap();
        let manifest = |paths: &str| -> TreeManifest {
            toml::from_str(&format!(
                r#"
                [tree]
                name = "t"
                roots = ["out"]

                [[generation]]
                arguments = ["orchestrate", "--language", "swift", "--output-dir", "{{out}}"]
                set = {{ paths = "{paths}", flag = "--forge" }}
                place = "out"
                "#
            ))
            .unwrap()
        };
        let tree = manifest("set/*.scxml").expand(tmp.path()).unwrap();
        assert_eq!(tree.generations.len(), 1);
        assert_eq!(
            tree.generations[0].generation.arguments[5..],
            ["--forge", "set/element.scxml", "--forge", "set/log.scxml"]
        );
        assert!(manifest("none/*.scxml").expand(tmp.path()).is_err());
    }

    /// A catalog names the document of each fixture it lists as
    /// `list-fixtures --documents` does: a file of the resources beside the
    /// catalog's directory, or a standard document by its library name.
    #[test]
    fn a_catalog_names_the_document_of_each_fixture() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("the checkout")
            .to_path_buf();
        let each = Each {
            directories: None,
            files: None,
            paths: None,
            catalog: Some("tests/forge/conformance/fixtures.json".to_string()),
            list: None,
            under: None,
            containing: None,
            except: Vec::new(),
            r#as: "doc".to_string(),
        };
        let members = each.members(&root).expect("the catalog reads");
        assert!(!members.is_empty());
        assert!(members.contains(&"sce:std/time/add_months.scxml".to_string()));
        assert!(
            members.iter().all(|m| m.starts_with("sce:std/")
                || (m.starts_with("tests/forge/resources/") && m.ends_with(".scxml"))),
            "{members:?}"
        );
        let mut sorted = members.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(members, sorted, "in byte order, each once");
    }

    /// A list is written, not matched, so it has nothing to filter by content
    /// and no directory to be matched under.
    #[test]
    fn a_list_is_refused_a_content_filter_and_a_directory() {
        let tmp = tempfile::tempdir().unwrap();
        for each in [
            r#"{ list = ["a"], containing = "x", as = "m" }"#,
            r#"{ list = ["a"], under = "lib", as = "m" }"#,
            r#"{ list = ["a"], files = "*.scxml", as = "m" }"#,
        ] {
            let manifest: TreeManifest = toml::from_str(&format!(
                r#"
                [tree]
                name = "t"
                roots = ["out"]

                [[generation]]
                each = {each}
                arguments = ["generate", "{{m}}", "-o", "{{out}}"]
                place = "out"
                "#
            ))
            .unwrap();
            assert!(manifest.expand(tmp.path()).is_err(), "{each}");
        }
    }

    #[test]
    fn an_unknown_key_is_refused() {
        let parsed: Result<TreeManifest, _> = toml::from_str(
            r#"
            [tree]
            name = "t"
            roots = ["out"]
            rootz = ["typo"]
            "#,
        );
        assert!(parsed.is_err());
    }
}

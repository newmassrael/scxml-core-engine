// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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

use std::collections::BTreeMap;
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
/// each a pattern whose components may hold `*`; a directory `except` names
/// is another tree's, though a pattern of this one matches it.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TreeHead {
    pub name: String,
    pub roots: Vec<String>,
    #[serde(default)]
    pub except: Vec<String>,
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
}

/// The members a rule is repeated for, each named `{<as>}` in it. Exactly one
/// of `directories`, `files` and `list` says what they are.
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
    /// The values written.
    #[serde(default)]
    pub list: Option<Vec<String>>,
    /// Of the files a pattern matches, only those holding this text.
    #[serde(default)]
    pub containing: Option<String>,
    /// Members left out: a rule of their own generates them.
    #[serde(default)]
    pub except: Vec<String>,
    /// The name a member is written by, `{<as>}`.
    pub r#as: String,
}

impl Each {
    /// The members, in byte order for a pattern and as written for a list,
    /// less those `except` names.
    fn members(&self, root: &Path) -> Result<Vec<String>, String> {
        if self.containing.is_some() && self.files.is_none() {
            return Err(format!(
                "the set named `{}` filters by content, which only `files` have",
                self.r#as
            ));
        }
        let members: Vec<String> = match (&self.directories, &self.files, &self.list) {
            (Some(pattern), None, None) => matching(root, pattern, Kind::Directory)?
                .iter()
                .map(|p| leaf(p).to_string())
                .collect(),
            (None, Some(pattern), None) => {
                let mut kept = Vec::new();
                for path in matching(root, pattern, Kind::File)? {
                    if let Some(text) = &self.containing {
                        let content = fs::read_to_string(root.join(&path))
                            .map_err(|e| format!("{path}: {e}"))?;
                        if !content.contains(text.as_str()) {
                            continue;
                        }
                    }
                    kept.push(stem(&path).to_string());
                }
                kept
            }
            (None, None, Some(list)) => list.clone(),
            _ => {
                return Err(format!(
                    "the set named `{}` has to say exactly one of `directories`, `files` and \
                     `list`",
                    self.r#as
                ))
            }
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
    /// The index file's name in each directory.
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
    pub generations: Vec<PlacedGeneration>,
    pub indexes: Vec<IndexRule>,
    pub formats: Vec<FormatRule>,
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
        for rule in &self.generation {
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
                let arguments: Vec<String> = rule.arguments.iter().map(|a| fill(a)).collect();
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
            generations,
            indexes: self.index.clone(),
            formats: self.format.clone(),
        })
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

// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A tree written into a directory from its actions (ADR 0097, section 4):
//! `sce-codegen materialize <tree> --into <dir>` and the build hooks of each
//! language call this, and nothing else writes a generated tree.
//!
//! The tree is written at the paths it has under the checkout's root, beneath
//! `into`: each generation's kept files where the manifest places them, then
//! the index files over what was placed, then the formatter over the
//! directories it keeps. The formatter is an action of its own, keyed on the
//! bytes it is given, its version and its settings, so a tree whose
//! generations all hit formats nothing either.

use std::cell::OnceCell;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command as Process;

use super::generation::{GenerationError, Generator, Provenance};
use super::input_root::{input_root, InputFile};
use super::reapi::{platform, Action, ActionResult, Command, Digest, OutputFile, Property};
use super::store::ContentStore;
use super::tree::{matching, wildcard, ExpandedTree, FormatTool, Kind};

/// What a materialization did.
#[derive(Clone, Debug, Default)]
pub struct Report {
    /// Every file written, relative to `into`.
    pub files: BTreeSet<String>,
    /// Generations and format passes the store answered.
    pub hits: usize,
    /// Generations and format passes that ran.
    pub runs: usize,
}

/// Why a tree was not written.
#[derive(Debug, thiserror::Error)]
pub enum MaterializeError {
    #[error("{0}")]
    Io(#[from] io::Error),
    #[error("tree `{tree}`: {source}")]
    Generation {
        tree: String,
        source: GenerationError,
    },
    #[error(
        "tree `{tree}`: {path} is written by two generations with different bytes \
         (`{first}` and `{second}`)"
    )]
    Collision {
        tree: String,
        path: String,
        first: String,
        second: String,
    },
    #[error("tree `{tree}`: the store lost {digest} between finding and reading it")]
    Lost { tree: String, digest: Digest },
    #[error("tree `{tree}`: {detail}")]
    Invalid { tree: String, detail: String },
    #[error("tree `{tree}`: {tool} failed on {dir}:\n{stderr}")]
    Format {
        tree: String,
        tool: &'static str,
        dir: String,
        stderr: String,
    },
}

/// Write `tree` into `into` from its actions, through `store`.
pub fn materialize(
    generator: &Generator,
    store: &dyn ContentStore,
    tree: &ExpandedTree,
    into: &Path,
) -> Result<Report, MaterializeError> {
    let name = || tree.name.clone();
    let mut report = Report::default();
    let mut placed: BTreeMap<String, (Digest, String)> = BTreeMap::new();
    for placed_generation in &tree.generations {
        let shown = placed_generation.generation.arguments.join(" ");
        let produced = generator
            .produce(store, &placed_generation.generation)
            .map_err(|source| MaterializeError::Generation {
                tree: name(),
                source,
            })?;
        match produced.provenance {
            Provenance::Hit => report.hits += 1,
            Provenance::Ran => report.runs += 1,
        }
        for (relative, file) in &produced.files {
            if !kept(&placed_generation.keep, relative) {
                continue;
            }
            // A generation that writes at the paths its files have under the
            // root (`generate-w3c` into an output root) is placed at the root.
            let path = match placed_generation.place.as_str() {
                "" | "." => relative.clone(),
                place => format!("{place}/{relative}"),
            };
            if let Some((digest, first)) = placed.get(&path) {
                if *digest != file.digest {
                    return Err(MaterializeError::Collision {
                        tree: name(),
                        path,
                        first: first.clone(),
                        second: shown,
                    });
                }
                continue;
            }
            let bytes = store
                .read_blob(&file.digest)?
                .ok_or_else(|| MaterializeError::Lost {
                    tree: name(),
                    digest: file.digest.clone(),
                })?;
            write_file(&into.join(&path), &bytes, file.is_executable)?;
            placed.insert(path.clone(), (file.digest.clone(), shown.clone()));
            report.files.insert(path);
        }
    }

    for index in &tree.indexes {
        let modules = matching(into, &index.modules, Kind::File).map_err(|detail| {
            MaterializeError::Invalid {
                tree: name(),
                detail,
            }
        })?;
        let mut text = index.header.clone();
        for module in &modules {
            if *module == index.path {
                continue;
            }
            text.push_str(&index.line.replace("{module}", super::tree::stem(module)));
        }
        write_file(&into.join(&index.path), text.as_bytes(), false)?;
        report.files.insert(index.path.clone());
    }

    let formatter = Formatter::new(generator.root());
    for format in &tree.formats {
        for pattern in &format.directories {
            let dirs = matching(into, pattern, Kind::Directory).map_err(|detail| {
                MaterializeError::Invalid {
                    tree: name(),
                    detail,
                }
            })?;
            for dir in dirs {
                let provenance = match format.tool {
                    FormatTool::Rustfmt => formatter
                        .rustfmt(store, &into.join(&dir))
                        .map_err(|e| e.in_tree(&tree.name, &dir))?,
                };
                match provenance {
                    Some(Provenance::Hit) => report.hits += 1,
                    Some(Provenance::Ran) => report.runs += 1,
                    None => {}
                }
            }
        }
    }
    Ok(report)
}

/// Whether a file at `relative` in a generation's output is one the tree
/// keeps: every file when `keep` is empty, else one some pattern matches
/// component by component.
fn kept(keep: &[String], relative: &str) -> bool {
    keep.is_empty()
        || keep.iter().any(|pattern| {
            let p: Vec<&str> = pattern.split('/').collect();
            let r: Vec<&str> = relative.split('/').collect();
            p.len() == r.len() && p.iter().zip(&r).all(|(p, r)| wildcard(p, r))
        })
}

fn write_file(path: &Path, bytes: &[u8], is_executable: bool) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, bytes)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = if is_executable { 0o755 } else { 0o644 };
        fs::set_permissions(path, fs::Permissions::from_mode(mode))?;
    }
    #[cfg(not(unix))]
    let _ = is_executable;
    Ok(())
}

/// Why a format pass did not run.
#[derive(Debug)]
enum FormatError {
    Io(io::Error),
    Tool { tool: &'static str, stderr: String },
}

impl From<io::Error> for FormatError {
    fn from(e: io::Error) -> Self {
        FormatError::Io(e)
    }
}

impl FormatError {
    fn in_tree(self, tree: &str, dir: &str) -> MaterializeError {
        match self {
            FormatError::Io(e) => MaterializeError::Io(e),
            FormatError::Tool { tool, stderr } => MaterializeError::Format {
                tree: tree.to_string(),
                tool,
                dir: dir.to_string(),
                stderr,
            },
        }
    }
}

/// The formatters of the checkout, each named by its version and settings.
struct Formatter {
    root: PathBuf,
    rustfmt: OnceCell<Result<(String, String), String>>,
}

impl Formatter {
    fn new(root: &Path) -> Formatter {
        Formatter {
            root: root.to_path_buf(),
            rustfmt: OnceCell::new(),
        }
    }

    /// `rustfmt --version` and the workspace's edition, the two things besides
    /// a file's bytes that decide what rustfmt makes of it (the repository
    /// holds no rustfmt configuration).
    fn rustfmt_settings(&self) -> Result<&(String, String), FormatError> {
        self.rustfmt
            .get_or_init(|| {
                let output = Process::new("rustfmt")
                    .arg("--version")
                    .output()
                    .map_err(|e| format!("rustfmt --version: {e}"))?;
                if !output.status.success() {
                    return Err(format!(
                        "rustfmt --version: {}",
                        String::from_utf8_lossy(&output.stderr).trim()
                    ));
                }
                let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
                let manifest = fs::read_to_string(self.root.join("Cargo.toml"))
                    .map_err(|e| format!("Cargo.toml: {e}"))?;
                let edition = manifest
                    .parse::<toml::Table>()
                    .ok()
                    .and_then(|t| {
                        t.get("workspace")?
                            .get("package")?
                            .get("edition")?
                            .as_str()
                            .map(str::to_string)
                    })
                    .ok_or_else(|| "Cargo.toml declares no workspace edition".to_string())?;
                Ok((version, edition))
            })
            .as_ref()
            .map_err(|stderr| FormatError::Tool {
                tool: "rustfmt",
                stderr: stderr.clone(),
            })
    }

    /// Format the `.rs` files directly in `dir` as rustfmt does, from `store`
    /// when it holds the result. `None` when the directory holds none.
    ///
    /// rustfmt formats a file and every module file it declares out of line,
    /// at any depth (a `mod.rs` naming `test144` reaches `test144/*.rs`). So
    /// the action's inputs and outputs are every `.rs` under `dir`, and the
    /// files it is handed are those directly in it.
    fn rustfmt(
        &self,
        store: &dyn ContentStore,
        dir: &Path,
    ) -> Result<Option<Provenance>, FormatError> {
        let (version, edition) = self.rustfmt_settings()?.clone();
        let all = rust_files_under(dir)?;
        let names: Vec<String> = all.iter().filter(|p| !p.contains('/')).cloned().collect();
        if names.is_empty() {
            return Ok(None);
        }
        let mut files = BTreeMap::new();
        for path in &all {
            files.insert(
                path.clone(),
                InputFile {
                    digest: Digest::of(&fs::read(dir.join(path))?),
                    is_executable: false,
                },
            );
        }
        let root = input_root(&files).map_err(|e| FormatError::Tool {
            tool: "rustfmt",
            stderr: e.to_string(),
        })?;
        let mut arguments = vec![
            "rustfmt".to_string(),
            "--edition".to_string(),
            edition,
            "--quiet".to_string(),
        ];
        arguments.extend(names.iter().cloned());
        let properties = vec![Property {
            name: "sce.formatter.rustfmt".to_string(),
            value: version,
        }];
        let command = Command {
            arguments: arguments.clone(),
            environment_variables: Vec::new(),
            platform: Some(platform(properties.clone())),
            working_directory: String::new(),
            output_paths: all.clone(),
        };
        let (command_digest, command_bytes) = Digest::of_message(&command);
        let key = Digest::of_message(&Action {
            command_digest: Some(command_digest),
            input_root_digest: Some(root.digest.clone()),
            do_not_cache: false,
            salt: Vec::new(),
            platform: Some(platform(properties)),
        })
        .0;

        if let Some(result) = store.action_result(&key)? {
            let mut formatted = Vec::new();
            for output in &result.output_files {
                let Some(digest) = &output.digest else {
                    formatted.clear();
                    break;
                };
                match store.read_blob(digest)? {
                    Some(bytes) => formatted.push((output.path.clone(), bytes)),
                    None => {
                        formatted.clear();
                        break;
                    }
                }
            }
            let complete = formatted.len() == all.len()
                && formatted.iter().all(|(path, _)| all.contains(path));
            if complete {
                for (path, bytes) in formatted {
                    write_file(&dir.join(&path), &bytes, false)?;
                }
                return Ok(Some(Provenance::Hit));
            }
        }

        let output = Process::new("rustfmt")
            .args(&arguments[1..])
            .current_dir(dir)
            .output()?;
        if !output.status.success() {
            return Err(FormatError::Tool {
                tool: "rustfmt",
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            });
        }
        let mut output_files = Vec::new();
        for path in &all {
            let digest = store.write_blob(&fs::read(dir.join(path))?)?;
            output_files.push(OutputFile {
                path: path.clone(),
                digest: Some(digest),
                is_executable: false,
            });
        }
        store.write_blob(&command_bytes)?;
        for (_, bytes) in &root.directories {
            store.write_blob(bytes)?;
        }
        store.record_action_result(
            &key,
            &ActionResult {
                output_files,
                ..ActionResult::default()
            },
        )?;
        Ok(Some(Provenance::Ran))
    }
}

/// How a materialized tree differs from a committed one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Difference {
    /// A generated file committed under a root of the tree that the actions
    /// did not write.
    NotProduced(String),
    /// A file the actions wrote that is not committed.
    NotCommitted(String),
    /// A file both hold, with different bytes.
    Differs(String),
}

impl std::fmt::Display for Difference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Difference::NotProduced(p) => write!(f, "committed, not produced: {p}"),
            Difference::NotCommitted(p) => write!(f, "produced, not committed: {p}"),
            Difference::Differs(p) => write!(f, "different bytes:          {p}"),
        }
    }
}

/// How the tree materialized into `into` differs from the one committed in the
/// checkout at `root`, over the tree's roots. A committed file counts when it
/// is generated ([`is_generated_file`]); the files written by hand beside a
/// generated tree are not the tree's.
pub fn compare(
    tree: &ExpandedTree,
    root: &Path,
    into: &Path,
) -> Result<Vec<Difference>, MaterializeError> {
    let invalid = |detail: String| MaterializeError::Invalid {
        tree: tree.name.clone(),
        detail,
    };
    let mut committed = BTreeSet::new();
    let mut produced = BTreeSet::new();
    for pattern in &tree.roots {
        for dir in matching(root, pattern, Kind::Directory).map_err(invalid)? {
            for file in files_under(root, &dir)? {
                if is_generated_file(&root.join(&file))? {
                    committed.insert(file);
                }
            }
        }
        for dir in matching(into, pattern, Kind::Directory).map_err(invalid)? {
            produced.extend(files_under(into, &dir)?);
        }
    }
    let mut differences = Vec::new();
    for file in committed.union(&produced) {
        match (committed.contains(file), produced.contains(file)) {
            (true, false) => differences.push(Difference::NotProduced(file.clone())),
            (false, true) => {
                // A file written by hand that the tree's actions also write is
                // still the actions' file: compare it.
                if root.join(file).is_file() {
                    if fs::read(root.join(file))? != fs::read(into.join(file))? {
                        differences.push(Difference::Differs(file.clone()));
                    }
                } else {
                    differences.push(Difference::NotCommitted(file.clone()));
                }
            }
            (true, true) => {
                if fs::read(root.join(file))? != fs::read(into.join(file))? {
                    differences.push(Difference::Differs(file.clone()));
                }
            }
            (false, false) => unreachable!("a member of the union is in one of the sets"),
        }
    }
    Ok(differences)
}

/// Every file under `dir` (relative to `base`), relative to `base`, sorted.
fn files_under(base: &Path, dir: &str) -> io::Result<Vec<String>> {
    let mut found = Vec::new();
    let mut pending = vec![dir.to_string()];
    while let Some(current) = pending.pop() {
        for entry in fs::read_dir(base.join(&current))? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().into_owned();
            let path = format!("{current}/{name}");
            let kind = entry.file_type()?;
            if kind.is_dir() {
                pending.push(path);
            } else if kind.is_file() {
                found.push(path);
            }
        }
    }
    found.sort();
    Ok(found)
}

/// Every `.rs` file under `dir`, by its path relative to `dir` with `/`
/// between components, sorted.
fn rust_files_under(dir: &Path) -> io::Result<Vec<String>> {
    let mut found = Vec::new();
    let mut pending = vec![String::new()];
    while let Some(current) = pending.pop() {
        for entry in fs::read_dir(dir.join(&current))? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().into_owned();
            let path = if current.is_empty() {
                name.clone()
            } else {
                format!("{current}/{name}")
            };
            let kind = entry.file_type()?;
            if kind.is_dir() {
                pending.push(path);
            } else if kind.is_file() && name.ends_with(".rs") {
                found.push(path);
            }
        }
    }
    found.sort();
    Ok(found)
}

/// Whether the file at `path` is generator output, by what it says of itself
/// (the rules of `sce_generated_file` in scripts/lib/sce_generated_tree.sh):
/// the drift header's banner or `GENERATED -- DO NOT EDIT` on its first line,
/// a sourcemap holding `"source_hash"`, or the stub document of a hybrid
/// invoke's child.
pub fn is_generated_file(path: &Path) -> io::Result<bool> {
    let bytes = fs::read(path)?;
    let text = String::from_utf8_lossy(&bytes);
    let first = text.lines().next().unwrap_or("");
    if (first.contains("SCE-GENERATED ") && first.contains("DO NOT EDIT"))
        || first.contains("GENERATED -- DO NOT EDIT")
    {
        return Ok(true);
    }
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();
    if name == "sce_sourcemap.json" {
        return Ok(text.contains("\"source_hash\""));
    }
    if let Some(stem) = name.strip_suffix(".scxml") {
        return Ok(text
            == format!(
                "<?xml version=\"1.0\"?>\n<scxml xmlns=\"http://www.w3.org/2005/07/scxml\" \
                 name=\"{stem}\" initial=\"final\" version=\"1.0\">\n  <final id=\"final\"/>\n\
                 </scxml>"
            ));
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_keep_pattern_matches_component_by_component() {
        assert!(kept(&[], "any/thing.rs"));
        let keep = vec!["*.rs".to_string()];
        assert!(kept(&keep, "a_sm.rs"));
        assert!(
            !kept(&keep, "sub/a_sm.rs"),
            "a top-level pattern kept a nested file"
        );
        assert!(!kept(&keep, "sce_sourcemap.json"));
        let nested = vec!["*/*.go".to_string()];
        assert!(kept(&nested, "fn/a.go"));
    }

    #[test]
    fn a_file_is_generated_by_what_it_says_of_itself() {
        let tmp = tempfile::tempdir().unwrap();
        let write = |name: &str, text: &str| {
            let path = tmp.path().join(name);
            fs::write(&path, text).unwrap();
            path
        };
        assert!(
            is_generated_file(&write("a.rs", "// SCE-GENERATED \u{2014} DO NOT EDIT\nx")).unwrap()
        );
        assert!(is_generated_file(&write("mod.rs", "// GENERATED -- DO NOT EDIT (x)\n")).unwrap());
        assert!(!is_generated_file(&write("harness.rs", "// hand-written\n")).unwrap());
        assert!(
            is_generated_file(&write("sce_sourcemap.json", "{\"source_hash\":\"x\"}")).unwrap()
        );
        assert!(!is_generated_file(&write("other.json", "{\"source_hash\":\"x\"}")).unwrap());
        let stub = "<?xml version=\"1.0\"?>\n<scxml xmlns=\"http://www.w3.org/2005/07/scxml\" \
                    name=\"t_hybrid0\" initial=\"final\" version=\"1.0\">\n  <final id=\"final\"/>\n</scxml>";
        assert!(is_generated_file(&write("t_hybrid0.scxml", stub)).unwrap());
        assert!(!is_generated_file(&write("other.scxml", stub)).unwrap());
    }
}

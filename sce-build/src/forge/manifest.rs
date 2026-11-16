// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE Forge build manifest — dependency graph and topological build order.
//
// Scans a directory for forge SCXML files, extracts `<sce:import>` declarations,
// builds a dependency graph, and outputs a JSON manifest with topological sort.
//
// Usage:
//   sce-codegen manifest <dir>

use crate::forge::error::{
    ForgeError, Located, ManifestError, RelatedRole, RelatedSite, SourceLocation,
};
use crate::forge::model::{ForgeManifest, ManifestEntry};
use crate::forge::parser;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// Attach `dir` as the `Located` file name for a bare [`ForgeError`].
/// Used at the manifest-scanning boundary when a cross-file failure
/// (directory IO, topological cycle) doesn't belong to any single
/// document and `dir` is the most specific filename we have.
fn locate_at_dir<E: Into<ForgeError>>(dir: &Path, err: E) -> Located<ForgeError> {
    Located::new(err.into(), dir.display().to_string(), None, None)
}

/// Scan a directory for forge SCXML files and build a dependency manifest.
///
/// Returns `ForgeManifest` with:
/// - `entries`: all forge documents found (with their imports)
/// - `build_order`: topologically sorted file list (leaves first)
pub fn build_manifest(dir: &Path) -> Result<ForgeManifest, Located<ForgeError>> {
    let mut entries = Vec::new();

    // Scan for .scxml files
    let scxml_files = collect_scxml_files(dir).map_err(|e| locate_at_dir(dir, e))?;

    for path in &scxml_files {
        let path_label = path.display().to_string();
        let content = std::fs::read_to_string(path).map_err(|e| {
            locate_at_dir(
                dir,
                ManifestError::Io {
                    context: format!("cannot read {}", path.display()),
                    source: e,
                },
            )
        })?;

        // Detect if forge document
        let kind = match parser::detect_kind(&content)
            .map_err(|e| Located::new(e, path_label.clone(), None, None))?
        {
            None => continue,
            // A statechart, and a `test-component`, which is one
            // (docs/SCE_TEST_PROFILE.md §11.5).
            Some(k) if k.reads_as_statechart() => continue,
            Some(k) => k,
        };

        if !kind.is_supported() {
            continue;
        }

        let src = path
            .strip_prefix(dir)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();

        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        // parse_imports_only already returns `Located<ForgeError>` keyed
        // to `path_label`, so per-<sce:import> line data survives the
        // hop through the manifest scanner.
        let imports = parser::parse_imports_only(&content, &path_label)?;

        entries.push(ManifestEntry {
            src,
            name: stem,
            runtime_dep: kind.max_runtime_dep(),
            kind,
            imports,
        });
    }

    // Topological sort
    let build_order =
        topological_sort(&entries).map_err(|e| locate_sort_refusal(dir, &entries, e))?;

    Ok(ForgeManifest {
        entries,
        build_order,
    })
}

/// Place a refusal of the sort. A cycle stands on the import that leaves
/// its first document, in that document's file, and every other import on
/// it is an `also-refused` site (SCE_ERROR_CONTRACT.md §2.4): removing any
/// one of them breaks the cycle. Anything else stands at the directory.
fn locate_sort_refusal(
    dir: &Path,
    entries: &[ManifestEntry],
    err: ManifestError,
) -> Located<ForgeError> {
    let ManifestError::CircularDependency(cycle) = &err else {
        return locate_at_dir(dir, err);
    };
    // Each import on the cycle: where it is written, and the `src` it
    // writes.
    let imports: Vec<(SourceLocation, String)> = cycle
        .windows(2)
        .map(|edge| {
            let line = entries
                .iter()
                .find(|entry| entry.src == edge[0])
                .and_then(|entry| entry.imports.iter().find(|imp| imp.src == edge[1]))
                .and_then(|imp| imp.line);
            let site = SourceLocation {
                file: dir.join(&edge[0]).display().to_string(),
                line,
                col: None,
            };
            (site, edge[1].clone())
        })
        .collect();
    let mut imports = imports.into_iter();
    let Some((first, _)) = imports.next() else {
        return locate_at_dir(dir, err);
    };
    imports.fold(
        Located::new(err.into(), first.file, first.line, None),
        |located, (location, src)| {
            located.related_to(RelatedSite {
                role: RelatedRole::AlsoRefused,
                location,
                actual: Some(src),
            })
        },
    )
}

/// Collect all .scxml files in a directory (non-recursive, single level only).
///
/// Intentionally non-recursive: forge projects are expected to use flat directory
/// layouts. For nested structures, call `build_manifest` on each subdirectory
/// separately or use `<sce:import src="subdir/file.scxml">` relative paths.
fn collect_scxml_files(dir: &Path) -> Result<Vec<std::path::PathBuf>, ManifestError> {
    let rd = std::fs::read_dir(dir).map_err(|e| ManifestError::Io {
        context: format!("cannot read directory {}", dir.display()),
        source: e,
    })?;

    let mut files = Vec::new();
    for entry in rd {
        let entry = entry.map_err(|e| ManifestError::Io {
            context: "directory entry error".to_string(),
            source: e,
        })?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("scxml") {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

/// Topological sort of manifest entries by import dependencies.
/// Returns file sources in dependency order (leaves first).
///
/// Uses Kahn's algorithm with reverse adjacency list for O(V+E) complexity.
/// BTreeMap/BTreeSet ensure deterministic output order.
fn topological_sort(entries: &[ManifestEntry]) -> Result<Vec<String>, ManifestError> {
    let src_set: BTreeSet<&str> = entries.iter().map(|e| e.src.as_str()).collect();

    // in_degree[node] = number of dependencies this node has
    let mut in_degree: BTreeMap<&str, usize> = BTreeMap::new();
    // reverse_deps[dep] = list of nodes that depend on `dep`
    let mut reverse_deps: BTreeMap<&str, Vec<&str>> = BTreeMap::new();

    for entry in entries {
        let entry_deps: Vec<&str> = entry
            .imports
            .iter()
            .filter(|imp| src_set.contains(imp.src.as_str()))
            .map(|imp| imp.src.as_str())
            .collect();
        in_degree.insert(entry.src.as_str(), entry_deps.len());
        for &dep in &entry_deps {
            reverse_deps
                .entry(dep)
                .or_default()
                .push(entry.src.as_str());
        }
        // Ensure every node has a reverse_deps entry
        reverse_deps.entry(entry.src.as_str()).or_default();
    }

    // Start with nodes that have zero dependencies (leaves)
    let mut queue: std::collections::VecDeque<&str> = in_degree
        .iter()
        .filter(|(_, &deg)| deg == 0)
        .map(|(&src, _)| src)
        .collect();

    let mut result = Vec::new();

    while let Some(node) = queue.pop_front() {
        result.push(node.to_string());

        // For every node that depends on `node`, decrement its in_degree
        let mut newly_ready: Vec<&str> = Vec::new();
        if let Some(dependents) = reverse_deps.get(node) {
            for &dependent in dependents {
                if let Some(deg) = in_degree.get_mut(dependent) {
                    *deg = deg.saturating_sub(1);
                    if *deg == 0 {
                        newly_ready.push(dependent);
                    }
                }
            }
        }
        newly_ready.sort();
        for n in newly_ready {
            queue.push_back(n);
        }
    }

    if result.len() != src_set.len() {
        let processed: BTreeSet<&str> = result.iter().map(|s| s.as_str()).collect();
        let remaining: BTreeSet<&str> = src_set
            .iter()
            .copied()
            .filter(|s| !processed.contains(s))
            .collect();
        return Err(ManifestError::CircularDependency(one_cycle(
            entries, &remaining,
        )));
    }

    Ok(result)
}

/// One cycle among the documents the sort left, closed: its first document
/// written again at the end, and rotated to begin at its least path so the
/// same cycle is named the same way whichever document the walk met first.
///
/// What the sort leaves is every document on a cycle and every document
/// that imports one, so each document left imports another one left (the
/// import that kept its count above zero). Following, from the least
/// document left, its least import among those left therefore reaches a
/// document already on the walk, and the walk from there is a cycle.
fn one_cycle(entries: &[ManifestEntry], remaining: &BTreeSet<&str>) -> Vec<String> {
    let next = |src: &str| -> Option<String> {
        entries
            .iter()
            .find(|entry| entry.src == src)?
            .imports
            .iter()
            .map(|imp| imp.src.as_str())
            .filter(|dep| remaining.contains(dep))
            .min()
            .map(str::to_string)
    };
    let mut walk: Vec<String> = Vec::new();
    let mut at = remaining.iter().next().map(|s| s.to_string());
    while let Some(src) = at {
        if let Some(start) = walk.iter().position(|seen| *seen == src) {
            let mut cycle = walk.split_off(start);
            let least = cycle
                .iter()
                .enumerate()
                .min_by(|a, b| a.1.cmp(b.1))
                .map_or(0, |(i, _)| i);
            cycle.rotate_left(least);
            let first = cycle[0].clone();
            cycle.push(first);
            return cycle;
        }
        at = next(&src);
        walk.push(src);
    }
    // Unreachable by the argument above; the walk is still a path through
    // the documents left, and better named than nothing.
    walk
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forge::model::{ForgeImport, ForgeKind, RuntimeDep};

    #[test]
    fn topological_sort_no_deps() {
        let entries = vec![
            ManifestEntry {
                src: "a.scxml".to_string(),
                name: "a".to_string(),
                kind: ForgeKind::Codec,
                runtime_dep: RuntimeDep::None,
                imports: vec![],
            },
            ManifestEntry {
                src: "b.scxml".to_string(),
                name: "b".to_string(),
                kind: ForgeKind::Transform,
                runtime_dep: RuntimeDep::None,
                imports: vec![],
            },
        ];
        let order = topological_sort(&entries).unwrap();
        assert_eq!(order, vec!["a.scxml", "b.scxml"]);
    }

    #[test]
    fn topological_sort_with_deps() {
        let entries = vec![
            ManifestEntry {
                src: "codec.scxml".to_string(),
                name: "codec".to_string(),
                kind: ForgeKind::Codec,
                runtime_dep: RuntimeDep::None,
                imports: vec![],
            },
            ManifestEntry {
                src: "procedure.scxml".to_string(),
                name: "procedure".to_string(),
                kind: ForgeKind::Procedure,
                runtime_dep: RuntimeDep::ForgeRuntime,
                imports: vec![ForgeImport {
                    src: "codec.scxml".to_string(),
                    kind: ForgeKind::Codec,
                    alias: "frame".to_string(),
                    line: None,
                    embed_dispatch: None,
                    flag_binds: Vec::new(),
                    presence: None,
                    requires: None,
                }],
            },
        ];
        let order = topological_sort(&entries).unwrap();
        assert_eq!(order, vec!["codec.scxml", "procedure.scxml"]);
    }

    #[test]
    fn topological_sort_circular_detected() {
        let entries = vec![
            ManifestEntry {
                src: "a.scxml".to_string(),
                name: "a".to_string(),
                kind: ForgeKind::Codec,
                runtime_dep: RuntimeDep::None,
                imports: vec![ForgeImport {
                    src: "b.scxml".to_string(),
                    kind: ForgeKind::Transform,
                    alias: "b".to_string(),
                    line: None,
                    embed_dispatch: None,
                    flag_binds: Vec::new(),
                    presence: None,
                    requires: None,
                }],
            },
            ManifestEntry {
                src: "b.scxml".to_string(),
                name: "b".to_string(),
                kind: ForgeKind::Transform,
                runtime_dep: RuntimeDep::None,
                imports: vec![ForgeImport {
                    src: "a.scxml".to_string(),
                    kind: ForgeKind::Codec,
                    alias: "a".to_string(),
                    line: None,
                    embed_dispatch: None,
                    flag_binds: Vec::new(),
                    presence: None,
                    requires: None,
                }],
            },
        ];
        let result = topological_sort(&entries);
        match result.unwrap_err() {
            ManifestError::CircularDependency(cycle) => {
                assert_eq!(cycle, vec!["a.scxml", "b.scxml", "a.scxml"]);
            }
            other => panic!("expected a cycle, got {other:?}"),
        }
    }

    /// An entry importing `imports`, each as a codec.
    fn entry(src: &str, imports: &[&str]) -> ManifestEntry {
        ManifestEntry {
            src: src.to_string(),
            name: src.trim_end_matches(".scxml").to_string(),
            kind: ForgeKind::Codec,
            runtime_dep: RuntimeDep::None,
            imports: imports
                .iter()
                .enumerate()
                .map(|(i, dep)| ForgeImport {
                    src: dep.to_string(),
                    kind: ForgeKind::Codec,
                    alias: format!("d{i}"),
                    line: Some(3 + i as u32),
                    embed_dispatch: None,
                    flag_binds: Vec::new(),
                    presence: None,
                    requires: None,
                })
                .collect(),
        }
    }

    /// The cycle is its documents alone: `a` imports into `b ↔ c` and is
    /// left unsorted with them, but is not on the cycle. It was named
    /// "among" it while the record listed everything the sort left.
    #[test]
    fn a_cycle_names_its_documents_and_not_the_ones_that_import_it() {
        let entries = vec![
            entry("a.scxml", &["c.scxml"]),
            entry("b.scxml", &["c.scxml"]),
            entry("c.scxml", &["b.scxml"]),
            entry("d.scxml", &[]),
        ];
        match topological_sort(&entries).unwrap_err() {
            ManifestError::CircularDependency(cycle) => {
                assert_eq!(cycle, vec!["b.scxml", "c.scxml", "b.scxml"]);
            }
            other => panic!("expected a cycle, got {other:?}"),
        }
    }

    /// The record stands on the import that leaves the cycle's first
    /// document, and the cycle's other import is an `also-refused` site.
    #[test]
    fn a_cycle_stands_on_its_imports() {
        let entries = vec![
            entry("b.scxml", &["c.scxml"]),
            entry("c.scxml", &["x.scxml", "b.scxml"]),
            entry("x.scxml", &[]),
        ];
        let err = topological_sort(&entries).unwrap_err();
        let located = locate_sort_refusal(Path::new("proj"), &entries, err);
        assert_eq!(
            (located.location.file.as_str(), located.location.line),
            (Path::new("proj").join("b.scxml").to_str().unwrap(), Some(3))
        );
        let related = located.related();
        assert_eq!(related.len(), 1, "{related:?}");
        assert_eq!(related[0].role, RelatedRole::AlsoRefused);
        assert_eq!(
            related[0].location.file,
            Path::new("proj").join("c.scxml").display().to_string()
        );
        assert_eq!(related[0].location.line, Some(4), "c's second import");
        assert_eq!(related[0].actual.as_deref(), Some("b.scxml"));
    }
}

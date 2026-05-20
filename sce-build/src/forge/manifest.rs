// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE Forge build manifest — dependency graph and topological build order.
//
// Scans a directory for forge SCXML files, extracts `<sce:import>` declarations,
// builds a dependency graph, and outputs a JSON manifest with topological sort.
//
// Usage:
//   sce-codegen manifest <dir>

use crate::forge::error::{ForgeError, Located, ManifestError};
use crate::forge::model::{ForgeKind, ForgeManifest, ManifestEntry};
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
            Some(ForgeKind::Statechart) => continue,
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
    let build_order = topological_sort(&entries).map_err(|e| locate_at_dir(dir, e))?;

    Ok(ForgeManifest {
        entries,
        build_order,
    })
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
        let remaining: Vec<String> = src_set
            .iter()
            .filter(|s| !processed.contains(**s))
            .map(|s| s.to_string())
            .collect();
        return Err(ManifestError::CircularDependency(remaining));
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forge::model::{ForgeImport, RuntimeDep};

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
                }],
            },
        ];
        let result = topological_sort(&entries);
        assert!(matches!(
            result.unwrap_err(),
            ManifestError::CircularDependency(_)
        ));
    }
}

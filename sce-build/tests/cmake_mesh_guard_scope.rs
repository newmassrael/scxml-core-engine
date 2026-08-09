// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! Nothing outside the `SCE_ENABLE_MESH` guard may read what only
//! exists inside it.
//!
//! `SCE_ENABLE_MESH` defaults to OFF (`sce/CMakeLists.txt`), so the
//! ~6400 guarded lines of `tests/CMakeLists.txt` are absent from a
//! default configure. Four DDS fixtures were appended *after* that
//! guard closed while still reading `MESH_TEST_DIR`,
//! `SCE_MESH_CPP_TEMPLATE` and linking `sce_mesh_common`. CMake does
//! not fault on an unset variable — it expands to the empty string —
//! so the tree configured cleanly and emitted rules whose input path
//! was `/brake_dds_multi.scxml`. The first symptom was at build time:
//!
//! ```text
//! ninja: error: '/brake_dds_multi.scxml', needed by
//! 'tests/mesh_dds_multi_generated/brake_dds_multi_sm.h', missing
//! ```
//!
//! It stayed invisible because every check that could have seen it
//! looks somewhere else. The repository's own `build/` tree carries
//! `SCE_ENABLE_MESH:BOOL=ON` in its cache, so a developer configuring
//! there takes the working path; no CI workflow configures the main
//! tree at all, and none installs Cyclone DDS. What was broken was the
//! path a new consumer takes first: clone, `mkdir build`, `cmake ..`.
//!
//! A general "variable used outside the branch that sets it" analysis
//! is the tempting gate and is not this one. Measured over the tracked
//! CMake files it reports twelve findings that are all correct code —
//! `if(A) set(V x) elseif(B) set(V y) endif()` followed by
//! `if(V)`, and variables reaching a file from its parent directory
//! scope. Suppressing those needs a twelve-entry allowlist, and an
//! allowlist entry is a claim about code it does not sit next to.
//! This gate instead pins the one guard whose falsity is the default
//! and whose body is large enough to be appended to by accident, where
//! the question has an exact answer: 94 variables and 117 targets are
//! mesh-scoped, and none of them is named outside. Before the move,
//! the same scan reported 31 variable reads and 4 target references
//! outside the guard.
//!
//! What running the same two scans over all 16 declared `option()`s
//! would add, measured rather than guessed: the variable half stays at
//! zero everywhere, and the target half reports 24 more, of which 23
//! are the identifier scan reading prose — `spdlog` inside an
//! `option()` help string, a `third_party/spdlog/LICENSE` path,
//! `spdlog::spdlog` split at the `::`. The twenty-fourth is real:
//! `backends/c/tests` links `sce_c_runtime_posix` unguarded while
//! `backends/c/runtime` creates it under `option(SCE_C_RUNTIME_POSIX
//! ... ON)`, so `-DSCE_C_RUNTIME_POSIX=OFF` configures, passes
//! `ninja -n`, and fails at link. Widening this gate is what closes
//! that, and it belongs in the commit that fixes it rather than in the
//! one that would only turn the gate red.
//!
//! Worth stating for whoever extends this: `ninja -n` sees the missing
//! *input file* half of the failure and is blind to the missing
//! *target* half. A plain library name becomes `-lfoo`, which the
//! build graph accepts and the linker rejects, so the target scan
//! below is the cheap way to see that half at all.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// A clean result has to be earned. If the pathspec or the region
/// scanner ever stops matching, every count below collapses to zero and
/// the gate passes for the wrong reason — these are the floors that
/// turn that into a failure. Measured on the tree that introduced this
/// gate: 45 CMake files, 4 of them carrying a mesh guard, 94 mesh-scoped
/// variables and 117 mesh-scoped targets.
const MIN_CMAKE_FILES: usize = 30;
const MIN_GUARDED_FILES: usize = 3;
const MIN_MESH_SCOPED_VARIABLES: usize = 60;
const MIN_MESH_SCOPED_TARGETS: usize = 80;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent directory")
        .to_path_buf()
}

/// Tracked CMake files, vendored trees excluded.
///
/// `git ls-files` is the enumeration source for the same reason the
/// other tree-wide gates use it: a configured build directory is full
/// of generated CMake, and an untracked scratch file must not be able
/// to red the gate.
fn tracked_cmake_files(root: &Path) -> Vec<(String, String)> {
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["ls-files", "-z", "*CMakeLists.txt", "*.cmake"])
        .output()
        .expect("git ls-files runs");
    assert!(out.status.success(), "git ls-files must succeed");

    String::from_utf8_lossy(&out.stdout)
        .split('\0')
        .filter(|p| !p.is_empty())
        .filter(|p| !p.starts_with("third_party/") && !p.starts_with("vendor/"))
        .map(|p| {
            let text =
                std::fs::read_to_string(root.join(p)).unwrap_or_else(|e| panic!("read {p}: {e}"));
            (p.to_string(), text)
        })
        .collect()
}

/// The line with its comment removed, respecting quoted `#`.
fn code(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut quoted = false;
    let mut prev_backslash = false;
    for c in line.chars() {
        match c {
            '"' if !prev_backslash => {
                quoted = !quoted;
                out.push(c);
            }
            '#' if !quoted => break,
            _ => out.push(c),
        }
        prev_backslash = c == '\\' && !prev_backslash;
    }
    out
}

/// Inclusive line-index ranges of every `if(SCE_ENABLE_MESH ...)` block.
///
/// Block nesting is counted over the flow commands rather than parsed:
/// `elseif(` carries no word boundary before its `if`, so the opener
/// pattern does not mistake a branch for a new block.
fn mesh_guard_regions(text: &str) -> Vec<(usize, usize)> {
    let opener = regex::Regex::new(r"\bif\s*\(\s*SCE_ENABLE_MESH\b").expect("opener");
    let open = regex::Regex::new(r"\b(?:if|foreach|while|function|macro)\s*\(").expect("open");
    let close = regex::Regex::new(r"\b(?:endif|endforeach|endwhile|endfunction|endmacro)\s*\(")
        .expect("close");

    let lines: Vec<String> = text.lines().map(code).collect();
    let mut regions = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        if opener.is_match(&lines[i]) {
            let mut depth = 0usize;
            let mut j = i;
            while j < lines.len() {
                depth += open.find_iter(&lines[j]).count();
                depth = depth.saturating_sub(close.find_iter(&lines[j]).count());
                if depth == 0 {
                    break;
                }
                j += 1;
            }
            regions.push((i, j));
            i = j;
        }
        i += 1;
    }
    regions
}

fn inside(regions: &[(usize, usize)], line: usize) -> bool {
    regions.iter().any(|&(a, b)| line >= a && line <= b)
}

/// `${NAME}` reads on a line.
fn variable_reads(line: &str) -> Vec<String> {
    regex::Regex::new(r"\$\{([A-Za-z_][A-Za-z0-9_]*)\}")
        .expect("read")
        .captures_iter(line)
        .map(|c| c[1].to_string())
        .collect()
}

/// Bare identifiers on a line, with every `${...}` expansion removed
/// first so a variable read is never mistaken for a target reference.
fn identifiers(line: &str) -> Vec<String> {
    let stripped = regex::Regex::new(r"\$\{[^}]*\}")
        .expect("expansion")
        .replace_all(line, " ");
    regex::Regex::new(r"[A-Za-z_][A-Za-z0-9_]*")
        .expect("identifier")
        .find_iter(&stripped)
        .map(|m| m.as_str().to_string())
        .collect()
}

struct Doc {
    path: String,
    lines: Vec<String>,
    regions: Vec<(usize, usize)>,
}

fn documents() -> Vec<Doc> {
    let root = repo_root();
    let files = tracked_cmake_files(&root);
    assert!(
        files.len() >= MIN_CMAKE_FILES,
        "only {} tracked CMake file(s) reached the scan (floor {}); the pathspec \
         is broken, not the tree — a clean result would prove nothing",
        files.len(),
        MIN_CMAKE_FILES,
    );
    files
        .into_iter()
        .map(|(path, text)| {
            let regions = mesh_guard_regions(&text);
            Doc {
                path,
                lines: text.lines().map(code).collect(),
                regions,
            }
        })
        .collect()
}

fn guarded(docs: &[Doc]) -> usize {
    docs.iter().filter(|d| !d.regions.is_empty()).count()
}

#[test]
fn no_mesh_scoped_variable_is_read_outside_the_mesh_guard() {
    let docs = documents();
    assert!(
        guarded(&docs) >= MIN_GUARDED_FILES,
        "found a mesh guard in only {} file(s) (floor {}); the region scanner \
         stopped matching",
        guarded(&docs),
        MIN_GUARDED_FILES,
    );

    let assign = regex::Regex::new(r"\bset\s*\(\s*([A-Za-z_][A-Za-z0-9_]*)").expect("assign");
    let mut scoped_total = 0usize;
    let mut violations: Vec<String> = Vec::new();

    // Variable scope in CMake is per directory, so the question is
    // answered within a file: a name assigned only under the guard and
    // read outside it reads as empty in exactly the configuration the
    // guard describes.
    for doc in &docs {
        if doc.regions.is_empty() {
            continue;
        }
        let mut set_inside: BTreeSet<&str> = BTreeSet::new();
        let mut set_outside: BTreeSet<&str> = BTreeSet::new();
        for (n, line) in doc.lines.iter().enumerate() {
            for cap in assign.captures_iter(line) {
                let name = cap.get(1).expect("group").as_str();
                if inside(&doc.regions, n) {
                    set_inside.insert(name);
                } else {
                    set_outside.insert(name);
                }
            }
        }
        let scoped: BTreeSet<&str> = set_inside.difference(&set_outside).copied().collect();
        scoped_total += scoped.len();

        for (n, line) in doc.lines.iter().enumerate() {
            if inside(&doc.regions, n) {
                continue;
            }
            for name in variable_reads(line) {
                if scoped.contains(name.as_str()) {
                    violations.push(format!("{}:{}  ${{{}}}", doc.path, n + 1, name));
                }
            }
        }
    }

    assert!(
        scoped_total >= MIN_MESH_SCOPED_VARIABLES,
        "only {scoped_total} mesh-scoped variable(s) found (floor \
         {MIN_MESH_SCOPED_VARIABLES}); the assignment scanner is broken",
    );
    assert!(
        violations.is_empty(),
        "{} read(s) of a mesh-scoped variable outside the SCE_ENABLE_MESH \
         guard:\n  {}\n\nSCE_ENABLE_MESH defaults to OFF, so each of these \
         expands to the empty string in a default configure — the tree still \
         configures and the emitted rule takes a path rooted at `/`. Move the \
         block inside the guard rather than repeating the condition.",
        violations.len(),
        violations.join("\n  "),
    );
}

#[test]
fn no_mesh_scoped_target_is_named_outside_the_mesh_guard() {
    let docs = documents();
    let create = regex::Regex::new(
        r"\badd_(?:library|executable|custom_target)\s*\(\s*([A-Za-z_][A-Za-z0-9_]*)",
    )
    .expect("create");

    // Targets are global, not per directory, so both halves of this
    // question span the whole tree.
    let mut created_inside: BTreeSet<String> = BTreeSet::new();
    let mut created_outside: BTreeSet<String> = BTreeSet::new();
    for doc in &docs {
        for (n, line) in doc.lines.iter().enumerate() {
            for cap in create.captures_iter(line) {
                let name = cap.get(1).expect("group").as_str().to_string();
                if inside(&doc.regions, n) {
                    created_inside.insert(name);
                } else {
                    created_outside.insert(name);
                }
            }
        }
    }
    let scoped: BTreeSet<&String> = created_inside.difference(&created_outside).collect();
    assert!(
        scoped.len() >= MIN_MESH_SCOPED_TARGETS,
        "only {} mesh-scoped target(s) found (floor {}); the creation scanner \
         is broken",
        scoped.len(),
        MIN_MESH_SCOPED_TARGETS,
    );

    let mut violations: Vec<String> = Vec::new();
    for doc in &docs {
        for (n, line) in doc.lines.iter().enumerate() {
            if inside(&doc.regions, n) {
                continue;
            }
            for ident in identifiers(line) {
                if scoped.contains(&ident) {
                    violations.push(format!("{}:{}  {}", doc.path, n + 1, ident));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "{} reference(s) to a mesh-scoped target outside the SCE_ENABLE_MESH \
         guard:\n  {}\n\nThe target does not exist when the guard is false. A \
         plain library name carries no `::`, so CMake accepts it at generate \
         time and the tree fails at link instead.",
        violations.len(),
        violations.join("\n  "),
    );
}

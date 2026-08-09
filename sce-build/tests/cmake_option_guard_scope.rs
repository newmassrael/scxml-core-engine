// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! Nothing outside a build option's guard may read what only exists
//! inside it.
//!
//! CMake does not fault on an unset variable — it expands to the empty
//! string — and it accepts an unknown plain library name, because a
//! name without `::` becomes `-lfoo` and only the linker knows whether
//! that resolves. Both make the same mistake silent at configure time:
//! a block that reads a guarded variable or links a guarded target,
//! placed where the guard does not reach, produces a tree that
//! configures cleanly and fails later, or not at all until someone
//! flips the option.
//!
//! Three instances were found in the tree this gate was written
//! against, each reachable and each invisible to every other check:
//!
//!   * `tests/CMakeLists.txt` registered four DDS fixtures past the
//!     `endif()` of the mesh guard. `SCE_ENABLE_MESH` defaults OFF, so
//!     a default configure emitted rules whose input path was the empty
//!     `MESH_TEST_DIR` expanded to `/` — `ninja` stopped on
//!     `'/brake_dds_multi.scxml' ... missing` before compiling anything.
//!   * `backends/c/tests` linked `sce_c_runtime_posix` unguarded while
//!     `backends/c/runtime` builds it under `option(SCE_C_RUNTIME_POSIX
//!     ... ON)`, the knob its own comment offers bare-metal consumers.
//!     `-DSCE_C_RUNTIME_POSIX=OFF` configured, passed `ninja -n`, and
//!     died at `ld.lld: error: unable to find library`.
//!   * The same directory linked `sce_c_scripting`, built under
//!     `SCE_ENABLE_LUA`, the same way — reachable through
//!     `-DSCE_ENABLE_LUA=OFF -DSCE_SCRIPT_ENGINE=quickjs`, since
//!     `SCE_ENABLE_LUA=OFF` alone is refused by an explicit
//!     prerequisite check.
//!
//! Note what the build system itself could and could not see. `ninja
//! -n` catches the missing *input file* and is blind to the missing
//! *target*: `-lfoo` is accepted by the graph and rejected by the
//! linker. Reading the CMake is the cheap way to see both halves, and
//! the only way to see either without the transport, the option, and
//! the configuration that reaches them.
//!
//! Scope and its limits, measured rather than assumed. Over the tracked
//! CMake files this reports zero violations for all 16 declared
//! options. It is deliberately conservative in three places, each a
//! missed violation rather than a false one: a condition containing a
//! top-level `OR` is not treated as a guard; a word is the maximal run
//! of non-whitespace, non-paren, non-quote characters, so
//! `spdlog::spdlog` and `third_party/spdlog/LICENSE` do not read as the
//! target `spdlog` — and neither would a name embedded in a `$<...>`
//! generator expression; and only variables assigned in the same file
//! are considered, because CMake variable scope is per directory while
//! targets are global.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

/// Every pattern the scan uses, compiled once.
///
/// They were built inline where they are used, which put a regex
/// compilation inside a loop over (option x file x line) and cost the
/// gate 116s. This lane is meant to be sub-second; the patterns are
/// constants, so they are compiled as constants.
macro_rules! pattern {
    ($name:ident, $re:expr) => {
        static $name: LazyLock<regex::Regex> =
            LazyLock::new(|| regex::Regex::new($re).expect(stringify!($name)));
    };
}
pattern!(RE_OPTION, r"\boption\s*\(\s*([A-Za-z_][A-Za-z0-9_]*)");
pattern!(RE_OR, r"\bOR\b");
pattern!(RE_AND, r"\bAND\b");
pattern!(RE_IF_OPEN, r"^\s*if\s*\((.*)$");
pattern!(RE_BLOCK_OPEN, r"\b(?:if|foreach|while|function|macro)\s*\(");
pattern!(
    RE_BLOCK_CLOSE,
    r"\b(?:endif|endforeach|endwhile|endfunction|endmacro)\s*\("
);
pattern!(RE_QUOTED, r#""[^"]*""#);
pattern!(RE_EXPANSION, r"\$\{[^}]*\}");
pattern!(RE_WORD_SPLIT, r#"[\s()"]+"#);
pattern!(RE_SET, r"\bset\s*\(\s*([A-Za-z_][A-Za-z0-9_]*)");
pattern!(RE_READ, r"\$\{([A-Za-z_][A-Za-z0-9_]*)\}");
pattern!(
    RE_CREATE,
    r"\badd_(?:library|executable|custom_target)\s*\(\s*([A-Za-z_][A-Za-z0-9_]*)"
);

/// A clean result has to be earned. If the pathspec, the option
/// scanner or the region scanner ever stops matching, every count
/// collapses to zero and the gate passes for the wrong reason. Measured
/// on the tree that introduced it: 45 CMake files, 16 declared options,
/// 7 of them carrying a guard, 112 guard-scoped variables and 162
/// guard-scoped targets.
const MIN_CMAKE_FILES: usize = 30;
const MIN_DECLARED_OPTIONS: usize = 10;
const MIN_GUARDED_OPTIONS: usize = 5;
const MIN_SCOPED_VARIABLES: usize = 70;
const MIN_SCOPED_TARGETS: usize = 100;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent directory")
        .to_path_buf()
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

struct Doc {
    path: String,
    lines: Vec<String>,
}

/// Tracked CMake files, vendored trees excluded.
///
/// `git ls-files` is the enumeration source for the same reason the
/// other tree-wide gates use it: a configured build directory is full
/// of generated CMake, and an untracked scratch file must not be able
/// to red the gate.
fn documents() -> Vec<Doc> {
    let root = repo_root();
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(&root)
        .args(["ls-files", "-z", "*CMakeLists.txt", "*.cmake"])
        .output()
        .expect("git ls-files runs");
    assert!(out.status.success(), "git ls-files must succeed");

    let docs: Vec<Doc> = String::from_utf8_lossy(&out.stdout)
        .split('\0')
        .filter(|p| !p.is_empty())
        .filter(|p| !p.starts_with("third_party/") && !p.starts_with("vendor/"))
        .map(|p| {
            let text =
                std::fs::read_to_string(root.join(p)).unwrap_or_else(|e| panic!("read {p}: {e}"));
            Doc {
                path: p.to_string(),
                lines: text.lines().map(code).collect(),
            }
        })
        .collect();

    assert!(
        docs.len() >= MIN_CMAKE_FILES,
        "only {} tracked CMake file(s) reached the scan (floor {}); the pathspec \
         is broken, not the tree — a clean result would prove nothing",
        docs.len(),
        MIN_CMAKE_FILES,
    );
    docs
}

/// Every option the project declares. Vendored options never reach
/// here because their files are not scanned.
fn declared_options(docs: &[Doc]) -> BTreeSet<String> {
    let opts: BTreeSet<String> = docs
        .iter()
        .flat_map(|d| d.lines.iter())
        .filter_map(|l| RE_OPTION.captures(l))
        .map(|c| c[1].to_string())
        .collect();
    assert!(
        opts.len() >= MIN_DECLARED_OPTIONS,
        "found only {} declared option(s) (floor {}); the option scanner is broken",
        opts.len(),
        MIN_DECLARED_OPTIONS,
    );
    opts
}

/// Whether an `if(...)` condition guarantees `opt` is true inside it.
///
/// Only a conjunction counts, and only with the option as a positive
/// literal: `A AND OPT AND NOT B` guards, `NOT OPT` and `A OR OPT` do
/// not. Rejecting a top-level `OR` outright is the conservative
/// direction — it can miss a guard, never invent one.
fn condition_guards(condition: &str, opt: &str) -> bool {
    if RE_OR.is_match(condition) {
        return false;
    }
    RE_AND
        .split(condition)
        .any(|conjunct| conjunct.trim() == opt)
}

/// Inclusive line-index ranges of every block guarding `opt`.
///
/// Block nesting is counted over the flow commands rather than parsed:
/// `elseif(` carries no word boundary before its `if`, so the opener
/// pattern does not mistake a branch for a new block.
fn guard_regions(lines: &[String], opt: &str) -> Vec<(usize, usize)> {
    let mut regions = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let guards = RE_IF_OPEN
            .captures(&lines[i])
            .is_some_and(|c| condition_guards(c[1].trim_end().trim_end_matches(')'), opt));
        if guards {
            let mut depth = 0usize;
            let mut j = i;
            while j < lines.len() {
                depth += RE_BLOCK_OPEN.find_iter(&lines[j]).count();
                depth = depth.saturating_sub(RE_BLOCK_CLOSE.find_iter(&lines[j]).count());
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

/// Bare words on a line: quoted strings and `${...}` expansions removed
/// first, then split on whitespace and parens only. Keeping `::` and
/// `/` inside a word is what stops a namespaced target and a file path
/// from reading as the plain target name they contain.
fn words(line: &str) -> Vec<String> {
    let no_quotes = RE_QUOTED.replace_all(line, " ");
    let no_expansion = RE_EXPANSION.replace_all(&no_quotes, " ");
    RE_WORD_SPLIT
        .split(&no_expansion)
        .filter(|w| !w.is_empty())
        .map(|w| w.to_string())
        .collect()
}

/// Per option, the regions guarding it in each document.
fn regions_by_option(
    docs: &[Doc],
    opts: &BTreeSet<String>,
) -> BTreeMap<String, Vec<Vec<(usize, usize)>>> {
    opts.iter()
        .map(|opt| {
            let per_doc = docs
                .iter()
                .map(|d| guard_regions(&d.lines, opt))
                .collect::<Vec<_>>();
            (opt.clone(), per_doc)
        })
        .filter(|(_, per_doc)| per_doc.iter().any(|r| !r.is_empty()))
        .collect()
}

#[test]
fn no_guard_scoped_variable_is_read_outside_its_guard() {
    let docs = documents();
    let opts = declared_options(&docs);
    let by_option = regions_by_option(&docs, &opts);
    assert!(
        by_option.len() >= MIN_GUARDED_OPTIONS,
        "only {} option(s) carry a guard (floor {}); the region scanner stopped \
         matching",
        by_option.len(),
        MIN_GUARDED_OPTIONS,
    );

    let mut scoped_total = 0usize;
    let mut violations: Vec<String> = Vec::new();

    for (opt, per_doc) in &by_option {
        // Variable scope in CMake is per directory, so the question is
        // answered within a file: a name assigned only under the guard
        // and read outside it reads as empty in exactly the
        // configuration the guard describes.
        for (doc, regions) in docs.iter().zip(per_doc) {
            if regions.is_empty() {
                continue;
            }
            let mut set_inside: BTreeSet<&str> = BTreeSet::new();
            let mut set_outside: BTreeSet<&str> = BTreeSet::new();
            for (n, line) in doc.lines.iter().enumerate() {
                for cap in RE_SET.captures_iter(line) {
                    let name = cap.get(1).expect("group").as_str();
                    if inside(regions, n) {
                        set_inside.insert(name);
                    } else {
                        set_outside.insert(name);
                    }
                }
            }
            let scoped: BTreeSet<&str> = set_inside.difference(&set_outside).copied().collect();
            scoped_total += scoped.len();

            for (n, line) in doc.lines.iter().enumerate() {
                if inside(regions, n) {
                    continue;
                }
                for cap in RE_READ.captures_iter(line) {
                    let name = cap.get(1).expect("group").as_str();
                    if scoped.contains(name) {
                        violations.push(format!("{}:{}  ${{{name}}}  [{opt}]", doc.path, n + 1));
                    }
                }
            }
        }
    }

    assert!(
        scoped_total >= MIN_SCOPED_VARIABLES,
        "only {scoped_total} guard-scoped variable(s) found (floor \
         {MIN_SCOPED_VARIABLES}); the assignment scanner is broken",
    );
    assert!(
        violations.is_empty(),
        "{} read(s) of a guard-scoped variable outside the guard:\n  {}\n\nThe \
         option can be off, and CMake expands an unset variable to the empty \
         string rather than failing — the tree still configures and the emitted \
         rule takes a path rooted at `/`. Move the block inside the guard \
         rather than repeating the condition.",
        violations.len(),
        violations.join("\n  "),
    );
}

#[test]
fn no_guard_scoped_target_is_named_outside_its_guard() {
    let docs = documents();
    let opts = declared_options(&docs);
    let by_option = regions_by_option(&docs, &opts);

    let mut scoped_total = 0usize;
    let mut violations: Vec<String> = Vec::new();

    for (opt, per_doc) in &by_option {
        // Targets are global, not per directory, so both halves of this
        // question span the whole tree.
        let mut created_inside: BTreeSet<String> = BTreeSet::new();
        let mut created_outside: BTreeSet<String> = BTreeSet::new();
        for (doc, regions) in docs.iter().zip(per_doc) {
            for (n, line) in doc.lines.iter().enumerate() {
                for cap in RE_CREATE.captures_iter(line) {
                    let name = cap.get(1).expect("group").as_str().to_string();
                    if inside(regions, n) {
                        created_inside.insert(name);
                    } else {
                        created_outside.insert(name);
                    }
                }
            }
        }
        let scoped: BTreeSet<&String> = created_inside.difference(&created_outside).collect();
        scoped_total += scoped.len();

        for (doc, regions) in docs.iter().zip(per_doc) {
            for (n, line) in doc.lines.iter().enumerate() {
                if inside(regions, n) {
                    continue;
                }
                for word in words(line) {
                    if scoped.contains(&word) {
                        violations.push(format!("{}:{}  {word}  [{opt}]", doc.path, n + 1));
                    }
                }
            }
        }
    }

    assert!(
        scoped_total >= MIN_SCOPED_TARGETS,
        "only {scoped_total} guard-scoped target(s) found (floor \
         {MIN_SCOPED_TARGETS}); the creation scanner is broken",
    );
    assert!(
        violations.is_empty(),
        "{} reference(s) to a guard-scoped target outside the guard:\n  {}\n\nThe \
         target does not exist when the option is off. A plain library name \
         carries no `::`, so CMake accepts it at generate time, `ninja -n` \
         accepts the resulting `-lname`, and the tree fails at link.",
        violations.len(),
        violations.join("\n  "),
    );
}

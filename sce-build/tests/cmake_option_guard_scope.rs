// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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
//!
//! A fragment a CMake file `include()`s runs where it is included, so a
//! fragment every `include()` of which lies under a guard is under that
//! guard as a whole (`tests/cmake/mesh_https.cmake`, included only inside
//! the mesh block, links `sce_mesh_common`). The includes are matched by
//! file name rather than by a resolved path, so an include whose path reads
//! a variable this scan cannot expand still counts, and an include outside
//! the guard of any file of that name leaves the fragment unguarded: a
//! missed guard, never an invented one.
//!
//! A test family is a guard too. `cmake/SCETestFamilies.cmake` declares
//! each family with `sce_test_family(<name> ...)`, the block of the test
//! tree that registers a family's tests opens with
//! `if(SCE_TEST_FAMILY_<NAME>)`, and a tree configured with
//! `SCE_TEST_FAMILIES` naming other families declares nothing of that
//! block. So a family's guard is held here as an option's is. Two more
//! claims are held, because a family list scopes a tree correctly only
//! while they hold: every test the test tree registers lies under exactly
//! one family's guard, or is one of `SHARED_TESTS`, so a list registers
//! each test of the families it names and the empty list every test the
//! tree had before families existed; and every family a guard names is
//! declared, since a misspelt guard is false in every tree, the default
//! one included. For those two, a directory `add_subdirectory` adds only
//! under a guard is under it as a whole, as an included fragment is.

mod common;

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
pattern!(RE_INCLUDE, r#"\binclude\s*\(\s*"?([^")\s]+)"#);
pattern!(RE_SUBDIRECTORY, r#"\badd_subdirectory\s*\(\s*"?([^")\s]+)"#);
pattern!(
    RE_TEST_FAMILY,
    r"\bsce_test_family\s*\(\s*([a-z][a-z0-9_]*)"
);
pattern!(RE_TEST_FAMILY_READ, r"\bSCE_TEST_FAMILY_([A-Z0-9_]+)\b");
pattern!(RE_ADD_TEST, r"\badd_test\s*\(");
pattern!(RE_TEST_NAME, r"\badd_test\s*\(\s*(?:NAME\s+)?([^\s)]+)");

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
/// Measured on the tree that introduced test families: 8 families, and 351
/// `add_test` calls in the test tree.
const MIN_TEST_FAMILIES: usize = 6;
const MIN_TEST_TREE_REGISTRATIONS: usize = 250;

/// The tests registered outside every family's guard, each with why: every
/// family's tree has them, whichever families it registers.
const SHARED_TESTS: &[(&str, &str)] = &[
    (
        "sce_build_is_current",
        "the staleness fixture's setup, which every registered test requires",
    ),
    (
        "install_tree_consumer",
        "an opt-in of its own (SCE_BUILD_INSTALL_CONSUMER_TEST), off by default",
    ),
];

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
    let docs: Vec<Doc> = common::repository::paths_git_tracks(&["*CMakeLists.txt", "*.cmake"])
        .iter()
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
    let mut opts: BTreeSet<String> = docs
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
    opts.extend(declared_test_families(docs));
    opts
}

/// The guard of every test family `cmake/SCETestFamilies.cmake` declares:
/// `SCE_TEST_FAMILY_<NAME>` for each `sce_test_family(<name> ...)`.
fn declared_test_families(docs: &[Doc]) -> BTreeSet<String> {
    docs.iter()
        .flat_map(|d| d.lines.iter())
        .filter_map(|l| RE_TEST_FAMILY.captures(l))
        .map(|c| format!("SCE_TEST_FAMILY_{}", c[1].to_uppercase()))
        .collect()
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

/// The last component of a path: the file an `include()` names.
fn file_name(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

/// Per document, the regions guarding `opt`: the blocks of its own text, or
/// the whole document where it is `include()`d and every include of it lies
/// in a block guarding `opt` or in a document so guarded as a whole.
///
/// A document no include names (a directory's `CMakeLists.txt`, which
/// `add_subdirectory` reaches) keeps its own blocks only.
fn guard_regions_in(docs: &[Doc], opt: &str) -> Vec<Vec<(usize, usize)>> {
    guarded_regions(docs, opt, false)
}

/// As `guard_regions_in`, and a directory's `CMakeLists.txt` is guarded as
/// a whole too where every `add_subdirectory` that adds it lies in a block
/// guarding `opt` or in a document so guarded as a whole.
fn guard_regions_through_directories(docs: &[Doc], opt: &str) -> Vec<Vec<(usize, usize)>> {
    guarded_regions(docs, opt, true)
}

/// The `CMakeLists.txt` an `add_subdirectory(<dir>)` in `from` adds, by its
/// path in the repository; none for a directory named through a variable.
fn added_directory(from: &str, dir: &str) -> Option<String> {
    if dir.contains('$') {
        return None;
    }
    let dir = dir.trim_end_matches('/');
    Some(match from.rsplit_once('/') {
        Some((base, _)) => format!("{base}/{dir}/CMakeLists.txt"),
        None => format!("{dir}/CMakeLists.txt"),
    })
}

/// Per document, the line ranges a guard holds.
type DocumentRegions = Vec<Vec<(usize, usize)>>;

fn guarded_regions(docs: &[Doc], opt: &str, through_directories: bool) -> DocumentRegions {
    let own: Vec<Vec<(usize, usize)>> = docs.iter().map(|d| guard_regions(&d.lines, opt)).collect();
    // Each document a directory adds, with the (document, line) of every
    // `add_subdirectory` that adds it.
    let mut added: BTreeMap<usize, Vec<(usize, usize)>> = BTreeMap::new();
    if through_directories {
        let at_path: BTreeMap<&str, usize> = docs
            .iter()
            .enumerate()
            .map(|(at, d)| (d.path.as_str(), at))
            .collect();
        for (at, doc) in docs.iter().enumerate() {
            for (n, line) in doc.lines.iter().enumerate() {
                for cap in RE_SUBDIRECTORY.captures_iter(line) {
                    let Some(path) =
                        added_directory(&doc.path, cap.get(1).expect("group").as_str())
                    else {
                        continue;
                    };
                    if let Some(&index) = at_path.get(path.as_str()) {
                        added.entry(index).or_default().push((at, n));
                    }
                }
            }
        }
    }
    // Each file name an include names, with the (document, line) of every
    // include of it.
    let mut includes: BTreeMap<&str, Vec<(usize, usize)>> = BTreeMap::new();
    for (at, doc) in docs.iter().enumerate() {
        for (n, line) in doc.lines.iter().enumerate() {
            for cap in RE_INCLUDE.captures_iter(line) {
                let name = file_name(cap.get(1).expect("group").as_str());
                includes.entry(name).or_default().push((at, n));
            }
        }
    }
    // A fragment included from a fragment is guarded once its includer is,
    // so the set grows until no document joins it.
    let mut whole = vec![false; docs.len()];
    let mut grew = true;
    while grew {
        grew = false;
        for (index, doc) in docs.iter().enumerate() {
            if whole[index] {
                continue;
            }
            let guarded = |sites: Option<&Vec<(usize, usize)>>| {
                sites.is_some_and(|sites| {
                    sites
                        .iter()
                        .all(|&(at, n)| whole[at] || inside(&own[at], n))
                })
            };
            if guarded(includes.get(file_name(&doc.path))) || guarded(added.get(&index)) {
                whole[index] = true;
                grew = true;
            }
        }
    }
    docs.iter()
        .zip(own)
        .zip(whole)
        .map(|((doc, regions), whole)| {
            if whole && !doc.lines.is_empty() {
                vec![(0, doc.lines.len() - 1)]
            } else {
                regions
            }
        })
        .collect()
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
        .map(|opt| (opt.clone(), guard_regions_in(docs, opt)))
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

/// A fragment `include()`d only under a guard is under it as a whole, and so
/// is a fragment included only from it; one also included outside the guard,
/// or included by nothing, keeps only its own blocks.
#[test]
fn a_fragment_included_only_under_a_guard_is_guarded_whole() {
    let doc = |path: &str, text: &str| Doc {
        path: path.to_string(),
        lines: text.lines().map(code).collect(),
    };
    let docs = [
        doc(
            "CMakeLists.txt",
            "if(OPT)\n  include(${CMAKE_CURRENT_SOURCE_DIR}/cmake/guarded.cmake)\n  \
             include(\"${X}/open.cmake\")\nendif()\ninclude(cmake/open.cmake)\n",
        ),
        doc(
            "cmake/guarded.cmake",
            "add_library(t STATIC t.c)\ninclude(nested.cmake)\n",
        ),
        doc("cmake/nested.cmake", "target_link_libraries(x PRIVATE t)\n"),
        doc("cmake/open.cmake", "target_link_libraries(y PRIVATE t)\n"),
        doc("cmake/orphan.cmake", "target_link_libraries(z PRIVATE t)\n"),
    ];
    let regions = guard_regions_in(&docs, "OPT");
    assert_eq!(regions[0], [(0, 3)], "the includer keeps its own block");
    assert_eq!(
        regions[1],
        [(0, 1)],
        "a fragment included only under the guard is guarded whole"
    );
    assert_eq!(
        regions[2],
        [(0, 0)],
        "a fragment included only from a guarded fragment is guarded whole"
    );
    assert!(
        regions[3].is_empty(),
        "a fragment also included outside the guard is not guarded: {:?}",
        regions[3]
    );
    assert!(
        regions[4].is_empty(),
        "a fragment nothing includes is not guarded: {:?}",
        regions[4]
    );
}

/// A directory `add_subdirectory` adds only under a guard is under it as a
/// whole, when directories are followed; one also added outside the guard is
/// not, and without following directories neither is.
#[test]
fn a_directory_added_only_under_a_guard_is_guarded_whole() {
    let doc = |path: &str, text: &str| Doc {
        path: path.to_string(),
        lines: text.lines().map(code).collect(),
    };
    let docs = [
        doc(
            "CMakeLists.txt",
            "if(OPT)\n  add_subdirectory(guarded)\n  add_subdirectory(open)\nendif()\n\
             add_subdirectory(open)\n",
        ),
        doc("guarded/CMakeLists.txt", "add_test(NAME t COMMAND t)\n"),
        doc("open/CMakeLists.txt", "add_test(NAME u COMMAND u)\n"),
    ];
    let through = guard_regions_through_directories(&docs, "OPT");
    assert_eq!(through[0], [(0, 3)], "the adder keeps its own block");
    assert_eq!(
        through[1],
        [(0, 0)],
        "a directory added only under the guard is guarded whole"
    );
    assert!(
        through[2].is_empty(),
        "a directory also added outside the guard is not guarded: {:?}",
        through[2]
    );
    assert!(
        guard_regions_in(&docs, "OPT")[1].is_empty(),
        "directories are followed only when asked"
    );
}

/// The name an `add_test` at line `n` registers, read from that line and the
/// two after it, since a long call names its test on the next line.
fn test_name(lines: &[String], n: usize) -> String {
    let call = lines[n..lines.len().min(n + 3)].join(" ");
    RE_TEST_NAME
        .captures(&call)
        .map_or_else(|| "<unnamed>".to_string(), |c| c[1].to_string())
}

/// Every test the test tree registers lies under exactly one family's guard,
/// or is one of `SHARED_TESTS`. The test tree is every document guarded by
/// `BUILD_TESTS` as a whole: the directories the top-level `CMakeLists.txt`
/// adds under it, those they add, and the fragments they include. An example
/// that registers its test under a `BUILD_TESTS` block of its own is not in
/// it: the examples are `BUILD_EXAMPLES`'s, not a family's.
///
/// This is what makes the empty `SCE_TEST_FAMILIES` register every test the
/// tree registered before families existed: each family resolves ON there
/// (`a_family_list_registers_the_families_it_names`), and a registration no
/// family guards would be the one place where the family list and the tree
/// disagree.
#[test]
fn every_test_the_test_tree_registers_belongs_to_one_family() {
    let docs = documents();
    let families = declared_test_families(&docs);
    assert!(
        families.len() >= MIN_TEST_FAMILIES,
        "found only {} test families (floor {MIN_TEST_FAMILIES}); \
         cmake/SCETestFamilies.cmake moved or its declarations stopped matching",
        families.len(),
    );
    let tree = guard_regions_through_directories(&docs, "BUILD_TESTS");
    let guards: Vec<(&str, DocumentRegions)> = families
        .iter()
        .map(|family| {
            (
                family.as_str(),
                guard_regions_through_directories(&docs, family),
            )
        })
        .collect();

    let mut registrations = 0usize;
    let mut per_family: BTreeMap<&str, usize> = BTreeMap::new();
    let mut shared_seen: BTreeSet<String> = BTreeSet::new();
    let mut violations: Vec<String> = Vec::new();
    for (at, doc) in docs.iter().enumerate() {
        if doc.lines.is_empty() || tree[at] != [(0, doc.lines.len() - 1)] {
            continue;
        }
        for (n, line) in doc.lines.iter().enumerate() {
            if !RE_ADD_TEST.is_match(line) {
                continue;
            }
            registrations += 1;
            let owners: Vec<&str> = guards
                .iter()
                .filter(|(_, regions)| inside(&regions[at], n))
                .map(|&(family, _)| family)
                .collect();
            if let [owner] = owners.as_slice() {
                *per_family.entry(*owner).or_default() += 1;
                continue;
            }
            let name = test_name(&doc.lines, n);
            if owners.is_empty() && SHARED_TESTS.iter().any(|&(shared, _)| shared == name) {
                shared_seen.insert(name);
                continue;
            }
            let held = if owners.is_empty() {
                "no family".to_string()
            } else {
                owners.join(", ")
            };
            violations.push(format!("{}:{}  {name}  [{held}]", doc.path, n + 1));
        }
    }

    assert!(
        registrations >= MIN_TEST_TREE_REGISTRATIONS,
        "only {registrations} add_test call(s) reached the scan (floor \
         {MIN_TEST_TREE_REGISTRATIONS}); the test tree is no longer found under \
         BUILD_TESTS, and a clean result would prove nothing",
    );
    assert!(
        violations.is_empty(),
        "{} test(s) the test tree registers outside exactly one family's guard:\n  {}\n\n\
         Open the block with the guard of the family the test belongs to \
         (`if(SCE_TEST_FAMILY_<NAME>)`, cmake/SCETestFamilies.cmake). A test no \
         family guards is registered by every family list, so a lane that names \
         one family configures it too; one two families guard is left out by a \
         list that names only one of them.",
        violations.len(),
        violations.join("\n  "),
    );
    let idle: Vec<&String> = families
        .iter()
        .filter(|family| !per_family.contains_key(family.as_str()))
        .collect();
    assert!(
        idle.is_empty(),
        "the test families {idle:?} guard no test: a family that registers nothing \
         scopes a tree to nothing, which reads as a passing lane",
    );
    let unused: Vec<&str> = SHARED_TESTS
        .iter()
        .map(|&(shared, _)| shared)
        .filter(|shared| !shared_seen.contains(*shared))
        .collect();
    assert!(
        unused.is_empty(),
        "SHARED_TESTS names {unused:?}, which the test tree no longer registers \
         outside every family: drop the entry",
    );
}

/// Every `SCE_TEST_FAMILY_<NAME>` a CMake file reads is a family
/// `cmake/SCETestFamilies.cmake` declares. A guard naming no family is false
/// in every tree, the default one included, so the block it opens would
/// silently stop being registered anywhere.
#[test]
fn every_family_a_guard_names_is_declared() {
    let docs = documents();
    let families = declared_test_families(&docs);
    let mut read = 0usize;
    let mut unknown: Vec<String> = Vec::new();
    for doc in &docs {
        for (n, line) in doc.lines.iter().enumerate() {
            for cap in RE_TEST_FAMILY_READ.captures_iter(line) {
                read += 1;
                let guard = format!("SCE_TEST_FAMILY_{}", &cap[1]);
                if !families.contains(&guard) {
                    unknown.push(format!("{}:{}  {guard}", doc.path, n + 1));
                }
            }
        }
    }
    assert!(
        read >= families.len(),
        "only {read} read(s) of a family guard for {} families; the scan stopped \
         matching",
        families.len(),
    );
    assert!(
        unknown.is_empty(),
        "{} guard(s) name a family cmake/SCETestFamilies.cmake does not declare:\n  {}",
        unknown.len(),
        unknown.join("\n  "),
    );
}

/// What the family list resolves to, asked of CMake: with no list every
/// family is ON, with a list exactly the families it names, and a name that
/// is no family stops the configure. A tiny project that includes only the
/// module keeps each configure to about a second.
#[test]
fn a_family_list_registers_the_families_it_names() {
    if std::process::Command::new("cmake")
        .arg("--version")
        .output()
        .map(|o| !o.status.success())
        .unwrap_or(true)
    {
        eprintln!("SKIP: cmake not on PATH");
        return;
    }
    let families = declared_test_families(&documents());
    let sandbox = std::env::temp_dir().join(format!("sce-test-families-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&sandbox);
    std::fs::create_dir_all(&sandbox).expect("create sandbox");
    // `project(... NONE)`: no compiler probe, so this does not depend on a
    // toolchain being installed.
    let probe = r#"cmake_minimum_required(VERSION 3.20)
project(sce_test_family_probe NONE)
include("@ROOT@/cmake/SCETestFamilies.cmake")
get_property(_names GLOBAL PROPERTY _SCE_TEST_FAMILY_NAMES)
foreach(_name IN LISTS _names)
    string(TOUPPER "${_name}" _upper)
    message(STATUS "FAMILY SCE_TEST_FAMILY_${_upper}=${SCE_TEST_FAMILY_${_upper}}")
endforeach()
"#
    .replace("@ROOT@", &repo_root().display().to_string());
    std::fs::write(sandbox.join("CMakeLists.txt"), probe).expect("write probe CMakeLists");

    // Each list in a build directory of its own: the list is a cache entry,
    // and a second configure of one directory would read the first's.
    let configure = |case: &str, list: Option<&str>| -> (bool, String) {
        let mut cmd = std::process::Command::new("cmake");
        cmd.arg("-S")
            .arg(&sandbox)
            .arg("-B")
            .arg(sandbox.join(format!("build-{case}")));
        if let Some(list) = list {
            cmd.arg(format!("-DSCE_TEST_FAMILIES={list}"));
        }
        let out = cmd.output().expect("cmake runs");
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        (out.status.success(), text)
    };
    let resolved = |text: &str| -> BTreeMap<String, String> {
        text.lines()
            .filter_map(|line| line.split_once("FAMILY "))
            .filter_map(|(_, rest)| rest.split_once('='))
            .map(|(guard, value)| (guard.to_string(), value.trim().to_string()))
            .collect()
    };
    let on = |values: &BTreeMap<String, String>| -> BTreeSet<String> {
        values
            .iter()
            .filter(|(_, value)| value.as_str() == "ON")
            .map(|(guard, _)| guard.clone())
            .collect()
    };

    let (ok, every) = configure("unset", None);
    assert!(ok, "the probe did not configure with no list:\n{every}");
    let every = resolved(&every);
    assert_eq!(
        every.keys().cloned().collect::<BTreeSet<String>>(),
        families,
        "the families the module resolves are not the ones it declares"
    );
    assert_eq!(
        on(&every),
        families,
        "with no SCE_TEST_FAMILIES every family must be ON, or the default tree \
         registers less than it did: {every:?}"
    );

    let (ok, one) = configure("one", Some("standard_host"));
    assert!(ok, "the probe did not configure with one family:\n{one}");
    assert_eq!(
        on(&resolved(&one)),
        BTreeSet::from(["SCE_TEST_FAMILY_STANDARD_HOST".to_string()]),
        "a list of one family must turn on that family alone"
    );

    let (ok, two) = configure("two", Some("w3c;c11"));
    assert!(ok, "the probe did not configure with two families:\n{two}");
    assert_eq!(
        on(&resolved(&two)),
        BTreeSet::from([
            "SCE_TEST_FAMILY_C11".to_string(),
            "SCE_TEST_FAMILY_W3C".to_string(),
        ]),
        "a list of two families must turn on those two alone"
    );

    let (ok, unknown) = configure("unknown", Some("w3c;no_such_family"));
    let _ = std::fs::remove_dir_all(&sandbox);
    assert!(
        !ok && unknown.contains("no_such_family"),
        "a list naming no family must stop the configure and name it:\n{unknown}"
    );
}

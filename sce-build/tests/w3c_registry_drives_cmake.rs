// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! `tests/CMakeLists.txt` derives its W3C registrations from the
//! conformance registry rather than spelling them.
//!
//! The registrations were 202 hand-written
//! `sce_generate_static_w3c_test(144 ...)` lines, and `generate-w3c`
//! read them back out with a regex — CMake was the registry, and a
//! checkout without CMake could not enumerate the suite. Now
//! `tests/w3c/conformance/fixtures.json` is the source and CMake asks
//! `list-fixtures` for the ids, the way every forge conformance harness
//! already consumes its own catalog.
//!
//! What this pins is that the derivation is real. A literal id creeping
//! back into a registration is the failure that matters: it would build
//! and run, so nothing else would notice, and the set would once again
//! be stated in two places that can disagree. The configure-time
//! dependency is pinned for the same reason — without it, editing the
//! registry leaves a configured tree building the previous set, and the
//! stale result looks like a correct one.

use std::path::{Path, PathBuf};

use sce_build::w3c_registry::{W3cRegistry, W3C_REGISTRY_RELATIVE_PATH};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

fn cmake_text() -> String {
    std::fs::read_to_string(repo_root().join("tests/CMakeLists.txt"))
        .expect("read tests/CMakeLists.txt")
}

/// Registration calls, paired with the argument naming the test.
///
/// Comment lines are skipped: the file documents the call shape in
/// prose, and a documented example is not a registration.
fn registration_arguments(cmake: &str) -> Vec<String> {
    let re = regex::Regex::new(r"sce_generate_static_w3c_test\(([^\s)]+)").expect("pattern");
    cmake
        .lines()
        .filter(|l| !l.trim_start().starts_with('#'))
        .filter_map(|l| re.captures(l))
        .map(|c| c[1].to_string())
        .collect()
}

#[test]
fn every_w3c_registration_takes_its_id_from_the_registry() {
    let cmake = cmake_text();
    let args = registration_arguments(&cmake);

    assert!(
        !args.is_empty(),
        "found no sce_generate_static_w3c_test call at all; the pattern is \
         broken, not the build file — a clean result would prove nothing",
    );

    // Every call must name a variable. `${_W3C_ID}` is the derived form;
    // a bare `144` is the hand-written one this replaced.
    let literal: Vec<&String> = args.iter().filter(|a| !a.starts_with("${")).collect();
    assert!(
        literal.is_empty(),
        "tests/CMakeLists.txt registers W3C test(s) by literal id: {literal:?}. \
         The registry at {W3C_REGISTRY_RELATIVE_PATH} is the source of truth — a \
         literal here restates the set in a second place, and the two can then \
         disagree without anything failing.",
    );
}

#[test]
fn the_build_file_reads_the_registry_through_list_fixtures() {
    let cmake = cmake_text();

    assert!(
        cmake.contains(W3C_REGISTRY_RELATIVE_PATH),
        "tests/CMakeLists.txt never names {W3C_REGISTRY_RELATIVE_PATH}, so its \
         registrations cannot be coming from the registry",
    );
    assert!(
        cmake.contains("list-fixtures") && cmake.contains("--catalog w3c"),
        "tests/CMakeLists.txt does not enumerate the registry through \
         `list-fixtures --catalog w3c`; a hand-rolled JSON read in CMake would \
         be a second parser of the same file",
    );
    assert!(
        cmake.contains("CMAKE_CONFIGURE_DEPENDS"),
        "tests/CMakeLists.txt does not depend on the registry at configure \
         time, so editing it would leave a configured tree building the \
         previous set of tests",
    );
}

/// Every harness the registry declares is one the build file can lower.
///
/// The loop maps a harness onto the registration function's `TYPE`
/// argument by upper-casing it, with `simple` meaning "no TYPE". A
/// harness added to the registry and not understood here would fail at
/// configure time on a real build — this says so at test time instead,
/// and names the harness.
#[test]
fn the_build_file_understands_every_declared_harness() {
    let registry =
        W3cRegistry::load(&repo_root().join(W3C_REGISTRY_RELATIVE_PATH)).expect("registry loads");
    let cmake = cmake_text();

    let declared: Vec<&str> = registry.harnesses.keys().map(String::as_str).collect();
    assert!(
        declared.len() >= 2,
        "only {} harness(es) declared; this assertion would be near-vacuous",
        declared.len(),
    );

    // Read the loop's own list rather than searching the file: `http`
    // and `simple` both occur in unrelated prose here, so a substring
    // match over 8000 lines would pass whatever the loop iterates.
    let re = regex::Regex::new(r"set\(_W3C_HARNESSES\s+([^)]*)\)").expect("pattern");
    let caps = re
        .captures(&cmake)
        .expect("tests/CMakeLists.txt must declare _W3C_HARNESSES for the loop to iterate");
    let iterated: Vec<&str> = caps[1].split_whitespace().collect();
    assert!(
        !iterated.is_empty(),
        "_W3C_HARNESSES is empty, so the loop registers nothing",
    );

    let unmapped: Vec<&&str> = declared
        .iter()
        .filter(|h| !iterated.contains(&h.to_string().as_str()))
        .collect();
    assert!(
        unmapped.is_empty(),
        "the registry declares harness(es) the CMake loop does not iterate, so \
         every fixture naming one would go unregistered: {unmapped:?} \
         (loop iterates {iterated:?})",
    );
}

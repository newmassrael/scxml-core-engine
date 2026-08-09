// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The conformance registry and the CMake registrations describe the
//! same set of W3C tests.
//!
//! `tests/w3c/conformance/fixtures.json` is what `sce-codegen
//! generate-w3c` reads and what `sce-codegen list-fixtures --catalog
//! w3c` enumerates, so the generated trees follow it. `tests/CMakeLists.txt`
//! still spells its own `sce_generate_static_w3c_test(...)` calls, which
//! is what schedules the C++ AOT targets. Until CMake derives those
//! calls from the registry — it can, via `list-fixtures --format cmake`,
//! the way every forge conformance harness already does — the two are
//! separate statements of one fact, and this gate is what stops them
//! becoming two different facts.
//!
//! Checked in both directions on purpose. A registry entry with no CMake
//! call is a test the C++ AOT suite silently stops building; a CMake call
//! with no registry entry is a test `generate-w3c` silently stops
//! emitting a state machine for, which surfaces as a missing-header
//! build error far from its cause. Harness agreement is checked too:
//! the harness decides the runner's completion budget and whether the
//! HTTP fixture endpoint has to be up, so a fixture that disagreed
//! across the two would run under one set of rules and be scheduled
//! under another.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use sce_build::w3c_registry::{W3cRegistry, DEFAULT_HARNESS, W3C_REGISTRY_RELATIVE_PATH};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

/// Lower bound on the registrations this gate must observe.
///
/// A regex that stopped matching would otherwise report perfect
/// agreement between two empty sets.
const MIN_REGISTRATIONS: usize = 150;

/// Registrations spelled in `tests/CMakeLists.txt`, keyed by test id.
///
/// The pattern is the one the generator itself used while CMake was the
/// registry, so this reads exactly what that code read — a gate that
/// parsed the calls differently could agree with the registry while the
/// old producer disagreed.
fn cmake_registrations(cmake: &str) -> BTreeMap<String, String> {
    let re = regex::Regex::new(
        r"sce_generate_static_w3c_test\((\S+)\s+\$\{STATIC_W3C_OUTPUT_DIR\}(?:\s+TYPE\s+(\w+))?\)",
    )
    .expect("registration pattern compiles");
    let mut out = BTreeMap::new();
    for line in cmake.lines() {
        // A commented-out registration is not a registration.
        if line.trim_start().starts_with('#') {
            continue;
        }
        if let Some(caps) = re.captures(line) {
            let id = caps[1].to_string();
            let harness = caps
                .get(2)
                .map(|m| m.as_str().to_ascii_lowercase())
                .unwrap_or_else(|| DEFAULT_HARNESS.to_string());
            out.insert(id, harness);
        }
    }
    out
}

#[test]
fn the_registry_and_the_cmake_registrations_describe_the_same_tests() {
    let root = repo_root();
    let registry =
        W3cRegistry::load(&root.join(W3C_REGISTRY_RELATIVE_PATH)).expect("registry loads");
    let cmake_text = std::fs::read_to_string(root.join("tests/CMakeLists.txt"))
        .expect("read tests/CMakeLists.txt");
    let cmake = cmake_registrations(&cmake_text);

    assert!(
        cmake.len() >= MIN_REGISTRATIONS,
        "read only {} registration(s) out of tests/CMakeLists.txt; the pattern \
         is broken, not the registry — agreement between two nearly-empty sets \
         would prove nothing",
        cmake.len(),
    );
    assert!(
        registry.fixtures().len() >= MIN_REGISTRATIONS,
        "the registry holds only {} fixture(s); expected at least \
         {MIN_REGISTRATIONS}",
        registry.fixtures().len(),
    );

    let declared: BTreeMap<String, String> = registry
        .fixtures()
        .iter()
        .map(|f| (f.id.clone(), f.harness.clone()))
        .collect();

    let missing_from_cmake: Vec<&String> = declared
        .keys()
        .filter(|id| !cmake.contains_key(*id))
        .collect();
    assert!(
        missing_from_cmake.is_empty(),
        "the registry declares test(s) that tests/CMakeLists.txt never registers, \
         so the C++ AOT suite does not build them: {missing_from_cmake:?}",
    );

    let missing_from_registry: Vec<&String> = cmake
        .keys()
        .filter(|id| !declared.contains_key(*id))
        .collect();
    assert!(
        missing_from_registry.is_empty(),
        "tests/CMakeLists.txt registers test(s) the registry does not declare, so \
         `generate-w3c` emits no state machine for them: {missing_from_registry:?}",
    );

    let disagreements: Vec<String> = declared
        .iter()
        .filter_map(|(id, harness)| {
            let cmake_harness = cmake.get(id)?;
            (cmake_harness != harness)
                .then(|| format!("  {id}: registry says `{harness}`, CMake says `{cmake_harness}`"))
        })
        .collect();
    assert!(
        disagreements.is_empty(),
        "the registry and tests/CMakeLists.txt disagree about which harness a test \
         needs; the harness decides the completion budget and whether the HTTP \
         fixture endpoint must be up:\n{}",
        disagreements.join("\n"),
    );
}

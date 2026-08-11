// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A fixture under `integration_resources/` is asserted in every channel it
//! is generated into.
//!
//! Adding a stem there fans it out to seven channels — C++ Interpreter, C++
//! AOT, Rust, Go, Python, Kotlin, C11 — and the sites that GENERATE each one
//! are loud when they are missing: `regen_all_committed_trees.sh` exits
//! non-zero without the four regen scripts, `rust-modrs-drift` blocks a push
//! without the `pub mod`, and CMake fails to build without its registration.
//!
//! The sites that ASSERT are silent. A stem can land with generated code in
//! all seven channels and a test driver in two, and every gate stays green
//! because generated code that nobody runs still compiles. Measured
//! 2026-08-11: `event_origin_is_a_location` had exactly that shape — it was
//! added to prove `_event.origin` is an address, and five of the seven
//! engines were never asked the question. All five were violating the clause.
//!
//! So this is not a registration checklist for its own sake. Every entry
//! below is a channel that runs the fixture, and its absence is a claim of
//! coverage the repository does not have. Violations are collected rather
//! than reported one at a time: a stem that is missing four drivers should
//! say so once, not across four pushes.

use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

/// `send_param_payload` -> `SendParamPayload`, the spelling the C++ and
/// Kotlin drivers are named with.
fn upper_camel(stem: &str) -> String {
    stem.split('_')
        .filter(|w| !w.is_empty())
        .map(|w| {
            let mut cs = w.chars();
            match cs.next() {
                Some(c) => c.to_uppercase().chain(cs).collect::<String>(),
                None => String::new(),
            }
        })
        .collect()
}

fn stems(root: &Path) -> Vec<String> {
    let mut found: Vec<String> = std::fs::read_dir(root.join("integration_resources"))
        .expect("read integration_resources/")
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();
    found.sort();
    found
}

fn read(root: &Path, rel: &str) -> String {
    std::fs::read_to_string(root.join(rel)).unwrap_or_else(|e| panic!("read {rel}: {e}"))
}

/// One required site, as the author would have to satisfy it.
struct Site {
    /// What the site is for, in the failure message.
    channel: &'static str,
    /// Path that must exist, or the file a marker must appear in.
    path: String,
    /// When set, the file must additionally contain this text.
    marker: Option<String>,
}

fn required_sites(stem: &str) -> Vec<Site> {
    let camel = upper_camel(stem);
    let file = |channel, path: String| Site {
        channel,
        path,
        marker: None,
    };
    let marker = |channel, path: &str, marker: String| Site {
        channel,
        path: path.to_string(),
        marker: Some(marker),
    };

    vec![
        // The fixture itself: the canonical document every channel compiles.
        file(
            "canonical fixture",
            format!("integration_resources/{stem}/{stem}.scxml"),
        ),
        // Generation. Loud when missing, but listed so one report covers the
        // whole contract instead of the author meeting it in instalments.
        file("regen (rust)", format!("scripts/regen_{stem}.sh")),
        file("regen (go)", format!("scripts/regen_{stem}_go.sh")),
        file("regen (kotlin)", format!("scripts/regen_{stem}_kotlin.sh")),
        file("regen (python)", format!("scripts/regen_{stem}_python.sh")),
        marker(
            "rust module tree",
            "backends/rust/tests/src/integration/mod.rs",
            format!("pub mod {stem};"),
        ),
        marker(
            "C++ AOT codegen registration",
            "tests/CMakeLists.txt",
            format!("sce_generate_static_integration_test({stem}"),
        ),
        marker(
            "C11 codegen registration",
            "backends/c/tests/CMakeLists.txt",
            format!("sce_generate_static_integration_c_test({stem}"),
        ),
        // Assertion. These are the silent ones.
        file(
            "Rust AOT driver",
            format!("backends/rust/tests/tests/{stem}.rs"),
        ),
        file(
            "Go AOT driver",
            format!("backends/go/tests/integration/{stem}/{stem}_test.go"),
        ),
        file(
            "Python AOT driver",
            format!("backends/python/tests/integration/{stem}/test_{stem}_aot.py"),
        ),
        file(
            "Kotlin AOT driver",
            format!("backends/kotlin/tests/src/test/kotlin/com/sce/integration/{camel}Test.kt"),
        ),
        file(
            "C11 AOT driver",
            format!("backends/c/tests/integration/test_{stem}.c"),
        ),
        file(
            "C++ Interpreter driver",
            format!("tests/integration/{camel}Test.cpp"),
        ),
        file(
            "C++ AOT driver",
            format!("tests/integration/{camel}AotTest.cpp"),
        ),
        // A driver CMake does not compile is a file, not a test.
        marker(
            "C++ driver compiled",
            "tests/CMakeLists.txt",
            format!("integration/{camel}Test.cpp"),
        ),
        marker(
            "C++ AOT driver compiled",
            "tests/CMakeLists.txt",
            format!("integration/{camel}AotTest.cpp"),
        ),
        marker(
            "C11 driver compiled",
            "backends/c/tests/CMakeLists.txt",
            format!("integration/test_{stem}.c"),
        ),
        // The layout doc records what axis each fixture owns, and "fixtures
        // stay on one axis" is the rule it states. A stem with no entry there
        // is an axis nobody chose — and the enumeration this replaced was
        // wrong within one round of a stem being added, which is why the
        // requirement is a per-stem mention rather than a list to maintain.
        marker(
            "layout doc entry",
            "docs/SCE_INTEGRATION_FIXTURE_LAYOUT.md",
            stem.to_string(),
        ),
    ]
}

#[test]
fn every_integration_stem_is_asserted_in_every_channel_it_is_generated_into() {
    let root = repo_root();
    let stems = stems(&root);

    // A scan that silently found nothing would report a clean tree. The
    // count is a floor, not the current total: it may only grow.
    assert!(
        stems.len() >= 12,
        "found {} stem(s) under integration_resources/; the directory scan is \
         broken, not the tree — twelve stems were committed when this floor \
         was set, and a shrinking scan cannot come back as OK",
        stems.len(),
    );

    let mut missing: Vec<String> = Vec::new();
    for stem in &stems {
        for site in required_sites(stem) {
            match &site.marker {
                None => {
                    if !root.join(&site.path).exists() {
                        missing.push(format!(
                            "{stem}: {} — {} is absent",
                            site.channel, site.path
                        ));
                    }
                }
                Some(needle) => {
                    if !read(&root, &site.path).contains(needle.as_str()) {
                        missing.push(format!(
                            "{stem}: {} — {} does not contain `{needle}`",
                            site.channel, site.path
                        ));
                    }
                }
            }
        }
    }

    assert!(
        missing.is_empty(),
        "an integration fixture is generated into channels that never run it \
         ({} site(s)):\n  {}\n\n\
         Each line is a channel whose engine compiles this fixture and is \
         never asked whether it behaves. That is the shape that let five \
         backends violate W3C SCXML C.1 while every gate stayed green: \
         generated code nobody runs still compiles, so only a driver turns \
         the fixture into coverage. Add the missing driver — or, if the \
         channel genuinely cannot host this fixture, that is a decision to \
         state here with its reason, not to leave as an absence.",
        missing.len(),
        missing.join("\n  "),
    );
}

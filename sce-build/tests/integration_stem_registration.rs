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

use regex::Regex;

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

/// The driver files a stem is asserted in, for the channels that have one.
///
/// A subset of `required_sites` — only the per-channel drivers, and only
/// the ones present, since the test above is what reports an absent one.
fn existing_driver_paths(root: &Path, stem: &str) -> Vec<String> {
    required_sites(stem)
        .into_iter()
        .filter(|s| s.marker.is_none() && s.channel.contains("driver"))
        .map(|s| s.path)
        .filter(|p| root.join(p).is_file())
        .collect()
}

/// Every `.scxml` path a driver's comments name is a path that is there.
///
/// A driver's header says which document it compiles, and that sentence is
/// the only place a reader learns where to edit the machine. When it is
/// wrong the reader is sent to a directory that does not exist, and
/// nothing says so: comments are not compiled, so the drift is invisible
/// to every other gate in this repository.
///
/// This has now been wrong twice for one stem. `backends/rust/tests/
/// fixtures/` was never a real directory — the canonical documents live
/// under `integration_resources/<stem>/`. A round in 2026-08 corrected the
/// copy in `src/integration/mod.rs` and left the copy in the driver, which
/// is the ordinary outcome of fixing a duplicated claim by hand: the
/// second copy is found by whoever trips over it next.
///
/// Scoped to `.scxml` deliberately. Every path-shaped token would flag
/// prose that names a path to say it is *gone* — a legitimate sentence
/// this repository writes often — and a gate that cries wolf gets an
/// exemption table instead of obedience. A fixture citation is a narrow,
/// checkable claim: this document, here.
#[test]
fn every_fixture_a_driver_cites_is_a_document_that_exists() {
    let root = repo_root();
    let stems = stems(&root);
    let scxml = Regex::new(r"[A-Za-z0-9_./-]+\.scxml").expect("static regex");

    let mut cited = 0usize;
    let mut broken: Vec<String> = Vec::new();

    for stem in &stems {
        for rel in existing_driver_paths(&root, stem) {
            for (n, line) in read(&root, &rel).lines().enumerate() {
                let trimmed = line.trim_start();
                // Comment leaders across the five driver languages. A
                // citation in live code is a path the compiler or the
                // runtime already resolves; only prose can rot unnoticed.
                if !(trimmed.starts_with("//")
                    || trimmed.starts_with('#')
                    || trimmed.starts_with('*'))
                {
                    continue;
                }
                for hit in scxml.find_iter(line) {
                    let path = hit.as_str();
                    // A bare filename names the document beside the text,
                    // not a location in the tree, so there is nothing to
                    // resolve it against.
                    if !path.contains('/') {
                        continue;
                    }
                    cited += 1;
                    if !root.join(path).exists() {
                        broken.push(format!("{rel}:{}: {path}", n + 1));
                    }
                }
            }
        }
    }

    // A floor, because the interesting failure is a scan that reads
    // nothing: no citations found reports a clean tree just as loudly as
    // every citation resolving. Eighty-nine were cited when this was set.
    assert!(
        cited >= 60,
        "found only {cited} fixture citation(s) across the integration \
         drivers; the comment scan or the driver enumeration is broken, not \
         the tree"
    );

    assert!(
        broken.is_empty(),
        "a driver names a fixture that is not there ({} citation(s)):\n  {}\n\n\
         The canonical document for a stem is \
         `integration_resources/<stem>/<stem>.scxml`. Point the comment at \
         it — a header that names the wrong location is the one sentence a \
         reader trusts to find the machine.",
        broken.len(),
        broken.join("\n  "),
    );
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

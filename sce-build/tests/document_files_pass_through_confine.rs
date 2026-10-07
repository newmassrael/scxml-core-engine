// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! Every place that opens a file a document names goes through `confine`.
//!
//! # What is held
//!
//! A caller can confine the generator to one folder (`SCE_FILE_ROOT`), so that a document handed
//! over by someone who does not own the machine cannot make it open a file elsewhere, or tell
//! that one is there. That holds for the places that were found, and a place added later that
//! opens a file with `std::fs` directly would be a way round it that nothing noticed: a document
//! would reach it and the folder would not.
//!
//! So the modules that read documents may not open or look for a file except through
//! `confine::{exists, read_to_string}`, and a place that does is listed here, with what it opens,
//! and why that is not a file a document names. A new one fails this test, and a listed one that
//! is gone does too, so that the list is no longer than what is true.
//!
//! The test reads the sources, which is what the rule is about: where a file is opened is a
//! fact of the text.

use std::fs;
use std::path::Path;

/// The modules that read a document or a file one names, and the part of each before its tests.
const MODULES: [&str; 6] = [
    "src/parser.rs",
    "src/resolve.rs",
    "src/xinclude.rs",
    "src/template.rs",
    "src/forge/import_source.rs",
    "src/lib.rs",
];

/// How a file is opened or looked for, past `confine`.
const RAW: [&str; 7] = [
    "fs::read_to_string(",
    "fs::read(",
    "File::open(",
    ".exists()",
    ".is_file()",
    ".is_dir()",
    "fs::read_dir(",
];

/// The places that open a file without `confine`: the module, the line, and why it names no
/// file of a document.
const ALLOWED: [(&str, &str, &str); 5] = [
    (
        "src/lib.rs",
        "let content = match std::fs::read_to_string(&peer_scxml_path) {",
        "the other machines of a deployment, from `deploy.yaml`: the operator's file, not a document's",
    ),
    (
        "src/lib.rs",
        "if p.join(\"state_machine.rs.jinja2\").exists() || p.join(\"state_machine.kt.jinja2\").exists()",
        "where the generator's own templates are: its installation, not a document's",
    ),
    (
        "src/lib.rs",
        "if candidate.exists() {",
        "where the generator's own templates are: its installation, not a document's",
    ),
    // The same line stands twice in `lib.rs` (the peers of a deployment are read in two places),
    // and the templates are looked for in two: each is listed as often as it occurs.
    (
        "src/lib.rs",
        "let content = match std::fs::read_to_string(&peer_scxml_path) {",
        "the other machines of a deployment, from `deploy.yaml`: the operator's file, not a document's",
    ),
    (
        "src/lib.rs",
        "if candidate.exists() {",
        "where the generator's own templates are: its installation, not a document's",
    ),
];

/// What a module holds before its tests, which have their own files to open.
fn before_its_tests(source: &str) -> &str {
    match source.find("\nmod tests") {
        Some(at) => &source[..at],
        None => source,
    }
}

#[test]
fn a_place_that_opens_a_file_goes_through_confine_unless_it_is_listed_as_not_a_documents() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut unlisted = Vec::new();
    let mut seen = vec![0usize; ALLOWED.len()];

    for module in MODULES {
        let source = fs::read_to_string(root.join(module)).expect("a module of the generator");
        for (row, line) in before_its_tests(&source).lines().enumerate() {
            let text = line.trim();
            if text.starts_with("//") || !RAW.iter().any(|raw| text.contains(raw)) {
                continue;
            }
            // Each listed entry covers one occurrence: a second one is a new place.
            let covered = ALLOWED
                .iter()
                .enumerate()
                .find(|(at, (file, line, _))| *file == module && *line == text && seen[*at] == 0);
            match covered {
                Some((at, _)) => seen[at] += 1,
                None => unlisted.push(format!("{module}:{}: {text}", row + 1)),
            }
        }
    }

    assert!(
        unlisted.is_empty(),
        "a file is opened without `confine`, so a document that names it is not held to the \
         folder a caller confined the generator to. Open it with `confine::exists` / \
         `confine::read_to_string`, or, when it is no file of a document's, list it in this test \
         with why:\n{}",
        unlisted.join("\n")
    );
    let gone: Vec<_> = ALLOWED
        .iter()
        .zip(&seen)
        .filter(|(_, count)| **count == 0)
        .map(|((file, line, _), _)| format!("{file}: {line}"))
        .collect();
    assert!(
        gone.is_empty(),
        "listed as a place that opens a file without `confine`, and no longer there: remove it \
         from the list so that the list is no longer than what is true:\n{}",
        gone.join("\n")
    );
}

#[test]
fn every_module_the_rule_names_is_there_and_has_a_part_before_its_tests() {
    // A renamed module would be skipped by a test that passes on what it does not find.
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    for module in MODULES {
        let source = fs::read_to_string(root.join(module))
            .unwrap_or_else(|e| panic!("{module} is not where this test looks for it: {e}"));
        assert!(
            before_its_tests(&source).lines().count() > 50,
            "{module} reads as a few lines: the part before its tests was cut in the wrong place"
        );
    }
}

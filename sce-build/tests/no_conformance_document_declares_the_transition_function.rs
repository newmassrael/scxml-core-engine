// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The CMake route of the conformance suite reaches no document that declares
//! the transition function (SCE Accepted Subset 2.21,
//! docs/SCE_TRANSITION_FUNCTION.md).
//!
//! `cmake/SCEStaticW3CTest.cmake` declares by hand the files a C++ or C11
//! fixture's generation writes — the machine, its children and their headers —
//! where the integration route asks `sce-codegen --plan generate`. A fixture
//! declaring the function would write `<stem>_transition_function.h` beside
//! them, undeclared, and its depfile would name an output no rule owns. That
//! route reads the documents under `resources/` and nothing else, so it is
//! complete exactly while none of them declares the function.
//!
//! This is a target of its own because of what it reads. The document that
//! would break the route is one an author adds under `resources/` next, and
//! the lane running the function's own suite filters on paths that do not
//! name `resources/`, so the check rides the workflow that filters on none
//! (`UNFILTERABLE_GATES` in `workflow_trigger_coverage.rs`). The function's
//! suite compiles every backend; this reads files, and is the part of it that
//! has to run on every push.

mod common;

/// No document under `resources/` declares the function: a fixture that comes
/// to declare one has to be declared in `cmake/SCEStaticW3CTest.cmake` first.
#[test]
fn no_document_the_conformance_cmake_route_reads_declares_the_function() {
    let root = common::repository::root();
    let documents: Vec<String> = common::repository::paths_git_holds(&["resources"])
        .into_iter()
        .filter(|path| path.ends_with(".scxml") || path.ends_with(".txml"))
        .collect();
    // A scan that found nothing would pass for the wrong reason; the
    // conformance corpus holds a few hundred documents.
    assert!(
        documents.len() >= 200,
        "found {} conformance documents under resources/; the scan is broken, not the tree",
        documents.len()
    );
    // Read as bytes: the corpus holds documents that are not UTF-8 on purpose
    // (test513.txml), and the attribute's name is ASCII in any encoding the
    // corpus uses.
    let needle = b"sce:transition-function";
    let declaring: Vec<&String> = documents
        .iter()
        .filter(|path| {
            let at = root.join(path);
            std::fs::read(&at)
                .unwrap_or_else(|e| panic!("read {}: {e}", at.display()))
                .windows(needle.len())
                .any(|window| window == needle)
        })
        .collect();
    assert!(
        declaring.is_empty(),
        "conformance documents declare the transition function, whose header \
         cmake/SCEStaticW3CTest.cmake does not declare as an output: {declaring:?}"
    );
}

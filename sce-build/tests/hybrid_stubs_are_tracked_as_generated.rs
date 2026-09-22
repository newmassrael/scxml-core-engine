// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! §scxml-6.4 — every hybrid `<invoke>` in the W3C corpus has its stub child
//! tracked beside it, and each tracked stub is what code generation writes.
//!
//! The W3C harnesses (C++ and C11) find a hybrid invoke's stub by globbing the
//! resource directory at configure time, so the stub has to be there before
//! any code is generated. Code generation used to keep it there by writing
//! into the source tree on every C11 run. It now writes its output where
//! output goes, and this test is what keeps the tracked copies true:
//!
//! * a hybrid invoke with no tracked stub would configure with no child and
//!   fail to link, on the first build of a fresh clone;
//! * a tracked stub that differs from `HybridInvokeInfo::stub_document` is a
//!   child the build compiles and the generator no longer describes;
//! * a tracked stub no invoke asks for is a document nothing reads.
//!
//! `UPDATE_EXPECT=1` writes the stubs the corpus asks for.

mod common;

use std::collections::BTreeMap;
use std::path::Path;

use sce_build::parser::SCXMLParser;

/// Every stub the corpus asks for, keyed by repository-relative path.
fn expected_stubs() -> BTreeMap<String, String> {
    let root = common::repository::root();
    let mut expected = BTreeMap::new();
    for rel in common::repository::paths_git_tracks(&["resources/*/*.scxml"]) {
        let path = root.join(&rel);
        let Some(path_str) = path.to_str() else {
            continue;
        };
        // A document the parser refuses has no invokes to ask about; the
        // harness refuses it too, so it cannot be a child's parent here.
        let Ok(model) = SCXMLParser::new().parse_file(path_str) else {
            continue;
        };
        let dir = Path::new(&rel)
            .parent()
            .expect("a resource sits in a directory");
        for invoke in model.iter_hybrid_invokes() {
            if let Some(stub) = invoke.stub_document() {
                let stub_rel = dir.join(format!("{}.scxml", invoke.child_name));
                expected.insert(stub_rel.to_string_lossy().into_owned(), stub);
            }
        }
    }
    expected
}

#[test]
fn hybrid_stubs_are_tracked_as_generated() {
    let root = common::repository::root();
    let expected = expected_stubs();
    // The corpus holds two hybrid invokes today (test216, test530). A sweep
    // that found none would pass while asserting nothing.
    assert!(
        !expected.is_empty(),
        "no hybrid invoke found in resources/ — the sweep is broken, not the corpus"
    );

    if std::env::var_os("UPDATE_EXPECT").is_some() {
        for (rel, stub) in &expected {
            std::fs::write(root.join(rel), stub).unwrap_or_else(|e| panic!("write {rel}: {e}"));
        }
        return;
    }

    let tracked: Vec<String> =
        common::repository::paths_git_tracks(&["resources/*/*_hybrid*.scxml"]);
    let mut problems = Vec::new();
    for (rel, stub) in &expected {
        if !tracked.contains(rel) {
            problems.push(format!(
                "{rel}: a hybrid invoke asks for this stub and git does not track it"
            ));
            continue;
        }
        let on_disk =
            std::fs::read_to_string(root.join(rel)).unwrap_or_else(|e| panic!("read {rel}: {e}"));
        if &on_disk != stub {
            problems.push(format!(
                "{rel}: differs from what code generation writes:\n--- tracked\n{on_disk}+++ generated\n{stub}"
            ));
        }
    }
    for rel in &tracked {
        if !expected.contains_key(rel) {
            problems.push(format!("{rel}: tracked, and no hybrid invoke asks for it"));
        }
    }
    assert!(
        problems.is_empty(),
        "{}\n\nRegenerate with UPDATE_EXPECT=1 cargo test -p sce-build --test \
         hybrid_stubs_are_tracked_as_generated, then review the diff.",
        problems.join("\n")
    );
}

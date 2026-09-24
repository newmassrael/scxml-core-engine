// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! An SCE element written where no reader looks for it is refused, in every
//! forge kind, rather than dropped.
//!
//! # What was wrong
//!
//! Each kind parser reaches its elements by asking a parent for a child by
//! name, and the `<scxml>` root admits SCE elements laxly because one root
//! serves seventeen kinds. So an element written anywhere else was never
//! read and never refused: a `<sce:helper>` under a procedure's root rather
//! than its `<datamodel>` vanished, and the refusal that followed named the
//! call that used it (measured 2026-09-24).
//!
//! # What is held
//!
//! Every forge document under `tests/forge/resources` that parses is
//! parsed again with a `<sce:helper>` — a real SCE element, valid where a
//! procedure's `<datamodel>` holds it — written as the root's first child,
//! where no kind reads one. Each must be refused as an unexpected child on
//! the row that holds it. The documents are the population, so a kind is
//! covered by having a document, not by a list here.

use std::collections::BTreeSet;
use std::path::Path;

use sce_build::forge::diagnostic::ToDiagnostics;
use sce_build::DocumentLabel;

const MISPLACED: &str =
    r#"<sce:helper name="misplaced" args="bytes" returns="bytes" sce:returns-max-size="64"/>"#;

fn parse(text: &str, stem: &str) -> Result<Option<String>, (String, Option<u32>)> {
    let label = DocumentLabel {
        identifier: stem,
        diagnostic_label: stem,
    };
    match sce_build::forge::parser::parse_forge_with_imports(text, label) {
        Ok(parsed) => Ok(parsed.map(|p| format!("{:?}", p.document.kind()))),
        Err(refusal) => {
            let code = serde_json::to_string(&refusal.error.to_diagnostics()[0].code)
                .expect("a code serialises");
            Err((code, refusal.location.line))
        }
    }
}

/// `text` with `MISPLACED` on a row of its own right after the root's start
/// tag, and that row's number.
fn with_misplaced_child(text: &str) -> (String, u32) {
    let doc = roxmltree::Document::parse(text).expect("the document parses");
    let root = doc.root_element();
    let first_child = root
        .children()
        .next()
        .expect("a forge root has children")
        .range()
        .start;
    let mut out = String::with_capacity(text.len() + MISPLACED.len() + 4);
    out.push_str(&text[..first_child]);
    out.push_str("\n  ");
    out.push_str(MISPLACED);
    out.push_str(&text[first_child..]);
    let row = text[..first_child].matches('\n').count() as u32 + 2;
    (out, row)
}

#[test]
fn an_sce_element_no_reader_asks_for_is_refused_in_every_kind() {
    let resources = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/forge/resources");
    let mut paths: Vec<_> = std::fs::read_dir(&resources)
        .expect("the forge resources directory")
        .map(|entry| entry.expect("a directory entry").path())
        .filter(|path| path.extension().is_some_and(|e| e == "scxml"))
        .collect();
    paths.sort();

    let mut kinds = BTreeSet::new();
    let mut judged = 0usize;
    let mut wrong = Vec::new();
    for path in &paths {
        let text = std::fs::read_to_string(path).expect("a resource reads");
        let stem = path.file_stem().unwrap().to_str().unwrap();
        let Ok(Some(kind)) = parse(&text, stem) else {
            // Not a forge document that parses on its own — one that needs
            // its imports resolved, or a refusal fixture. Not this test's.
            continue;
        };
        let (misplaced, row) = with_misplaced_child(&text);
        match parse(&misplaced, stem) {
            Err((code, line))
                if code == "\"validation/unexpected-child-element\"" && line == Some(row) => {}
            other => wrong.push(format!("{} ({kind}): {other:?}", path.display())),
        }
        kinds.insert(kind);
        judged += 1;
    }

    // Floors: a scan of nothing refuses nothing. The resources hold well
    // over a hundred documents that parse on their own, across the kinds
    // the conformance catalog names.
    assert!(judged >= 100, "only {judged} document(s) were judged");
    assert!(
        kinds.len() >= 12,
        "only {} kind(s) were judged: {kinds:?}",
        kinds.len()
    );
    assert!(
        wrong.is_empty(),
        "a misplaced <sce:helper> was not refused where it was written:\n  {}",
        wrong.join("\n  ")
    );
}

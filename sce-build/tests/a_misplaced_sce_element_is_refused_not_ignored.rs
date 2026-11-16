// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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
//! where no kind reads one. Each must be refused on the row that holds it:
//! as misplaced, with the move to the element that reads helpers, where the
//! document has one (a procedure's `<datamodel>`), and as an unexpected
//! child everywhere else. The documents are the population, so a kind is
//! covered by having a document, not by a list here.

use std::collections::BTreeSet;
use std::path::Path;

use sce_build::forge::diagnostic::ToDiagnostics;
use sce_build::forge::error::RelatedRole;
use sce_build::DocumentLabel;

const MISPLACED: &str =
    r#"<sce:helper name="misplaced" args="bytes" returns="bytes" sce:returns-max-size="64"/>"#;

/// What a refusal said: its code, its row, and for a misplaced element the
/// fix's `under` and the row of its `destination` site.
#[derive(Debug)]
struct Refusal {
    code: String,
    line: Option<u32>,
    under: Option<String>,
    destination_line: Option<u32>,
    /// The `destination` site's `actual` begins at its `col`
    /// (SCE_ERROR_CONTRACT.md §2.2, §2.4).
    destination_at_its_col: bool,
}

fn parse(text: &str, stem: &str) -> Result<Option<String>, Refusal> {
    let label = DocumentLabel {
        identifier: stem,
        diagnostic_label: stem,
    };
    match sce_build::forge::parser::parse_forge_with_imports(text, label) {
        Ok(parsed) => Ok(parsed.map(|p| format!("{:?}", p.document.kind()))),
        Err(refusal) => {
            let record = &refusal.error.to_diagnostics()[0];
            let code = serde_json::to_string(&record.code).expect("a code serialises");
            let fix = serde_json::to_value(&record.fix).expect("a fix serialises");
            let destination = refusal
                .related()
                .iter()
                .find(|site| site.role == RelatedRole::Destination);
            let destination_at_its_col = destination.is_some_and(|site| {
                let (Some(line), Some(col), Some(actual)) = (
                    site.location.line,
                    site.location.col,
                    site.actual.as_deref(),
                ) else {
                    return false;
                };
                let row = text.lines().nth(line as usize - 1).unwrap_or_default();
                let from: String = row.chars().skip(col as usize - 1).collect();
                from.starts_with(actual)
            });
            Err(Refusal {
                destination_at_its_col,
                code,
                line: refusal.location.line,
                under: fix
                    .get("under")
                    .and_then(|u| u.as_str())
                    .map(str::to_string),
                destination_line: refusal
                    .related()
                    .iter()
                    .find(|site| site.role == RelatedRole::Destination)
                    .and_then(|site| site.location.line),
            })
        }
    }
}

/// The row of the root's `<datamodel>` child in `text`, when it has one.
fn datamodel_row(text: &str) -> Option<u32> {
    let doc = roxmltree::Document::parse(text).expect("the document parses");
    doc.root_element()
        .children()
        .find(|n| n.has_tag_name("datamodel"))
        .map(|n| doc.text_pos_at(n.range().start).row)
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
    let mut moved = 0usize;
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
        // A procedure's `<datamodel>` is where a helper is read, so there
        // the parse knows the move; in every other kind nothing reads one.
        let home = if kind == "Procedure" {
            datamodel_row(&misplaced)
        } else {
            None
        };
        match (parse(&misplaced, stem), home) {
            (Err(refusal), Some(home_row))
                if refusal.code == "\"validation/misplaced-element\""
                    && refusal.line == Some(row)
                    && refusal.under.as_deref() == Some("datamodel")
                    && refusal.destination_line == Some(home_row)
                    && refusal.destination_at_its_col =>
            {
                moved += 1;
            }
            (Err(refusal), None)
                if refusal.code == "\"validation/unexpected-child-element\""
                    && refusal.line == Some(row) => {}
            (other, _) => wrong.push(format!("{} ({kind}): {other:?}", path.display())),
        }
        kinds.insert(kind);
        judged += 1;
    }

    // Floors: a scan of nothing refuses nothing. The resources hold well
    // over a hundred documents that parse on their own, across the kinds
    // the conformance catalog names, and procedures among them.
    assert!(judged >= 100, "only {judged} document(s) were judged");
    assert!(
        moved >= 1,
        "no procedure was judged, so the move was never asked for"
    );
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

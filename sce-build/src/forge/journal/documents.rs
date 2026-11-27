// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The documents the event journal's record amends say so, and state what its parts hold
//! (docs/adr/0114, "Order of work" step 5).
//!
//! The record's "Amends" names each document its landing moves, and each gains an "Amended by:
//! ADR 0114" line in the landing that moves it. A document a landing forgot keeps describing the
//! server as it was, and nothing disagreed: three of the six named had no such line when the
//! record's last step of code landed. What the documents state of the reader of a followed journal
//! and of the store's layout is read here against the sources that hold it, as
//! `mesh::transport`'s tests hold SCE_MESH.md's matrix to the registry.

use std::collections::BTreeSet;

/// The record.
const RECORD: &str = include_str!(
    "../../../../docs/adr/0114-a-keyed-sessions-facts-are-kept-in-a-journal-written-with-its-state-and-a-follower-reads-them-in-an-order-that-misses-nothing.md"
);

const MESH: &str = include_str!("../../../../SCE_MESH.md");
const SUBSET: &str = include_str!("../../../../docs/SCE_ACCEPTED_SUBSET.md");
const SERVER_DEPLOY: &str = include_str!("../../../../docs/SCE_SERVER_DEPLOY.md");
const SEALED_RECORD: &str = include_str!("../../../../docs/SCE_SEALED_RECORD.md");
const HOST_FEATURES: &str = include_str!("../../../../docs/SCE_HOST_FEATURES.md");
const SERVER_RECORD: &str = include_str!(
    "../../../../docs/adr/0109-a-server-on-the-public-network-keeps-what-it-owes-bounds-what-its-clients-cost-and-is-upgraded-in-place.md"
);

/// The reader of a followed journal, which declares its page, its window and the origin type of
/// what it hands a session.
const FOLLOW: &str = include_str!("../../../../backends/rust/mesh/src/pool/follow.rs");

/// The reference store, which declares its layout.
const SQLITE: &str = include_str!("../../../../backends/rust/session-store/src/sqlite.rs");

/// Each document the record amends, by the name its "Amends" gives it, and its text.
const AMENDED: &[(&str, &str)] = &[
    ("SCE_MESH.md", MESH),
    ("ADR 0109", SERVER_RECORD),
    ("docs/SCE_ACCEPTED_SUBSET.md", SUBSET),
    ("docs/SCE_SERVER_DEPLOY.md", SERVER_DEPLOY),
    ("docs/SCE_SEALED_RECORD.md", SEALED_RECORD),
    ("docs/SCE_HOST_FEATURES.md", HOST_FEATURES),
];

/// The record's "Amends" bullet, its lines joined: from the line that opens it to the next bullet
/// of the header.
fn amends() -> String {
    let mut lines = RECORD
        .lines()
        .skip_while(|line| !line.starts_with("- Amends"));
    let mut bullet = lines
        .next()
        .expect("the record's header names what it amends")
        .to_string();
    for line in lines.take_while(|line| line.starts_with("  ")) {
        bullet.push(' ');
        bullet.push_str(line.trim());
    }
    bullet
}

/// Whether `text` says the record amended it, in either spelling the documents give the line.
fn says_it_is_amended(text: &str) -> bool {
    text.contains("Amended by: ADR 0114") || text.contains("Amended by ADR 0114")
}

/// Whether `step`, the text after a step's number, opens with the day it landed: `(YYYY-MM-DD)`.
fn is_dated(step: &str) -> bool {
    step.strip_prefix('(')
        .and_then(|rest| rest.get(..11))
        .is_some_and(|day| {
            day.ends_with(')')
                && day.bytes().take(10).enumerate().all(|(at, byte)| {
                    if at == 4 || at == 7 {
                        byte == b'-'
                    } else {
                        byte.is_ascii_digit()
                    }
                })
        })
}

/// The value the declaration that starts with `declaration` gives in `source`: the text after
/// `= ` and before its `;`.
fn declared<'s>(source: &'s str, declaration: &str) -> &'s str {
    source
        .lines()
        .find(|line| line.trim_start().starts_with(declaration))
        .and_then(|line| line.split_once(" = "))
        .and_then(|(_, value)| value.trim_end().strip_suffix(';'))
        .unwrap_or_else(|| panic!("no declaration '{declaration} = ..;' in the source"))
}

/// The number a declaration gives, its digit separators dropped.
fn declared_number(source: &str, declaration: &str) -> u64 {
    let value = declared(source, declaration).replace('_', "");
    value
        .parse()
        .unwrap_or_else(|_| panic!("'{declaration}' is no number: '{value}'"))
}

#[test]
fn every_document_the_record_amends_says_it_is_amended() {
    let bullet = amends();
    let named: BTreeSet<&str> = bullet
        .split('`')
        .skip(1)
        .step_by(2)
        .filter(|token| token.ends_with(".md"))
        .collect();
    let listed: BTreeSet<&str> = AMENDED
        .iter()
        .map(|(name, _)| *name)
        .filter(|name| name.ends_with(".md"))
        .collect();
    assert_eq!(
        named, listed,
        "the documents the record's \"Amends\" names are not the ones this test reads"
    );
    for (name, _) in AMENDED.iter().filter(|(name, _)| !name.ends_with(".md")) {
        assert!(
            bullet.contains(name),
            "the record's \"Amends\" no longer names {name}: {bullet}"
        );
    }
    let silent: Vec<&str> = AMENDED
        .iter()
        .filter(|(_, text)| !says_it_is_amended(text))
        .map(|(name, _)| *name)
        .collect();
    assert!(
        silent.is_empty(),
        "documents ADR 0114 amends that carry no \"Amended by: ADR 0114\" line: {silent:?}"
    );
}

#[test]
fn the_record_says_when_each_step_of_its_order_of_work_landed() {
    let order = RECORD
        .split("\n## Order of work\n")
        .nth(1)
        .and_then(|rest| rest.split("\n## ").next())
        .expect("the record has an \"Order of work\"");
    let steps: Vec<&str> = order
        .lines()
        .filter_map(|line| {
            let (number, step) = line.split_once(". ")?;
            number.parse::<u32>().ok().map(|_| step)
        })
        .collect();
    assert_eq!(steps.len(), 5, "the record's order of work has five steps");
    let open: Vec<&str> = steps.into_iter().filter(|step| !is_dated(step)).collect();
    assert!(
        open.is_empty(),
        "steps of ADR 0114 not marked landed: {open:?}"
    );
}

#[test]
fn the_documents_state_what_the_reader_and_the_store_hold() {
    let page = declared_number(FOLLOW, "pub const FOLLOW_PAGE:");
    let window = declared_number(FOLLOW, "pub const FOLLOW_WINDOW:");
    let origin_type = declared(FOLLOW, "pub const JOURNAL_ORIGIN_TYPE:").trim_matches('"');
    let layout = declared_number(SQLITE, "const LAYOUT_VERSION:");
    let statements = [
        ("SCE_MESH.md", MESH, format!("`FOLLOW_PAGE` ({page})")),
        ("SCE_MESH.md", MESH, format!("`FOLLOW_WINDOW` ({window})")),
        (
            "SCE_MESH.md",
            MESH,
            format!("`_event.origintype` is `{origin_type}`"),
        ),
        (
            "docs/SCE_SERVER_DEPLOY.md",
            SERVER_DEPLOY,
            format!("names the layout, `{layout}` today"),
        ),
        (
            "docs/SCE_SERVER_DEPLOY.md",
            SERVER_DEPLOY,
            format!("\n| {layout}, "),
        ),
        (
            "docs/SCE_HOST_FEATURES.md",
            HOST_FEATURES,
            format!("(`sce_rust_session_store::SqliteStore`, layout {layout})"),
        ),
    ];
    let unstated: Vec<String> = statements
        .iter()
        .filter(|(_, text, statement)| !text.contains(statement.as_str()))
        .map(|(name, _, statement)| format!("{name}: {statement:?}"))
        .collect();
    assert!(
        unstated.is_empty(),
        "documents that do not state what the reader of a followed journal and the store hold:\n  {}",
        unstated.join("\n  ")
    );
}

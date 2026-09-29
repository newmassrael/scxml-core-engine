// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! An acceptance record names what its design was authored from, so the
//! same specification asked for again is answered with the accepted design
//! rather than a new draft.
//!
//! The model that writes a draft runs in the owner's client, and two drafts
//! of one specification differ in every case measured (the pilot of
//! 2026-09-29: thirty drafts, no two byte-identical, no two review pages
//! identical). The only way a second request for the same inputs gets the
//! same design is to be handed the one already accepted, and "the same
//! inputs" is a question about the specification and the owner's decision
//! record. These cases hold the record to answering it:
//!
//! - a revised specification or decision record lapses the acceptance, and
//!   says which role moved;
//! - the question is about CONTENT: the same specification under another
//!   name is the same input, and a different one is not;
//! - a role left out of the question is part of the answer;
//! - a record taken without sources keeps the bytes it always had, and says
//!   it cannot answer for a specification rather than holding.

use std::fs;
use std::path::{Path, PathBuf};

use sce_build::acceptance_record::{AcceptanceRecord, Lapse, SourceRole};

const MANIFEST: &str = "spec/iso13400_2_nl_socket_handling.manifest.json";
const DOCUMENT: &str = "design/doip_nl_connection_states.scxml";
const PROSE: &str = "spec/connection.md";
const DECISIONS: &str = "spec/decisions.json";

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/requirement_closure")
}

fn design_root() -> tempfile::TempDir {
    let root = tempfile::TempDir::new().expect("tempdir");
    let put = |rel: &str, bytes: &[u8]| {
        let path = root.path().join(rel);
        fs::create_dir_all(path.parent().expect("a parent")).expect("mkdir");
        fs::write(path, bytes).expect("write");
    };
    put(
        MANIFEST,
        &fs::read(fixtures().join("iso13400_2_nl_socket_handling.manifest.json"))
            .expect("the committed manifest is readable"),
    );
    put(
        DOCUMENT,
        &fs::read(fixtures().join("doip_nl_connection_states.scxml"))
            .expect("the committed document is readable"),
    );
    put(
        PROSE,
        b"The connection closes after the inactivity timeout.\n",
    );
    put(
        DECISIONS,
        br#"{"decisions": [{"id": "D1", "answer": "5 minutes"}]}"#,
    );
    root
}

fn authored(root: &Path) -> AcceptanceRecord {
    AcceptanceRecord::take_authored(
        root,
        &root.join(DOCUMENT),
        &root.join(MANIFEST),
        "base",
        &[
            (SourceRole::Specification, &root.join(PROSE)),
            (SourceRole::Decisions, &root.join(DECISIONS)),
        ],
    )
    .expect("a record over the design, authored from the prose and the decisions")
}

#[test]
fn the_record_pins_what_the_design_was_authored_from() {
    let root = design_root();
    let record = authored(root.path());
    let roles: Vec<(SourceRole, &str)> = record
        .authored_from
        .iter()
        .map(|p| (p.role, p.path.as_str()))
        .collect();
    assert_eq!(
        roles,
        [
            (SourceRole::Specification, PROSE),
            (SourceRole::Decisions, DECISIONS)
        ]
    );
    let json = record.to_json();
    assert!(json.contains("\"authored_from\""), "{json}");
    assert_eq!(
        AcceptanceRecord::from_json(&json).expect("reads back"),
        record
    );
    assert_eq!(
        record.recheck(root.path(), "base").expect("rechecks"),
        [],
        "nothing moved"
    );
}

#[test]
fn a_revised_specification_lapses_the_acceptance_and_says_so() {
    let root = design_root();
    let record = authored(root.path());
    fs::write(
        root.path().join(PROSE),
        "The connection closes after two inactivity timeouts.\n",
    )
    .expect("revise");
    let lapses = record.recheck(root.path(), "base").expect("rechecks");
    assert!(
        matches!(lapses.as_slice(), [Lapse::Source { role: SourceRole::Specification, path, current_sha256: Some(_), .. }] if path == PROSE),
        "{lapses:?}"
    );
    assert!(lapses[0]
        .to_string()
        .contains("specification this design was authored from changed"));
}

#[test]
fn a_decision_record_that_is_gone_lapses_the_acceptance() {
    let root = design_root();
    let record = authored(root.path());
    fs::remove_file(root.path().join(DECISIONS)).expect("remove");
    let lapses = record.recheck(root.path(), "base").expect("rechecks");
    assert!(
        matches!(
            lapses.as_slice(),
            [Lapse::Source {
                role: SourceRole::Decisions,
                current_sha256: None,
                ..
            }]
        ),
        "{lapses:?}"
    );
}

#[test]
fn the_question_is_about_content_and_not_about_names() {
    let root = design_root();
    let record = authored(root.path());
    // The owner's copy of the same prose, under the name their client gave it.
    let elsewhere = tempfile::TempDir::new().expect("tempdir");
    let copy = elsewhere.path().join("spec.md");
    let decisions = elsewhere.path().join("answers.json");
    fs::copy(root.path().join(PROSE), &copy).expect("copy prose");
    fs::copy(root.path().join(DECISIONS), &decisions).expect("copy decisions");
    let asked = [
        (SourceRole::Specification, copy.as_path()),
        (SourceRole::Decisions, decisions.as_path()),
    ];
    assert_eq!(
        record
            .recheck_for(root.path(), "base", &asked)
            .expect("rechecks"),
        []
    );

    fs::write(&copy, "A different specification.\n").expect("edit the copy");
    let lapses = record
        .recheck_for(root.path(), "base", &asked)
        .expect("rechecks");
    assert!(
        matches!(
            lapses.as_slice(),
            [Lapse::NotAuthoredFrom {
                role: SourceRole::Specification,
                ..
            }]
        ),
        "{lapses:?}"
    );
}

#[test]
fn a_role_left_out_of_the_question_is_part_of_the_answer() {
    let root = design_root();
    let record = authored(root.path());
    // The same prose, and no decision record: not the inputs the design
    // followed, because it followed the owner's answers too.
    let lapses = record
        .recheck_for(
            root.path(),
            "base",
            &[(SourceRole::Specification, &root.path().join(PROSE))],
        )
        .expect("rechecks");
    assert!(
        matches!(lapses.as_slice(), [Lapse::NotAuthoredFrom { role: SourceRole::Decisions, asked, .. }] if asked.is_empty()),
        "{lapses:?}"
    );
}

#[test]
fn a_record_without_sources_keeps_its_bytes_and_cannot_answer_for_one() {
    let root = design_root();
    let record = AcceptanceRecord::take(
        root.path(),
        &root.path().join(DOCUMENT),
        &root.path().join(MANIFEST),
        "base",
    )
    .expect("a record over the design alone");
    assert!(record.authored_from.is_empty());
    assert!(
        !record.to_json().contains("authored_from"),
        "a record taken without sources is written in the format it always had"
    );
    let lapses = record
        .recheck_for(
            root.path(),
            "base",
            &[(SourceRole::Specification, &root.path().join(PROSE))],
        )
        .expect("rechecks");
    assert!(
        matches!(lapses.as_slice(), [Lapse::NotAuthoredFrom { role: SourceRole::Specification, pinned, .. }] if pinned.is_empty()),
        "{lapses:?}"
    );
    assert!(lapses[0]
        .to_string()
        .contains("does not say which specification"));
}

#[test]
fn two_decision_records_are_refused() {
    let root = design_root();
    let second = root.path().join("spec/more.json");
    fs::write(&second, "{}").expect("write");
    let refused = AcceptanceRecord::take_authored(
        root.path(),
        &root.path().join(DOCUMENT),
        &root.path().join(MANIFEST),
        "base",
        &[
            (SourceRole::Decisions, &root.path().join(DECISIONS)),
            (SourceRole::Decisions, &second),
        ],
    )
    .expect_err("one record of the owner's answers");
    assert_eq!(refused.kind(), "source");
}

#[test]
fn a_record_whose_sources_break_the_rules_is_refused_on_reading() {
    let root = design_root();
    let json = authored(root.path()).to_json();
    let mut wire: serde_json::Value = serde_json::from_str(&json).expect("json");
    let first = wire["authored_from"][0].clone();
    wire["authored_from"]
        .as_array_mut()
        .expect("array")
        .push(first);
    let refused = AcceptanceRecord::from_json(&wire.to_string()).expect_err("a path pinned twice");
    assert_eq!(refused.kind(), "format");
}

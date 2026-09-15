// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The completion test for the acceptance record — Requirement-closure RFC
//! §8.3.
//!
//! A record that re-checks as held proves nothing about whether it could
//! lapse. So every case below takes a record over a design, moves exactly
//! one pinned component, and asks the record what moved. The expected
//! answer is the whole set of lapses, not "at least one": a record that
//! lapsed for the wrong reason, or named a component that did not move,
//! misleads the person who has to decide whether to accept again.
//!
//! The controls are the other half, and they fail the opposite way:
//!
//! - the design re-checked unchanged holds;
//! - the same files copied under another root, same basenames, hold — a
//!   record that lapsed on relocation would lapse on every checkout but the
//!   one that took it, and a lapse everyone learns to ignore is no lapse;
//! - a re-take after a change holds again, so the way back from a lapse is
//!   a new acceptance and not a lenient check.
//!
//! The design is the committed ISO 13400-2 pair from Atomic D, and beside it
//! a small document that composes one transition from an `<xi:include>`d
//! fragment — because the entry document is not the whole design, and a pin
//! that stopped at it would hold across a fragment edit.

use std::fs;
use std::path::{Path, PathBuf};

use sce_build::acceptance_record::{AcceptanceRecord, Lapse};

const MANIFEST: &str = "spec/iso13400_2_nl_socket_handling.manifest.json";
const DOCUMENT: &str = "design/doip_nl_connection_states.scxml";
const HOST: &str = "composed/host.scxml";
const FRAGMENT: &str = "composed/frag.xml";

const HOST_TEXT: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:xi="http://www.w3.org/2001/XInclude"
       version="1.0" name="host" initial="waiting">
  <state id="waiting">
    <xi:include href="frag.xml"/>
  </state>
  <final id="done"/>
</scxml>
"#;

const FRAGMENT_TEXT: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<fragment>
  <transition event="tick" target="done" xmlns="http://www.w3.org/2005/07/scxml"/>
</fragment>
"#;

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/requirement_closure")
}

/// A root holding the design under test, laid out the way a consumer's
/// repository would hold it: specification and design in separate trees.
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
    put(HOST, HOST_TEXT.as_bytes());
    put(FRAGMENT, FRAGMENT_TEXT.as_bytes());
    root
}

fn take(root: &Path, document: &str, variant: &str) -> AcceptanceRecord {
    AcceptanceRecord::take(root, &root.join(document), &root.join(MANIFEST), variant)
        .unwrap_or_else(|e| panic!("taking a record over {document}: {e}"))
}

fn copy_tree(from: &Path, to: &Path) {
    for entry in fs::read_dir(from).expect("read dir") {
        let entry = entry.expect("entry");
        let target = to.join(entry.file_name());
        if entry.path().is_dir() {
            fs::create_dir_all(&target).expect("mkdir");
            copy_tree(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), &target).expect("copy");
        }
    }
}

fn edit(root: &Path, rel: &str, change: impl FnOnce(String) -> String) {
    let path = root.join(rel);
    let before = fs::read_to_string(&path).expect("read");
    let after = change(before.clone());
    assert_ne!(
        before, after,
        "the edit to {rel} changed nothing, so it tests nothing"
    );
    fs::write(path, after).expect("write");
}

/// The lapse kinds and paths, which is what a person acting on a lapse
/// needs; the digests themselves are noise in a failure message.
fn named(lapses: &[Lapse]) -> Vec<String> {
    let mut names: Vec<String> = lapses
        .iter()
        .map(|lapse| match lapse {
            Lapse::Variant { .. } => "variant".to_string(),
            Lapse::Manifest {
                recorded_rev,
                current_rev,
                ..
            } => format!(
                "manifest {recorded_rev}->{}",
                current_rev.as_deref().unwrap_or("<unloadable>")
            ),
            Lapse::Missing { path } => format!("missing {path}"),
            Lapse::Moved { path, .. } => format!("moved {path}"),
            Lapse::Added { path } => format!("added {path}"),
            Lapse::Unparseable { path, .. } => format!("unparseable {path}"),
        })
        .collect();
    names.sort();
    names
}

/// The controls: unchanged holds, relocated holds, and the record says it
/// pinned the fragment at all.
#[test]
fn an_unchanged_design_holds_wherever_it_is_checked_out() {
    let root = design_root();
    let plain = take(root.path(), DOCUMENT, "base");
    let composed = take(root.path(), HOST, "base");

    // A floor under every lapse case below: a record that pinned only the
    // entry document would hold across a fragment edit, and the fragment
    // case would then pass for the wrong reason.
    let pinned: Vec<&str> = composed
        .inputs
        .iter()
        .map(|pin| pin.path.as_str())
        .collect();
    assert_eq!(
        pinned,
        vec![FRAGMENT, HOST],
        "the record must pin every file the parse read"
    );
    assert_eq!(plain.inputs.len(), 1, "{:?}", plain.inputs);
    assert_eq!(plain.manifest.rev, "2019");

    for record in [&plain, &composed] {
        assert_eq!(
            named(&record.recheck(root.path(), "base").expect("re-check")),
            Vec::<String>::new(),
            "an unchanged design lapsed"
        );
    }

    // Same basenames, another root — and read back from the committed form,
    // which is what another checkout holds.
    let elsewhere = tempfile::TempDir::new().expect("tempdir");
    copy_tree(root.path(), elsewhere.path());
    for record in [&plain, &composed] {
        let read_back = AcceptanceRecord::from_json(&record.to_json()).expect("reads back");
        assert_eq!(&read_back, record);
        assert_eq!(
            named(
                &read_back
                    .recheck(elsewhere.path(), "base")
                    .expect("re-check")
            ),
            Vec::<String>::new(),
            "the same design under another root lapsed"
        );
    }
}

/// Each pinned component, moved on its own, is the lapse the record names.
#[test]
fn each_pinned_component_lapses_as_itself() {
    struct Case {
        what: &'static str,
        document: &'static str,
        asked_variant: &'static str,
        change: fn(&Path),
        expected: &'static [&'static str],
    }
    let cases = [
        Case {
            what: "the entry document",
            document: DOCUMENT,
            asked_variant: "base",
            change: |root| edit(root, DOCUMENT, |text| text + "<!-- retimed -->\n"),
            expected: &["moved design/doip_nl_connection_states.scxml"],
        },
        Case {
            what: "a fragment, with the entry document untouched",
            document: HOST,
            asked_variant: "base",
            change: |root| edit(root, FRAGMENT, |text| text.replace("tick", "tock")),
            expected: &["moved composed/frag.xml"],
        },
        Case {
            what: "a composition that stopped reading the fragment",
            document: HOST,
            asked_variant: "base",
            change: |root| {
                edit(root, HOST, |text| {
                    text.replace(
                        r#"<xi:include href="frag.xml"/>"#,
                        r#"<transition event="tick" target="done"/>"#,
                    )
                })
            },
            expected: &["missing composed/frag.xml", "moved composed/host.scxml"],
        },
        Case {
            what: "a composition that reads a file never accepted",
            document: HOST,
            asked_variant: "base",
            change: |root| {
                fs::write(
                    root.join("composed/more.xml"),
                    FRAGMENT_TEXT.replace("tick", "tock"),
                )
                .expect("write");
                edit(root, HOST, |text| {
                    text.replace(
                        r#"<xi:include href="frag.xml"/>"#,
                        r#"<xi:include href="frag.xml"/><xi:include href="more.xml"/>"#,
                    )
                })
            },
            expected: &["added composed/more.xml", "moved composed/host.scxml"],
        },
        Case {
            what: "the manifest's revision",
            document: DOCUMENT,
            asked_variant: "base",
            change: |root| {
                edit(root, MANIFEST, |text| {
                    text.replacen(r#""rev": "2019""#, r#""rev": "2020""#, 1)
                })
            },
            expected: &["manifest 2019->2020"],
        },
        Case {
            what: "the manifest's bytes under an unchanged revision",
            document: DOCUMENT,
            asked_variant: "base",
            change: |root| edit(root, MANIFEST, |text| text + "\n"),
            expected: &["manifest 2019->2019"],
        },
        Case {
            what: "the variant asked about",
            document: DOCUMENT,
            asked_variant: "base+tls",
            change: |_| {},
            expected: &["variant"],
        },
        Case {
            what: "a deleted document",
            document: DOCUMENT,
            asked_variant: "base",
            change: |root| fs::remove_file(root.join(DOCUMENT)).expect("remove"),
            expected: &["missing design/doip_nl_connection_states.scxml"],
        },
        Case {
            what: "a document that no longer parses",
            document: DOCUMENT,
            asked_variant: "base",
            change: |root| edit(root, DOCUMENT, |text| text.replace("</scxml>", "")),
            expected: &["unparseable design/doip_nl_connection_states.scxml"],
        },
    ];

    let mut failures = Vec::new();
    for case in &cases {
        let root = design_root();
        let record = take(root.path(), case.document, "base");
        (case.change)(root.path());
        let got = named(
            &record
                .recheck(root.path(), case.asked_variant)
                .expect("re-check"),
        );
        if got != case.expected {
            failures.push(format!(
                "  {}: expected {:?}, got {got:?}",
                case.what, case.expected
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "a record did not name what moved:\n{}",
        failures.join("\n")
    );
}

/// The way back from a lapse is a new acceptance, not a lenient check.
#[test]
fn a_record_taken_again_after_a_change_holds_again() {
    let root = design_root();
    let before = take(root.path(), HOST, "base");
    edit(root.path(), FRAGMENT, |text| text.replace("tick", "tock"));
    assert_eq!(
        named(&before.recheck(root.path(), "base").expect("re-check")),
        vec!["moved composed/frag.xml".to_string()],
    );
    let after = take(root.path(), HOST, "base");
    assert_ne!(
        before, after,
        "re-taking after a change pinned the old bytes"
    );
    assert_eq!(
        named(&after.recheck(root.path(), "base").expect("re-check")),
        Vec::<String>::new(),
    );
}

/// A design reaching outside the root cannot be named relative to it, and
/// is refused rather than pinned by a path that means nothing elsewhere.
#[test]
fn a_design_outside_the_root_is_refused() {
    let root = design_root();
    let inner = root.path().join("design");
    let refused = AcceptanceRecord::take(
        &inner,
        &root.path().join(DOCUMENT),
        &root.path().join(MANIFEST),
        "base",
    );
    assert!(
        refused.is_err(),
        "a manifest outside the root was pinned: {refused:?}"
    );
}

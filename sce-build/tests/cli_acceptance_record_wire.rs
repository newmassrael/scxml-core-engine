// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! What `sce-codegen accept` writes and what `sce-codegen acceptance-check`
//! answers are what the library decides — checked through the binary.
//!
//! `an_acceptance_record_lapses_when_what_was_accepted_moves` holds the
//! record itself and never spawns the binary, so it says nothing about
//! whether the subcommands publish that record or turn a lapse into an exit
//! status a gate can branch on. This file is that layer: the record `accept`
//! writes must be the bytes `AcceptanceRecord::take` produces, a held record
//! must exit 0, and a lapsed one must exit with `cli/acceptance-lapsed` and
//! name what moved.
//!
//! The controls fail the opposite way: an unchanged design and the same
//! design under another root both exit 0, so a subcommand that exited
//! non-zero on everything would fail here as surely as one that never did.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use sce_build::acceptance_record::AcceptanceRecord;

/// The generator binary. `env!` here rather than in a helper, for the reason
/// `cli_acceptance_report_wire` gives.
const CODEGEN: &str = env!("CARGO_BIN_EXE_sce-codegen");

const MANIFEST: &str = "spec/manifest.json";
const HOST: &str = "design/host.scxml";
const FRAGMENT: &str = "design/frag.xml";
const RECORD: &str = "acceptance/base.json";

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

fn design_root() -> tempfile::TempDir {
    let root = tempfile::TempDir::new().expect("tempdir");
    let put = |rel: &str, bytes: &[u8]| {
        let path = root.path().join(rel);
        fs::create_dir_all(path.parent().expect("a parent")).expect("mkdir");
        fs::write(path, bytes).expect("write");
    };
    put(
        MANIFEST,
        &fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join(
            "tests/fixtures/requirement_closure/iso13400_2_nl_socket_handling.manifest.json",
        ))
        .expect("the committed manifest is readable"),
    );
    put(HOST, HOST_TEXT.as_bytes());
    put(FRAGMENT, FRAGMENT_TEXT.as_bytes());
    fs::create_dir_all(root.path().join("acceptance")).expect("mkdir");
    root
}

fn run(args: &[&str], root: &Path) -> Output {
    Command::new(CODEGEN)
        .arg("--error-format=json")
        .args(args)
        .current_dir(root)
        .output()
        .expect("spawn sce-codegen")
}

fn accept(root: &Path) -> Output {
    run(
        &[
            "accept",
            &root.join(HOST).display().to_string(),
            "--manifest",
            &root.join(MANIFEST).display().to_string(),
            "--variant",
            "base",
            "--root",
            &root.display().to_string(),
            "--out",
            &root.join(RECORD).display().to_string(),
        ],
        root,
    )
}

fn check(root: &Path, record: &Path, variant: &str) -> Output {
    run(
        &[
            "acceptance-check",
            &record.display().to_string(),
            "--variant",
            variant,
            "--root",
            &root.display().to_string(),
        ],
        root,
    )
}

/// The codes the run reported, one per NDJSON record on stderr.
fn codes(out: &Output) -> Vec<String> {
    String::from_utf8_lossy(&out.stderr)
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .filter_map(|v| v.get("code").and_then(|c| c.as_str()).map(str::to_string))
        .collect()
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

/// `accept` writes the record the library takes, byte for byte.
#[test]
fn accept_writes_the_record_the_library_takes() {
    let root = design_root();
    let out = accept(root.path());
    assert!(
        out.status.success(),
        "`accept` exited {:?}\nstderr: {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr)
    );
    let written = fs::read_to_string(root.path().join(RECORD)).expect("the record was written");
    let taken = AcceptanceRecord::take(
        root.path(),
        &root.path().join(HOST),
        &root.path().join(MANIFEST),
        "base",
    )
    .expect("the library takes the same record");
    assert_eq!(
        written,
        taken.to_json(),
        "the subcommand wrote something other than the record the library takes"
    );
    // A floor under the equality: two empty records compare equal.
    assert_eq!(taken.inputs.len(), 2, "{:?}", taken.inputs);
}

/// A held record exits 0 — unchanged, and under another root.
#[test]
fn an_unchanged_design_checks_clean_wherever_it_is() {
    let root = design_root();
    assert!(accept(root.path()).status.success());

    let here = check(root.path(), &root.path().join(RECORD), "base");
    assert_eq!(
        here.status.code(),
        Some(0),
        "an unchanged design did not check clean: {}",
        String::from_utf8_lossy(&here.stderr)
    );

    let elsewhere = tempfile::TempDir::new().expect("tempdir");
    copy_tree(root.path(), elsewhere.path());
    let moved = check(elsewhere.path(), &elsewhere.path().join(RECORD), "base");
    assert_eq!(
        moved.status.code(),
        Some(0),
        "the same design under another root did not check clean: {}",
        String::from_utf8_lossy(&moved.stderr)
    );
}

/// A lapse exits with the lapse code and names what moved; a record that is
/// not one exits with the unusable-input code.
#[test]
fn a_lapse_is_reported_as_a_record_that_names_what_moved() {
    let root = design_root();
    assert!(accept(root.path()).status.success());

    fs::write(
        root.path().join(FRAGMENT),
        FRAGMENT_TEXT.replace("tick", "tock"),
    )
    .expect("edit the fragment");
    let lapsed = check(root.path(), &root.path().join(RECORD), "base");
    assert_eq!(lapsed.status.code(), Some(20), "{lapsed:?}");
    assert_eq!(codes(&lapsed), vec!["cli/acceptance-lapsed".to_string()]);
    let stderr = String::from_utf8_lossy(&lapsed.stderr);
    assert!(
        stderr.contains(FRAGMENT),
        "the record does not name the fragment: {stderr}"
    );
    assert!(
        !stderr.contains(HOST),
        "the record names a file that did not move: {stderr}"
    );

    let other_variant = check(root.path(), &root.path().join(RECORD), "base+tls");
    assert_eq!(other_variant.status.code(), Some(20));
    assert!(String::from_utf8_lossy(&other_variant.stderr).contains("base+tls"));

    fs::write(root.path().join("acceptance/not_a_record.json"), "{}\n").expect("write");
    let refused = check(
        root.path(),
        &root.path().join("acceptance/not_a_record.json"),
        "base",
    );
    assert_eq!(refused.status.code(), Some(20));
    assert_eq!(
        codes(&refused),
        vec!["cli/closure-input-unusable".to_string()]
    );
}

/// `--source` reaches the record, and `acceptance-check --source` answers
/// whether a specification is the one the design was authored from — by
/// content, so the owner's copy under another name holds and a revised one
/// lapses.
#[test]
fn the_specification_a_design_was_authored_from_is_pinned_and_asked_about() {
    let root = design_root();
    let prose = root.path().join("spec/prose.md");
    fs::write(&prose, "Waiting ends on a tick.\n").expect("write prose");
    let taken = run(
        &[
            "accept",
            &root.path().join(HOST).display().to_string(),
            "--manifest",
            &root.path().join(MANIFEST).display().to_string(),
            "--variant",
            "base",
            "--root",
            &root.path().display().to_string(),
            "--out",
            &root.path().join(RECORD).display().to_string(),
            "--source",
            &prose.display().to_string(),
        ],
        root.path(),
    );
    assert!(taken.status.success(), "{taken:?}");
    let record = fs::read_to_string(root.path().join(RECORD)).expect("record written");
    assert!(record.contains("\"spec/prose.md\""), "{record}");

    let copy = root.path().join("the owner's copy.md");
    fs::copy(&prose, &copy).expect("copy");
    let asked = |source: &Path| {
        run(
            &[
                "acceptance-check",
                &root.path().join(RECORD).display().to_string(),
                "--variant",
                "base",
                "--root",
                &root.path().display().to_string(),
                "--source",
                &source.display().to_string(),
            ],
            root.path(),
        )
    };
    let held = asked(&copy);
    assert!(held.status.success(), "{held:?}");

    fs::write(&copy, "Waiting ends on a tock.\n").expect("revise the copy");
    let lapsed = asked(&copy);
    assert_eq!(lapsed.status.code(), Some(20), "{lapsed:?}");
    assert_eq!(codes(&lapsed), vec!["cli/acceptance-lapsed".to_string()]);
    assert!(String::from_utf8_lossy(&lapsed.stderr).contains("authored from"));
}

const SET_TEXT: &str = r#"{"record":"sce-scenario-set","v":1,
  "specification":{"doc_id":"nl","rev":"1"},"origin":"ai-proposed",
  "interface":{"inputs":[],"outputs":[]},
  "scenarios":[{"id":"S1","quote":"Waiting ends on a tick.",
    "steps":[{"advance_ms":5,"expect":{"outbound":[]}}]}]}"#;

/// `--scenarios` reaches the record as a role of its own, and the examples
/// edited afterwards lapse the acceptance: the passes that closed a requirement
/// were the old set's. A file that is no scenario set is refused through its own
/// door and not pinned as the examples the design was held to.
#[test]
fn the_examples_a_design_was_held_to_are_pinned_and_a_changed_set_lapses_it() {
    let root = design_root();
    let set = root.path().join("spec/examples.json");
    fs::write(&set, SET_TEXT).expect("write the set");
    let accept_with = |file: &Path| {
        run(
            &[
                "accept",
                &root.path().join(HOST).display().to_string(),
                "--manifest",
                &root.path().join(MANIFEST).display().to_string(),
                "--variant",
                "base",
                "--root",
                &root.path().display().to_string(),
                "--out",
                &root.path().join(RECORD).display().to_string(),
                "--scenarios",
                &file.display().to_string(),
            ],
            root.path(),
        )
    };

    let taken = accept_with(&set);
    assert!(taken.status.success(), "{taken:?}");
    let record = fs::read_to_string(root.path().join(RECORD)).expect("record written");
    assert!(record.contains("\"role\": \"examples\""), "{record}");
    assert!(record.contains("\"spec/examples.json\""), "{record}");

    let check = || {
        run(
            &[
                "acceptance-check",
                &root.path().join(RECORD).display().to_string(),
                "--variant",
                "base",
                "--root",
                &root.path().display().to_string(),
                "--scenarios",
                &set.display().to_string(),
            ],
            root.path(),
        )
    };
    let held = check();
    assert!(held.status.success(), "{held:?}");

    fs::write(&set, SET_TEXT.replace("a tick", "a tock")).expect("edit the set");
    let lapsed = check();
    assert_eq!(lapsed.status.code(), Some(20), "{lapsed:?}");
    assert_eq!(codes(&lapsed), vec!["cli/acceptance-lapsed".to_string()]);
    let said = String::from_utf8_lossy(&lapsed.stderr);
    assert!(
        said.contains("the scenario set this design was held to changed"),
        "{said}"
    );

    // A file that is no scenario set is refused, and nothing is written.
    let not_a_set = root.path().join("spec/not-a-set.json");
    fs::write(&not_a_set, r#"{"record":"sce-authoring-profile","v":1}"#).expect("write");
    fs::remove_file(root.path().join(RECORD)).expect("clear the record");
    let refused = accept_with(&not_a_set);
    assert_eq!(refused.status.code(), Some(20), "{refused:?}");
    assert_eq!(
        codes(&refused),
        vec!["cli/closure-input-unusable".to_string()]
    );
    assert!(
        !root.path().join(RECORD).exists(),
        "a refused acceptance wrote a record"
    );
}

/// `accept` with the channel an acceptance states, as the command line writes it.
fn accept_stating(root: &Path, channel: &str) -> Output {
    run(
        &[
            "accept",
            &root.join(HOST).display().to_string(),
            "--manifest",
            &root.join(MANIFEST).display().to_string(),
            "--variant",
            "base",
            "--root",
            &root.display().to_string(),
            "--out",
            &root.join(RECORD).display().to_string(),
            "--channel",
            channel,
        ],
        root,
    )
}

/// The channel an acceptance states is written as the caller said it, read back
/// as the same record, and absent from a record that was never told one — whose
/// bytes are then exactly what they were before the field existed.
#[test]
fn the_channel_an_acceptance_states_is_recorded_and_read_back() {
    use sce_build::acceptance_record::Channel;

    let root = design_root();
    for (word, channel) in [("direct", Channel::Direct), ("relayed", Channel::Relayed)] {
        let out = accept_stating(root.path(), word);
        assert!(
            out.status.success(),
            "`accept --channel {word}` exited {:?}: {}",
            out.status.code(),
            String::from_utf8_lossy(&out.stderr)
        );
        let written = fs::read_to_string(root.path().join(RECORD)).expect("the record was written");
        let wire: serde_json::Value = serde_json::from_str(&written).expect("JSON");
        assert_eq!(wire["channel"], word, "{written}");

        let read = AcceptanceRecord::from_json(&written).expect("a record states its channel");
        assert_eq!(read.channel, Some(channel));
        assert_eq!(read.to_json(), written, "read back and written again");
        // It says how the acceptance stood and cannot lapse it.
        let held = check(root.path(), &root.path().join(RECORD), "base");
        assert_eq!(
            held.status.code(),
            Some(0),
            "{}",
            String::from_utf8_lossy(&held.stderr)
        );
    }

    // Left out, nothing is said, and the bytes are the ones the library takes.
    assert!(accept(root.path()).status.success());
    let silent = fs::read_to_string(root.path().join(RECORD)).expect("the record was written");
    assert!(!silent.contains("channel"), "{silent}");
    let taken = AcceptanceRecord::take(
        root.path(),
        &root.path().join(HOST),
        &root.path().join(MANIFEST),
        "base",
    )
    .expect("the library takes the same record");
    assert_eq!(taken.channel, None);
    assert_eq!(silent, taken.to_json());
    // And stating one afterwards is the only thing that differs.
    assert_eq!(taken.clone().stated_by(Some(Channel::Direct)).to_json(), {
        accept_stating(root.path(), "direct");
        fs::read_to_string(root.path().join(RECORD)).expect("the record was written")
    });
}

/// A word that is not a channel is refused before anything is written, and a record
/// that carries one is refused when the word is not a channel this build knows.
#[test]
fn a_channel_that_is_not_one_is_refused() {
    let root = design_root();
    for word in ["", "owner", "Direct", "relay", "verified"] {
        let out = accept_stating(root.path(), word);
        assert!(!out.status.success(), "`--channel {word:?}` was accepted");
        assert!(
            !root.path().join(RECORD).exists(),
            "a refused channel left a record behind"
        );
    }

    assert!(accept_stating(root.path(), "direct").status.success());
    let written = fs::read_to_string(root.path().join(RECORD)).expect("the record was written");
    for bad in ["owner", "verified", ""] {
        let forged = written.replace("\"direct\"", &format!("\"{bad}\""));
        assert!(
            AcceptanceRecord::from_json(&forged).is_err(),
            "a record naming the channel {bad:?} was read"
        );
    }
    // A channel written twice is a key written twice.
    let twice = written.replace(
        "\"channel\": \"direct\"",
        "\"channel\": \"direct\",\n  \"channel\": \"relayed\"",
    );
    assert!(AcceptanceRecord::from_json(&twice).is_err(), "{twice}");
}

/// The record has no schema file, so the registry's row and
/// `ACCEPTANCE_RECORD_STATUS` are the two places its stability lives —
/// `SCE_WIRE_CONTRACTS.md` requires one commit to move both.
#[test]
fn the_registry_declares_the_records_status() {
    use sce_build::acceptance_record::ACCEPTANCE_RECORD_STATUS;
    let registry = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("sce-build has a parent dir")
            .join("SCE_WIRE_CONTRACTS.md"),
    )
    .expect("SCE_WIRE_CONTRACTS.md is readable");
    let row = registry
        .lines()
        .find(|line| line.starts_with("| Acceptance record (`sce-codegen accept`)"))
        .expect(
            "SCE_WIRE_CONTRACTS.md has no row for the acceptance record, which \
             `acceptance-check` and the authoring MCP read back as authority",
        );
    assert!(
        row.contains(&format!("`{ACCEPTANCE_RECORD_STATUS}`")),
        "the registry's row does not name the status the producer declares \
         ({ACCEPTANCE_RECORD_STATUS}).\nrow: {row}",
    );
}

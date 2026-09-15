// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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

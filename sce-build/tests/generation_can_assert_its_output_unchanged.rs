// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// `--assert-unchanged`: a generation that writes nothing and fails when what
// it would write differs from what is on disk — SCE Protocol-Synthesis RFC
// §6.2.6, drift decided on content rather than on a hash a header carries.
//
// Each property is a test of its own:
//   - a tree the same generation just wrote passes, and the run writes nothing;
//   - a hand edit fails the run, is named, and survives it;
//   - a deleted file fails the run as absent and stays deleted;
//   - a changed input fails the run;
//   - a file the run writes and then reads back is judged on what it would
//     finally hold, read back from what the run produced, not from disk;
//   - a file the generation would remove fails the run and is not removed;
//   - a child document the generation copies is compared, not copied;
//   - the run's own checks read what it produced, not what is on disk;
//   - a command the flag cannot reach refuses it, rather than passing.
//
// The second is the one a header hash can never see: editing a generated
// file by hand changes neither its inputs nor the templates, so every hash
// in its header still matches — which is how "manual edits to `out/` are
// forbidden" stayed a sentence for as long as `verify` was its only check.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::SystemTime;

fn codegen() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

const DOOR: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="closed" name="Door">
  <state id="closed"><transition event="open" target="opened"/></state>
  <state id="opened"><transition event="close" target="closed"/></state>
</scxml>
"#;

/// A directory holding `door.scxml`, generated once into `out/`.
fn generated_door() -> tempfile::TempDir {
    let tmp = tempfile::tempdir().expect("temp dir");
    fs::write(tmp.path().join("door.scxml"), DOOR).expect("write the document");
    let first = generate(tmp.path(), &[]);
    assert!(
        first.status.success(),
        "the plain generation failed: {}",
        stderr(&first)
    );
    tmp
}

/// `sce-codegen generate door.scxml -o out -l cpp` in `dir`, plus `extra`.
///
/// `SOURCE_DATE_EPOCH` pins the header stamp, so two runs over one input
/// produce one sequence of bytes and a comparison means something.
fn generate(dir: &Path, extra: &[&str]) -> Output {
    Command::new(codegen())
        .args(["--error-format", "json", "generate"])
        .arg(dir.join("door.scxml"))
        .arg("-o")
        .arg(dir.join("out"))
        .args(["-l", "cpp"])
        .args(extra)
        .env("SOURCE_DATE_EPOCH", "0")
        .output()
        .expect("run sce-codegen")
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

/// The one NDJSON record a failed run leaves on stderr.
fn record(out: &Output) -> serde_json::Value {
    let text = stderr(out);
    let line = text
        .lines()
        .find(|l| l.starts_with('{'))
        .unwrap_or_else(|| panic!("no NDJSON record on stderr:\n{text}"));
    serde_json::from_str(line).unwrap_or_else(|e| panic!("record does not parse ({e}): {line}"))
}

/// Every file under `dir`, recursively, with its bytes and its mtime.
fn snapshot(dir: &Path) -> BTreeMap<PathBuf, (Vec<u8>, SystemTime)> {
    fn walk(dir: &Path, files: &mut BTreeMap<PathBuf, (Vec<u8>, SystemTime)>) {
        for entry in fs::read_dir(dir).expect("read an output directory") {
            let path = entry.expect("dir entry").path();
            if path.is_dir() {
                walk(&path, files);
            } else {
                let bytes = fs::read(&path).expect("read an output");
                let mtime = fs::metadata(&path)
                    .and_then(|m| m.modified())
                    .expect("mtime");
                files.insert(path, (bytes, mtime));
            }
        }
    }
    let mut files = BTreeMap::new();
    walk(dir, &mut files);
    files
}

/// A registry holding only `ids`, taken from the committed one, so a suite
/// generation runs over one fixture rather than two hundred.
fn stage_registry(dir: &Path, ids: &[&str]) -> PathBuf {
    let full =
        fs::read_to_string(repo_root().join(sce_build::w3c_registry::W3C_REGISTRY_RELATIVE_PATH))
            .expect("read the committed registry");
    let mut doc: serde_json::Value = serde_json::from_str(&full).expect("registry is JSON");
    let kept: Vec<serde_json::Value> = doc["fixtures"]
        .as_array()
        .expect("fixtures array")
        .iter()
        .filter(|f| f["id"].as_str().is_some_and(|id| ids.contains(&id)))
        .cloned()
        .collect();
    assert_eq!(
        kept.len(),
        ids.len(),
        "a fixture this test needs is unregistered"
    );
    doc["fixtures"] = serde_json::Value::Array(kept);
    let path = dir.join("fixtures.json");
    fs::write(
        &path,
        serde_json::to_string_pretty(&doc).expect("serialize"),
    )
    .expect("write the registry");
    path
}

/// `sce-codegen generate-w3c -l <language>` over `registry`, into `out`.
fn generate_w3c(language: &str, registry: &Path, out: &Path, extra: &[&str]) -> Output {
    Command::new(codegen())
        .args(["--error-format", "json", "generate-w3c", "-l", language])
        .arg("--registry")
        .arg(registry)
        .arg("--resources")
        .arg(repo_root().join("resources"))
        .arg("--output-dir")
        .arg(out)
        .args(extra)
        .env("SOURCE_DATE_EPOCH", "0")
        .output()
        .expect("run sce-codegen generate-w3c")
}

/// The one file under `dir` named `name`.
fn find_file(dir: &Path, name: &str) -> PathBuf {
    let found: Vec<PathBuf> = snapshot(dir)
        .into_keys()
        .filter(|p| p.file_name().is_some_and(|n| n == name))
        .collect();
    assert_eq!(
        found.len(),
        1,
        "expected one {name} under {}: {found:?}",
        dir.display()
    );
    found.into_iter().next().expect("one match")
}

#[test]
fn a_tree_the_same_generation_wrote_passes_and_the_run_writes_nothing() {
    let tmp = generated_door();
    let out_dir = tmp.path().join("out");
    let before = snapshot(&out_dir);
    assert!(
        before.len() >= 2,
        "the fixture generated too little to compare: {:?}",
        before.keys()
    );

    let check = generate(tmp.path(), &["--assert-unchanged"]);

    assert!(
        check.status.success(),
        "a tree this generation just wrote was reported changed: {}",
        stderr(&check)
    );
    // Bytes AND mtimes: a run that rewrote every file with the same bytes
    // would pass a content comparison while writing everything.
    assert_eq!(snapshot(&out_dir), before, "the asserting run wrote");
}

#[test]
fn a_hand_edit_fails_the_run_is_named_and_survives_it() {
    let tmp = generated_door();
    let header = tmp.path().join("out/door_sm.h");
    let mut edited = fs::read_to_string(&header).expect("read the header");
    edited.push_str("// edited by hand\n");
    fs::write(&header, &edited).expect("edit the header");

    let check = generate(tmp.path(), &["--assert-unchanged"]);

    assert_eq!(check.status.code(), Some(20), "{}", stderr(&check));
    let rec = record(&check);
    assert_eq!(rec["code"], "forge/generated-output-changed", "{rec}");
    assert!(
        rec["message"].as_str().unwrap_or("").contains("door_sm.h"),
        "the record does not name the edited file: {rec}"
    );
    assert_eq!(rec["actual"], "changed=1 missing=0 stale=0", "{rec}");
    assert_eq!(
        fs::read_to_string(&header).expect("reread the header"),
        edited,
        "the asserting run overwrote the hand edit it was reporting"
    );
}

#[test]
fn a_deleted_file_fails_the_run_as_absent_and_stays_deleted() {
    let tmp = generated_door();
    let inl = tmp.path().join("out/door_sm.inl");
    fs::remove_file(&inl).expect("delete an output");

    let check = generate(tmp.path(), &["--assert-unchanged"]);

    assert_eq!(check.status.code(), Some(20), "{}", stderr(&check));
    let rec = record(&check);
    assert_eq!(rec["code"], "forge/generated-output-changed", "{rec}");
    assert_eq!(rec["actual"], "changed=1 missing=1 stale=0", "{rec}");
    assert!(
        !inl.exists(),
        "the asserting run recreated a file it reported absent"
    );
}

#[test]
fn a_changed_input_fails_the_run() {
    let tmp = generated_door();
    let document = tmp.path().join("door.scxml");
    let grown = DOOR.replace(
        r#"<state id="opened">"#,
        r#"<state id="ajar"><transition event="close" target="closed"/></state>
  <state id="opened">"#,
    );
    assert_ne!(grown, DOOR, "the fixture edit found nothing to change");
    fs::write(&document, grown).expect("change the document");

    let check = generate(tmp.path(), &["--assert-unchanged"]);

    assert_eq!(check.status.code(), Some(20), "{}", stderr(&check));
    assert_eq!(
        record(&check)["code"],
        "forge/generated-output-changed",
        "{}",
        stderr(&check)
    );
}

#[test]
fn a_file_the_run_reads_back_is_judged_on_what_it_would_finally_hold() {
    // test187 invokes a child. The Rust suite writes the test's `mod.rs`,
    // then reads it back and appends the child's module. On disk, the
    // previous run's `mod.rs` already names that child: a run that read the
    // disk would append nothing, and a run that judged each write would
    // judge the child-less first one. Either way it reports a change the
    // same generation never makes.
    let staging = tempfile::tempdir().expect("temp dir");
    let out = tempfile::tempdir().expect("temp dir");
    let registry = stage_registry(staging.path(), &["187"]);
    let first = generate_w3c("rust", &registry, out.path(), &[]);
    assert!(
        first.status.success(),
        "the plain generation failed: {}",
        stderr(&first)
    );
    let child = find_file(out.path(), "test187__sce_synth_invoke__invoke_0_sm.rs");
    let index = fs::read_to_string(child.with_file_name("mod.rs")).expect("read mod.rs");
    assert!(
        index.contains("mod test187__sce_synth_invoke__invoke_0_sm;"),
        "the fixture no longer appends a child module, so nothing here is read back:\n{index}"
    );
    let before = snapshot(out.path());

    let check = generate_w3c("rust", &registry, out.path(), &["--assert-unchanged"]);

    assert!(
        check.status.success(),
        "a suite this generation just wrote was reported changed: {}",
        stderr(&check)
    );
    assert_eq!(snapshot(out.path()), before, "the asserting run wrote");
}

#[test]
fn a_file_the_generation_would_remove_fails_the_run_and_is_not_removed() {
    // The Kotlin suite removes the test class of a fixture that is no longer
    // registered. A run that writes nothing must not remove it either, and a
    // tree still holding it is not the tree the generation leaves.
    let staging = tempfile::tempdir().expect("temp dir");
    let out = tempfile::tempdir().expect("temp dir");
    let registry = stage_registry(staging.path(), &["144"]);
    let first = generate_w3c("kotlin", &registry, out.path(), &[]);
    assert!(
        first.status.success(),
        "the plain generation failed: {}",
        stderr(&first)
    );
    let stale = find_file(out.path(), "Test144.kt").with_file_name("Test999.kt");
    fs::write(
        &stale,
        "// the test class of a fixture since unregistered\n",
    )
    .expect("plant a stale test class");

    let check = generate_w3c("kotlin", &registry, out.path(), &["--assert-unchanged"]);

    assert_eq!(check.status.code(), Some(20), "{}", stderr(&check));
    let rec = record(&check);
    assert_eq!(rec["code"], "forge/generated-output-changed", "{rec}");
    assert_eq!(rec["actual"], "changed=1 missing=0 stale=1", "{rec}");
    assert!(
        rec["message"].as_str().unwrap_or("").contains("Test999.kt"),
        "the record does not name the file the generation would remove: {rec}"
    );
    assert!(
        stale.exists(),
        "the asserting run removed the file it was reporting"
    );
}

const PARENT: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="waiting" name="Parent">
  <state id="waiting">
    <invoke type="http://www.w3.org/TR/scxml/" src="file:child.scxml"/>
    <transition event="done.invoke" target="finished"/>
  </state>
  <final id="finished"/>
</scxml>
"#;

const CHILD: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="only" name="Child">
  <final id="only"/>
</scxml>
"#;

#[test]
fn a_child_document_the_generation_copies_is_compared_not_copied() {
    // `generate` copies an invoked child document next to its output. The
    // documents sit in `src/` and the output in its sibling `out/`: an
    // output directory inside the input root would carry the copy into the
    // next run's source set.
    let tmp = tempfile::tempdir().expect("temp dir");
    let src = tmp.path().join("src");
    fs::create_dir(&src).expect("create src/");
    fs::write(src.join("parent.scxml"), PARENT).expect("write the parent");
    fs::write(src.join("child.scxml"), CHILD).expect("write the child");
    let run = |extra: &[&str]| {
        Command::new(codegen())
            .args(["--error-format", "json", "generate"])
            .arg(src.join("parent.scxml"))
            .arg("-o")
            .arg(tmp.path().join("out"))
            .args(["-l", "cpp"])
            .args(extra)
            .env("SOURCE_DATE_EPOCH", "0")
            .output()
            .expect("run sce-codegen")
    };
    let first = run(&[]);
    assert!(
        first.status.success(),
        "the plain generation failed: {}",
        stderr(&first)
    );
    let copy = tmp.path().join("out/child.scxml");
    assert_eq!(
        fs::read_to_string(&copy).expect("the generation copies the child"),
        CHILD
    );
    fs::remove_file(&copy).expect("delete the copy");

    let check = run(&["--assert-unchanged"]);

    assert_eq!(check.status.code(), Some(20), "{}", stderr(&check));
    let rec = record(&check);
    assert_eq!(rec["code"], "forge/generated-output-changed", "{rec}");
    assert_eq!(rec["actual"], "changed=1 missing=1 stale=0", "{rec}");
    assert!(
        rec["message"]
            .as_str()
            .unwrap_or("")
            .contains("child.scxml"),
        "the record does not name the absent copy: {rec}"
    );
    assert!(
        !copy.exists(),
        "the asserting run copied the child it reported absent"
    );
}

#[test]
fn the_runs_own_checks_read_what_it_produced_not_the_disk() {
    // After emitting, a run checks that every generated file carries a
    // source marker. Under --assert-unchanged that check has to read the
    // files the run produced. Reading the disk, a hand edit that strips the
    // markers is reported as a template that lost them — the wrong repair,
    // and the run stops before naming what differs.
    let tmp = generated_door();
    let header = tmp.path().join("out/door_sm.h");
    let original = fs::read_to_string(&header).expect("read the header");
    let stripped: String = original
        .lines()
        .filter(|line| !line.contains("SCE-MAP:"))
        .map(|line| format!("{line}\n"))
        .collect();
    assert_ne!(stripped, original, "the header carries no marker to strip");
    fs::write(&header, &stripped).expect("strip the markers");

    let check = generate(tmp.path(), &["--assert-unchanged"]);

    assert_eq!(check.status.code(), Some(20), "{}", stderr(&check));
    let rec = record(&check);
    assert_eq!(rec["code"], "forge/generated-output-changed", "{rec}");
    assert_eq!(rec["actual"], "changed=1 missing=0 stale=0", "{rec}");
}

#[test]
fn generate_w3c_clean_and_list_refuse_the_flag_and_run_nothing() {
    // Neither generates a file, so a run in either would pass having
    // compared nothing — and `--clean` deletes.
    let staging = tempfile::tempdir().expect("temp dir");
    let out = tempfile::tempdir().expect("temp dir");
    let registry = stage_registry(staging.path(), &["144"]);
    let first = generate_w3c("rust", &registry, out.path(), &[]);
    assert!(
        first.status.success(),
        "the plain generation failed: {}",
        stderr(&first)
    );
    let before = snapshot(out.path());

    for mode in ["--clean", "--list"] {
        let refused = generate_w3c("rust", &registry, out.path(), &[mode, "--assert-unchanged"]);
        assert_eq!(
            refused.status.code(),
            Some(20),
            "{mode}: {}",
            stderr(&refused)
        );
        assert_eq!(
            record(&refused)["code"],
            "cli/usage",
            "{mode}: {}",
            stderr(&refused)
        );
        assert_eq!(
            snapshot(out.path()),
            before,
            "{mode} ran despite the refusal"
        );
    }
}

#[test]
fn a_command_the_flag_cannot_reach_refuses_it() {
    // `generate-integration` runs one shell pipeline per stem, and those
    // write through their own copies and formatter — a flag this process
    // honours could only ever report "unchanged" for them.
    let integration = Command::new(codegen())
        .args([
            "--error-format",
            "json",
            "generate-integration",
            "-l",
            "rust",
            "--assert-unchanged",
        ])
        .output()
        .expect("run sce-codegen");
    assert_eq!(
        integration.status.code(),
        Some(20),
        "{}",
        stderr(&integration)
    );
    let rec = record(&integration);
    assert_eq!(rec["code"], "cli/usage", "{rec}");
    assert!(
        rec["message"]
            .as_str()
            .unwrap_or("")
            .contains("generate-integration"),
        "{rec}"
    );

    // A command that generates nothing has nothing to compare.
    let lookup = Command::new(codegen())
        .args([
            "--error-format",
            "json",
            "provenance-roster",
            "--assert-unchanged",
        ])
        .output()
        .expect("run sce-codegen");
    assert_eq!(lookup.status.code(), Some(20), "{}", stderr(&lookup));
    assert_eq!(record(&lookup)["code"], "cli/usage", "{}", stderr(&lookup));
}

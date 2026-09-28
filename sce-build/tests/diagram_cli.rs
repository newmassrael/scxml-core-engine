// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! `sce-codegen diagram` on the binary an author runs: one SVG per figure,
//! named by state; a figure too large for its page refused with its size
//! and nothing written; the written set checkable with `--assert-unchanged`.
//!
//! The figure's content — nothing missing, every transition described
//! once, the page's own words in each box — is held by the library's tests
//! in `sce_build::diagram`, on the model rather than on pixels. These
//! cases hold the command's side of it.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const LOCK: &str = r##"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="released">
  <state id="released" initial="unlocked">
    <state id="unlocked"><transition event="lock.request" target="locked"/></state>
    <state id="relocking" initial="waiting">
      <state id="waiting"><transition event="timer" target="armed"/></state>
      <state id="armed"><transition event="expire" target="locked"/></state>
    </state>
    <transition event="speed.high" target="relocking"/>
  </state>
  <final id="locked"/>
</scxml>"##;

fn scratch(label: &str) -> PathBuf {
    let dir =
        PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("{label}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create scratch dir");
    dir
}

fn diagram(doc: &Path, out: &Path, extra: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_sce-codegen"))
        .args(["--error-format", "json"])
        .args(extra)
        .arg("diagram")
        .arg(doc)
        .arg("-o")
        .arg(out)
        .output()
        .expect("run sce-codegen diagram")
}

/// One SVG per container, named by the state it opens, each well-formed;
/// stdout names exactly the files written.
#[test]
fn writes_one_svg_per_figure_named_by_state() {
    let dir = scratch("diagram-writes");
    let doc = dir.join("lock.scxml");
    std::fs::write(&doc, LOCK).expect("write fixture");
    let out = dir.join("figures");
    let run = diagram(&doc, &out, &[]);
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );

    let printed: Vec<PathBuf> = String::from_utf8(run.stdout)
        .expect("utf-8")
        .lines()
        .map(PathBuf::from)
        .collect();
    let expected: Vec<PathBuf> = ["document", "inside-released", "inside-relocking"]
        .iter()
        .map(|s| out.join(format!("{s}.svg")))
        .collect();
    assert_eq!(printed, expected);

    let mut on_disk: Vec<PathBuf> = std::fs::read_dir(&out)
        .expect("read out dir")
        .map(|e| e.expect("entry").path())
        .collect();
    on_disk.sort();
    let mut sorted = expected.clone();
    sorted.sort();
    assert_eq!(on_disk, sorted, "nothing but the figures is written");

    for path in &expected {
        let svg = std::fs::read_to_string(path).expect("read svg");
        roxmltree::Document::parse(&svg).unwrap_or_else(|e| panic!("{path:?}: {e}"));
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// A second run over the same document leaves the same bytes, and
/// `--assert-unchanged` agrees; a hand edit to a figure fails it.
#[test]
fn the_written_figures_are_checkable_for_drift() {
    let dir = scratch("diagram-drift");
    let doc = dir.join("lock.scxml");
    std::fs::write(&doc, LOCK).expect("write fixture");
    let out = dir.join("figures");
    assert!(diagram(&doc, &out, &[]).status.success());

    let check = diagram(&doc, &out, &["--assert-unchanged"]);
    assert!(
        check.status.success(),
        "a fresh set is unchanged: {}",
        String::from_utf8_lossy(&check.stderr)
    );

    let figure = out.join("document.svg");
    let mut bytes = std::fs::read(&figure).expect("read");
    bytes.extend_from_slice(b"<!-- edited by hand -->\n");
    std::fs::write(&figure, bytes).expect("write");
    let check = diagram(&doc, &out, &["--assert-unchanged"]);
    assert!(!check.status.success(), "a hand edit is drift");
    let _ = std::fs::remove_dir_all(&dir);
}

/// A type size no page holds is refused as `cli/diagram-does-not-fit`,
/// naming the figure, with exit status 20 and no file written.
#[test]
fn a_figure_that_does_not_fit_is_refused_and_nothing_is_written() {
    let dir = scratch("diagram-refused");
    let doc = dir.join("lock.scxml");
    std::fs::write(&doc, LOCK).expect("write fixture");
    let out = dir.join("figures");
    let run = Command::new(env!("CARGO_BIN_EXE_sce-codegen"))
        .args(["--error-format", "json", "diagram"])
        .arg(&doc)
        .arg("-o")
        .arg(&out)
        .args(["--min-pt", "96"])
        .output()
        .expect("run sce-codegen diagram");
    assert_eq!(run.status.code(), Some(20));
    assert!(run.stdout.is_empty());
    let stderr = String::from_utf8(run.stderr).expect("utf-8");
    let record: serde_json::Value =
        serde_json::from_str(stderr.lines().next().expect("one record")).expect("json");
    assert_eq!(record["code"], "cli/diagram-does-not-fit", "{record}");
    assert!(
        record["message"]
            .as_str()
            .is_some_and(|m| m.contains("whole document")),
        "the message names the figure: {record}"
    );
    assert!(!out.exists(), "a refusal writes nothing");
    let _ = std::fs::remove_dir_all(&dir);
}

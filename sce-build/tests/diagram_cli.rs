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
    <history id="back" type="deep"><transition target="unlocked"/></history>
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

/// Given the specification's manifest, the command also writes the
/// requirement checklist after the figures, and every requirement of the
/// manifest has its row there — the missing ones saying no figure shows
/// them.
#[test]
fn a_manifest_adds_the_requirement_checklist() {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/requirement_closure");
    let doc = fixtures.join("doip_nl_connection_states.scxml");
    let manifest = fixtures.join("iso13400_2_nl_socket_handling.manifest.json");
    let dir = scratch("diagram-checklist");
    let out = dir.join("figures");
    let run = Command::new(env!("CARGO_BIN_EXE_sce-codegen"))
        .args(["--error-format", "json", "diagram"])
        .arg(&doc)
        .arg("-o")
        .arg(&out)
        .args(["--page", "a3-landscape", "--manifest"])
        .arg(&manifest)
        .output()
        .expect("run sce-codegen diagram");
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
    let sheets: Vec<&PathBuf> = printed
        .iter()
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("checklist-"))
        })
        .collect();
    assert_eq!(
        sheets.first().map(|p| p.as_path()),
        Some(out.join("checklist-1.svg").as_path())
    );
    assert!(
        printed.iter().position(|p| p == sheets[0])
            > printed.iter().position(|p| p.ends_with("document.svg")),
        "after the figures: {printed:?}"
    );

    let manifest_json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&manifest).expect("read")).expect("json");
    let ids: Vec<String> = manifest_json["requirements"]
        .as_array()
        .expect("a requirements list")
        .iter()
        .map(|r| r["id"].as_str().expect("an id").to_string())
        .collect();
    assert!(!ids.is_empty());
    let mut texts = Vec::new();
    for sheet in &sheets {
        let svg = std::fs::read_to_string(sheet).expect("read");
        let parsed = roxmltree::Document::parse(&svg)
            .expect("well-formed")
            .descendants()
            .filter(|n| n.has_tag_name("text"))
            .filter_map(|n| n.text().map(str::to_string))
            .collect::<Vec<_>>();
        texts.extend(parsed);
    }
    for id in &ids {
        assert!(texts.contains(id), "{id} has no row: {texts:?}");
    }
    assert!(
        texts.iter().any(|t| t == "not shown"),
        "a missing one says so"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

fn kind_example(kind: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("kind-examples")
        .join(format!("{kind}.scxml"))
}

/// `diagram` with options of its own after the output directory.
fn diagram_with(doc: &Path, out: &Path, options: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_sce-codegen"))
        .args(["--error-format", "json", "diagram"])
        .arg(doc)
        .arg("-o")
        .arg(out)
        .args(options)
        .output()
        .expect("run sce-codegen diagram")
}

fn svg_words(path: &Path) -> Vec<String> {
    let svg = std::fs::read_to_string(path).expect("read svg");
    roxmltree::Document::parse(&svg)
        .unwrap_or_else(|e| panic!("{path:?}: {e}"))
        .descendants()
        .filter(|n| n.has_tag_name("text"))
        .filter_map(|n| n.text().map(str::to_string))
        .collect()
}

/// A document that is not a statechart has no boxes to draw, so it is set
/// as the table of every value it states: `fields-<n>.svg`, well-formed,
/// carrying the document's own values, in the page's language, and
/// checkable for drift like a figure.
#[test]
fn a_document_of_another_kind_is_set_as_its_field_table() {
    let dir = scratch("diagram-fields");
    let out = dir.join("sheets");
    let run = diagram(&kind_example("lookup"), &out, &[]);
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
    assert_eq!(printed, vec![out.join("fields-1.svg")]);
    let words = svg_words(&printed[0]);
    assert_eq!(words[0], "field table: lookup");
    for expected in ["document.entries", "NONE", "LOW", "MEDIUM", "HIGH", "level"] {
        assert!(words.iter().any(|w| w == expected), "{expected}: {words:?}");
    }

    let check = diagram(&kind_example("lookup"), &out, &["--assert-unchanged"]);
    assert!(
        check.status.success(),
        "a fresh set is unchanged: {}",
        String::from_utf8_lossy(&check.stderr)
    );

    let korean = dir.join("korean");
    let run = diagram_with(&kind_example("lookup"), &korean, &["--lexicon", "ko"]);
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    let words = svg_words(&korean.join("fields-1.svg"));
    assert!(words[0].ends_with(": lookup"), "{words:?}");
    assert_ne!(
        words[0], "field table: lookup",
        "the title is in the page's language"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// The checklist's rows name a statechart's boxes and table rows, so a
/// manifest given with a document of another kind is refused by name and
/// nothing is written.
#[test]
fn a_manifest_is_refused_for_a_document_that_is_not_a_statechart() {
    let dir = scratch("diagram-fields-manifest");
    let out = dir.join("sheets");
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/requirement_closure/iso13400_2_nl_socket_handling.manifest.json");
    let run = diagram_with(
        &kind_example("lookup"),
        &out,
        &["--manifest", manifest.to_str().expect("utf-8")],
    );
    assert!(!run.status.success());
    assert!(run.stdout.is_empty());
    let stderr = String::from_utf8(run.stderr).expect("utf-8");
    let record: serde_json::Value =
        serde_json::from_str(stderr.lines().next().expect("one record")).expect("json");
    assert_eq!(record["code"], "cli/diagram-unavailable", "{record}");
    assert!(
        record["message"]
            .as_str()
            .is_some_and(|m| m.contains("lookup document")),
        "the message names the kind: {record}"
    );
    assert!(!out.exists(), "a refusal writes nothing");
    let _ = std::fs::remove_dir_all(&dir);
}

/// A table that cannot be set at the requested type size is refused as
/// `cli/diagram-does-not-fit`, naming the field table, with nothing
/// written.
#[test]
fn a_table_that_does_not_fit_is_refused_and_nothing_is_written() {
    let dir = scratch("diagram-fields-refused");
    let out = dir.join("sheets");
    let run = diagram_with(&kind_example("lookup"), &out, &["--min-pt", "96"]);
    assert_eq!(run.status.code(), Some(20));
    assert!(run.stdout.is_empty());
    let stderr = String::from_utf8(run.stderr).expect("utf-8");
    let record: serde_json::Value =
        serde_json::from_str(stderr.lines().next().expect("one record")).expect("json");
    assert_eq!(record["code"], "cli/diagram-does-not-fit", "{record}");
    assert!(
        record["message"]
            .as_str()
            .is_some_and(|m| m.contains("field table")),
        "the message names the table: {record}"
    );
    assert!(!out.exists(), "a refusal writes nothing");
    let _ = std::fs::remove_dir_all(&dir);
}

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

/// A parallel of many regions that share no arrow is one rank of boxes: set
/// in a single row it was wider than A3 (measured 2026-10-11 on a document
/// of seven regions, 1345 pt against 1106) and was refused. It is drawn now,
/// the regions in several rows, on the page that refused it and on a smaller
/// one, and every region is in the figure.
#[test]
fn a_parallel_of_many_regions_is_drawn_in_rows_on_the_page_that_refused_it() {
    let dir = scratch("diagram-regions");
    let doc = dir.join("regions.scxml");
    let mut body = String::from(
        r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="main"><parallel id="main">"#,
    );
    for i in 0..12 {
        body.push_str(&format!(
            r#"<state id="a_region_with_a_long_descriptive_name_{i}" initial="idle_{i}"><state id="idle_{i}"><transition event="go_{i}" target="busy_{i}"/></state><state id="busy_{i}"/></state>"#
        ));
    }
    body.push_str("</parallel></scxml>");
    std::fs::write(&doc, body).expect("write fixture");
    for page in ["a4-portrait", "a3-landscape"] {
        let out = dir.join(page);
        let run = diagram_with(&doc, &out, &["--page", page]);
        assert_eq!(run.status.code(), Some(0), "{page}: {run:?}");
        let svg =
            std::fs::read_to_string(out.join("inside-main.svg")).expect("the parallel's figure");
        for i in 0..12 {
            assert!(
                svg.contains(&format!("a_region_with_a_long_descriptive_name_{i}")),
                "{page}: region {i} is in the figure"
            );
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// `--page fit` draws the set on the smallest page of its ladder that holds
/// it, byte for byte as that page named would; at the type size the default
/// page holds it is the default page; and where no page holds it, the refusal
/// is the largest page's. Measured on this fixture: at 16 pt only A3 holds
/// it, at 20 pt only A3 landscape, at 24 pt nothing.
#[test]
fn fit_draws_on_the_smallest_page_that_holds_the_set_and_never_shrinks_the_type() {
    let dir = scratch("diagram-fit");
    let doc = dir.join("lock.scxml");
    std::fs::write(&doc, LOCK).expect("write fixture");
    let files = |out: &Path| -> Vec<(String, String)> {
        let mut found: Vec<(String, String)> = std::fs::read_dir(out)
            .expect("read out dir")
            .map(|e| {
                let path = e.expect("entry").path();
                let name = path
                    .file_name()
                    .expect("name")
                    .to_string_lossy()
                    .into_owned();
                (name, std::fs::read_to_string(&path).expect("svg"))
            })
            .collect();
        found.sort();
        found
    };
    for (pt, page) in [
        ("7", "a4-portrait"),
        ("16", "a3-portrait"),
        ("20", "a3-landscape"),
    ] {
        let fitted = dir.join(format!("fit-{pt}"));
        let named = dir.join(format!("{page}-{pt}"));
        let run = diagram_with(&doc, &fitted, &["--page", "fit", "--min-pt", pt]);
        assert_eq!(run.status.code(), Some(0), "fit at {pt} pt: {run:?}");
        let run = diagram_with(&doc, &named, &["--page", page, "--min-pt", pt]);
        assert_eq!(run.status.code(), Some(0), "{page} at {pt} pt: {run:?}");
        assert_eq!(files(&fitted), files(&named), "fit at {pt} pt is {page}");
    }
    // The nearest page below the one chosen refuses it: the ladder was climbed, not skipped.
    let run = diagram_with(
        &doc,
        &dir.join("a3-portrait-20"),
        &["--page", "a3-portrait", "--min-pt", "20"],
    );
    assert_eq!(run.status.code(), Some(20), "{run:?}");

    let out = dir.join("fit-24");
    let run = diagram_with(&doc, &out, &["--page", "fit", "--min-pt", "24"]);
    assert_eq!(run.status.code(), Some(20), "{run:?}");
    let stderr = String::from_utf8(run.stderr).expect("utf-8");
    let record: serde_json::Value =
        serde_json::from_str(stderr.lines().next().expect("one record")).expect("json");
    assert_eq!(record["code"], "cli/diagram-does-not-fit", "{record}");
    assert!(
        record["message"]
            .as_str()
            .is_some_and(|m| m.contains("the page gives 1106 x 757 pt")),
        "the refusal is the largest page's: {record}"
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

/// The paths a run printed, one a line.
fn written(run: &Output) -> Vec<PathBuf> {
    String::from_utf8(run.stdout.clone())
        .expect("utf-8")
        .lines()
        .map(PathBuf::from)
        .collect()
}

/// The lookup example with one of its values replaced.
fn lookup_with_value(dir: &Path, value: &str) -> PathBuf {
    let text = std::fs::read_to_string(kind_example("lookup")).expect("read the example");
    let path = dir.join("symbols.scxml");
    std::fs::write(
        &path,
        text.replace(r#"value="HIGH""#, &format!(r#"value="{value}""#)),
    )
    .expect("write the document");
    path
}

/// A value with a unit, an operator, a check mark or a Hanja term is drawn,
/// with the characters on the sheet; a character the font has no glyph for is
/// refused by name and nothing is written. Measured 2026-10-03: these were all
/// refused whole until the font table covered them.
#[test]
fn symbols_a_specification_holds_are_drawn_and_a_glyphless_one_is_refused_by_name() {
    let dir = scratch("diagram-symbols");
    for value in [
        "10 \u{03A9}",
        "\u{2265} 5 \u{03BC}s",
        "\u{2713} ok",
        "\u{6F22}\u{5B57}",
    ] {
        let out = dir.join("drawn");
        let run = diagram(&lookup_with_value(&dir, value), &out, &[]);
        assert!(
            run.status.success(),
            "{value}: {}",
            String::from_utf8_lossy(&run.stderr)
        );
        let shown: Vec<String> = written(&run).iter().flat_map(|p| svg_words(p)).collect();
        assert!(
            shown.iter().any(|w| w == value),
            "{value} is on a sheet: {shown:?}"
        );
        let _ = std::fs::remove_dir_all(&out);
    }

    let out = dir.join("refused");
    let run = diagram(&lookup_with_value(&dir, "\u{1F600}"), &out, &[]);
    assert_eq!(run.status.code(), Some(20));
    let stderr = String::from_utf8(run.stderr).expect("utf-8");
    let record: serde_json::Value =
        serde_json::from_str(stderr.lines().next().expect("one record")).expect("json");
    assert_eq!(record["code"], "cli/diagram-unavailable", "{record}");
    assert!(
        record["message"]
            .as_str()
            .is_some_and(|m| m.contains("U+1F600")),
        "the refusal names the character: {record}"
    );
    assert!(!out.exists(), "a refusal writes nothing");
    let _ = std::fs::remove_dir_all(&dir);
}

/// A document that is not a statechart has no boxes to draw, so it is set
/// as the table of every value it states: `fields-<n>.svg`, well-formed,
/// carrying the document's own values, in the page's language, and
/// checkable for drift like a figure. (An algorithm has no picture of its
/// own, so the table is all it gets.)
#[test]
fn a_document_of_another_kind_is_set_as_its_field_table() {
    let dir = scratch("diagram-fields");
    let out = dir.join("sheets");
    let run = diagram(&kind_example("algorithm"), &out, &[]);
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
    assert_eq!(words[0], "field table: algorithm");
    for expected in ["document.signature", "data", "bytes", "uint32"] {
        assert!(words.iter().any(|w| w == expected), "{expected}: {words:?}");
    }

    let check = diagram(&kind_example("algorithm"), &out, &["--assert-unchanged"]);
    assert!(
        check.status.success(),
        "a fresh set is unchanged: {}",
        String::from_utf8_lossy(&check.stderr)
    );

    let korean = dir.join("korean");
    let run = diagram_with(&kind_example("algorithm"), &korean, &["--lexicon", "ko"]);
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    let words = svg_words(&korean.join("fields-1.svg"));
    assert!(words[0].ends_with(": algorithm"), "{words:?}");
    assert_ne!(
        words[0], "field table: algorithm",
        "the title is in the page's language"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// The figure of a statechart takes the `plain` lexicon: the same
/// English phrases around the figure, and the page's plain words in its
/// boxes and table, because the figure writes an action with the page's
/// own words.
#[test]
fn a_statechart_is_drawn_in_the_plain_lexicon() {
    let dir = scratch("diagram-plain");
    let doc = dir.join("lock.scxml");
    std::fs::write(
        &doc,
        r##"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="off">
  <state id="off">
    <transition event="lock" target="on"><raise event="eval"/></transition>
  </state>
  <state id="on"/>
</scxml>"##,
    )
    .expect("write the document");
    let canonical = dir.join("canonical");
    let plain = dir.join("plain");
    let default_run = diagram_with(&doc, &canonical, &[]);
    assert!(
        default_run.status.success(),
        "{}",
        String::from_utf8_lossy(&default_run.stderr)
    );
    let run = diagram_with(&doc, &plain, &["--lexicon", "plain"]);
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    let words = svg_words(&plain.join("document.svg"));
    assert!(
        words.iter().any(|w| w.contains("tell itself eval")),
        "{words:?}"
    );
    assert!(!words.iter().any(|w| w.contains("raise")), "{words:?}");
    let default_words = svg_words(&canonical.join("document.svg"));
    assert!(
        default_words.iter().any(|w| w.contains("raise eval")),
        "{default_words:?}"
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
    let run = diagram_with(&kind_example("algorithm"), &out, &["--min-pt", "96"]);
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

/// A picture that cannot be set is refused by its own title, and the table
/// beside it is not written either: a set missing a sheet reads like a
/// complete one.
#[test]
fn a_picture_that_does_not_fit_is_refused_by_name_and_nothing_is_written() {
    let dir = scratch("diagram-picture-refused");
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
            .is_some_and(|m| m.contains("lookup: mapping")),
        "the message names the picture: {record}"
    );
    assert!(!out.exists(), "a refusal writes nothing");
    let _ = std::fs::remove_dir_all(&dir);
}

/// A kind that has a picture gets it beside the table, written first and
/// named for what it shows; the table is still there.
#[test]
fn a_kind_with_a_picture_gets_it_beside_its_field_table() {
    let dir = scratch("diagram-picture");
    let out = dir.join("sheets");
    let run = diagram(&kind_example("buffer-pool"), &out, &[]);
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
    assert_eq!(
        printed,
        vec![out.join("slots.svg"), out.join("fields-1.svg")]
    );
    let words = svg_words(&printed[0]);
    for expected in ["8 slots", "256 bytes", "2048 bytes"] {
        assert!(words.iter().any(|w| w == expected), "{expected}: {words:?}");
    }
    let table = svg_words(&printed[1]);
    assert!(
        table.iter().any(|w| w == "slot_size") && table.iter().any(|w| w == "256"),
        "the table still holds what the picture draws: {table:?}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// The width and height an SVG's header declares, in points.
fn canvas(path: &Path) -> (f64, f64) {
    let svg = std::fs::read_to_string(path).expect("read svg");
    let doc = roxmltree::Document::parse(&svg).expect("well-formed");
    let size = |name: &str| {
        doc.root_element()
            .attribute(name)
            .and_then(|v| v.strip_suffix("pt"))
            .and_then(|v| v.parse::<f64>().ok())
            .unwrap_or_else(|| panic!("{path:?} declares no {name} in points"))
    };
    (size("width"), size("height"))
}

/// `--margin` writes paper around every figure of every kind: each SVG is
/// twice the margin larger each way, and what it draws is the figure it
/// drew without one. The default, zero, writes the bytes the command always
/// wrote.
#[test]
fn a_margin_is_written_around_every_figure_of_every_kind() {
    let dir = scratch("diagram-margin");
    let lock = dir.join("lock.scxml");
    std::fs::write(&lock, LOCK).expect("write fixture");
    let documents = [lock, kind_example("lookup"), kind_example("buffer-pool")];
    for (n, doc) in documents.iter().enumerate() {
        let (plain, zero, framed) = (
            dir.join(format!("plain-{n}")),
            dir.join(format!("zero-{n}")),
            dir.join(format!("framed-{n}")),
        );
        assert!(diagram_with(doc, &plain, &[]).status.success());
        assert!(diagram_with(doc, &zero, &["--margin", "0"])
            .status
            .success());
        let run = diagram_with(doc, &framed, &["--margin", "12"]);
        assert!(
            run.status.success(),
            "{}",
            String::from_utf8_lossy(&run.stderr)
        );
        let files = written(&run);
        assert!(!files.is_empty());
        for path in files {
            let name = path.file_name().expect("a file name");
            assert_eq!(
                std::fs::read(plain.join(name)).expect("plain"),
                std::fs::read(zero.join(name)).expect("zero"),
                "{name:?}: a margin of zero is what the command always wrote"
            );
            let (pw, ph) = canvas(&plain.join(name));
            let (fw, fh) = canvas(&path);
            assert!(
                (fw - (pw + 24.0)).abs() < 0.02 && (fh - (ph + 24.0)).abs() < 0.02,
                "{name:?}: {pw}x{ph} became {fw}x{fh}"
            );
            assert_eq!(svg_words(&plain.join(name)), svg_words(&path), "{name:?}");
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// The margin is spent from the page: a figure that fits with none is
/// refused when the paper around it would not fit, and a margin no page can
/// hold is a usage error. Nothing is written in either case.
#[test]
fn a_margin_is_counted_in_the_fit_and_a_margin_no_page_holds_is_refused() {
    let dir = scratch("diagram-margin-fit");
    let doc = dir.join("lock.scxml");
    std::fs::write(&doc, LOCK).expect("write fixture");

    let out = dir.join("a");
    let run = diagram_with(&doc, &out, &["--margin", "250"]);
    assert_eq!(
        run.status.code(),
        Some(20),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    let record: serde_json::Value = serde_json::from_str(
        String::from_utf8(run.stderr)
            .expect("utf-8")
            .lines()
            .next()
            .expect("a record"),
    )
    .expect("json");
    assert_eq!(record["code"], "cli/diagram-does-not-fit", "{record}");
    assert!(!out.exists(), "a refusal writes nothing");

    for bad in ["-1", "nan", "1000"] {
        let out = dir.join(format!("b{bad}"));
        // `=` so that a negative number is a value and not an option.
        let run = diagram_with(&doc, &out, &[&format!("--margin={bad}")]);
        assert!(!run.status.success(), "--margin {bad} is refused");
        let stderr = String::from_utf8_lossy(&run.stderr);
        assert!(stderr.contains("--margin"), "--margin {bad}: {stderr}");
        assert!(!out.exists(), "--margin {bad} writes nothing");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

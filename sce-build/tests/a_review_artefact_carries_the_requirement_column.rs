//! NL→IR closure ledger row G3 — a review artefact with the requirement
//! column, for a kind family that is not the statechart.
//!
//! `transition_table_closure.rs` is this file's sibling and asserts the
//! same two readings for the statechart family. Until this landed the
//! other kinds could not have had such an artefact: `sce:req` was
//! refused on every `sce:`-namespace element and read by nobody where it
//! was admitted, so the column would have been `(none)` on every row for
//! a reason that has nothing to do with the document.
//!
//! The readings asserted here, each with a non-empty example:
//!
//!   (a) rows whose `source` is `(none)` — a mapping row nobody
//!       required, which is the same finding an unannotated transition
//!       is on the statechart side;
//!   (b) requirements a manifest declares that no row claims.
//!
//! # And the root of every kind
//!
//! Every document's first row is its root, where it claims a requirement
//! as a whole (docs/adr/0106, item G3). A kind with no finer claim site —
//! a `condition` — answers with that row alone, so there is no kind SCE
//! gives nowhere to put a claim, and no empty table to mistake for a clean
//! review.
//!
//! ⚠ The fixtures are written by this test rather than checked in, so
//! the mutant and its control cannot drift apart in the tree, and so a
//! fixture cannot be edited into agreement with an artefact that stopped
//! reporting.

use std::path::PathBuf;
use std::process::Command;

/// Five mapping rows; one claims a requirement, four claim nothing.
const LOOKUP: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="lookup" name="gear_position_lookup">
  <datamodel>
    <data id="gearRaw" sce:type="uint8" sce:direction="in"/>
    <data id="gear" sce:type="string" sce:direction="out"/>
    <data id="mapping" sce:default="NEUTRAL">
      <sce:entry key="0" value="PARK"/>
      <sce:entry key="1" value="REVERSE"/>
      <sce:entry key="2" value="NEUTRAL"/>
      <sce:entry key="3" value="DRIVE" sce:req="REQ-DRIVE"/>
      <sce:entry key="4" value="SPORT"/>
    </data>
  </datamodel>
</scxml>
"#;

/// A kind with no claim site but its root.
const CONDITION: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="condition" name="rpm_in_range">
  <datamodel>
    <data id="rpm" sce:type="uint32" sce:direction="in"/>
    <data id="result" sce:type="bool" sce:direction="out" expr="rpm &gt;= 10"/>
  </datamodel>
</scxml>
"#;

fn write(dir: &std::path::Path, name: &str, body: &str) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, body).expect("the fixture is writable");
    path
}

fn review_table(doc: &std::path::Path) -> std::process::Output {
    Command::new(PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen")))
        .arg("review-table")
        .arg(doc)
        .output()
        .expect("sce-codegen runs")
}

fn rows(out: &std::process::Output) -> Vec<serde_json::Value> {
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).expect("each line is one JSON record"))
        .collect()
}

fn tmpdir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "sce-g3-{tag}-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    std::fs::create_dir_all(&dir).expect("the temp dir is creatable");
    dir
}

#[test]
fn a_lookup_row_carries_the_requirement_it_claims() {
    let dir = tmpdir("claims");
    let doc = write(&dir, "lookup.scxml", LOOKUP);

    let out = review_table(&doc);
    assert!(
        out.status.success(),
        "review-table refused a lookup: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let rows = rows(&out);
    // Arity floor: the readings below are about WHICH rows say what, and
    // an empty or one-row table would satisfy them vacuously.
    assert_eq!(
        rows.len(),
        6,
        "the document's root row, then one row per mapping entry, got {rows:?}"
    );
    assert_eq!(rows[0]["node"], "document", "the root's row comes first");

    let claimed: Vec<&serde_json::Value> =
        rows.iter().filter(|r| r["source"] != "(none)").collect();
    assert_eq!(claimed.len(), 1, "exactly one row claims anything");
    assert_eq!(claimed[0]["source"], "REQ-DRIVE");
    assert_eq!(claimed[0]["detail"], "3 -> DRIVE");
    assert_eq!(claimed[0]["node"], "entries[key=3]");
}

#[test]
fn reading_a_the_unclaimed_rows_collect_under_one_word() {
    let dir = tmpdir("none");
    let doc = write(&dir, "lookup.scxml", LOOKUP);
    let rows = rows(&review_table(&doc));

    let unclaimed: Vec<&serde_json::Value> =
        rows.iter().filter(|r| r["source"] == "(none)").collect();
    assert_eq!(
        unclaimed.len(),
        5,
        "the root, which claims nothing, and four mapping rows nobody required, got {rows:?}"
    );
    // The word, not an empty cell: an empty string sorts with whatever
    // else is empty and reads as a rendering slip, while `(none)` sorts
    // as one block and reads as a finding. Same rule as the statechart
    // table, and asserted here so the two cannot drift.
    assert_eq!(
        sce_build::forge::review_table::NO_SOURCE,
        sce_build::transition_table::NO_SOURCE,
        "the two artefacts must collect under the SAME word"
    );
}

#[test]
fn reading_b_a_declared_requirement_no_row_claims_is_missing() {
    use sce_build::forge::review_table::{missing_requirements, ReviewRow};

    let dir = tmpdir("missing");
    let doc = write(&dir, "lookup.scxml", LOOKUP);
    let parsed: Vec<serde_json::Value> = rows(&review_table(&doc));
    let rows: Vec<ReviewRow> = parsed
        .iter()
        .map(|r| ReviewRow {
            source: r["source"]
                .as_str()
                .expect("source is a string")
                .to_string(),
            node: r["node"].as_str().expect("node is a string").to_string(),
            // Not read back from the wire: the column is a `&'static str`
            // and this reading does not depend on it.
            node_type: "entry",
            detail: r["detail"]
                .as_str()
                .expect("detail is a string")
                .to_string(),
        })
        .collect();

    let missing = missing_requirements(&rows, ["REQ-DRIVE", "REQ-PARK"].into_iter());
    assert_eq!(
        missing,
        vec!["REQ-PARK".to_string()],
        "REQ-PARK is declared and claimed by no row; REQ-DRIVE is claimed"
    );

    // ⭐ The control. A predicate that reported everything missing would
    // pass the assertion above, so the claimed id must NOT appear.
    let none_missing = missing_requirements(&rows, ["REQ-DRIVE"].into_iter());
    assert!(
        none_missing.is_empty(),
        "a requirement a row does claim must not be reported missing, got {none_missing:?}"
    );
}

/// Every kind claims on its root (docs/adr/0106, item G3): a condition,
/// which has no finer claim site, answers with its root's row, claimed or
/// `(none)`. It was refused as a kind with no annotation site until
/// 2026-11-16, and a claim written on its root was refused as unread.
#[test]
fn a_kind_with_no_finer_claim_site_answers_with_its_root() {
    let dir = tmpdir("rootSite");
    let unclaimed = write(&dir, "condition.scxml", CONDITION);
    let out = review_table(&unclaimed);
    assert!(
        out.status.success(),
        "review-table refused a condition: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let root_only = rows(&out);
    assert_eq!(root_only.len(), 1, "{root_only:?}");
    assert_eq!(root_only[0]["node"], "document");
    assert_eq!(root_only[0]["source"], "(none)");

    let claimed = write(
        &dir,
        "claimed.scxml",
        &CONDITION.replacen(
            "sce:kind=\"condition\"",
            "sce:kind=\"condition\" sce:req=\"REQ-GATE\"",
            1,
        ),
    );
    let claimed_rows = rows(&review_table(&claimed));
    assert_eq!(claimed_rows.len(), 1, "{claimed_rows:?}");
    assert_eq!(claimed_rows[0]["source"], "REQ-GATE");
}

#[test]
fn the_claim_is_read_through_the_same_reader_as_the_statechart_family() {
    let dir = tmpdir("dup");
    // The duplicate rule is the statechart parser's, and it must fire on
    // a forge node too — the tell that both families go through one
    // `collect_sce_req` rather than two readers free to diverge.
    let doc = write(
        &dir,
        "lookup.scxml",
        &LOOKUP.replace(r#"sce:req="REQ-DRIVE""#, r#"sce:req="REQ-DRIVE REQ-DRIVE""#),
    );

    let out = review_table(&doc);
    assert!(
        !out.status.success(),
        "a requirement id repeated on one node must be refused"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("duplicate sce:req id") && stderr.contains("REQ-DRIVE"),
        "the forge side must raise the statechart side's own duplicate \
         diagnostic, got: {stderr}"
    );
}

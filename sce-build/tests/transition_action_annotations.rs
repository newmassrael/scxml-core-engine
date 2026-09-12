//! HOLE-3 — `sce:req` on a transition's own action is read.
//!
//! The walk in `requirements_report` descended into `on_entry_blocks`,
//! `on_exit_blocks` and `invokes`, and took a `<transition>` whole. An
//! annotation on executable content *inside* a transition therefore
//! reached the IR, was emitted by every backend, and was read by
//! nobody — absent from the requirements report, absent from the trace
//! table, and reported `missing` by the manifest comparison however
//! carefully it had been written.
//!
//! # Why this file asserts THREE readings and not one
//!
//! The gap was in the walk, not the parse, and `walk_nodes` is THE
//! traversal: the report, the manifest classification and the
//! transition table all sit on it rather than re-walking. One fix
//! therefore moves all three, and a test that checked only the report
//! would leave the other two asserted by nothing. What this file pins
//! is the property that makes them one export — **all three name the
//! same `node_path` for the same node**.
//!
//! # The witness is committed, not written for the test
//!
//! `codegen_smoke/sce_annotations.scxml` already carries
//! `REQ_TRANS_LOG` on a `<log>` inside a `<transition>`, and
//! `sce_annotation_emission.rs` already requires every backend to
//! emit that token. So before this fix the tree asserted that
//! annotation reached generated source while its own coverage
//! machinery could not see it: two halves disagreeing about whether
//! one requirement was implemented, with only the coverage half
//! wrong. Measured over all 733 tracked `.scxml`, it was 1 of 32
//! `sce:req` annotations — small, and a silent wrong answer rather
//! than a crash.

use sce_build::parser::SCXMLParser;
use sce_build::requirement_manifest::{classify, Outcome, RequirementManifest};
use sce_build::requirements_report::emit_requirements_ndjson;
use sce_build::transition_table::transition_table;

/// The committed witness — the annotation site this hole hid.
const WITNESS: &str = "tests/fixtures/codegen_smoke/sce_annotations.scxml";

/// The node the three readings must agree on.
const TRANSITION_ACTION: &str = "states.s0.transitions[0].actions[0]";

fn witness_model() -> sce_build::model::SCXMLModel {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(WITNESS);
    let raw = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "witness fixture must be readable at {}: {e}",
            path.display()
        )
    });
    SCXMLParser::new()
        .parse_string(&raw, "sce_annotations")
        .unwrap_or_else(|e| panic!("witness fixture must parse: {:?}", e.error))
}

fn report_lines(model: &sce_build::model::SCXMLModel) -> Vec<serde_json::Value> {
    let mut out = Vec::new();
    emit_requirements_ndjson(model, &mut out).expect("report writes to a Vec");
    String::from_utf8(out)
        .expect("utf-8 report")
        .lines()
        .map(|l| serde_json::from_str(l).expect("each report line is JSON"))
        .collect()
}

/// All three readings reach the transition's own action, and name it
/// identically.
#[test]
fn the_report_the_table_and_the_classifier_all_see_a_transitions_own_action() {
    let model = witness_model();

    // ① the requirements report
    let records = report_lines(&model);
    let reported: Vec<&serde_json::Value> = records
        .iter()
        .filter(|r| r["node_path"] == TRANSITION_ACTION)
        .collect();
    assert_eq!(
        reported.len(),
        1,
        "the report must carry exactly one record for {TRANSITION_ACTION}; \
         got {}. Before HOLE-3 was closed it carried none, and the \
         annotation was invisible to every consumer",
        reported.len(),
    );
    assert_eq!(
        reported[0]["requirement_ids"][0], "REQ_TRANS_LOG",
        "the record must carry the id the fixture writes",
    );

    // ② the trace table
    let rows = transition_table(&model);
    let row = rows
        .iter()
        .find(|row| row.node_path == TRANSITION_ACTION)
        .unwrap_or_else(|| {
            panic!(
                "the table must carry a row for {TRANSITION_ACTION}. Its own \
                 rule is one row per node that can carry a requirement — \
                 without this row, \"a requirement with no row is missing\" \
                 would call an annotated transition action missing"
            )
        });
    assert_eq!(
        row.event, "(transition)",
        "the pseudo-event must say which of the three sites this is, or a \
         reader cannot tell it from an `(entry)` action on the same state",
    );
    assert_eq!(row.source, "REQ_TRANS_LOG");

    // ③ the manifest classification
    let manifest = RequirementManifest::from_json(
        r#"{ "doc_id": "probe-spec", "rev": "A",
             "requirements": [{ "id": "REQ_TRANS_LOG" }] }"#,
        "probe",
    )
    .expect("probe manifest loads");
    let classification = classify(&model, &manifest);
    let verdict = classification
        .outcomes
        .iter()
        .find(|o| o.id == "REQ_TRANS_LOG")
        .expect("the declared requirement is classified");
    assert_eq!(
        verdict.node_paths,
        vec![TRANSITION_ACTION.to_string()],
        "the classifier must name the same node the other two do; when the \
         three disagree about a node_path they have stopped being readings \
         of one walk",
    );
    assert_ne!(
        verdict.outcome,
        Outcome::Missing,
        "before HOLE-3 was closed this read `missing` — the annotation was \
         there, correct, and reported as silently dropped",
    );

    println!(
        "HOLE-3: {} report record(s), {} table row(s), {} outcome(s) agree on {TRANSITION_ACTION}",
        reported.len(),
        1,
        verdict.node_paths.len(),
    );
}

/// ⭐ The witness carries `sce:unresolved` on that same node, so the
/// honest verdict is `unresolved` — not `implemented`, and not the
/// `missing` it used to be.
///
/// Worth its own assertion because the `unresolved` flag travelled the
/// same blind path: a transition action's "I did not know" was as
/// invisible as its requirement id, so a reviewer was never sent to
/// look at a decision the author had explicitly deferred.
#[test]
fn an_unresolved_marker_on_a_transition_action_reaches_the_verdict() {
    let model = witness_model();
    let manifest = RequirementManifest::from_json(
        r#"{ "doc_id": "probe-spec", "rev": "A",
             "requirements": [{ "id": "REQ_TRANS_LOG" }] }"#,
        "probe",
    )
    .expect("probe manifest loads");

    let classification = classify(&model, &manifest);
    let verdict = classification
        .outcomes
        .iter()
        .find(|o| o.id == "REQ_TRANS_LOG")
        .expect("classified");
    assert_eq!(
        verdict.outcome,
        Outcome::Unresolved,
        "the fixture writes `sce:unresolved=\"TBD_LOG_LABEL\"` on that \
         action, so the only node citing this requirement is an open \
         question and the verdict has to say so",
    );
    println!("HOLE-3: the unresolved marker on a transition action is read");
}

/// The sweep: what the walk now reaches, counted and floored.
///
/// ⚠ Without the floor this file would pass over a fixture that had
/// stopped carrying annotations at all — an empty walk and a complete
/// one print the same green, which is the shape this repository keeps
/// being caught by.
#[test]
fn the_walk_reaches_every_annotation_site_and_says_how_many() {
    let model = witness_model();
    let rows = transition_table(&model);

    let mut by_site = std::collections::BTreeMap::new();
    for row in &rows {
        *by_site.entry(row.event.clone()).or_insert(0usize) += 1;
    }
    let annotated = rows.iter().filter(|row| !row.is_unclaimed()).count();

    println!(
        "HOLE-3: {} table row(s) over the witness; annotated {annotated}",
        rows.len()
    );
    for (event, n) in &by_site {
        println!("    {event}: {n}");
    }

    for site in ["(state)", "(transition)", "(entry)", "(exit)"] {
        assert!(
            by_site.contains_key(site),
            "the walk reached no `{site}` node. All four sites exist in the \
             witness, so a missing one means the traversal stopped \
             descending somewhere — which is exactly the defect HOLE-3 was",
        );
    }
    assert!(
        annotated >= 5,
        "only {annotated} annotated row(s); the witness carries an id on a \
         state, a transition, a transition's action, an onentry action and \
         an onexit action, so below five the fixture has been gutted or the \
         walk has narrowed again",
    );
}

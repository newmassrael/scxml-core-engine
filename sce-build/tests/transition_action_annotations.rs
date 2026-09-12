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
             "extraction": { "ids": "native", "trace": "none",
                             "modality_convention": "english-modal-verbs",
                             "method": "hand" },
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
             "extraction": { "ids": "native", "trace": "none",
                             "modality_convention": "english-modal-verbs",
                             "method": "hand" },
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

/// ⭐⭐ The fourth reading, and the one with the worst failure mode.
///
/// `classify` derives its revision-staleness note from the
/// `sce:provenance` anchors the same walk yields. An anchor sitting
/// only on a transition's own action was therefore invisible too — so
/// a document citing a superseded revision **there** was compared
/// against a stale manifest in silence, and the comparison came back
/// clean. `requirement_manifest`'s own doc says the staleness check is
/// worth more than the comparison it qualifies, which is exactly why
/// this gets its own assertion rather than being assumed to follow
/// from the `sce:req` case.
#[test]
fn a_provenance_anchor_on_a_transition_action_reaches_the_staleness_check() {
    let doc = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
                        xmlns:sce="http://sce.dev/ext" version="1.0"
                        name="prov" initial="s0">
                   <state id="s0">
                     <transition event="go" target="done">
                       <raise event="e" sce:req="REQ_A"
                              sce:provenance="probe-spec@D3#1.1"/>
                     </transition>
                   </state>
                   <final id="done"/>
                 </scxml>"#;
    let model = SCXMLParser::new()
        .parse_string(doc, "prov")
        .unwrap_or_else(|e| panic!("probe parses: {:?}", e.error));
    let manifest = RequirementManifest::from_json(
        r#"{ "doc_id": "probe-spec", "rev": "D4",
             "extraction": { "ids": "native", "trace": "none",
                             "modality_convention": "english-modal-verbs",
                             "method": "hand" },
             "requirements": [{ "id": "REQ_A" }] }"#,
        "probe",
    )
    .expect("probe manifest loads");

    let note = classify(&model, &manifest).revision_note.expect(
        "the anchor sits only on the transition's own action, and the \
         manifest is a revision ahead of it — without the note, a document \
         is scored against a denominator that no longer describes it and \
         nothing says so",
    );
    assert!(
        note.contains("D3") && note.contains("D4"),
        "the note must name both revisions so a reader can tell which way \
         the drift runs; got: {note}",
    );
    println!("HOLE-3: a transition-action anchor reaches the staleness note");
}

/// The same claim at the wire, which is what a consumer reads.
///
/// ⚠ The three checks above drive the library. A consumer runs the
/// command, and this repository has twice shipped a check that was
/// green at the library and blind at the wire, so the wire gets its
/// own assertion rather than an argument that it must follow.
#[test]
fn the_command_reports_a_transitions_own_action() {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(WITNESS);
    let output =
        std::process::Command::new(std::path::PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen")))
            .arg("requirements")
            .arg(&path)
            .output()
            .expect("sce-codegen runs");
    assert!(
        output.status.success(),
        "sce-codegen requirements failed: {}",
        String::from_utf8_lossy(&output.stderr),
    );
    let stdout = String::from_utf8(output.stdout).expect("utf-8 stdout");
    assert!(
        stdout.contains(&format!("\"node_path\":\"{TRANSITION_ACTION}\"")),
        "the emitted report carries no record for {TRANSITION_ACTION}.\n{stdout}",
    );
    assert!(
        stdout.contains("REQ_TRANS_LOG"),
        "the emitted report does not carry the id the fixture writes.\n{stdout}",
    );
    println!("HOLE-3: the command reports the transition's own action");
}

/// ⭐⭐⭐ What generated HOLE-3, guarded — not the instance it took.
///
/// The defect was never "transition actions were forgotten". It was
/// that **two enumerations of where annotations live** existed with
/// nothing tying them together: the parser (and every codegen
/// template) knew about a transition's own actions, and [`walk_nodes`]
/// did not. Nothing compared the two, so they drifted, and the drift
/// was found by accident four months later.
///
/// This sweep removes the need for anyone to notice. It reads every
/// `sce:req` a document in the working tree *declares*, textually and
/// without going through the walk, and requires the walk to yield each
/// one. A site the parser learns about and the walk does not now fails
/// here, on the day it is added, whatever site it is.
///
/// ⚠ The working tree, not `git ls-files` — so generated trees are
/// swept too and the document count differs between machines (1409
/// locally, 2556 on the build host, which carries generated output).
/// That is deliberate: for a reachability check a superset is the safe
/// direction, and the annotation-bearing set was identical on both.
///
/// ⚠ The document is the independent source on purpose. Comparing the
/// walk against another hand-kept list would be a third enumeration to
/// drift; comparing it against the text cannot drift, because the text
/// is what an author wrote.
///
/// ⚠⚠ What it does NOT cover, stated so it is not mistaken for total:
/// this sweep is **corpus-driven**, so it catches a site the moment
/// some document annotates it, and not before. Measured over the
/// working tree, the corpus annotates three kinds of node — `state`
/// (9), `transition` (16) and `action` (6) — and **no document
/// annotates an `<invoke>`**. A regression that made invoke
/// annotations unreachable would therefore pass here. That site is
/// held by the unit test in `requirements_report.rs`, which asserts
/// one record for each of state, onentry action, transition and
/// invoke. The two checks are complements: this one generalises over
/// sites nobody enumerated, that one covers a site nobody has used
/// yet.
#[test]
fn the_walk_reaches_every_annotation_any_document_declares() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf();

    let mut documents = Vec::new();
    collect_scxml(&root, &mut documents);
    assert!(
        documents.len() >= 400,
        "found only {} .scxml under {}; the scan resolved far less than \
         the tree holds and would pass while checking almost nothing",
        documents.len(),
        root.display(),
    );

    let mut files_with_ids = 0usize;
    let mut declared_total = 0usize;
    let mut unreachable: Vec<String> = Vec::new();
    let mut unparsable = 0usize;

    for path in &documents {
        let Ok(text) = std::fs::read_to_string(path) else {
            continue;
        };
        let declared = declared_req_ids(&text);
        if declared.is_empty() {
            continue;
        }
        files_with_ids += 1;
        declared_total += declared.len();

        let Ok(model) = SCXMLParser::new().parse_string(&text, "sweep") else {
            // ⚠ Reached only by a document that DECLARES ids and then
            // will not parse. Its annotations cannot be checked at all,
            // so this is an unverified population rather than a clean
            // one, and the assertion below refuses to call it a pass.
            // (A malformed fixture carrying no ids never gets here —
            // it is skipped above, which is why this counter reads 0
            // while the tree does contain one on purpose.)
            unparsable += 1;
            continue;
        };
        let reached: std::collections::BTreeSet<String> = report_lines(&model)
            .iter()
            .filter_map(|r| r["requirement_ids"].as_array().cloned())
            .flatten()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect();
        for id in declared {
            if !reached.contains(&id) {
                unreachable.push(format!("{}: {id}", path.display()));
            }
        }
    }

    println!(
        "HOLE-3 sweep: {} document(s), {files_with_ids} carrying ids, \
         {declared_total} declared id(s), {unparsable} of those unparsable",
        documents.len(),
    );

    assert_eq!(
        unparsable, 0,
        "{unparsable} document(s) declare `sce:req` and do not parse, so \
         their annotations were skipped rather than checked. A skipped \
         check is an unrun one: this sweep would go green while saying \
         nothing about exactly the documents that carry the thing it \
         guards",
    );

    // ⚠ 26, and the number's basis matters more than the number. An
    // earlier floor here said 30, taken from a measurement of 32 —
    // but that 32 counted annotated ELEMENTS, and this counts distinct
    // IDS summed per file. Three documents carry annotations: 6, 1 and
    // 19 ids across 6, 1 and 25 elements. Reusing a figure measured
    // for a neighbouring quantity is how a floor ends up right-looking
    // and wrong, so this one names what it counts.
    assert!(
        declared_total >= 25,
        "only {declared_total} declared id(s) across the tree; 26 were \
         measured when this was written, so a number below that means the \
         scan stopped matching rather than that the tree got cleaner",
    );
    assert!(
        unreachable.is_empty(),
        "the walk does not reach {} annotation(s) that documents declare. \
         Each is an `sce:req` an author wrote, the parser stored and every \
         backend emits, which no requirement-closure reading can see — the \
         shape HOLE-3 was:\n  {}",
        unreachable.len(),
        unreachable.join("\n  "),
    );
}

/// Every `.scxml` under `dir`, skipping build and VCS directories.
fn collect_scxml(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if path.is_dir() {
            if name == "target" || name == ".git" || name == "node_modules" {
                continue;
            }
            collect_scxml(&path, out);
        } else if path.extension().is_some_and(|e| e == "scxml") {
            out.push(path);
        }
    }
}

/// The `sce:req` tokens a document's TEXT declares, read without the
/// parser and without the walk.
///
/// XML comments are stripped first: several fixtures discuss `sce:req`
/// in their leading comment, and counting prose as a declaration would
/// make this sweep fail for a reason that has nothing to do with the
/// walk.
fn declared_req_ids(text: &str) -> std::collections::BTreeSet<String> {
    let mut body = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find("<!--") {
        body.push_str(&rest[..start]);
        match rest[start..].find("-->") {
            Some(end) => rest = &rest[start + end + 3..],
            None => return scan_req_attrs(&body),
        }
    }
    body.push_str(rest);
    scan_req_attrs(&body)
}

fn scan_req_attrs(body: &str) -> std::collections::BTreeSet<String> {
    let mut out = std::collections::BTreeSet::new();
    for (idx, _) in body.match_indices("sce:req=") {
        let rest = &body[idx + "sce:req=".len()..];
        let quote = match rest.chars().next() {
            Some(q @ ('"' | '\'')) => q,
            _ => continue,
        };
        let Some(end) = rest[1..].find(quote) else {
            continue;
        };
        for token in rest[1..1 + end].split_whitespace() {
            out.insert(token.to_string());
        }
    }
    out
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

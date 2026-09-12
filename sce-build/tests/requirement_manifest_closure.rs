//! Requirement-closure RFC ① — Atomic A.
//!
//! `sce-codegen requirements --manifest <manifest.json> <doc.scxml>`
//! answers four things about a document it did not write the
//! denominator for: `implemented`, `unresolved`, `missing`,
//! `dangling`.
//!
//! # What this file is really guarding
//!
//! Not "does the classifier run" — that a passing test of a set
//! comparison proves almost nothing is exactly the trap this
//! repository has been caught by before. A sweep that examined zero
//! requirements prints the same green as one that examined fifty, and
//! a suite covering three of the four outcomes prints the same green
//! as one covering four.
//!
//! So the closing test here does three things a plain assertion would
//! not:
//!
//!   1. it requires **every one of the four outcomes to have a
//!      non-empty example**, by name, so a bucket that stopped being
//!      reachable is a failure rather than a silence;
//!   2. it **prints what it examined** — requirements and documents —
//!      so the number is in the log rather than in someone's belief;
//!   3. it **asserts a floor** under both, so a corpus that shrank to
//!      nothing cannot pass.
//!
//! ⚠ The fixtures are synthetic on purpose. A real standard is the
//! next milestone, and doing both at once would mean neither failure
//! could be told from the other.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::Command;

use sce_build::parser::SCXMLParser;
use sce_build::requirement_manifest::{classify, Outcome, RequirementManifest};

/// A document that cites four requirements, one of them not in the
/// manifest, with one marked unresolved — and a manifest that declares
/// one the document never mentions.
///
/// One fixture rather than four, because the four outcomes are a
/// PARTITION of one comparison: separate fixtures would let each be
/// right while the partition was wrong (an id counted as both
/// implemented and dangling, say). Deciding them together is what
/// makes them exclusive.
const DOC: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
                            xmlns:sce="http://sce.dev/ext"
                            version="1.0" initial="idle" datamodel="null">
  <state id="idle" sce:req="REQ-001"
         sce:provenance="car-body-spec@D3#3.1:12">
    <transition event="btn_press" target="emergency" sce:req="REQ-002"/>
    <transition event="diag_req" target="diagnostic"/>
  </state>
  <state id="emergency" sce:req="REQ-003">
    <sce:unresolved id="REQ-003" reason="spec unclear whether 3 s or 5 s"/>
    <onentry sce:req="REQ-999">
      <raise event="drive_off"/>
    </onentry>
  </state>
  <state id="diagnostic"/>
</scxml>"#;

/// Coordinates only — no requirement sentence. See
/// `requirement_manifest`'s module doc: a manifest is committed and
/// specification text is usually somebody else's copyright, so the
/// sentence lives in an uncommitted sidecar and this format has no
/// field that could hold it.
const MANIFEST: &str = r#"{
  "doc_id": "car-body-spec",
  "rev": "D3",
  "sections": [
    { "id": "3.1", "title": "Start procedure" },
    { "id": "3.3", "title": "Emergency mode" },
    { "id": "3.5", "title": "Diagnostic access" }
  ],
  "requirements": [
    { "id": "REQ-001", "section": "3.1", "page": 12 },
    { "id": "REQ-002", "section": "3.3", "page": 41 },
    { "id": "REQ-003", "section": "3.3", "page": 42 },
    { "id": "REQ-004", "section": "3.5", "page": 55 }
  ]
}"#;

fn parse(scxml: &str, label: &str) -> sce_build::model::SCXMLModel {
    SCXMLParser::new()
        .parse_string(scxml, label)
        .unwrap_or_else(|e| panic!("fixture {label} must parse: {:?}", e.error))
}

fn manifest(raw: &str, label: &str) -> RequirementManifest {
    RequirementManifest::from_json(raw, label)
        .unwrap_or_else(|e| panic!("fixture manifest {label} must load: {e}"))
}

#[test]
fn the_four_outcomes_each_have_a_non_empty_example() {
    let model = parse(DOC, "closure_doc");
    let declared = manifest(MANIFEST, "closure_manifest");
    let result = classify(&model, &declared);

    let of = |outcome: Outcome| -> Vec<&str> {
        result
            .outcomes
            .iter()
            .filter(|o| o.outcome == outcome)
            .map(|o| o.id.as_str())
            .collect()
    };

    assert_eq!(
        of(Outcome::Implemented),
        vec!["REQ-001", "REQ-002"],
        "cited on a node with no unresolved marker",
    );
    assert_eq!(
        of(Outcome::Unresolved),
        vec!["REQ-003"],
        "cited on a node carrying <sce:unresolved> — the AI said it did not know",
    );
    assert_eq!(
        of(Outcome::Missing),
        vec!["REQ-004"],
        "declared by the specification and on no node at all — the outcome \
         that does not exist without a manifest",
    );
    assert_eq!(
        of(Outcome::Dangling),
        vec!["REQ-999"],
        "cited by the document and absent from the manifest — invented, \
         or a stale revision",
    );

    // The buckets partition the ids rather than merely covering them.
    let mut seen = BTreeSet::new();
    for outcome in &result.outcomes {
        assert!(
            seen.insert(outcome.id.as_str()),
            "`{}` was classified twice; the four outcomes are meant to be \
             exclusive, so a reader can act on exactly one of them",
            outcome.id,
        );
    }
}

#[test]
fn an_implemented_requirement_names_the_nodes_that_carry_it() {
    let model = parse(DOC, "closure_doc");
    let declared = manifest(MANIFEST, "closure_manifest");
    let result = classify(&model, &declared);

    let req_002 = result
        .outcomes
        .iter()
        .find(|o| o.id == "REQ-002")
        .expect("REQ-002 is in the manifest");
    assert_eq!(
        req_002.node_paths,
        vec!["states.idle.transitions[0]"],
        "the classification names nodes by the same `node_path` the \
         requirements report prints, because it walks that report — a \
         path a reader cannot find there would be worse than none",
    );
    assert!(
        result
            .outcomes
            .iter()
            .find(|o| o.id == "REQ-004")
            .expect("REQ-004 is in the manifest")
            .node_paths
            .is_empty(),
        "a missing requirement is on no node by definition",
    );
}

#[test]
fn a_section_that_yielded_nothing_is_still_listed() {
    let model = parse(DOC, "closure_doc");
    let declared = manifest(MANIFEST, "closure_manifest");
    let result = classify(&model, &declared);

    // RFC §5.2a: the empty section is the whole point. A per-section
    // count that skipped the zeroes would answer "what was found" and
    // silently drop "what to go and look at", which is the only handle
    // on a requirement nobody extracted.
    let counts: Vec<(&str, usize)> = result
        .section_counts
        .iter()
        .map(|(id, n)| (id.as_str(), *n))
        .collect();
    assert_eq!(counts, vec![("3.1", 1), ("3.3", 2), ("3.5", 1)]);
}

#[test]
fn a_document_citing_another_revision_says_so() {
    let stale = r#"{
      "doc_id": "car-body-spec",
      "rev": "D4",
      "requirements": [{ "id": "REQ-001" }]
    }"#;
    let model = parse(DOC, "closure_doc");
    let declared = manifest(stale, "stale_manifest");
    let result = classify(&model, &declared);
    let note = result
        .revision_note
        .expect("the document anchors car-body-spec@D3 and the manifest is D4");
    assert!(note.contains("D3") && note.contains("D4"), "got: {note}");
}

/// The staleness check sees an anchor wherever the IR allows one.
///
/// Regression: the first version of `classify` walked states and
/// transitions by hand, so a document whose only `sce:provenance` sat
/// on an `<onentry>` action was compared against a stale manifest in
/// silence. A partial walk is worst exactly here — the note qualifies
/// every other verdict in the report, so losing it leaves the rest
/// confidently wrong rather than merely incomplete.
#[test]
fn the_staleness_check_sees_an_anchor_on_an_action() {
    let anchored_on_action = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
                                       xmlns:sce="http://sce.dev/ext"
                                       version="1.0" initial="s0" datamodel="null">
      <state id="s0">
        <onentry>
          <raise event="e" sce:req="REQ-1"
                 sce:provenance="car-body-spec@D3#3.1:12"/>
        </onentry>
      </state>
    </scxml>"#;
    let stale = r#"{ "doc_id": "car-body-spec", "rev": "D4",
                     "requirements": [{ "id": "REQ-1" }] }"#;
    let model = parse(anchored_on_action, "action_anchor");
    let declared = manifest(stale, "stale_manifest");
    let note = classify(&model, &declared)
        .revision_note
        .expect("the only anchor in this document is on an action, and it is stale");
    assert!(note.contains("D3") && note.contains("D4"), "got: {note}");
}

/// ⭐ A manifest cannot carry the requirement sentence.
///
/// This is the copyright split made structural rather than advisory.
/// A specification is usually somebody else's copyrighted document and
/// a manifest is a checked-in file; a format with a `text` field
/// invites one mistake, makes it silently, and makes it permanent in
/// the history. `deny_unknown_fields` turns that into a load failure
/// for everyone, which is the difference between a rule and a guard.
#[test]
fn a_manifest_carrying_requirement_text_is_refused() {
    let with_text = r#"{
      "doc_id": "car-body-spec",
      "rev": "D3",
      "requirements": [
        { "id": "REQ-001", "section": "3.1", "page": 12,
          "text": "Holding the start button for 3 seconds enters emergency mode." }
      ]
    }"#;
    let err = RequirementManifest::from_json(with_text, "with_text")
        .expect_err("a manifest carrying `text` must not load");
    let rendered = err.to_string();
    assert!(
        rendered.contains("COORDINATES only") && rendered.contains("sidecar"),
        "the refusal must say where the sentence belongs, or the author \
         will simply delete the field and lose it; got: {rendered}",
    );
}

#[test]
fn a_manifest_that_would_measure_nothing_is_refused() {
    let empty = r#"{ "doc_id": "d", "rev": "1", "requirements": [] }"#;
    assert!(
        RequirementManifest::from_json(empty, "empty")
            .expect_err("an empty manifest must not load")
            .to_string()
            .contains("vacuously clean"),
        "an empty denominator reports every document perfect",
    );

    let duplicated = r#"{
      "doc_id": "d", "rev": "1",
      "requirements": [{ "id": "REQ-1" }, { "id": "REQ-1" }]
    }"#;
    assert!(
        RequirementManifest::from_json(duplicated, "duplicated")
            .expect_err("a duplicated id must not load")
            .to_string()
            .contains("more than once"),
        "a denominator that counts one requirement twice is not a set",
    );
}

/// The four outcomes through the COMMAND, not the library.
///
/// The library tests above would all stay green if `--manifest` were
/// unwired, misrouted, or silently ignored — every one of them calls
/// `classify` directly. What the milestone asks for is a *command*
/// that answers four ways, so one test has to pay the cost of
/// spawning it.
#[test]
fn the_command_emits_all_four_outcomes() {
    let dir = tempfile::tempdir().expect("tempdir");
    let doc = dir.path().join("doc.scxml");
    let manifest_path = dir.path().join("manifest.json");
    std::fs::write(&doc, DOC).expect("write fixture document");
    std::fs::write(&manifest_path, MANIFEST).expect("write fixture manifest");

    let bin = PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"));
    let output = Command::new(&bin)
        .arg("requirements")
        .arg("--manifest")
        .arg(&manifest_path)
        .arg(&doc)
        .output()
        .expect("sce-codegen runs");
    assert!(
        output.status.success(),
        "sce-codegen requirements --manifest failed: {}",
        String::from_utf8_lossy(&output.stderr),
    );
    let stdout = String::from_utf8(output.stdout).expect("utf-8 stdout");

    for outcome in ["implemented", "unresolved", "missing", "dangling"] {
        assert!(
            stdout.contains(&format!("\"outcome\":\"{outcome}\"")),
            "the command emitted no `{outcome}` record. stdout:\n{stdout}",
        );
    }
    // The section that yielded nothing rides the same stream, because
    // the omission handle is useless if a consumer has to run a second
    // command for it.
    assert!(
        stdout.contains("\"kind\":\"section-coverage\""),
        "no section coverage on the stream. stdout:\n{stdout}",
    );

    // Without `--manifest` the subcommand must still answer its old
    // question, in its old shape. This is the report other tooling
    // already parses; a four-way classification appearing there
    // instead would break every existing consumer.
    let plain = Command::new(&bin)
        .arg("requirements")
        .arg(&doc)
        .output()
        .expect("sce-codegen runs");
    assert!(plain.status.success());
    let plain_stdout = String::from_utf8(plain.stdout).expect("utf-8 stdout");
    assert!(
        plain_stdout.contains("\"node_type\":\"state\"") && !plain_stdout.contains("\"outcome\""),
        "the manifest-less report changed shape. stdout:\n{plain_stdout}",
    );
}

/// A manifest that cannot be loaded stops the run.
///
/// Falling back to the annotation-only report would answer a different
/// question, print a clean-looking result, and never say the
/// denominator had been dropped — which is the failure mode this whole
/// RFC exists to remove.
#[test]
fn the_command_refuses_a_manifest_it_cannot_use() {
    let dir = tempfile::tempdir().expect("tempdir");
    let doc = dir.path().join("doc.scxml");
    let manifest_path = dir.path().join("manifest.json");
    std::fs::write(&doc, DOC).expect("write fixture document");
    std::fs::write(
        &manifest_path,
        r#"{ "doc_id": "d", "rev": "1",
             "requirements": [{ "id": "REQ-1", "text": "a sentence" }] }"#,
    )
    .expect("write fixture manifest");

    let output = Command::new(PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen")))
        .arg("requirements")
        .arg("--manifest")
        .arg(&manifest_path)
        .arg(&doc)
        .output()
        .expect("sce-codegen runs");
    assert!(
        !output.status.success(),
        "a manifest carrying requirement text must stop the run, not \
         degrade to the report that needs no manifest",
    );
    assert!(
        output.stdout.is_empty(),
        "a refused manifest must emit no classification at all",
    );
}

/// The closing count: what was examined, printed, with a floor.
///
/// ⚠ Without the floor this file passes having examined nothing. That
/// is not hypothetical here — an empty sweep and a clean one print the
/// same green, and this repository has been caught by exactly that
/// before. The floor is set under today's corpus so ordinary growth
/// does not move it, and far above zero so a corpus that stopped
/// producing requirements cannot pass quietly.
#[test]
fn the_sweep_reports_what_it_examined_and_asserts_a_floor() {
    // Every (document, manifest) pair this file classifies.
    let corpus: Vec<(&str, &str, &str)> = vec![
        ("closure_doc", DOC, MANIFEST),
        (
            "second_doc",
            r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
                      xmlns:sce="http://sce.dev/ext"
                      version="1.0" initial="s0" datamodel="null">
                 <state id="s0" sce:req="REQ-A">
                   <transition event="go" target="s1" sce:req="REQ-B"/>
                 </state>
                 <state id="s1" sce:req="REQ-ZZZ"/>
               </scxml>"#,
            r#"{ "doc_id": "other-spec", "rev": "A",
                 "sections": [{ "id": "1", "title": "Only section" }],
                 "requirements": [
                   { "id": "REQ-A", "section": "1" },
                   { "id": "REQ-B", "section": "1" },
                   { "id": "REQ-C", "section": "1" }
                 ] }"#,
        ),
    ];

    let mut documents = 0usize;
    let mut requirements = 0usize;
    let mut per_outcome = std::collections::BTreeMap::new();

    for (label, scxml, manifest_json) in &corpus {
        let model = parse(scxml, label);
        let declared = manifest(manifest_json, label);
        let result = classify(&model, &declared);
        documents += 1;
        requirements += result.outcomes.len();
        for outcome in [
            Outcome::Implemented,
            Outcome::Unresolved,
            Outcome::Missing,
            Outcome::Dangling,
        ] {
            *per_outcome.entry(outcome).or_insert(0usize) += result.count(outcome);
        }
    }

    // Printed, not merely asserted: a number in the log is checkable
    // by a reader who does not trust the floor.
    println!(
        "requirement closure: examined {requirements} requirement(s) across \
         {documents} document(s)"
    );
    for (outcome, count) in &per_outcome {
        println!("  {:<12} {count}", outcome.as_str());
    }

    assert!(
        documents >= 2,
        "examined {documents} document(s); a single-document sweep cannot \
         show that the comparison works on a document it was not shaped around",
    );
    assert!(
        requirements >= 8,
        "examined {requirements} requirement(s), which is too few to mean \
         anything — the corpus stopped producing them",
    );
    for outcome in [
        Outcome::Implemented,
        Outcome::Unresolved,
        Outcome::Missing,
        Outcome::Dangling,
    ] {
        assert!(
            per_outcome.get(&outcome).copied().unwrap_or(0) > 0,
            "`{}` has no non-empty example in the corpus. An outcome \
             nothing exercises is an outcome nobody would notice \
             breaking",
            outcome.as_str(),
        );
    }
}

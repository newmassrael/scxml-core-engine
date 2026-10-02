//! `sce-codegen requirements A B C --manifest m` measures the documents of one
//! design TOGETHER.
//!
//! A statechart that closes its interface is checked with the event schemas it
//! imports, so the design an owner asks about is a set. Measured one file at a
//! time, a schema that claims nothing reads every requirement `missing`, which
//! is true of the schema and says nothing about the design: the requirement
//! list could not be set against the check the client actually makes. The
//! claims of every document are pooled and classified once, by the same
//! function that answers for a single document.
//!
//! What these hold, each by a case that fails when it stops being true:
//!   * a requirement met by a node of ANY document is met, and one that no
//!     document claims is `missing`
//!   * a document that cannot claim (an event schema) takes nothing away
//!   * `states.idle` in two documents is two places, and each path says which
//!   * the rule "unresolved only when EVERY claim is" holds across documents
//!   * a document of another kind claims through its own annotation site (a
//!     lookup row), and is pooled like a statechart
//!   * one document answers exactly as it did before, paths unqualified
//!   * several documents with no requirement set are refused, because without
//!     a denominator the claims of several files in one stream would not say
//!     which file a node is in

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

const FRONT: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext" version="1.0" initial="idle" datamodel="null">
  <state id="idle" sce:req="R1">
    <transition event="go" target="busy" sce:req="R2"/>
  </state>
  <state id="busy"/>
</scxml>"#;

// Claims R3, cites an id the list lacks, and marks R4's node unresolved.
const BACK: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext" version="1.0" initial="idle" datamodel="null">
  <state id="idle" sce:req="R3">
    <transition event="go" target="held" sce:req="R99"/>
  </state>
  <state id="held" sce:req="R4">
    <sce:unresolved id="R4" reason="the specification does not say how long"/>
  </state>
</scxml>"#;

// R4 is also claimed here on a settled node, so across the two documents it has
// been answered somewhere and is not an open question.
const SETTLED: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext" version="1.0" initial="idle" datamodel="null">
  <state id="idle" sce:req="R4"/>
</scxml>"#;

// A document of another kind whose rows can claim: the fourth entry claims R5.
const GEAR_LOOKUP: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="lookup" name="gear_position_lookup">
  <datamodel>
    <data id="gearRaw" sce:type="uint8" sce:direction="in"/>
    <data id="gear" sce:type="string" sce:direction="out"/>
    <data id="mapping" sce:default="NEUTRAL">
      <sce:entry key="0" value="PARK"/>
      <sce:entry key="3" value="DRIVE" sce:req="R5"/>
    </data>
  </datamodel>
</scxml>"#;

const EVENT_SCHEMA: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="event-schema" name="RequestNeeded" sce:event-name="RequestNeeded">
  <sce:kind-basis>
    <sce:evidence>the specification names the signal</sce:evidence>
    <sce:rejected kind="codec">no wire layout is stated</sce:rejected>
  </sce:kind-basis>
  <datamodel sce:unresolved="payload" sce:unresolved-reason="the payload is not stated"/>
</scxml>"#;

const MANIFEST: &str = r#"{
  "doc_id": "retry-spec",
  "rev": "1",
  "extraction": {
    "ids": "native", "trace": "none",
    "modality_convention": "english-modal-verbs", "method": "hand"
  },
  "sections": [ { "id": "S1", "title": "Behaviour" } ],
  "requirements": [
    { "id": "R1", "section": "S1" },
    { "id": "R2", "section": "S1" },
    { "id": "R3", "section": "S1" },
    { "id": "R4", "section": "S1" },
    { "id": "R5", "section": "S1" }
  ]
}"#;

struct Run {
    success: bool,
    stdout: String,
    stderr: String,
}

impl Run {
    fn records(&self) -> Vec<Value> {
        self.stdout
            .lines()
            .map(|line| serde_json::from_str(line).expect("each line is a JSON record"))
            .collect()
    }

    fn requirement(&self, id: &str) -> Value {
        self.records()
            .into_iter()
            .find(|r| r["kind"] == "requirement" && r["id"] == id)
            .unwrap_or_else(|| panic!("no record for {id}. stdout:\n{}", self.stdout))
    }
}

fn requirements(dir: &Path, documents: &[&str], manifest: bool) -> Run {
    let mut command = Command::new(PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen")));
    command.current_dir(dir).arg("requirements");
    for document in documents {
        command.arg(document);
    }
    if manifest {
        command.arg("--manifest").arg("manifest.json");
    }
    let output = command.output().expect("sce-codegen runs");
    Run {
        success: output.status.success(),
        stdout: String::from_utf8(output.stdout).expect("utf-8 stdout"),
        stderr: String::from_utf8(output.stderr).expect("utf-8 stderr"),
    }
}

fn the_files() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    for (name, text) in [
        ("front.scxml", FRONT),
        ("back.scxml", BACK),
        ("settled.scxml", SETTLED),
        ("RequestNeeded.scxml", EVENT_SCHEMA),
        ("gear.scxml", GEAR_LOOKUP),
        ("manifest.json", MANIFEST),
    ] {
        std::fs::write(dir.path().join(name), text).expect("write fixture");
    }
    dir
}

fn paths(record: &Value) -> Vec<String> {
    record["node_paths"]
        .as_array()
        .expect("node_paths is a list")
        .iter()
        .map(|p| p.as_str().expect("a path is a string").to_string())
        .collect()
}

#[test]
fn a_requirement_any_document_claims_is_met_and_one_none_claims_is_missing() {
    let dir = the_files();
    let run = requirements(dir.path(), &["front.scxml", "back.scxml"], true);
    assert!(run.success, "failed: {}", run.stderr);
    // R1 and R2 are front's, R3 is back's: the design carries all three.
    for id in ["R1", "R2", "R3"] {
        assert_eq!(run.requirement(id)["outcome"], "implemented", "{id}");
    }
    // Nothing claims R5, in either document.
    assert_eq!(run.requirement("R5")["outcome"], "missing");
}

#[test]
fn a_document_that_cannot_claim_takes_nothing_away() {
    let dir = the_files();
    // Measured alone, the event schema reads every requirement missing.
    let alone = requirements(dir.path(), &["RequestNeeded.scxml"], true);
    assert_eq!(alone.requirement("R1")["outcome"], "missing");
    // Beside the statechart it is part of the design and R1 is still met.
    let together = requirements(dir.path(), &["front.scxml", "RequestNeeded.scxml"], true);
    assert!(together.success, "failed: {}", together.stderr);
    assert_eq!(together.requirement("R1")["outcome"], "implemented");
    assert_eq!(together.requirement("R3")["outcome"], "missing");
}

#[test]
fn the_same_path_in_two_documents_is_two_places_and_each_says_which() {
    let dir = the_files();
    // `states.idle` carries R1 in front and R3 in back, and R4 in settled.
    let run = requirements(
        dir.path(),
        &["front.scxml", "back.scxml", "settled.scxml"],
        true,
    );
    assert!(run.success, "failed: {}", run.stderr);
    assert_eq!(paths(&run.requirement("R1")), ["front.scxml#states.idle"]);
    assert_eq!(paths(&run.requirement("R3")), ["back.scxml#states.idle"]);
    let r4 = paths(&run.requirement("R4"));
    assert_eq!(r4.len(), 2, "R4 is claimed in two documents: {r4:?}");
    assert!(r4.contains(&"back.scxml#states.held".to_string()), "{r4:?}");
    assert!(
        r4.contains(&"settled.scxml#states.idle".to_string()),
        "{r4:?}"
    );
}

#[test]
fn an_id_the_list_lacks_is_dangling_in_the_document_that_cites_it() {
    let dir = the_files();
    let run = requirements(dir.path(), &["front.scxml", "back.scxml"], true);
    let dangling = run.requirement("R99");
    assert_eq!(dangling["outcome"], "dangling");
    assert_eq!(paths(&dangling), ["back.scxml#states.idle.transitions[0]"]);
}

#[test]
fn a_requirement_answered_in_one_document_is_not_open_because_another_leaves_it_open() {
    let dir = the_files();
    // Alone, back claims R4 only on a node marked unresolved.
    let alone = requirements(dir.path(), &["back.scxml", "RequestNeeded.scxml"], true);
    assert_eq!(alone.requirement("R4")["outcome"], "unresolved");
    // With a settled claim in another document it has been answered somewhere.
    let together = requirements(dir.path(), &["back.scxml", "settled.scxml"], true);
    assert_eq!(together.requirement("R4")["outcome"], "implemented");
}

#[test]
fn a_document_of_another_kind_claims_through_its_own_annotation_site() {
    let dir = the_files();
    // R5 is claimed by no statechart; the lookup row claims it.
    let alone = requirements(dir.path(), &["front.scxml"], true);
    assert_eq!(alone.requirement("R5")["outcome"], "missing");
    let together = requirements(dir.path(), &["front.scxml", "gear.scxml"], true);
    assert!(together.success, "failed: {}", together.stderr);
    let r5 = together.requirement("R5");
    assert_eq!(r5["outcome"], "implemented");
    assert_eq!(paths(&r5), ["gear.scxml#entries[key=3]"]);
    // And the statechart's own claims are still there beside it.
    assert_eq!(together.requirement("R1")["outcome"], "implemented");
}

#[test]
fn one_document_answers_as_it_did_before_with_paths_unqualified() {
    let dir = the_files();
    let run = requirements(dir.path(), &["front.scxml"], true);
    assert!(run.success, "failed: {}", run.stderr);
    assert_eq!(paths(&run.requirement("R1")), ["states.idle"]);
    assert_eq!(
        paths(&run.requirement("R2")),
        ["states.idle.transitions[0]"]
    );
}

/// `classify_documents` over ONE member answers as `classify` does, with paths
/// unqualified: its own documentation says "byte for byte". The command never
/// reaches it with one document (a single file goes to the one-document path), so
/// only a caller of the library can, and this is that caller. Until it was
/// written, the clause that decides it was defended by nothing: a mutation that
/// qualified the path for one document survived the whole suite (Mutation
/// Rounds, 2026-10-01, `a node path is qualified even for one document`).
#[test]
fn a_set_of_one_document_is_answered_as_the_document_alone_with_paths_unqualified() {
    use sce_build::requirement_manifest::{
        citations_of_model, classify, classify_documents, Classification, RequirementManifest,
    };

    let dir = the_files();
    let manifest = RequirementManifest::load(&dir.path().join("manifest.json"))
        .expect("the requirement set loads");
    let front = dir.path().join("front.scxml");
    let model = sce_build::parser::SCXMLParser::new()
        .parse_file(front.to_str().expect("a utf-8 path"))
        .expect("front.scxml parses");

    let alone = classify(&model, &manifest);
    let set = classify_documents(
        &[("front.scxml".to_string(), citations_of_model(&model))],
        &manifest,
    );
    let places = |classification: &Classification| -> Vec<(String, Vec<String>)> {
        classification
            .outcomes
            .iter()
            .map(|outcome| (outcome.id.clone(), outcome.node_paths.clone()))
            .collect()
    };
    assert_eq!(places(&set), places(&alone));
    assert!(
        places(&set)
            .iter()
            .any(|(_, node_paths)| !node_paths.is_empty()),
        "the fixture must cite something, or the comparison above compares two silences"
    );
    assert!(
        places(&set)
            .iter()
            .all(|(_, node_paths)| node_paths.iter().all(|path| !path.contains('#'))),
        "a document alone has no other document to be told apart from: {:?}",
        places(&set)
    );
}

#[test]
fn several_documents_with_no_requirement_set_are_refused() {
    let dir = the_files();
    let run = requirements(dir.path(), &["front.scxml", "back.scxml"], false);
    assert!(
        !run.success,
        "several documents without a manifest must stop"
    );
    assert!(
        run.stdout.is_empty(),
        "no claims may be emitted: {}",
        run.stdout
    );
    assert!(
        run.stderr.contains("--manifest"),
        "the refusal must say what to give: {}",
        run.stderr
    );
}

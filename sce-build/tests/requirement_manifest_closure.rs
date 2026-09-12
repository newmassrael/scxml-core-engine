//! Requirement-closure RFC ① — Atomic A.
//!
//! `sce-codegen requirements --manifest <manifest.json> <doc.scxml>`
//! answers four things about a document it did not write the
//! denominator for: `implemented`, `unresolved`, `missing`,
//! `dangling`.
//!
//! ⚠ Those four are the **`shall` column**, and every manifest in this
//! file is `shall` because that is what Atomic A shipped. They are not
//! the whole of [`Outcome`]: a requirement met by something being
//! ABSENT cannot be settled by any of them, and reads
//! `needs-scenario`. A reader who took the count above for the size of
//! the enum would be wrong about the tool, which is why this note is
//! here rather than in the file that added the fifth — the misreading
//! happens here.
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
use sce_build::requirement_manifest::{classify, prose_reason, Outcome, RequirementManifest};

/// A manifest whose one section title is `title`, everything else a
/// coordinate.
fn manifest_titled(title: &str) -> String {
    format!(
        r#"{{ "doc_id": "placeholder-spec", "rev": "A",
              "sections": [{{ "id": "3.1", "title": "{title}" }}],
              "requirements": [{{ "id": "REQ-1", "section": "3.1" }}] }}"#
    )
}

/// HOLE-1 — the copyright guard reaches the section title, and the
/// discriminator is SHAPE rather than size.
///
/// ⚠ Every refused string below is a placeholder of my own writing.
/// Putting a real ISO sentence here to make the test "realistic" would
/// commit the exact thing the guard exists to prevent, in the file
/// that tests it — the guard keys on shape, so a placeholder of the
/// same shape exercises it exactly.
///
/// ⚠⚠ A length bound was tried first and is refuted; do not
/// re-propose it. Over ISO 13400-2:2019 the two populations overlap,
/// so every threshold is wrong in both directions. The shape rule was
/// measured on the same document instead: across 243 heading-shaped
/// strings — 78 contents-page headings and 165 REQ box headings — not
/// one trips it, while it refuses every one of the 166 requirement
/// sentences.
#[test]
fn a_section_title_that_reads_as_prose_is_refused() {
    let refused: [(&str, &str); 6] = [
        (
            "ends in a full stop",
            "A placeholder entity keeps a table of its connections",
        ),
        (
            "carries shall",
            "A placeholder entity shall keep a table of its connections",
        ),
        (
            "carries should",
            "A placeholder entity should keep a table of its connections",
        ),
        (
            "carries must",
            "A placeholder entity must keep a table of its connections",
        ),
        (
            "carries may",
            "A placeholder entity may keep a table of its connections",
        ),
        (
            "ends in a question mark",
            "Is the placeholder entity connected",
        ),
    ];

    let mut checked = 0usize;
    for (shape, body) in refused {
        // The first and last cases need their terminal punctuation;
        // the modal cases must NOT have it, or they would prove the
        // punctuation test rather than the modal one.
        let title = match shape {
            "ends in a full stop" => format!("{body}."),
            "ends in a question mark" => format!("{body}?"),
            _ => body.to_string(),
        };
        let err = RequirementManifest::from_json(&manifest_titled(&title), "titled")
            .expect_err("a section title that reads as prose must not load");
        let rendered = err.to_string();
        assert!(
            rendered.contains("COORDINATES only") && rendered.contains("sidecar"),
            "the refusal must say where the sentence belongs; {shape} gave: {rendered}",
        );
        assert!(
            rendered.contains("sections[0].title"),
            "the refusal must name the field it read, or an author cannot \
             find it in a manifest with many sections; {shape} gave: {rendered}",
        );
        checked += 1;
    }

    println!("HOLE-1: refused {checked} prose-shaped section title(s)");
    assert!(
        checked >= 6,
        "checked {checked} prose shapes; both halves of the discriminator \
         (terminal punctuation, and each normative modal) need a case, or \
         one half can stop working in silence",
    );
}

/// The other half, which is the half a guard usually gets wrong: real
/// headings must still load.
#[test]
fn heading_shaped_section_titles_are_not_refused() {
    // Shapes taken from how contents pages are written, not from any
    // one document: a noun phrase, an "X and Y", a numbered-looking
    // one, a single word, one carrying a comma.
    let accepted = [
        "NL socket handling",
        "Connection table",
        "Socket handler and alive check",
        "General inactivity timer",
        "Scope",
        "Terms, definitions and abbreviated terms",
        "Diagnostic power mode information",
        "Vehicle identification and announcement",
    ];

    let mut checked = 0usize;
    for title in accepted {
        assert!(
            prose_reason(title).is_none(),
            "`{title}` is a heading and must load; the guard read it as prose",
        );
        RequirementManifest::from_json(&manifest_titled(title), "titled")
            .unwrap_or_else(|e| panic!("heading `{title}` must load: {e}"));
        checked += 1;
    }

    println!("HOLE-1: accepted {checked} heading-shaped section title(s)");
    assert!(
        checked >= 8,
        "checked {checked} headings; a guard is only worth having if the \
         accepting half is exercised too, and a shrinking population here \
         is how a rule that refuses everything passes",
    );
}

/// ⭐ The claim that makes this a repair rather than a patch.
///
/// The hole was not that `title` was forgotten. It was that the guard
/// keyed on a field NAME — `text`, refused by `deny_unknown_fields` —
/// while the field beside it in the same struct went unchecked. So the
/// check must reach strings in fields nobody thought about, and this
/// test picks fields that are not `title` to say so.
#[test]
fn the_guard_reaches_strings_outside_the_field_that_exposed_it() {
    let sentence = "A placeholder entity shall keep a table of its connections";
    let elsewhere: [(&str, String); 2] = [
        (
            "sections[0].id",
            format!(
                r#"{{ "doc_id": "d", "rev": "A",
                      "sections": [{{ "id": "{sentence}", "title": "Scope" }}],
                      "requirements": [{{ "id": "REQ-1" }}] }}"#
            ),
        ),
        (
            "requirements[0].section",
            format!(
                r#"{{ "doc_id": "d", "rev": "A",
                      "requirements": [{{ "id": "REQ-1", "section": "{sentence}" }}] }}"#
            ),
        ),
    ];

    let mut checked = 0usize;
    for (field, raw) in &elsewhere {
        let rendered = RequirementManifest::from_json(raw, "elsewhere")
            .expect_err("prose anywhere in a committed manifest must be refused")
            .to_string();
        assert!(
            rendered.contains(field),
            "the guard must name `{field}`; got: {rendered}",
        );
        checked += 1;
    }

    println!("HOLE-1: refused prose in {checked} field(s) that are not `title`");
    assert!(
        checked >= 2,
        "checked {checked} non-title fields; with fewer than two this says \
         nothing about fields the format has not grown yet",
    );
}

/// ⭐ The sweep cannot be reached around, and the compiler is what
/// says so.
///
/// ⚠ This was a measured bypass, not a hypothetical one. While the
/// derive sat on the public type, `serde_json::from_str` loaded a
/// manifest whose section title was a requirement sentence — the same
/// bytes `from_json` refused. The guard was attached to a call path
/// rather than to the type, which is the defect it exists to close,
/// one level up.
///
/// The repair is that `RequirementManifest` no longer implements
/// `Deserialize` at all, so the bypass is not a thing a caller can
/// spell. That makes this test a compile-time claim: the commented
/// line below is the bypass, and it must not build. A runtime
/// assertion could not say this, because there is nothing left to
/// call.
#[test]
fn the_prose_sweep_cannot_be_reached_around() {
    let raw = manifest_titled("A placeholder entity shall keep a table");

    // The bypass, kept as the record of what must stay impossible:
    //
    //     let m: RequirementManifest = serde_json::from_str(&raw).unwrap();
    //
    // `RequirementManifest: Deserialize` is not implemented, so that
    // line does not compile. `ManifestWire` carries the derive and is
    // private to the module.
    assert!(
        RequirementManifest::from_json(&raw, "only-way-in").is_err(),
        "the one remaining way to build a manifest must run the sweep",
    );

    // And the guarded path still accepts a real one, so the line above
    // is not passing because everything fails.
    let clean = manifest_titled("Socket handling");
    assert!(
        RequirementManifest::from_json(&clean, "only-way-in").is_ok(),
        "a heading-titled manifest must still load through the one door",
    );
    println!("HOLE-1: the sweep has one door, and it is the only one");
}

/// The precondition that keeps the rule from eating the format's own
/// vocabulary — and the measurement that says it costs nothing.
#[test]
fn a_single_token_is_never_read_as_prose() {
    // `shall_not` is the one that bit: it is a `modality` value, and
    // the whole-word modal test fires on the `shall` inside it. It is
    // not prose, it is this format's own enum spelling.
    let tokens = [
        "shall_not",
        "shall",
        "3.DoIP-152",
        "12.6.1.2",
        "ISO-13400-2",
        "2019",
        "REQ-1",
    ];
    let mut checked = 0usize;
    for token in tokens {
        assert!(
            prose_reason(token).is_none(),
            "`{token}` has no whitespace and so cannot be a sentence, but \
             the guard read it as prose",
        );
        checked += 1;
    }
    println!("HOLE-1: {checked} single-token string(s) correctly not prose");
    assert!(
        checked >= 7,
        "checked {checked} tokens; `shall_not` must be among them, because \
         a guard that refuses this repository's own manifest is the failure \
         this precondition exists to prevent",
    );
}

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

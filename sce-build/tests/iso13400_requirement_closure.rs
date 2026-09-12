//! Requirement-closure RFC Atomic D — the first real consumer.
//!
//! Atomics A and B were measured only against fixtures their own
//! author wrote, which is RFC §5.5's defect (the numerator writing the
//! denominator) one level up: a tool that scores documents cannot be
//! scored by documents shaped to it. This file ends that by running
//! the whole of A and B over **ISO 13400-2:2019 §12.6**, a standard
//! nobody here controls — a sealed 30-entry manifest of its
//! requirement coordinates, and the DoIP connection-state statechart
//! an authoring pass produced from §12.6.1.
//!
//! # ⚠ The manifest is coordinates. The standard's text is not here
//!
//! ISO 13400-2 may be read and may not be redistributed. The
//! committed manifest holds `id`, `section` and `page`; the committed
//! statechart holds requirement identifiers and section/page numbers.
//! Neither holds a sentence of it, and neither does this file — where
//! a test below needs a manifest carrying requirement text, it builds
//! one at run time out of a placeholder, because what the guard keys
//! on is the FIELD and not what anyone puts in it.
//!
//! # ⭐ How to re-derive the coordinates, and why that is not a test
//!
//! A coordinate-only manifest is **not self-checking** (RFC §5.2b): a
//! reader cannot tell from `{"id": "3.DoIP-152", "page": 68}` alone
//! that the standard really puts it there, and nothing in this file
//! can, because the standard is not in this repository and must not
//! be. So the check lives outside, is run against a local copy, and is
//! recorded here as a procedure rather than as an assertion:
//!
//! ```text
//!   1. take the subclause region, 12.6 .. the next numbered clause
//!   2. strip the PDF's control bytes  (the page footers carry \x08,
//!      and a naive regex silently matches NOTHING because of them)
//!   3. every `REQ <id>` in order, and every page footer in order
//!   4. a REQ belongs to the page whose footer comes after it
//!   5. compare ids, ORDER and pages against this manifest
//! ```
//!
//! Run 2026-09-12 against ISO 13400-2:2019: 30 REQ boxes derived, 30
//! committed, id set identical, id order identical, **zero page
//! mismatches**. Worth repeating whenever the manifest's `rev` moves,
//! which RFC §5.6 says it will — the seal is per revision, not
//! forever.
//!
//! ⚠ Step 2 is in the list because it cost a wrong answer first time:
//! the footers are `"           68\x08"`, and a `^\s*(\d{2})\s*$`
//! match finds none of them while reporting no error at all. An
//! extraction that quietly matches zero page numbers assigns every
//! requirement a null page and then agrees with nothing, which reads
//! like a manifest defect rather than a scanner defect.
//!
//! # What a real standard changed
//!
//! Two things, and both were found by running the tools rather than by
//! reasoning about them:
//!
//! 1. **A prohibition scored green over a document that violates it.**
//!    `3.DoIP-131` forbids routing a diagnostic message before the
//!    connection is "Registered [Routing Active]". The statechart
//!    obeys it by handling that message in one state only, so the
//!    annotation went on that handler and coverage read
//!    `implemented`. Adding a second handler to `initialized` — a
//!    flat violation — left the annotated node untouched and the
//!    verdict unchanged. `Modality` and `Outcome::NeedsScenario`
//!    exist because of that measurement, and
//!    [`a_prohibition_is_not_certified_by_an_annotation_that_survives_its_violation`]
//!    is the test that would have caught it.
//!
//! 2. **A mis-spelled citation is a real failure mode, not a
//!    hypothetical one.** The standard's prose on p.70 writes
//!    `DoIP-134` where its own REQ box writes `3.DoIP-134`, and the
//!    statechart copied the prose. `dangling` reports it.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use sce_build::parser::SCXMLParser;
use sce_build::requirement_manifest::{
    classify, Classification, Modality, Outcome, RequirementManifest,
};
use sce_build::transition_table::{requirements_without_a_row, transition_table, NO_SOURCE};

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/requirement_closure")
}

fn manifest_path() -> PathBuf {
    fixture_dir().join("iso13400_2_nl_socket_handling.manifest.json")
}

fn document_path() -> PathBuf {
    fixture_dir().join("doip_nl_connection_states.scxml")
}

/// Every conversion the fixture directory holds, **discovered rather
/// than listed**.
///
/// ⚠ This is a function because the list it replaced was a literal of
/// one element, and that made two things wrong at once. The count the
/// sweep prints is supposed to be a measurement of the corpus; over a
/// literal it is a restatement of the literal, and a floor asserted
/// against it cannot fail however broken everything else gets. Worse,
/// a second standard dropped into this directory would have been
/// **silently unexamined** — the corpus grows, the number does not,
/// and the run stays green. That is this repository's most-repeated
/// defect shape, and the fixture sweep that exists to guard against it
/// is the last place it should have been reproduced.
fn documents_in_fixture_dir() -> Vec<PathBuf> {
    let dir = fixture_dir();
    let mut paths: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("fixture dir must be readable at {}: {e}", dir.display()))
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().is_some_and(|ext| ext == "scxml"))
        .collect();
    // `read_dir` order is not stable across filesystems; sort so a
    // failure list reads the same on every machine.
    paths.sort();
    paths
}

fn load_manifest() -> RequirementManifest {
    RequirementManifest::load(&manifest_path())
        .unwrap_or_else(|e| panic!("the committed ISO manifest must load: {e}"))
}

fn parse_file(path: &Path) -> sce_build::model::SCXMLModel {
    let raw = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("fixture must be readable at {}: {e}", path.display()));
    parse_str(&raw, &path.display().to_string())
}

fn parse_str(scxml: &str, label: &str) -> sce_build::model::SCXMLModel {
    SCXMLParser::new()
        .parse_string(scxml, label)
        .unwrap_or_else(|e| panic!("document {label} must parse: {:?}", e.error))
}

fn ids_with(classification: &Classification, want: Outcome) -> Vec<String> {
    let mut ids: Vec<String> = classification
        .outcomes
        .iter()
        .filter(|o| o.outcome == want)
        .map(|o| o.id.clone())
        .collect();
    ids.sort();
    ids
}

/// The four outcomes of RFC §5.3, each with a non-empty example, on a
/// standard this repository did not write.
///
/// ⚠ The floors are the point. Every count below would read green at
/// zero — an empty manifest classifies nothing and reports no
/// failures — and this repository has been caught by that shape often
/// enough that a bare `assert!(errors.is_empty())` over an unstated
/// population is not evidence of anything.
#[test]
fn the_four_outcomes_are_each_non_empty_against_a_real_standard() {
    let manifest = load_manifest();
    let model = parse_file(&document_path());
    let classification = classify(&model, &manifest);

    let implemented = classification.count(Outcome::Implemented);
    let unresolved = classification.count(Outcome::Unresolved);
    let missing = classification.count(Outcome::Missing);
    let dangling = classification.count(Outcome::Dangling);
    let needs_scenario = classification.count(Outcome::NeedsScenario);

    println!(
        "ISO 13400-2:2019 §12.6: {} requirement(s) declared; \
         {implemented} implemented, {unresolved} unresolved, \
         {missing} missing, {dangling} dangling, \
         {needs_scenario} needs-scenario",
        manifest.requirements.len(),
    );

    assert!(
        manifest.requirements.len() >= 30,
        "the manifest declares {} requirement(s); §12.6 carries thirty \
         REQ boxes, and a denominator that has quietly shrunk makes \
         every ratio taken against it wrong",
        manifest.requirements.len(),
    );
    assert!(
        implemented >= 10,
        "{implemented} implemented — the document stopped claiming what \
         it implements, or the walk stopped reaching it",
    );
    assert!(
        unresolved >= 3,
        "{unresolved} unresolved; the two inactivity timers are armed \
         without the standard's tabulated bounds and say so, so this \
         cannot honestly be empty",
    );
    assert!(
        missing >= 10,
        "{missing} missing; §12.6.4's socket handler is a different \
         machine and is not modelled here, so `missing` has a real \
         non-empty example and losing it means the comparison stopped \
         running",
    );
    assert!(
        dangling >= 1,
        "no dangling id; the document cites the standard's prose \
         spelling `DoIP-134` where the manifest carries the REQ box's \
         `3.DoIP-134`, and that must be reported",
    );

    // The honest-`missing` set, named rather than counted. A count
    // alone would survive the socket-handler entries being swapped for
    // any other ten.
    let missing_ids = ids_with(&classification, Outcome::Missing);
    for socket_handler in ["3.DoIP-088", "3.DoIP-092", "3.DoIP-096"] {
        assert!(
            missing_ids.contains(&socket_handler.to_string()),
            "§12.6.4's {socket_handler} is not modelled by this document \
             and must read `missing`; got {missing_ids:?}",
        );
    }
    assert_eq!(
        ids_with(&classification, Outcome::Dangling),
        vec!["DoIP-134".to_string()],
        "the prose spelling is the only id the document invents",
    );
}

/// RFC §5.2a's only handle on what was left out, over a real contents
/// page.
#[test]
fn the_section_counts_cover_the_whole_subclause() {
    let manifest = load_manifest();
    let model = parse_file(&document_path());
    let classification = classify(&model, &manifest);

    let counts = &classification.section_counts;
    println!("section coverage: {counts:?}");
    assert!(
        counts.len() >= 7,
        "§12.6 has seven subclauses on the contents page; got {}",
        counts.len(),
    );

    let empty: Vec<&String> = counts
        .iter()
        .filter(|(_, n)| *n == 0)
        .map(|(id, _)| id)
        .collect();
    assert_eq!(
        empty,
        vec!["12.6.1.1"],
        "§12.6.1.1 is a scope paragraph and yields no requirement, which \
         is what a zero here looks like when it is CORRECT — the RFC \
         calls this column a pointer and not a proof, and this is the \
         case that shows why",
    );
}

/// RFC §6.2's two readings of the table, each with a non-empty
/// example, with the population printed and floored.
#[test]
fn the_unclaimed_block_and_the_rowless_requirements_are_each_non_empty() {
    let manifest = load_manifest();
    let documents = documents_in_fixture_dir();
    assert!(
        !documents.is_empty(),
        "no .scxml under {} — the scan resolved nothing, and every count \
         below would then be a floor over an empty population, which is \
         exactly the green this test exists to refuse",
        fixture_dir().display(),
    );

    let mut documents_examined = 0usize;
    let mut rows_examined = 0usize;
    let mut unclaimed_rows = Vec::new();
    // ⚠ Per document, not summed. Rows and unclaimed rows are
    // POPULATION facts and adding them across documents is meaningful;
    // "requirements with no row" is a COVERAGE fact and adding those is
    // not. RFC §5.2d says coverage is per variant and a single number
    // over several documents is meaningless — a requirement one
    // conversion implements and another does not would be counted as
    // uncovered while being covered where it applies. Summed over two
    // documents this printed 39 out of a 30-entry manifest, which is
    // the shape of that mistake announcing itself.
    let mut without_a_row: Vec<(String, Vec<String>)> = Vec::new();

    for path in &documents {
        let raw = std::fs::read_to_string(path).expect("discovered document is readable");
        // The sweep classifies whatever it discovers against THIS
        // manifest, so a document for another standard would be scored
        // against a denominator that never mentions it and would read
        // as a total failure to implement anything. A guard rather than
        // a note in the directory, because a note is something someone
        // has to remember: a conversion of another standard belongs in
        // its own directory beside its own manifest.
        assert!(
            raw.contains(&manifest.doc_id),
            "{} anchors at no `{}` provenance, so it is not a conversion \
             of the standard this manifest declares and must not be \
             swept against it",
            path.display(),
            manifest.doc_id,
        );
        let model = parse_str(&raw, &path.display().to_string());
        let rows = transition_table(&model);
        documents_examined += 1;
        rows_examined += rows.len();
        unclaimed_rows.extend(
            rows.iter()
                .filter(|row| row.is_unclaimed())
                .map(|row| (row.from.clone(), row.event.clone())),
        );
        // The `shall` entries only. `requirements_without_a_row` answers
        // "is there a node carrying this id", which is the one question
        // a `shall_not` entry may not be asked — see its doc comment.
        without_a_row.push((
            path.file_name()
                .expect("a discovered file has a name")
                .to_string_lossy()
                .into_owned(),
            requirements_without_a_row(
                &rows,
                manifest
                    .requirements
                    .iter()
                    .filter(|entry| entry.modality == Modality::Shall)
                    .map(|entry| entry.id.as_str()),
            ),
        ));
    }

    println!(
        "transition table over ISO 13400-2:2019 §12.6: examined \
         {rows_examined} row(s) across {documents_examined} document(s); \
         {} unclaimed",
        unclaimed_rows.len(),
    );
    for (name, ids) in &without_a_row {
        println!(
            "  {name}: {} declared `shall` requirement(s) with no row",
            ids.len(),
        );
    }

    assert_eq!(
        documents_examined,
        documents.len(),
        "the sweep examined fewer documents than the directory holds",
    );
    assert!(
        rows_examined >= 25,
        "examined {rows_examined} row(s); the connection machine carries \
         more than that, so the walk has stopped reaching most of it",
    );
    assert!(
        !unclaimed_rows.is_empty(),
        "no `{NO_SOURCE}` row: reading (a) — behaviour the specification \
         never asked for — has no non-empty example on a real standard",
    );
    assert!(
        without_a_row.iter().any(|(_, ids)| !ids.is_empty()),
        "no declared requirement lacked a row in any document examined: \
         reading (b) has no non-empty example on a real standard",
    );

    // Named, because a count alone would survive the two honest rows
    // being replaced by two accidental ones. These two are behaviour
    // the authoring pass added on its own initiative: §12.6 has no REQ
    // box for either.
    //
    // ⚠ By SUBJECT — the owning state and the event — and deliberately
    // not by `node_path`. The first spelling of this assertion named
    // `states.registered.transitions[4]`, which is a position: adding
    // any earlier transition to `registered` renumbers it, and the
    // test would then fail for a reason that has nothing to do with
    // what it is checking. An anchor that quotes a position dies on
    // every reordering, and the failure it produces sends the reader
    // to the wrong place.
    let unclaimed: BTreeSet<(&str, &str)> = unclaimed_rows
        .iter()
        .map(|(from, event)| (from.as_str(), event.as_str()))
        .collect();
    for (from, event) in [("registered", "close_requested"), ("registered", "(exit)")] {
        assert!(
            unclaimed.contains(&(from, event)),
            "`{event}` on `{from}` is behaviour no §12.6 requirement \
             asks for and must sit in the `{NO_SOURCE}` block; got \
             {unclaimed:?}",
        );
    }
}

/// ① and ② are two readings of one export — over the `shall` column.
#[test]
fn table_derived_missing_agrees_with_the_classifier() {
    let manifest = load_manifest();
    let model = parse_file(&document_path());

    let from_classifier: BTreeSet<String> =
        ids_with(&classify(&model, &manifest), Outcome::Missing)
            .into_iter()
            .collect();
    let from_table: BTreeSet<String> = requirements_without_a_row(
        &transition_table(&model),
        manifest
            .requirements
            .iter()
            .filter(|entry| entry.modality == Modality::Shall)
            .map(|entry| entry.id.as_str()),
    )
    .into_iter()
    .collect();

    assert!(
        !from_classifier.is_empty(),
        "both sides are empty, so agreeing proves nothing",
    );
    assert_eq!(
        from_classifier, from_table,
        "the classification and the table disagree about which \
         requirements are missing. RFC §6.2 claims these are two \
         readings of ONE export; when they diverge that sentence is \
         false and one of the two answers is wrong without anyone \
         being told which",
    );
}

/// ⭐ The measurement that produced [`Modality`].
///
/// `3.DoIP-131` forbids routing a diagnostic message before the
/// connection reaches "Registered [Routing Active]". This test builds
/// a document that does exactly the forbidden thing — a second
/// `diagnostic_message` handler in `initialized` — and shows that
/// treating the entry as a `shall` certifies it anyway, because the
/// annotated node is still there and presence is all an annotation can
/// report.
///
/// ⚠ It asserts BOTH halves on purpose. Only asserting the fix would
/// leave nothing in the tree that fails if the routing is removed and
/// every entry silently goes back to being a `shall`.
#[test]
fn a_prohibition_is_not_certified_by_an_annotation_that_survives_its_violation() {
    let clean = std::fs::read_to_string(document_path()).expect("fixture readable");
    let anchor = r#"<state id="initialized" sce:req="3.DoIP-127">"#;
    assert!(
        clean.contains(anchor),
        "the injection point moved; without it this test builds a \
         document that violates nothing and passes for the wrong reason",
    );
    let violating = clean.replace(
        anchor,
        concat!(
            r#"<state id="initialized" sce:req="3.DoIP-127">"#,
            "\n    <transition event=\"diagnostic_message\">",
            "<send event=\"route_to_target\"/></transition>",
        ),
    );
    assert_ne!(clean, violating, "the injection produced no change");

    let model = parse_str(&violating, "doip_violating");
    let mut manifest = load_manifest();

    let entry = manifest
        .requirements
        .iter()
        .find(|e| e.id == "3.DoIP-131")
        .expect("the manifest declares 3.DoIP-131");
    assert_eq!(
        entry.modality,
        Modality::ShallNot,
        "3.DoIP-131 is a `shall not` in the standard; a manifest that \
         types it otherwise is what this whole test exists to stop",
    );

    // As the standard types it: no annotation can settle it.
    let verdict = |m: &RequirementManifest| {
        classify(&model, m)
            .outcomes
            .iter()
            .find(|o| o.id == "3.DoIP-131")
            .expect("3.DoIP-131 is classified")
            .clone()
    };
    let typed = verdict(&manifest);
    assert_eq!(
        typed.outcome,
        Outcome::NeedsScenario,
        "a document that routes diagnostic messages before \
         `routing_active` violates 3.DoIP-131, and no reading of its \
         annotations may say otherwise",
    );
    assert!(
        !typed.node_paths.is_empty(),
        "the verdict must still name where the author claimed the \
         prohibition is enforced; an honest answer that says nothing \
         actionable is a worse report than the false one it replaced",
    );

    // As Atomic A typed everything before this: green over a violation.
    for entry in &mut manifest.requirements {
        if entry.id == "3.DoIP-131" {
            entry.modality = Modality::Shall;
        }
    }
    assert_eq!(
        verdict(&manifest).outcome,
        Outcome::Implemented,
        "this is the defect being recorded, not a property worth \
         keeping: asked the `shall` question, the tool certifies a \
         prohibition from an annotation that the violation left in \
         place. If this assertion ever fails, the presence-question \
         has changed and this file's reason for existing has moved",
    );
}

/// The copyright gate of Atomic A, fired on the real standard.
///
/// ⚠ The injected value is a placeholder, and that is not a dodge —
/// `deny_unknown_fields` keys on the FIELD. Putting a sentence of
/// ISO 13400-2 here to make the test "realistic" would commit the
/// exact thing the guard exists to prevent, in the file that tests it.
#[test]
fn the_copyright_gate_refuses_the_real_iso_manifest_once_it_carries_text() {
    let raw = std::fs::read_to_string(manifest_path()).expect("manifest readable");
    RequirementManifest::from_json(&raw, "iso-committed")
        .expect("the committed manifest carries coordinates only and must load");

    let anchor = r#"{ "id": "3.DoIP-152", "section": "12.6.1.2", "page": 68 }"#;
    assert!(
        raw.contains(anchor),
        "the entry this test mutates has moved; without it the mutation \
         is a no-op and the refusal below would be proving nothing",
    );
    let with_text = raw.replace(
        anchor,
        r#"{ "id": "3.DoIP-152", "section": "12.6.1.2", "page": 68,
             "text": "<the requirement sentence, which must never be committed>" }"#,
    );

    let err = RequirementManifest::from_json(&with_text, "iso-with-text")
        .expect_err("a manifest carrying `text` must not load, ISO or otherwise");
    let rendered = err.to_string();
    assert!(
        rendered.contains("COORDINATES only") && rendered.contains("sidecar"),
        "the refusal must say where the sentence belongs, or the author \
         deletes the field and loses it; got: {rendered}",
    );
}

/// End to end through the shipped subcommand, on the real pair.
#[test]
fn the_command_classifies_the_real_standard() {
    let output = Command::new(PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen")))
        .arg("requirements")
        .arg("--manifest")
        .arg(manifest_path())
        .arg(document_path())
        .output()
        .expect("sce-codegen runs");
    assert!(
        output.status.success(),
        "sce-codegen requirements --manifest failed: {}",
        String::from_utf8_lossy(&output.stderr),
    );
    let stdout = String::from_utf8(output.stdout).expect("utf-8 stdout");
    for outcome in [
        "implemented",
        "unresolved",
        "missing",
        "dangling",
        "needs-scenario",
    ] {
        assert!(
            stdout.contains(&format!("\"outcome\":\"{outcome}\"")),
            "`{outcome}` has no record on the real standard. stdout:\n{stdout}",
        );
    }
    assert!(
        stdout.contains(r#""kind":"section-coverage""#),
        "the per-section counts are absent. stdout:\n{stdout}",
    );
}

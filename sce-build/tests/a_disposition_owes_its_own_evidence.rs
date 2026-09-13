// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! Requirement-closure RFC ① — Atomic I: a layered standard satisfies
//! most of its requirements SOMEWHERE ELSE, and each "elsewhere" owes
//! different evidence.
//!
//! RFC §5.2e, measured in ISO 13400-2:2019 — a middle layer behaving
//! like one. It defers to the layer below, to the layer above, to IETF
//! documents seven times, and declares something out of scope once. If
//! every one of those reported `missing`, the risk column would fill
//! with correct-but-uninteresting rows until a reader stopped reading
//! it. That is the same failure `needs-scenario` exists to prevent,
//! arriving by a different route.
//!
//! # ⭐ What each disposition owes, and what a citation means
//!
//! ```text
//!   disposition     refused at load unless it carries      a node citing it
//!   implemented     (the annotation, per modality)         is the evidence
//!   delegated       to_doc + to_id, not this document      contradicted
//!   out_of_scope    a reason of more than one word         contradicted
//!   system_level    realised_by                            contradicted
//! ```
//!
//! Every row is exercised in BOTH directions, because either alone
//! proves the wrong thing. A classifier that ignored the disposition
//! would pass a test that only checked delegated rows are not
//! `missing` if the fixture happened to annotate them; one that ignored
//! the citation would pass a test that never cited a delegated id. So
//! each verdict below is paired with the same entry under the other
//! answer, and the pair must differ.
//!
//! # ⭐ Why the evidence is IN the variant
//!
//! `delegated` is the dangerous one: written alone it is a
//! not-our-problem stamp — the requirement leaves this manifest and
//! nothing checks that it arrived anywhere. Binding the destination
//! into the variant means a delegation with no destination cannot be
//! expressed at all, so the wire format refuses it rather than a
//! reviewer having to notice. And a destination filled in BLANK is the
//! same stamp with its label left empty, which the format admits — so
//! the load refuses that too.
//! [`a_disposition_cannot_be_written_without_its_evidence`] drives both
//! kinds of refusal through the real load path.
//!
//! # ⚠ Residue: naming a destination is not confirming one
//!
//! A named target says the delegation was INTENDED. Only the target's
//! own manifest can say it ARRIVED (RFC §5.2e, second clause), and
//! nothing here checks that. A two-manifest check was drafted in this
//! file's first version and removed before landing: given one target
//! manifest it reported every delegation to any OTHER document as
//! broken — and a middle-layer document delegates to several at once —
//! while leaving "arrived" undefined for a target entry that is itself
//! `out_of_scope` or delegated onward. Those are decisions a real second
//! manifest should force.

use std::collections::BTreeMap;

use sce_build::parser::SCXMLParser;
use sce_build::requirement_manifest::{
    classify, emit_classification_ndjson, Classification, Disposition, ManifestError, Outcome,
    RequirementManifest,
};

/// Cites nothing at all.
const BARE: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
                            xmlns:sce="http://sce.dev/ext"
                            version="1.0" name="i" initial="a">
  <state id="a"/>
</scxml>"#;

/// Cites the one requirement satisfied here, and nothing else.
const HERE_ONLY: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
                            xmlns:sce="http://sce.dev/ext"
                            version="1.0" name="i" initial="a">
  <state id="a" sce:req="REQ-HERE"/>
</scxml>"#;

/// Cites every requirement in [`layered_manifest`], and one it lacks.
const ALL_CITED: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
                            xmlns:sce="http://sce.dev/ext"
                            version="1.0" name="i" initial="a">
  <state id="a" sce:req="REQ-HERE">
    <transition event="lower_ready" target="b" sce:req="REQ-BELOW"/>
  </state>
  <state id="b" sce:req="REQ-NOPE"/>
  <state id="c" sce:req="REQ-DEPLOY"/>
  <state id="d" sce:req="REQ-INVENTED"/>
</scxml>"#;

/// The requirements placed elsewhere, with the outcome each owes when
/// no node cites it.
const ELSEWHERE: [(&str, Outcome); 3] = [
    ("REQ-BELOW", Outcome::Delegated),
    ("REQ-NOPE", Outcome::OutOfScope),
    ("REQ-DEPLOY", Outcome::SystemLevel),
];

fn extraction() -> &'static str {
    r#""extraction": { "ids": "native", "trace": "none",
                       "modality_convention": "english-modal-verbs",
                       "method": "hand" }"#
}

/// One manifest carrying every disposition, so no bucket is empty.
fn layered_manifest() -> String {
    format!(
        r#"{{ "doc_id": "middle", "rev": "A", {},
              "requirements": [
                {{ "id": "REQ-HERE" }},
                {{ "id": "REQ-BELOW",
                   "disposition": {{ "kind": "delegated",
                                     "to_doc": "lower", "to_id": "L-1" }} }},
                {{ "id": "REQ-NOPE",
                   "disposition": {{ "kind": "out_of_scope",
                                     "reason": "no transport carries it" }} }},
                {{ "id": "REQ-DEPLOY",
                   "disposition": {{ "kind": "system_level",
                                     "realised_by": "deploy.yaml" }} }}
              ] }}"#,
        extraction()
    )
}

/// A one-requirement manifest whose entry carries `disposition` verbatim.
fn manifest_disposing(disposition: &str) -> String {
    format!(
        r#"{{ "doc_id": "middle", "rev": "A", {},
              "requirements": [{{ "id": "R", "disposition": {disposition} }}] }}"#,
        extraction()
    )
}

fn load(raw: &str, label: &str) -> RequirementManifest {
    RequirementManifest::from_json(raw, label)
        .unwrap_or_else(|e| panic!("fixture {label} must load: {e}"))
}

fn run(scxml: &str, manifest: &RequirementManifest) -> Classification {
    let model = SCXMLParser::new()
        .parse_string(scxml, "i_doc")
        .unwrap_or_else(|e| panic!("fixture parses: {:?}", e.error));
    classify(&model, manifest)
}

fn outcome_of(result: &Classification, id: &str) -> Outcome {
    result
        .outcomes
        .iter()
        .find(|o| o.id == id)
        .unwrap_or_else(|| panic!("{id} has no outcome at all"))
        .outcome
}

/// The same manifest with every disposition returned to the default.
///
/// This is the other half of every pair below: the entry unchanged but
/// for the one field under test, so a verdict that moves between the
/// two moved because of the disposition and nothing else.
fn all_implemented(manifest: &RequirementManifest) -> RequirementManifest {
    let mut stripped = manifest.clone();
    for entry in &mut stripped.requirements {
        entry.disposition = Disposition::default();
    }
    stripped
}

/// Every disposition reaches its own outcome, and reaches it BECAUSE of
/// the disposition.
///
/// ⚠ The pair is what makes this a test of the branch rather than of
/// the fixture. `REQ-BELOW` reading `delegated` would also pass if the
/// classifier ignored the disposition and the fixture happened to be
/// shaped right; reading `missing` under `implemented`, over the same
/// document, is what shows the disposition moved it.
///
/// It also holds [`sce_build::requirement_manifest::RequirementEntry::is_settled_by_annotation`]
/// to the classifier: the transition-table readings filter with that
/// predicate, so if it and the classifier disagreed about which entries
/// a node can settle, the two readings would disagree about `missing`.
#[test]
fn every_disposition_reaches_its_own_outcome_because_of_its_disposition() {
    const FLOOR: usize = 8;

    let layered = load(&layered_manifest(), "middle");
    let stripped = all_implemented(&layered);
    let as_disposed = run(HERE_ONLY, &layered);
    let as_implemented = run(HERE_ONLY, &stripped);

    let mut checked: BTreeMap<&str, usize> = BTreeMap::new();
    let mut wrong: Vec<String> = Vec::new();
    let mut expect = |kind: &'static str, what: String, want: Outcome, got: Outcome| {
        if want == got {
            *checked.entry(kind).or_default() += 1;
        } else {
            wrong.push(format!(
                "  {what}: want {}, got {}",
                want.as_str(),
                got.as_str()
            ));
        }
    };

    // `implemented`: the annotation is the evidence, so removing it is
    // what moves the verdict.
    expect(
        "implemented",
        "REQ-HERE cited".to_string(),
        Outcome::Implemented,
        outcome_of(&as_disposed, "REQ-HERE"),
    );
    expect(
        "implemented",
        "REQ-HERE uncited".to_string(),
        Outcome::Missing,
        outcome_of(&run(BARE, &layered), "REQ-HERE"),
    );
    // The three placed elsewhere: uncited in both runs, so the
    // disposition is the only thing that differs between them.
    for (id, owed) in ELSEWHERE {
        let kind = owed.as_str();
        expect(
            kind,
            format!("{id} as disposed"),
            owed,
            outcome_of(&as_disposed, id),
        );
        expect(
            kind,
            format!("{id} with its disposition removed"),
            Outcome::Missing,
            outcome_of(&as_implemented, id),
        );
    }
    assert!(
        wrong.is_empty(),
        "a disposition did not decide its own outcome:\n{}\n\nA delegated \
         requirement reported as `missing` is true of this document and \
         useless to a reader — for a middle-layer standard that is most \
         of the rows, and a risk column nobody reads protects nobody.",
        wrong.join("\n"),
    );

    // The predicate the table readings use must ask the classifier's
    // question: a node settles exactly the entries whose outcome is one
    // of the presence answers.
    let mut agreed = 0usize;
    for (manifest, result) in [(&layered, &as_disposed), (&stripped, &as_implemented)] {
        for entry in &manifest.requirements {
            let presence = matches!(
                outcome_of(result, &entry.id),
                Outcome::Implemented | Outcome::Unresolved | Outcome::Missing
            );
            assert_eq!(
                entry.is_settled_by_annotation(),
                presence,
                "`{}`: is_settled_by_annotation and the classifier disagree \
                 about whether a node can settle it",
                entry.id,
            );
            agreed += 1;
        }
    }

    let total: usize = checked.values().sum();
    println!("classified {total} disposition case(s) in both directions: {checked:?}");
    println!("held is_settled_by_annotation to the classifier on {agreed} entr(ies)");
    for kind in ["implemented", "delegated", "out-of-scope", "system-level"] {
        assert_eq!(
            checked.get(kind).copied().unwrap_or(0),
            2,
            "`{kind}` must be exercised in both directions",
        );
    }
    assert!(
        total >= FLOOR,
        "only {total} case(s) examined, floor {FLOOR}"
    );
    assert!(
        agreed >= FLOOR,
        "only {agreed} agreement(s) examined, floor {FLOOR}"
    );
}

/// A node citing a requirement the manifest placed elsewhere is a
/// contradiction, not evidence.
///
/// ⭐ Reporting `delegated` here would certify the manifest's claim
/// while the document's contrary claim sat unread in `node_paths`. The
/// pair holds the document fixed and removes the dispositions: every
/// contradicted row must become `implemented`, so the contradiction is
/// the disposition meeting the citation and nothing about the document.
#[test]
fn a_citation_contradicts_a_disposition_that_places_the_requirement_elsewhere() {
    const FLOOR: usize = 3;

    let layered = load(&layered_manifest(), "middle");
    let result = run(ALL_CITED, &layered);
    let control = run(ALL_CITED, &all_implemented(&layered));

    assert_eq!(
        outcome_of(&result, "REQ-HERE"),
        Outcome::Implemented,
        "a citation remains the evidence for a requirement satisfied here",
    );

    let mut contradicted = 0usize;
    for (id, _) in ELSEWHERE {
        let row = result
            .outcomes
            .iter()
            .find(|o| o.id == id)
            .unwrap_or_else(|| panic!("{id} has no outcome"));
        assert_eq!(
            row.outcome,
            Outcome::Contradicted,
            "{id} is placed elsewhere by the manifest and cited here by the \
             document; one of the two claims is wrong, and the verdict must \
             say so rather than certify the first",
        );
        assert!(
            !row.node_paths.is_empty()
                && !matches!(row.disposition, Some(Disposition::Implemented {}) | None),
            "{id}: a contradicted row must carry BOTH claims — the \
             disposition and the nodes — or the reader cannot tell which \
             one is wrong: {row:?}",
        );
        assert_eq!(
            outcome_of(&control, id),
            Outcome::Implemented,
            "{id}: over the same document with the disposition removed the \
             citation is ordinary evidence; if this is not `implemented`, \
             the contradiction above was not the disposition's doing",
        );
        contradicted += 1;
    }

    println!("found {contradicted} disposition(s) contradicted by a citation");
    assert!(
        contradicted >= FLOOR,
        "only {contradicted} examined, floor {FLOOR}"
    );
}

/// ⭐ The evidence reaches the ARTEFACT, not just the type.
///
/// Binding a destination into [`Disposition`] stops the stamp being
/// written; this is what stops it being read. A report that says
/// `"outcome":"delegated"` and nothing else hands a reviewer a verdict
/// they cannot act on — the not-our-problem stamp rebuilt one layer
/// out, where the type system no longer reaches.
#[test]
fn the_artefact_carries_the_evidence_beside_the_verdict() {
    const FLOOR: usize = 3;

    let layered = load(&layered_manifest(), "middle");
    let artefact = |scxml: &str| -> String {
        let mut bytes: Vec<u8> = Vec::new();
        emit_classification_ndjson(&run(scxml, &layered), &mut bytes)
            .expect("writing to a Vec cannot fail");
        String::from_utf8(bytes).expect("the artefact is UTF-8")
    };
    let record = |artefact: &str, id: &str| -> String {
        artefact
            .lines()
            .find(|l| l.contains(&format!(r#""id":"{id}""#)))
            .unwrap_or_else(|| panic!("{id} has no record:\n{artefact}"))
            .to_string()
    };

    // Each disposition placed elsewhere shows what it owes, whether its
    // verdict is its own bucket or a contradiction.
    let owed: [(&str, &str); 3] = [
        ("REQ-BELOW", r#""to_doc":"lower","to_id":"L-1""#),
        ("REQ-NOPE", r#""reason":"no transport carries it""#),
        ("REQ-DEPLOY", r#""realised_by":"deploy.yaml""#),
    ];
    let uncited = artefact(HERE_ONLY);
    let cited = artefact(ALL_CITED);
    let mut carried = 0usize;
    let mut bare: Vec<String> = Vec::new();
    for (id, evidence) in owed {
        for (label, stream) in [("uncited", &uncited), ("contradicted", &cited)] {
            let line = record(stream, id);
            if line.contains(evidence) {
                carried += 1;
            } else {
                bare.push(format!(
                    "  {id} ({label}): expected {evidence} in\n    {line}"
                ));
            }
        }
    }
    assert!(
        bare.is_empty(),
        "a verdict reached the artefact without the evidence it owes:\n{}",
        bare.join("\n"),
    );

    // ...the default is omitted, or the column says "not elsewhere" on
    // nearly every row and a reader learns to skip it...
    let here = record(&uncited, "REQ-HERE");
    assert!(
        !here.contains("disposition"),
        "the default disposition was printed; a column that is nearly \
         always the same word is one a reader skips, including on the \
         rows where it is not:\n  {here}",
    );
    // ...and an id the manifest never listed has no disposition to show,
    // in the artefact or in the library.
    let invented = record(&cited, "REQ-INVENTED");
    assert!(
        invented.contains(r#""outcome":"dangling""#) && !invented.contains("disposition"),
        "a dangling id was given a disposition the manifest never wrote:\n  {invented}",
    );
    let dangling = run(ALL_CITED, &layered);
    assert!(
        dangling
            .outcomes
            .iter()
            .find(|o| o.id == "REQ-INVENTED")
            .is_some_and(|o| o.disposition.is_none()),
        "the library reported a disposition for an id the manifest does not contain",
    );

    println!("carried {carried} disposition evidence(s) into the artefact");
    assert_eq!(carried, owed.len() * 2, "every evidence must be carried");
    assert!(carried >= FLOOR, "only {carried} carried; floor {FLOOR}");
}

/// A disposition cannot be written without the evidence it owes.
///
/// ⭐ Two kinds of refusal, and both are driven through the real load
/// path. The FORMAT refuses a variant missing its field or wearing
/// another variant's: a delegation with no destination has no spelling.
/// The LOAD refuses a field that is present and says nothing, because
/// the format admits `"to_doc": ""` and a blank label is the stamp
/// again.
///
/// Each refusal is matched to its kind and, for the second, to the
/// field it names — a manifest refused for the wrong reason would pass
/// a test that only asked whether it was refused.
#[test]
fn a_disposition_cannot_be_written_without_its_evidence() {
    const FLOOR: usize = 18;

    // Refused by the format, before any value is looked at.
    let unspellable: [(&str, &str, &str); 10] = [
        (
            "delegated",
            r#"{ "kind": "delegated" }"#,
            "a delegation naming no destination",
        ),
        (
            "delegated",
            r#"{ "kind": "delegated", "to_doc": "lower" }"#,
            "a document without the id it becomes there",
        ),
        (
            "delegated",
            r#"{ "kind": "delegated", "to_id": "L-1" }"#,
            "an id without the document that carries it",
        ),
        (
            "delegated",
            r#"{ "kind": "delegated", "reason": "no destination was chosen" }"#,
            "another variant's evidence in place of its own",
        ),
        (
            "out-of-scope",
            r#"{ "kind": "out_of_scope" }"#,
            "out of scope with no reason",
        ),
        (
            "out-of-scope",
            r#"{ "kind": "out_of_scope", "reason": "no transport carries it", "to_doc": "lower" }"#,
            "a reason with a stray destination beside it",
        ),
        (
            "system-level",
            r#"{ "kind": "system_level" }"#,
            "system level naming no artefact",
        ),
        (
            "implemented",
            r#"{ "kind": "implemented", "reason": "no transport carries it" }"#,
            "the default carrying evidence it does not owe",
        ),
        (
            // The dangerous half of the row above. This loaded while
            // `Implemented` was a unit variant: serde does not apply
            // `deny_unknown_fields` to one, so a delegation with its
            // `kind` mistyped became a requirement satisfied here and
            // lost its destination without a word.
            "implemented",
            r#"{ "kind": "implemented", "to_doc": "lower", "to_id": "L-1" }"#,
            "a delegation whose kind was mistyped as the default",
        ),
        (
            "unknown",
            r#"{ "kind": "deferred" }"#,
            "a disposition this format does not have",
        ),
    ];
    // Spellable, and refused at load for saying nothing.
    let blank: [(&str, &str, &str); 8] = [
        (
            "delegated",
            r#"{ "kind": "delegated", "to_doc": "", "to_id": "L-1" }"#,
            "to_doc",
        ),
        (
            "delegated",
            r#"{ "kind": "delegated", "to_doc": "   ", "to_id": "L-1" }"#,
            "to_doc",
        ),
        (
            "delegated",
            r#"{ "kind": "delegated", "to_doc": "middle", "to_id": "L-1" }"#,
            "to_doc",
        ),
        (
            "delegated",
            r#"{ "kind": "delegated", "to_doc": "lower", "to_id": "" }"#,
            "to_id",
        ),
        (
            "out-of-scope",
            r#"{ "kind": "out_of_scope", "reason": "" }"#,
            "reason",
        ),
        (
            "out-of-scope",
            r#"{ "kind": "out_of_scope", "reason": "n/a" }"#,
            "reason",
        ),
        (
            "out-of-scope",
            r#"{ "kind": "out_of_scope", "reason": "  tbd  " }"#,
            "reason",
        ),
        (
            "system-level",
            r#"{ "kind": "system_level", "realised_by": "" }"#,
            "realised_by",
        ),
    ];

    let mut refused: BTreeMap<&str, usize> = BTreeMap::new();
    let mut wrong: Vec<String> = Vec::new();

    for (kind, disposition, why) in unspellable {
        match RequirementManifest::from_json(&manifest_disposing(disposition), "unspellable") {
            Err(ManifestError::Parse { .. }) => *refused.entry(kind).or_default() += 1,
            Err(other) => wrong.push(format!("  {why}: refused, but not by the format: {other}")),
            Ok(_) => wrong.push(format!("  {why}: loaded — {disposition}")),
        }
    }
    for (kind, disposition, field) in blank {
        match RequirementManifest::from_json(&manifest_disposing(disposition), "blank") {
            Err(ref err @ ManifestError::DispositionWithoutEvidence { field: named, .. })
                if named == field =>
            {
                let rendered = err.to_string();
                if rendered.contains(&format!("disposition.{field}")) && rendered.contains("`R`") {
                    *refused.entry(kind).or_default() += 1;
                } else {
                    wrong.push(format!(
                        "  {disposition}: the message must name the requirement and \
                         the field a reviewer has to fill in: {rendered}"
                    ));
                }
            }
            Err(other) => wrong.push(format!(
                "  {disposition}: refused for the wrong reason (want `{field}`): {other}"
            )),
            Ok(_) => wrong.push(format!(
                "  {disposition}: loaded with `{field}` saying nothing"
            )),
        }
    }
    assert!(
        wrong.is_empty(),
        "a disposition loaded without its evidence, or was refused for \
         another reason:\n{}",
        wrong.join("\n"),
    );

    // ...and the complete spellings load, so the refusals above are about
    // the evidence and not about the shape in general.
    let complete = load(&layered_manifest(), "complete");
    assert_eq!(
        complete
            .requirements
            .iter()
            .filter(|e| e.disposition != Disposition::default())
            .count(),
        3,
        "the complete manifest must load all three placed-elsewhere \
         dispositions, or every refusal above proves nothing about evidence",
    );

    let total: usize = refused.values().sum();
    println!("refused {total} disposition(s) written without evidence: {refused:?}");
    for kind in ["delegated", "out-of-scope", "system-level"] {
        assert!(
            refused.get(kind).copied().unwrap_or(0) >= 2,
            "`{kind}` must be refused both by the format and at load",
        );
    }
    assert_eq!(
        total,
        unspellable.len() + blank.len(),
        "every case must be refused"
    );
    assert!(
        total >= FLOOR,
        "only {total} case(s) examined; floor {FLOOR}"
    );
}

/// An `out_of_scope` reason is prose: the author's, not a flag and not
/// the specification's.
///
/// Three ways to write one, and only the middle one is a reason. A
/// single word is a flag spelled as a string, and silence is not a
/// disposition. A sentence in the specification's shape is exactly what
/// the copyright sweep refuses everywhere else — and this field, being
/// the one that is meant to hold words, is the likeliest place for the
/// document's own "not covered" sentence to be pasted, so it is not
/// exempt. The placeholders below are of my own writing; a real
/// sentence here would commit what the sweep exists to keep out.
#[test]
fn an_out_of_scope_reason_is_the_authors_prose() {
    const FLOOR: usize = 4;

    let reason = |text: &str| {
        manifest_disposing(&format!(
            r#"{{ "kind": "out_of_scope", "reason": "{text}" }}"#
        ))
    };
    let mut examined = 0usize;

    // A flag.
    match RequirementManifest::from_json(&reason("tbd"), "flag") {
        Err(ManifestError::DispositionWithoutEvidence {
            field: "reason", ..
        }) => examined += 1,
        other => panic!("a one-word reason must be refused as missing evidence: {other:?}"),
    }
    // The specification's shape, by punctuation and by modal.
    for text in [
        "Placeholder signalling is not covered by this placeholder part.",
        "placeholder signalling shall be carried by another placeholder part",
    ] {
        match RequirementManifest::from_json(&reason(text), "pasted") {
            Err(ManifestError::Prose { field, .. }) if field.ends_with("disposition.reason") => {
                examined += 1
            }
            other => panic!(
                "a reason in the specification's shape must be refused by the \
                 prose sweep, naming the field: {text:?} -> {other:?}"
            ),
        }
    }
    // The author's words.
    let authored = load(
        &reason("no statechart can observe a physical line"),
        "authored",
    );
    let verdict = run(BARE, &authored);
    assert_eq!(
        outcome_of(&verdict, "R"),
        Outcome::OutOfScope,
        "a reason in the author's own words must load and be honoured",
    );
    examined += 1;

    println!("examined {examined} out_of_scope reason(s): flag, two pasted, one authored");
    assert!(examined >= FLOOR, "only {examined} examined; floor {FLOOR}");
}

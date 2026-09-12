// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! Requirement-closure RFC ① — Atomic K ⑴: a manifest declares how it
//! was made, and the coverage artefact carries that declaration.
//!
//! RFC §5.2g, measured across the corpus a real consumer holds: the
//! inputs come in grades and the weakest is the most common. One
//! standard publishes its own requirement ids and its own tracing
//! chapter; another publishes ids and no trace; an OEM engineering
//! standard publishes neither, and a machine pass over its prose has to
//! invent the list before anything can be counted.
//!
//! A coverage figure over the first and a coverage figure over the
//! third are not the same kind of number, and printing both as "94 %"
//! is a lie of omission. So the manifest declares what is true of its
//! extraction, and the artefact carries it to whoever reads the
//! percentage.
//!
//! # ⭐ Why the declaration is PROPERTIES and never a source's name
//!
//! Owner's instruction, 2026-09-12: *"범용적으로 만들어야 해, 특정
//! 스펙에 종속되면 안 돼"*. A field whose values were `"autosar"` or
//! `"iso"` would make SCE the keeper of a catalogue of standards — and
//! the next source, an OEM document or a spreadsheet or a ticket
//! system, has no entry in that catalogue. Asking what is TRUE of the
//! extraction instead (`ids: native`, `trace: published`) is a question
//! a source SCE has never heard of can answer.
//!
//! ⚠ `a_standard_named_in_code_is_one_sce_implements` refuses a
//! specification's name in executable code, and it is silent about
//! DATA: nothing in it would notice a manifest field carrying `"iso"`.
//! That is why the refusal here has to be structural. Every one of the
//! four fields is a closed enum, so a specification's name is refused
//! without SCE holding any list of specifications to check against —
//! the name is simply not a member.
//!
//! # What this file guards, beyond "it parses"
//!
//! Following the discipline `requirement_manifest_closure.rs` sets out
//! for the four outcomes, and for the same reason — a sweep that
//! examined nothing prints the same green as one that examined
//! everything:
//!
//!   1. **both denominator kinds have a non-empty example**, driven end
//!      to end into the emitted artefact, and the two manifests differ
//!      in exactly one field so the difference is attributable;
//!   2. the tests **print what they examined** and **assert a floor**
//!      under it;
//!   3. the closed-enum refusal is checked with values somebody would
//!      plausibly write — `iso-iec-directives` is the sharpest, because
//!      that is where the verbal-form table actually came from.

use sce_build::parser::SCXMLParser;
use sce_build::requirement_manifest::{
    classify, emit_classification_ndjson, Extraction, Ids, Method, ModalityConvention,
    RequirementManifest, Trace,
};

/// Values a real author might plausibly write, each naming a document
/// rather than describing an extraction.
///
/// `iso-iec-directives` is the sharpest of them: that IS where the
/// verbal-form table came from, so it is the wrong answer most likely
/// to look right to somebody filling the field in.
///
/// Used twice — once to check that no field ACCEPTS such a value, and
/// once to check that no declaration EMITS one. The two are different
/// failures (an open field against a field that was never asked for)
/// and one list is enough for both.
const NAMES: &[&str] = &[
    "iso",
    "autosar",
    "iso-13400-2",
    "iso-iec-directives",
    "sae-j1939",
    "doip",
];

/// A document carrying two of the three ids the manifests declare, so
/// every case below classifies something rather than nothing.
const DOC: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
                            xmlns:sce="http://sce.dev/ext"
                            version="1.0" name="k1" initial="idle">
  <state id="idle" sce:req="REQ-1">
    <transition event="go" target="running" sce:req="REQ-2"/>
  </state>
  <state id="running"/>
</scxml>"#;

/// The declaration under test, as JSON, with one field substituted.
///
/// Written as a substitution rather than as two hand-written manifests
/// because the claim is that ONE field moves the artefact. Two
/// hand-written manifests could differ somewhere else and the test
/// would still pass, which is the shape of evidence that proves the
/// wrong thing.
fn manifest_json(ids: &str) -> String {
    format!(
        r#"{{ "doc_id": "placeholder-spec", "rev": "A",
              "extraction": {{ "ids": "{ids}", "trace": "none",
                               "modality_convention": "english-modal-verbs",
                               "method": "hand" }},
              "requirements": [{{ "id": "REQ-1" }}, {{ "id": "REQ-2" }},
                               {{ "id": "REQ-3" }}] }}"#
    )
}

/// A valid extraction block, as the TYPE spells it.
///
/// ⭐ Serialised from [`Extraction`] rather than typed out, so the field
/// list below is derived and not remembered. A hand-written list checks
/// the fields its author happened to know about: a fifth field added
/// tomorrow would be absent from it, and the refusal sweep would report
/// full coverage while never touching the new field. That failure is
/// not hypothetical — an exclusion list in
/// `a_standard_named_in_code_is_one_sce_implements` named six of this
/// tree's sixteen test directories for exactly this reason, and was
/// found by accident rather than by any check.
fn valid_block() -> serde_json::Map<String, serde_json::Value> {
    let valid = Extraction {
        ids: Ids::Native,
        trace: Trace::None,
        modality_convention: ModalityConvention::EnglishModalVerbs,
        method: Method::Hand,
    };
    match serde_json::to_value(valid).expect("Extraction serialises") {
        serde_json::Value::Object(map) => map,
        other => panic!("the extraction block must serialise as an object, got {other}"),
    }
}

/// A manifest whose extraction block is the valid one, with one field
/// replaced by `substitute`.
///
/// Built as a MAP, so a substitution replaces rather than appends. An
/// appended key would leave the field written twice and serde would
/// refuse the duplicate with a message of its own — every row of the
/// sweep would then be "refused" for a reason unrelated to the value
/// under test, which is this suite's most-repeated way of proving the
/// wrong thing.
fn extraction_manifest(substitute: Option<(&str, &str)>) -> String {
    let mut block = valid_block();
    if let Some((field, value)) = substitute {
        assert!(
            block.contains_key(field),
            "`{field}` is not a field of the extraction block; the sweep \
             would be substituting into nothing",
        );
        block.insert(field.to_string(), serde_json::Value::String(value.into()));
    }
    serde_json::json!({
        "doc_id": "d",
        "rev": "A",
        "extraction": block,
        "requirements": [{ "id": "REQ-1" }],
    })
    .to_string()
}

fn parse(scxml: &str, label: &str) -> sce_build::model::SCXMLModel {
    SCXMLParser::new()
        .parse_string(scxml, label)
        .unwrap_or_else(|e| panic!("fixture {label} must parse: {:?}", e.error))
}

/// The artefact `sce-codegen requirements --manifest` writes, as lines.
fn artefact_lines(ids: &str) -> (Vec<String>, usize) {
    let model = parse(DOC, "k1_doc");
    let declared = RequirementManifest::from_json(&manifest_json(ids), "k1_manifest")
        .unwrap_or_else(|e| panic!("fixture manifest ({ids}) must load: {e}"));
    let classification = classify(&model, &declared);
    let classified = classification.outcomes.len();
    let mut bytes: Vec<u8> = Vec::new();
    emit_classification_ndjson(&classification, &mut bytes).expect("writing to a Vec cannot fail");
    let text = String::from_utf8(bytes).expect("the artefact is UTF-8");
    (text.lines().map(str::to_string).collect(), classified)
}

/// Both denominator kinds reach the artefact, from a non-empty case.
///
/// ⚠ The assertion that matters is the LAST one: that the two artefacts
/// disagree. Each half alone would pass against an emitter that had the
/// answer hard-coded — `contains("derived")` is true of a line that
/// says `derived` whatever the manifest declared. Only the comparison
/// shows the declaration is being read.
#[test]
fn both_denominator_kinds_have_a_non_empty_example() {
    /// Requirements each case must classify before its answer counts.
    const FLOOR_PER_CASE: usize = 3;
    /// Denominator kinds that must each have an example.
    const FLOOR_CASES: usize = 2;

    let cases: [(&str, &str); 2] = [("native", "derived"), ("synthesized", "synthesized")];

    let mut examined = 0usize;
    let mut extraction_lines: Vec<String> = Vec::new();
    for (ids, expected_basis) in cases {
        let (lines, classified) = artefact_lines(ids);
        assert!(
            classified >= FLOOR_PER_CASE,
            "the `{ids}` case classified {classified} requirement(s), floor \
             {FLOOR_PER_CASE} — an empty case proves nothing about the \
             denominator it would have been over",
        );
        let line = lines
            .iter()
            .find(|l| l.contains(r#""kind":"extraction""#))
            .unwrap_or_else(|| {
                panic!(
                    "the `{ids}` artefact carries no extraction record; a \
                     percentage whose denominator has no stated standing is \
                     the lie of omission this milestone exists to stop.\n{}",
                    lines.join("\n")
                )
            })
            .clone();
        assert!(
            line.contains(&format!(r#""denominator":"{expected_basis}""#)),
            "the `{ids}` artefact should report a `{expected_basis}` \
             denominator, and says:\n  {line}",
        );
        assert!(
            line.contains(&format!(r#""ids":"{ids}""#)),
            "the artefact must carry what the manifest DECLARED beside what \
             SCE concluded, so a reader who disagrees with the conclusion can \
             see its input:\n  {line}",
        );
        examined += classified;
        extraction_lines.push(line);
    }

    assert_ne!(
        extraction_lines[0], extraction_lines[1],
        "both denominator kinds produced the SAME record, so the artefact is \
         not reading the manifest's declaration — the two manifests differ in \
         exactly one field and that field is supposed to be the difference",
    );

    println!(
        "examined {} requirement outcome(s) across {} denominator kind(s)",
        examined,
        extraction_lines.len()
    );
    assert!(
        extraction_lines.len() >= FLOOR_CASES,
        "only {} denominator kind(s) examined, floor {FLOOR_CASES}",
        extraction_lines.len()
    );
}

/// The declaration precedes every number in the artefact.
///
/// Not cosmetic. A reader who stops partway through an NDJSON stream
/// should never have met a count without having met the standing of the
/// denominator it is over; putting the record last would guarantee the
/// opposite for anyone who skims.
#[test]
fn the_declaration_precedes_every_number() {
    let (lines, classified) = artefact_lines("synthesized");
    assert!(
        classified > 0,
        "a case that classified nothing orders nothing"
    );
    let first = lines.first().expect("the artefact is not empty");
    assert!(
        first.contains(r#""kind":"extraction""#),
        "the extraction record must be the first line of the artefact, and \
         the first line is:\n  {first}",
    );
    println!("checked ordering over {} artefact line(s)", lines.len());
    assert!(
        lines.len() > 1,
        "an artefact of one line cannot show that anything precedes anything",
    );
}

/// A manifest that does not say how it was made is refused.
///
/// ⚠ This is the assertion that makes the other tests mean something.
/// An OPTIONAL declaration would let every future manifest omit it, and
/// an omitted declaration is indistinguishable from a trustworthy one
/// at the point the percentage is read — which is precisely the defect.
#[test]
fn a_manifest_that_does_not_say_how_it_was_made_is_refused() {
    let undeclared = r#"{ "doc_id": "d", "rev": "A",
                          "requirements": [{ "id": "REQ-1" }] }"#;
    let err = RequirementManifest::from_json(undeclared, "undeclared")
        .expect_err("a manifest with no extraction block must not load")
        .to_string();
    assert!(
        err.contains("extraction"),
        "the refusal must name the missing block so the author knows what to \
         write; it says: {err}",
    );

    // ...and the same manifest WITH the block loads, so the refusal above
    // is about the block and not about something else in the fixture.
    let declared = r#"{ "doc_id": "d", "rev": "A",
                        "extraction": { "ids": "native", "trace": "none",
                                        "modality_convention": "english-modal-verbs",
                                        "method": "hand" },
                        "requirements": [{ "id": "REQ-1" }] }"#;
    assert!(
        RequirementManifest::from_json(declared, "declared").is_ok(),
        "the control must load, or the refusal above proves nothing",
    );
}

/// A value that names a specification is refused, in every field.
///
/// ⭐ The refusal is structural rather than a check: each field is a
/// closed enum, so SCE needs no list of specifications to recognise
/// one. That matters more than it looks — a denylist of standard names
/// in production code would itself be the defect `§5.2h` forbids, and
/// would go red under
/// `a_standard_named_in_code_is_one_sce_implements`.
///
/// ⚠ This test is also what keeps the fields from quietly becoming
/// `String`. A free-text `modality_convention` would accept every value
/// below, and nothing else in this suite would notice.
#[test]
fn a_value_that_names_a_specification_is_refused() {
    /// Fields the block has today. A fifth one is covered the day it
    /// lands, because the list is derived — this floor only has to fail
    /// when the derivation stops working.
    const FIELD_FLOOR: usize = 4;

    let control = extraction_manifest(None);
    assert!(
        RequirementManifest::from_json(&control, "control").is_ok(),
        "the unsubstituted manifest must load, or every refusal below is \
         about the fixture rather than about the value:\n{control}",
    );

    // ⭐ Derived from the type, never listed. This is the whole reason
    // the helper serialises an `Extraction` instead of typing the block
    // out: a field added tomorrow is swept on the day it appears, and a
    // field that is a `String` accepts every name below and fails here.
    let fields: Vec<String> = valid_block().keys().cloned().collect();
    assert!(
        fields.len() >= FIELD_FLOOR,
        "the extraction block reports {} field(s), floor {FIELD_FLOOR} — the \
         derivation stopped working, so this sweep covers nothing",
        fields.len(),
    );

    let mut refused = 0usize;
    let mut admitted: Vec<String> = Vec::new();
    for field in &fields {
        for name in NAMES {
            let raw = extraction_manifest(Some((field, name)));
            match RequirementManifest::from_json(&raw, "named") {
                Err(_) => refused += 1,
                Ok(_) => admitted.push(format!("  {field} accepted \"{name}\"")),
            }
        }
    }

    assert!(
        admitted.is_empty(),
        "a manifest field admitted a specification's name:\n{}\n\n\
         Every field of the extraction block is a closed set of PROPERTIES. A \
         field that takes a document's name makes SCE the keeper of a \
         catalogue of standards, and the next source has no entry in it — \
         which is the dependence RFC §5.2h forbids, arriving through data \
         instead of through code.",
        admitted.join("\n"),
    );

    println!(
        "refused {refused} specification-named value(s) across {} derived field(s): {}",
        fields.len(),
        fields.join(", "),
    );
    assert_eq!(
        refused,
        fields.len() * NAMES.len(),
        "every attempt must be refused, and {refused} of {} were",
        fields.len() * NAMES.len(),
    );
    // An absolute floor beside the relative one, because `refused == 0`
    // also satisfies `refused == fields * names` when either list is
    // empty — a sweep can be perfectly consistent about nothing.
    const ATTEMPT_FLOOR: usize = 20;
    assert!(
        refused >= ATTEMPT_FLOOR,
        "only {refused} attempt(s) were made, floor {ATTEMPT_FLOOR}",
    );
}

/// The denominator's standing follows what can be AUDITED, not how much
/// care went in.
///
/// ⚠ A non-empty example on both sides of the distinction that the
/// obvious reading gets wrong: a hand-typed list of invented ids is
/// careful and still unauditable, while an unreviewed machine pass over
/// a document's own tags is checkable against that document. If
/// `method` ever starts feeding the basis, this fails.
#[test]
fn care_does_not_substitute_for_auditability() {
    let careful_but_invented = Extraction {
        ids: Ids::Synthesized,
        trace: Trace::None,
        modality_convention: ModalityConvention::EnglishModalVerbs,
        method: Method::Hand,
    };
    let unreviewed_but_checkable = Extraction {
        ids: Ids::Native,
        trace: Trace::None,
        modality_convention: ModalityConvention::EnglishModalVerbs,
        method: Method::AiPass1,
    };
    assert_ne!(
        careful_but_invented.denominator_basis(),
        unreviewed_but_checkable.denominator_basis(),
        "these two differ in BOTH fields and must land on different bases; \
         reading them as the same would mean the basis is answering neither \
         question",
    );
    assert_eq!(
        unreviewed_but_checkable.denominator_basis(),
        Extraction {
            method: Method::Derived,
            ..unreviewed_but_checkable
        }
        .denominator_basis(),
        "changing only `method` moved the denominator's basis — care is a \
         claim about confidence, and the basis answers whether the list can \
         be checked at all",
    );
    println!("checked 3 extraction shape(s) against the basis rule");
}

/// The three source shapes RFC §5.2g measured are told apart by these
/// properties alone — and the data names none of them.
///
/// ⭐ This is the owner's instruction stated as an assertion. §5.2g
/// lettered its three grades A/B/C purely to make the section readable,
/// and the warning attached to that lettering is that it must never
/// reach the schema. So the shapes below are described in COMMENTS by
/// what they are, and expressed in DATA by what is true of them; the
/// test then requires that the data alone separates all three.
///
/// ⚠ This is also `trace: published`'s only non-empty case. Before it,
/// that value was declarable and constructed by nothing — a variant no
/// test reaches is a variant whose behaviour nobody has checked, and
/// the milestone asks for an example per item precisely to stop that.
#[test]
fn three_real_source_shapes_are_told_apart_without_naming_any() {
    // The measurements behind each row are in RFC §5.2g: 2536 published
    // ids with a tracing chapter; 166 published ids and no such chapter;
    // and an engineering standard with no ids at all, whose list a
    // machine pass has to invent before anything can be counted.
    let shapes: [(&str, Extraction); 3] = [
        (
            "publishes its own ids AND its own requirement-to-artefact map",
            Extraction {
                ids: Ids::Native,
                trace: Trace::Published,
                modality_convention: ModalityConvention::EnglishModalVerbs,
                method: Method::Derived,
            },
        ),
        (
            "publishes its own ids, and no map",
            Extraction {
                ids: Ids::Native,
                trace: Trace::None,
                modality_convention: ModalityConvention::EnglishModalVerbs,
                method: Method::AiPass1,
            },
        ),
        (
            "publishes neither, so the list had to be invented from prose",
            Extraction {
                ids: Ids::Synthesized,
                trace: Trace::None,
                modality_convention: ModalityConvention::EnglishModalVerbs,
                method: Method::AiPass1,
            },
        ),
    ];

    let mut pairs = 0usize;
    for (i, (what_a, a)) in shapes.iter().enumerate() {
        for (what_b, b) in shapes.iter().skip(i + 1) {
            assert_ne!(
                a, b,
                "two source shapes are indistinguishable by their declared \
                 properties:\n  {what_a}\n  {what_b}\n\nIf the properties \
                 cannot separate them, the only thing that could is a name — \
                 which is the dependence this whole block exists to avoid.",
            );
            pairs += 1;
        }
    }

    // ...and none of them says WHICH document it is. A `source` field
    // would pass every assertion above and be the defect anyway.
    for (what, shape) in &shapes {
        let json = serde_json::to_string(shape).expect("Extraction serialises");
        for name in NAMES {
            assert!(
                !json.contains(name),
                "the declaration for `{what}` carries `{name}`, which names a \
                 document rather than describing the extraction:\n  {json}",
            );
        }
    }

    println!(
        "told {} source shape(s) apart over {pairs} pair(s), naming none",
        shapes.len()
    );
    assert!(
        pairs >= 3,
        "only {pairs} pair(s) compared; three shapes make three pairs, and \
         fewer means the sweep stopped covering them",
    );
}

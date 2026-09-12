// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! Requirement-closure RFC ① — Atomic K ⑵: the coordinate is a variant,
//! and the division it groups under survives every shape of it.
//!
//! RFC §5.2g measured the corpus a real consumer holds — 146 PDF, 3
//! xlsx, 1 docx, 1 arxml — and the landed manifest could address only
//! the first of those: `section` plus `page`. A spreadsheet requirement
//! is at a row, a structured document's at a path, and a field named
//! `page` makes every non-paginated source lose the coordinate that
//! makes a review report checkable at all.
//!
//! # ⭐ What is NOT in the variant, and why that is the design
//!
//! `section` stays a plain key beside the position rather than being
//! folded into it. It is what
//! [`sce_build::requirement_manifest::Classification::section_counts`]
//! groups by, and RFC §5.2a calls those per-division counts its only
//! handle on omission — the one report that can say a whole division
//! yielded nothing. Had the division gone into the variant, every shape
//! would have had to answer "which group" its own way, and for a
//! path-addressed source there is no honest answer without measuring
//! one, which nobody has done. Keeping it out means a workbook names
//! its sheet and a structured document names its package in the same
//! slot a specification names its subclause, and the omission report
//! works for all of them unchanged.
//!
//! That claim is what [`the_division_survives_every_shape_of_position`]
//! holds: the same three divisions must come back with the same counts
//! whichever shape addresses them.
//!
//! # What this file guards, beyond "it parses"
//!
//! Each position shape gets a NON-EMPTY case driven into the emitted
//! artefact, the tests print what they examined, and each asserts a
//! floor — the discipline `requirement_manifest_closure.rs` sets out,
//! for the reason it gives: a sweep that examined nothing prints the
//! same green as one that examined everything.

use sce_build::parser::SCXMLParser;
use sce_build::provenance::{Position, SpecProvenance};
use sce_build::requirement_manifest::{classify, emit_classification_ndjson, RequirementManifest};

/// The three shapes, each with the division it sits in and the wire
/// spelling of its position.
///
/// Written out rather than derived, because these are FIXTURES: a case
/// drawn from the tree would test the tree as it stands, and the shape
/// this file exists for is the one that arrives from a source nobody
/// has loaded yet.
/// How many shapes there must be at least.
///
/// ⚠ An absolute floor beside every `== SHAPES.len()` below, because
/// that equality is satisfied by ZERO: a `SHAPES` trimmed to nothing
/// would make all three tests here agree perfectly about nothing and
/// report green. The same hole was written once already, in the
/// extraction-block suite, and caught there — so it is closed here by
/// construction rather than by remembering.
const SHAPE_FLOOR: usize = 3;

const SHAPES: &[(&str, &str, &str)] = &[
    // (division, wire position, what kind of source spells it this way)
    ("12.6.1.2", r#"{ "page": 68 }"#, "a paginated specification"),
    ("Sheet1", r#"{ "row": 41 }"#, "a worksheet"),
    (
        "Pkg/Diag",
        r#"{ "path": "/Elem/x" }"#,
        "a structured document",
    ),
];

/// A document citing all three requirements, so no case classifies
/// nothing.
const DOC: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
                            xmlns:sce="http://sce.dev/ext"
                            version="1.0" name="k2" initial="a">
  <state id="a" sce:req="REQ-1">
    <transition event="go" target="b" sce:req="REQ-2"/>
  </state>
  <state id="b" sce:req="REQ-3"/>
</scxml>"#;

/// A manifest declaring one requirement per shape, each in its own
/// division.
fn manifest_json() -> String {
    let sections = SHAPES
        .iter()
        .map(|(division, _, _)| format!(r#"{{ "id": "{division}", "title": "d" }}"#))
        .collect::<Vec<_>>()
        .join(", ");
    let requirements = SHAPES
        .iter()
        .enumerate()
        .map(|(i, (division, at, _))| {
            format!(
                r#"{{ "id": "REQ-{}", "section": "{division}", "at": {at} }}"#,
                i + 1
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        r#"{{ "doc_id": "d", "rev": "A",
              "extraction": {{ "ids": "native", "trace": "none",
                               "modality_convention": "english-modal-verbs",
                               "method": "hand" }},
              "sections": [{sections}],
              "requirements": [{requirements}] }}"#
    )
}

fn loaded() -> RequirementManifest {
    RequirementManifest::from_json(&manifest_json(), "k2_manifest")
        .unwrap_or_else(|e| panic!("a manifest of mixed locator shapes must load: {e}"))
}

/// Every position shape reaches the emitted artefact from a non-empty
/// case.
#[test]
fn every_position_shape_has_a_non_empty_example() {
    let model = SCXMLParser::new()
        .parse_string(DOC, "k2_doc")
        .unwrap_or_else(|e| panic!("fixture parses: {:?}", e.error));
    let classification = classify(&model, &loaded());
    assert!(
        classification.outcomes.len() >= SHAPES.len(),
        "only {} outcome(s) for {} shape(s) — a case that classified \
         nothing says nothing about the coordinate it would have carried",
        classification.outcomes.len(),
        SHAPES.len(),
    );

    let mut bytes: Vec<u8> = Vec::new();
    emit_classification_ndjson(&classification, &mut bytes).expect("writing to a Vec cannot fail");
    let artefact = String::from_utf8(bytes).expect("the artefact is UTF-8");

    let mut found = 0usize;
    let mut missing: Vec<String> = Vec::new();
    for (division, at, what) in SHAPES {
        // The wire spelling with its whitespace removed is what serde
        // writes, so a hit proves the shape survived the round trip
        // rather than proving this file can concatenate strings.
        let compact: String = at.chars().filter(|c| !c.is_whitespace()).collect();
        if artefact.contains(&compact) {
            found += 1;
        } else {
            missing.push(format!("  {what}: `{compact}` (division {division})"));
        }
    }
    assert!(
        missing.is_empty(),
        "a position shape did not reach the artefact:\n{}\n\nA source \
         whose coordinate the report cannot print is one a reviewer \
         cannot open, which is the whole of what the locator is for.\n{artefact}",
        missing.join("\n"),
    );

    println!("carried {found} position shape(s) into the artefact");
    assert_eq!(found, SHAPES.len(), "every shape must be carried");
    assert!(
        found >= SHAPE_FLOOR,
        "only {found} shape(s) examined, floor {SHAPE_FLOOR} — agreeing \
         about nothing is not the same as carrying every shape",
    );
}

/// ⭐ The division groups the same way whatever shape addresses it.
///
/// This is the constraint that decided the design. `section_counts` is
/// RFC §5.2a's only handle on omission, and it must not start depending
/// on how the source spells its coordinates.
#[test]
fn the_division_survives_every_shape_of_position() {
    let model = SCXMLParser::new()
        .parse_string(DOC, "k2_doc")
        .unwrap_or_else(|e| panic!("fixture parses: {:?}", e.error));
    let counts = classify(&model, &loaded()).section_counts;

    let mut checked = 0usize;
    let mut wrong: Vec<String> = Vec::new();
    for (division, _, what) in SHAPES {
        match counts.iter().find(|(id, _)| id == division) {
            Some((_, 1)) => checked += 1,
            Some((_, n)) => wrong.push(format!(
                "  {what}: division `{division}` counted {n}, want 1"
            )),
            None => wrong.push(format!(
                "  {what}: division `{division}` is absent from the per-division counts"
            )),
        }
    }
    assert!(
        wrong.is_empty(),
        "the omission report lost a division:\n{}\n\nThe division is a \
         plain key precisely so that a workbook's sheet and a \
         specification's subclause group the same way. If grouping has \
         started to depend on the position's shape, the coordinate has \
         absorbed something that was never part of it.\n  counts: {counts:?}",
        wrong.join("\n"),
    );

    println!("grouped {checked} division(s), one per position shape");
    assert_eq!(checked, SHAPES.len(), "every division must be grouped");
    assert!(
        checked >= SHAPE_FLOOR,
        "only {checked} division(s) grouped, floor {SHAPE_FLOOR}",
    );
}

/// The same variant serves the document side, through the compact form
/// a `sce:provenance` attribute is written in.
///
/// ⚠ One type, two surfaces, and the test is here because the failure
/// it guards is silent: the codegen template renders this by reading
/// the IR through string keys, so a rename drops the coordinate from
/// every backend's output with nothing to compile against.
#[test]
fn the_document_side_carries_every_shape_too() {
    let cases: [(&str, Position); 3] = [
        ("S#12.6.1.2:page=68", Position::Page(68)),
        ("S#Sheet1:row=41", Position::Row(41)),
        ("S#Pkg/Diag:path=/Elem/x", Position::Path("/Elem/x".into())),
    ];
    let mut checked = 0usize;
    for (compact, expected) in cases {
        let parsed = SpecProvenance::parse_compact(compact)
            .unwrap_or_else(|| panic!("`{compact}` must parse"));
        assert_eq!(parsed.at, Some(expected), "for `{compact}`");
        checked += 1;
    }
    println!("parsed {checked} document-side coordinate shape(s)");
    assert_eq!(checked, SHAPES.len(), "both surfaces cover the same shapes");
    assert!(
        checked >= SHAPE_FLOOR,
        "only {checked} coordinate shape(s) parsed, floor {SHAPE_FLOOR}",
    );
}

/// The fourth source shape: addressed by its DIVISION alone.
///
/// ⭐ RFC §5.2g names four shapes, and this is the one with no position
/// at all — a flowed document's requirement sits at a heading, and the
/// heading IS its division. The design covers it by the position being
/// optional rather than by a fourth variant, which is why it needs a
/// case of its own: a shape that works because nothing forbids it is a
/// promise with nothing behind it, and it would keep passing if the
/// field ever stopped being optional.
///
/// ⚠ The sharp assertion is the last one. It is not enough that the
/// entry loads — the artefact must OMIT the position rather than invent
/// one, because a coordinate a reviewer cannot act on is worse than an
/// absent one: it sends them to a page that does not exist.
#[test]
fn a_requirement_addressed_by_its_division_alone_is_carried() {
    let raw = r#"{ "doc_id": "d", "rev": "A",
                   "extraction": { "ids": "native", "trace": "none",
                                   "modality_convention": "english-modal-verbs",
                                   "method": "hand" },
                   "sections": [{ "id": "Timing requirements", "title": "h" }],
                   "requirements": [{ "id": "REQ-1",
                                      "section": "Timing requirements" }] }"#;
    let declared = RequirementManifest::from_json(raw, "heading_manifest")
        .unwrap_or_else(|e| panic!("a division-only entry must load: {e}"));
    let model = SCXMLParser::new()
        .parse_string(DOC, "k2_doc")
        .unwrap_or_else(|e| panic!("fixture parses: {:?}", e.error));
    let classification = classify(&model, &declared);

    let counts = &classification.section_counts;
    assert_eq!(
        counts.iter().find(|(id, _)| id == "Timing requirements"),
        Some(&("Timing requirements".to_string(), 1)),
        "a heading groups like any other division: {counts:?}",
    );

    let mut bytes: Vec<u8> = Vec::new();
    emit_classification_ndjson(&classification, &mut bytes).expect("writing to a Vec cannot fail");
    let artefact = String::from_utf8(bytes).expect("the artefact is UTF-8");
    let record = artefact
        .lines()
        .find(|l| l.contains(r#""id":"REQ-1""#))
        .unwrap_or_else(|| panic!("REQ-1 has no record:\n{artefact}"));
    assert!(
        !record.contains(r#""at":"#),
        "the artefact invented a position for an entry that declared \
         none, which sends a reviewer somewhere that does not exist:\n  {record}",
    );
    assert!(
        record.contains(r#""section":"Timing requirements""#),
        "the division must survive even when nothing follows it:\n  {record}",
    );
    println!("carried 1 division-only requirement, position omitted");
}

// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A `delegated` requirement is a "not our problem" stamp, and until
//! something reads the document it names, the stamp can be pressed
//! anywhere — Requirement-closure RFC §5.2e.
//!
//! [`RequirementSet`] holds several manifests and follows each claim to
//! its end. This file pins what it must catch AND what it must leave
//! alone, because a checker that refused everything would catch the
//! defects too:
//!
//! | case | must be |
//! |---|---|
//! | the destination carries the id | silent |
//! | the destination is in the set and lacks the id | a defect |
//! | the chain returns to a document already stood in | a defect |
//! | the destination is not in the set at all | neither — and COUNTED |
//!
//! ⚠ The fourth row is why `answered()` is asserted everywhere a zero
//! defect count is: zero out of zero questions is what an empty set
//! returns, and it looks exactly like a clean bill of health.

use sce_build::requirement_manifest::RequirementManifest;
use sce_build::requirement_set::{Arrival, RequirementRef, RequirementSet, Terminus};

/// A manifest whose requirements are given as JSON objects, built
/// through `from_json` rather than by struct literal: the wire is the
/// door a real manifest arrives by, and `decomposes_into` has to survive
/// it.
fn manifest(doc_id: &str, requirements: &str) -> RequirementManifest {
    let raw = format!(
        r#"{{
          "doc_id": "{doc_id}",
          "rev": "1",
          "extraction": {{
            "ids": "native",
            "trace": "none",
            "modality_convention": "english-modal-verbs",
            "method": "hand"
          }},
          "requirements": [{requirements}]
        }}"#
    );
    RequirementManifest::from_json(&raw, doc_id)
        .unwrap_or_else(|e| panic!("{doc_id} should load: {e}"))
}

fn delegated(id: &str, to_doc: &str, to_id: &str) -> String {
    format!(
        r#"{{"id": "{id}", "disposition": {{"kind": "delegated",
            "to_doc": "{to_doc}", "to_id": "{to_id}"}}}}"#
    )
}

fn implemented(id: &str) -> String {
    format!(r#"{{"id": "{id}"}}"#)
}

fn set(manifests: Vec<RequirementManifest>) -> RequirementSet {
    RequirementSet::new(manifests).expect("distinct doc-ids")
}

fn only(report: &sce_build::requirement_set::SetReport) -> &Arrival {
    assert_eq!(
        report.verdicts.len(),
        1,
        "expected exactly one cross-document claim: {:?}",
        report.verdicts
    );
    &report.verdicts[0].arrival
}

// ── The control, first: what must NOT be caught ──────────────────

/// A delegation whose destination carries the id is silent.
///
/// ⚠ This is the control for every assertion below. Without it a
/// checker that reported EVERY delegation as broken — which is exactly
/// how the first attempt at this check failed, per RFC §5.2e — would
/// satisfy all the defect cases in this file.
#[test]
fn a_delegation_that_arrives_is_not_a_defect() {
    let report = set(vec![
        manifest("upper", &delegated("U-1", "lower", "L-1")),
        manifest("lower", &implemented("L-1")),
    ])
    .report();

    assert_eq!(report.defects().count(), 0, "{:?}", report.verdicts);
    assert_eq!(report.unchecked().count(), 0, "{:?}", report.verdicts);
    // The floor under that zero: the question was actually asked.
    assert_eq!(report.answered(), 1, "{:?}", report.verdicts);
    assert!(
        matches!(
            only(&report),
            Arrival::Arrived {
                terminus: Terminus::Implemented,
                ..
            }
        ),
        "{:?}",
        only(&report)
    );
}

/// A delegation that travels several documents still arrives.
///
/// The second half of the control: a middle layer delegating onward is
/// the case RFC §5.2e recorded as leaving "arrived" undefined, and it
/// must not be mistaken for a loop.
#[test]
fn a_delegation_that_travels_several_documents_still_arrives() {
    let report = set(vec![
        manifest("top", &delegated("T-1", "mid", "M-1")),
        manifest("mid", &delegated("M-1", "bottom", "B-1")),
        manifest("bottom", &implemented("B-1")),
    ])
    .report();

    assert_eq!(report.defects().count(), 0, "{:?}", report.verdicts);
    assert_eq!(report.answered(), 2, "{:?}", report.verdicts);
    let travelled = report
        .verdicts
        .iter()
        .find(|v| v.from.id == "T-1")
        .expect("T-1 is judged");
    match &travelled.arrival {
        Arrival::Arrived { at, hops, .. } => {
            assert_eq!(at.doc, "bottom");
            assert_eq!(at.id, "B-1");
            assert_eq!(hops.len(), 3, "the chain is reported, not just its end");
        }
        other => panic!("T-1 should arrive: {other:?}"),
    }
}

// ── The defects ──────────────────────────────────────────────────

/// The destination is in the set and does not carry the id.
#[test]
fn a_delegation_the_destination_never_took_is_caught() {
    let report = set(vec![
        manifest("upper", &delegated("U-1", "lower", "L-404")),
        manifest("lower", &implemented("L-1")),
    ])
    .report();

    assert_eq!(report.defects().count(), 1, "{:?}", report.verdicts);
    assert_eq!(report.unchecked().count(), 0, "{:?}", report.verdicts);
    match only(&report) {
        Arrival::NotArrived { at } => {
            assert_eq!(at.doc, "lower");
            assert_eq!(at.id, "L-404");
        }
        other => panic!("expected a non-arrival: {other:?}"),
    }
    // The sentence a reader gets has to name both halves.
    let said = only(&report).to_string();
    assert!(said.contains("lower") && said.contains("L-404"), "{said}");
}

/// Every link resolves and the requirement is met nowhere.
///
/// The shape a one-hop check cannot see: both ends would read as
/// arrived, because each names a document that really does carry the id.
#[test]
fn a_delegation_cycle_is_caught_although_every_link_resolves() {
    let cycle = set(vec![
        manifest("a", &delegated("A-1", "b", "B-1")),
        manifest("b", &delegated("B-1", "a", "A-1")),
    ]);
    let report = cycle.report();

    assert_eq!(
        report.defects().count(),
        2,
        "both ends are defects: {:?}",
        report.verdicts
    );
    for verdict in &report.verdicts {
        assert!(
            matches!(verdict.arrival, Arrival::Cycle { .. }),
            "{verdict:?}"
        );
    }

    // The discriminator against the control above: EVERY link resolves.
    // If it did not, this would be caught as a non-arrival and the cycle
    // logic would never be exercised.
    for (doc, id) in [("a", "A-1"), ("b", "B-1")] {
        assert!(
            matches!(
                cycle.resolve(&RequirementRef {
                    doc: doc.into(),
                    id: id.into()
                }),
                sce_build::requirement_set::Resolution::Found(_)
            ),
            "{id} must exist, or this test proves something else"
        );
    }
}

// ── Neither: the set was too small ───────────────────────────────

/// A destination outside the set is reported as unchecked, not as
/// arrived and not as broken.
///
/// ⚠⚠ The half that keeps this check honest. If an absent manifest read
/// as a pass, the whole check would be defeated by leaving it off the
/// command line; if it read as a defect, no partial set could ever be
/// used, which is the failure that sank the first attempt.
#[test]
fn a_destination_outside_the_set_is_unchecked_and_counted() {
    let report = set(vec![manifest("upper", &delegated("U-1", "lower", "L-1"))]).report();

    assert_eq!(report.defects().count(), 0, "not a defect");
    assert_eq!(report.unchecked().count(), 1, "but not silent either");
    assert_eq!(
        report.answered(),
        0,
        "and it does not count toward what was answered"
    );
    let said = only(&report).to_string();
    assert!(said.contains("lower"), "{said}");
}

// ── Arrival is presence; what the target says is separate ────────

/// Arriving at an `out_of_scope` entry is an arrival AND a refusal, and
/// the verdict carries both.
#[test]
fn arriving_at_an_out_of_scope_entry_reports_both_facts() {
    let report = set(vec![
        manifest("upper", &delegated("U-1", "lower", "L-1")),
        manifest(
            "lower",
            r#"{"id": "L-1", "disposition": {"kind": "out_of_scope",
                "reason": "realised in the transceiver hardware"}}"#,
        ),
    ])
    .report();

    assert_eq!(
        report.defects().count(),
        0,
        "it did arrive: {:?}",
        report.verdicts
    );
    match only(&report) {
        Arrival::Arrived {
            terminus: Terminus::OutOfScope { reason },
            ..
        } => assert_eq!(reason, "realised in the transceiver hardware"),
        other => panic!("expected an out-of-scope terminus: {other:?}"),
    }
    assert!(
        only(&report).to_string().contains("hardware"),
        "the reason reaches the reader: {}",
        only(&report)
    );
}

// ── Decomposition: the other half of G4 ──────────────────────────

/// A requirement split into children is expressed, and its children are
/// resolved by the same walk.
#[test]
fn a_decomposed_requirement_names_children_that_are_checked() {
    let arrives = set(vec![
        manifest(
            "upper",
            r#"{"id": "U-1", "decomposes_into": [
                 {"doc": "upper", "id": "U-1a"},
                 {"doc": "lower", "id": "L-1"}]}"#,
        ),
        manifest("lower", &implemented("L-1")),
    ]);
    // The parent really did survive the wire.
    let parent = arrives
        .report()
        .verdicts
        .iter()
        .filter(|v| v.from.id == "U-1")
        .count();
    assert_eq!(parent, 2, "both children are judged");

    let report = arrives.report();
    assert_eq!(
        report.defects().count(),
        1,
        "U-1a is named and does not exist: {:?}",
        report.verdicts
    );
    assert_eq!(report.answered(), 2);
    let broken: Vec<String> = report.defects().map(ToString::to_string).collect();
    assert_eq!(broken.len(), 1);
    assert!(broken[0].contains("U-1a"), "{:?}", broken);

    // CONTROL: once the child exists, the same manifest is silent — so
    // the defect above is the missing child, not the decomposition.
    let whole = set(vec![
        manifest(
            "upper",
            r#"{"id": "U-1", "decomposes_into": [
                 {"doc": "upper", "id": "U-1a"},
                 {"doc": "lower", "id": "L-1"}]},
               {"id": "U-1a"}"#,
        ),
        manifest("lower", &implemented("L-1")),
    ])
    .report();
    assert_eq!(whole.defects().count(), 0, "{:?}", whole.verdicts);
    assert_eq!(whole.answered(), 2, "and both were still asked");
}

/// Undecomposed is the default, so every manifest written before the
/// field keeps its meaning.
#[test]
fn a_manifest_without_the_field_decomposes_into_nothing() {
    let m = manifest("solo", &implemented("S-1"));
    assert!(m.requirements[0].decomposes_into.is_empty());
    let report = set(vec![m]).report();
    assert_eq!(report.verdicts.len(), 0, "no claim points out of it");
}

/// Two manifests claiming one `doc_id` are refused rather than silently
/// collapsed, because every later answer about that document would come
/// from whichever one won.
#[test]
fn a_set_refuses_two_manifests_claiming_one_doc_id() {
    let err = RequirementSet::new(vec![
        manifest("same", &implemented("A")),
        manifest("same", &implemented("B")),
    ])
    .expect_err("a duplicate doc-id is refused");
    assert_eq!(err.kind(), "duplicate-doc-id");
    assert!(err.to_string().contains("same"), "{err}");
}

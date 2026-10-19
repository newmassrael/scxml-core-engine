// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A document being divided among specs, kept beside them (ADR 0040).
//!
//! What is held here is what the store does around the ledger: that an intake is kept in no spec and is
//! never listed as one, that its ledger moves by a save that names its base, that a ledger and the count of it
//! must name each other and the intake's extraction, and above all that a proposal is admitted as a spec's part
//! of the division only when its scope is exactly what the ledger gives that spec. Each refusal is paired with
//! the nearest input that must not be refused, so that a store that refuses everything fails the second half.

mod common;

use std::path::PathBuf;
use std::sync::Arc;

use sce_specs::{
    call, IntakeLink, IntakeRegistration, ManualClock, MemberDraft, OriginalDraft,
    ProposalRegistration, Revision, Role, SpecId, SpecStore, StoreError,
};
use serde_json::{json, Value};

use common::{scratch, FakeRenderer};

const T0: u64 = 1_791_190_800;
const BLOCKS: [&str; 6] = ["b1", "b2", "b3", "b4", "b5", "b6"];

struct Rig {
    root: PathBuf,
    store: SpecStore<Arc<ManualClock>>,
}

fn rig(label: &str) -> Rig {
    let root = scratch(label);
    let store = SpecStore::with_clock(&root, Arc::new(ManualClock::at(T0)));
    Rig { root, store }
}

/// What the authoring tools write of a document of six blocks, which the store keeps as it is given.
fn extraction_text() -> String {
    json!({
        "v": 1, "extractor": "plain", "extractor_version": "1", "member": "spec.md",
        "blocks": BLOCKS.iter().map(|id| json!({"id": id})).collect::<Vec<_>>(),
        "not_carried": [],
    })
    .to_string()
}

fn draft() -> OriginalDraft {
    OriginalDraft {
        label: "from the customer".to_string(),
        members: vec![MemberDraft::new(
            "spec.md",
            Role::Document,
            b"The lamp. The fan.".to_vec(),
        )],
        missing: vec![],
    }
}

fn registration<'a>(key: &'a str, extraction: &'a str) -> IntakeRegistration<'a> {
    IntakeRegistration {
        key,
        origin: "gui",
        title: "Controller spec",
        draft: draft(),
        member: "spec.md",
        extraction,
    }
}

fn refusal(error: StoreError) -> (&'static str, Value) {
    match error {
        StoreError::Refused { kind, detail, .. } => (kind, detail),
        other => panic!("expected a refusal, got {other:?}"),
    }
}

/// A ledger of the intake's extraction, as the tools write one (the store keeps the text as given).
fn ledger_of(extraction: &Revision, allotments: Value) -> String {
    json!({"v": 1, "extraction": extraction.as_str(), "allotments": allotments}).to_string()
}

/// SCE's count of a ledger, as `check-ledger --json` writes it.
fn report_of(extraction: &Revision, ledger: &str, complete: bool) -> String {
    json!({
        "extraction": extraction.as_str(),
        "ledger": Revision::of(ledger.as_bytes()).as_str(),
        "complete": complete,
        "unplaced": if complete { json!([]) } else { json!(["b6"]) },
    })
    .to_string()
}

fn spec_of(r: &Rig, title: &str) -> SpecId {
    r.store.create_spec(title).unwrap().id
}

/// The allotments of a division: the lamp (b1 to b3) to `lamp`, the fan (b4 to b6) to a spec to be made.
fn lamp_and_fan(lamp: &SpecId) -> Value {
    json!([
        {"to": {"kind": "spec", "spec": lamp.as_str()}, "blocks": ["b1", "b2", "b3"]},
        {"to": {"kind": "new", "title": "Fan"}, "blocks": ["b4", "b5", "b6"]},
    ])
}

struct Started {
    id: String,
    extraction: Revision,
    original: Revision,
}

fn start(r: &Rig, key: &str) -> Started {
    let text = extraction_text();
    let started = r.store.start_intake(registration(key, &text)).unwrap();
    Started {
        id: started.intake.id,
        extraction: started.intake.extraction,
        original: started.intake.original,
    }
}

fn save(
    r: &Rig,
    s: &Started,
    allotments: Value,
    complete: bool,
    base: Option<&Revision>,
) -> Revision {
    let ledger = ledger_of(&s.extraction, allotments);
    let report = report_of(&s.extraction, &ledger, complete);
    r.store
        .save_ledger(&s.id, &ledger, &report, base)
        .unwrap()
        .entry
        .digest
}

// ------------------------------------------------------------------------------ starting an intake

#[test]
fn an_intake_keeps_the_document_and_is_never_listed_as_a_spec() {
    let r = rig("intake-start");
    let text = extraction_text();

    let started = r
        .store
        .start_intake(registration("press-1", &text))
        .unwrap();

    assert!(started.created);
    assert_eq!(started.intake.title, "Controller spec");
    assert_eq!(started.intake.extraction, Revision::of(text.as_bytes()));
    // It is kept in the specs folder's root, in a folder that begins with a dot, and no spec is made of it.
    assert!(r
        .root
        .join(".sce-intakes")
        .join(&started.intake.id)
        .is_dir());
    assert!(r.store.list_specs().unwrap().specs.is_empty());
    assert_eq!(
        r.store.list_intakes().unwrap(),
        vec![started.intake.clone()]
    );
    let view = r.store.read_intake(&started.intake.id).unwrap();
    assert_eq!(view.intake, started.intake);
    assert!(view.ledger.is_none() && view.destinations.is_empty());
}

#[test]
fn the_same_press_again_is_the_intake_it_already_made_and_another_document_under_its_key_is_refused(
) {
    let r = rig("intake-key");
    let text = extraction_text();
    let first = r
        .store
        .start_intake(registration("press-1", &text))
        .unwrap();

    let again = r
        .store
        .start_intake(registration("press-1", &text))
        .unwrap();
    let mut other = registration("press-1", &text);
    other.draft.members[0].bytes = b"Another document.".to_vec();
    let refused = r.store.start_intake(other).unwrap_err();

    assert!(!again.created);
    assert_eq!(again.intake.id, first.intake.id);
    assert_eq!(refusal(refused).0, "key-reused");
    assert_eq!(r.store.list_intakes().unwrap().len(), 1);
    // Another key is another intake, even of the same document.
    assert!(
        r.store
            .start_intake(registration("press-2", &text))
            .unwrap()
            .created
    );
}

#[test]
fn a_document_the_store_refuses_leaves_no_intake_behind() {
    let r = rig("intake-refused");
    let text = extraction_text();
    let wrong_member = IntakeRegistration {
        member: "other.md",
        ..registration("press-1", &text)
    };

    let refused = r.store.start_intake(wrong_member).unwrap_err();
    let not_json = r
        .store
        .start_intake(registration("press-2", "not json"))
        .unwrap_err();

    assert_eq!(refusal(refused).0, "bad-original");
    assert_eq!(refusal(not_json).0, "bad-original");
    assert!(r.store.list_intakes().unwrap().is_empty());
    let kept: Vec<_> = std::fs::read_dir(r.root.join(".sce-intakes"))
        .unwrap()
        .filter_map(Result::ok)
        .filter(|entry| entry.path().is_dir())
        .collect();
    assert!(kept.is_empty(), "{kept:?}");
    // The nearest that must pass.
    assert!(r
        .store
        .start_intake(registration("press-3", &extraction_text()))
        .is_ok());
}

#[test]
fn a_key_and_an_origin_that_are_not_names_are_refused() {
    let r = rig("intake-names");
    let text = extraction_text();

    for (key, origin) in [("", "gui"), ("k", "not a name!"), (&"k".repeat(201), "gui")] {
        let refused = r
            .store
            .start_intake(IntakeRegistration {
                origin,
                ..registration(key, &text)
            })
            .unwrap_err();
        assert_eq!(refusal(refused).0, "bad-proposal", "{key} {origin}");
    }
}

#[test]
fn an_intake_that_is_not_there_is_not_found_and_an_id_that_is_not_one_is_invalid() {
    let r = rig("intake-missing");

    let missing = r.store.read_intake("intake-00000000").unwrap_err();
    let invalid = r.store.read_intake("../x").unwrap_err();

    assert_eq!(missing.kind(), "not-found");
    assert_eq!(invalid.kind(), "invalid-id");
}

// ---------------------------------------------------------------------------------- the ledger

#[test]
fn a_ledger_is_kept_with_its_count_and_read_back_with_where_each_spec_has_come() {
    let r = rig("ledger-save");
    let s = start(&r, "k");
    let lamp = spec_of(&r, "Lamp");
    let allotments = lamp_and_fan(&lamp);
    let ledger = ledger_of(&s.extraction, allotments);
    let report = report_of(&s.extraction, &ledger, true);

    let saved = r.store.save_ledger(&s.id, &ledger, &report, None).unwrap();
    let view = r.store.read_intake(&s.id).unwrap();

    assert!(saved.created);
    assert_eq!(
        (
            saved.entry.n,
            saved.entry.complete,
            saved.entry.parent.clone()
        ),
        (1, true, None)
    );
    let head = view.ledger.unwrap();
    assert_eq!((head.text, head.report), (ledger, report));
    assert_eq!(head.entry.digest, saved.entry.digest);
    // The spec that exists is named; the spec to be made is not made yet, so it has no spec and no proposal.
    assert_eq!(view.destinations.len(), 2);
    assert_eq!(view.destinations[0].spec.as_deref(), Some(lamp.as_str()));
    assert!(view.destinations[0].proposal.is_none() && !view.destinations[0].spec_missing);
    assert_eq!(view.destinations[1].spec, None);
}

#[test]
fn a_ledger_moves_by_a_save_that_names_its_base() {
    let r = rig("ledger-base");
    let s = start(&r, "k");
    let lamp = spec_of(&r, "Lamp");
    let first = save(&r, &s, lamp_and_fan(&lamp), true, None);

    // Not naming the head is the refusal every head gives, and nothing is written.
    let stale = {
        let ledger = ledger_of(
            &s.extraction,
            json!([{"to": {"kind": "left-out"}, "reason": "", "blocks": ["b1"]}]),
        );
        let report = report_of(&s.extraction, &ledger, false);
        r.store
            .save_ledger(&s.id, &ledger, &report, None)
            .unwrap_err()
    };
    let second = save(
        &r,
        &s,
        json!([{"to": {"kind": "spec", "spec": lamp.as_str()}, "blocks": ["b1", "b2"]}]),
        false,
        Some(&first),
    );

    assert_eq!(stale.kind(), "conflict");
    let view = r.store.read_intake(&s.id).unwrap();
    let head = view.ledger.unwrap();
    assert_eq!(
        (head.entry.n, head.entry.digest.clone(), head.entry.parent),
        (2, second, Some(first))
    );
    assert!(!head.entry.complete);
}

#[test]
fn saving_the_head_again_writes_nothing_and_says_so() {
    let r = rig("ledger-same");
    let s = start(&r, "k");
    let lamp = spec_of(&r, "Lamp");
    let ledger = ledger_of(&s.extraction, lamp_and_fan(&lamp));
    let report = report_of(&s.extraction, &ledger, true);
    let first = r.store.save_ledger(&s.id, &ledger, &report, None).unwrap();

    let again = r
        .store
        .save_ledger(&s.id, &ledger, &report, Some(&first.entry.digest))
        .unwrap();

    assert!(!again.created);
    assert_eq!(again.entry, first.entry);
    let lines = std::fs::read_to_string(r.root.join(".sce-intakes").join(&s.id).join("ledger.log"))
        .unwrap();
    assert_eq!(lines.lines().count(), 1);
}

#[test]
fn a_ledger_that_is_not_one_of_this_intake_or_not_in_form_is_refused_and_the_nearest_good_one_is_not(
) {
    let r = rig("ledger-form");
    let s = start(&r, "k");
    let lamp = spec_of(&r, "Lamp");
    let good = lamp_and_fan(&lamp);
    let spec =
        |blocks: Value| json!([{"to": {"kind": "spec", "spec": lamp.as_str()}, "blocks": blocks}]);
    let other = Revision::of(b"another extraction");
    let cases: Vec<(&str, String)> = vec![
        ("another extraction", ledger_of(&other, good.clone())),
        (
            "a block the extraction does not have",
            ledger_of(&s.extraction, spec(json!(["b1", "b99"]))),
        ),
        (
            "blocks out of order",
            ledger_of(&s.extraction, spec(json!(["b2", "b1"]))),
        ),
        (
            "a block twice",
            ledger_of(&s.extraction, spec(json!(["b1", "b1"]))),
        ),
        (
            "an allotment of no block",
            ledger_of(&s.extraction, spec(json!([]))),
        ),
        (
            "a reason for a part that is not left out",
            ledger_of(
                &s.extraction,
                json!([{"to": {"kind": "spec", "spec": lamp.as_str()}, "reason": "why", "blocks": ["b1"]}]),
            ),
        ),
        (
            "a spec id that is not one",
            ledger_of(
                &s.extraction,
                json!([{"to": {"kind": "spec", "spec": "Lamp/../x"}, "blocks": ["b1"]}]),
            ),
        ),
        (
            "a kind nobody defined",
            ledger_of(
                &s.extraction,
                json!([{"to": {"kind": "elsewhere"}, "blocks": ["b1"]}]),
            ),
        ),
        (
            "two allotments to one destination",
            ledger_of(
                &s.extraction,
                json!([
                    {"to": {"kind": "spec", "spec": lamp.as_str()}, "blocks": ["b1"]},
                    {"to": {"kind": "spec", "spec": lamp.as_str()}, "blocks": ["b2"]},
                ]),
            ),
        ),
        ("not JSON", "[1".to_string()),
    ];
    for (name, ledger) in cases {
        let report = report_of(&s.extraction, &ledger, true);

        let refused = r
            .store
            .save_ledger(&s.id, &ledger, &report, None)
            .unwrap_err();

        assert_eq!(refusal(refused).0, "bad-ledger", "{name}");
    }
    // The nearest that must pass: a ledger that places nothing yet, and the good division.
    assert!(r.store.read_intake(&s.id).unwrap().ledger.is_none());
    save(&r, &s, json!([]), false, None);
}

#[test]
fn a_count_that_does_not_name_the_ledger_and_extraction_is_refused() {
    let r = rig("ledger-count");
    let s = start(&r, "k");
    let lamp = spec_of(&r, "Lamp");
    let ledger = ledger_of(&s.extraction, lamp_and_fan(&lamp));
    let of_another_ledger = report_of(&s.extraction, "another ledger", true);
    let of_another_extraction = report_of(&Revision::of(b"x"), &ledger, true);
    let no_verdict = json!({
        "extraction": s.extraction.as_str(),
        "ledger": Revision::of(ledger.as_bytes()).as_str(),
    })
    .to_string();

    for (name, report) in [
        ("another ledger", of_another_ledger),
        ("another extraction", of_another_extraction),
        ("no word on whether it is complete", no_verdict),
        ("not JSON", "[".to_string()),
    ] {
        let refused = r
            .store
            .save_ledger(&s.id, &ledger, &report, None)
            .unwrap_err();
        assert_eq!(refusal(refused).0, "bad-ledger", "{name}");
    }
    r.store
        .save_ledger(
            &s.id,
            &ledger,
            &report_of(&s.extraction, &ledger, true),
            None,
        )
        .unwrap();
}

// ------------------------------------------------------------------- a spec to be made, and the copy

#[test]
fn a_spec_to_be_made_is_bound_once_and_to_a_title_the_ledger_gives() {
    let r = rig("bind");
    let s = start(&r, "k");
    let lamp = spec_of(&r, "Lamp");
    let fan = spec_of(&r, "Fan");
    let elsewhere = spec_of(&r, "Elsewhere");
    // Before there is a ledger there is no spec to be made.
    assert_eq!(
        refusal(r.store.bind_new_spec(&s.id, "Fan", &fan).unwrap_err()).0,
        "bad-ledger"
    );
    save(&r, &s, lamp_and_fan(&lamp), true, None);

    let bound = r.store.bind_new_spec(&s.id, "Fan", &fan).unwrap();
    let again = r.store.bind_new_spec(&s.id, "Fan", &fan).unwrap();
    let no_such_title = r
        .store
        .bind_new_spec(&s.id, "Wheel", &elsewhere)
        .unwrap_err();
    let other_spec = r.store.bind_new_spec(&s.id, "Fan", &elsewhere).unwrap_err();
    let removed = SpecId::parse("not-a-spec-00000000").unwrap();
    let no_spec = r.store.bind_new_spec(&s.id, "Fan", &removed).unwrap_err();

    assert!(bound.created && !again.created);
    assert_eq!(again.binding, bound.binding);
    assert_eq!(refusal(no_such_title).0, "bad-ledger");
    assert_eq!(refusal(other_spec).0, "already-bound");
    assert_eq!(no_spec.kind(), "not-found");
    // The intake now says which spec the destination is, and that the spec has no proposal yet.
    let view = r.store.read_intake(&s.id).unwrap();
    assert_eq!(view.destinations[1].spec.as_deref(), Some(fan.as_str()));
    assert_eq!(view.bindings.len(), 1);
    // A spec is bound to one title: it cannot also be the spec of another one.
    let two_titles = ledger_of(
        &s.extraction,
        json!([
            {"to": {"kind": "new", "title": "Fan"}, "blocks": ["b1", "b2", "b3"]},
            {"to": {"kind": "new", "title": "Wheel"}, "blocks": ["b4", "b5", "b6"]},
        ]),
    );
    let head = view.ledger.unwrap().entry.digest;
    r.store
        .save_ledger(
            &s.id,
            &two_titles,
            &report_of(&s.extraction, &two_titles, true),
            Some(&head),
        )
        .unwrap();
    assert_eq!(
        refusal(r.store.bind_new_spec(&s.id, "Wheel", &fan).unwrap_err()).0,
        "already-bound"
    );
}

#[test]
fn the_document_is_copied_into_a_spec_by_hash_and_copying_it_again_is_the_same() {
    let r = rig("give");
    let s = start(&r, "k");
    let lamp = spec_of(&r, "Lamp");

    let given = r.store.give_intake_to_spec(&s.id, &lamp).unwrap();
    let again = r.store.give_intake_to_spec(&s.id, &lamp).unwrap();

    assert_eq!(
        (&given.original, &given.extraction),
        (&s.original, &s.extraction)
    );
    assert_eq!(again, given);
    // The spec holds the document as its own, once, and can be proposed from it.
    assert_eq!(r.store.list_originals(&lamp).unwrap().len(), 1);
    assert_eq!(r.store.list_extractions(&lamp, None).unwrap().len(), 1);
    assert_eq!(
        r.store.read_extraction(&lamp, &s.extraction).unwrap(),
        extraction_text()
    );
    assert_eq!(
        r.store
            .read_original_member(&lamp, &s.original, "spec.md")
            .unwrap(),
        b"The lamp. The fan.".to_vec()
    );
    let gone = SpecId::parse("not-a-spec-00000000").unwrap();
    assert_eq!(
        r.store
            .give_intake_to_spec(&s.id, &gone)
            .unwrap_err()
            .kind(),
        "not-found"
    );
}

// ------------------------------------------------------------- admitting a proposal as a spec's part

fn conversion_of(extraction: &Revision, cites: &[&str]) -> String {
    json!({
        "v": 1,
        "converter": {"kind": "rules", "name": "blocks-to-markdown", "version": "1"},
        "extraction": extraction.as_str(),
        "blocks": cites.iter().enumerate()
            .map(|(i, cite)| json!({"id": format!("m{}", i + 1), "markdown": "x", "cites": [cite]}))
            .collect::<Vec<_>>(),
    })
    .to_string()
}

fn scope_of(extraction: &Revision, blocks: &[&str]) -> String {
    json!({"v": 1, "extraction": extraction.as_str(), "blocks": blocks}).to_string()
}

/// A proposal for `spec` of `blocks` (`None` for the whole), made under the ledger `ledger`, as a host makes it.
struct Proposing<'a> {
    spec: &'a SpecId,
    key: &'a str,
    blocks: Option<&'a [&'a str]>,
    ledger: &'a Revision,
}

fn propose(
    r: &Rig,
    s: &Started,
    p: Proposing<'_>,
) -> Result<sce_specs::ProposalRegistered, StoreError> {
    let cites: Vec<&str> = p.blocks.map_or_else(|| BLOCKS.to_vec(), <[&str]>::to_vec);
    let conversion = conversion_of(&s.extraction, &cites);
    let scope = p.blocks.map(|blocks| scope_of(&s.extraction, blocks));
    let mut report: Value = json!({
        "extraction": s.extraction.as_str(),
        "conversion": Revision::of(conversion.as_bytes()).as_str(),
        "complete": true,
        "queue": [],
    });
    if let Some(scope) = &scope {
        report["scope"] = json!(Revision::of(scope.as_bytes()).as_str());
    }
    let report = report.to_string();
    r.store.propose_import(
        p.spec,
        ProposalRegistration {
            key: p.key,
            origin: "gui",
            original: s.original.clone(),
            extraction: s.extraction.clone(),
            conversion: &conversion,
            report: &report,
            scope: scope.as_deref(),
            intake: Some(IntakeLink {
                id: s.id.clone(),
                ledger: p.ledger.clone(),
            }),
            expect: r.store.head(p.spec).unwrap(),
            supersede: false,
        },
    )
}

/// An intake whose ledger gives `lamp` the blocks b1 to b3 and a spec to be made as `fan` b4 to b6.
fn divided(r: &Rig, label: &str) -> (Started, SpecId, SpecId, Revision) {
    let _ = label;
    let s = start(r, "k");
    let lamp = spec_of(r, "Lamp");
    let fan = spec_of(r, "Fan");
    let head = save(r, &s, lamp_and_fan(&lamp), true, None);
    r.store.bind_new_spec(&s.id, "Fan", &fan).unwrap();
    r.store.give_intake_to_spec(&s.id, &lamp).unwrap();
    r.store.give_intake_to_spec(&s.id, &fan).unwrap();
    (s, lamp, fan, head)
}

#[test]
fn a_proposal_that_is_exactly_what_the_ledger_gives_its_spec_is_admitted_and_says_which_division_it_is_of(
) {
    let r = rig("admit-ok");
    let (s, lamp, fan, head) = divided(&r, "ok");

    let for_lamp = propose(
        &r,
        &s,
        Proposing {
            spec: &lamp,
            key: "a",
            blocks: Some(&["b1", "b2", "b3"]),
            ledger: &head,
        },
    )
    .unwrap();
    let for_fan = propose(
        &r,
        &s,
        Proposing {
            spec: &fan,
            key: "b",
            blocks: Some(&["b4", "b5", "b6"]),
            ledger: &head,
        },
    )
    .unwrap();

    let link = for_lamp.proposal.intake.clone().unwrap();
    assert_eq!((link.id.as_str(), &link.ledger), (s.id.as_str(), &head));
    assert!(for_fan.proposal.intake.is_some());
    // The intake says how far each spec has come: both have a proposal that waits.
    let view = r.store.read_intake(&s.id).unwrap();
    let states: Vec<_> = view
        .destinations
        .iter()
        .map(|d| d.proposal.as_ref().map(|p| p.state.clone()))
        .collect();
    assert_eq!(
        states,
        vec![Some("pending".to_string()), Some("pending".to_string())]
    );
    assert_eq!(
        view.destinations[0].proposal.as_ref().unwrap().id,
        for_lamp.proposal.id
    );
}

#[test]
fn a_proposal_whose_scope_is_not_what_the_ledger_gives_its_spec_is_refused() {
    let r = rig("admit-scope");
    let (s, lamp, _fan, head) = divided(&r, "scope");

    for (name, blocks) in [
        ("a part of what it is given", Some(&["b1", "b2"][..])),
        ("more than it is given", Some(&["b1", "b2", "b3", "b4"][..])),
        ("what another spec is given", Some(&["b4", "b5", "b6"][..])),
        ("the whole document, when it is given a part", None),
    ] {
        let refused = propose(
            &r,
            &s,
            Proposing {
                spec: &lamp,
                key: name,
                blocks,
                ledger: &head,
            },
        )
        .unwrap_err();
        assert_eq!(refusal(refused).0, "bad-proposal", "{name}");
    }
    // The nearest that must pass: exactly what it is given.
    propose(
        &r,
        &s,
        Proposing {
            spec: &lamp,
            key: "good",
            blocks: Some(&["b1", "b2", "b3"]),
            ledger: &head,
        },
    )
    .unwrap();
}

#[test]
fn a_spec_the_ledger_gives_every_block_is_proposed_without_a_scope() {
    let r = rig("admit-whole");
    let s = start(&r, "k");
    let all = spec_of(&r, "All");
    let head = save(
        &r,
        &s,
        json!([{"to": {"kind": "spec", "spec": all.as_str()}, "blocks": BLOCKS}]),
        true,
        None,
    );
    r.store.give_intake_to_spec(&s.id, &all).unwrap();

    let with_scope = propose(
        &r,
        &s,
        Proposing {
            spec: &all,
            key: "a",
            blocks: Some(&["b1", "b2"]),
            ledger: &head,
        },
    )
    .unwrap_err();
    let whole = propose(
        &r,
        &s,
        Proposing {
            spec: &all,
            key: "b",
            blocks: None,
            ledger: &head,
        },
    )
    .unwrap();

    assert_eq!(refusal(with_scope).0, "bad-proposal");
    assert!(whole.proposal.scope.is_none() && whole.proposal.intake.is_some());
}

#[test]
fn a_proposal_made_under_a_ledger_that_has_moved_or_that_was_not_complete_is_refused() {
    let r = rig("admit-ledger");
    let (s, lamp, fan, head) = divided(&r, "ledger");
    // The ledger moves: the owner changes the division after the proposal was prepared.
    let moved = save(
        &r,
        &s,
        json!([
            {"to": {"kind": "spec", "spec": lamp.as_str()}, "blocks": ["b1", "b2", "b3", "b4"]},
            {"to": {"kind": "new", "title": "Fan"}, "blocks": ["b5", "b6"]},
        ]),
        true,
        Some(&head),
    );

    let stale = propose(
        &r,
        &s,
        Proposing {
            spec: &lamp,
            key: "a",
            blocks: Some(&["b1", "b2", "b3"]),
            ledger: &head,
        },
    )
    .unwrap_err();
    let (kind, detail) = refusal(stale);
    let current = propose(
        &r,
        &s,
        Proposing {
            spec: &lamp,
            key: "b",
            blocks: Some(&["b1", "b2", "b3", "b4"]),
            ledger: &moved,
        },
    );

    assert_eq!(kind, "ledger-moved");
    assert_eq!(detail["current"], json!(moved));
    assert!(current.is_ok());
    // A ledger SCE did not count complete is the head, and nothing is proposed from it.
    let incomplete = save(
        &r,
        &s,
        json!([
            {"to": {"kind": "spec", "spec": lamp.as_str()}, "blocks": ["b1", "b2", "b3", "b4"]},
            {"to": {"kind": "new", "title": "Fan"}, "blocks": ["b5"]},
        ]),
        false,
        Some(&moved),
    );
    let refused = propose(
        &r,
        &s,
        Proposing {
            spec: &fan,
            key: "c",
            blocks: Some(&["b5"]),
            ledger: &incomplete,
        },
    )
    .unwrap_err();
    assert_eq!(refusal(refused).0, "ledger-incomplete");
}

#[test]
fn a_spec_the_ledger_gives_nothing_and_a_document_that_is_not_the_intakes_are_refused() {
    let r = rig("admit-stranger");
    let (s, _lamp, _fan, head) = divided(&r, "stranger");
    let stranger = spec_of(&r, "Stranger");
    r.store.give_intake_to_spec(&s.id, &stranger).unwrap();

    let nothing = propose(
        &r,
        &s,
        Proposing {
            spec: &stranger,
            key: "a",
            blocks: Some(&["b1", "b2", "b3"]),
            ledger: &head,
        },
    )
    .unwrap_err();
    let no_such_intake = {
        let mut other = Started {
            id: "intake-00000000".to_string(),
            extraction: s.extraction.clone(),
            original: s.original.clone(),
        };
        other.id = "intake-00000000".to_string();
        propose(
            &r,
            &other,
            Proposing {
                spec: &stranger,
                key: "b",
                blocks: None,
                ledger: &head,
            },
        )
        .unwrap_err()
    };

    assert_eq!(refusal(nothing).0, "bad-proposal");
    assert_eq!(no_such_intake.kind(), "not-found");
}

#[test]
fn the_same_press_under_another_division_is_another_input() {
    let r = rig("admit-key");
    let (s, lamp, _fan, head) = divided(&r, "key");
    let first = propose(
        &r,
        &s,
        Proposing {
            spec: &lamp,
            key: "k",
            blocks: Some(&["b1", "b2", "b3"]),
            ledger: &head,
        },
    )
    .unwrap();

    let again = propose(
        &r,
        &s,
        Proposing {
            spec: &lamp,
            key: "k",
            blocks: Some(&["b1", "b2", "b3"]),
            ledger: &head,
        },
    )
    .unwrap();

    assert!(!again.created);
    assert_eq!(again.proposal.id, first.proposal.id);
    // The same key given as an import of its own (no division) is another input.
    let conversion = conversion_of(&s.extraction, &["b1", "b2", "b3"]);
    let scope = scope_of(&s.extraction, &["b1", "b2", "b3"]);
    let report = json!({
        "extraction": s.extraction.as_str(),
        "conversion": Revision::of(conversion.as_bytes()).as_str(),
        "complete": true, "queue": [],
        "scope": Revision::of(scope.as_bytes()).as_str(),
    })
    .to_string();
    let alone = r.store.propose_import(
        &lamp,
        ProposalRegistration {
            key: "k",
            origin: "gui",
            original: s.original.clone(),
            extraction: s.extraction.clone(),
            conversion: &conversion,
            report: &report,
            scope: Some(&scope),
            intake: None,
            expect: r.store.head(&lamp).unwrap(),
            supersede: false,
        },
    );
    assert_eq!(refusal(alone.unwrap_err()).0, "key-reused");
}

#[test]
fn a_spec_that_was_removed_is_said_to_be_missing_and_the_intake_still_reads() {
    let r = rig("removed");
    let (s, lamp, _fan, _head) = divided(&r, "removed");
    r.store.remove_spec(&lamp).unwrap();

    let view = r.store.read_intake(&s.id).unwrap();

    assert!(view.destinations[0].spec_missing);
    assert_eq!(view.destinations[0].spec.as_deref(), Some(lamp.as_str()));
    assert!(view.destinations[0].proposal.is_none());
}

#[test]
fn a_proposal_carried_forward_keeps_the_division_it_was_made_under() {
    let r = rig("rebase");
    let (s, lamp, _fan, head) = divided(&r, "rebase");
    r.store.save_source(&lamp, "Before.", None).unwrap();
    let made = propose(
        &r,
        &s,
        Proposing {
            spec: &lamp,
            key: "k",
            blocks: Some(&["b1", "b2", "b3"]),
            ledger: &head,
        },
    )
    .unwrap()
    .proposal;
    // The spec's text moved after the proposal was made, so it is carried forward onto the new text.
    r.store
        .save_source(&lamp, "After.", Some(&Revision::of(b"Before.")))
        .unwrap();

    let carried = r.store.rebase_proposal(&lamp, &made.id).unwrap().proposal;

    assert_eq!(carried.intake, made.intake);
    assert!(carried.intake.is_some());
}

// ------------------------------------------------------------------------------ the command layer

fn run(r: &Rig, name: &str, args: Value) -> Result<Value, sce_specs::CommandError> {
    call(&r.store, &FakeRenderer, name, args)
}

#[test]
fn the_commands_start_save_and_read_an_intake_and_say_their_refusals_by_word() {
    let r = rig("commands");
    let text = extraction_text();
    let members =
        json!([{"path": "spec.md", "role": "document", "data": "VGhlIGxhbXAuIFRoZSBmYW4u"}]);

    let started = run(
        &r,
        "start_intake",
        json!({
            "key": "k", "origin": "gui", "title": "Controller spec", "members": members,
            "member": "spec.md", "extraction": text,
        }),
    )
    .unwrap();
    let id = started["intake"]["id"].as_str().unwrap().to_string();
    let extraction = Revision::of(text.as_bytes());
    let ledger = ledger_of(
        &extraction,
        json!([{"to": {"kind": "left-out"}, "reason": "", "blocks": ["b1"]}]),
    );
    let report = report_of(&extraction, &ledger, false);
    let saved = run(
        &r,
        "save_ledger",
        json!({"id": id, "ledger": ledger, "report": report}),
    )
    .unwrap();
    let read = run(&r, "read_intake", json!({"id": id})).unwrap();
    let listed = run(&r, "list_intakes", json!({})).unwrap();
    let stale = run(
        &r,
        "save_ledger",
        json!({"id": id, "ledger": ledger, "report": report}),
    )
    .unwrap_err();
    let bad = run(
        &r,
        "save_ledger",
        json!({"id": id, "ledger": "[", "report": report, "base": saved["entry"]["digest"]}),
    )
    .unwrap_err();
    let extra = run(&r, "read_intake", json!({"id": id, "colour": "red"})).unwrap_err();

    assert_eq!(started["created"], true);
    assert_eq!(saved["entry"]["n"], 1);
    assert_eq!(read["ledger"]["text"], json!(ledger));
    assert_eq!(read["destinations"][0]["to"]["kind"], "left-out");
    assert_eq!(listed["intakes"][0]["id"], json!(id));
    assert_eq!(stale.kind, "conflict");
    assert_eq!(bad.kind, "bad-ledger");
    assert_eq!(extra.kind, "bad-request");
    let described = run(&r, "describe", json!({})).unwrap();
    let commands: Vec<&str> = described["commands"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c.as_str().unwrap())
        .collect();
    for name in [
        "start_intake",
        "read_intake",
        "list_intakes",
        "save_ledger",
        "bind_new_spec",
        "give_intake_to_spec",
    ] {
        assert!(commands.contains(&name), "{name}");
    }
}

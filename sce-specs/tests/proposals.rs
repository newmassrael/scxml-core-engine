// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Import proposals in a spec's folder (ADR 0025, 0037).
//!
//! The life of a proposal is the machine's and is held where it is (`import_life`'s own tests). What
//! is held here is what the store does around it: what it refuses to keep, what accepting does to the
//! spec's text and what it does not, that the decisions are a log, and that a proposal whose text
//! moved is carried forward and not redone. Each refusal is paired with the nearest input that must
//! not be refused, so that a store that refuses everything fails the second half.

mod common;

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use sce_specs::{
    call, DecisionKind, ManualClock, MemberDraft, OriginalDraft, ProposalRegistration,
    ProposalState, Revision, Role, SpecId, SpecStore, StoreError,
};
use serde_json::{json, Value};

use common::{scratch, FakeRenderer};

const T0: u64 = 1_791_190_800;

struct Fixture {
    root: PathBuf,
    store: SpecStore<Arc<ManualClock>>,
    spec: SpecId,
    original: Revision,
    extraction_text: String,
    extraction: Revision,
}

/// What the authoring tools write: an extraction of three blocks, which is kept as it is given.
fn extraction_text() -> String {
    json!({
        "v": 1, "extractor": "plain", "extractor_version": "1", "member": "spec.md",
        "blocks": [{"id": "b1"}, {"id": "b2"}, {"id": "b3"}], "not_carried": [],
    })
    .to_string()
}

fn fixture(label: &str) -> Fixture {
    let root = scratch(label);
    let store = SpecStore::with_clock(&root, Arc::new(ManualClock::at(T0)));
    let spec = store.create_spec("Door lock").unwrap().id;
    let original = store
        .add_original(
            &spec,
            OriginalDraft {
                label: "from the customer".to_string(),
                members: vec![MemberDraft::new(
                    "spec.md",
                    Role::Document,
                    b"The lock.".to_vec(),
                )],
                missing: vec![],
            },
        )
        .unwrap()
        .digest;
    let extraction_text = extraction_text();
    let extraction = store
        .save_extraction(&spec, &original, "spec.md", &extraction_text)
        .unwrap()
        .digest;
    Fixture {
        root,
        store,
        spec,
        original,
        extraction_text,
        extraction,
    }
}

/// A conversion of the fixture's extraction, as the authoring tools write one.
fn conversion_of(extraction: &Revision, blocks: &[&str]) -> String {
    json!({
        "v": 1,
        "converter": {"kind": "model", "name": "a-model", "version": "1"},
        "extraction": extraction.as_str(),
        "blocks": blocks
            .iter()
            .map(|id| json!({"id": id, "markdown": "x", "cites": ["b1"]}))
            .collect::<Vec<_>>(),
    })
    .to_string()
}

/// The check of a conversion, as `check-conversion --json` writes it, asking about `asks`.
fn report_of(extraction: &Revision, conversion: &str, asks: &[(&str, &str)]) -> String {
    json!({
        "extraction": extraction.as_str(),
        "conversion": Revision::of(conversion.as_bytes()).as_str(),
        "complete": asks.is_empty(),
        "queue": asks.iter().map(|(why, id)| json!({"why": why, "id": id})).collect::<Vec<_>>(),
    })
    .to_string()
}

struct Made {
    conversion: String,
    report: String,
}

impl Fixture {
    fn made(&self, asks: &[(&str, &str)]) -> Made {
        let conversion = conversion_of(&self.extraction, &["m1", "m2"]);
        let report = report_of(&self.extraction, &conversion, asks);
        Made { conversion, report }
    }

    fn registration<'a>(&self, key: &'a str, made: &'a Made) -> ProposalRegistration<'a> {
        ProposalRegistration {
            key,
            origin: "gui",
            original: self.original.clone(),
            extraction: self.extraction.clone(),
            conversion: &made.conversion,
            report: &made.report,
            scope: None,
            expect: self.store.head(&self.spec).unwrap(),
            supersede: false,
        }
    }

    fn propose(&self, key: &str, asks: &[(&str, &str)]) -> sce_specs::Proposal {
        let made = self.made(asks);
        self.store
            .propose_import(&self.spec, self.registration(key, &made))
            .unwrap()
            .proposal
    }

    fn decide(&self, proposal: &str, block: &str, what: DecisionKind) {
        self.store
            .decide_block(&self.spec, proposal, block, what, None)
            .unwrap();
    }

    fn dir(&self) -> PathBuf {
        self.root.join(self.spec.as_str())
    }
}

fn refusal(error: StoreError) -> (&'static str, Value) {
    match error {
        StoreError::Refused { kind, detail, .. } => (kind, detail),
        other => panic!("expected a refusal, got {other:?}"),
    }
}

fn source_text(f: &Fixture) -> Option<String> {
    f.store.read_source(&f.spec, None).unwrap().map(|s| s.text)
}

// ------------------------------------------------------------------------------ making a proposal

#[test]
fn a_proposal_for_a_spec_with_no_text_is_pending_against_nothing_and_says_what_it_asks() {
    let f = fixture("prop-new");

    let made = f.made(&[("check", "m1"), ("missing", "b3")]);
    let registered = f
        .store
        .propose_import(&f.spec, f.registration("press-1", &made))
        .unwrap();

    let view = f
        .store
        .read_proposal(&f.spec, &registered.proposal.id)
        .unwrap();
    assert!(registered.created);
    assert_eq!(view.proposal.state, ProposalState::Pending);
    assert_eq!(view.proposal.against, None);
    assert!(view.head_matches);
    assert_eq!(view.proposal.converter.name, "a-model");
    assert_eq!(view.proposal.original, f.original);
    assert_eq!(
        view.undecided
            .iter()
            .map(|a| (a.why.as_str(), a.id.as_str()))
            .collect::<Vec<_>>(),
        [("check", "m1"), ("missing", "b3")]
    );
    assert!(view.decisions.is_empty());
    assert!(registered.proposal.id.starts_with("imp-"));
    assert_eq!(registered.proposal.seq, 1);
}

#[test]
fn a_proposal_is_made_against_the_text_head_there_is() {
    let f = fixture("prop-against");
    let head = match f
        .store
        .save_source(&f.spec, "The lock opens.", None)
        .unwrap()
    {
        sce_specs::Saved::Saved { revision, .. } | sce_specs::Saved::Unchanged { revision } => {
            revision
        }
    };

    let proposal = f.propose("k", &[]);

    assert_eq!(proposal.against, Some(head));
}

#[test]
fn what_the_proposal_is_made_of_is_kept_and_read_back_as_it_was_given() {
    let f = fixture("prop-documents");
    let made = f.made(&[("check", "m1")]);
    let proposal = f
        .store
        .propose_import(&f.spec, f.registration("k", &made))
        .unwrap()
        .proposal;

    let documents = f
        .store
        .read_proposal_documents(&f.spec, &proposal.id)
        .unwrap();

    assert_eq!(documents.conversion, made.conversion);
    assert_eq!(documents.report, made.report);
}

#[test]
fn the_same_press_again_is_the_same_proposal_and_another_input_under_its_key_is_refused() {
    let f = fixture("prop-key");
    let made = f.made(&[]);
    let first = f
        .store
        .propose_import(&f.spec, f.registration("k", &made))
        .unwrap();

    let again = f
        .store
        .propose_import(&f.spec, f.registration("k", &made))
        .unwrap();
    let other_conversion = conversion_of(&f.extraction, &["m1", "m2", "m3"]);
    let other = Made {
        report: report_of(&f.extraction, &other_conversion, &[]),
        conversion: other_conversion,
    };
    let reused = f
        .store
        .propose_import(&f.spec, f.registration("k", &other))
        .unwrap_err();

    assert!(first.created && !again.created);
    assert_eq!(first.proposal.id, again.proposal.id);
    assert_eq!(refusal(reused).0, "key-reused");
    assert_eq!(f.store.list_proposals(&f.spec).unwrap().len(), 1);
}

#[test]
fn a_key_and_an_origin_that_cannot_be_kept_are_refused() {
    let f = fixture("prop-names");
    let made = f.made(&[]);
    for key in ["".to_string(), "k".repeat(201)] {
        let error = f
            .store
            .propose_import(&f.spec, f.registration(&key, &made))
            .unwrap_err();
        assert_eq!(refusal(error).0, "bad-proposal");
    }
    for origin in ["", "a b", "../x", &"o".repeat(65)] {
        let mut registration = f.registration("k", &made);
        registration.origin = origin;
        let error = f.store.propose_import(&f.spec, registration).unwrap_err();
        assert_eq!(refusal(error).0, "bad-proposal", "{origin:?}");
    }
    let longest_key = "k".repeat(200);
    let mut longest = f.registration(&longest_key, &made);
    longest.origin = "client:name_1.2";
    f.store.propose_import(&f.spec, longest).unwrap();
}

#[test]
fn a_proposal_whose_parts_do_not_name_each_other_is_refused_and_nothing_is_kept() {
    let f = fixture("prop-mismatch");
    let made = f.made(&[]);
    let stranger = Revision::of(b"another extraction");
    let other_conversion = conversion_of(&stranger, &["m1"]);
    let cases: Vec<(&str, Made)> = vec![
        (
            "a conversion of another extraction",
            Made {
                report: report_of(&f.extraction, &other_conversion, &[]),
                conversion: other_conversion,
            },
        ),
        (
            "a report of another conversion",
            Made {
                conversion: made.conversion.clone(),
                report: report_of(&f.extraction, "not this conversion", &[]),
            },
        ),
        (
            "a report of another extraction",
            Made {
                conversion: made.conversion.clone(),
                report: report_of(&stranger, &made.conversion, &[]),
            },
        ),
        (
            "a question about a block nobody has",
            Made {
                conversion: made.conversion.clone(),
                report: report_of(&f.extraction, &made.conversion, &[("check", "m99")]),
            },
        ),
    ];
    for (what, case) in cases {
        let error = f
            .store
            .propose_import(&f.spec, f.registration("k", &case))
            .expect_err(what);
        assert_eq!(refusal(error).0, "bad-proposal", "{what}");
    }
    assert!(f.store.list_proposals(&f.spec).unwrap().is_empty());
    assert!(!f.dir().join("conversions").exists());
}

#[test]
fn questions_about_blocks_of_the_conversion_and_of_the_original_are_both_kept() {
    let f = fixture("prop-asks");

    let proposal = f.propose(
        "k",
        &[("check", "m2"), ("missing", "b2"), ("recognised", "b1")],
    );

    assert_eq!(proposal.asks.len(), 3);
}

#[test]
fn an_original_or_an_extraction_the_spec_does_not_hold_is_refused() {
    let f = fixture("prop-unknown");
    let made = f.made(&[]);
    let mut no_original = f.registration("k1", &made);
    no_original.original = Revision::of(b"no such original");
    let mut no_extraction = f.registration("k2", &made);
    no_extraction.extraction = Revision::of(b"no such extraction");
    // An extraction another original of the spec saved is not this original's.
    let second = f
        .store
        .add_original(
            &f.spec,
            OriginalDraft {
                members: vec![MemberDraft::new("b.md", Role::Document, b"b".to_vec())],
                ..OriginalDraft::default()
            },
        )
        .unwrap()
        .digest;
    let theirs = f
        .store
        .save_extraction(
            &f.spec,
            &second,
            "b.md",
            &json!({"v":1,"extractor":"e","extractor_version":"1","blocks":[{"id":"b1"}]})
                .to_string(),
        )
        .unwrap()
        .digest;
    let mut crossed = f.registration("k3", &made);
    crossed.extraction = theirs;

    assert!(matches!(
        f.store.propose_import(&f.spec, no_original).unwrap_err(),
        StoreError::NotFound { .. }
    ));
    assert_eq!(
        refusal(f.store.propose_import(&f.spec, no_extraction).unwrap_err()).0,
        "bad-proposal"
    );
    assert_eq!(
        refusal(f.store.propose_import(&f.spec, crossed).unwrap_err()).0,
        "bad-proposal"
    );
}

#[test]
fn a_text_that_changed_after_it_was_read_is_moved_and_nothing_is_registered() {
    let f = fixture("prop-expect-moved");
    let made = f.made(&[]);
    let stale = f.registration("k", &made);
    f.store
        .save_source(&f.spec, "The text moved.", None)
        .unwrap();

    let error = f.store.propose_import(&f.spec, stale).unwrap_err();

    let (kind, detail) = refusal(error);
    assert_eq!(kind, "moved");
    assert_eq!(detail["moved"], json!(["source"]));
    assert!(f.store.list_proposals(&f.spec).unwrap().is_empty());
}

#[test]
fn a_spec_has_one_open_proposal_and_a_second_replaces_it_only_when_it_says_so() {
    let f = fixture("prop-active");
    let first = f.propose("first", &[("check", "m1")]);
    let made = f.made(&[]);

    let refused = f
        .store
        .propose_import(&f.spec, f.registration("second", &made))
        .unwrap_err();
    let (kind, detail) = refusal(refused);
    assert_eq!(kind, "active-proposal");
    assert_eq!(detail["proposal"], json!(first.id));
    assert_eq!(
        f.store
            .read_proposal(&f.spec, &first.id)
            .unwrap()
            .proposal
            .state,
        ProposalState::Pending
    );

    let mut replacing = f.registration("second", &made);
    replacing.supersede = true;
    let second = f.store.propose_import(&f.spec, replacing).unwrap().proposal;

    let old = f.store.read_proposal(&f.spec, &first.id).unwrap().proposal;
    assert_eq!(old.state, ProposalState::Superseded);
    assert_eq!(old.note.as_deref(), Some("a newer proposal replaced it"));
    assert!(old.ended_at.is_some());
    assert_eq!(second.seq, 2);
    assert_eq!(
        f.store
            .read_proposal(&f.spec, &second.id)
            .unwrap()
            .proposal
            .state,
        ProposalState::Pending
    );
}

#[test]
fn a_proposal_that_ended_does_not_stand_in_the_way_of_the_next() {
    let f = fixture("prop-after-end");
    let first = f.propose("first", &[]);
    f.store.reject_proposal(&f.spec, &first.id, None).unwrap();

    let second = f.propose("second", &[]);

    assert_eq!(second.seq, 2);
}

#[test]
fn proposals_are_listed_newest_first_and_an_id_that_is_not_one_is_not_found() {
    let f = fixture("prop-list");
    let first = f.propose("first", &[]);
    f.store.reject_proposal(&f.spec, &first.id, None).unwrap();
    let second = f.propose("second", &[]);

    let ids: Vec<String> = f
        .store
        .list_proposals(&f.spec)
        .unwrap()
        .into_iter()
        .map(|p| p.id)
        .collect();

    assert_eq!(ids, [second.id, first.id]);
    for bad in ["imp-nothere0000", "../etc", "", "imp-", "req-0123456789ab"] {
        assert!(
            matches!(
                f.store.read_proposal(&f.spec, bad).unwrap_err(),
                StoreError::NotFound { .. }
            ),
            "{bad:?}"
        );
    }
}

#[test]
fn a_conversion_or_a_report_past_the_limit_is_refused_and_one_at_it_is_kept() {
    let f = fixture("prop-limits");
    let made = f.made(&[]);
    let mut big = Made {
        conversion: made.conversion.clone(),
        report: made.report.clone(),
    };
    big.conversion
        .push_str(&" ".repeat(sce_specs::proposals::MAX_CONVERSION_BYTES));

    let error = f
        .store
        .propose_import(&f.spec, f.registration("k", &big))
        .unwrap_err();

    assert!(matches!(error, StoreError::TooLarge { .. }), "{error:?}");
    f.store
        .propose_import(&f.spec, f.registration("k", &made))
        .unwrap();
}

#[test]
fn a_removed_spec_takes_no_proposal_and_gives_none() {
    let f = fixture("prop-removed");
    let proposal = f.propose("k", &[]);
    let made = f.made(&[]);
    // Made while the spec still has a head to expect; the removal is what the store must refuse over.
    let registration = f.registration("k2", &made);
    f.store.remove_spec(&f.spec).unwrap();

    for error in [
        f.store
            .propose_import(&f.spec, registration)
            .map(|_| ())
            .unwrap_err(),
        f.store
            .read_proposal(&f.spec, &proposal.id)
            .map(|_| ())
            .unwrap_err(),
        f.store.list_proposals(&f.spec).map(|_| ()).unwrap_err(),
        f.store
            .accept_proposal(&f.spec, &proposal.id, "x", 0)
            .map(|_| ())
            .unwrap_err(),
        f.store
            .reject_proposal(&f.spec, &proposal.id, None)
            .map(|_| ())
            .unwrap_err(),
    ] {
        assert!(matches!(error, StoreError::NotFound { .. }), "{error:?}");
    }
}

#[test]
fn a_conversion_whose_file_was_changed_is_corrupt_and_is_not_given_back() {
    let f = fixture("prop-tampered");
    let made = f.made(&[]);
    let proposal = f
        .store
        .propose_import(&f.spec, f.registration("k", &made))
        .unwrap()
        .proposal;
    let path = f
        .dir()
        .join("conversions")
        .join(format!("{}.json", proposal.conversion.as_str()));
    fs::write(&path, "{}").unwrap();

    let error = f
        .store
        .read_proposal_documents(&f.spec, &proposal.id)
        .unwrap_err();

    assert!(matches!(error, StoreError::Corrupt { .. }), "{error:?}");
}

// --------------------------------------------------------------------------------------- decisions

#[test]
fn a_decision_is_a_numbered_line_of_a_log_and_the_log_only_grows() {
    let f = fixture("dec-log");
    let proposal = f.propose("k", &[("check", "m1"), ("check", "m2")]);

    let first = f
        .store
        .decide_block(&f.spec, &proposal.id, "m1", DecisionKind::Keep, None)
        .unwrap();
    let second = f
        .store
        .decide_block(
            &f.spec,
            &proposal.id,
            "m2",
            DecisionKind::Correct,
            Some("the owner's words"),
        )
        .unwrap();
    let again = f
        .store
        .decide_block(&f.spec, &proposal.id, "m1", DecisionKind::Drop, None)
        .unwrap();

    let view = f.store.read_proposal(&f.spec, &proposal.id).unwrap();
    assert_eq!((first.n, second.n, again.n), (1, 2, 3));
    assert_eq!(view.decisions.len(), 3);
    assert_eq!(view.decisions[1].text.as_deref(), Some("the owner's words"));
    assert_eq!(view.decisions[2].what, DecisionKind::Drop);
    assert!(view.undecided.is_empty());
    assert!(first.at.ends_with('Z'));
}

#[test]
fn a_question_is_answered_by_a_decision_on_its_block_and_by_no_other() {
    let f = fixture("dec-answers");
    let proposal = f.propose("k", &[("check", "m1"), ("missing", "b2")]);

    f.decide(&proposal.id, "m2", DecisionKind::Keep);
    let open = f
        .store
        .read_proposal(&f.spec, &proposal.id)
        .unwrap()
        .undecided;
    assert_eq!(
        open.iter().map(|a| a.id.as_str()).collect::<Vec<_>>(),
        ["m1", "b2"]
    );

    f.decide(&proposal.id, "b2", DecisionKind::Add);
    let open = f
        .store
        .read_proposal(&f.spec, &proposal.id)
        .unwrap()
        .undecided;
    assert_eq!(
        open.iter().map(|a| a.id.as_str()).collect::<Vec<_>>(),
        ["m1"]
    );
}

#[test]
fn only_a_correction_carries_words_and_a_correction_must() {
    let f = fixture("dec-words");
    let proposal = f.propose("k", &[("check", "m1")]);
    for (what, text) in [
        (DecisionKind::Correct, None),
        (DecisionKind::Correct, Some("")),
        (DecisionKind::Correct, Some("   ")),
        (DecisionKind::Keep, Some("words")),
        (DecisionKind::Drop, Some("words")),
        (DecisionKind::Add, Some("words")),
    ] {
        let error = f
            .store
            .decide_block(&f.spec, &proposal.id, "m1", what, text)
            .unwrap_err();
        assert_eq!(refusal(error).0, "bad-proposal", "{what:?} {text:?}");
    }
    assert!(f
        .store
        .read_proposal(&f.spec, &proposal.id)
        .unwrap()
        .decisions
        .is_empty());
    f.store
        .decide_block(
            &f.spec,
            &proposal.id,
            "m1",
            DecisionKind::Correct,
            Some("words"),
        )
        .unwrap();
}

#[test]
fn a_decision_names_a_block_that_is_there_and_add_names_one_of_the_original() {
    let f = fixture("dec-blocks");
    let proposal = f.propose("k", &[("check", "m1")]);

    for (block, what) in [
        ("m99", DecisionKind::Keep),
        ("b99", DecisionKind::Add),
        ("", DecisionKind::Drop),
        // `add` puts the original's block into the spec, so a block only the conversion has is not one.
        ("m1", DecisionKind::Add),
    ] {
        let error = f
            .store
            .decide_block(&f.spec, &proposal.id, block, what, None)
            .unwrap_err();
        assert_eq!(refusal(error).0, "bad-proposal", "{block:?} {what:?}");
    }
    f.decide(&proposal.id, "b3", DecisionKind::Add);
    f.decide(&proposal.id, "m2", DecisionKind::Drop);
}

#[test]
fn what_may_be_said_of_a_block_depends_on_whose_it_is() {
    let f = fixture("dec-whose");
    let proposal = f.propose("k", &[("check", "m1"), ("missing", "b3")]);

    // A block of the original the conversion left out has nothing in it to keep or to correct.
    for what in [DecisionKind::Keep, DecisionKind::Correct] {
        let words = (what == DecisionKind::Correct).then_some("words");
        let error = f
            .store
            .decide_block(&f.spec, &proposal.id, "b3", what, words)
            .unwrap_err();
        let (kind, detail) = refusal(error);
        assert_eq!(kind, "bad-proposal", "{what:?}");
        assert_eq!(detail["block"], "b3");
    }
    // It is added, or it is left out, and both answer the question that was asked about it.
    f.decide(&proposal.id, "b3", DecisionKind::Drop);
    assert!(f
        .store
        .read_proposal(&f.spec, &proposal.id)
        .unwrap()
        .undecided
        .iter()
        .all(|ask| ask.id != "b3"));
    f.decide(&proposal.id, "b3", DecisionKind::Add);
    // A block of the conversion is kept, corrected or dropped.
    f.decide(&proposal.id, "m1", DecisionKind::Keep);
    f.store
        .decide_block(
            &f.spec,
            &proposal.id,
            "m1",
            DecisionKind::Correct,
            Some("words"),
        )
        .unwrap();
    f.decide(&proposal.id, "m1", DecisionKind::Drop);
}

#[test]
fn nothing_is_decided_about_a_proposal_that_ended() {
    let f = fixture("dec-ended");
    let proposal = f.propose("k", &[("check", "m1")]);
    f.store
        .reject_proposal(&f.spec, &proposal.id, None)
        .unwrap();

    let error = f
        .store
        .decide_block(&f.spec, &proposal.id, "m1", DecisionKind::Keep, None)
        .unwrap_err();

    let (kind, detail) = refusal(error);
    assert_eq!(kind, "proposal-ended");
    assert_eq!(detail["state"], "rejected");
    assert!(f
        .store
        .read_proposal(&f.spec, &proposal.id)
        .unwrap()
        .decisions
        .is_empty());
}

#[test]
fn a_decision_past_the_limit_of_words_is_refused() {
    let f = fixture("dec-big");
    let proposal = f.propose("k", &[("check", "m1")]);
    let too_long = "x".repeat(sce_specs::proposals::MAX_DECISION_TEXT_BYTES + 1);

    let error = f
        .store
        .decide_block(
            &f.spec,
            &proposal.id,
            "m1",
            DecisionKind::Correct,
            Some(&too_long),
        )
        .unwrap_err();

    assert!(matches!(error, StoreError::TooLarge { .. }), "{error:?}");
}

#[test]
fn a_torn_end_of_the_log_is_not_a_decision_and_the_next_is_not_glued_to_it() {
    let f = fixture("dec-torn");
    let proposal = f.propose("k", &[("check", "m1")]);
    f.decide(&proposal.id, "m1", DecisionKind::Keep);
    let log = f
        .dir()
        .join("decisions")
        .join(format!("{}.log", proposal.id));
    let mut text = fs::read_to_string(&log).unwrap();
    text.push_str("{\"n\":2,\"at\":\"half a li");
    fs::write(&log, text).unwrap();

    assert_eq!(
        f.store
            .read_proposal(&f.spec, &proposal.id)
            .unwrap()
            .decisions
            .len(),
        1
    );
    let next = f
        .store
        .decide_block(&f.spec, &proposal.id, "m2", DecisionKind::Keep, None)
        .unwrap();

    assert_eq!(next.n, 2);
    assert_eq!(
        f.store
            .read_proposal(&f.spec, &proposal.id)
            .unwrap()
            .decisions
            .len(),
        2
    );
}

// ---------------------------------------------------------------------------------------- accepting

#[test]
fn accepting_a_proposal_with_nothing_asked_makes_its_text_the_specs_in_one_step() {
    let f = fixture("acc-simple");
    let proposal = f.propose("k", &[]);

    let accepted = f
        .store
        .accept_proposal(&f.spec, &proposal.id, "# Door lock\n", 0)
        .unwrap();

    assert!(accepted.created);
    assert_eq!(accepted.proposal.state, ProposalState::Accepted);
    assert_eq!(accepted.saved, Revision::of(b"# Door lock\n"));
    assert_eq!(accepted.proposal.saved, Some(accepted.saved.clone()));
    assert!(accepted.proposal.ended_at.is_some());
    assert_eq!(source_text(&f).as_deref(), Some("# Door lock\n"));
    assert_eq!(f.store.head(&f.spec).unwrap(), Some(accepted.saved));
    let read = f.store.read_proposal(&f.spec, &proposal.id).unwrap();
    assert_eq!(read.proposal.state, ProposalState::Accepted);
}

#[test]
fn accepting_replaces_the_text_the_spec_had_and_keeps_it_in_the_history() {
    let f = fixture("acc-history");
    f.store.save_source(&f.spec, "The old text.", None).unwrap();
    let proposal = f.propose("k", &[]);

    f.store
        .accept_proposal(&f.spec, &proposal.id, "The imported text.", 0)
        .unwrap();

    let history = f.store.history(&f.spec).unwrap();
    assert_eq!(history.len(), 2);
    assert_eq!(source_text(&f).as_deref(), Some("The imported text."));
    assert!(f
        .store
        .read_source(&f.spec, Some(&Revision::of(b"The old text.")))
        .unwrap()
        .is_some());
}

#[test]
fn accepting_is_refused_while_a_question_has_no_decision_and_writes_nothing() {
    let f = fixture("acc-undecided");
    f.store.save_source(&f.spec, "The old text.", None).unwrap();
    let proposal = f.propose("k", &[("check", "m1"), ("missing", "b2")]);
    f.decide(&proposal.id, "m1", DecisionKind::Keep);

    let error = f
        .store
        .accept_proposal(&f.spec, &proposal.id, "New.", 1)
        .unwrap_err();

    let (kind, detail) = refusal(error);
    assert_eq!(kind, "undecided");
    assert_eq!(detail["blocks"], json!(["b2"]));
    assert_eq!(source_text(&f).as_deref(), Some("The old text."));
    assert_eq!(
        f.store
            .read_proposal(&f.spec, &proposal.id)
            .unwrap()
            .proposal
            .state,
        ProposalState::Pending
    );

    f.decide(&proposal.id, "b2", DecisionKind::Drop);
    f.store
        .accept_proposal(&f.spec, &proposal.id, "New.", 2)
        .unwrap();
}

#[test]
fn accepting_after_the_text_moved_is_refused_as_moved_naming_both_revisions_and_the_proposal_stays()
{
    let f = fixture("acc-moved");
    let before = match f
        .store
        .save_source(&f.spec, "The text it was made against.", None)
        .unwrap()
    {
        sce_specs::Saved::Saved { revision, .. } | sce_specs::Saved::Unchanged { revision } => {
            revision
        }
    };
    let proposal = f.propose("k", &[]);
    let after = match f
        .store
        .save_source(&f.spec, "Somebody else's text.", Some(&before))
        .unwrap()
    {
        sce_specs::Saved::Saved { revision, .. } | sce_specs::Saved::Unchanged { revision } => {
            revision
        }
    };

    let error = f
        .store
        .accept_proposal(&f.spec, &proposal.id, "Imported.", 0)
        .unwrap_err();

    let (kind, detail) = refusal(error);
    assert_eq!(kind, "moved");
    assert_eq!(detail["against"], json!(before));
    assert_eq!(detail["current"]["source"], json!(after));
    assert_eq!(detail["moved"], json!(["source"]));
    assert_eq!(source_text(&f).as_deref(), Some("Somebody else's text."));
    let view = f.store.read_proposal(&f.spec, &proposal.id).unwrap();
    assert_eq!(view.proposal.state, ProposalState::Pending);
    assert!(!view.head_matches);
    assert_eq!(view.head, Some(after));
}

#[test]
fn a_spec_that_had_no_text_and_got_one_is_moved_too() {
    let f = fixture("acc-moved-from-nothing");
    let proposal = f.propose("k", &[]);
    f.store
        .save_source(&f.spec, "A first text, typed meanwhile.", None)
        .unwrap();

    let error = f
        .store
        .accept_proposal(&f.spec, &proposal.id, "Imported.", 0)
        .unwrap_err();

    assert_eq!(refusal(error).0, "moved");
    assert_eq!(
        source_text(&f).as_deref(),
        Some("A first text, typed meanwhile.")
    );
}

#[test]
fn a_moved_head_is_said_before_a_question_that_is_still_open() {
    let f = fixture("acc-moved-first");
    f.store.save_source(&f.spec, "Before.", None).unwrap();
    let proposal = f.propose("k", &[("check", "m1")]);
    f.store
        .save_source(&f.spec, "After.", Some(&Revision::of(b"Before.")))
        .unwrap();

    let error = f
        .store
        .accept_proposal(&f.spec, &proposal.id, "Imported.", 0)
        .unwrap_err();

    assert_eq!(refusal(error).0, "moved");
}

#[test]
fn a_text_assembled_before_one_more_decision_is_not_saved_as_if_it_answered_the_log() {
    let f = fixture("acc-decisions-moved");
    let proposal = f.propose("k", &[("check", "m1")]);
    f.decide(&proposal.id, "m1", DecisionKind::Keep);

    // Assembled when the log held one decision; the owner then decided another block.
    f.decide(&proposal.id, "m2", DecisionKind::Drop);
    let error = f
        .store
        .accept_proposal(&f.spec, &proposal.id, "Assembled early.", 1)
        .unwrap_err();

    let (kind, detail) = refusal(error);
    assert_eq!(kind, "decisions-moved");
    assert_eq!(detail["assembled_from"], 1);
    assert_eq!(detail["decisions"], 2);
    assert_eq!(source_text(&f), None);
    f.store
        .accept_proposal(&f.spec, &proposal.id, "Assembled late.", 2)
        .unwrap();
    assert_eq!(source_text(&f).as_deref(), Some("Assembled late."));
}

#[test]
fn the_same_accept_again_is_the_same_acceptance_and_writes_nothing() {
    let f = fixture("acc-again");
    let proposal = f.propose("k", &[]);
    let first = f
        .store
        .accept_proposal(&f.spec, &proposal.id, "Imported.", 0)
        .unwrap();
    // The spec moved on after it was accepted.
    f.store
        .save_source(&f.spec, "Edited later.", Some(&first.saved))
        .unwrap();

    let again = f
        .store
        .accept_proposal(&f.spec, &proposal.id, "Imported.", 0)
        .unwrap();

    assert!(!again.created);
    assert_eq!(again.saved, first.saved);
    assert_eq!(source_text(&f).as_deref(), Some("Edited later."));
    assert_eq!(f.store.history(&f.spec).unwrap().len(), 2);
}

#[test]
fn what_ended_otherwise_takes_no_accept() {
    let f = fixture("acc-ended");
    let rejected = f.propose("one", &[]);
    f.store
        .reject_proposal(&f.spec, &rejected.id, None)
        .unwrap();
    let to_be_replaced = f.propose("two", &[]);
    let made = f.made(&[]);
    let mut replacing = f.registration("three", &made);
    replacing.supersede = true;
    f.store.propose_import(&f.spec, replacing).unwrap();

    for id in [&rejected.id, &to_be_replaced.id] {
        let error = f.store.accept_proposal(&f.spec, id, "x", 0).unwrap_err();
        assert_eq!(refusal(error).0, "proposal-ended", "{id}");
    }
    assert_eq!(source_text(&f), None);
}

#[test]
fn a_text_past_the_limit_is_refused_before_anything_is_looked_at() {
    let f = fixture("acc-big");
    let proposal = f.propose("k", &[]);
    let big = "x".repeat(sce_specs::MAX_SOURCE_BYTES + 1);

    let error = f
        .store
        .accept_proposal(&f.spec, &proposal.id, &big, 0)
        .unwrap_err();

    assert!(matches!(error, StoreError::TooLarge { .. }), "{error:?}");
    assert_eq!(
        f.store
            .read_proposal(&f.spec, &proposal.id)
            .unwrap()
            .proposal
            .state,
        ProposalState::Pending
    );
}

#[test]
fn accepting_a_text_the_spec_already_has_is_unchanged_and_still_ends_the_proposal() {
    let f = fixture("acc-same-text");
    f.store.save_source(&f.spec, "Same.", None).unwrap();
    let proposal = f.propose("k", &[]);

    let accepted = f
        .store
        .accept_proposal(&f.spec, &proposal.id, "Same.", 0)
        .unwrap();

    assert_eq!(accepted.saved, Revision::of(b"Same."));
    assert_eq!(accepted.proposal.state, ProposalState::Accepted);
    assert_eq!(f.store.history(&f.spec).unwrap().len(), 1);
}

#[test]
fn two_accepts_at_once_cannot_both_move_the_head() {
    // Two proposals of one spec cannot be open together, so the race is between an accept and a
    // save of the text: whichever takes the lock second finds the head the first one moved.
    for round in 0..8 {
        let f = Arc::new(fixture(&format!("acc-race-{round}")));
        f.store.save_source(&f.spec, "Base.", None).unwrap();
        let proposal = f.propose("k", &[]);
        let accepting = {
            let f = Arc::clone(&f);
            let id = proposal.id.clone();
            std::thread::spawn(move || f.store.accept_proposal(&f.spec, &id, "Imported.", 0))
        };
        let saving = {
            let f = Arc::clone(&f);
            std::thread::spawn(move || {
                f.store
                    .save_source(&f.spec, "Typed.", Some(&Revision::of(b"Base.")))
            })
        };

        let accepted = accepting.join().unwrap();
        let saved = saving.join().unwrap();

        // Exactly one of them moved the head from `Base.`, and the other was refused.
        assert!(
            accepted.is_ok() ^ saved.is_ok(),
            "round {round}: {accepted:?} / {saved:?}"
        );
        let text = source_text(&f).unwrap();
        assert_eq!(
            text,
            if accepted.is_ok() {
                "Imported."
            } else {
                "Typed."
            },
            "round {round}"
        );
    }
}

// ----------------------------------------------------------------------------------------- rejecting

#[test]
fn rejecting_ends_a_pending_proposal_and_says_why_and_the_same_word_again_is_the_same() {
    let f = fixture("rej-simple");
    f.store.save_source(&f.spec, "Untouched.", None).unwrap();
    let proposal = f.propose("k", &[("check", "m1")]);

    let rejected = f
        .store
        .reject_proposal(&f.spec, &proposal.id, Some("not what we meant"))
        .unwrap();
    let again = f
        .store
        .reject_proposal(&f.spec, &proposal.id, Some("another note"))
        .unwrap();

    assert_eq!(rejected.state, ProposalState::Rejected);
    assert_eq!(rejected.note.as_deref(), Some("not what we meant"));
    assert_eq!(again, rejected);
    assert_eq!(source_text(&f).as_deref(), Some("Untouched."));
}

#[test]
fn what_was_accepted_or_replaced_is_not_rejected_afterwards() {
    let f = fixture("rej-ended");
    let accepted = f.propose("one", &[]);
    f.store
        .accept_proposal(&f.spec, &accepted.id, "Imported.", 0)
        .unwrap();
    let to_be_replaced = f.propose("two", &[]);
    let made = f.made(&[]);
    let mut replacing = f.registration("three", &made);
    replacing.supersede = true;
    f.store.propose_import(&f.spec, replacing).unwrap();

    for id in [&accepted.id, &to_be_replaced.id] {
        let error = f.store.reject_proposal(&f.spec, id, None).unwrap_err();
        assert_eq!(refusal(error).0, "proposal-ended", "{id}");
    }
}

// ------------------------------------------------------------------------------------------ rebasing

#[test]
fn a_proposal_whose_text_moved_is_carried_forward_with_its_decisions_and_the_old_one_is_let_go_of()
{
    let f = fixture("reb-simple");
    let before = match f.store.save_source(&f.spec, "Before.", None).unwrap() {
        sce_specs::Saved::Saved { revision, .. } | sce_specs::Saved::Unchanged { revision } => {
            revision
        }
    };
    let old = f.propose("k", &[("check", "m1"), ("check", "m2")]);
    f.decide(&old.id, "m2", DecisionKind::Keep);
    f.store
        .decide_block(
            &f.spec,
            &old.id,
            "m1",
            DecisionKind::Correct,
            Some("my words"),
        )
        .unwrap();
    let after = match f
        .store
        .save_source(&f.spec, "After.", Some(&before))
        .unwrap()
    {
        sce_specs::Saved::Saved { revision, .. } | sce_specs::Saved::Unchanged { revision } => {
            revision
        }
    };

    let carried = f.store.rebase_proposal(&f.spec, &old.id).unwrap();

    assert!(carried.created);
    let new = &carried.proposal;
    assert_ne!(new.id, old.id);
    assert_eq!(new.against, Some(after.clone()));
    assert_eq!(new.state, ProposalState::Pending);
    assert_eq!(new.rebased_from.as_deref(), Some(old.id.as_str()));
    assert_eq!(
        (&new.original, &new.extraction, &new.conversion, &new.report),
        (&old.original, &old.extraction, &old.conversion, &old.report)
    );
    assert_eq!(new.asks, old.asks);
    assert_eq!(new.seq, 2);
    let view = f.store.read_proposal(&f.spec, &new.id).unwrap();
    assert!(view.head_matches);
    assert!(view.undecided.is_empty());
    assert_eq!(
        view.decisions
            .iter()
            .map(|d| (d.block.as_str(), d.what))
            .collect::<Vec<_>>(),
        [("m2", DecisionKind::Keep), ("m1", DecisionKind::Correct)]
    );
    assert_eq!(view.decisions[1].text.as_deref(), Some("my words"));
    let let_go = f.store.read_proposal(&f.spec, &old.id).unwrap();
    assert_eq!(let_go.proposal.state, ProposalState::Superseded);
    assert!(let_go.proposal.note.unwrap().contains(&new.id));
    // The old one's log is kept as it was.
    assert_eq!(let_go.decisions.len(), 2);

    // The carried proposal can be accepted, where the old one could not.
    f.store
        .accept_proposal(&f.spec, &new.id, "Imported.", 2)
        .unwrap();
    assert_eq!(source_text(&f).as_deref(), Some("Imported."));
}

#[test]
fn rebasing_the_same_proposal_again_for_the_same_head_is_the_same_proposal() {
    let f = fixture("reb-again");
    f.store.save_source(&f.spec, "Before.", None).unwrap();
    let old = f.propose("k", &[]);
    f.store
        .save_source(&f.spec, "After.", Some(&Revision::of(b"Before.")))
        .unwrap();

    let first = f.store.rebase_proposal(&f.spec, &old.id).unwrap();
    let again = f.store.rebase_proposal(&f.spec, &old.id).unwrap();

    assert!(first.created && !again.created);
    assert_eq!(first.proposal.id, again.proposal.id);
    assert_eq!(f.store.list_proposals(&f.spec).unwrap().len(), 2);
}

#[test]
fn nothing_is_carried_forward_where_the_text_has_not_moved_or_the_proposal_ended() {
    let f = fixture("reb-refused");
    f.store.save_source(&f.spec, "Here.", None).unwrap();
    let proposal = f.propose("k", &[]);

    let error = f.store.rebase_proposal(&f.spec, &proposal.id).unwrap_err();
    assert_eq!(refusal(error).0, "nothing-to-carry");
    assert_eq!(f.store.list_proposals(&f.spec).unwrap().len(), 1);

    f.store
        .reject_proposal(&f.spec, &proposal.id, None)
        .unwrap();
    f.store
        .save_source(&f.spec, "There.", Some(&Revision::of(b"Here.")))
        .unwrap();
    let error = f.store.rebase_proposal(&f.spec, &proposal.id).unwrap_err();
    assert_eq!(refusal(error).0, "proposal-ended");

    assert!(matches!(
        f.store
            .rebase_proposal(&f.spec, "imp-000000000000")
            .unwrap_err(),
        StoreError::NotFound { .. }
    ));
}

#[test]
fn a_proposal_made_against_no_text_is_carried_forward_to_the_text_there_is() {
    let f = fixture("reb-from-nothing");
    let old = f.propose("k", &[("check", "m1")]);
    f.decide(&old.id, "m1", DecisionKind::Keep);
    let typed = match f
        .store
        .save_source(&f.spec, "Typed meanwhile.", None)
        .unwrap()
    {
        sce_specs::Saved::Saved { revision, .. } | sce_specs::Saved::Unchanged { revision } => {
            revision
        }
    };

    let carried = f.store.rebase_proposal(&f.spec, &old.id).unwrap().proposal;

    assert_eq!(carried.against, Some(typed));
    assert!(f
        .store
        .read_proposal(&f.spec, &carried.id)
        .unwrap()
        .undecided
        .is_empty());
}

// ---------------------------------------------------------------------------------------- commands

fn run(f: &Fixture, name: &str, args: Value) -> Result<Value, sce_specs::CommandError> {
    call(&f.store, &FakeRenderer, name, args)
}

#[test]
fn the_commands_make_decide_accept_and_read_a_proposal() {
    let f = fixture("cmd-flow");
    let made = f.made(&[("check", "m1")]);

    let registered = run(
        &f,
        "propose_import",
        json!({
            "id": f.spec.as_str(), "key": "press-1", "origin": "gui",
            "original": f.original, "extraction": f.extraction,
            "conversion": made.conversion, "report": made.report,
        }),
    )
    .unwrap();
    let proposal = registered["proposal"]["id"].as_str().unwrap().to_string();
    let refused = run(
        &f,
        "accept_proposal",
        json!({"id": f.spec.as_str(), "proposal": proposal, "text": "New.", "decisions": 0}),
    )
    .unwrap_err();
    let decided = run(
        &f,
        "decide_block",
        json!({"id": f.spec.as_str(), "proposal": proposal, "block": "m1", "what": "keep"}),
    )
    .unwrap();
    let read = run(
        &f,
        "read_proposal",
        json!({"id": f.spec.as_str(), "proposal": proposal}),
    )
    .unwrap();
    let accepted = run(
        &f,
        "accept_proposal",
        json!({"id": f.spec.as_str(), "proposal": proposal, "text": "New.", "decisions": 1}),
    )
    .unwrap();
    let listed = run(&f, "list_proposals", json!({"id": f.spec.as_str()})).unwrap();
    let documents = run(
        &f,
        "read_proposal_documents",
        json!({"id": f.spec.as_str(), "proposal": proposal}),
    )
    .unwrap();

    assert_eq!(registered["created"], true);
    assert_eq!(registered["proposal"]["state"], "pending");
    assert_eq!(refused.kind, "undecided");
    assert_eq!(decided["decision"]["n"], 1);
    assert_eq!(decided["decision"]["what"], "keep");
    assert_eq!(read["undecided"], json!([]));
    assert_eq!(read["head_matches"], true);
    assert_eq!(accepted["created"], true);
    assert_eq!(accepted["proposal"]["state"], "accepted");
    assert_eq!(accepted["saved"], json!(Revision::of(b"New.")));
    assert_eq!(listed["proposals"][0]["id"], json!(proposal));
    assert_eq!(documents["conversion"], json!(made.conversion));
    assert_eq!(source_text(&f).as_deref(), Some("New."));
}

#[test]
fn the_commands_reject_and_rebase_and_refuse_by_their_words() {
    let f = fixture("cmd-end");
    f.store.save_source(&f.spec, "Before.", None).unwrap();
    let old = f.propose("k", &[]);
    f.store
        .save_source(&f.spec, "After.", Some(&Revision::of(b"Before.")))
        .unwrap();

    let moved = run(
        &f,
        "accept_proposal",
        json!({"id": f.spec.as_str(), "proposal": old.id, "text": "x", "decisions": 0}),
    )
    .unwrap_err();
    let carried = run(
        &f,
        "rebase_proposal",
        json!({"id": f.spec.as_str(), "proposal": old.id}),
    )
    .unwrap();
    let new_id = carried["proposal"]["id"].as_str().unwrap().to_string();
    let rejected = run(
        &f,
        "reject_proposal",
        json!({"id": f.spec.as_str(), "proposal": new_id, "note": "no"}),
    )
    .unwrap();
    let ended = run(
        &f,
        "decide_block",
        json!({"id": f.spec.as_str(), "proposal": new_id, "block": "m1", "what": "keep"}),
    )
    .unwrap_err();

    assert_eq!(moved.kind, "moved");
    assert_eq!(carried["created"], true);
    assert_eq!(rejected["proposal"]["state"], "rejected");
    assert_eq!(rejected["proposal"]["note"], "no");
    assert_eq!(ended.kind, "proposal-ended");
}

#[test]
fn an_argument_nothing_reads_and_a_decision_nobody_defined_are_refused() {
    let f = fixture("cmd-args");
    let proposal = f.propose("k", &[]);
    let spec = f.spec.as_str();

    let extra = run(
        &f,
        "read_proposal",
        json!({"id": spec, "proposal": proposal.id, "colour": "red"}),
    );
    let unknown_kind = run(
        &f,
        "decide_block",
        json!({"id": spec, "proposal": proposal.id, "block": "m1", "what": "shrug"}),
    );
    let missing = run(
        &f,
        "accept_proposal",
        json!({"id": spec, "proposal": proposal.id, "text": "x"}),
    );

    for result in [extra, unknown_kind, missing] {
        assert_eq!(result.unwrap_err().kind, "bad-request");
    }
}

#[test]
fn the_new_commands_are_listed_and_the_version_moved() {
    let f = fixture("cmd-describe");

    let described = run(&f, "describe", json!({})).unwrap();

    assert_eq!(
        described["command_set_version"],
        sce_specs::COMMAND_SET_VERSION
    );
    let commands: Vec<&str> = described["commands"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c.as_str().unwrap())
        .collect();
    for name in [
        "propose_import",
        "read_proposal",
        "list_proposals",
        "read_proposal_documents",
        "decide_block",
        "accept_proposal",
        "reject_proposal",
        "rebase_proposal",
    ] {
        assert!(commands.contains(&name), "{name}");
    }
    // Used only so that the fixture's own text is read once: the extraction is kept as it was given.
    assert_eq!(
        f.store.read_extraction(&f.spec, &f.extraction).unwrap(),
        f.extraction_text
    );
}

// ------------------------------------------------------------------------------ a part of the original

/// A scope of the fixture's extraction, as `make-scope` writes it.
fn scope_of(extraction: &Revision, blocks: &[&str]) -> String {
    json!({"v": 1, "extraction": extraction.as_str(), "blocks": blocks}).to_string()
}

/// The check of a conversion made under `scope`, which names it by its digest as the tools do.
fn scoped_report(
    extraction: &Revision,
    conversion: &str,
    scope: &str,
    asks: &[(&str, &str)],
) -> String {
    let mut report: Value = serde_json::from_str(&report_of(extraction, conversion, asks)).unwrap();
    report["scope"] = json!(Revision::of(scope.as_bytes()).as_str());
    report["out_of_scope"] = json!(1);
    report.to_string()
}

impl Fixture {
    fn scoped(&self, blocks: &[&str], asks: &[(&str, &str)]) -> (Made, String) {
        let scope = scope_of(&self.extraction, blocks);
        let conversion = conversion_of(&self.extraction, &["m1", "m2"]);
        let report = scoped_report(&self.extraction, &conversion, &scope, asks);
        (Made { conversion, report }, scope)
    }

    fn registration_in<'a>(
        &self,
        key: &'a str,
        made: &'a Made,
        scope: Option<&'a str>,
    ) -> ProposalRegistration<'a> {
        ProposalRegistration {
            scope,
            ..self.registration(key, made)
        }
    }
}

#[test]
fn a_proposal_of_a_part_keeps_the_scope_and_gives_it_back_with_the_conversion() {
    let f = fixture("scope-kept");
    let (made, scope) = f.scoped(&["b1", "b2"], &[("check", "m1")]);

    let registered = f
        .store
        .propose_import(&f.spec, f.registration_in("k", &made, Some(&scope)))
        .unwrap();
    let documents = f
        .store
        .read_proposal_documents(&f.spec, &registered.proposal.id)
        .unwrap();

    assert_eq!(
        registered.proposal.scope,
        Some(Revision::of(scope.as_bytes()))
    );
    assert_eq!(documents.scope.as_deref(), Some(scope.as_str()));
    assert!(f
        .dir()
        .join("scopes")
        .join(format!("{}.json", Revision::of(scope.as_bytes()).as_str()))
        .is_file());
    // The nearest that must differ: a proposal of the whole holds no scope.
    let whole = fixture("scope-kept-whole").propose("whole", &[]);
    assert_eq!(whole.scope, None);
}

#[test]
fn a_scope_that_is_not_one_of_this_extraction_is_refused_and_the_nearest_that_is_is_not() {
    let f = fixture("scope-shape");
    let other = Revision::of(b"another extraction");
    let wrong_version =
        json!({"v": 2, "extraction": f.extraction.as_str(), "blocks": ["b1"]}).to_string();
    let extra_field =
        json!({"v": 1, "extraction": f.extraction.as_str(), "blocks": ["b1"], "why": "x"})
            .to_string();
    let cases = [
        ("another extraction", scope_of(&other, &["b1", "b2"])),
        ("no block", scope_of(&f.extraction, &[])),
        (
            "a block it does not have",
            scope_of(&f.extraction, &["b1", "b9"]),
        ),
        ("a block twice", scope_of(&f.extraction, &["b1", "b1"])),
        (
            "not in the extraction's order",
            scope_of(&f.extraction, &["b2", "b1"]),
        ),
        ("every block", scope_of(&f.extraction, &["b1", "b2", "b3"])),
        ("a version nobody reads", wrong_version),
        ("a field nobody defined", extra_field),
        ("not JSON", "[1".to_string()),
    ];
    for (name, scope) in cases {
        let conversion = conversion_of(&f.extraction, &["m1", "m2"]);
        let report = scoped_report(&f.extraction, &conversion, &scope, &[]);
        let made = Made { conversion, report };

        let error = f
            .store
            .propose_import(&f.spec, f.registration_in(name, &made, Some(&scope)))
            .unwrap_err();

        assert_eq!(refusal(error).0, "bad-proposal", "{name}");
    }
    let (made, scope) = f.scoped(&["b1", "b3"], &[]);
    f.store
        .propose_import(&f.spec, f.registration_in("good", &made, Some(&scope)))
        .unwrap();
}

#[test]
fn a_report_made_under_another_part_than_the_one_named_is_refused() {
    let f = fixture("scope-report");
    let (made, scope) = f.scoped(&["b1", "b2"], &[]);
    let conversion = made.conversion.clone();
    let under_another = scoped_report(
        &f.extraction,
        &conversion,
        &scope_of(&f.extraction, &["b1", "b3"]),
        &[],
    );
    let under_none = report_of(&f.extraction, &conversion, &[]);
    let wrong = [
        (
            "a report of another part",
            Made {
                conversion: conversion.clone(),
                report: under_another,
            },
        ),
        (
            "a report of the whole, with a scope named",
            Made {
                conversion: conversion.clone(),
                report: under_none,
            },
        ),
    ];

    for (name, made) in &wrong {
        let error = f
            .store
            .propose_import(&f.spec, f.registration_in(name, made, Some(&scope)))
            .unwrap_err();
        assert_eq!(refusal(error).0, "bad-proposal", "{name}");
    }
    // And the other way: a report made under a part, with no scope named.
    let error = f
        .store
        .propose_import(&f.spec, f.registration_in("no scope", &made, None))
        .unwrap_err();
    assert_eq!(refusal(error).0, "bad-proposal");
    // The nearest that must pass: the report and the scope that name each other.
    f.store
        .propose_import(&f.spec, f.registration_in("good", &made, Some(&scope)))
        .unwrap();
}

#[test]
fn what_the_check_asks_of_the_original_is_inside_the_part() {
    let f = fixture("scope-asks");
    let (outside, scope) = f.scoped(&["b1", "b2"], &[("missing", "b3")]);
    let (inside, _) = f.scoped(&["b1", "b2"], &[("missing", "b2"), ("check", "m1")]);

    let error = f
        .store
        .propose_import(&f.spec, f.registration_in("out", &outside, Some(&scope)))
        .unwrap_err();
    let kept = f
        .store
        .propose_import(&f.spec, f.registration_in("in", &inside, Some(&scope)))
        .unwrap();

    assert_eq!(refusal(error).0, "bad-proposal");
    assert_eq!(kept.proposal.asks.len(), 2);
}

#[test]
fn the_same_key_with_another_part_is_another_input() {
    let f = fixture("scope-key");
    let (made, scope) = f.scoped(&["b1", "b2"], &[]);
    let first = f
        .store
        .propose_import(&f.spec, f.registration_in("k", &made, Some(&scope)))
        .unwrap();

    let again = f
        .store
        .propose_import(&f.spec, f.registration_in("k", &made, Some(&scope)))
        .unwrap();
    let (other_made, other_scope) = f.scoped(&["b1", "b3"], &[]);
    let error = f
        .store
        .propose_import(
            &f.spec,
            f.registration_in("k", &other_made, Some(&other_scope)),
        )
        .unwrap_err();
    let whole = f.made(&[]);
    let error_whole = f
        .store
        .propose_import(&f.spec, f.registration("k", &whole))
        .unwrap_err();

    assert!(!again.created);
    assert_eq!(again.proposal.id, first.proposal.id);
    assert_eq!(refusal(error).0, "key-reused");
    assert_eq!(refusal(error_whole).0, "key-reused");
}

#[test]
fn a_block_outside_the_part_is_neither_added_nor_dropped_and_one_inside_is() {
    let f = fixture("scope-decide");
    let (made, scope) = f.scoped(&["b1", "b2"], &[]);
    let proposal = f
        .store
        .propose_import(&f.spec, f.registration_in("k", &made, Some(&scope)))
        .unwrap()
        .proposal;

    for what in [DecisionKind::Add, DecisionKind::Drop] {
        let error = f
            .store
            .decide_block(&f.spec, &proposal.id, "b3", what, None)
            .unwrap_err();
        assert_eq!(refusal(error).0, "bad-proposal", "{what:?}");
    }
    f.decide(&proposal.id, "b2", DecisionKind::Add);
    f.decide(&proposal.id, "m1", DecisionKind::Keep);

    let view = f.store.read_proposal(&f.spec, &proposal.id).unwrap();
    assert_eq!(view.decisions.len(), 2);
}

#[test]
fn a_proposal_carried_forward_keeps_its_part() {
    let f = fixture("scope-rebase");
    f.store.save_source(&f.spec, "Before.", None).unwrap();
    let (made, scope) = f.scoped(&["b1", "b2"], &[]);
    let old = f
        .store
        .propose_import(&f.spec, f.registration_in("k", &made, Some(&scope)))
        .unwrap()
        .proposal;
    f.store
        .save_source(&f.spec, "After.", Some(&Revision::of(b"Before.")))
        .unwrap();

    let carried = f.store.rebase_proposal(&f.spec, &old.id).unwrap().proposal;
    let documents = f
        .store
        .read_proposal_documents(&f.spec, &carried.id)
        .unwrap();

    assert_eq!(carried.scope, old.scope);
    assert_eq!(documents.scope.as_deref(), Some(scope.as_str()));
}

#[test]
fn the_command_takes_a_scope_and_reads_it_back() {
    let f = fixture("scope-command");
    let (made, scope) = f.scoped(&["b1", "b2"], &[]);

    let registered = run(
        &f,
        "propose_import",
        json!({
            "id": f.spec.as_str(), "key": "k", "origin": "gui",
            "original": f.original, "extraction": f.extraction,
            "conversion": made.conversion, "report": made.report, "scope": scope,
        }),
    )
    .unwrap();
    let proposal = registered["proposal"]["id"].as_str().unwrap().to_string();
    let documents = run(
        &f,
        "read_proposal_documents",
        json!({"id": f.spec.as_str(), "proposal": proposal}),
    )
    .unwrap();
    let other = fixture("scope-command-whole");
    let whole = other.propose("whole", &[]);
    let whole_documents = run(
        &other,
        "read_proposal_documents",
        json!({"id": other.spec.as_str(), "proposal": whole.id}),
    )
    .unwrap();

    assert_eq!(
        registered["proposal"]["scope"],
        json!(Revision::of(scope.as_bytes()))
    );
    assert_eq!(documents["scope"], json!(scope));
    // The whole has no `scope` in either place, so a screen written before it reads what it always did.
    assert!(whole_documents.get("scope").is_none());
    assert!(whole.scope.is_none());
}

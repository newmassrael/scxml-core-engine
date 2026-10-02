// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What a played example says about the requirement it names.
//!
//! Requirement closure asks whether a node claims a requirement's id, which is
//! evidence only for a requirement met by something existing. A requirement
//! met by something NOT happening has no honest answer among the annotation's
//! outcomes, and [`Outcome::NeedsScenario`] says what would carry it: a
//! scenario asserting the thing does not occur, and passing. A scenario names
//! the requirements it is about (`requirements`), and the judge already carries
//! those ids on each verdict. [`close`] is the step that reads them: given
//! examples an owner accepted and what a driver observed running them, it moves
//! the rows the examples speak to and says which examples and which engine did.
//!
//! The rule, per requirement, over every scenario that names it:
//!
//! * one scenario FAILED, which is a check observed and not holding
//!   -> [`Outcome::ScenarioFailed`], for any requirement the document is
//!   meant to carry, whatever the annotation said;
//! * otherwise, for a requirement the annotation could not decide
//!   ([`Outcome::NeedsScenario`]), every scenario that names it passed
//!   -> [`Outcome::ScenarioPassed`];
//! * anything else leaves the row where it was. A scenario that was not
//!   judged, is blocked on a fact nobody has decided, or awaits a decision does
//!   not close a requirement, and one that did is carried on the row so the
//!   reader sees why it is still open.
//!
//! ⚠ [`Outcome::ScenarioPassed`] is not `implemented`. It says the machine
//! behaved as these examples say on the engine the summary names, over these
//! inputs, and a bounded scenario only up to its bound. It does not say the
//! examples are the whole of what the requirement means, that they are the
//! owner's (the summary carries the set's `origin`), or that another engine
//! agrees. A row that is `implemented` keeps that meaning: a node carries the
//! id and nothing was run.
//!
//! ⚠ Evidence about another specification is no evidence. A set whose
//! `specification` is not the requirement list's `doc_id` and `rev`, and a set
//! the judge did not judge, move nothing, and the summary says so.

use std::collections::BTreeMap;

use crate::requirement_manifest::{
    Classification, EvidenceEngine, Outcome, RequirementManifest, ScenarioEvidence,
    ScenarioEvidenceProblem, ScenarioEvidenceSummary,
};
use crate::scenario_judge::{Engine, Judgement, ScenarioVerdict, Verdict};
use crate::scenario_set::ScenarioSet;

/// Hold judged examples against a classification: move the rows they speak
/// to, carry their verdicts on those rows, and record on the classification
/// what was held, on which engine, and whether it moved anything.
///
/// `engine` is the engine the trace names; `set_sha256` is the digest of the
/// set the judgement was made against.
pub fn close(
    classification: &mut Classification,
    manifest: &RequirementManifest,
    set: &ScenarioSet,
    set_sha256: &str,
    engine: &Engine,
    judgement: &Judgement,
) {
    let mut summary = ScenarioEvidenceSummary {
        doc_id: set.specification.doc_id.clone(),
        rev: set.specification.rev.clone(),
        set_sha256: set_sha256.to_string(),
        origin: set.origin.as_str(),
        engine: EvidenceEngine {
            name: engine.name.clone(),
            detail: engine.detail.clone(),
        },
        used: false,
        why_not_used: None,
        problems: Vec::new(),
    };

    if !judgement.judged {
        summary.why_not_used = Some(format!(
            "the examples were not judged ({} problem(s) in the set, {} in the trace), so no \
             requirement is moved by them",
            judgement.set_problems.len(),
            judgement.trace_problems.len()
        ));
        classification.scenario_evidence = Some(summary);
        return;
    }
    if set.specification.doc_id != manifest.doc_id || set.specification.rev != manifest.rev {
        summary.why_not_used = Some(format!(
            "the examples are about {}@{} and the requirement list is {}@{}, so they say nothing \
             about it",
            set.specification.doc_id, set.specification.rev, manifest.doc_id, manifest.rev
        ));
        classification.scenario_evidence = Some(summary);
        return;
    }

    let listed: std::collections::BTreeSet<&str> = manifest
        .requirements
        .iter()
        .map(|entry| entry.id.as_str())
        .collect();
    let mut by_requirement: BTreeMap<&str, Vec<&ScenarioVerdict>> = BTreeMap::new();
    for verdict in &judgement.verdicts {
        for id in &verdict.requirements {
            if listed.contains(id.as_str()) {
                by_requirement.entry(id.as_str()).or_default().push(verdict);
            } else {
                summary.problems.push(ScenarioEvidenceProblem {
                    scenario: verdict.id.clone(),
                    requirement: id.clone(),
                });
            }
        }
    }

    for row in &mut classification.outcomes {
        let Some(verdicts) = by_requirement.get(row.id.as_str()) else {
            continue;
        };
        row.scenarios = verdicts
            .iter()
            .map(|verdict| ScenarioEvidence {
                scenario: verdict.id.clone(),
                verdict: verdict.verdict.as_str(),
                bound: verdict.bound,
                reason: verdict.reason.clone(),
            })
            .collect();
        // Only a requirement the document is meant to carry is moved: one the
        // manifest places elsewhere, or that nothing lists, keeps the word it
        // has and carries the evidence beside it.
        let movable = matches!(
            row.outcome,
            Outcome::Implemented | Outcome::Unresolved | Outcome::Missing | Outcome::NeedsScenario
        );
        if !movable {
            continue;
        }
        if verdicts.iter().any(|v| v.verdict == Verdict::Fail) {
            row.outcome = Outcome::ScenarioFailed;
            summary.used = true;
        } else if row.outcome == Outcome::NeedsScenario
            && verdicts.iter().all(|v| v.verdict == Verdict::Pass)
        {
            row.outcome = Outcome::ScenarioPassed;
            summary.used = true;
        }
    }

    if !summary.used && summary.why_not_used.is_none() {
        summary.why_not_used = Some(
            "no requirement was moved: none the examples name was failed by them, and none \
             that needs a scenario had every scenario naming it pass"
                .to_string(),
        );
    }
    classification.scenario_evidence = Some(summary);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::requirement_manifest::classify_citations;
    use crate::scenario_judge::{judge, Trace};

    const MANIFEST: &str = r#"{
      "doc_id": "retry", "rev": "1",
      "extraction": {"ids": "native", "trace": "none",
                     "modality_convention": "english-modal-verbs", "method": "ai-pass-1"},
      "sections": [{"id": "1", "title": "Retry"}],
      "requirements": [
        {"id": "R1", "section": "1", "modality": "shall_not", "at": {"page": 1}},
        {"id": "R2", "section": "1", "modality": "shall_not", "at": {"page": 1}},
        {"id": "R3", "section": "1", "at": {"page": 1}},
        {"id": "R4", "section": "1", "modality": "shall_not", "at": {"page": 1}},
        {"id": "R5", "section": "1", "at": {"page": 1}}
      ]
    }"#;

    fn set_text(doc_id: &str, rev: &str, extra_requirement: &str) -> String {
        format!(
            r#"{{
          "record": "sce-scenario-set", "v": 1,
          "specification": {{"doc_id": "{doc_id}", "rev": "{rev}"}},
          "origin": "ai-proposed",
          "interface": {{"inputs": [], "outputs": []}},
          "scenarios": [
            {{"id": "S-holds", "quote": "q1", "requirements": ["R1"],
              "steps": [{{"advance_ms": 5, "expect": {{"outbound": []}}}}]}},
            {{"id": "S-breaks", "quote": "q2", "requirements": ["R2", "R3"],
              "steps": [{{"advance_ms": 5, "expect": {{"outbound": []}}}}]}},
            {{"id": "S-open", "quote": "q3", "requirements": ["R4"], "status": "awaiting-decision",
              "decision": {{"question": "Who is the caller?"}},
              "steps": [{{"advance_ms": 5, "expect": {{"outbound": []}}}}]}},
            {{"id": "S-stray", "quote": "q4", "requirements": ["{extra_requirement}"],
              "steps": [{{"advance_ms": 5, "expect": {{"outbound": []}}}}]}},
            {{"id": "S-shall", "quote": "q5", "requirements": ["R5"],
              "steps": [{{"advance_ms": 5, "expect": {{"outbound": []}}}}]}}
          ]
        }}"#
        )
    }

    const TRACE: &str = r#"{
      "record": "sce-observation-trace", "v": 1,
      "engine": {"name": "test engine", "detail": "hand-written"},
      "observes": {"outbound": true, "finished": true, "configuration": true, "data": true},
      "runs": [
        {"scenario": "S-holds",
         "observations": [{"outbound": [], "finished": false, "configuration": [], "data": {}}]},
        {"scenario": "S-breaks",
         "observations": [{"outbound": [{"event": "Late"}], "finished": false,
                           "configuration": [], "data": {}}]},
        {"scenario": "S-stray",
         "observations": [{"outbound": [], "finished": false, "configuration": [], "data": {}}]},
        {"scenario": "S-shall",
         "observations": [{"outbound": [], "finished": false, "configuration": [], "data": {}}]}
      ]
    }"#;

    fn closed(doc_id: &str, rev: &str, extra: &str) -> Classification {
        let manifest = RequirementManifest::from_json(MANIFEST, "manifest").expect("manifest");
        let set = ScenarioSet::from_json(&set_text(doc_id, rev, extra), "set").expect("set");
        let trace = Trace::from_json(TRACE, "trace").expect("trace");
        let judgement = judge(&set, "digest", &trace);
        let mut classification = classify_citations(&[], &manifest);
        close(
            &mut classification,
            &manifest,
            &set,
            "digest",
            &trace.engine,
            &judgement,
        );
        classification
    }

    fn outcome_of(classification: &Classification, id: &str) -> Outcome {
        classification
            .outcomes
            .iter()
            .find(|row| row.id == id)
            .unwrap_or_else(|| panic!("no row {id}"))
            .outcome
    }

    #[test]
    fn a_shall_not_whose_every_scenario_passed_is_closed_by_them() {
        let classification = closed("retry", "1", "R4");
        assert_eq!(Outcome::ScenarioPassed, outcome_of(&classification, "R1"));
        let row = &classification.outcomes[0];
        assert_eq!("S-holds", row.scenarios[0].scenario);
        assert_eq!("pass", row.scenarios[0].verdict);
    }

    #[test]
    fn a_failed_scenario_moves_every_requirement_it_names_whatever_the_annotation_said() {
        let classification = closed("retry", "1", "R4");
        // R2 was needs-scenario, R3 (a shall, nothing cites it) was missing.
        assert_eq!(Outcome::ScenarioFailed, outcome_of(&classification, "R2"));
        assert_eq!(Outcome::ScenarioFailed, outcome_of(&classification, "R3"));
    }

    #[test]
    fn a_pass_does_not_make_a_shall_scenario_passed_it_is_carried_beside_the_annotation() {
        // R5 is a `shall` nothing cites: `missing`. Its only scenario passed,
        // which is evidence beside the annotation and not a node that carries it.
        let classification = closed("retry", "1", "R4");
        assert_eq!(Outcome::Missing, outcome_of(&classification, "R5"));
        let row = classification
            .outcomes
            .iter()
            .find(|r| r.id == "R5")
            .expect("R5");
        assert_eq!("pass", row.scenarios[0].verdict);
    }

    #[test]
    fn a_scenario_that_is_not_a_verdict_closes_nothing_and_is_carried_on_the_row() {
        let classification = closed("retry", "1", "R4");
        assert_eq!(Outcome::NeedsScenario, outcome_of(&classification, "R4"));
        let row = classification
            .outcomes
            .iter()
            .find(|r| r.id == "R4")
            .expect("R4");
        assert_eq!("awaiting-decision", row.scenarios[0].verdict);
    }

    #[test]
    fn a_pass_beside_an_open_scenario_does_not_close_the_requirement() {
        let manifest = RequirementManifest::from_json(MANIFEST, "manifest").expect("manifest");
        // S-holds passes; give R1 a second scenario that is awaiting a decision.
        let text = set_text("retry", "1", "R1");
        let set = ScenarioSet::from_json(&text, "set").expect("set");
        let trace = Trace::from_json(TRACE, "trace").expect("trace");
        let mut judgement = judge(&set, "digest", &trace);
        judgement
            .verdicts
            .iter_mut()
            .find(|v| v.id == "S-open")
            .expect("S-open")
            .requirements
            .push("R1".to_string());
        let mut classification = classify_citations(&[], &manifest);
        close(
            &mut classification,
            &manifest,
            &set,
            "digest",
            &trace.engine,
            &judgement,
        );
        assert_eq!(Outcome::NeedsScenario, outcome_of(&classification, "R1"));
    }

    #[test]
    fn the_summary_names_the_examples_and_the_engine_they_were_played_on() {
        let classification = closed("retry", "1", "R4");
        let summary = classification.scenario_evidence.expect("a summary");
        assert!(summary.used);
        assert_eq!("test engine", summary.engine.name);
        assert_eq!(Some("hand-written".to_string()), summary.engine.detail);
        assert_eq!("digest", summary.set_sha256);
        assert_eq!("ai-proposed", summary.origin);
    }

    #[test]
    fn a_scenario_naming_a_requirement_the_list_does_not_hold_is_a_problem_not_a_row() {
        let classification = closed("retry", "1", "R99");
        let summary = classification.scenario_evidence.expect("a summary");
        assert_eq!(
            vec![ScenarioEvidenceProblem {
                scenario: "S-stray".to_string(),
                requirement: "R99".to_string()
            }],
            summary.problems
        );
        assert!(classification.outcomes.iter().all(|row| row.id != "R99"));
    }

    #[test]
    fn examples_about_another_specification_move_nothing() {
        for (doc_id, rev) in [("another", "1"), ("retry", "2")] {
            let classification = closed(doc_id, rev, "R4");
            let summary = classification
                .scenario_evidence
                .as_ref()
                .expect("a summary");
            assert!(!summary.used, "{doc_id}@{rev}");
            let why = summary.why_not_used.as_ref().expect("a reason");
            assert!(why.contains("say nothing about it"), "{why}");
            assert_eq!(Outcome::NeedsScenario, outcome_of(&classification, "R1"));
            assert!(classification
                .outcomes
                .iter()
                .all(|row| row.scenarios.is_empty()));
        }
    }

    #[test]
    fn examples_the_judge_did_not_judge_move_nothing() {
        let manifest = RequirementManifest::from_json(MANIFEST, "manifest").expect("manifest");
        let set = ScenarioSet::from_json(&set_text("retry", "1", "R4"), "set").expect("set");
        let trace = Trace::from_json(TRACE, "trace").expect("trace");
        let mut judgement = judge(&set, "digest", &trace);
        judgement.judged = false;
        let mut classification = classify_citations(&[], &manifest);
        close(
            &mut classification,
            &manifest,
            &set,
            "digest",
            &trace.engine,
            &judgement,
        );
        let summary = classification
            .scenario_evidence
            .as_ref()
            .expect("a summary");
        assert!(!summary.used);
        let why = summary.why_not_used.as_ref().expect("a reason");
        assert!(why.contains("not judged"), "{why}");
        assert_eq!(Outcome::NeedsScenario, outcome_of(&classification, "R1"));
    }
}

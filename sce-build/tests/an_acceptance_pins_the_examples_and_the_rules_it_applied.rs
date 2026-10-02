// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! An acceptance pins the examples a design was held to and the rules it applied.
//!
//! Two things were left out of what the record pinned. The scenario set whose
//! passing closed a requirement (`scenario-passed`) was part of what the owner
//! accepted the design WITH, and a set edited afterwards is another set. And the
//! profile was pinned by its bytes, so a profile that changed lapsed the
//! acceptance with a sentence true of every rule in it and useful for none: it did
//! not say whether a rule THIS design relied on had moved.
//!
//! Held here, over a small design that cites one house rule at two places:
//!
//!   the set is pinned by its bytes, as a fourth role, and asking about another
//!       set (or none, once one is pinned) is a different request
//!   the rules the design cited are kept as the profile worded them, with the
//!       number of places, sorted, and a rule the design did not cite is not
//!   a profile edit that changes a cited rule's words, or removes it, names that
//!       rule as well as the profile; an edit to a rule the design never applied
//!       lapses the profile and names no rule; a profile that no longer loads
//!       names no rule it cannot read
//!   a record naming applied rules with no profile pinned, a rule twice or at no
//!       place is refused when read

use std::fs;
use std::path::Path;

use sce_build::acceptance_record::{AcceptanceRecord, AppliedRule, Lapse, RecordError, SourceRole};
use sce_build::authoring_profile::Confirmation;

const MANIFEST: &str = r#"{
  "doc_id": "door", "rev": "1",
  "extraction": {"ids": "native", "trace": "none",
                 "modality_convention": "english-modal-verbs", "method": "ai-pass-1"},
  "sections": [{"id": "1", "title": "Door"}],
  "requirements": [{"id": "R1", "section": "1", "at": {"page": 1}}]
}"#;

const SAID: &str = "sce:assumed-reason=\"the specification names no event for this state\"";

/// Cites house rule `H1` at two places and `H9`, which no profile holds, at one.
fn design() -> String {
    format!(
        r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="idle">
  <state id="idle" sce:assumed="H1" {SAID}><transition event="go" target="open"/></state>
  <state id="open" sce:assumed="H1" {SAID}><transition event="shut" target="stray"/></state>
  <state id="stray" sce:assumed="H9" {SAID}/>
</scxml>"#
    )
}

fn profile(h1: &str) -> String {
    format!(
        r#"{{"record":"sce-authoring-profile","v":1,"house_rules":[
  {{"id":"H1","rule":{h1:?}}},{{"id":"H2","rule":"A door that is open stays open until told."}}]}}"#
    )
}

const H1: &str = "An event a state does not mention is ignored.";

const SET: &str = r#"{"record":"sce-scenario-set","v":1,
  "specification":{"doc_id":"door","rev":"1"},"origin":"ai-proposed",
  "interface":{"inputs":[],"outputs":[]},
  "scenarios":[{"id":"S1","quote":"q","requirements":["R1"],
    "steps":[{"advance_ms":5,"expect":{"outbound":[]}}]}]}"#;

struct Tree {
    dir: tempfile::TempDir,
}

impl Tree {
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("tempdir");
        for (name, text) in [
            ("door.scxml", design()),
            ("list.json", MANIFEST.to_string()),
            ("profile.json", profile(H1)),
            ("set.json", SET.to_string()),
        ] {
            fs::write(dir.path().join(name), text).expect("write");
        }
        Tree { dir }
    }

    fn root(&self) -> &Path {
        self.dir.path()
    }

    fn path(&self, name: &str) -> std::path::PathBuf {
        self.dir.path().join(name)
    }

    fn write(&self, name: &str, text: &str) {
        fs::write(self.path(name), text).expect("write");
    }

    fn take(&self, with: &[(SourceRole, &str)]) -> AcceptanceRecord {
        let owned: Vec<(SourceRole, std::path::PathBuf)> = with
            .iter()
            .map(|(role, name)| (*role, self.path(name)))
            .collect();
        let sources: Vec<(SourceRole, &Path)> = owned
            .iter()
            .map(|(role, path)| (*role, path.as_path()))
            .collect();
        AcceptanceRecord::take_authored(
            self.root(),
            &self.path("door.scxml"),
            &self.path("list.json"),
            "base",
            &sources,
        )
        .expect("the record is taken")
    }

    fn full(&self) -> AcceptanceRecord {
        self.take(&[
            (SourceRole::Profile, "profile.json"),
            (SourceRole::Examples, "set.json"),
        ])
    }
}

fn named(lapses: &[Lapse]) -> Vec<String> {
    let mut names: Vec<String> = lapses
        .iter()
        .map(|lapse| match lapse {
            Lapse::Source { role, path, .. } => format!("source {role} {path}"),
            Lapse::Rule { id, current, .. } => match current {
                Some(_) => format!("rule {id} changed"),
                None => format!("rule {id} gone"),
            },
            Lapse::NotAuthoredFrom { role, .. } => format!("not authored from {role}"),
            other => format!("other: {other}"),
        })
        .collect();
    names.sort();
    names
}

#[test]
fn the_rules_the_design_cited_are_kept_as_the_profile_worded_them() {
    let tree = Tree::new();
    let record = tree.full();
    assert_eq!(
        vec![AppliedRule {
            id: "H1".to_string(),
            rule: H1.to_string(),
            places: 2,
            quote: None,
            confirmation: None,
        }],
        record.applied_rules,
        "H2 is in the profile and not cited; H9 is cited and in no profile"
    );
    let json = record.to_json();
    assert!(json.contains("\"applied_rules\""), "{json}");
    assert_eq!(
        record,
        AcceptanceRecord::from_json(&json).expect("it reads back")
    );
}

const QUOTE: &str = "just ignore anything the door does not mention";

/// The profile with `H1` carrying the owner's words and a relayed confirmation.
fn profile_said(quote: &str) -> String {
    format!(
        r#"{{"record":"sce-authoring-profile","v":1,"house_rules":[
  {{"id":"H1","rule":{H1:?},"quote":{quote:?},"confirmation":"relayed"}},
  {{"id":"H2","rule":"A door that is open stays open until told."}}]}}"#
    )
}

#[test]
fn a_rule_is_kept_with_the_words_and_the_vouching_it_was_accepted_with() {
    let tree = Tree::new();
    tree.write("profile.json", &profile_said(QUOTE));
    let record = tree.full();
    let [applied] = record.applied_rules.as_slice() else {
        panic!("one rule cited: {:?}", record.applied_rules);
    };
    assert_eq!(applied.quote.as_deref(), Some(QUOTE));
    assert_eq!(applied.confirmation, Some(Confirmation::Relayed));
    let written: serde_json::Value =
        serde_json::from_str(&record.to_json()).expect("the record is JSON");
    assert_eq!(written["applied_rules"][0]["quote"], QUOTE);
    assert_eq!(written["applied_rules"][0]["confirmation"], "relayed");
    assert_eq!(
        record,
        AcceptanceRecord::from_json(&record.to_json()).expect("it reads back")
    );
    // A rule the profile holds with neither says neither: the record does not
    // write a quote nobody gave.
    let plain = Tree::new().full().to_json();
    let plain: serde_json::Value = serde_json::from_str(&plain).expect("JSON");
    assert!(plain["applied_rules"][0].get("quote").is_none(), "{plain}");
    assert!(
        plain["applied_rules"][0].get("confirmation").is_none(),
        "{plain}"
    );
}

#[test]
fn a_rule_whose_provenance_moved_and_whose_words_did_not_lapses_the_profile_only() {
    // How the rule stood when it was accepted is recorded, and not compared:
    // the lapse of a rule is about what it SAYS.
    let tree = Tree::new();
    tree.write("profile.json", &profile_said(QUOTE));
    let record = tree.full();
    tree.write(
        "profile.json",
        &profile_said("ignore what the door never says"),
    );
    let lapses = record.recheck(tree.root(), "base").expect("recheck");
    assert_eq!(
        vec!["source authoring profile profile.json"],
        named(&lapses)
    );
}

#[test]
fn a_record_taken_under_no_profile_applies_no_rule_even_when_the_design_cites_one() {
    let record = Tree::new().take(&[]);
    assert!(record.applied_rules.is_empty());
    assert!(!record.to_json().contains("applied_rules"));
}

#[test]
fn the_examples_are_pinned_as_a_role_of_their_own() {
    let tree = Tree::new();
    let record = tree.full();
    assert_eq!(
        vec![SourceRole::Profile, SourceRole::Examples],
        record
            .authored_from
            .iter()
            .map(|p| p.role)
            .collect::<Vec<_>>()
    );
    assert!(record
        .recheck(tree.root(), "base")
        .expect("recheck")
        .is_empty());

    tree.write("set.json", &SET.replace("\"q\"", "\"another quote\""));
    let lapses = record.recheck(tree.root(), "base").expect("recheck");
    assert_eq!(vec!["source scenario set set.json"], named(&lapses));
    assert!(lapses[0]
        .to_string()
        .contains("the scenario set this design was held to changed"));
}

#[test]
fn another_set_or_none_at_all_is_a_different_request_once_one_is_pinned() {
    let tree = Tree::new();
    let record = tree.full();
    let profile = tree.path("profile.json");
    let set = tree.path("set.json");
    let asked = |roles: &[(SourceRole, &Path)]| {
        named(
            &record
                .recheck_for(tree.root(), "base", roles)
                .expect("recheck_for"),
        )
    };
    assert!(asked(&[
        (SourceRole::Profile, &profile),
        (SourceRole::Examples, &set)
    ])
    .is_empty());
    assert_eq!(
        vec!["not authored from scenario set"],
        asked(&[(SourceRole::Profile, &profile)])
    );
    tree.write("other.json", &SET.replace("\"q\"", "\"another quote\""));
    let other = tree.path("other.json");
    assert_eq!(
        vec!["not authored from scenario set"],
        asked(&[
            (SourceRole::Profile, &profile),
            (SourceRole::Examples, &other)
        ])
    );
}

#[test]
fn a_cited_rule_that_now_says_something_else_is_named() {
    let tree = Tree::new();
    let record = tree.full();
    tree.write(
        "profile.json",
        &profile("An unmentioned event is an error."),
    );
    let lapses = record.recheck(tree.root(), "base").expect("recheck");
    assert_eq!(
        vec!["rule H1 changed", "source authoring profile profile.json"],
        named(&lapses)
    );
    let said = lapses
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(" | ");
    assert!(
        said.contains("house rule `H1`, which the design applied at 2 places"),
        "{said}"
    );
    assert!(
        said.contains("now says \"An unmentioned event is an error.\""),
        "{said}"
    );
    assert!(
        said.contains(&format!("it said {H1:?} when the design was accepted")),
        "{said}"
    );
}

#[test]
fn a_cited_rule_that_is_gone_from_the_profile_is_named() {
    let tree = Tree::new();
    let record = tree.full();
    tree.write(
        "profile.json",
        r#"{"record":"sce-authoring-profile","v":1,"house_rules":[{"id":"H2","rule":"x"}]}"#,
    );
    let lapses = record.recheck(tree.root(), "base").expect("recheck");
    assert_eq!(
        vec!["rule H1 gone", "source authoring profile profile.json"],
        named(&lapses)
    );
    assert!(lapses
        .iter()
        .any(|l| l.to_string().contains("is no longer in the profile")));
}

#[test]
fn an_edit_to_a_rule_the_design_never_applied_lapses_the_profile_and_names_no_rule() {
    let tree = Tree::new();
    let record = tree.full();
    tree.write(
        "profile.json",
        &profile(H1).replace("stays open until told", "closes by itself"),
    );
    let lapses = record.recheck(tree.root(), "base").expect("recheck");
    assert_eq!(
        vec!["source authoring profile profile.json"],
        named(&lapses)
    );
}

#[test]
fn a_profile_that_no_longer_loads_names_no_rule_it_cannot_read() {
    let tree = Tree::new();
    let record = tree.full();
    tree.write("profile.json", "{ not json");
    let lapses = record.recheck(tree.root(), "base").expect("recheck");
    assert_eq!(
        vec!["source authoring profile profile.json"],
        named(&lapses)
    );
}

fn refused(json: &str) -> String {
    match AcceptanceRecord::from_json(json) {
        Err(RecordError::Format { detail }) => detail,
        Err(other) => panic!("refused for another reason: {other}"),
        Ok(_) => panic!("a record that must be refused was read"),
    }
}

#[test]
fn a_record_that_names_rules_it_cannot_have_come_from_is_refused_when_read() {
    let tree = Tree::new();
    let text = tree.full().to_json();
    let value: serde_json::Value = serde_json::from_str(&text).expect("json");

    let mut without_profile = value.clone();
    without_profile["authored_from"] = serde_json::json!([]);
    assert!(refused(&without_profile.to_string()).contains("no authoring profile is pinned"));

    let mut twice = value.clone();
    let rule = twice["applied_rules"][0].clone();
    twice["applied_rules"]
        .as_array_mut()
        .expect("array")
        .push(rule);
    assert!(refused(&twice.to_string()).contains("named more than once"));

    let mut nowhere = value.clone();
    nowhere["applied_rules"][0]["places"] = serde_json::json!(0);
    assert!(refused(&nowhere.to_string()).contains("no place that cites it"));
}

#[test]
fn two_scenario_sets_are_refused_when_taking_a_record() {
    let tree = Tree::new();
    tree.write("second.json", SET);
    let owned = [tree.path("set.json"), tree.path("second.json")];
    let sources: Vec<(SourceRole, &Path)> = owned
        .iter()
        .map(|p| (SourceRole::Examples, p.as_path()))
        .collect();
    let error = AcceptanceRecord::take_authored(
        tree.root(),
        &tree.path("door.scxml"),
        &tree.path("list.json"),
        "base",
        &sources,
    )
    .expect_err("one set at most");
    assert!(
        error.to_string().contains("2 scenario sets given"),
        "{error}"
    );
}

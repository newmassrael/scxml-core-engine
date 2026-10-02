// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! `acceptance-impact` names what a change to a shared file touches.
//!
//! A profile of house rules is shared by every specification an owner keeps, so
//! editing one rule raises a question `acceptance-check` answers one record at a
//! time: which of the acceptances that applied it moved? Held here, over three
//! designs under one profile and a fourth accepted under none:
//!
//!   a profile edit lapses every design accepted under it, and names the rule
//!       (id, places, both wordings) only for the designs that APPLIED it
//!   a design accepted under no profile still holds
//!   a record that cannot be read is a line of its own and does not end the scan
//!   the lapse is data beside its sentence, so no consumer reads a rule's id out
//!       of prose
//!   it is a report: exit status 0 whenever it ran

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use sce_build::acceptance_record::{AcceptanceRecord, SourceRole};
use serde_json::Value;

const CODEGEN: &str = env!("CARGO_BIN_EXE_sce-codegen");

const MANIFEST: &str = r#"{
  "doc_id": "door", "rev": "1",
  "extraction": {"ids": "native", "trace": "none",
                 "modality_convention": "english-modal-verbs", "method": "ai-pass-1"},
  "sections": [{"id": "1", "title": "Door"}],
  "requirements": [{"id": "R1", "section": "1", "at": {"page": 1}}]
}"#;

const SAID: &str = "sce:assumed-reason=\"the specification names no event for this state\"";

const H1: &str = "An event a state does not mention is ignored.";
const H2: &str = "A door that is open stays open until told.";

/// A door that cites `rules` (one per state), so the record that takes it applies
/// exactly those.
fn design(rules: &[&str]) -> String {
    let states: String = rules
        .iter()
        .enumerate()
        .map(|(n, rule)| format!("  <state id=\"s{n}\" sce:assumed=\"{rule}\" {SAID}/>\n"))
        .collect();
    format!(
        r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="idle">
  <state id="idle"><transition event="go" target="idle"/></state>
{states}</scxml>"#
    )
}

fn profile(h1: &str) -> String {
    format!(
        r#"{{"record":"sce-authoring-profile","v":1,"house_rules":[
  {{"id":"H1","rule":{h1:?}}},{{"id":"H2","rule":{H2:?}}}]}}"#
    )
}

struct Tree {
    dir: tempfile::TempDir,
}

impl Tree {
    /// Four designs and their records: `a` applies H1 at two places, `b` applies
    /// H2, `c` applies neither, and `d` is `a` accepted under no profile.
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("tempdir");
        let tree = Tree { dir };
        tree.write("list.json", MANIFEST);
        tree.write("profile.json", &profile(H1));
        for (name, rules) in [("a", &["H1", "H1"][..]), ("b", &["H2"]), ("c", &[])] {
            tree.write(&format!("{name}.scxml"), &design(rules));
            tree.accept(name, name, true);
        }
        tree.write("d.scxml", &design(&["H1", "H1"]));
        tree.accept("d", "d", false);
        tree
    }

    fn path(&self, name: &str) -> PathBuf {
        self.dir.path().join(name)
    }

    fn write(&self, name: &str, text: &str) {
        fs::write(self.path(name), text).expect("write");
    }

    fn accept(&self, design: &str, record: &str, under_the_profile: bool) {
        let profile = self.path("profile.json");
        let sources: Vec<(SourceRole, &Path)> = if under_the_profile {
            vec![(SourceRole::Profile, profile.as_path())]
        } else {
            Vec::new()
        };
        let taken = AcceptanceRecord::take_authored(
            self.dir.path(),
            &self.path(&format!("{design}.scxml")),
            &self.path("list.json"),
            "base",
            &sources,
        )
        .expect("the record is taken");
        self.write(&format!("{record}.accepted.json"), &taken.to_json());
    }

    /// `acceptance-impact` over `records`, as the lines it printed.
    fn impact(&self, records: &[&str]) -> (i32, Vec<Value>) {
        let out = Command::new(CODEGEN)
            .arg("acceptance-impact")
            .args(records.iter().map(|r| self.path(r)))
            .arg("--root")
            .arg(self.dir.path())
            .output()
            .expect("run sce-codegen");
        let lines = String::from_utf8_lossy(&out.stdout)
            .lines()
            .map(|line| serde_json::from_str(line).expect("every line is JSON"))
            .collect();
        (out.status.code().unwrap_or(-1), lines)
    }
}

const RECORDS: [&str; 4] = [
    "a.accepted.json",
    "b.accepted.json",
    "c.accepted.json",
    "d.accepted.json",
];

fn lapse_kinds(line: &Value) -> Vec<String> {
    line["lapses"]
        .as_array()
        .expect("lapses")
        .iter()
        .map(|l| l["kind"].as_str().expect("kind").to_string())
        .collect()
}

#[test]
fn an_unchanged_tree_holds_for_every_record() {
    let tree = Tree::new();
    let (status, lines) = tree.impact(&RECORDS);
    assert_eq!(status, 0, "{lines:?}");
    assert_eq!(lines.len(), 5, "a line per record and a summary: {lines:?}");
    for line in &lines[..4] {
        assert_eq!(line["holds"], true, "{line}");
        assert_eq!(line["lapses"], serde_json::json!([]), "{line}");
        assert_eq!(line["variant"], "base", "{line}");
    }
    assert_eq!(
        lines[4],
        serde_json::json!({"v":1,"kind":"acceptance-impact-summary",
                           "records":4,"holding":4,"lapsed":0,"unusable":0})
    );
}

#[test]
fn an_edited_rule_is_named_only_for_the_designs_that_applied_it() {
    let tree = Tree::new();
    tree.write(
        "profile.json",
        &profile("Every unmentioned event is refused."),
    );
    let (status, lines) = tree.impact(&RECORDS);
    assert_eq!(status, 0, "{lines:?}");

    // a applied H1 at two places: the profile moved AND the rule is named, with
    // both wordings and the places, as data.
    let a = &lines[0];
    assert_eq!(a["holds"], false, "{a}");
    assert_eq!(lapse_kinds(a), ["source", "rule"], "{a}");
    let rule = &a["lapses"][1];
    assert_eq!(rule["id"], "H1", "{rule}");
    assert_eq!(rule["places"], 2, "{rule}");
    assert_eq!(rule["recorded"], H1, "{rule}");
    assert_eq!(
        rule["current"], "Every unmentioned event is refused.",
        "{rule}"
    );
    assert!(
        rule["message"]
            .as_str()
            .unwrap()
            .contains("house rule `H1`"),
        "the sentence rides beside the data: {rule}"
    );

    // b applied H2, which did not change, and c applied neither: both are under
    // the profile whose bytes moved, and neither names a rule.
    for line in [&lines[1], &lines[2]] {
        assert_eq!(line["holds"], false, "{line}");
        assert_eq!(lapse_kinds(line), ["source"], "{line}");
    }

    // d was accepted under no profile, so a profile edit is none of its business.
    assert_eq!(lines[3]["holds"], true, "{}", lines[3]);
    assert_eq!(
        lines[4],
        serde_json::json!({"v":1,"kind":"acceptance-impact-summary",
                           "records":4,"holding":1,"lapsed":3,"unusable":0})
    );
}

#[test]
fn a_rule_that_is_gone_is_a_lapse_with_no_current_wording() {
    let tree = Tree::new();
    tree.write(
        "profile.json",
        &format!(
            r#"{{"record":"sce-authoring-profile","v":1,"house_rules":[
  {{"id":"H2","rule":{H2:?}}}]}}"#
        ),
    );
    let (_, lines) = tree.impact(&RECORDS);
    let rule = &lines[0]["lapses"][1];
    assert_eq!(rule["kind"], "rule", "{rule}");
    assert_eq!(rule["id"], "H1", "{rule}");
    assert_eq!(rule["current"], Value::Null, "{rule}");
}

#[test]
fn a_record_that_cannot_be_read_is_a_line_and_the_scan_goes_on() {
    let tree = Tree::new();
    tree.write("garbage.accepted.json", "this is not a record");
    let (status, lines) = tree.impact(&[
        "missing.accepted.json",
        "a.accepted.json",
        "garbage.accepted.json",
        "b.accepted.json",
    ]);
    assert_eq!(status, 0, "a report exits 0 whenever it ran: {lines:?}");
    assert_eq!(lines.len(), 5, "{lines:?}");
    assert_eq!(lines[0]["unusable"]["kind"], "read", "{}", lines[0]);
    assert!(lines[0].get("holds").is_none(), "{}", lines[0]);
    assert_eq!(lines[1]["holds"], true, "{}", lines[1]);
    assert!(lines[2]["unusable"]["kind"].is_string(), "{}", lines[2]);
    assert_eq!(
        lines[3]["holds"], true,
        "a record after a bad one is still asked"
    );
    assert_eq!(
        lines[4],
        serde_json::json!({"v":1,"kind":"acceptance-impact-summary",
                           "records":4,"holding":2,"lapsed":0,"unusable":2})
    );
}

#[test]
fn a_design_that_moved_is_a_lapse_of_its_own_kind() {
    let tree = Tree::new();
    tree.write("b.scxml", &design(&["H2", "H2"]));
    let (_, lines) = tree.impact(&RECORDS);
    assert_eq!(lines[1]["holds"], false, "{}", lines[1]);
    assert!(
        lapse_kinds(&lines[1]).iter().any(|k| k == "moved"),
        "{}",
        lines[1]
    );
    assert_eq!(
        lines[0]["holds"], true,
        "the others are untouched: {}",
        lines[0]
    );
}

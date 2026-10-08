// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! An acceptance says what moved in a design since it was taken.
//!
//! A record that keeps, per requirement, the digests of the rows the owner was
//! shown can be set against the design as it is now, and say requirement by
//! requirement whether the rows moved, where the new ones are, and how many of
//! the recorded ones are gone
//! (`docs/adr/0008-an-acceptance-says-what-moved-since-it-was-taken.md`).
//!
//! Held here, on the alarm of `an_acceptance_keeps_what_each_requirement_rests_on`
//! (a timer, three cited requirements and one nobody cites) and on a lookup:
//!
//!   the design as it was accepted reports every requirement unchanged
//!   retiming the delay reports the requirements whose closure holds it as
//!       changed, with the `<send>`'s place and one recorded row gone, and no other
//!   a transition inserted ahead of a cited one reports its requirement changed
//!       and names the place the row has now
//!   a citation taken off reports the requirement dropped; one put on reports it
//!       new, with the place
//!   a state no requirement reaches reports only unclaimed rows added
//!   a lookup's edited entry reports its requirement changed with the entry's place
//!   a record from before evidence is refused, a design that does not parse is
//!       refused, and neither is mistaken for "everything is new"
//!   the same holds through the binary: its lines are the library's

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use sce_build::acceptance_record::{
    AcceptanceRecord, EvidenceDelta, RecordError, RequirementChange,
};

const CODEGEN: &str = env!("CARGO_BIN_EXE_sce-codegen");

const MANIFEST: &str = r#"{
  "doc_id": "alarm", "rev": "1",
  "extraction": {"ids": "native", "trace": "none",
                 "modality_convention": "english-modal-verbs", "method": "ai-pass-1"},
  "sections": [{"id": "1", "title": "Alarm"}],
  "requirements": [
    {"id": "R1", "section": "1", "at": {"page": 1}},
    {"id": "R2", "section": "1", "at": {"page": 1}},
    {"id": "R3", "section": "1", "at": {"page": 1}},
    {"id": "R4", "section": "1", "at": {"page": 1}}
  ]
}"#;

/// R1 on the armed state, R2 on the transition into the alarm, R3 on the one that
/// silences it, which waits for an event a timer raises. R4 is cited by no node.
const ALARM: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="armed">
  <state id="armed" sce:req="R1">
    <transition event="trip" target="alarm" sce:req="R2"/>
  </state>
  <state id="alarm">
    <onentry><send event="silence" delay="3s"/></onentry>
    <transition event="silence" target="armed" sce:req="R3"/>
  </state>
</scxml>
"#;

const LOOKUP: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="lookup" name="severity">
  <datamodel>
    <data id="code" sce:type="uint8" sce:direction="in"/>
    <data id="severity" sce:type="string" sce:direction="out"/>
    <data id="mapping" sce:default="UNKNOWN">
      <sce:entry key="0" value="OK" sce:req="R1"/>
      <sce:entry key="1" value="WARN" sce:req="R2"/>
      <sce:entry key="2" value="FAIL"/>
    </data>
  </datamodel>
</scxml>
"#;

struct Tree {
    dir: tempfile::TempDir,
}

impl Tree {
    fn new(design: &str) -> Self {
        let dir = tempfile::tempdir().expect("tempdir");
        fs::write(dir.path().join("design.scxml"), design).expect("write");
        fs::write(dir.path().join("list.json"), MANIFEST).expect("write");
        Tree { dir }
    }

    fn root(&self) -> &Path {
        self.dir.path()
    }

    fn design(&self, text: &str) {
        fs::write(self.root().join("design.scxml"), text).expect("write");
    }

    fn take(&self) -> AcceptanceRecord {
        AcceptanceRecord::take(
            self.root(),
            &self.root().join("design.scxml"),
            &self.root().join("list.json"),
            "base",
        )
        .expect("the record is taken")
    }

    fn delta(&self, record: &AcceptanceRecord) -> EvidenceDelta {
        record
            .evidence_delta(&self.root().join("design.scxml"))
            .expect("the design is compared")
    }
}

fn change<'a>(delta: &'a EvidenceDelta, id: &str) -> &'a RequirementChange {
    delta
        .requirements
        .get(id)
        .unwrap_or_else(|| panic!("no line for {id}: {:?}", delta.requirements.keys()))
}

fn unchanged(delta: &EvidenceDelta) -> Vec<&str> {
    delta
        .requirements
        .iter()
        .filter(|(_, c)| **c == RequirementChange::Unchanged)
        .map(|(id, _)| id.as_str())
        .collect()
}

#[test]
fn the_design_as_it_was_accepted_reports_every_requirement_unchanged() {
    let tree = Tree::new(ALARM);
    let delta = tree.delta(&tree.take());
    assert_eq!(unchanged(&delta), vec!["R1", "R2", "R3"]);
    assert_eq!(
        delta.requirements.len(),
        3,
        "R4 is cited by no node and has no line"
    );
    assert!(
        delta.unclaimed_added.is_empty() && delta.unclaimed_gone == 0,
        "{delta:?}"
    );
}

#[test]
fn retiming_the_delay_reports_what_depends_on_it_and_nothing_else() {
    let tree = Tree::new(ALARM);
    let record = tree.take();
    tree.design(&ALARM.replace("delay=\"3s\"", "delay=\"5s\""));
    let delta = tree.delta(&record);
    assert_eq!(*change(&delta, "R1"), RequirementChange::Unchanged);
    for id in ["R2", "R3"] {
        match change(&delta, id) {
            RequirementChange::Changed { moved, gone } => {
                assert_eq!(moved.len(), 1, "{id}: {moved:?}");
                assert!(
                    moved[0].starts_with("states.alarm.on_entry_blocks"),
                    "{id} names {moved:?}, not the timer's send"
                );
                assert_eq!(*gone, 1, "{id}");
            }
            other => panic!("{id} should have changed, got {other:?}"),
        }
    }
    // The send claims nothing itself, so it is also the one unclaimed row that moved.
    assert_eq!(
        delta.unclaimed_added.len(),
        1,
        "{:?}",
        delta.unclaimed_added
    );
    assert_eq!(delta.unclaimed_gone, 1);
}

#[test]
fn a_transition_inserted_ahead_of_a_cited_one_names_the_place_its_row_has_now() {
    let tree = Tree::new(ALARM);
    let record = tree.take();
    tree.design(&ALARM.replace(
        "<transition event=\"trip\" target=\"alarm\" sce:req=\"R2\"/>",
        "<transition event=\"test\" target=\"armed\"/>\n    \
         <transition event=\"trip\" target=\"alarm\" sce:req=\"R2\"/>",
    ));
    let delta = tree.delta(&record);
    match change(&delta, "R2") {
        RequirementChange::Changed { moved, gone } => {
            assert!(
                moved.contains(&"states.armed.transitions[1]".to_string()),
                "R2 does not name where the cited transition is now: {moved:?}"
            );
            assert!(*gone >= 1, "{gone}");
        }
        other => panic!("R2 should have changed, got {other:?}"),
    }
    assert_eq!(*change(&delta, "R1"), RequirementChange::Unchanged);
    assert_eq!(*change(&delta, "R3"), RequirementChange::Unchanged);
}

#[test]
fn a_citation_taken_off_is_dropped_and_one_put_on_is_new_with_its_place() {
    let tree = Tree::new(ALARM);
    let record = tree.take();
    tree.design(&ALARM.replace(" sce:req=\"R3\"", "").replace(
        "<transition event=\"trip\" target=\"alarm\" sce:req=\"R2\"/>",
        "<transition event=\"trip\" target=\"alarm\" sce:req=\"R2 R4\"/>",
    ));
    let delta = tree.delta(&record);
    match change(&delta, "R3") {
        RequirementChange::Dropped { gone } => assert!(*gone >= 1, "{gone}"),
        other => panic!("R3 should be dropped, got {other:?}"),
    }
    match change(&delta, "R4") {
        // The cited transition, and the entry of the state it enters, which arms
        // the timer: the closure the report prints, not the citing node alone.
        RequirementChange::New { at } => assert_eq!(
            at,
            &vec![
                "states.alarm.on_entry_blocks[0][0]".to_string(),
                "states.armed.transitions[0]".to_string()
            ]
        ),
        other => panic!("R4 should be new, got {other:?}"),
    }
}

#[test]
fn a_row_taken_out_is_changed_with_nothing_moved_and_one_gone() {
    // The timer is removed. Nothing in the design is new for R2 or R3: what they
    // rested on is simply no longer there, and only a count can say so.
    let tree = Tree::new(ALARM);
    let record = tree.take();
    tree.design(&ALARM.replace(
        "<onentry><send event=\"silence\" delay=\"3s\"/></onentry>",
        "",
    ));
    let delta = tree.delta(&record);
    for id in ["R2", "R3"] {
        assert_eq!(
            *change(&delta, id),
            RequirementChange::Changed {
                moved: vec![],
                gone: 1
            },
            "{id}"
        );
    }
    assert_eq!(*change(&delta, "R1"), RequirementChange::Unchanged);
    assert!(
        delta.unclaimed_added.is_empty(),
        "{:?}",
        delta.unclaimed_added
    );
    assert_eq!(delta.unclaimed_gone, 1);
}

#[test]
fn a_state_no_requirement_reaches_reports_only_unclaimed_rows_added() {
    let tree = Tree::new(ALARM);
    let record = tree.take();
    tree.design(&ALARM.replace(
        "</scxml>",
        "  <state id=\"spare\"><transition event=\"poke\" target=\"spare\"/></state>\n</scxml>",
    ));
    let delta = tree.delta(&record);
    assert_eq!(unchanged(&delta), vec!["R1", "R2", "R3"]);
    assert!(
        delta
            .unclaimed_added
            .iter()
            .any(|at| at.starts_with("states.spare")),
        "{:?}",
        delta.unclaimed_added
    );
    assert_eq!(delta.unclaimed_gone, 0);
}

#[test]
fn a_lookups_edited_entry_reports_its_requirement_changed_with_the_entrys_place() {
    let tree = Tree::new(LOOKUP);
    let record = tree.take();
    tree.design(&LOOKUP.replace("value=\"OK\"", "value=\"GOOD\""));
    let delta = tree.delta(&record);
    match change(&delta, "R1") {
        RequirementChange::Changed { moved, gone } => {
            assert_eq!((moved.len(), *gone), (1, 1), "{moved:?}");
            assert!(
                moved[0].contains("key=0") || moved[0].contains('0'),
                "{moved:?}"
            );
        }
        other => panic!("R1 should have changed, got {other:?}"),
    }
    assert_eq!(*change(&delta, "R2"), RequirementChange::Unchanged);
}

#[test]
fn another_draft_is_compared_when_it_is_named() {
    let tree = Tree::new(ALARM);
    let record = tree.take();
    fs::write(
        tree.root().join("draft.scxml"),
        ALARM.replace("delay=\"3s\"", "delay=\"9s\""),
    )
    .expect("write");
    let delta = record
        .evidence_delta(&tree.root().join("draft.scxml"))
        .expect("the draft is compared");
    assert!(matches!(
        change(&delta, "R3"),
        RequirementChange::Changed { .. }
    ));
    assert_eq!(*change(&delta, "R1"), RequirementChange::Unchanged);
}

#[test]
fn what_cannot_be_compared_is_refused_and_not_read_as_everything_new() {
    let tree = Tree::new(ALARM);
    // A record from before records kept evidence: no evidence, nothing unclaimed.
    let mut old = serde_json::from_str::<serde_json::Value>(&tree.take().to_json()).expect("JSON");
    old.as_object_mut().expect("an object").remove("evidence");
    old.as_object_mut().expect("an object").remove("unclaimed");
    let old = AcceptanceRecord::from_json(&old.to_string()).expect("an old record loads");
    let refused = old.evidence_delta(&tree.root().join("design.scxml"));
    assert!(
        matches!(refused, Err(RecordError::Format { .. })),
        "{refused:?}"
    );
    assert!(refused
        .expect_err("refused")
        .to_string()
        .contains("states no evidence"));

    // A design that does not parse.
    fs::write(tree.root().join("broken.scxml"), "<scxml").expect("write");
    let broken = tree
        .take()
        .evidence_delta(&tree.root().join("broken.scxml"));
    assert!(broken.is_err(), "a design that does not parse was compared");
}

mod through_the_binary {
    use super::*;

    fn run(args: &[&str], cwd: &Path) -> Output {
        Command::new(CODEGEN)
            .arg("--error-format=json")
            .args(args)
            .current_dir(cwd)
            .output()
            .expect("spawn sce-codegen")
    }

    fn lines(out: &Output) -> Vec<serde_json::Value> {
        assert!(
            out.status.success(),
            "exited {:?}: {}",
            out.status.code(),
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .map(|line| serde_json::from_str(line).expect("NDJSON"))
            .collect()
    }

    fn accepted(tree: &Tree) -> PathBuf {
        let record = tree.root().join("record.json");
        let out = run(
            &[
                "accept",
                &tree.root().join("design.scxml").display().to_string(),
                "--manifest",
                &tree.root().join("list.json").display().to_string(),
                "--variant",
                "base",
                "--root",
                &tree.root().display().to_string(),
                "--out",
                &record.display().to_string(),
            ],
            tree.root(),
        );
        assert!(out.status.success(), "{out:?}");
        record
    }

    fn delta_lines(tree: &Tree, record: &Path) -> Vec<serde_json::Value> {
        lines(&run(
            &[
                "acceptance-delta",
                &record.display().to_string(),
                "--root",
                &tree.root().display().to_string(),
            ],
            tree.root(),
        ))
    }

    #[test]
    fn the_lines_are_the_ones_the_library_computes() {
        let tree = Tree::new(ALARM);
        let record = accepted(&tree);
        tree.design(&ALARM.replace("delay=\"3s\"", "delay=\"5s\""));
        let library = tree.delta(
            &AcceptanceRecord::from_json(&fs::read_to_string(&record).expect("read"))
                .expect("loads"),
        );
        let printed = delta_lines(&tree, &record);

        let by_requirement: BTreeMap<String, &serde_json::Value> = printed
            .iter()
            .filter(|l| l["kind"] == "acceptance-delta")
            .map(|l| (l["requirement"].as_str().expect("an id").to_string(), l))
            .collect();
        assert_eq!(by_requirement.len(), library.requirements.len());
        assert_eq!(by_requirement["R1"]["evidence"], "unchanged");
        assert_eq!(by_requirement["R3"]["evidence"], "changed");
        assert_eq!(by_requirement["R3"]["gone"], 1);
        assert_eq!(
            by_requirement["R3"]["moved"][0],
            match change(&library, "R3") {
                RequirementChange::Changed { moved, .. } => moved[0].as_str(),
                other => panic!("{other:?}"),
            }
        );
        let unclaimed = printed
            .iter()
            .find(|l| l["kind"] == "acceptance-delta-unclaimed")
            .expect("an unclaimed line");
        assert_eq!(unclaimed["gone"], library.unclaimed_gone);
        let summary = printed.last().expect("a summary");
        assert_eq!(summary["kind"], "acceptance-delta-summary");
        assert_eq!(
            (
                summary["unchanged"].as_u64(),
                summary["changed"].as_u64(),
                summary["new"].as_u64(),
                summary["dropped"].as_u64()
            ),
            (Some(1), Some(2), Some(0), Some(0))
        );
        assert!(printed.iter().all(|l| l["v"] == 1));
    }

    #[test]
    fn it_is_a_report_so_a_changed_design_still_exits_zero() {
        let tree = Tree::new(ALARM);
        let record = accepted(&tree);
        tree.design(&ALARM.replace("delay=\"3s\"", "delay=\"5s\""));
        let out = run(
            &[
                "acceptance-delta",
                &record.display().to_string(),
                "--root",
                &tree.root().display().to_string(),
            ],
            tree.root(),
        );
        assert_eq!(out.status.code(), Some(0), "{out:?}");
    }

    #[test]
    fn a_record_that_cannot_be_compared_exits_with_the_unusable_input_code() {
        let tree = Tree::new(ALARM);
        fs::write(tree.root().join("not.json"), "{}\n").expect("write");
        let out = run(
            &[
                "acceptance-delta",
                &tree.root().join("not.json").display().to_string(),
                "--root",
                &tree.root().display().to_string(),
            ],
            tree.root(),
        );
        assert_eq!(out.status.code(), Some(20), "{out:?}");
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(stderr.contains("cli/closure-input-unusable"), "{stderr}");
    }

    #[test]
    fn another_draft_is_named_with_design() {
        let tree = Tree::new(ALARM);
        let record = accepted(&tree);
        fs::write(
            tree.root().join("draft.scxml"),
            ALARM.replace("delay=\"3s\"", "delay=\"9s\""),
        )
        .expect("write");
        let printed = lines(&run(
            &[
                "acceptance-delta",
                &record.display().to_string(),
                "--root",
                &tree.root().display().to_string(),
                "--design",
                &tree.root().join("draft.scxml").display().to_string(),
            ],
            tree.root(),
        ));
        let summary = printed.last().expect("a summary");
        assert_eq!(summary["changed"], 2);
    }
}

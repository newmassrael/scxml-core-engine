// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! An acceptance keeps, per requirement, what the owner was shown for it.
//!
//! A record that pins bytes can say only that something moved. The acceptance
//! report puts in front of the owner, for each requirement, the rows of its
//! dependency closure (`acceptance_report::fragment`) and not the nodes that
//! carry the id, so that retiming a delay moves the page. The record keeps those
//! rows as digests, so that the NEXT acceptance can be offered as a difference
//! (`docs/adr/0007-an-acceptance-keeps-what-each-requirement-rests-on.md`).
//!
//! Held here, on a small alarm with one timer and on a lookup:
//!
//!   a record taken twice over the same files is the same bytes
//!   a requirement's evidence is its fragment (the arming `<send>`, the target's
//!       entry), not only the node that cites it
//!   retiming the delay moves the evidence of the requirements that depend on it
//!       and not of the one that does not
//!   a state no requirement reaches adds unclaimed rows and moves no evidence
//!   a transition inserted ahead of a cited one in its state moves that
//!       requirement's evidence (order is behaviour, so position is in the digest)
//!   a requirement no node cites has no key; a design that cites nothing has no
//!       evidence and every row unclaimed
//!   a lookup's entries are the rows, and editing one moves only its requirement
//!   a record from before any of this reads, and writes back, as it was; a digest
//!       that is not a sha256, a list out of order, an empty id or a field this
//!       module does not know is refused
//!   a record can name the one it replaces, which has to be a record for the same
//!       specification, and that cannot lapse it

use std::fs;
use std::path::Path;

use sce_build::acceptance_record::{AcceptanceRecord, RecordError};

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

/// R1 is on the armed state, R2 on the transition into the alarm, R3 on the
/// transition that silences it, which waits for an event a timer raises. R4 is
/// cited by no node.
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
}

fn digests<'a>(record: &'a AcceptanceRecord, id: &str) -> &'a Vec<String> {
    record
        .evidence
        .get(id)
        .unwrap_or_else(|| panic!("no evidence for {id}: {:?}", record.evidence.keys()))
}

#[test]
fn a_record_taken_twice_over_the_same_files_is_the_same_bytes() {
    let tree = Tree::new(ALARM);
    assert_eq!(tree.take().to_json(), tree.take().to_json());
}

#[test]
fn a_requirements_evidence_is_its_fragment_and_not_only_the_node_that_cites_it() {
    let record = Tree::new(ALARM).take();
    // R1 is cited by one node and depends on nothing else.
    assert_eq!(digests(&record, "R1").len(), 1, "{:?}", record.evidence);
    // R3 is cited by one transition; its fragment also holds the `<send>` that
    // arms it, the target's entry and the source's exit, so more than one row.
    assert!(
        digests(&record, "R3").len() > 1,
        "the evidence of R3 is only the node that cites it: {:?}",
        record.evidence
    );
    // R2 enters the alarm, whose entry arms the timer: the timer is in R2's too.
    assert!(digests(&record, "R2").len() > 1, "{:?}", record.evidence);
    // And every list is in order, which is how the wire reads them back.
    for list in record
        .evidence
        .values()
        .chain(std::iter::once(&record.unclaimed))
    {
        assert!(list.windows(2).all(|pair| pair[0] <= pair[1]), "{list:?}");
    }
}

#[test]
fn a_requirement_no_node_cites_has_no_key() {
    let record = Tree::new(ALARM).take();
    assert!(!record.evidence.contains_key("R4"), "{:?}", record.evidence);
    assert_eq!(
        record
            .evidence
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        vec!["R1", "R2", "R3"]
    );
}

#[test]
fn retiming_the_delay_moves_the_evidence_of_what_depends_on_it_and_no_other() {
    let tree = Tree::new(ALARM);
    let before = tree.take();
    tree.design(&ALARM.replace("delay=\"3s\"", "delay=\"5s\""));
    let after = tree.take();
    assert_eq!(
        digests(&before, "R1"),
        digests(&after, "R1"),
        "R1 does not depend on the timer and its evidence moved"
    );
    for id in ["R2", "R3"] {
        assert_ne!(
            digests(&before, id),
            digests(&after, id),
            "{id} depends on the timer and its evidence did not move"
        );
    }
}

#[test]
fn a_state_no_requirement_reaches_adds_unclaimed_rows_and_moves_no_evidence() {
    let tree = Tree::new(ALARM);
    let before = tree.take();
    tree.design(&ALARM.replace(
        "</scxml>",
        "  <state id=\"spare\"><transition event=\"poke\" target=\"spare\"/></state>\n</scxml>",
    ));
    let after = tree.take();
    assert_eq!(before.evidence, after.evidence);
    assert!(
        after.unclaimed.len() > before.unclaimed.len(),
        "{:?} then {:?}",
        before.unclaimed,
        after.unclaimed
    );
}

#[test]
fn a_transition_inserted_ahead_of_a_cited_one_moves_its_evidence_because_order_is_behaviour() {
    let tree = Tree::new(ALARM);
    let before = tree.take();
    tree.design(&ALARM.replace(
        "<transition event=\"trip\" target=\"alarm\" sce:req=\"R2\"/>",
        "<transition event=\"test\" target=\"armed\"/>\n    \
         <transition event=\"trip\" target=\"alarm\" sce:req=\"R2\"/>",
    ));
    let after = tree.take();
    assert_ne!(
        digests(&before, "R2"),
        digests(&after, "R2"),
        "the cited transition is now second in its state and its evidence did not say so"
    );
    // The state's own row and R1 do not depend on that order.
    assert_eq!(digests(&before, "R1"), digests(&after, "R1"));
}

#[test]
fn a_design_that_cites_nothing_has_no_evidence_and_every_row_unclaimed() {
    let tree = Tree::new(
        &ALARM
            .replace(" sce:req=\"R1\"", "")
            .replace(" sce:req=\"R2\"", "")
            .replace(" sce:req=\"R3\"", ""),
    );
    let record = tree.take();
    assert!(record.evidence.is_empty(), "{:?}", record.evidence);
    assert!(!record.unclaimed.is_empty());
}

#[test]
fn a_lookups_entries_are_its_rows_and_editing_one_moves_only_its_requirement() {
    let tree = Tree::new(LOOKUP);
    let before = tree.take();
    assert_eq!(digests(&before, "R1").len(), 1, "{:?}", before.evidence);
    assert_eq!(digests(&before, "R2").len(), 1, "{:?}", before.evidence);
    assert!(
        !before.unclaimed.is_empty(),
        "the FAIL entry claims nothing"
    );
    tree.design(&LOOKUP.replace("value=\"OK\"", "value=\"GOOD\""));
    let after = tree.take();
    assert_ne!(digests(&before, "R1"), digests(&after, "R1"));
    assert_eq!(digests(&before, "R2"), digests(&after, "R2"));
    assert_eq!(before.unclaimed, after.unclaimed);
}

#[test]
fn the_evidence_cannot_lapse_the_acceptance_but_the_design_still_does() {
    let tree = Tree::new(ALARM);
    let record = tree.take();
    assert!(record
        .recheck(tree.root(), "base")
        .expect("rechecks")
        .is_empty());
    tree.design(&ALARM.replace("delay=\"3s\"", "delay=\"5s\""));
    assert!(
        !record
            .recheck(tree.root(), "base")
            .expect("rechecks")
            .is_empty(),
        "the design moved and the acceptance went on holding"
    );
}

mod on_the_wire {
    use super::*;

    const SHA: &str = "0000000000000000000000000000000000000000000000000000000000000000";

    fn old_record() -> serde_json::Value {
        serde_json::json!({
            "record": "sce-acceptance-record", "v": 1, "variant": "base",
            "document": "design.scxml",
            "manifest": {"path": "list.json", "doc_id": "alarm", "rev": "1", "sha256": SHA},
            "inputs": [{"path": "design.scxml", "sha256": SHA}],
        })
    }

    fn with(edit: impl FnOnce(&mut serde_json::Value)) -> String {
        let mut value = old_record();
        edit(&mut value);
        value.to_string()
    }

    #[test]
    fn a_record_from_before_the_evidence_existed_reads_and_writes_back_as_it_was() {
        let loaded = AcceptanceRecord::from_json(&old_record().to_string()).expect("loads");
        assert!(
            loaded.evidence.is_empty() && loaded.unclaimed.is_empty() && loaded.succeeds.is_none()
        );
        let written: serde_json::Value = serde_json::from_str(&loaded.to_json()).expect("JSON");
        assert_eq!(written, old_record(), "an old record gained a field");
    }

    #[test]
    fn a_record_that_has_evidence_round_trips() {
        let record = Tree::new(ALARM).take();
        let again = AcceptanceRecord::from_json(&record.to_json()).expect("loads");
        assert_eq!(again, record);
    }

    #[test]
    fn what_take_would_not_write_is_refused() {
        let two = "1".repeat(64);
        let cases: Vec<(&str, String)> = vec![
            (
                "a short digest",
                with(|v| v["evidence"] = serde_json::json!({"R1": ["abc"]})),
            ),
            (
                "an upper-case digest",
                with(|v| v["unclaimed"] = serde_json::json!(["A".repeat(64)])),
            ),
            (
                "a list out of order",
                with(|v| v["evidence"] = serde_json::json!({"R1": [two.clone(), SHA]})),
            ),
            (
                "an unclaimed list out of order",
                with(|v| v["unclaimed"] = serde_json::json!([two.clone(), SHA])),
            ),
            (
                "an empty requirement id",
                with(|v| v["evidence"] = serde_json::json!({"": [SHA]})),
            ),
            (
                "an unknown field",
                with(|v| v["evidence_of"] = serde_json::json!({})),
            ),
            (
                "a predecessor that climbs",
                with(|v| v["succeeds"] = serde_json::json!({"path": "../old.json", "sha256": SHA})),
            ),
            (
                "a predecessor with a short digest",
                with(|v| v["succeeds"] = serde_json::json!({"path": "old.json", "sha256": "abc"})),
            ),
            (
                "a predecessor with another field",
                with(|v| {
                    v["succeeds"] =
                        serde_json::json!({"path": "old.json", "sha256": SHA, "why": "x"})
                }),
            ),
        ];
        for (what, bytes) in cases {
            assert!(
                AcceptanceRecord::from_json(&bytes).is_err(),
                "{what} was loaded as a record: {bytes}"
            );
        }
        // The control: the same records, well formed, load.
        let ok = with(|v| {
            v["evidence"] = serde_json::json!({"R1": [SHA, two.clone()]});
            v["unclaimed"] = serde_json::json!([SHA]);
            v["succeeds"] = serde_json::json!({"path": "old.json", "sha256": SHA});
        });
        assert!(AcceptanceRecord::from_json(&ok).is_ok(), "{ok}");
    }
}

mod succession {
    use super::*;

    #[test]
    fn a_record_can_name_the_one_it_replaces_and_that_cannot_lapse_it() {
        let tree = Tree::new(ALARM);
        let first = tree.take();
        fs::write(tree.root().join("first.json"), first.to_json()).expect("write");
        let second = tree
            .take()
            .succeeding(tree.root(), &tree.root().join("first.json"))
            .expect("a record for the same specification can be replaced");
        let pin = second
            .succeeds
            .clone()
            .expect("the successor names its predecessor");
        assert_eq!(pin.path, "first.json");
        assert_eq!(pin.sha256.len(), 64);
        let again = AcceptanceRecord::from_json(&second.to_json()).expect("round-trips");
        assert_eq!(again, second);
        assert!(second
            .recheck(tree.root(), "base")
            .expect("rechecks")
            .is_empty());
        // Deleting the predecessor does not lapse an acceptance of the design.
        fs::remove_file(tree.root().join("first.json")).expect("remove");
        assert!(second
            .recheck(tree.root(), "base")
            .expect("rechecks")
            .is_empty());
    }

    #[test]
    fn a_file_that_is_not_a_record_is_refused() {
        let tree = Tree::new(ALARM);
        fs::write(tree.root().join("not.json"), "{}\n").expect("write");
        let refused = tree
            .take()
            .succeeding(tree.root(), &tree.root().join("not.json"));
        assert!(
            matches!(refused, Err(RecordError::Format { .. })),
            "{refused:?}"
        );
    }

    #[test]
    fn a_record_of_another_specification_is_refused() {
        let tree = Tree::new(ALARM);
        let foreign = tree
            .take()
            .to_json()
            .replace("\"doc_id\": \"alarm\"", "\"doc_id\": \"elsewhere\"");
        fs::write(tree.root().join("foreign.json"), foreign).expect("write");
        let refused = tree
            .take()
            .succeeding(tree.root(), &tree.root().join("foreign.json"));
        assert!(
            matches!(refused, Err(RecordError::Format { .. })),
            "{refused:?}"
        );
        let message = refused.expect_err("refused").to_string();
        assert!(
            message.contains("elsewhere") && message.contains("alarm"),
            "{message}"
        );
    }

    #[test]
    fn a_predecessor_outside_the_root_is_refused() {
        let tree = Tree::new(ALARM);
        let outside = tempfile::tempdir().expect("tempdir");
        fs::write(outside.path().join("far.json"), tree.take().to_json()).expect("write");
        let refused = tree
            .take()
            .succeeding(tree.root(), &outside.path().join("far.json"));
        assert!(
            matches!(refused, Err(RecordError::OutsideRoot { .. })),
            "{refused:?}"
        );
    }
}

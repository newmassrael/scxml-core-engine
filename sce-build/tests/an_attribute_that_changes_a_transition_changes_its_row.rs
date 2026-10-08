// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! An attribute that changes what a transition or a state does changes that
//! node's row in the transition table.
//!
//! `an_attribute_that_changes_an_action_changes_its_row` held the rows of
//! executable content and said in its header that a transition's own
//! attributes were "not yet in the table at all". One of them is `type`:
//! `external` and `internal` differ in whether a transition to a descendant
//! leaves its source state, and so in whether that state's `<onentry>` runs
//! again (§scxml-3.13). Measured before this file existed, the rows of a
//! transition written `external` and the same transition written `internal`
//! were byte-identical, and the acceptance record's evidence is a digest of
//! these rows (`acceptance_record::statechart_row_digest`), so
//! `acceptance-delta` called the requirement `unchanged` and a revision check
//! carried it over. A review of the revision tools found it with a counter in
//! the source's `<onentry>`, which it reported as 2 after one such transition
//! when `external` and 1 when `internal`.
//!
//! The oracle is the one the action test uses (`common::row_oracle`): nothing
//! is listed. Every attribute of every `<transition>` of the fixture, and every
//! attribute but the id of every `<state>`, `<parallel>`, `<final>` and
//! `<history>`, is moved one at a time; the parsed model of the node is
//! compared before and after; if it moved, the row has to have moved.
//!
//! ⚠ Scope, stated rather than implied: the rows held are the rows of
//! transitions and of states. The `<transition>` inside an `<initial>` or a
//! `<history>` is an action row and is not mutated here. An `id` is not moved:
//! renaming a state moves every reference to it, so the model of other nodes
//! moves with it and the rule would be asked about nodes the mutation did not
//! touch.

mod common;

use common::row_oracle::{at, parse, parse_fixture, sites, SCXML_NS};
use sce_build::transition_table::{transition_table, TransitionRow};

const DOC: &str = r##"<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" name="cells" initial="a" datamodel="ecmascript">
  <datamodel>
    <data id="v" expr="0"/>
  </datamodel>
  <state id="a" initial="a1">
    <onentry>
      <assign location="v" expr="v + 1"/>
    </onentry>
    <transition event="go" cond="v == 0" target="b" type="external"/>
    <transition event="in" cond="v == 1" target="a2" type="internal"/>
    <transition event="stay"/>
    <transition target="a2" type="internal"/>
    <state id="a1"/>
    <state id="a2"/>
  </state>
  <state id="b" initial="b1">
    <transition event="back" target="a"/>
    <state id="b1"/>
    <history id="bh" type="deep">
      <transition target="b1"/>
    </history>
  </state>
  <parallel id="p">
    <state id="p1"/>
    <state id="p2"/>
  </parallel>
  <final id="f"/>
</scxml>"##;

/// A transition's serialised form with what is not behaviour removed: annotations,
/// the source coordinate, and the actions, which have rows of their own.
fn transition_behaviour(transition: &serde_json::Value) -> serde_json::Value {
    let mut transition = transition.clone();
    if let Some(members) = transition.as_object_mut() {
        for key in [
            "req",
            "provenance",
            "unresolved",
            "source_location",
            "actions",
        ] {
            members.remove(key);
        }
    }
    transition
}

/// A state's own data: its scalars and its lists of scalars. What it holds, the
/// states and transitions and blocks inside it, has rows of its own.
fn state_behaviour(state: &serde_json::Value) -> serde_json::Value {
    let mut own = serde_json::Map::new();
    if let Some(members) = state.as_object() {
        for (key, value) in members {
            let scalar = |v: &serde_json::Value| !v.is_object() && !v.is_array();
            let own_data = match value {
                serde_json::Value::Array(items) => items.iter().all(scalar),
                other => scalar(other),
            };
            if own_data && !matches!(key.as_str(), "source_location" | "req") {
                own.insert(key.clone(), value.clone());
            }
        }
    }
    serde_json::Value::Object(own)
}

/// A transition, as opposed to a state or an action: the model serialises its
/// `transition_type` as `type`, and it alone has both an `event` and a `cond` and
/// no `action_type`.
fn is_transition(model: &serde_json::Value) -> bool {
    model.get("type").is_some()
        && model.get("event").is_some()
        && model.get("cond").is_some()
        && model.get("action_type").is_none()
}

fn is_state_row(row: &TransitionRow) -> bool {
    row.event == "(state)"
}

fn in_scxml(node: &roxmltree::Node, names: &[&str]) -> bool {
    node.tag_name().namespace() == Some(SCXML_NS) && names.contains(&node.tag_name().name())
}

/// Moves every attribute `keep` accepts, one at a time, and returns what hid:
/// how many moves changed the node's model, how many changed none, how many
/// stopped the document parsing, and the rows that did not move with the model.
///
/// `holds` says which rows are held, given the node's serialised model, and
/// `behaviour` reads what is behaviour out of it.
fn sweep(
    keep: impl Fn(&roxmltree::Node, &str) -> bool,
    holds: impl Fn(&TransitionRow, &serde_json::Value) -> bool,
    behaviour: impl Fn(&serde_json::Value) -> serde_json::Value,
) -> (usize, usize, usize, Vec<String>) {
    let original = parse_fixture(DOC);
    let original_tree = serde_json::to_value(&original).expect("the model serialises");
    let original_rows = transition_table(&original);

    let (mut effective, mut inert, mut unparsable) = (0usize, 0usize, 0usize);
    let mut hidden = Vec::new();
    for site in sites(DOC, keep) {
        let mut mutated = DOC.to_string();
        mutated.insert(site.range.end, 'M');
        let Some(model) = parse(&mutated) else {
            unparsable += 1;
            continue;
        };
        let tree = serde_json::to_value(&model).expect("the model serialises");
        let rows = transition_table(&model);
        assert_eq!(
            rows.len(),
            original_rows.len(),
            "{}: mutating an attribute value changed the number of rows, so rows \
             cannot be lined up by position",
            site.what,
        );
        let mut moved_model = false;
        for (before, after) in original_rows.iter().zip(&rows) {
            assert_eq!(before.node_path, after.node_path, "rows must line up");
            let Some(model_before) = at(&original_tree, &before.node_path) else {
                continue;
            };
            if !holds(before, model_before) {
                continue;
            }
            let model_after = at(&tree, &after.node_path);
            if Some(behaviour(model_before)) != model_after.map(&behaviour) {
                moved_model = true;
                if before == after {
                    hidden.push(format!(
                        "  {} changed {} but its row did not move:\n    {:?}",
                        site.what, before.node_path, before
                    ));
                }
            }
        }
        if moved_model {
            effective += 1;
        } else {
            inert += 1;
        }
    }
    (effective, inert, unparsable, hidden)
}

/// The cell a transition that is not `internal` had before the cell learned `type`,
/// byte for byte.
///
/// The parser reads an absent `type` as `external`, so no pair of documents can tell
/// "left out" from "spelled out" and the sweep above cannot hold this. A record taken
/// of a document carries a digest of its rows: if the default were written into the
/// cell, every record already taken would read every transition as moved.
#[test]
fn a_transition_that_is_not_internal_keeps_the_cell_it_always_had() {
    let rows = transition_table(&parse_fixture(DOC));
    let to = |event: &str| -> Vec<&str> {
        rows.iter()
            .filter(|row| row.event == event)
            .map(|row| row.to.as_str())
            .collect()
    };
    assert_eq!(to("go"), ["b"], "written `external`");
    assert_eq!(to("back"), ["a"], "no `type`");
    assert_eq!(to("stay"), ["-"], "no target and no `type`");
    assert_eq!(to("in"), ["a2 type=internal"]);
    assert_eq!(to("(eventless)"), ["a2 type=internal"]);
}

/// ⚠ The rule, over every attribute of every transition.
#[test]
fn an_attribute_that_changes_a_transition_changes_its_row() {
    const FLOOR: usize = 10;
    let (effective, inert, unparsable, hidden) = sweep(
        |node, _| {
            in_scxml(node, &["transition"])
                && !node
                    .parent()
                    .is_some_and(|p| in_scxml(&p, &["history", "initial"]))
        },
        |_, model| is_transition(model),
        transition_behaviour,
    );
    println!(
        "mutated the attributes of the transitions: {effective} changed a transition's \
         model, {inert} changed none, {unparsable} stopped the document parsing; {} \
         hidden from the table",
        hidden.len(),
    );
    assert!(
        hidden.is_empty(),
        "an attribute changed what a transition does and its table row stayed \
         byte-identical — the acceptance record's evidence is a digest of these rows, \
         so a revision would carry the requirement over unchanged:\n{}",
        hidden.join("\n"),
    );
    assert!(
        effective >= FLOOR,
        "only {effective} mutation(s) changed a transition's model; floor {FLOOR}. \
         Either the fixture lost its attributes or mutations stopped parsing \
         ({unparsable} did)",
    );
}

/// ⚠ The rule, over every attribute but the id of every state.
#[test]
fn an_attribute_that_changes_a_state_changes_its_row() {
    const FLOOR: usize = 2;
    let (effective, inert, unparsable, hidden) = sweep(
        |node, attribute| {
            attribute != "id" && in_scxml(node, &["state", "parallel", "final", "history"])
        },
        |row, _| is_state_row(row),
        state_behaviour,
    );
    println!(
        "mutated the attributes of the states: {effective} changed a state's model, {inert} \
         changed none, {unparsable} stopped the document parsing; {} hidden from the table",
        hidden.len(),
    );
    assert!(
        hidden.is_empty(),
        "an attribute changed what a state is and its table row stayed byte-identical:\n{}",
        hidden.join("\n"),
    );
    assert!(
        effective >= FLOOR,
        "only {effective} mutation(s) changed a state's model; floor {FLOOR} ({unparsable} \
         stopped the document parsing)",
    );
}

// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! An attribute that changes what an action does changes that action's
//! row in the transition table.
//!
//! Requirement-closure RFC §12, Atomic C, question 1. The acceptance
//! report's fragment has to move under every mutation that violates a
//! requirement (§7a.1), and the table is the half of that report a
//! reviewer can diff. Measured before this file existed: the `action`
//! cell carried the action's KIND and nothing else —
//! `action: action.action_type.clone()` — so renaming the event a
//! `<send>` raises, or the `sendid` a `<cancel>` names, left the row
//! byte-identical. On the real ISO 13400-2 statechart that is exactly
//! how an inactivity timer comes apart from the transition that is
//! waiting for it, and §6.1's "exhaustive — nothing can hide" was false
//! for timers.
//!
//! # ⭐ Why the oracle is the model, and the population the document
//!
//! A test that listed "the attributes that matter" would be the defect
//! it guards against, written a second time: the cell was short because
//! somebody chose what to print. So nothing here is listed.
//!
//! - **What is mutated** is every attribute of every element inside an
//!   `<onentry>`, `<onexit>` or `<transition>` — derived from the
//!   document's structure, not from a roster of action names. `sce:`
//!   annotations are excluded: they reach the table through `source`,
//!   which other files hold.
//! - **What counts as a change** is the parsed model. For every action
//!   row, the action's serialised form is compared before and after, with
//!   only the fields that are not behaviour stripped — annotations, the
//!   source coordinate, and the nested action arrays, whose actions have
//!   rows of their own.
//! - **The rule** is one line: if the model of an action moved, its row
//!   moved. An attribute the parser ignores changes no model and asks
//!   nothing of the table, which is the right answer, and it is counted
//!   as inert rather than silently dropped.
//!
//! ⚠ Scope, stated rather than implied: this holds action rows. A
//! transition's own attributes (`type`, for one) and a state's are not
//! yet in the table at all, and are residue for the rounds after this.

use std::collections::BTreeSet;

use sce_build::model::SCXMLModel;
use sce_build::parser::SCXMLParser;
use sce_build::transition_table::{transition_table, TransitionRow};

/// Every kind of executable content, with the attributes each accepts.
///
/// `ecmascript` so every expression attribute is admissible; nothing
/// here is evaluated, only parsed.
///
/// `r##` because the document itself contains `"#` (`target="#_internal"`),
/// which would close a single-hash raw string.
const DOC: &str = r##"<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" name="cells" initial="s" datamodel="ecmascript">
  <datamodel>
    <data id="v" expr="0"/>
    <data id="items" expr="[1, 2]"/>
  </datamodel>
  <state id="s">
    <onentry>
      <raise event="raised"/>
      <send event="armed" id="timer" target="#_internal"
            type="http://www.w3.org/TR/scxml/#SCXMLEventProcessor"
            delay="1s" namelist="v">
        <param name="p" expr="1"/>
      </send>
      <send eventexpr="'dynamic'" idlocation="v" targetexpr="'#_internal'"
            delayexpr="'2s'">
        <content expr="v"/>
      </send>
      <cancel sendid="timer"/>
      <cancel sendidexpr="'timer'"/>
      <assign location="v" expr="1"/>
      <log label="note" expr="v"/>
      <foreach array="items" item="it" index="ix">
        <raise event="each"/>
      </foreach>
      <if cond="v == 0">
        <raise event="then_ev"/>
      <elseif cond="v == 1"/>
        <raise event="elseif_ev"/>
      <else/>
        <raise event="else_ev"/>
      </if>
    </onentry>
    <onexit>
      <log label="leaving" expr="v"/>
    </onexit>
    <transition event="armed" target="t">
      <raise event="went"/>
      <sce:action name="notify">
        <sce:arg name="a" expr="1"/>
      </sce:action>
    </transition>
  </state>
  <state id="t"/>
</scxml>"##;

const SCXML_NS: &str = "http://www.w3.org/2005/07/scxml";
const SCE_NS: &str = "http://sce.dev/ext";

fn parse(text: &str) -> Option<SCXMLModel> {
    SCXMLParser::new().parse_string(text, "cells").ok()
}

/// One attribute to mutate: its byte range in [`DOC`] and what it names.
struct Site {
    range: std::ops::Range<usize>,
    what: String,
}

/// Every attribute of every element inside executable content.
///
/// Read from the document with a real XML parser, so the population is
/// what the author wrote rather than what anyone thought to list.
fn sites() -> Vec<Site> {
    let doc = roxmltree::Document::parse(DOC).expect("the fixture is well-formed XML");
    let mut out = Vec::new();
    for node in doc.descendants().filter(|n| n.is_element()) {
        let inside_content = node.ancestors().skip(1).any(|a| {
            a.tag_name().namespace() == Some(SCXML_NS)
                && matches!(a.tag_name().name(), "onentry" | "onexit" | "transition")
        });
        if !inside_content {
            continue;
        }
        for attr in node.attributes() {
            if attr.namespace() == Some(SCE_NS) {
                continue;
            }
            out.push(Site {
                range: attr.range_value(),
                what: format!("<{} {}>", node.tag_name().name(), attr.name()),
            });
        }
    }
    out
}

/// The value of a JSON tree at a `node_path` such as
/// `states.s.on_entry_blocks[0][3].then_actions[0]`.
///
/// Walk paths name the serialised model's own fields, which is what lets
/// a row be lined up with the model it came from.
fn at<'a>(tree: &'a serde_json::Value, node_path: &str) -> Option<&'a serde_json::Value> {
    let mut cursor = tree;
    for segment in node_path.split('.') {
        let (key, indices) = match segment.find('[') {
            Some(open) => (&segment[..open], &segment[open..]),
            None => (segment, ""),
        };
        if !key.is_empty() {
            cursor = cursor.get(key)?;
        }
        for index in indices.split(['[', ']']).filter(|s| !s.is_empty()) {
            cursor = cursor.get(index.parse::<usize>().ok()?)?;
        }
    }
    Some(cursor)
}

/// An action's serialised form with what is not behaviour removed.
fn behaviour(action: &serde_json::Value) -> serde_json::Value {
    let mut action = action.clone();
    if let Some(members) = action.as_object_mut() {
        for key in [
            "req",
            "provenance",
            "unresolved",
            "source_location",
            "actions",
            "then_actions",
            "else_actions",
        ] {
            members.remove(key);
        }
        if let Some(serde_json::Value::Array(branches)) = members.get_mut("elseif_branches") {
            for branch in branches {
                if let Some(branch) = branch.as_object_mut() {
                    branch.remove("actions");
                }
            }
        }
        if let Some(serde_json::Value::Array(params)) = members.get_mut("params") {
            for param in params {
                if let Some(param) = param.as_object_mut() {
                    param.remove("source_location");
                }
            }
        }
    }
    action
}

fn is_action_row(row: &TransitionRow) -> bool {
    matches!(
        row.event.as_str(),
        "(entry)" | "(exit)" | "(transition)" | "(initial)" | "(history)"
    )
}

/// ⚠ The rule, over every attribute inside executable content.
#[test]
fn an_attribute_that_changes_an_action_changes_its_row() {
    const FLOOR: usize = 30;

    let original = parse(DOC).expect("the fixture parses");
    let original_tree = serde_json::to_value(&original).expect("the model serialises");
    let original_rows = transition_table(&original);

    let sites = sites();
    let (mut effective, mut inert, mut unparsable) = (0usize, 0usize, 0usize);
    let mut hidden: Vec<String> = Vec::new();

    for site in &sites {
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
            if !is_action_row(before) {
                continue;
            }
            assert_eq!(before.node_path, after.node_path, "rows must line up");
            let model_before = at(&original_tree, &before.node_path).map(behaviour);
            let model_after = at(&tree, &after.node_path).map(behaviour);
            assert!(
                model_before.is_some(),
                "{} does not resolve in the serialised model; the walk's paths \
                 and the model's fields have drifted",
                before.node_path,
            );
            if model_before != model_after {
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

    let kinds: BTreeSet<&str> = original_rows
        .iter()
        .filter(|row| is_action_row(row))
        .map(|row| row.action.split(' ').next().unwrap_or(""))
        .collect();
    println!(
        "mutated {} attribute(s) inside executable content: {effective} changed an \
         action's model, {inert} changed none, {unparsable} stopped the document \
         parsing; {} hidden from the table; action kinds with rows: {kinds:?}",
        sites.len(),
        hidden.len(),
    );
    assert!(
        hidden.is_empty(),
        "an attribute changed what an action does and its table row stayed \
         byte-identical — a reviewer diffing the table, or the acceptance report \
         built on it, cannot see that change however carefully they read:\n{}",
        hidden.join("\n"),
    );
    assert!(
        effective >= FLOOR,
        "only {effective} mutation(s) changed an action's model; floor {FLOOR}. \
         Either the fixture lost its attributes or mutations stopped parsing \
         ({unparsable} did)",
    );
}

// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! An annotation the parsed model holds is one every reader of
//! annotations reaches — wherever in the executable content it sits.
//!
//! Found while answering Atomic C's first question (RFC §12). The
//! acceptance report's block A must draw a requirement's dependencies,
//! and those include actions nested inside `<if>` and `<foreach>`. Asking
//! whether the shared walk reaches them turned up something wider: the
//! model has no traversal of executable content at all, so every reader
//! writes its own and chooses its own depth. Measured 2026-09-13:
//!
//! ```text
//!   reader                         transition's   nested if /      foreach   initial /
//!                                  own actions    elseif / else    body      history default
//!   walk_nodes (report, table,     yes            no               no        no
//!     manifest classification)
//!   first_unresolved (the          NO             no               no        no
//!     --strict-unresolved gate)
//!   emit_unresolved_ndjson         NO             no               no        no
//!   inherit_req / _provenance      -              no               no        -
//! ```
//!
//! ⚠⚠ The second row is a gate that passes when it should refuse. The
//! parser records `sce:unresolved` on EVERY action it builds, nested or
//! not, so an author's honest "I did not know" on a transition's own
//! action reaches the IR — and `--strict-unresolved` lets the build
//! through, while the manifest classification, which does reach that
//! action, reports the same requirement `unresolved`. Two readings of
//! one marker, one of them a gate, disagreeing in the unsafe direction.
//!
//! # ⭐ Why the population is read from the model and not listed
//!
//! Each reader above is a hand-written list of containers, and each is
//! short by a different amount — which is exactly how a list goes wrong.
//! A test that listed containers too would inherit the same blindness
//! for whichever container it forgot. So the oracle here is the parsed
//! model itself, serialised: every non-empty `unresolved` and `req`
//! array anywhere in that tree is a marker the model HOLDS, and every
//! reader must reach every one. A container added to the model later
//! joins the population on the day it exists.
//!
//! The document below still has to place markers somewhere, and a
//! container it never uses is not exercised. That residue is narrower
//! than a listed population's, and it is reported rather than hidden:
//! each test prints how many distinct markers it swept and asserts a
//! floor.

use std::collections::BTreeSet;

use sce_build::model::SCXMLModel;
use sce_build::parser::SCXMLParser;
use sce_build::requirement_manifest::{classify, Outcome, RequirementManifest};
use sce_build::requirements_report::emit_requirements_ndjson;
use sce_build::transition_table::transition_table;
use sce_build::unresolved_check::{check_strict_unresolved, emit_unresolved_ndjson};

/// Every container of executable content the model has, each carrying
/// one `sce:req` and one `sce:unresolved` of its own, plus the node
/// kinds the readers already reach as positive controls.
const DOC: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" name="reach" initial="armed" datamodel="ecmascript">
  <datamodel>
    <data id="items" expr="[1, 2]"/>
    <data id="n" expr="0"/>
  </datamodel>
  <state id="armed" sce:req="R_STATE" sce:unresolved="U_STATE">
    <onentry sce:req="R_BLOCK">
      <raise event="entered" sce:req="R_ENTRY" sce:unresolved="U_ENTRY"/>
      <if cond="n == 0">
        <raise event="then_ev" sce:req="R_THEN" sce:unresolved="U_THEN"/>
      <elseif cond="n == 1"/>
        <raise event="elseif_ev" sce:req="R_ELSEIF" sce:unresolved="U_ELSEIF"/>
      <else/>
        <raise event="else_ev" sce:req="R_ELSE" sce:unresolved="U_ELSE"/>
      </if>
    </onentry>
    <transition event="go" target="parent" sce:req="R_TRANSITION" sce:unresolved="U_TRANSITION">
      <raise event="went" sce:req="R_TRANSITION_ACTION" sce:unresolved="U_TRANSITION_ACTION"/>
      <foreach array="items" item="x">
        <raise event="each" sce:req="R_FOREACH" sce:unresolved="U_FOREACH"/>
      </foreach>
    </transition>
  </state>
  <state id="parent">
    <initial>
      <transition target="inner">
        <raise event="init_ev" sce:req="R_INITIAL" sce:unresolved="U_INITIAL"/>
      </transition>
    </initial>
    <state id="inner">
      <transition event="leave" target="h"/>
    </state>
    <state id="deep">
      <history id="h" type="shallow">
        <transition target="d1">
          <raise event="hist_ev" sce:req="R_HISTORY" sce:unresolved="U_HISTORY"/>
        </transition>
      </history>
      <state id="d1"/>
    </state>
  </state>
</scxml>"#;

fn parse(text: &str, label: &str) -> SCXMLModel {
    SCXMLParser::new()
        .parse_string(text, label)
        .unwrap_or_else(|e| panic!("{label} must parse: {:?}", e.error))
}

/// The ids a document's TEXT writes for one attribute, read without the
/// parser: `sce:unresolved="U_X"` / `sce:req="R_X"`.
fn written(text: &str, attribute: &str) -> BTreeSet<String> {
    let needle = format!("{attribute}=\"");
    text.match_indices(&needle)
        .filter_map(|(at, _)| {
            let rest = &text[at + needle.len()..];
            rest.find('"').map(|end| rest[..end].to_string())
        })
        .flat_map(|value| {
            value
                .split_whitespace()
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .collect()
}

/// Every marker id the parsed model HOLDS, found by walking its
/// serialised form rather than by naming the fields that can hold one.
fn held(model: &SCXMLModel) -> (BTreeSet<String>, BTreeSet<String>) {
    fn walk(
        tree: &serde_json::Value,
        unresolved: &mut BTreeSet<String>,
        req: &mut BTreeSet<String>,
    ) {
        match tree {
            serde_json::Value::Object(members) => {
                for (key, value) in members {
                    match (key.as_str(), value) {
                        ("unresolved", serde_json::Value::Array(markers)) => {
                            for marker in markers {
                                if let Some(id) = marker.get("id").and_then(|v| v.as_str()) {
                                    unresolved.insert(id.to_string());
                                }
                            }
                        }
                        ("req", serde_json::Value::Array(ids)) => {
                            for id in ids {
                                if let Some(id) = id.as_str() {
                                    req.insert(id.to_string());
                                }
                            }
                        }
                        _ => {}
                    }
                    walk(value, unresolved, req);
                }
            }
            serde_json::Value::Array(items) => {
                for item in items {
                    walk(item, unresolved, req);
                }
            }
            _ => {}
        }
    }
    let tree = serde_json::to_value(model).expect("the model serialises");
    let (mut unresolved, mut req) = (BTreeSet::new(), BTreeSet::new());
    walk(&tree, &mut unresolved, &mut req);
    (unresolved, req)
}

/// The document with every `sce:unresolved` removed but `keep`'s, so the
/// gate — which reports the FIRST marker only — is asked about each one
/// on its own.
fn only_marker(keep: &str, all: &BTreeSet<String>) -> String {
    all.iter()
        .filter(|id| id.as_str() != keep)
        .fold(DOC.to_string(), |doc, id| {
            doc.replace(&format!(" sce:unresolved=\"{id}\""), "")
        })
}

/// Written markers the parser did not store are a different defect from
/// stored markers no reader reaches, and each test says which it found.
fn assert_the_model_holds_what_the_document_writes(attribute: &str, held: &BTreeSet<String>) {
    let dropped: Vec<String> = written(DOC, attribute).difference(held).cloned().collect();
    assert!(
        dropped.is_empty(),
        "the parser did not store {} `{attribute}` id(s) the document writes: {dropped:?}. \
         That is a parse defect, reported apart from the readers below so the two \
         cannot be mistaken for each other",
        dropped.len(),
    );
}

/// ⚠ The gate is asked about every marker the model holds, one at a time.
#[test]
fn every_unresolved_marker_the_model_holds_reaches_the_report_and_the_gate() {
    const FLOOR: usize = 10;

    let model = parse(DOC, "reach");
    let (unresolved, _) = held(&model);
    assert_the_model_holds_what_the_document_writes("sce:unresolved", &unresolved);

    let mut out = Vec::new();
    emit_unresolved_ndjson(&model, &mut out).expect("writes to a Vec");
    let reported: BTreeSet<String> = String::from_utf8(out)
        .expect("utf-8")
        .lines()
        .filter_map(|line| {
            serde_json::from_str::<serde_json::Value>(line)
                .ok()?
                .get("id")?
                .as_str()
                .map(str::to_string)
        })
        .collect();
    let unreported: Vec<&String> = unresolved.difference(&reported).collect();

    let mut let_through: Vec<&String> = Vec::new();
    for id in &unresolved {
        let alone = parse(&only_marker(id, &unresolved), "reach_one_marker");
        match check_strict_unresolved(&alone) {
            Err(err) if format!("{err:?}").contains(id.as_str()) => {}
            _ => let_through.push(id),
        }
    }

    println!(
        "swept {} unresolved marker(s) the model holds: {} unreported, {} let through by the gate",
        unresolved.len(),
        unreported.len(),
        let_through.len(),
    );
    assert!(
        unreported.is_empty() && let_through.is_empty(),
        "readers of `sce:unresolved` miss markers the parsed model holds.\n  \
         not in `sce-codegen unresolved`: {unreported:?}\n  \
         passed by `--strict-unresolved`:  {let_through:?}\n\
         Each is an author's \"I did not know\" that the IR carries. A gate that \
         passes one lets a build proceed on a placeholder the manifest \
         classification calls `unresolved` — the two readings disagree, and the \
         gate is the one that is wrong.",
    );
    assert!(
        unresolved.len() >= FLOOR,
        "only {} marker(s) swept; floor {FLOOR}",
        unresolved.len()
    );
}

/// Every requirement id the model holds reaches all three readings of
/// the shared walk: the report, the transition table and the verdict.
#[test]
fn every_requirement_the_model_holds_reaches_the_report_the_table_and_the_verdict() {
    const FLOOR: usize = 11;

    let model = parse(DOC, "reach");
    let (_, req) = held(&model);
    assert_the_model_holds_what_the_document_writes("sce:req", &req);

    let mut out = Vec::new();
    emit_requirements_ndjson(&model, &mut out).expect("writes to a Vec");
    let in_report: BTreeSet<String> = String::from_utf8(out)
        .expect("utf-8")
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .filter_map(|record| record.get("requirement_ids").cloned())
        .filter_map(|ids| ids.as_array().cloned())
        .flatten()
        .filter_map(|id| id.as_str().map(str::to_string))
        .collect();

    let in_table: BTreeSet<String> = transition_table(&model)
        .iter()
        .flat_map(|row| {
            row.source
                .split(' ')
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .collect();

    let manifest_json = format!(
        r#"{{ "doc_id": "reach", "rev": "A",
              "extraction": {{ "ids": "native", "trace": "none",
                               "modality_convention": "english-modal-verbs",
                               "method": "hand" }},
              "requirements": [{}] }}"#,
        req.iter()
            .map(|id| format!(r#"{{ "id": "{id}" }}"#))
            .collect::<Vec<_>>()
            .join(", ")
    );
    let manifest =
        RequirementManifest::from_json(&manifest_json, "reach_manifest").expect("manifest loads");
    let missing: BTreeSet<String> = classify(&model, &manifest)
        .outcomes
        .iter()
        .filter(|o| o.outcome == Outcome::Missing)
        .map(|o| o.id.clone())
        .collect();

    let not_in_report: Vec<&String> = req.difference(&in_report).collect();
    let not_in_table: Vec<&String> = req.difference(&in_table).collect();
    let called_missing: Vec<&String> = req.intersection(&missing).collect();

    println!(
        "swept {} requirement id(s) the model holds: {} not in the report, \
         {} not in the table, {} classified missing",
        req.len(),
        not_in_report.len(),
        not_in_table.len(),
        called_missing.len(),
    );
    assert!(
        not_in_report.is_empty() && not_in_table.is_empty() && called_missing.is_empty(),
        "readings of the shared walk miss `sce:req` ids the parsed model holds.\n  \
         not in the report:        {not_in_report:?}\n  \
         not in the table:         {not_in_table:?}\n  \
         classified `missing`:     {called_missing:?}\n\
         An id the author wrote and the IR carries, reported `missing` however \
         carefully it was placed, is the silent wrong answer HOLE-3 closed for a \
         transition's own action — one container deeper.",
    );
    assert!(
        req.len() >= FLOOR,
        "only {} id(s) swept; floor {FLOOR}",
        req.len()
    );
}

/// A block annotation reaches every action inside the block, nested ones
/// included.
///
/// `docs/SCE_ACCEPTED_SUBSET.md` §2.10: "Block annotations on `<onentry>`
/// / `<onexit>` inherit onto every action inside the block". An action
/// inside an `<if>` inside the block is inside the block.
#[test]
fn a_block_annotation_reaches_every_action_nested_inside_the_block() {
    const FLOOR: usize = 5;

    fn actions_of(value: &serde_json::Value, out: &mut Vec<serde_json::Value>) {
        let Some(items) = value.as_array() else {
            return;
        };
        for action in items {
            out.push(action.clone());
            for nested in ["actions", "then_actions", "else_actions"] {
                actions_of(&action[nested], out);
            }
            if let Some(branches) = action["elseif_branches"].as_array() {
                for branch in branches {
                    actions_of(&branch["actions"], out);
                }
            }
        }
    }

    let tree = serde_json::to_value(parse(DOC, "reach")).expect("the model serialises");
    let block = &tree["states"]["armed"]["on_entry_blocks"][0];
    let mut every = Vec::new();
    actions_of(block, &mut every);

    let without: Vec<String> = every
        .iter()
        .filter(|action| {
            !action["req"]
                .as_array()
                .is_some_and(|ids| ids.iter().any(|id| id == "R_BLOCK"))
        })
        .map(|action| {
            format!(
                "<{}> carrying {}",
                // `Action::action_type` serialises as `type`, the name
                // the templates read it by.
                action["type"].as_str().unwrap_or("?"),
                action["req"]
            )
        })
        .collect();

    println!(
        "examined {} action(s) inside the annotated block, {} without its id",
        every.len(),
        without.len()
    );
    assert!(
        without.is_empty(),
        "the `<onentry sce:req=\"R_BLOCK\">` id did not reach every action inside \
         the block:\n  {}",
        without.join("\n  "),
    );
    assert!(
        every.len() >= FLOOR,
        "only {} action(s) found in the block; floor {FLOOR}",
        every.len()
    );
}

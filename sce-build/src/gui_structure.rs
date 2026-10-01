// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The statechart structure the visualizer draws, built from the Rust model.
//!
//! # Why
//!
//! The visualizer used to draw what the C++ engine's parser read
//! (`InteractiveTestRunner::buildStructureFromModel`), while every other
//! surface a reviewer reads — the pseudocode page, the print figures, the
//! requirement checklist — is built from this crate's model. Two parsers
//! meant two interpretations of one document on the reviewer's screen. The
//! GUI's drawing, layout and interaction stay as they are; only its INPUT
//! moves here. Execution stays with the C++ interpreter, so the contract is
//! the one the C++ builder publishes — the GUI's live highlighting joins
//! this structure to that runtime's state ids and transition positions.
//!
//! # The contract (the C++ builder's, measured 2026-09-29)
//!
//! `{states, transitions, initial}`:
//!
//! - `states`: every state node in document order, `<history>` included in
//!   its written place. `{id, type, historyType?, children?, initial?,
//!   onentry?, onexit?, hasInvoke?, invokes?}`; `type` is `atomic`,
//!   `compound`, `parallel`, `final` or `history`; `children` lists a
//!   history in its written place too; `initial` is a compound's resolved
//!   initial child; entry and exit blocks are flattened. `<initial>` makes
//!   no node.
//! - `transitions`: one object PER TARGET, in state order then transition
//!   order, a history's default transition at the history's place.
//!   `{id, source, sourceIndex, target, event, events?, eventless,
//!   isInternal, type, cond?, actions?}`: `id` counts objects from 0 as a
//!   string, `sourceIndex` is the transition's position in its source,
//!   `target` is `""` for a targetless transition, `event` is the attribute
//!   as written and `events` its descriptors.
//! - `initial`: the document's initial state.
//!
//! Two additions the C++ builder never had: `unresolved` and `assumed` on a
//! state or transition, listing the ids of the markers the author left
//! there — the marks the prototype page showed its reviewer first.
//!
//! ⚠ Where the model lowered what the document wrote, this reports the
//! model: an SCXML `<invoke>`'s written `type` is not kept (the model knows
//! it is SCXML), so `invokeType` is present only where the model holds one.

use crate::model::{Action, BlockRole, HistoryInfo, Invoke, SCXMLModel, State, Transition};
use crate::provenance::{MarkerKind, UnresolvedMarker};
use serde_json::{json, Map, Value};

/// One state node in document order: a state, or a history.
enum Node<'a> {
    State(&'a State),
    History(&'a str, &'a HistoryInfo),
}

/// The structure the visualizer draws for `model`.
pub fn gui_structure(model: &SCXMLModel) -> Value {
    let nodes = nodes_in_document_order(model);

    let mut states = Vec::new();
    for node in &nodes {
        states.push(match node {
            Node::State(s) => state_object(model, s),
            Node::History(id, h) => json!({
                "id": id,
                "type": "history",
                "historyType": if h.history_type == "deep" { "deep" } else { "shallow" },
            }),
        });
    }

    let mut transitions = Vec::new();
    for node in &nodes {
        match node {
            Node::State(s) => {
                for (index, t) in s.transitions.iter().enumerate() {
                    push_transition(&mut transitions, &s.id, index, t);
                }
            }
            Node::History(id, h) => {
                // §scxml-3.10.2: the history's default transition, eventless
                // and external, at the history's place.
                let object = transition_object(id, 0, "", false, "", &h.default_actions, &[]);
                for target in &h.default_targets {
                    push_target(&mut transitions, &object, target);
                }
            }
        }
    }

    json!({
        "states": states,
        "transitions": transitions,
        // As written, not `model.initial`, which a pass resolves to a leaf.
        "initial": written_initial(&model.initial_targets, &model.initial),
    })
}

/// The initial transition's targets as written (§scxml-3.2 / §scxml-3.3),
/// space-joined; `fallback` only for a model built without them.
fn written_initial(targets: &[String], fallback: &str) -> String {
    if targets.is_empty() {
        fallback.to_string()
    } else {
        targets.join(" ")
    }
}

/// Every state and history, ordered as the author wrote them.
///
/// States have a document-order rank; histories do not (their rank would
/// renumber the states generated code indexes by), so both are placed by
/// source position, the rank breaking ties for nodes that carry none.
fn nodes_in_document_order(model: &SCXMLModel) -> Vec<Node<'_>> {
    let position = |loc: &Option<crate::forge::error::SourceLocation>| {
        loc.as_ref()
            .map(|l| (l.line.unwrap_or(0), l.col.unwrap_or(0)))
            .unwrap_or((u32::MAX, u32::MAX))
    };
    let mut keyed: Vec<((u32, u32), u32, Node<'_>)> = Vec::new();
    for s in model.states.values() {
        keyed.push((
            position(&s.source_location),
            s.document_order,
            Node::State(s),
        ));
    }
    for (id, h) in &model.history_states {
        keyed.push((position(&h.source_location), u32::MAX, Node::History(id, h)));
    }
    keyed.sort_by_key(|k| (k.0, k.1));
    keyed.into_iter().map(|(_, _, n)| n).collect()
}

fn state_object(model: &SCXMLModel, s: &State) -> Value {
    let mut o = Map::new();
    o.insert("id".into(), json!(s.id));

    // A compound's children, histories in their written place.
    let mut children: Vec<(&str, (u32, u32))> = Vec::new();
    for c in &s.children {
        if let Some(child) = model.states.get(c) {
            children.push((c, position_of(&child.source_location, child.document_order)));
        }
    }
    for (id, h) in model
        .history_states
        .iter()
        .filter(|(_, h)| h.parent == s.id)
    {
        children.push((id, position_of(&h.source_location, u32::MAX)));
    }
    children.sort_by_key(|(_, p)| *p);

    let kind = if s.is_final {
        "final"
    } else if s.is_parallel {
        "parallel"
    } else if children.is_empty() {
        "atomic"
    } else {
        "compound"
    };
    o.insert("type".into(), json!(kind));
    if !children.is_empty() {
        o.insert(
            "children".into(),
            json!(children.iter().map(|(id, _)| *id).collect::<Vec<_>>()),
        );
    }
    if kind == "compound" {
        // `initial_targets` already holds the first child in document order
        // when the document names none (§scxml-3.3).
        let initial = written_initial(&s.initial_targets, &s.initial);
        if !initial.is_empty() {
            o.insert("initial".into(), json!(initial));
        }
    }
    for (key, blocks) in [
        ("onentry", &s.on_entry_blocks),
        ("onexit", &s.on_exit_blocks),
    ] {
        let flat: Vec<&Action> = blocks.iter().flatten().collect();
        if !flat.is_empty() {
            o.insert(key.into(), actions(flat));
        }
    }
    if !s.invokes.is_empty() {
        o.insert("hasInvoke".into(), json!(true));
        o.insert(
            "invokes".into(),
            json!(s.invokes.iter().map(invoke_object).collect::<Vec<_>>()),
        );
    }
    marks(&mut o, &s.unresolved);
    Value::Object(o)
}

fn position_of(loc: &Option<crate::forge::error::SourceLocation>, fallback: u32) -> (u32, u32) {
    loc.as_ref()
        .map(|l| (l.line.unwrap_or(0), l.col.unwrap_or(0)))
        .unwrap_or((u32::MAX, fallback))
}

fn push_transition(out: &mut Vec<Value>, source: &str, index: usize, t: &Transition) {
    let object = transition_object(
        source,
        index,
        &t.event,
        // §scxml-3.13: `type` as written, "external" unless written
        // "internal". A targetless transition exits nothing whatever its
        // type, and that is the empty `target` below, not this flag; the C++
        // engine's `ITransitionNode::isInternal()` is the attribute alone.
        t.transition_type == "internal",
        &t.cond,
        &t.actions,
        &t.unresolved,
    );
    if t.targets.is_empty() {
        push_target(out, &object, "");
    }
    for target in &t.targets {
        push_target(out, &object, target);
    }
}

/// Everything but `id` and `target`, which each target's copy gets.
fn transition_object(
    source: &str,
    index: usize,
    event: &str,
    internal: bool,
    cond: &str,
    transition_actions: &[Action],
    markers: &[UnresolvedMarker],
) -> Map<String, Value> {
    let mut o = Map::new();
    o.insert("source".into(), json!(source));
    o.insert("sourceIndex".into(), json!(index));
    let event = event.trim();
    o.insert("event".into(), json!(event));
    if !event.is_empty() {
        o.insert(
            "events".into(),
            json!(event.split_whitespace().collect::<Vec<_>>()),
        );
    }
    o.insert("eventless".into(), json!(event.is_empty()));
    o.insert("isInternal".into(), json!(internal));
    o.insert(
        "type".into(),
        json!(if internal { "internal" } else { "external" }),
    );
    if !cond.is_empty() {
        o.insert("cond".into(), json!(cond));
    }
    if !transition_actions.is_empty() {
        o.insert("actions".into(), actions(transition_actions.iter()));
    }
    marks(&mut o, markers);
    o
}

fn push_target(out: &mut Vec<Value>, object: &Map<String, Value>, target: &str) {
    let mut o = Map::new();
    o.insert("id".into(), json!(out.len().to_string()));
    for (k, v) in object {
        if k == "sourceIndex" {
            o.insert(k.clone(), v.clone());
            o.insert("target".into(), json!(target));
        } else {
            o.insert(k.clone(), v.clone());
        }
    }
    out.push(Value::Object(o));
}

/// `unresolved` / `assumed`: the ids of the markers of each kind.
fn marks(o: &mut Map<String, Value>, markers: &[UnresolvedMarker]) {
    for (key, kind) in [
        ("unresolved", MarkerKind::Unresolved),
        ("assumed", MarkerKind::Assumed),
    ] {
        let ids: Vec<&str> = markers
            .iter()
            .filter(|m| m.kind == kind)
            .map(|m| m.id.as_str())
            .collect();
        if !ids.is_empty() {
            o.insert(key.into(), json!(ids));
        }
    }
}

fn actions<'a>(list: impl IntoIterator<Item = &'a Action>) -> Value {
    Value::Array(list.into_iter().map(action_object).collect())
}

fn action_object(a: &Action) -> Value {
    let mut o = Map::new();
    let mut put = |k: &str, v: &str| {
        if !v.is_empty() {
            o.insert(k.into(), json!(v));
        }
    };
    match a.action_type.as_str() {
        "assign" => {
            put("location", &a.location);
            put("expr", &a.expr);
        }
        "raise" => put("event", &a.event),
        "log" => {
            put("expr", &a.expr);
            put("label", &a.label);
        }
        "send" => {
            put("event", &a.event);
            put("eventexpr", &a.eventexpr);
            put("target", &a.target);
            put("targetexpr", &a.targetexpr);
            put("delay", &a.delay);
            put("delayexpr", &a.delayexpr);
            put("content", &a.content);
            put("contentexpr", &a.contentexpr);
            put("sendid", &a.id);
            put("idlocation", &a.idlocation);
            put("type", &a.send_type);
            put("typeexpr", &a.typeexpr);
            put("namelist", &a.namelist);
        }
        "cancel" => {
            put("sendid", &a.sendid);
            put("sendidexpr", &a.sendidexpr);
        }
        "script" => put("content", &a.content),
        "foreach" => put("index", &a.index),
        _ => {}
    }
    // Fields every action carries, and the nested ones, after the
    // type-specific scalars.
    let mut head = Map::new();
    head.insert("actionType".into(), json!(a.action_type));
    // The C++ engine's rule (`ActionParser.cpp`): the `name` attribute, else
    // `id`, else the element's name.
    let id = if !a.native_action_name.is_empty() {
        &a.native_action_name
    } else if !a.id.is_empty() {
        &a.id
    } else {
        &a.action_type
    };
    head.insert("id".into(), json!(id));
    head.extend(o);
    let mut o = head;
    match a.action_type.as_str() {
        "assign" => {
            // C++ writes `location` and `expr` for every assign, empty or not.
            o.entry("location").or_insert_with(|| json!(""));
            o.entry("expr").or_insert_with(|| json!(""));
        }
        "raise" if o.get("event").is_none() => {
            o.insert("event".into(), json!(""));
        }
        "send" if !a.params.is_empty() => {
            o.insert("params".into(), params(&a.params));
        }
        // The nested blocks come through `Action::nested_blocks`, the one
        // definition of what lies inside an action (closure-ledger row S1).
        "foreach" => {
            // `array` and `item` are written even when empty, as C++ does.
            o.insert("array".into(), json!(a.array));
            o.insert("item".into(), json!(a.item));
            let body = a
                .nested_blocks()
                .into_iter()
                .filter(|b| b.role == BlockRole::Body)
                .flat_map(|b| b.actions.iter());
            o.insert("iterationActions".into(), actions(body));
        }
        "if" => {
            o.insert("cond".into(), json!(a.cond));
            let mut branches = Vec::new();
            for b in a.nested_blocks() {
                let (condition, is_else) = match b.role {
                    // The `then` block is selected by the action's own cond.
                    BlockRole::Then => (a.cond.as_str(), false),
                    BlockRole::ElseIf => (b.cond.unwrap_or(""), false),
                    // C++ reports an else branch only where one is written;
                    // the model cannot tell an empty `<else/>` from none.
                    BlockRole::Else if !b.actions.is_empty() => ("", true),
                    _ => continue,
                };
                branches.push(json!({
                    "condition": condition, "isElse": is_else, "actions": actions(b.actions)
                }));
            }
            o.insert("branches".into(), Value::Array(branches));
        }
        _ => {}
    }
    marks(&mut o, &a.unresolved);
    Value::Object(o)
}

fn params(list: &[crate::model::Param]) -> Value {
    Value::Array(
        list.iter()
            .map(|p| {
                let mut o = Map::new();
                o.insert("name".into(), json!(p.name));
                o.insert("expr".into(), json!(p.expr));
                if !p.location.is_empty() {
                    o.insert("location".into(), json!(p.location));
                }
                Value::Object(o)
            })
            .collect(),
    )
}

fn invoke_object(inv: &Invoke) -> Value {
    let mut o = Map::new();
    let mut put = |k: &str, v: &str| {
        if !v.is_empty() {
            o.insert(k.into(), json!(v));
        }
    };
    let base = inv.base();
    // Only the id the author wrote; the C++ engine reports no other.
    if !base.id_generated {
        put("invokeId", &base.invoke_id);
    }
    put("invokeIdLocation", &base.idlocation);
    let mut autoforward = false;
    match inv {
        Invoke::Scxml(i) => {
            // An inline child is lowered to a synthesised `src`
            // (`#<doc>__sce_synth_invoke__<id>`) the author never wrote;
            // the GUI gets the content they did write instead.
            match &i.inline_child_xml {
                Some(xml) => put("invokeContent", xml),
                None => put("invokeSrc", &i.src),
            }
            put("invokeNamelist", &i.namelist);
            put("invokeFinalize", &i.finalize_content);
            autoforward = i.common.autoforward;
        }
        Invoke::Hybrid(i) => {
            put("invokeSrcExpr", &i.srcexpr);
            put("invokeContentExpr", &i.contentexpr);
            put("invokeNamelist", &i.namelist);
            autoforward = i.common.autoforward;
        }
        Invoke::MeshRpc(_) => {}
        Invoke::Unsupported(i) => {
            put("invokeType", &i.invoke_type);
            put("invokeSrc", &i.src);
            put("invokeSrcExpr", &i.srcexpr);
            put("invokeContent", &i.content);
            put("invokeContentExpr", &i.contentexpr);
            put("invokeNamelist", &i.namelist);
        }
    }
    if !base.params.is_empty() {
        o.insert("invokeParams".into(), params(&base.params));
    }
    if autoforward {
        o.insert("invokeAutoForward".into(), json!(true));
    }
    marks(&mut o, &base.unresolved);
    Value::Object(o)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::SCXMLParser;

    fn structure(body: &str) -> Value {
        gui_structure(
            &SCXMLParser::new()
                .parse_string(body, "gui")
                .expect("parses"),
        )
    }

    const DOC: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
        xmlns:sce="http://sce.dev/ext" version="1.0" initial="top" datamodel="ecmascript">
  <datamodel><data id="n" expr="0"/></datamodel>
  <state id="top" initial="a">
    <history id="back" type="deep"><transition target="b"/></history>
    <state id="a">
      <onentry><assign location="n" expr="1"/></onentry>
      <transition event="go more" target="b c"/>
      <transition event="tick" type="internal"/>
    </state>
    <state id="b"/>
    <state id="c"/>
  </state>
  <final id="done"/>
</scxml>"#;

    /// States in written order with the history in its place, and the
    /// compound listing it among its children.
    #[test]
    fn states_come_in_written_order_with_histories_in_place() {
        let s = structure(DOC);
        let ids: Vec<&str> = s["states"]
            .as_array()
            .unwrap()
            .iter()
            .map(|n| n["id"].as_str().unwrap())
            .collect();
        assert_eq!(ids, ["top", "back", "a", "b", "c", "done"]);
        let top = &s["states"][0];
        assert_eq!(top["type"], "compound");
        assert_eq!(top["children"], json!(["back", "a", "b", "c"]));
        assert_eq!(top["initial"], "a");
        assert_eq!(s["states"][1]["historyType"], "deep");
        assert_eq!(s["states"][5]["type"], "final");
        assert_eq!(s["initial"], "top");
    }

    /// One object per target, ids counted over all of them, the history's
    /// default at its place, and a targetless transition kept.
    #[test]
    fn transitions_are_one_object_per_target() {
        let s = structure(DOC);
        let t: Vec<(&str, u64, &str, &str)> = s["transitions"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| {
                (
                    t["source"].as_str().unwrap(),
                    t["sourceIndex"].as_u64().unwrap(),
                    t["target"].as_str().unwrap(),
                    t["id"].as_str().unwrap(),
                )
            })
            .collect();
        assert_eq!(
            t,
            [
                ("back", 0, "b", "0"),
                ("a", 0, "b", "1"),
                ("a", 0, "c", "2"),
                ("a", 1, "", "3"),
            ]
        );
        assert_eq!(s["transitions"][1]["events"], json!(["go", "more"]));
        assert_eq!(s["transitions"][3]["type"], "internal");
        assert_eq!(s["transitions"][0]["eventless"], true);
    }

    /// §scxml-3.13: `type` is reported as written. A transition with no
    /// target is not "internal" for lacking one: it stays external unless it
    /// says otherwise, and its emptiness is the `target` field's to carry.
    /// The C++ engine reports the attribute alone, and the parity gate holds
    /// the two to each other.
    #[test]
    fn a_transition_type_is_reported_as_written_whether_or_not_it_has_a_target() {
        let s = structure(
            r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="a">
  <state id="a">
    <transition event="tick"/>
    <transition event="tock" type="internal"/>
    <transition event="go" target="b"/>
  </state>
  <state id="b"/>
</scxml>"#,
        );
        let written: Vec<(&str, &str, bool)> = s["transitions"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| {
                (
                    t["event"].as_str().unwrap(),
                    t["type"].as_str().unwrap(),
                    t["isInternal"].as_bool().unwrap(),
                )
            })
            .collect();
        assert_eq!(
            written,
            [
                ("tick", "external", false),
                ("tock", "internal", true),
                ("go", "external", false),
            ]
        );
        assert_eq!(s["transitions"][0]["target"], "");
    }
}

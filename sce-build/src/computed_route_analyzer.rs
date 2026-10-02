// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Which sends of a document choose where they go at run time, and which
//! questions that choice rests on.
//!
//! §scxml-6.2.4: a `<send>` names its processor and its target either by
//! literal (`type`, `target`) or by expression (`typeexpr`, `targetexpr`). A
//! literal route is the document's own statement, and [`crate::parent_send_analyzer`]
//! publishes the one a machine cannot start without. A computed route is
//! decided by the datamodel when the send runs, and when the datamodel item it
//! reads is one the specification left open (`sce:unresolved` on the `<data>`),
//! the route is a question nobody has answered: the draft wrote
//! `targetexpr="callerTarget"` because a send needs a target, and the owner has
//! not said who the caller is.
//!
//! Measured 2026-10-02: a scenario run of such a design ended in the engine's
//! `error.communication`, and the verdict said so in the engine's words and the
//! design's. The failure is the owner's question showing through, not a defect of
//! the design and not a fact about the machine that ran it, and a consumer that
//! plays the design can only say so if it knows WHICH sends route by an open
//! question. This publishes that: per computed-route send, the data items its
//! route expressions read and the ids of the questions open on the send or on
//! any of those items.
//!
//! The literal-parent case has its own list because it also answers "can this
//! machine run on its own". This is the sibling for the routes a build-time walk
//! cannot name the value of; the two together are every send whose destination
//! rests on a question.
//!
//! ⚠ `reads` is the intersection of the names the route expressions use with the
//! document's DECLARED data (root and state datamodels), found by the lexer
//! ([`crate::ecmascript::reads`]). A name that merely looks like a data item is
//! listed, and a route that goes through a function or `_event` data is not
//! traced further: the list says what the expression names, not what its value
//! derives from.

use std::collections::BTreeMap;

use crate::ecmascript::reads::identifiers_read;
use crate::forge::error::SourceLocation;
use crate::model::{Action, SCXMLModel};
use crate::provenance::MarkerKind;

/// One `<send>` whose `typeexpr` or `targetexpr` is written.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct ComputedRoute {
    /// The `event` attribute verbatim, or `None` when the site names its event
    /// with `eventexpr`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event: Option<String>,
    /// The state whose executable content carries the send.
    pub state: String,
    /// Where in the source, in the `{file, line, col}` shape a diagnostic
    /// carries.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<SourceLocation>,
    /// The document's data items the route expressions name, in the order first
    /// written.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub reads: Vec<String>,
    /// The ids of the questions the specification leaves open (`sce:unresolved`,
    /// not a value chosen without an answer) that sit ON this send or on a data
    /// item in [`Self::reads`], each once. Empty when the route rests on none.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub decisions: Vec<String>,
}

/// The open questions marked on each declared data item, by the item's id.
fn open_questions_by_data(model: &SCXMLModel) -> BTreeMap<String, Vec<String>> {
    let mut found: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let state_items = model
        .states
        .values()
        .flat_map(|state| state.datamodel.iter());
    for item in model.variables.iter().chain(state_items) {
        let entry = found.entry(item.id.clone()).or_default();
        for marker in &item.unresolved {
            if marker.kind == MarkerKind::Unresolved && !entry.contains(&marker.id) {
                entry.push(marker.id.clone());
            }
        }
    }
    found
}

/// What one computed route reads and which questions it rests on.
struct Reading {
    /// The declared data items the route expressions name, in the order first
    /// written.
    reads: Vec<String>,
    /// The ids of the open questions on the send or on any item in `reads`,
    /// each once, the send's own first.
    decisions: Vec<String>,
}

/// What one `<send>`'s route reads and rests on, or `None` for a send whose
/// `type` and `target` are both literal. The one reading both the manifest's
/// list and the annotation the generated machine carries are made from, so the
/// two cannot name different questions.
fn route_of(send: &Action, data: &BTreeMap<String, Vec<String>>) -> Option<Reading> {
    if send.typeexpr.is_empty() && send.targetexpr.is_empty() {
        return None;
    }
    let mut reads: Vec<String> = Vec::new();
    for expression in [&send.typeexpr, &send.targetexpr] {
        for name in identifiers_read(expression) {
            if data.contains_key(name.as_str()) && !reads.contains(&name) {
                reads.push(name);
            }
        }
    }
    let mut decisions: Vec<String> = Vec::new();
    let own = send
        .unresolved
        .iter()
        .filter(|marker| marker.kind == MarkerKind::Unresolved)
        .map(|marker| marker.id.clone());
    let read = reads
        .iter()
        .flat_map(|name| data[name.as_str()].iter().cloned());
    for id in own.chain(read) {
        if !decisions.contains(&id) {
            decisions.push(id);
        }
    }
    Some(Reading { reads, decisions })
}

/// Every computed-route `<send>` of `model`, ordered by where it is written.
/// Sites with no location sort last: absent position is not position zero.
pub fn analyze(model: &SCXMLModel) -> Vec<ComputedRoute> {
    let data = open_questions_by_data(model);
    let mut routes: Vec<ComputedRoute> = model
        .sends()
        .into_iter()
        .filter_map(|(state_id, send)| {
            let Reading { reads, decisions } = route_of(send, &data)?;
            Some(ComputedRoute {
                event: (!send.event.is_empty()).then(|| send.event.clone()),
                state: state_id.to_string(),
                location: send.source_location.clone(),
                reads,
                decisions,
            })
        })
        .collect();
    routes.sort_by_key(|route| {
        route
            .location
            .as_ref()
            .map(|l| (l.line.unwrap_or(0), l.col.unwrap_or(0)))
            .unwrap_or((u32::MAX, u32::MAX))
    });
    // A history's default actions are copied into the state whose `initial`
    // names it, so one written site is walked twice (see
    // `parent_send_analyzer::analyze`).
    routes.dedup();
    routes
}

/// [`analyze`], each location placed where the author wrote it — in the fragment
/// an `<xi:include>` spliced in, not in the expanded text — the way every
/// manifest cause is (`SCXMLModel::authored_location`).
pub fn records(model: &SCXMLModel) -> Vec<ComputedRoute> {
    let mut routes = analyze(model);
    for route in &mut routes {
        route.location = route
            .location
            .as_ref()
            .map(|at| model.authored_location(at));
    }
    routes
}

/// Write onto each computed-route `<send>` the questions its route rests on
/// ([`Action::route_decisions`]), the reading [`analyze`] publishes. Run with
/// the rest of the model analysis, before any backend renders, so a backend
/// that lets its host tell such a failure from a fault of the document reads
/// the same ids the manifest names.
///
/// Walks every place [`SCXMLModel::sends`] does: a state's executable blocks,
/// and a history's default actions, which the parser holds a second copy of.
pub fn annotate(model: &mut SCXMLModel) {
    fn walk(actions: &mut [Action], data: &BTreeMap<String, Vec<String>>) {
        for action in actions {
            if action.action_type == "send" {
                action.route_decisions = route_of(action, data)
                    .map(|reading| reading.decisions)
                    .unwrap_or_default();
            }
            for block in action.nested_blocks_mut() {
                walk(block, data);
            }
        }
    }
    let data = open_questions_by_data(model);
    for state in model.states.values_mut() {
        for block in state.executable_blocks_mut() {
            walk(block, &data);
        }
    }
    for history in model.history_states.values_mut() {
        walk(&mut history.default_actions, &data);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::SCXMLParser;

    fn parse(body: &str) -> SCXMLModel {
        SCXMLParser::new()
            .parse_string(body, "computed_route.scxml")
            .expect("parses")
    }

    /// A history's default transition is copied into the state whose `initial`
    /// names it, so one written site is walked twice. It is listed once.
    #[test]
    fn a_history_default_send_is_listed_once() {
        let m = parse(
            r##"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="a">
  <state id="a" initial="h">
    <history id="h">
      <transition target="b"><send event="e" targetexpr="where"/></transition>
    </history>
    <state id="b"/>
  </state>
</scxml>"##,
        );
        let routes = analyze(&m);
        let found: Vec<(Option<&str>, &str)> = routes
            .iter()
            .map(|r| (r.event.as_deref(), r.state.as_str()))
            .collect();
        assert_eq!(found, [(Some("e"), "a")], "{routes:?}");
    }

    fn parse_with_questions(body: &str) -> SCXMLModel {
        let mut model = parse(body);
        annotate(&mut model);
        model
    }

    /// The generated machine carries the questions the manifest names, on the
    /// send itself, wherever the send sits: at the top of a block, nested in an
    /// `<if>`, in an `<onexit>`, and in a history's default transition.
    #[test]
    fn every_computed_send_carries_the_questions_its_route_rests_on() {
        let m = parse_with_questions(
            r##"<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext" version="1.0" initial="a" datamodel="ecmascript">
  <datamodel>
    <data id="where" expr="''" sce:unresolved="caller-routing" sce:unresolved-reason="r"/>
    <data id="plain" expr="''"/>
  </datamodel>
  <state id="a" initial="h">
    <history id="h">
      <transition target="b"><send event="from.history" targetexpr="where"/></transition>
    </history>
    <state id="b">
      <onentry>
        <send event="top" targetexpr="where"/>
        <if cond="true"><send event="nested" targetexpr="where"/></if>
      </onentry>
      <onexit><send event="leaving" targetexpr="where"/></onexit>
      <transition event="go" target="c">
        <send event="calm" targetexpr="plain"/>
        <send event="literal" target="#_internal"/>
      </transition>
    </state>
    <state id="c"/>
  </state>
</scxml>"##,
        );
        let mut carried: Vec<(String, Vec<String>)> = m
            .sends()
            .into_iter()
            .map(|(_, send)| (send.event.clone(), send.route_decisions.clone()))
            .collect();
        carried.sort();
        carried.dedup();
        let asks = vec!["caller-routing".to_string()];
        assert_eq!(
            carried,
            [
                ("calm".to_string(), vec![]),
                ("from.history".to_string(), asks.clone()),
                ("leaving".to_string(), asks.clone()),
                ("literal".to_string(), vec![]),
                ("nested".to_string(), asks.clone()),
                ("top".to_string(), asks),
            ]
        );
    }

    /// An inline child's route is the child's own: it chooses where the CHILD
    /// sends, and says nothing about the document that invokes it.
    #[test]
    fn an_inline_childs_computed_route_is_not_the_parents() {
        let m = parse(
            r##"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="a">
  <state id="a">
    <invoke type="http://www.w3.org/TR/scxml/">
      <content>
        <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="c">
          <state id="c"><onentry><send event="x" targetexpr="where"/></onentry></state>
        </scxml>
      </content>
    </invoke>
  </state>
</scxml>"##,
        );
        assert!(analyze(&m).is_empty(), "{:?}", analyze(&m));
    }
}

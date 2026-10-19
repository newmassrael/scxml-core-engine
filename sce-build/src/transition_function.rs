// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A statechart as a pure function of (leaf state, event), computed once.
//!
//! A document that declares `sce:transition-function="emit"` (SCE Accepted
//! Subset §2.21) is held to the shape in which every macrostep is exactly one
//! microstep and the configuration is fixed by one leaf: no `<parallel>`, no
//! `<history>`, no executable content, no eventless transition, no guard, no
//! `<final>` below the root and no transition with several targets
//! ([`crate::scxml_transition_function`] refuses the rest). For such a
//! document the next configuration depends on the current leaf and the event
//! alone, and this module computes it at build time with the selection rule
//! of the W3C algorithm applied to one atomic state: the leaf's transitions,
//! then each proper ancestor's, outward, each state's in document order, and
//! the first whose descriptors match the event is taken. No engine is
//! simulated, because none is needed: with nothing to execute, the states a
//! transition exits and enters have no effect beyond the leaf that results,
//! and that leaf is the default entry of the target. An internal transition
//! and an external one therefore reach the same leaf; the type changes which
//! states are exited and re-entered, not where the machine ends up.
//!
//! The ONE producer. Every backend renders the [`TransitionFunction`] built
//! here, and the subset check reads the same events and the same selection,
//! so a dead end it refuses and the row of the table that would have shown
//! one are a single fact. The transitions are [`transition_records`], the
//! list every transition surface renders, and a descriptor is matched by
//! [`crate::event_descriptor`], the crate's one definition of what a
//! descriptor matches.

use std::collections::BTreeMap;

use crate::event_descriptor::{literal_event, EventDescriptor};
use crate::model::{
    SCXMLModel, State, TransitionFunction, TransitionFunctionCell, TransitionFunctionState,
};
use crate::transition_records::{transition_records, TransitionRecord};

impl TransitionFunction {
    /// The transition function of `model`, carrying `source_hash` — the
    /// digest the generated files of the same run embed.
    ///
    /// Total over any model, so that a caller holding one the subset check
    /// has not judged gets a value rather than a panic; the value means what
    /// its name says only for a document the check accepts.
    pub fn of(model: &SCXMLModel, source_hash: String) -> Self {
        let events = events(model);
        let states = rows(model, &events);
        let every_pair_taken = states
            .iter()
            .all(|state| state.next.iter().all(|cell| cell.to.is_some()));
        Self {
            source_hash,
            initial: initial_leaf(model),
            states,
            events,
            every_pair_taken,
        }
    }

    /// The state `event` leaves `state` in, or `None` when it is refused —
    /// and `None` for a state or an event the function does not name.
    pub fn transition(&self, state: &str, event: &str) -> Option<&str> {
        self.state(state)?
            .next
            .iter()
            .find(|cell| cell.event == event)?
            .to
            .as_deref()
    }

    /// The row of the state named `id`.
    pub fn state(&self, id: &str) -> Option<&TransitionFunctionState> {
        self.states.iter().find(|state| state.id == id)
    }
}

/// The states a machine can be in, in document order: every atomic `<state>`
/// and every `<final>` — a state with no child state. A `<parallel>` always
/// has regions, so it is never one of these.
pub fn leaves(model: &SCXMLModel) -> Vec<&State> {
    let mut leaves: Vec<&State> = model
        .states
        .values()
        .filter(|state| state.children.is_empty() && !state.is_parallel)
        .collect();
    leaves.sort_by_key(|state| state.document_order);
    leaves
}

/// Every event `model`'s transitions name literally, in the order of
/// [`transition_records`] — states in document order, each state's
/// transitions by position — each once, at its first appearance.
///
/// A pattern names no event: `*`, `error.*` and `error.` match events and
/// declare none ([`literal_event`]). A literal `error` is an event and a
/// family at once, so it is listed and still matches `error.send`.
pub fn events(model: &SCXMLModel) -> Vec<String> {
    let mut events: Vec<String> = Vec::new();
    for record in transition_records(model) {
        for descriptor in &record.events {
            if let Some(name) = literal_event(descriptor) {
                if !events.iter().any(|known| known == name) {
                    events.push(name.to_string());
                }
            }
        }
    }
    events
}

/// One row per state of [`leaves`], each with one cell per member of
/// `events`, in that order.
pub fn rows(model: &SCXMLModel, events: &[String]) -> Vec<TransitionFunctionState> {
    let selector = Selector::new(model);
    leaves(model)
        .into_iter()
        .map(|leaf| TransitionFunctionState {
            id: leaf.id.clone(),
            is_final: leaf.is_final,
            configuration: configuration(model, &leaf.id),
            next: events
                .iter()
                .map(|event| TransitionFunctionCell {
                    event: event.clone(),
                    to: selector.next(&leaf.id, event),
                })
                .collect(),
        })
        .collect()
}

/// The transitions of a model by owning state, each state's in document
/// order, so a selection reads a state's list without walking the rest.
struct Selector<'m> {
    model: &'m SCXMLModel,
    owned: BTreeMap<String, Vec<TransitionRecord>>,
}

impl<'m> Selector<'m> {
    fn new(model: &'m SCXMLModel) -> Self {
        let mut owned: BTreeMap<String, Vec<TransitionRecord>> = BTreeMap::new();
        for record in transition_records(model) {
            owned.entry(record.from.clone()).or_default().push(record);
        }
        Self { model, owned }
    }

    /// The transition taken when `event` arrives while `leaf` is the current
    /// state: the first, in the leaf and then in each proper ancestor in
    /// turn, whose descriptors match the event.
    fn select(&self, leaf: &str, event: &str) -> Option<&TransitionRecord> {
        ancestry(self.model, leaf).into_iter().find_map(|state| {
            self.owned.get(state)?.iter().find(|record| {
                record
                    .events
                    .iter()
                    .any(|descriptor| EventDescriptor::parse(descriptor).matches(event))
            })
        })
    }

    /// The state `event` leaves `leaf` in. A `<final>` takes no event: the
    /// machine has ended. A targetless transition leaves the leaf where it
    /// is; one with a target leaves the machine in that target's default
    /// entry.
    fn next(&self, leaf: &str, event: &str) -> Option<String> {
        if self
            .model
            .states
            .get(leaf)
            .is_some_and(|state| state.is_final)
        {
            return None;
        }
        let taken = self.select(leaf, event)?;
        Some(match taken.targets.first() {
            None => leaf.to_string(),
            Some(target) => default_entry(self.model, target),
        })
    }
}

/// `id` and then each proper ancestor of it, outward. The `<scxml>` root is
/// not a state, so the chain ends at a top-level state.
fn ancestry<'a>(model: &'a SCXMLModel, id: &'a str) -> Vec<&'a str> {
    let mut chain: Vec<&'a str> = Vec::new();
    let mut current = Some(id);
    while let Some(state_id) = current {
        // A parent link the parser derived cannot cycle; the guard keeps a
        // model built by hand from looping all the same.
        if chain.contains(&state_id) {
            break;
        }
        chain.push(state_id);
        current = model
            .states
            .get(state_id)
            .and_then(|state| state.parent.as_deref());
    }
    chain
}

/// The ids active while `leaf` is the current state, outermost first.
pub fn configuration(model: &SCXMLModel, leaf: &str) -> Vec<String> {
    ancestry(model, leaf)
        .into_iter()
        .rev()
        .map(str::to_string)
        .collect()
}

/// The leaf entering `target` by default arrives at: `target` itself when it
/// has no child state, and otherwise the default entry of its initial child —
/// the one its `initial` attribute or `<initial>` element names, or its first
/// child state in document order when it names none.
pub fn default_entry(model: &SCXMLModel, target: &str) -> String {
    let mut current = target;
    // Each step descends one level, so the depth of the tree bounds the walk.
    for _ in 0..=model.states.len() {
        let Some(state) = model.states.get(current) else {
            break;
        };
        if state.children.is_empty() || state.is_parallel {
            break;
        }
        let initial = state
            .initial_targets
            .iter()
            .map(String::as_str)
            .find(|child| model.states.contains_key(*child))
            .or_else(|| state.children.first().map(String::as_str));
        match initial {
            Some(child) => current = child,
            None => break,
        }
    }
    current.to_string()
}

/// The state the machine starts in: the default entry of the document's
/// initial state — the one `<scxml initial>` names, or the first top-level
/// state in document order when it names none.
fn initial_leaf(model: &SCXMLModel) -> String {
    let root = model
        .initial_targets
        .iter()
        .map(String::as_str)
        .find(|id| model.states.contains_key(*id))
        .or_else(|| {
            model
                .states
                .values()
                .filter(|state| state.parent.is_none())
                .min_by_key(|state| state.document_order)
                .map(|state| state.id.as_str())
        });
    root.map(|id| default_entry(model, id)).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::SCXMLParser;

    /// A session with ancestor-held transitions, every way a compound state
    /// names its default child, an internal and an external transition, a
    /// prefix and a wildcard descriptor, a refused pair and a top-level final.
    const NESTED: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
                              xmlns:sce="http://sce.dev/ext" sce:transition-function="emit"
                              version="1.0" initial="session" datamodel="null">
        <state id="session" initial="setup">
            <transition event="abort" target="closed"/>
            <state id="setup">
                <initial><transition target="loading"/></initial>
                <state id="loading">
                    <transition event="data.ready" target="ready"/>
                    <transition event="data.failed" target="retrying"/>
                </state>
                <state id="retrying">
                    <transition event="data.ready" target="ready"/>
                    <transition event="data.*" target="loading"/>
                </state>
                <state id="ready">
                    <transition event="go" target="active"/>
                </state>
            </state>
            <state id="active">
                <transition event="pause" type="internal" target="held"/>
                <transition event="reset" target="active"/>
                <state id="working">
                    <transition event="done" target="closed"/>
                    <transition event="resume"/>
                </state>
                <state id="held">
                    <transition event="resume" target="working"/>
                    <transition event="*"/>
                </state>
            </state>
        </state>
        <final id="closed"/>
    </scxml>"#;

    /// A flat cycle with a targetless transition and a self-transition with a
    /// target, and no final.
    const FLAT: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
                              xmlns:sce="http://sce.dev/ext" sce:transition-function="emit"
                              version="1.0" initial="idle" datamodel="null">
        <state id="idle">
            <transition event="start" target="running"/>
            <transition event="ping"/>
        </state>
        <state id="running">
            <transition event="pause" target="paused"/>
            <transition event="tick" target="running"/>
            <transition event="ping"/>
        </state>
        <state id="paused">
            <transition event="resume" target="running"/>
            <transition event="stop" target="idle"/>
        </state>
    </scxml>"#;

    fn function(doc: &str) -> TransitionFunction {
        let model = SCXMLParser::new()
            .parse_string(doc, "transition_function")
            .expect("parse");
        TransitionFunction::of(&model, "cd".repeat(32))
    }

    fn ids(function: &TransitionFunction) -> Vec<&str> {
        function.states.iter().map(|s| s.id.as_str()).collect()
    }

    #[test]
    fn the_states_are_the_leaves_in_document_order() {
        assert_eq!(
            ids(&function(NESTED)),
            ["loading", "retrying", "ready", "working", "held", "closed"]
        );
        assert_eq!(ids(&function(FLAT)), ["idle", "running", "paused"]);
    }

    #[test]
    fn the_events_are_the_literal_names_in_order_of_first_appearance() {
        // `data.*` and `*` match events and name none.
        assert_eq!(
            function(NESTED).events,
            [
                "abort",
                "data.ready",
                "data.failed",
                "go",
                "pause",
                "reset",
                "done",
                "resume"
            ]
        );
        assert_eq!(
            function(FLAT).events,
            ["start", "ping", "pause", "tick", "resume", "stop"]
        );
    }

    #[test]
    fn the_initial_state_is_the_default_entry_of_the_document() {
        // `<scxml initial>` names `session`, whose `initial` attribute names
        // `setup`, whose `<initial>` names `loading`.
        assert_eq!(function(NESTED).initial, "loading");
        assert_eq!(function(FLAT).initial, "idle");
    }

    #[test]
    fn the_default_entry_follows_the_attribute_the_element_and_the_first_child() {
        let model = SCXMLParser::new()
            .parse_string(NESTED, "default_entry")
            .expect("parse");
        assert_eq!(
            default_entry(&model, "session"),
            "loading",
            "initial attribute"
        );
        assert_eq!(
            default_entry(&model, "setup"),
            "loading",
            "<initial> element"
        );
        assert_eq!(default_entry(&model, "active"), "working", "first child");
        assert_eq!(
            default_entry(&model, "held"),
            "held",
            "a leaf is its own entry"
        );
    }

    #[test]
    fn a_transition_held_by_an_ancestor_is_taken_from_a_leaf_that_has_none() {
        let f = function(NESTED);
        assert_eq!(f.transition("loading", "abort"), Some("closed"));
        assert_eq!(f.transition("ready", "abort"), Some("closed"));
        assert_eq!(f.transition("working", "pause"), Some("held"));
    }

    #[test]
    fn the_first_transition_in_document_order_is_the_one_taken() {
        let f = function(NESTED);
        // `data.ready` precedes `data.*` in `retrying`.
        assert_eq!(f.transition("retrying", "data.ready"), Some("ready"));
        // The leaf's own wildcard comes before its ancestor's `abort`.
        assert_eq!(f.transition("held", "abort"), Some("held"));
    }

    #[test]
    fn a_prefix_and_a_wildcard_match_the_events_they_cover() {
        let f = function(NESTED);
        assert_eq!(f.transition("retrying", "data.failed"), Some("loading"));
        for event in ["go", "pause", "reset", "done", "data.ready"] {
            assert_eq!(f.transition("held", event), Some("held"), "{event}");
        }
        assert_eq!(f.transition("held", "resume"), Some("working"));
    }

    #[test]
    fn a_targetless_transition_leaves_the_state_where_it_is() {
        let f = function(NESTED);
        assert_eq!(f.transition("working", "resume"), Some("working"));
        let flat = function(FLAT);
        assert_eq!(flat.transition("idle", "ping"), Some("idle"));
        assert_eq!(flat.transition("running", "ping"), Some("running"));
    }

    #[test]
    fn a_self_transition_with_a_target_re_enters_by_default() {
        let flat = function(FLAT);
        assert_eq!(flat.transition("running", "tick"), Some("running"));
        // `reset` targets the compound `active` from inside it, and the
        // machine enters it again by its first child.
        let f = function(NESTED);
        assert_eq!(
            f.transition("held", "reset"),
            Some("held"),
            "held's wildcard"
        );
        assert_eq!(f.transition("working", "reset"), Some("working"));
    }

    #[test]
    fn an_internal_and_an_external_transition_reach_the_leaf_their_target_enters() {
        let f = function(NESTED);
        // Internal, into a child of its source.
        assert_eq!(f.transition("working", "pause"), Some("held"));
        // External, into a compound state entered by its first child.
        assert_eq!(f.transition("ready", "go"), Some("working"));
    }

    #[test]
    fn an_event_no_transition_takes_is_refused() {
        let f = function(NESTED);
        assert_eq!(f.transition("loading", "go"), None);
        assert_eq!(f.transition("ready", "resume"), None);
        assert!(!f.every_pair_taken);
        let flat = function(FLAT);
        assert_eq!(flat.transition("idle", "stop"), None);
        assert_eq!(flat.transition("paused", "ping"), None);
    }

    #[test]
    fn a_top_level_final_ends_the_machine_and_takes_no_event() {
        let f = function(NESTED);
        let closed = f.state("closed").expect("the final is a state");
        assert!(closed.is_final);
        assert_eq!(closed.configuration, ["closed"]);
        assert!(closed.next.iter().all(|cell| cell.to.is_none()));
        assert!(f
            .states
            .iter()
            .filter(|s| s.id != "closed")
            .all(|s| !s.is_final));
        assert!(function(FLAT).states.iter().all(|s| !s.is_final));
    }

    #[test]
    fn the_configuration_lists_the_ancestors_outermost_first() {
        let f = function(NESTED);
        let configuration = |id: &str| f.state(id).expect("a state").configuration.clone();
        assert_eq!(configuration("working"), ["session", "active", "working"]);
        assert_eq!(configuration("retrying"), ["session", "setup", "retrying"]);
        assert_eq!(
            function(FLAT).state("idle").unwrap().configuration,
            ["idle"]
        );
    }

    #[test]
    fn every_state_has_one_cell_per_event_in_the_order_of_the_events() {
        let f = function(NESTED);
        for state in &f.states {
            let events: Vec<&str> = state.next.iter().map(|c| c.event.as_str()).collect();
            assert_eq!(events, f.events, "{}", state.id);
        }
    }

    #[test]
    fn a_table_with_no_refused_pair_says_so() {
        let f = function(
            r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="a">
                 <state id="a"><transition event="x"/></state>
               </scxml>"#,
        );
        assert_eq!(f.transition("a", "x"), Some("a"));
        assert!(f.every_pair_taken);
    }

    #[test]
    fn the_hash_is_the_one_the_run_gave() {
        assert_eq!(function(FLAT).source_hash, "cd".repeat(32));
    }

    #[test]
    fn a_name_the_function_does_not_hold_is_answered_with_none() {
        let f = function(FLAT);
        assert_eq!(f.transition("nowhere", "start"), None);
        assert_eq!(f.transition("idle", "nothing"), None);
        assert!(f.state("nowhere").is_none());
    }
}

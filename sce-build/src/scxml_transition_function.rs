// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What a statechart declaring `sce:transition-function="emit"` is held to
//! (SCE Accepted Subset §2.21).
//!
//! The declaration says the document is a pure function of (leaf state,
//! event): every macrostep is one microstep, and one leaf fixes the
//! configuration. [`crate::transition_function`] computes that function at
//! build time, and it is only the document's meaning when nothing in the
//! document contradicts the shape. This module names every construct that
//! does, each where it is written:
//!
//! - a `<parallel>` — its regions are active together, so the configuration
//!   is more than one leaf;
//! - a `<history>` — entering it restores a configuration recorded earlier;
//! - executable content anywhere — `<datamodel>` / `<data>`, a top-level
//!   `<script>`, an `<onentry>` or `<onexit>` that holds anything, a
//!   transition's content, an `<initial>` transition's content, an
//!   `<invoke>`, a `<donedata>` (an empty `<onentry/>` runs nothing and is
//!   accepted);
//! - a transition without an event, which is taken inside the macrostep;
//! - a `cond`, which makes the next state depend on data;
//! - a `<final>` inside a compound state, whose entry raises `done.state`;
//! - a transition with several targets, which only regions take together;
//!
//! and then two things the table itself shows: a descriptor that matches no
//! event the machine names, and a non-final state no event leaves. Last, the
//! states the machine can never be in, from [`crate::scxml_reachability`]:
//! the reachability lints are opt-in for a document that declares nothing,
//! and a state of a transition function that no path reaches is a row no
//! caller can arrive at.
//!
//! Every refusal at once, in document order: an author holding a design to
//! the function repairs all of them, and a report that stopped at the first
//! would cost a build round each. A document that does not declare the
//! function is judged by none of this.

use crate::event_descriptor::EventDescriptor;
use crate::forge::error::{ForgeError, Located, SourceLocation};
use crate::model::{SCXMLModel, State, Transition};
use crate::scxml_semantic::ScxmlSemanticError;
use crate::transition_function::{events, rows};

/// A construct the function does not admit, and why — the sentence a record
/// carries as its `rule`.
const PARALLEL: &str = "its regions are active together, so the configuration is more than one \
                        leaf and the current state is no longer one value";
const HISTORY: &str = "entering it restores a configuration the machine recorded earlier, so \
                       where the machine goes depends on more than the current state and the \
                       event";
const DATA: &str = "it holds a value beside the current state, which a function of the state \
                    and the event does not carry";
const SCRIPT: &str = "it runs when the machine starts, and a function of the state and the \
                      event runs nothing";
const ENTRY: &str = "it runs when the state is entered, and a function of the state and the \
                     event runs nothing (an empty <onentry/> is accepted)";
const EXIT: &str = "it runs when the state is exited, and a function of the state and the \
                    event runs nothing (an empty <onexit/> is accepted)";
const INITIAL_CONTENT: &str = "its transition carries executable content, which runs when the \
                               state is entered by default, and a function of the state and the \
                               event runs nothing";
const INVOKE: &str = "it starts another session, whose events and lifetime a function of the \
                      state and the event does not represent";
const DONEDATA: &str = "it is evaluated when the machine ends, and a function of the state and \
                        the event evaluates nothing";
const NESTED_FINAL: &str = "entering a <final> inside a compound state raises done.state for its \
                            parent, a second microstep; the final states of a transition \
                            function are children of <scxml>";
const EVENTLESS: &str = "a transition without an event is taken inside the macrostep, so one \
                         macrostep would be more than one microstep";
const COND: &str = "its cond makes the next state depend on data beside the current state and \
                    the event";
const TARGETS: &str = "it names several targets, which only the regions of a <parallel> take \
                       together";
const CONTENT: &str = "it carries executable content, which runs when the transition is taken, \
                       and a function of the state and the event runs nothing";

/// Every refusal of a document that declares a transition function, in the
/// order the module header gives; none for a document that does not.
pub fn findings(model: &SCXMLModel, source: &str) -> Vec<Located<ForgeError>> {
    if !model.transition_function_declared {
        return Vec::new();
    }
    let mut refusals = Refusals {
        model,
        source,
        out: Vec::new(),
    };
    let events = events(model);

    // The document's own scope first: it is set up before any state is
    // entered.
    for variable in &model.variables {
        refusals.rule(
            format!("<data id=\"{}\">", variable.id),
            DATA,
            "",
            Some("data"),
            variable.source_location.as_ref(),
        );
    }
    for script in &model.global_scripts {
        refusals.rule(
            "<script>".to_string(),
            SCRIPT,
            "",
            Some("script"),
            script.source_location.as_ref(),
        );
    }

    let mut states: Vec<&State> = model.states.values().collect();
    states.sort_by_key(|state| state.document_order);
    for state in states {
        refusals.state(state);
        for transition in &state.transitions {
            refusals.transition(state, transition, &events);
        }
    }

    // What the table shows: a non-final state every event leaves where the
    // machine cannot go on from.
    for row in rows(model, &events) {
        if row.is_final || row.next.iter().any(|cell| cell.to.is_some()) {
            continue;
        }
        let at = model
            .states
            .get(&row.id)
            .and_then(|state| state.source_location.as_ref());
        refusals.push(
            ScxmlSemanticError::TransitionFunctionDeadEnd { state: row.id },
            at,
        );
    }

    let mut out = refusals.out;
    out.extend(crate::scxml_reachability::findings(model, source));
    out
}

/// The records of one judgement, located where each construct is written.
struct Refusals<'m> {
    model: &'m SCXMLModel,
    source: &'m str,
    out: Vec<Located<ForgeError>>,
}

impl Refusals<'_> {
    fn push(&mut self, error: ScxmlSemanticError, at: Option<&SourceLocation>) {
        let located = self.model.locate(error.into(), at, self.source);
        self.out.push(located);
    }

    fn rule(
        &mut self,
        construct: String,
        rule: &str,
        state: &str,
        observed: Option<&str>,
        at: Option<&SourceLocation>,
    ) {
        self.push(
            ScxmlSemanticError::TransitionFunctionRule {
                construct,
                rule: rule.to_string(),
                state: state.to_string(),
                observed: observed.map(str::to_string),
            },
            at,
        );
    }

    /// What a state element itself, and what it holds besides its
    /// transitions, can contradict.
    fn state(&mut self, state: &State) {
        let at = state.source_location.as_ref();
        let id = state.id.as_str();
        if state.is_parallel {
            self.rule(
                format!("<parallel id=\"{id}\">"),
                PARALLEL,
                id,
                Some("parallel"),
                at,
            );
        }
        if state.is_final {
            if let Some(parent) = &state.parent {
                self.rule(
                    format!("<final id=\"{id}\"> inside state '{parent}'"),
                    NESTED_FINAL,
                    id,
                    Some("final"),
                    at,
                );
            }
        }
        for variable in &state.datamodel {
            self.rule(
                format!("<data id=\"{}\"> of state '{id}'", variable.id),
                DATA,
                id,
                Some("data"),
                variable.source_location.as_ref().or(at),
            );
        }
        for block in &state.on_entry_blocks {
            self.rule(
                format!("the <onentry> of state '{id}'"),
                ENTRY,
                id,
                Some("onentry"),
                block
                    .first()
                    .and_then(|a| a.source_location.as_ref())
                    .or(at),
            );
        }
        for block in &state.on_exit_blocks {
            self.rule(
                format!("the <onexit> of state '{id}'"),
                EXIT,
                id,
                Some("onexit"),
                block
                    .first()
                    .and_then(|a| a.source_location.as_ref())
                    .or(at),
            );
        }
        if !state.initial_transition_actions.is_empty() {
            self.rule(
                format!("the <initial> of state '{id}'"),
                INITIAL_CONTENT,
                id,
                Some("initial"),
                state
                    .initial_transition_actions
                    .first()
                    .and_then(|a| a.source_location.as_ref())
                    .or(at),
            );
        }
        for invoke in &state.invokes {
            self.rule(
                format!("the <invoke> of state '{id}'"),
                INVOKE,
                id,
                Some("invoke"),
                invoke.base().source_location.as_ref().or(at),
            );
        }
        if state.donedata.is_some() {
            self.rule(
                format!("the <donedata> of <final id=\"{id}\">"),
                DONEDATA,
                id,
                Some("donedata"),
                at,
            );
        }
        let model = self.model;
        let mut histories: Vec<(&String, &crate::model::HistoryInfo)> = model
            .history_states
            .iter()
            .filter(|(_, history)| history.parent == state.id)
            .collect();
        histories.sort_by_key(|(_, history)| history.source_location.as_ref().and_then(|l| l.line));
        for (history_id, history) in histories {
            self.rule(
                format!("<history id=\"{history_id}\"> of state '{id}'"),
                HISTORY,
                id,
                Some("history"),
                history.source_location.as_ref().or(at),
            );
        }
    }

    /// What one transition can contradict, and every descriptor of it that
    /// matches no event the machine names.
    fn transition(&mut self, state: &State, transition: &Transition, events: &[String]) {
        let at = transition.source_location.as_ref();
        let id = state.id.as_str();
        let written = transition.event.split_whitespace().collect::<Vec<_>>();
        let construct = if written.is_empty() {
            format!("the <transition> without an event of state '{id}'")
        } else {
            format!(
                "the <transition event=\"{}\"> of state '{id}'",
                written.join(" ")
            )
        };
        if written.is_empty() {
            self.rule(construct.clone(), EVENTLESS, id, Some("transition"), at);
        }
        if !transition.cond.is_empty() {
            self.rule(
                construct.clone(),
                COND,
                id,
                Some(transition.cond.as_str()),
                at,
            );
        }
        if transition.targets.len() > 1 {
            self.rule(
                construct.clone(),
                TARGETS,
                id,
                Some(transition.targets.join(" ").as_str()),
                at,
            );
        }
        if !transition.actions.is_empty() {
            self.rule(construct, CONTENT, id, Some("transition"), at);
        }
        for descriptor in written {
            let parsed = EventDescriptor::parse(descriptor);
            if events.iter().any(|event| parsed.matches(event)) {
                continue;
            }
            self.push(
                ScxmlSemanticError::TransitionFunctionUnmatchedDescriptor {
                    state: id.to_string(),
                    descriptor: descriptor.to_string(),
                    events: events.to_vec(),
                },
                at,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forge::diagnostic::ToDiagnostics;
    use crate::parser::SCXMLParser;

    /// The code and `actual` of every record `findings` returns for `doc`.
    fn refused(doc: &str) -> Vec<(String, Option<String>)> {
        let mut model = SCXMLParser::new()
            .parse_string(doc, "scxml_transition_function")
            .expect("parse");
        crate::analyzer::analyze(&mut model, "");
        findings(&model, "scxml_transition_function")
            .into_iter()
            .map(|err| {
                let record = err.error.to_diagnostics().remove(0);
                (record.code.as_str().to_string(), record.actual)
            })
            .collect()
    }

    /// A document that declares the function, with `body` as its states.
    fn declared(datamodel: &str, body: &str) -> String {
        format!(
            r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:transition-function="emit" version="1.0" initial="idle" datamodel="{datamodel}">
{body}
</scxml>"#
        )
    }

    /// The pure part every refusal below is added to: `idle` and `busy` take
    /// each other on `go` and `back`, and `stop` ends the machine.
    const PURE: &str = r#"
  <state id="idle"><transition event="go" target="busy"/></state>
  <state id="busy">
    <transition event="back" target="idle"/>
    <transition event="stop" target="done"/>
  </state>
  <final id="done"/>"#;

    const RULE: &str = "scxml/transition-function-rule";

    fn has(found: &[(String, Option<String>)], code: &str, actual: &str) -> bool {
        found
            .iter()
            .any(|(c, a)| c == code && a.as_deref() == Some(actual))
    }

    #[test]
    fn a_pure_document_is_refused_nothing() {
        let found = refused(&declared("null", PURE));
        assert!(found.is_empty(), "{found:?}");
    }

    #[test]
    fn a_document_that_declares_nothing_is_judged_by_nothing_here() {
        let undeclared = declared(
            "null",
            &format!("{PURE}<parallel id=\"p\"><state id=\"a\"/><state id=\"b\"/></parallel>"),
        )
        .replace(r#"sce:transition-function="emit""#, "");
        let mut model = SCXMLParser::new()
            .parse_string(&undeclared, "undeclared")
            .expect("parse");
        crate::analyzer::analyze(&mut model, "");
        assert!(findings(&model, "undeclared").is_empty());
    }

    #[test]
    fn a_parallel_is_refused() {
        let doc = declared(
            "null",
            &format!(
                r#"{PURE}
  <state id="side"><transition event="split" target="both"/></state>
  <parallel id="both"><state id="r1"/><state id="r2"/></parallel>"#
            ),
        );
        assert!(has(&refused(&doc), RULE, "parallel"));
    }

    #[test]
    fn a_history_is_refused() {
        let doc = declared(
            "null",
            r#"
  <state id="idle"><transition event="go" target="h"/></state>
  <state id="outer">
    <history id="h"><transition target="inner"/></history>
    <state id="inner"><transition event="back" target="idle"/></state>
  </state>"#,
        );
        assert!(has(&refused(&doc), RULE, "history"));
    }

    #[test]
    fn executable_content_is_refused_wherever_it_is_written() {
        let doc = declared(
            "ecmascript",
            r#"
  <datamodel><data id="count" expr="0"/></datamodel>
  <state id="idle">
    <onentry><raise event="poke"/></onentry>
    <onexit><raise event="poke"/></onexit>
    <transition event="go" target="busy"><raise event="poke"/></transition>
    <transition event="poke"/>
  </state>
  <state id="busy">
    <datamodel><data id="local" expr="1"/></datamodel>
    <initial><transition target="inner"><raise event="poke"/></transition></initial>
    <state id="inner">
      <invoke type="x-job" id="job"/>
      <transition event="back" target="idle"/>
    </state>
  </state>"#,
        );
        let found = refused(&doc);
        for observed in [
            "data",
            "onentry",
            "onexit",
            "transition",
            "initial",
            "invoke",
        ] {
            assert!(has(&found, RULE, observed), "{observed}: {found:?}");
        }
        // Two `<data>`: the document's and `busy`'s.
        assert_eq!(
            found
                .iter()
                .filter(|(c, a)| c == RULE && a.as_deref() == Some("data"))
                .count(),
            2,
            "{found:?}"
        );
    }

    #[test]
    fn an_empty_onentry_runs_nothing_and_is_accepted() {
        let doc = declared(
            "null",
            r#"
  <state id="idle"><onentry/><onexit/><transition event="go" target="busy"/></state>
  <state id="busy"><transition event="back" target="idle"/></state>"#,
        );
        let found = refused(&doc);
        assert!(found.is_empty(), "{found:?}");
    }

    #[test]
    fn a_top_level_script_and_a_donedata_are_refused() {
        let doc = declared(
            "ecmascript",
            r#"
  <script>var x = 1;</script>
  <state id="idle"><transition event="stop" target="done"/></state>
  <final id="done"><donedata><content expr="1"/></donedata></final>"#,
        );
        let found = refused(&doc);
        assert!(has(&found, RULE, "script"), "{found:?}");
        assert!(has(&found, RULE, "donedata"), "{found:?}");
    }

    #[test]
    fn an_eventless_transition_and_a_guard_are_refused() {
        let doc = declared(
            "null",
            r#"
  <state id="idle">
    <transition target="busy"/>
    <transition event="go" cond="In('idle')" target="busy"/>
  </state>
  <state id="busy"><transition event="back" target="idle"/></state>"#,
        );
        let found = refused(&doc);
        assert!(has(&found, RULE, "transition"), "{found:?}");
        assert!(has(&found, RULE, "In('idle')"), "{found:?}");
    }

    #[test]
    fn a_final_inside_a_compound_state_is_refused() {
        let doc = declared(
            "null",
            r#"
  <state id="idle">
    <transition event="go" target="end"/>
    <state id="inner"><transition event="back" target="idle"/></state>
    <final id="end"/>
  </state>"#,
        );
        assert!(has(&refused(&doc), RULE, "final"));
    }

    #[test]
    fn a_transition_with_several_targets_is_refused() {
        let doc = declared(
            "null",
            r#"
  <state id="idle"><transition event="split" target="r1 r2"/></state>
  <parallel id="both">
    <state id="r1"><transition event="back" target="idle"/></state>
    <state id="r2"/>
  </parallel>"#,
        );
        assert!(has(&refused(&doc), RULE, "r1 r2"));
    }

    #[test]
    fn a_descriptor_that_matches_no_event_is_refused() {
        // `door.*` is a family nothing fills; `go.*` is filled by `go.now`.
        let doc = declared(
            "null",
            r#"
  <state id="idle">
    <transition event="go.now" target="busy"/>
    <transition event="door.*" target="busy"/>
  </state>
  <state id="busy">
    <transition event="go.*" target="idle"/>
  </state>"#,
        );
        let found = refused(&doc);
        assert!(
            has(
                &found,
                "scxml/transition-function-unmatched-descriptor",
                "door.*"
            ),
            "{found:?}"
        );
        assert!(
            !has(
                &found,
                "scxml/transition-function-unmatched-descriptor",
                "go.*"
            ),
            "{found:?}"
        );
    }

    #[test]
    fn a_wildcard_in_a_machine_that_names_no_event_matches_nothing() {
        let doc = declared(
            "null",
            r#"
  <state id="idle"><transition event="*" target="done"/></state>
  <final id="done"/>"#,
        );
        assert!(has(
            &refused(&doc),
            "scxml/transition-function-unmatched-descriptor",
            "*"
        ));
    }

    #[test]
    fn a_state_no_event_leaves_is_a_dead_end_and_a_final_is_not() {
        let doc = declared(
            "null",
            r#"
  <state id="idle"><transition event="go" target="stuck"/><transition event="stop" target="done"/></state>
  <state id="stuck"/>
  <final id="done"/>"#,
        );
        let found = refused(&doc);
        assert_eq!(
            found,
            [(
                "scxml/transition-function-dead-end".to_string(),
                Some("stuck".to_string())
            )]
        );
    }

    #[test]
    fn a_transition_held_by_an_ancestor_is_a_way_out() {
        let doc = declared(
            "null",
            r#"
  <state id="idle" initial="a">
    <transition event="reset" target="idle"/>
    <state id="a"/>
  </state>"#,
        );
        let found = refused(&doc);
        assert!(found.is_empty(), "{found:?}");
    }

    #[test]
    fn an_unreachable_state_is_refused_without_lint() {
        let doc = declared(
            "null",
            &format!(
                "{PURE}\n  <state id=\"orphan\"><transition event=\"go\" target=\"idle\"/></state>"
            ),
        );
        assert!(has(&refused(&doc), "scxml/dead-transition", "idle"));
    }

    /// Every refusal comes in one answer, not the first alone.
    #[test]
    fn every_refusal_is_reported_at_once() {
        let doc = declared(
            "null",
            r#"
  <state id="idle">
    <onentry><raise event="poke"/></onentry>
    <transition event="go" target="both"/>
    <transition target="idle" cond="In('idle')"/>
    <transition event="split" target="r1 r2"/>
    <transition event="door.*" target="idle"/>
  </state>
  <parallel id="both">
    <state id="r1"/>
    <state id="r2"/>
  </parallel>
  <state id="outer">
    <history id="h"><transition target="inner"/></history>
    <state id="inner"><transition event="go" target="done"/></state>
    <final id="done"/>
  </state>"#,
        );
        let mut codes: Vec<String> = refused(&doc).into_iter().map(|(code, _)| code).collect();
        codes.sort();
        assert_eq!(
            codes,
            [
                "scxml/dead-transition",
                "scxml/transition-function-dead-end",
                "scxml/transition-function-dead-end",
                "scxml/transition-function-rule",
                "scxml/transition-function-rule",
                "scxml/transition-function-rule",
                "scxml/transition-function-rule",
                "scxml/transition-function-rule",
                "scxml/transition-function-rule",
                "scxml/transition-function-rule",
                "scxml/transition-function-unmatched-descriptor",
                "scxml/unreachable-state",
                "scxml/unreachable-state",
            ]
        );
    }
}

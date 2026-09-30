// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A `<send>` a statechart addresses to itself that nothing takes —
//! `scxml/self-send-discarded`, a design-time lint — and the one reading of
//! "discarded" the closed interface shares.
//!
//! # The case, measured
//!
//! Drafted statecharts wrote their outputs as `<send event="…"/>` with no
//! target: the prose said what the machine announces and not to whom, and a
//! `<send>` with no `target` is a message to the session itself. No
//! transition took any of them, so every output was queued and thrown away
//! — and `check --lint` accepted them, because the document is valid SCXML
//! with a defined meaning. All five drafts of one vending specification
//! (2026-09-29) did it. The closed interface ([`crate::scxml_interface`])
//! refuses it where it is declared; this lint says it for every statechart
//! an author asks design advice about.
//!
//! # What is flagged
//!
//! A `<send>` with a literal `event` that goes to the session itself
//! ([`sends_to_itself`]) and that no transition of the document takes, by
//! any of its descriptors — a prefix and `*` included, and in any state,
//! since which state is active when it arrives is not decided here. One
//! the session's children receive instead is not discarded
//! ([`is_discarded`]). An `eventexpr` names its event at runtime and is not
//! judged.

use crate::event_descriptor::attribute_matches;
use crate::forge::error::{ForgeError, Located, SourceLocation};
use crate::host_processor_analyzer::{walk_model_actions, SCXML_EVENT_PROCESSOR_TYPE};
use crate::model::{Action, SCXMLModel};
use crate::scxml_semantic::ScxmlSemanticError;

/// Whether a `<send>` goes to this session's own queue.
///
/// §scxml-6.2.4: with neither `target` nor `targetexpr`, the event is added
/// to the external queue of the sending session, and `#_internal` names its
/// internal queue — both the session itself. That holds only through the
/// SCXML Event I/O Processor, written or defaulted; a `type` or `typeexpr`
/// hands the event to another processor, which decides where it goes.
pub(crate) fn sends_to_itself(action: &Action) -> bool {
    action.targetexpr.is_empty()
        && action.typeexpr.is_empty()
        && (action.target.is_empty() || action.target == "#_internal")
        && (action.send_type.is_empty() || action.send_type == SCXML_EVENT_PROCESSOR_TYPE)
}

/// Whether `action` is a `<send>` whose event the session sends itself and
/// then throws away: it goes to the session's own queue
/// ([`sends_to_itself`]), names its event literally, and no transition in
/// the document takes it.
///
/// ⚠ Not when a child receives it. §scxml-6.4.1: an `<invoke
/// autoforward="true">` forwards every external event the invoking session
/// takes to the child, so an event sent to the external queue reaches that
/// child whether or not a transition here takes it. Which invoke is active
/// when it arrives is a runtime fact, so any autoforwarding invoke in the
/// document is enough. `#_internal` goes to the internal queue, which is
/// never forwarded.
pub(crate) fn is_discarded(model: &SCXMLModel, action: &Action) -> bool {
    if action.action_type != "send"
        || !action.eventexpr.is_empty()
        || action.event.is_empty()
        || !sends_to_itself(action)
    {
        return false;
    }
    let taken = model.states.values().any(|state| {
        state
            .transitions
            .iter()
            .any(|t| attribute_matches(&t.event, &action.event))
    });
    let forwarded = action.target != "#_internal"
        && model
            .states
            .values()
            .any(|state| state.invokes.iter().any(|invoke| invoke.autoforward()));
    !taken && !forwarded
}

/// Every discarded self-send, located on its `<send>`, in the order the
/// states are written.
pub fn findings(model: &SCXMLModel, source: &str) -> Vec<Located<ForgeError>> {
    let mut found: Vec<(u32, String, String, Option<SourceLocation>)> = Vec::new();
    walk_model_actions(model, &mut |state, action| {
        if is_discarded(model, action) {
            let order = model.states.get(state).map_or(0, |s| s.document_order);
            found.push((
                order,
                state.to_string(),
                action.event.clone(),
                action.source_location.clone(),
            ));
        }
    });
    // `walk_model_actions` walks the state map, whose order is the ids';
    // the author reads the document, so the findings follow it.
    found.sort_by_key(|(order, _, _, at)| {
        (
            *order,
            at.as_ref().and_then(|at| at.line),
            at.as_ref().and_then(|at| at.col),
        )
    });
    found
        .into_iter()
        .map(|(_, state, event, at)| {
            model.locate(
                ScxmlSemanticError::SelfSendDiscarded { event, state }.into(),
                at.as_ref(),
                source,
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::SCXMLParser;

    fn found(body: &str) -> Vec<String> {
        let m = SCXMLParser::new()
            .parse_string(body, "self_send")
            .expect("parses");
        findings(&m, body)
            .into_iter()
            .map(|e| e.error.to_string())
            .collect()
    }

    /// A draft that announces its output to the machine itself, and waits
    /// for a completion that nothing can send.
    const GATE: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
        initial="idle">
  <state id="idle">
    <transition event="start" target="waiting"/>
  </state>
  <state id="waiting">
    <onentry><send event="announce"/></onentry>
    <transition event="finish" target="idle"/>
  </state>
</scxml>"#;

    #[test]
    fn an_output_sent_to_itself_that_nothing_takes_is_flagged() {
        let f = found(GATE);
        assert_eq!(f.len(), 1, "{f:?}");
        assert!(
            f[0].contains("'waiting'") && f[0].contains("'announce'"),
            "{f:?}"
        );
    }

    /// The same event through `#_internal`, and after a delay: still the
    /// session itself.
    #[test]
    fn the_internal_queue_and_a_delay_are_the_session_too() {
        for send in [
            r##"<send event="announce" target="#_internal"/>"##,
            r#"<send event="announce" delay="1s"/>"#,
        ] {
            let body = GATE.replace(r#"<send event="announce"/>"#, send);
            assert_eq!(found(&body).len(), 1, "{send}");
        }
    }

    /// Taken by a transition — literally, by a prefix, or by `*` — it is a
    /// message the machine acts on, not a lost output.
    #[test]
    fn a_self_send_some_transition_takes_is_not_flagged() {
        for taker in ["announce", "announce.*", "announce.", "*"] {
            let body = GATE.replace(
                r#"<transition event="finish" target="idle"/>"#,
                &format!(
                    r#"<transition event="finish" target="idle"/><transition event="{taker}"/>"#
                ),
            );
            assert!(found(&body).is_empty(), "{taker}: {:?}", found(&body));
        }
        let body = GATE.replace(
            r#"<transition event="finish" target="idle"/>"#,
            r#"<transition event="announceX"/>"#,
        );
        assert_eq!(found(&body).len(), 1, "another token is another event");
    }

    /// Addressed elsewhere, it leaves the session: a parent, another
    /// processor, or a target computed at runtime.
    #[test]
    fn a_send_that_leaves_the_session_is_not_flagged() {
        for send in [
            r##"<send event="announce" target="#_parent"/>"##,
            r#"<send event="announce" type="http://www.w3.org/TR/scxml/#BasicHTTPEventProcessor" target="http://host/in"/>"#,
            r#"<send event="announce" type="x-host-bus"/>"#,
            r#"<send event="announce" targetexpr="'#_parent'"/>"#,
        ] {
            let body = GATE.replace(r#"<send event="announce"/>"#, send);
            assert!(found(&body).is_empty(), "{send}: {:?}", found(&body));
        }
    }

    /// An autoforwarding child receives every external event (W3C SCXML
    /// 6.4.1), so the send reaches it; the internal queue is not forwarded.
    #[test]
    fn an_autoforwarding_child_receives_what_the_external_queue_gets() {
        let body = GATE.replace(
            r#"<transition event="finish" target="idle"/>"#,
            r#"<invoke type="scxml" autoforward="true"><content><scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"><final id="f"/></scxml></content></invoke>
    <transition event="finish" target="idle"/>"#,
        );
        assert!(found(&body).is_empty(), "{:?}", found(&body));
        let internal = body.replace(
            r#"<send event="announce"/>"#,
            r##"<send event="announce" target="#_internal"/>"##,
        );
        assert_eq!(found(&internal).len(), 1);
    }

    /// A computed name cannot be judged, and is not.
    #[test]
    fn a_computed_event_name_is_not_judged() {
        let body = GATE.replace(
            r#"<send event="announce"/>"#,
            r#"<send eventexpr="'Send' + 'Request'"/>"#,
        );
        assert!(found(&body).is_empty());
    }
}

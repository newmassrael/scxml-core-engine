// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A recording an inner transition intercepts —
//! `scxml/recording-intercepted`, a design-time lint.
//!
//! # The case, measured
//!
//! A door lock kept the vehicle speed by recording it on the outer state:
//! `<state id="released"><transition event="speed.update"><assign
//! location="speed" …/>`. Inside it, `unlocked` locked itself on a fast
//! `speed.update`. Driven through the generated code (2026-09-28): a slow
//! update recorded 10, a fast one (20) locked the door — and left `speed`
//! at 10, so the next unlock request passed its "below 15" guard and
//! unlocked the door at 20 km/h. The same machine with the recording in a
//! `<parallel>` region of its own stayed locked.
//!
//! §scxml-3.13 (transition selection, Appendix D `selectTransitions`): for
//! each atomic state the first enabled transition is taken walking OUT from
//! it, so while `unlocked` is active its own `speed.update` transition is
//! the one taken and `released`'s is never considered. The document is
//! valid and every structural check passes; the value is simply not
//! recorded. Nothing but running it showed it — hence this lint.
//!
//! # What is flagged
//!
//! An outer state's transition on event `E` whose executable content
//! assigns a location `L`, and a proper descendant with a transition whose
//! descriptors overlap `E`'s and which does not itself assign `L` — reached
//! from the outer state through `<state>`s only. A `<parallel>` on the way,
//! or as the outer state, is the other design the measurement vindicated:
//! each region selects its own transition, so the recording region is not
//! pre-empted by a sibling's, and the cross-region case depends on conflict
//! resolution this walk does not model — it is not flagged rather than
//! guessed. The descendant's transition is reported, located on its
//! `<transition>`, once per outer transition: that is where the author
//! either records `L` too or decides the skip is intended.

use crate::event_descriptor::descriptors;
use crate::forge::error::{ForgeError, Located};
use crate::model::{Action, SCXMLModel, State};
use crate::scxml_semantic::ScxmlSemanticError;

/// Every interception, outer state by outer state in document order, then
/// its transitions in order, then the intercepting descendants in document
/// order.
pub fn findings(model: &SCXMLModel, source: &str) -> Vec<Located<ForgeError>> {
    let mut states: Vec<&State> = model.states.values().collect();
    states.sort_by_key(|s| s.document_order);

    let mut out = Vec::new();
    for outer in &states {
        if outer.is_parallel {
            continue;
        }
        for recording in &outer.transitions {
            if recording.event.trim().is_empty() {
                continue;
            }
            let recorded = assigned_locations(&recording.actions);
            if recorded.is_empty() {
                continue;
            }
            for inner in states
                .iter()
                .filter(|s| reached_through_states(model, s, &outer.id))
            {
                let Some(intercepting) = inner.transitions.iter().find(|t| {
                    descriptors(&t.event)
                        .any(|d| descriptors(&recording.event).any(|r| r.overlaps(&d)))
                }) else {
                    continue;
                };
                let also = assigned_locations(&intercepting.actions);
                let skipped: Vec<&str> = recorded
                    .iter()
                    .copied()
                    .filter(|l| !also.contains(l))
                    .collect();
                if skipped.is_empty() {
                    continue;
                }
                out.push(
                    model.locate(
                        ScxmlSemanticError::RecordingIntercepted {
                            outer: outer.id.clone(),
                            event: recording.event.trim().to_string(),
                            locations: skipped.join(", "),
                            inner: inner.id.clone(),
                        }
                        .into(),
                        intercepting.source_location.as_ref(),
                        source,
                    ),
                );
            }
        }
    }
    out
}

/// The locations `actions` assign, nested blocks included, in document
/// order and each once.
fn assigned_locations(actions: &[Action]) -> Vec<&str> {
    let mut out: Vec<&str> = Vec::new();
    for action in actions {
        action.walk(&mut |a| {
            if a.action_type == "assign" && !a.location.is_empty() && !out.contains(&&*a.location) {
                out.push(&a.location);
            }
        });
    }
    out
}

/// Whether `inner` is a proper descendant of `outer` whose ancestors up to
/// `outer` are all `<state>`s — no `<parallel>` between them.
fn reached_through_states(model: &SCXMLModel, inner: &State, outer: &str) -> bool {
    let mut at = inner.parent.as_deref();
    while let Some(id) = at {
        if id == outer {
            return true;
        }
        match model.states.get(id) {
            Some(s) if !s.is_parallel => at = s.parent.as_deref(),
            _ => return false,
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::SCXMLParser;

    fn found(body: &str) -> Vec<String> {
        let m = SCXMLParser::new()
            .parse_string(body, "interception")
            .expect("parses");
        findings(&m, body)
            .into_iter()
            .map(|e| e.error.to_string())
            .collect()
    }

    /// The measured door lock: the fast update is taken by `unlocked`, and
    /// `speed` is not recorded.
    const LOCK: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
        initial="released" datamodel="ecmascript">
  <datamodel><data id="speed" expr="0"/></datamodel>
  <state id="released" initial="unlocked">
    <transition event="speed.update"><assign location="speed" expr="_event.data.v"/></transition>
    <state id="unlocked">
      <transition event="speed.update" cond="_event.data.v &gt;= 15" target="locked"/>
    </state>
    <state id="locked"/>
  </state>
</scxml>"#;

    #[test]
    fn an_inner_transition_on_the_recorded_event_is_flagged() {
        let f = found(LOCK);
        assert_eq!(f.len(), 1, "{f:?}");
        assert!(
            f[0].contains("'released'") && f[0].contains("'unlocked'"),
            "{f:?}"
        );
        assert!(f[0].contains("speed"), "{f:?}");
    }

    /// A descriptor that covers the event intercepts it too: `speed`
    /// matches `speed.update`.
    #[test]
    fn a_prefix_descriptor_intercepts() {
        let body = LOCK.replace(
            r#"<transition event="speed.update" cond"#,
            r#"<transition event="speed" cond"#,
        );
        assert_eq!(found(&body).len(), 1);
        let body = LOCK.replace(
            r#"<transition event="speed.update" cond"#,
            r#"<transition event="speedo" cond"#,
        );
        assert!(
            found(&body).is_empty(),
            "a different token is a different event"
        );
    }

    /// Recording it again on the inner transition is the repair.
    #[test]
    fn an_inner_transition_that_records_it_too_is_not_flagged() {
        let body = LOCK.replace(
            r#"target="locked"/>"#,
            r#"target="locked"><assign location="speed" expr="_event.data.v"/></transition>"#,
        );
        assert!(found(&body).is_empty(), "{:?}", found(&body));
    }

    /// The design the measurement vindicated: the recording in a region of
    /// its own, beside the region that locks.
    #[test]
    fn a_parallel_between_them_is_not_flagged() {
        let body = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
        initial="run" datamodel="ecmascript">
  <datamodel><data id="speed" expr="0"/></datamodel>
  <state id="run" initial="p">
    <transition event="speed.update"><assign location="speed" expr="_event.data.v"/></transition>
    <parallel id="p">
      <state id="lock" initial="unlocked">
        <state id="unlocked"><transition event="speed.update" target="locked"/></state>
        <state id="locked"/>
      </state>
      <state id="meter"/>
    </parallel>
  </state>
</scxml>"#;
        assert!(found(body).is_empty(), "{:?}", found(body));
    }

    /// No assignment, nothing to lose: an outer transition that only moves
    /// is not a recording.
    #[test]
    fn an_outer_transition_that_records_nothing_is_not_flagged() {
        let body = LOCK.replace(
            r#"<transition event="speed.update"><assign location="speed" expr="_event.data.v"/></transition>"#,
            r#"<transition event="speed.update" target="locked"/>"#,
        );
        assert!(found(&body).is_empty());
    }
}

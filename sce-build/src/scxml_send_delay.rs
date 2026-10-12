// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A `<send>` whose literal `delay` is not a time — `scxml/send-delay-invalid`,
//! a design-time lint.
//!
//! # The case, measured
//!
//! A drafted statechart armed its ten-second wait as
//! `<send target="#_internal" delay="10000" event="hold.done"/>`: a number and
//! no unit. §scxml-6.2.4 makes `delay` a CSS2 time, and a bare number is not one,
//! so every backend raises the argument error at that `<send>` instead of
//! scheduling anything (`Action::delay_invalid`). The document's own `*`
//! handler took the error event and answered as it answered any event: the
//! wait was never armed, the position it guarded was written at 0 ms where ten
//! seconds were expected, and `check --lint` said nothing, because the document
//! is valid SCXML with a defined meaning (2026-10-12, a Qwen draft of
//! `Washer_Low_Warning`: two of four cases failed, and `verify` could say
//! only "got at 0 ms").
//!
//! # What is flagged
//!
//! A `<send>` with a `delay` written, no `delayexpr`, and a value
//! [`crate::parser`] could not read as a CSS2 time. The parser decides it once
//! and this lint relays it, so the lint, the generated code and the
//! Interpreter cannot disagree about what a time is; the grammar stays
//! ARCHITECTURE.md's "Durations", held to `tests/durations/css2_time.json`. A
//! `delayexpr` is computed at run time and is not judged.

use crate::forge::error::{ForgeError, Located, SourceLocation};
use crate::host_processor_analyzer::walk_model_actions;
use crate::model::SCXMLModel;
use crate::scxml_semantic::ScxmlSemanticError;

/// Every `<send>` whose literal `delay` is not a CSS2 time, located on the
/// `<send>`, in the order the states are written.
pub fn findings(model: &SCXMLModel, source: &str) -> Vec<Located<ForgeError>> {
    let mut found: Vec<(u32, String, String, Option<SourceLocation>)> = Vec::new();
    walk_model_actions(model, &mut |state, action| {
        // A `delayexpr` wins over a `delay` in every backend's send, so a send
        // that carries one is read at run time and not here.
        if action.action_type == "send" && action.delay_invalid && action.delayexpr.is_empty() {
            let order = model.states.get(state).map_or(0, |s| s.document_order);
            found.push((
                order,
                state.to_string(),
                action.delay.clone(),
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
        .map(|(_, state, delay, at)| {
            model.locate(
                ScxmlSemanticError::SendDelayInvalid { delay, state }.into(),
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
            .parse_string(body, "send_delay")
            .expect("parses");
        findings(&m, body)
            .into_iter()
            .map(|e| e.error.to_string())
            .collect()
    }

    /// A wait armed on entering a state and answered by a transition.
    const WAIT: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
        initial="idle">
  <state id="idle">
    <transition event="start" target="waiting"/>
  </state>
  <state id="waiting">
    <onentry><send event="hold.done" delay="10s"/></onentry>
    <transition event="hold.done" target="idle"/>
  </state>
</scxml>"#;

    /// The draft that was measured: a number with no unit.
    #[test]
    fn a_delay_with_no_unit_is_flagged_and_named() {
        let body = WAIT.replace(r#"delay="10s""#, r#"delay="10000""#);
        let f = found(&body);
        assert_eq!(f.len(), 1, "{f:?}");
        assert!(
            f[0].contains("'waiting'") && f[0].contains(r#"delay="10000""#),
            "{f:?}"
        );
    }

    /// Anything the grammar does not read as a time is the same finding: a
    /// unit it does not know, a sign, an exponent, a stray word, nothing.
    #[test]
    fn what_the_grammar_does_not_read_is_flagged() {
        for delay in [
            "10", "10 s", "10sec", "-1s", "1e3ms", "soon", "", "s", "1.s",
        ] {
            let body = WAIT.replace(r#"delay="10s""#, &format!(r#"delay="{delay}""#));
            assert_eq!(found(&body).len(), 1, "{delay:?}");
        }
    }

    /// The forms the grammar reads, with its whitespace and case rules.
    #[test]
    fn a_time_the_grammar_reads_is_not_flagged() {
        for delay in [
            "10s", "10000ms", "1.5s", ".5s", " 500ms ", "2S", "7MS", "0s",
        ] {
            let body = WAIT.replace(r#"delay="10s""#, &format!(r#"delay="{delay}""#));
            assert!(found(&body).is_empty(), "{delay:?}: {:?}", found(&body));
        }
    }

    /// A time computed at run time cannot be judged here, and a send with no
    /// delay has nothing to judge.
    #[test]
    fn a_computed_delay_and_no_delay_are_not_judged() {
        let computed = WAIT.replace(r#"delay="10s""#, r#"delayexpr="'10000'""#);
        assert!(found(&computed).is_empty());
        let none = WAIT.replace(r#" delay="10s""#, "");
        assert!(found(&none).is_empty());
    }

    /// Each send is its own finding, in the order the document writes them.
    #[test]
    fn every_send_is_a_finding_in_document_order() {
        let body = WAIT.replace(
            r#"<send event="hold.done" delay="10s"/>"#,
            r#"<send event="hold.done" delay="10"/><send event="again" delay="5"/>"#,
        );
        let f = found(&body);
        assert_eq!(f.len(), 2, "{f:?}");
        assert!(
            f[0].contains(r#"delay="10""#) && f[1].contains(r#"delay="5""#),
            "{f:?}"
        );
    }
}

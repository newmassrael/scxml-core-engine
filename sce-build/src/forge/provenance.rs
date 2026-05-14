// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// Watching-zenoh RFC §5.O Atomic 0 — IR provenance pre-emit guard.
//
// Codegen consumes per-IR-node `source_location` to emit SCE-MAP
// markers above every generated function header (Atomic 0a) and to
// drive the per-symbol attribution sourcemap JSON (Atomic 1). The
// markers are silent when the IR field is `None`, which would
// regress the spec contract (lines 3280-3284 + 3289-3290) without
// any visible signal — a classic silently-broken-hook situation
// [[feedback-silently-broken-hooks]].
//
// This pre-emit walker is the consumer that prevents that regression.
// It runs after analyzer and before generator. Every node eligible for
// marker emission must carry `source_location: Some(_)`; the first
// `None` fires `traceability/scxml-line-range-missing` (a
// codegen-internal diagnostic with no author repair — the fix lives
// in the parser site that produced the IR node).
//
// ## Eligibility scope (Atomic 0a)
//
// Atomic 0a populates the IR provenance field at four parser sites
// in `parser.rs`:
//
// 1. `SCXMLModel` root  — every parse goes through `parse_impl`.
// 2. `State` (state/final/parallel) — every `<state>`, `<final>`,
//    `<parallel>` reaches `parse_states`'s three creation sites.
// 3. `Transition` — every `<transition>` reaches `parse_transition`.
// 4. `Action` (executable content) — every `<raise>`, `<send>`,
//    `<assign>`, `<log>`, `<script>`, `<if>`, `<foreach>`,
//    `<cancel>` reaches `parse_executable_content_single`.
//
// Synthesised IR nodes (e.g. detect_features rewrites,
// resolve_history_targets stubs) that do not derive from a single
// authored XML element are *intentionally* left `None` and excluded
// from this walker per the inherited-content-branch carve-out
// documented in spec line 3286-3289 ("XInclude / sce:template
// composition MUST track per-element coordinates and attach them to
// every IR node"). The walker covers only the four types above for
// Atomic 0a; Atomic 0b extends to the per-action attribution emit.

use crate::forge::error::{ForgeError, Located, ValidationError};
use crate::model::{Action, SCXMLModel, State, Transition};

/// Walk every emission-eligible node in `model` and assert that
/// `source_location` is populated. On the first offender, return a
/// `Located<ForgeError>` carrying [`ValidationError::TraceabilityScxmlLineRangeMissing`]
/// pinned at `scxml_path`. Author callers see the codegen-internal
/// diagnostic on the wire; the actual fix lives in the parser site
/// that produced the IR node.
///
/// First-failure short-circuit: there is no value in accumulating
/// multiple instances of the same parser-site regression, and
/// downstream codegen (which the call site is about to invoke) would
/// fail anyway. The single emitted diagnostic identifies the node
/// kind + id so an author or CI consumer can pinpoint the parser
/// site that needs the populate.
///
/// `scxml_path` is the diagnostic label (mirrors the call site's
/// `source_name` / `diag_label` threading) so the wire payload's
/// `location.file` matches the rest of the parser's `Located::new`
/// raises.
pub fn validate_emission_provenance(
    model: &SCXMLModel,
    scxml_path: &str,
) -> Result<(), Located<ForgeError>> {
    // 1. Root SCXMLModel provenance — the top-level state machine
    //    function emission consumes this.
    if model.source_location.is_none() {
        return Err(emit(scxml_path, "<scxml>", &model.name));
    }

    // 2. Per-state provenance — every state lowers to at least one
    //    per-state function (on_entry / on_exit). The state's own
    //    source position drives every marker that function family
    //    needs; the per-action source position (covered below) is
    //    consumed by Atomic 0b's per-statement attribution.
    for (state_id, state) in &model.states {
        if state.source_location.is_none() {
            let kind = state_kind_label(state);
            return Err(emit(scxml_path, kind, state_id));
        }

        // 3. Per-transition provenance — every transition lowers
        //    to its own handler function in most backends, so each
        //    transition needs a marker. The pinned id is the parent
        //    state id + the event / target / index triple so the
        //    diagnostic uniquely identifies the parser site.
        for (idx, trans) in state.transitions.iter().enumerate() {
            if trans.source_location.is_none() {
                let pinned = format_transition_id(state_id, trans, idx);
                return Err(emit(scxml_path, "<transition>", &pinned));
            }
        }

        // 4. Per-action provenance — flat first-level walk over the
        //    state's executable-content blocks (on_entry, on_exit,
        //    initial transition actions, and per-transition actions).
        //    Nested if/foreach bodies hold their own per-action
        //    records; the walker recurses through them too so the
        //    populate gap is caught wherever it occurs.
        for block in state.on_entry_blocks.iter().chain(state.on_exit_blocks.iter()) {
            check_action_block(block, state_id, scxml_path)?;
        }
        check_action_block(&state.initial_transition_actions, state_id, scxml_path)?;
        for trans in &state.transitions {
            check_action_block(&trans.actions, state_id, scxml_path)?;
        }
    }

    // 5. Global script actions — top-level `<script>` elements live
    //    on `model.global_scripts`, outside the per-state walk.
    check_action_block(&model.global_scripts, "<scxml>", scxml_path)?;

    Ok(())
}

/// Build the Located<ForgeError> wrapping
/// [`ValidationError::TraceabilityScxmlLineRangeMissing`] with the
/// node kind / id pinned at `scxml_path`. Centralised so every fire
/// site emits the same wire shape.
fn emit(scxml_path: &str, node_kind: &'static str, node_id: &str) -> Located<ForgeError> {
    Located::new(
        ValidationError::TraceabilityScxmlLineRangeMissing {
            node_kind,
            node_id: node_id.to_string(),
        }
        .into(),
        scxml_path,
        None,
        None,
    )
}

/// Recursive walk over an action vector. Each leaf must carry
/// `source_location: Some(_)`; the per-block check descends into
/// `if` branches and `foreach` bodies so a `None` in a nested
/// `<assign>` inside an `<if>` does not slip past.
fn check_action_block(
    actions: &[Action],
    state_id: &str,
    scxml_path: &str,
) -> Result<(), Located<ForgeError>> {
    for action in actions {
        if action.source_location.is_none() {
            let pinned = action_pinned_id(state_id, action);
            return Err(emit(scxml_path, "<action>", &pinned));
        }
        // Nested executable content — same invariant applies.
        check_action_block(&action.then_actions, state_id, scxml_path)?;
        check_action_block(&action.else_actions, state_id, scxml_path)?;
        check_action_block(&action.actions, state_id, scxml_path)?;
        for branch in &action.elseif_branches {
            check_action_block(&branch.actions, state_id, scxml_path)?;
        }
    }
    Ok(())
}

/// Map a `State` to the originating XML element name so the
/// diagnostic surfaces the same vocabulary the author wrote.
fn state_kind_label(state: &State) -> &'static str {
    if state.is_final {
        "<final>"
    } else if state.is_parallel {
        "<parallel>"
    } else {
        "<state>"
    }
}

/// Build a stable per-transition identifier for the diagnostic
/// payload. Format: `<state_id>[event=…][target=…]#<idx>`. The
/// transition index falls back when neither event nor target is
/// authored (every-event no-target transitions).
fn format_transition_id(state_id: &str, trans: &Transition, idx: usize) -> String {
    let mut s = state_id.to_string();
    if !trans.event.is_empty() {
        s.push_str("[event=");
        s.push_str(&trans.event);
        s.push(']');
    }
    if !trans.target.is_empty() {
        s.push_str("[target=");
        s.push_str(&trans.target);
        s.push(']');
    }
    s.push('#');
    s.push_str(&idx.to_string());
    s
}

/// Build a stable per-action identifier for the diagnostic payload.
/// Format: `<state_id>::<action_type>[<distinguisher>]` where the
/// distinguisher is the action's most identifying field (`event` for
/// `<raise>` / `<send>`, `location` for `<assign>`, etc.).
fn action_pinned_id(state_id: &str, action: &Action) -> String {
    let distinguisher = if !action.event.is_empty() {
        action.event.clone()
    } else if !action.location.is_empty() {
        action.location.clone()
    } else if !action.label.is_empty() {
        action.label.clone()
    } else if !action.target.is_empty() {
        action.target.clone()
    } else {
        // No identifying field on the action; the type alone has to
        // carry the diagnostic, which is acceptable because the
        // wire id additionally hashes `node_kind + node_id` into a
        // distinct content hash.
        String::new()
    };
    if distinguisher.is_empty() {
        format!("{}::{}", state_id, action.action_type)
    } else {
        format!("{}::{}[{}]", state_id, action.action_type, distinguisher)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forge::error::SourceLocation;

    /// A model populated by the real parser MUST pass the walker.
    /// This is the round-trip invariant that says: every parser site
    /// that creates an IR node populates `source_location`.
    #[test]
    fn parser_output_is_provenance_complete() {
        let scxml = r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
       initial="s1" datamodel="ecmascript">
  <state id="s1">
    <onentry>
      <log label="entered" expr="'hello'"/>
    </onentry>
    <transition event="go" target="s2"/>
  </state>
  <final id="s2">
    <onentry>
      <raise event="done"/>
    </onentry>
  </final>
</scxml>"#;
        let mut parser = crate::parser::SCXMLParser::new();
        let model = parser
            .parse_string(scxml, "fixture")
            .expect("fixture parses cleanly");
        validate_emission_provenance(&model, "fixture")
            .expect("real parser output must satisfy §5.O Atomic 0 provenance");
    }

    /// Synthesised None on the root surfaces the diagnostic with
    /// `node_kind = "<scxml>"`. Mirrors what would happen if a
    /// future `parse_impl` edit forgot to populate the root field.
    #[test]
    fn missing_root_provenance_fires_diagnostic() {
        let mut model = SCXMLModel::default();
        model.name = "broken".into();
        model.source_location = None;
        let err = validate_emission_provenance(&model, "broken.scxml")
            .expect_err("must fire when root.source_location is None");
        match &err.error {
            ForgeError::Validation(ValidationError::TraceabilityScxmlLineRangeMissing {
                node_kind,
                node_id,
            }) => {
                assert_eq!(*node_kind, "<scxml>");
                assert_eq!(node_id, "broken");
            }
            other => panic!("expected TraceabilityScxmlLineRangeMissing, got {other:?}"),
        }
    }

    /// Synthesised None on a state surfaces the diagnostic with the
    /// state's id verbatim in `node_id` so the parser-side regression
    /// is locatable from the wire payload alone.
    #[test]
    fn missing_state_provenance_fires_diagnostic() {
        let mut model = SCXMLModel::default();
        model.source_location = Some(SourceLocation {
            file: "x.scxml".into(),
            line: Some(1),
            col: Some(1),
        });
        let mut state = State::default();
        state.id = "armed".into();
        state.source_location = None;
        model.states.insert("armed".into(), state);
        let err = validate_emission_provenance(&model, "x.scxml")
            .expect_err("must fire when state.source_location is None");
        match &err.error {
            ForgeError::Validation(ValidationError::TraceabilityScxmlLineRangeMissing {
                node_kind,
                node_id,
            }) => {
                assert_eq!(*node_kind, "<state>");
                assert_eq!(node_id, "armed");
            }
            other => panic!("expected TraceabilityScxmlLineRangeMissing, got {other:?}"),
        }
    }
}

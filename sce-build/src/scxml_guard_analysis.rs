// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// NL→IR Mapping Roadmap Item 3 Phase C — Statechart guard analysis.
//
// Two rejection codes, both NeutralOrDeterministic:
//
//   * `scxml/always-false-guard` — `<transition cond="...">` whose
//     guard is statically determinable as `false` so the transition
//     can never fire. The validator stops short of full SMT and
//     recognises only structurally trivial cases:
//
//       - The literal `false` (lowercase per W3C SCXML §B
//         ECMAScript convention).
//       - The numeric literal `0`.
//       - `N == M` where both sides parse as decimal numeric
//         literals with differing values.
//       - `N != M` where both sides parse as decimal numeric
//         literals with equal values.
//
//     Language-prefixed conds (`cpp:`, `kotlin:`, `rust:`) stay
//     opaque to the validator — their semantics depend on the host
//     language's evaluator, which the parser cannot inspect.
//
//   * `scxml/shadowed-transition` — a state's transition list
//     contains an unconditional transition followed by a same-event
//     sibling. Per W3C SCXML §5.10 transition selection, the first
//     matching transition in document order wins, so the second is
//     dead. The validator requires literal equality of the `event`
//     attribute between the two transitions — token-prefix superset
//     cases depend on ancestor-priority rules the parser-stage
//     walker cannot disambiguate without running the full selection
//     algorithm, so they are deliberately not flagged.
//
// Runs after `scxml_exhaustiveness::validate` so the structural
// guards (initial state resolved, reachability sound, exhaustiveness
// satisfied) fire ahead of the guard-analysis heuristic.

use crate::forge::error::{ForgeError, Located};
use crate::model::{SCXMLModel, State};
use crate::scxml_semantic::ScxmlSemanticError;

/// Reject the document on the first guard-analysis violation.
/// Mirrors the short-circuit convention of the Phase A / B
/// validators.
pub fn validate(model: &SCXMLModel, source: &str) -> Result<(), Located<ForgeError>> {
    let mut states: Vec<&State> = model.states.values().collect();
    states.sort_by_key(|s| s.document_order);

    for state in states {
        // First-pass: always-false guards.
        for trans in &state.transitions {
            if let Some(cond) = classify_always_false(&trans.cond) {
                return Err(Located::new(
                    ScxmlSemanticError::AlwaysFalseGuard {
                        state: state.id.clone(),
                        cond,
                    }
                    .into(),
                    source,
                    None,
                    None,
                ));
            }
        }

        // Second-pass: shadowed transitions. The first unconditional
        // transition seen for each event descriptor wins; any later
        // same-event sibling is dead.
        for (i, earlier) in state.transitions.iter().enumerate() {
            if !is_unconditional(&earlier.cond) {
                continue;
            }
            for (j, later) in state.transitions.iter().enumerate().skip(i + 1) {
                if event_descriptors_match(&earlier.event, &later.event) {
                    return Err(Located::new(
                        ScxmlSemanticError::ShadowedTransition {
                            state: state.id.clone(),
                            event: later.event.clone(),
                            shadowing_index: i,
                            shadowed_index: j,
                        }
                        .into(),
                        source,
                        None,
                        None,
                    ));
                }
            }
        }
    }

    Ok(())
}

/// Classify a guard expression. Returns the raw `cond` text the
/// caller should propagate to the diagnostic when the expression is
/// statically false; otherwise `None`. Language-prefixed conds and
/// any expression the validator cannot prove false return `None` so
/// the build remains forward-compatible with author-supplied opaque
/// guards.
fn classify_always_false(cond: &str) -> Option<String> {
    let raw = cond.trim();
    if raw.is_empty() {
        return None;
    }
    // Language-prefixed conds are opaque.
    if raw.starts_with("cpp:")
        || raw.starts_with("kt:")
        || raw.starts_with("kotlin:")
        || raw.starts_with("rust:")
    {
        return None;
    }
    if raw == "false" || raw == "0" {
        return Some(cond.to_string());
    }
    // Binary equality / inequality on two decimal numeric literals.
    if let Some((lhs, rhs)) = split_binary_op(raw, "==") {
        if let (Some(l), Some(r)) = (parse_decimal(lhs), parse_decimal(rhs)) {
            if (l - r).abs() > f64::EPSILON {
                return Some(cond.to_string());
            }
        }
    }
    if let Some((lhs, rhs)) = split_binary_op(raw, "!=") {
        if let (Some(l), Some(r)) = (parse_decimal(lhs), parse_decimal(rhs)) {
            if (l - r).abs() <= f64::EPSILON {
                return Some(cond.to_string());
            }
        }
    }
    None
}

/// Split `expr` on the first occurrence of `op` that is not part of
/// a different operator. Returns `Some((lhs, rhs))` on success with
/// leading/trailing whitespace stripped from each side, or `None`
/// when the operator is absent or the split would produce an empty
/// side.
fn split_binary_op<'a>(expr: &'a str, op: &str) -> Option<(&'a str, &'a str)> {
    let bytes = expr.as_bytes();
    let op_bytes = op.as_bytes();
    let mut i = 0;
    while i + op_bytes.len() <= bytes.len() {
        if &bytes[i..i + op_bytes.len()] == op_bytes {
            // Ensure we're not seeing a longer operator like `===`
            // or `!==` (ECMAScript strict equality). The accepted
            // surface intentionally rejects those — they are
            // language-specific and we stay conservative.
            let before = i.checked_sub(1).map(|k| bytes[k]);
            let after = bytes.get(i + op_bytes.len()).copied();
            let prefix_extends =
                matches!(before, Some(b'=') | Some(b'!') | Some(b'<') | Some(b'>'));
            let suffix_extends = matches!(after, Some(b'='));
            if !prefix_extends && !suffix_extends {
                let lhs = expr[..i].trim();
                let rhs = expr[i + op_bytes.len()..].trim();
                if !lhs.is_empty() && !rhs.is_empty() {
                    return Some((lhs, rhs));
                }
            }
        }
        i += 1;
    }
    None
}

/// Parse a strict decimal literal. Returns `None` for hex / binary
/// / octal / scientific notation — those carry distinct semantics
/// per language and the validator stays narrow on what it claims
/// to understand. Negative literals (`-1`) are supported.
fn parse_decimal(s: &str) -> Option<f64> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return None;
    }
    // Reject hex / binary / octal / scientific so the classifier
    // doesn't mis-interpret `0x10 == 16` (numerically equal in many
    // languages, but their treatment of hex parsing varies — keep
    // it narrow).
    let body = trimmed.strip_prefix('-').unwrap_or(trimmed);
    if body.contains('x') || body.contains('X') || body.contains('e') || body.contains('E') {
        return None;
    }
    if body.starts_with('0') && body.len() > 1 && !body.starts_with("0.") {
        // Leading-zero numbers (`012`) are octal in some languages —
        // refuse to classify so the heuristic stays conservative.
        return None;
    }
    trimmed.parse::<f64>().ok()
}

/// Is `cond` unconditional? Mirrors the W3C SCXML §5.10 selection
/// model: a transition without a guard always matches when its
/// event descriptor matches, and a transition with a literally-true
/// guard is equivalent for shadowing purposes.
fn is_unconditional(cond: &str) -> bool {
    let raw = cond.trim();
    if raw.is_empty() {
        return true;
    }
    // Language-prefixed conds carry runtime-dependent truth values
    // — never treat as unconditional even when the trailing text
    // looks like `true`.
    if raw.starts_with("cpp:")
        || raw.starts_with("kt:")
        || raw.starts_with("kotlin:")
        || raw.starts_with("rust:")
    {
        return false;
    }
    raw == "true" || raw == "1"
}

/// Do two `event` attribute values describe literally the same event
/// descriptor set? The Phase C walker requires literal equality (after
/// whitespace normalisation) so the shadowing claim is unambiguous —
/// token-prefix superset cases (`event="foo"` covering `event="foo.bar"`)
/// depend on ancestor-priority resolution and are intentionally not
/// flagged.
fn event_descriptors_match(a: &str, b: &str) -> bool {
    let mut ta: Vec<&str> = a.split_whitespace().collect();
    let mut tb: Vec<&str> = b.split_whitespace().collect();
    ta.sort_unstable();
    tb.sort_unstable();
    ta == tb
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forge::diagnostic::ToDiagnostics;
    use crate::model::{SCXMLModel, State, Transition};

    fn state(id: &str, document_order: u32) -> State {
        State {
            id: id.to_string(),
            document_order,
            ..Default::default()
        }
    }

    fn transition(event: &str, cond: &str, target: &str) -> Transition {
        Transition {
            event: event.to_string(),
            cond: cond.to_string(),
            target: target.to_string(),
            ..Default::default()
        }
    }

    fn insert_state(model: &mut SCXMLModel, s: State) {
        model.states.insert(s.id.clone(), s);
    }

    // ── classify_always_false ────────────────────────────────────

    #[test]
    fn literal_false_classified() {
        assert!(classify_always_false("false").is_some());
        assert!(classify_always_false("  false  ").is_some());
    }

    #[test]
    fn literal_zero_classified() {
        assert!(classify_always_false("0").is_some());
        assert!(classify_always_false(" 0 ").is_some());
    }

    #[test]
    fn equal_literals_not_classified() {
        // 1 == 1 is true, not false — skip.
        assert!(classify_always_false("1 == 1").is_none());
    }

    #[test]
    fn differing_literals_classified() {
        assert!(classify_always_false("1 == 2").is_some());
        assert!(classify_always_false("0==1").is_some());
        assert!(classify_always_false("42 == 99").is_some());
    }

    #[test]
    fn negative_literals_classified() {
        assert!(classify_always_false("-1 == 1").is_some());
        assert!(classify_always_false("-1 != -1").is_some());
    }

    #[test]
    fn inequality_equal_literals_classified() {
        assert!(classify_always_false("1 != 1").is_some());
        assert!(classify_always_false("0!=0").is_some());
    }

    #[test]
    fn strict_equality_not_classified() {
        // `===` / `!==` strict equality is language-specific —
        // stay opaque.
        assert!(classify_always_false("1 === 2").is_none());
        assert!(classify_always_false("1 !== 1").is_none());
    }

    #[test]
    fn hex_literals_not_classified() {
        // Cross-base parsing is host-language-specific — refuse.
        assert!(classify_always_false("0x10 == 16").is_none());
    }

    #[test]
    fn language_prefixed_opaque() {
        assert!(classify_always_false("cpp:false").is_none());
        assert!(classify_always_false("kotlin:1==2").is_none());
        assert!(classify_always_false("rust:false").is_none());
    }

    #[test]
    fn identifier_comparison_not_classified() {
        // `flag == false` is not statically false — `flag` is opaque.
        assert!(classify_always_false("flag == false").is_none());
        assert!(classify_always_false("count == 0").is_none());
    }

    // ── is_unconditional ─────────────────────────────────────────

    #[test]
    fn empty_cond_is_unconditional() {
        assert!(is_unconditional(""));
        assert!(is_unconditional("   "));
    }

    #[test]
    fn literal_true_is_unconditional() {
        assert!(is_unconditional("true"));
        assert!(is_unconditional("1"));
    }

    #[test]
    fn opaque_cond_is_not_unconditional() {
        assert!(!is_unconditional("flag"));
        assert!(!is_unconditional("cpp:true"));
        assert!(!is_unconditional("ctx.armed()"));
    }

    // ── event_descriptors_match ──────────────────────────────────

    #[test]
    fn exact_match() {
        assert!(event_descriptors_match("foo", "foo"));
        assert!(event_descriptors_match("foo bar", "foo bar"));
        assert!(event_descriptors_match("foo bar", "bar foo"));
    }

    #[test]
    fn different_descriptors_dont_match() {
        assert!(!event_descriptors_match("foo", "foo.bar"));
        assert!(!event_descriptors_match("foo", "bar"));
        assert!(!event_descriptors_match("*", "foo"));
    }

    // ── validator behaviour ──────────────────────────────────────

    #[test]
    fn always_false_guard_flagged() {
        let mut model = SCXMLModel {
            initial: "armed".to_string(),
            ..Default::default()
        };
        let mut armed = state("armed", 0);
        armed.transitions.push(transition("fire", "false", "armed"));
        insert_state(&mut model, armed);
        let err = validate(&model, "test.scxml").expect_err("must reject");
        let diags = err.error.to_diagnostics();
        assert_eq!(diags[0].code.as_str(), "scxml/always-false-guard");
        assert_eq!(diags[0].actual.as_deref(), Some("false"));
    }

    #[test]
    fn opaque_guard_accepted() {
        let mut model = SCXMLModel {
            initial: "armed".to_string(),
            ..Default::default()
        };
        let mut armed = state("armed", 0);
        armed
            .transitions
            .push(transition("fire", "cpp:ctx.is_armed()", "armed"));
        insert_state(&mut model, armed);
        assert!(validate(&model, "test.scxml").is_ok());
    }

    #[test]
    fn shadowed_transition_flagged() {
        let mut model = SCXMLModel {
            initial: "armed".to_string(),
            ..Default::default()
        };
        let mut armed = state("armed", 0);
        // Unconditional first.
        armed.transitions.push(transition("fire", "", "armed"));
        // Guarded second — shadowed.
        armed
            .transitions
            .push(transition("fire", "ctx.ready", "armed"));
        insert_state(&mut model, armed);
        let err = validate(&model, "test.scxml").expect_err("must reject");
        let diags = err.error.to_diagnostics();
        assert_eq!(diags[0].code.as_str(), "scxml/shadowed-transition");
        assert_eq!(diags[0].actual.as_deref(), Some("fire"));
    }

    #[test]
    fn guarded_then_unconditional_accepted() {
        // Reverse document order — guarded first, unconditional
        // second. No shadowing.
        let mut model = SCXMLModel {
            initial: "armed".to_string(),
            ..Default::default()
        };
        let mut armed = state("armed", 0);
        armed
            .transitions
            .push(transition("fire", "ctx.ready", "armed"));
        armed.transitions.push(transition("fire", "", "armed"));
        insert_state(&mut model, armed);
        assert!(validate(&model, "test.scxml").is_ok());
    }

    #[test]
    fn different_event_not_shadowed() {
        let mut model = SCXMLModel {
            initial: "armed".to_string(),
            ..Default::default()
        };
        let mut armed = state("armed", 0);
        armed.transitions.push(transition("fire", "", "armed"));
        armed
            .transitions
            .push(transition("disarm", "ctx.ready", "armed"));
        insert_state(&mut model, armed);
        assert!(validate(&model, "test.scxml").is_ok());
    }

    #[test]
    fn token_prefix_not_shadowed() {
        // Token-prefix superset deliberately not flagged — the
        // ancestor-priority resolution is beyond the parser-stage
        // walker's scope.
        let mut model = SCXMLModel {
            initial: "armed".to_string(),
            ..Default::default()
        };
        let mut armed = state("armed", 0);
        armed.transitions.push(transition("fire", "", "armed"));
        armed
            .transitions
            .push(transition("fire.specific", "ctx.ready", "armed"));
        insert_state(&mut model, armed);
        assert!(validate(&model, "test.scxml").is_ok());
    }

    #[test]
    fn literal_true_unconditional_shadows() {
        let mut model = SCXMLModel {
            initial: "armed".to_string(),
            ..Default::default()
        };
        let mut armed = state("armed", 0);
        // `cond="true"` is literally unconditional — should shadow.
        armed.transitions.push(transition("fire", "true", "armed"));
        armed
            .transitions
            .push(transition("fire", "ctx.ready", "armed"));
        insert_state(&mut model, armed);
        let err = validate(&model, "test.scxml").expect_err("must reject");
        let diags = err.error.to_diagnostics();
        assert_eq!(diags[0].code.as_str(), "scxml/shadowed-transition");
    }

    #[test]
    fn empty_model_silent() {
        let model = SCXMLModel::default();
        assert!(validate(&model, "test.scxml").is_ok());
    }
}

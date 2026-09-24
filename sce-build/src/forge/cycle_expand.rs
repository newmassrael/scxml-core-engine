//! Expand `cycle_*` calls into ordinary conditional expressions, once,
//! before any backend sees the document.
//!
//! ⚠ WHY HERE AND NOT IN THE EMITTERS. Navigating a `<sce:cycle>` is
//! arithmetic over values the document already declares: the stops are
//! constants and their `when` conditions are expressions over the same
//! fields. Nothing about it differs per language — unlike `round`, whose
//! halfway rule genuinely diverges across the six targets. So an emitter
//! arm per backend would be six copies of one rewrite, and the seventh
//! backend would silently lack it.
//!
//! Expanding before codegen means every downstream pass — type
//! inference, constant folding, the unit checker, all six emitters, and
//! any backend added later — sees expressions it already understands.
//! The same reason [`crate::forge::enum_naming`] owns variant spelling
//! in one place instead of six.
//!
//! ⚠⚠ THE REWRITE IS ON THE EXPRESSION TEXT, and that is a deliberate
//! trade. The alternative is an AST-to-AST pass, which would need a
//! neutral printer this tree does not have (its printers are the
//! language emitters). Text means the result must survive the ordinary
//! parser to get anywhere, so a malformed expansion fails loudly at the
//! next stage rather than reaching a backend. Every spliced fragment is
//! parenthesised for the same reason: precedence is the one thing a
//! textual splice can quietly get wrong.
//!
//! ⚠⚠ WHAT IS REWRITTEN IS FOUND IN THE PARSED TREE, not in the text: a
//! call is a call node whose callee is the bare name, and its bounds and
//! its cursor argument are the spans the parser gave them. Until
//! 2026-09-24 they were found by searching the text for the name and
//! counting parentheses and commas, so a string literal spelling a call
//! (`'cycle_next(modes, cursor)'`) was expanded inside the string — a
//! valid document's string output came out as the expansion's text — and
//! a longer name ending in one (`mycycle_next(…)`) was cut in two.
//!
//! ⚠⚠⚠ THE BOUNDARY RULES ARE DECIDED HERE, not per backend:
//!
//! * **Nothing present.** `cycle_first(c, cur)` answers `cur`. The
//!   signature takes it precisely so the degenerate case has an answer
//!   the DOCUMENT chose. Returning the first *declared* stop instead
//!   would be the primitive claiming a stop is present when its own
//!   condition says it is not.
//! * **The cursor is not a stop.** `cycle_next` / `cycle_prev` answer
//!   the cursor unchanged. Moving along a cycle is the one thing they
//!   do, and "not on the cycle" is not a move they can make. Snapping to
//!   the first stop instead would be defensible — the surveyed
//!   specification states exactly that rule — which is why the rule
//!   belongs in the document, written with `cycle_has`. Folded in here
//!   it would vanish from the document, and the specification's sentence
//!   would have no counterpart to read against.
//!   Standing still is also the louder failure: a document that forgets
//!   the rule gets a cursor that sticks, not one that silently jumps.
//! * **Cost.** `cycle_next` expands to O(n²) conditional terms, because
//!   the expression language has no way to bind the cursor's position
//!   once and reuse it. At the arity this was built for (seven stops,
//!   49 terms) that is fine, and the escape hatch if it ever is not is a
//!   generated helper function per cycle, which is O(n) with a local.

use std::ops::Range;

use crate::forge::expr::{parse_to_ast, ExprKind, TypedExpr};
use crate::forge::model::{Cycle, ForgeDocument, ForgeField, ParsedForge};

/// The call names this pass rewrites.
const CYCLE_CALLS: [&str; 4] = ["cycle_has", "cycle_first", "cycle_next", "cycle_prev"];

/// Is any cycle call present in the document's expressions?
fn uses_cycles(doc: &ForgeDocument) -> bool {
    fields(doc).iter().any(|f| {
        f.expr
            .as_deref()
            .is_some_and(|e| CYCLE_CALLS.iter().any(|c| e.contains(c)))
    })
}

fn fields(doc: &ForgeDocument) -> Vec<&ForgeField> {
    match doc {
        ForgeDocument::Transform(m) => m.inputs.iter().chain(m.outputs.iter()).collect(),
        _ => Vec::new(),
    }
}

/// Expand every cycle call in the document, or `None` when there is
/// nothing to expand.
///
/// ⚠ Only `transform` is rewritten. A cycle call in another kind reaches
/// the type checker unexpanded and is refused there as a call to a name
/// nothing provides — a refusal, not a silent pass-through, which is why
/// the narrow scope is safe to state rather than to widen speculatively.
pub fn expand(parsed: &ParsedForge) -> Option<ForgeDocument> {
    if parsed.cycles.is_empty() || !uses_cycles(&parsed.document) {
        return None;
    }
    let ForgeDocument::Transform(m) = &parsed.document else {
        return None;
    };
    let mut m = m.clone();
    for field in m.inputs.iter_mut().chain(m.outputs.iter_mut()) {
        if let Some(text) = field.expr.take() {
            field.expr = Some(expand_text(&text, &parsed.cycles));
        }
    }
    Some(ForgeDocument::Transform(m))
}

/// Rewrite until no cycle call is left, so a call nested inside
/// another's argument is expanded too.
///
/// ⚠ The pass count is BOUNDED. Each pass replaces a call whose arguments
/// hold no cycle call with text holding none, so the number of calls left
/// strictly decreases and a pass is taken per call the author wrote; the
/// cap is a guard against a rewrite that fails to shrink, which would
/// otherwise be an infinite loop inside a compiler.
fn expand_text(text: &str, cycles: &[Cycle]) -> String {
    let mut out = text.to_string();
    for _ in 0..32 {
        match expand_once(&out, cycles) {
            Some(next) => out = next,
            None => break,
        }
    }
    out
}

/// Replace the innermost cycle call. `None` when there is none left, or
/// when the text does not parse — in which case it is left alone and the
/// ordinary parser reports it.
///
/// Innermost first, so a cursor argument holding a cycle call of its own
/// is expanded once, before the call around it copies that argument into
/// every term of its expansion.
fn expand_once(text: &str, cycles: &[Cycle]) -> Option<String> {
    let tree = parse_to_ast(text).ok()?;
    let call = innermost_cycle_call(&tree, cycles)?;
    let replacement = render(call.name, call.cycle, text[call.cursor].trim());
    Some(format!(
        "{}{}{}",
        &text[..call.at.start],
        replacement,
        &text[call.at.end..]
    ))
}

/// A cycle call as the parser read it.
struct CycleCall<'c> {
    /// The call's own text, callee to closing parenthesis.
    at: Range<usize>,
    name: &'static str,
    cycle: &'c Cycle,
    /// The cursor argument's text.
    cursor: Range<usize>,
}

/// The first cycle call under `node` whose arguments hold none of their
/// own — what is under a node before the node itself.
fn innermost_cycle_call<'c>(node: &TypedExpr, cycles: &'c [Cycle]) -> Option<CycleCall<'c>> {
    let under = node
        .children()
        .into_iter()
        .find_map(|child| innermost_cycle_call(child, cycles));
    under.or_else(|| cycle_call(node, cycles))
}

/// `node` as a cycle call: a call node whose callee is the bare name of
/// one of [`CYCLE_CALLS`], with a declared cycle's id and a cursor as its
/// two arguments. A call of any other shape is not rewritten — the type
/// checker refuses it as a call to a name nothing provides.
fn cycle_call<'c>(node: &TypedExpr, cycles: &'c [Cycle]) -> Option<CycleCall<'c>> {
    let ExprKind::Call { callee, args, .. } = &node.kind else {
        return None;
    };
    let ExprKind::Ident(callee) = &callee.kind else {
        return None;
    };
    let name = CYCLE_CALLS
        .into_iter()
        .find(|name| *name == callee.as_str())?;
    let [id, cursor] = args.as_slice() else {
        return None;
    };
    let ExprKind::Ident(id) = &id.kind else {
        return None;
    };
    Some(CycleCall {
        at: node.span.clone()?,
        name,
        cycle: cycles.iter().find(|cycle| cycle.id == *id)?,
        cursor: cursor.span.clone()?,
    })
}

/// `A.STEP` — how a document names a variant, which the existing
/// variant-reference lowering already resolves per backend.
fn stop(cycle: &Cycle, i: usize) -> String {
    format!("{}.{}", cycle.of, cycle.steps[i].name)
}

/// The condition under which stop `i` is present. A step with no `when`
/// is always present, which is the common case for a fixed-membership
/// cycle.
fn present(cycle: &Cycle, i: usize) -> String {
    match &cycle.steps[i].when {
        Some(w) => format!("({w})"),
        None => "true".to_string(),
    }
}

fn render(name: &str, cycle: &Cycle, arg: &str) -> String {
    let n = cycle.steps.len();
    match name {
        // `(v === S0 && W0) || (v === S1 && W1) || …`
        "cycle_has" => {
            let terms: Vec<String> = (0..n)
                .map(|i| {
                    format!(
                        "((({arg}) === {}) && {})",
                        stop(cycle, i),
                        present(cycle, i)
                    )
                })
                .collect();
            format!("({})", terms.join(" || "))
        }
        // `W0 ? S0 : W1 ? S1 : … : cur`
        "cycle_first" => {
            let mut out = format!("({arg})");
            for i in (0..n).rev() {
                out = format!("({} ? {} : {})", present(cycle, i), stop(cycle, i), out);
            }
            out
        }
        // For each stop, the first present stop after (or before) it in
        // wrap order; the cursor unchanged when it is on no stop, or when
        // no other stop is present.
        "cycle_next" | "cycle_prev" => {
            let mut out = format!("({arg})");
            for i in (0..n).rev() {
                let mut inner = format!("({arg})");
                for k in (1..n).rev() {
                    let j = if name == "cycle_next" {
                        (i + k) % n
                    } else {
                        (i + n - k) % n
                    };
                    inner = format!("({} ? {} : {})", present(cycle, j), stop(cycle, j), inner);
                }
                out = format!("((({arg}) === {}) ? {} : {})", stop(cycle, i), inner, out);
            }
            out
        }
        _ => unreachable!("render is only called with a name from CYCLE_CALLS"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forge::model::CycleStep;

    /// A cycle `modes` over `Mode`, one stop per name, each present when
    /// `<name>On` holds.
    fn cycle(stops: &[&str]) -> Cycle {
        Cycle {
            id: "modes".to_string(),
            of: "Mode".to_string(),
            steps: stops
                .iter()
                .map(|name| CycleStep {
                    name: (*name).to_string(),
                    when: Some(format!("{}On", name.to_lowercase())),
                })
                .collect(),
            line: None,
        }
    }

    fn expanded(text: &str, stops: &[&str]) -> String {
        expand_text(text, &[cycle(stops)])
    }

    #[test]
    fn a_string_that_spells_a_call_is_left_as_written() {
        let text = "'cycle_next(modes, cursor)'";
        assert_eq!(expanded(text, &["ECO", "NORMAL"]), text);
    }

    #[test]
    fn a_longer_name_ending_in_a_call_is_not_one() {
        let text = "mycycle_next(modes, cursor)";
        assert_eq!(expanded(text, &["ECO", "NORMAL"]), text);
    }

    #[test]
    fn a_member_named_like_a_call_is_not_one() {
        let text = "gear.cycle_next(modes, cursor)";
        assert_eq!(expanded(text, &["ECO", "NORMAL"]), text);
    }

    #[test]
    fn a_string_in_the_cursor_ends_neither_the_argument_nor_the_call() {
        let stops = ["ECO", "NORMAL"];
        let cursor = "pick(')', 'a,b')";
        assert_eq!(
            expanded(&format!("cycle_has(modes, {cursor})"), &stops),
            render("cycle_has", &cycle(&stops), cursor)
        );
    }

    #[test]
    fn a_call_keeps_the_text_around_it() {
        let stops = ["ECO", "NORMAL"];
        assert_eq!(
            expanded("  x + cycle_first(modes, cursor) * 2", &stops),
            format!(
                "  x + {} * 2",
                render("cycle_first", &cycle(&stops), "cursor")
            )
        );
    }

    /// Expanded innermost first, the cursor's own call is expanded once;
    /// expanded from the outside, `cycle_next` copies its cursor into
    /// n² places — 49 for seven stops — and the pass cap ran out with
    /// calls still in the text, which the type checker then refused as
    /// calls to a name nothing provides.
    #[test]
    fn a_call_in_the_cursor_is_expanded_before_the_call_around_it() {
        let stops = ["A", "B", "C", "D", "E", "F", "G"];
        let text = "cycle_next(modes, cycle_prev(modes, cursor))";
        let inner = render("cycle_prev", &cycle(&stops), "cursor");
        assert_eq!(
            expanded(text, &stops),
            render("cycle_next", &cycle(&stops), &inner)
        );
    }

    #[test]
    fn a_call_to_a_cycle_nobody_declared_is_left_for_the_type_checker() {
        let text = "cycle_next(nothing, cursor)";
        assert_eq!(expanded(text, &["ECO", "NORMAL"]), text);
    }
}

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
/// ⚠ The pass count is BOUNDED. Each pass replaces the outermost call it
/// finds with text containing no cycle calls of its own except those
/// that came from the arguments, so the nesting depth strictly
/// decreases; the cap is a guard against a rewrite that fails to shrink,
/// which would otherwise be an infinite loop inside a compiler.
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

/// Find the first cycle call and replace it. `None` when there is none
/// left, or when its shape is not one this pass can read — in which case
/// the text is left alone and the ordinary parser reports it.
fn expand_once(text: &str, cycles: &[Cycle]) -> Option<String> {
    for name in CYCLE_CALLS {
        let mut from = 0usize;
        while let Some(rel) = text[from..].find(name) {
            let at = from + rel;
            from = at + name.len();
            // A call, not an identifier that merely starts the same way.
            let rest = text[at + name.len()..].trim_start();
            if !rest.starts_with('(') {
                continue;
            }
            let open = text[at + name.len()..].find('(')? + at + name.len();
            let close = matching_paren(text, open)?;
            let inside = &text[open + 1..close];
            let (id, arg) = split_top_comma(inside)?;
            let cycle = cycles.iter().find(|c| c.id == id.trim())?;
            let replacement = render(name, cycle, arg.trim());
            return Some(format!(
                "{}{}{}",
                &text[..at],
                replacement,
                &text[close + 1..]
            ));
        }
    }
    None
}

/// Index of the `)` matching the `(` at `open`.
fn matching_paren(text: &str, open: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut depth = 0usize;
    for (i, b) in bytes.iter().enumerate().skip(open) {
        match b {
            b'(' => depth += 1,
            b')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

/// Split `"<id>, <expr>"` at the comma that is not inside parentheses.
fn split_top_comma(inside: &str) -> Option<(&str, &str)> {
    let mut depth = 0usize;
    for (i, ch) in inside.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => depth = depth.checked_sub(1)?,
            ',' if depth == 0 => return Some((&inside[..i], &inside[i + 1..])),
            _ => {}
        }
    }
    None
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
